#!/usr/bin/env python3
"""Checks how tools/check-hagency-regressions.py runs its checks, without
building anything: stand-ins for cargo, python3 and bash record when each
command ran, and the order is checked one after another and side by side.

python3 tools/test-hagency-regressions-runner.py
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

GATE = Path(__file__).resolve().parent / "check-hagency-regressions.py"

STAND_IN = '''#!{python}
import json, os, random, sys, time
args = sys.argv[1:]
name = os.path.basename(sys.argv[0])
line = " ".join(args)
start = time.time()
slow = any(part in line for part in os.environ.get("STAND_IN_SLOW", "").split("|") if part)
time.sleep(1.5 if slow else 0.01 * (1 + random.random()))
code = 1 if any(part in line for part in os.environ.get("STAND_IN_FAIL", "").split("|") if part) else 0
if name == "cargo":
    if "--message-format=json" in args:
        for target in ("hagency-managed-runtime-probe", "hagency-runtime-probe"):
            path = os.path.join(os.environ["STAND_IN_BUILT"], target)
            open(path, "w").write("built with shared-context")
            print(json.dumps({{"reason": "compiler-artifact", "target": {{"name": target}}, "executable": path}}))
    # As Cargo writes it with CARGO_TERM_COLOR=always, which CI sets.
    print("\\x1b[1m\\x1b[32m    Finished\\x1b[0m `test` profile [unoptimized + debuginfo] target(s) in 1m 02s")
    if "test" in args and code == 0:
        print("test result: ok. 3 passed; 0 failed; 0 ignored")
with open(os.environ["STAND_IN_TRACE"], "a") as stream:
    stream.write(json.dumps({{"program": name, "args": args, "cwd": os.getcwd(), "start": start, "end": time.time(),
                             "probe": os.environ.get("HAGENCY_TEST_RUNTIME_PROBE"),
                             "claude": os.environ.get("HAGENCY_TEST_CLAUDE_RUNTIME_PROBE")}}) + "\\n")
sys.exit(code)
'''


class Fixture:
    def __init__(self, root):
        self.root = root
        (root / "tree/tools").mkdir(parents=True)
        (root / "tree/crates/hagency-client").mkdir(parents=True)
        (root / "built").mkdir()
        (root / "bin").mkdir()
        self.gate = root / "tree/tools" / GATE.name
        shutil.copy(GATE, self.gate)
        stand_in = root / "bin/stand-in"
        stand_in.write_text(STAND_IN.format(python=sys.executable))
        stand_in.chmod(0o755)
        for name in ("cargo", "python3", "bash"):
            (root / "bin" / name).symlink_to("stand-in")
        self.output = root / "output"
        self.runs = 0

    def run(self, *options, fail="", slow="", fresh=True, jobs_from_environment=None):
        if fresh:
            shutil.rmtree(self.output, ignore_errors=True)
        self.runs += 1
        trace = self.root / f"trace-{self.runs}"
        env = dict(os.environ, PATH=f"{self.root / 'bin'}{os.pathsep}{os.environ.get('PATH', '')}",
                   STAND_IN_TRACE=str(trace), STAND_IN_BUILT=str(self.root / "built"), STAND_IN_FAIL=fail,
                   STAND_IN_SLOW=slow)
        env.pop("HAGENCY_REGRESSION_JOBS", None)
        env.pop("CARGO_TARGET_DIR", None)
        if jobs_from_environment:
            env["HAGENCY_REGRESSION_JOBS"] = jobs_from_environment
        done = subprocess.run([sys.executable, str(self.gate), "--output", str(self.output), *options],
                              env=env, capture_output=True, text=True)
        ran = [json.loads(line) for line in trace.read_text().splitlines()] if trace.exists() else []
        report = self.output / "report.json"
        return done, ran, json.loads(report.read_text()) if report.exists() else []


def most_at_once(ran):
    now = most = 0
    for _, step in sorted([(r["start"], 1) for r in ran] + [(r["end"], -1) for r in ran]):
        now += step
        most = max(most, now)
    return most


def overlap(a, b):
    return a["start"] < b["end"] and b["start"] < a["end"]


def command(r):
    return json.dumps([r["program"], r["args"], r["cwd"]])


def commands(ran):
    return sorted(command(r) for r in ran)


def order_faults(ran, output, listed):
    """What must hold however many checks run side by side. `listed` is the
    commands in the order of the list, as a run one after another gives it."""
    faults = []
    place = {key: index for index, key in reversed(list(enumerate(listed)))}
    builders = [r for r in ran if r["program"] == "cargo" and {"--test", "--all-targets", "--bin"} & set(r["args"])]

    def package(r):
        args = r["args"]
        return (r["cwd"], args[args.index("-p") + 1] if "-p" in args else "",
                args[args.index("--manifest-path") + 1] if "--manifest-path" in args else "")

    for first, a in enumerate(builders):
        for b in builders[first + 1:]:
            if package(a) == package(b) and overlap(a, b):
                faults.append(f"two commands that build the programs of {package(a)[1]} overlap")
    kept = str(output / "programs/hagency-managed-runtime-probe")
    kept_claude = str(output / "programs/hagency-runtime-probe")
    probes = [r for r in ran if "--message-format=json" in r["args"]]
    for r in ran:
        if r in probes:
            continue
        before = place[command(r)] < next(place[key] for key in listed if "--message-format=json" in key)
        if before and (r["probe"] or r["claude"]):
            faults.append("a check listed before the probe build was handed a probe")
        elif not before and (r["probe"], r["claude"]) != (kept, kept_claude):
            faults.append("a check listed after the probe build was not handed the kept probes")
        elif not before and probes and r["start"] < probes[0]["end"]:
            faults.append("a check listed after the probe build started before it ended")
    vm = [r for r in ran if "tools/hagency-script-review/Cargo.toml" in r["args"]]
    for r in ran:
        if r["program"] == "python3" and r["args"][0].startswith("apps/hagency/tests/") and vm and r["start"] < vm[0]["end"]:
            faults.append("a script check started before the script VM was built")
    return faults


def main():
    outcomes = []

    def expect(label, condition, detail=""):
        outcomes.append(bool(condition))
        print(("ok    " if condition else "WRONG ") + label + (f": {detail}" if detail and not condition else ""), flush=True)

    with tempfile.TemporaryDirectory() as folder:
        fixture = Fixture(Path(folder).resolve())
        output = fixture.output

        done, ran, report = fixture.run()
        every = [row["check"] for row in report]
        expect("one after another is the default and passes", done.returncode == 0 and most_at_once(ran) == 1,
               done.stderr[-300:])
        expect("the report lists every command that ran, each passed",
               len(report) == len(ran) > 100 and all(row["passed"] for row in report), (len(report), len(ran)))
        listed = [command(r) for r in ran]
        expect("no command is in the list twice", len(set(listed)) == len(listed))
        expect("one after another: order rules hold", not order_faults(ran, output, listed), order_faults(ran, output, listed)[:3])
        times = {row["check"]: row for row in report}
        expect("each row carries its seconds, and Cargo's build time where Cargo ran",
               all(row["seconds"] >= 0 for row in report) and times["budget-host"]["build_seconds"] == 62.0
               and times["owner-source-pins"]["build_seconds"] == 0, times["budget-host"])
        serial = commands(ran)

        done, ran, report = fixture.run("--jobs", "6")
        expect("six at a time passes, with more than one and at most six at once",
               done.returncode == 0 and 1 < most_at_once(ran) <= 6, (done.returncode, most_at_once(ran)))
        expect("six at a time runs the same commands", commands(ran) == serial)
        expect("six at a time: the report is in list order, each passed",
               [row["check"] for row in report] == every and all(row["passed"] for row in report))
        expect("six at a time: order rules hold", not order_faults(ran, output, listed), order_faults(ran, output, listed)[:3])
        expect("six at a time: every check has its log", all((output / f"{name}.log").is_file() for name in every))

        done, ran, report = fixture.run("--jobs", "4", slow="hagency-matrix --lib approval_")
        long = [r for r in ran if r["end"] - r["start"] > 1]
        probe = next(index for index, key in enumerate(listed) if "--message-format=json" in key)
        place = {key: index for index, key in enumerate(listed)}
        expect("a long check listed before the probe build holds up nothing listed after it",
               done.returncode == 0 and len(long) == 1 and place[command(long[0])] < probe
               and any(place[command(r)] > probe and r["start"] < long[0]["end"] for r in ran)
               and not order_faults(ran, output, listed), (done.returncode, len(long)))

        done, ran, report = fixture.run(fail="hagency::run_controls::tests")
        expect("one after another, a failed check ends the list there",
               done.returncode != 0 and "agent-run-cards.log; no release until this check passes" in done.stderr
               and report[-1] == dict(report[-1], check="agent-run-cards", passed=False)
               and all(row["passed"] for row in report[:-1]) and len(ran) == len(report), done.stderr[-300:])
        expect("the end of the failed check's log is printed", "Last 200 lines of agent-run-cards.log" in done.stdout)

        done, ran, report = fixture.run("--jobs", "6", fail="hagency::run_controls::tests|home::space_management::tests")
        first = every.index("space-management")
        names = [row["check"] for row in report]
        expect("six at a time, the first failed check in the list is the one named",
               done.returncode != 0 and "space-management.log; no release until this check passes" in done.stderr,
               done.stderr[-300:])
        expect("every check listed before it ran and passed",
               names[:first] == every[:first] and all(row["passed"] for row in report[:first]))
        expect("the checks listed well after it did not run", len(ran) < first + 40, (len(ran), first))
        expect("a second failed check is named as well, if it ran",
               "Also failed: agent-run-cards" in done.stdout or "agent-run-cards" not in names)

        done, ran, report = fixture.run("--jobs", "6", "--start-at", "space-management", fresh=False)
        expect("resuming at the failed check runs it and what follows, and reuses the rest",
               done.returncode == 0 and len(ran) == len(every) - first
               and [row["check"] for row in report] == every and all(row["passed"] for row in report),
               (done.returncode, len(ran), len(every) - first))
        expect("resuming: order rules hold", not order_faults(ran, output, listed), order_faults(ran, output, listed)[:3])

        done, ran, report = fixture.run("--jobs", "4", "--start-at", "native-claude-runtime-protocol", fresh=False)
        expect("resuming after the probe build still hands on the kept probe",
               done.returncode == 0 and ran and all(r["probe"] == str(output / "programs/hagency-managed-runtime-probe") for r in ran),
               done.stderr[-300:])

        done, ran, report = fixture.run("--jobs", "6", fail="tools/hagency-script-review/Cargo.toml")
        expect("no script check runs when the script VM does not build",
               "script-vm.log; no release" in done.stderr
               and not any(r["program"] == "python3" and r["args"][0].startswith("apps/hagency/tests/") for r in ran),
               done.stderr[-300:])

        done, ran, report = fixture.run("--jobs", "6", fail="--message-format=json")
        expect("no check listed after a failed probe build runs",
               "native-context-probe.log; no release" in done.stderr
               and not any(place[command(r)] > probe for r in ran), done.stderr[-300:])

        done, ran, report = fixture.run("--jobs", "0")
        expect("--jobs 0 is refused", done.returncode == 2 and not ran)
        done, ran, report = fixture.run("--jobs", "3", "--cargo-config", "config.toml")
        expect("--jobs above 1 with --cargo-config is refused", done.returncode == 2 and not ran)
        done, ran, report = fixture.run(jobs_from_environment="5")
        expect("HAGENCY_REGRESSION_JOBS sets the number", done.returncode == 0 and "5 at a time" in done.stdout,
               done.stdout[-300:])

    wrong = outcomes.count(False)
    print(f"{len(outcomes)} checks of the runner, {wrong} wrong")
    return 1 if wrong else 0


if __name__ == "__main__":
    sys.exit(main())
