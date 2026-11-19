#!/usr/bin/env python3
"""Checks how tools/parallel-tests.py gives a failed test process its second
run, without building anything: stand-ins for cargo, rustc and the test
programs record when each process ran.

python3 tools/test-parallel-tests.py
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

TOOL = Path(__file__).resolve().parent / "parallel-tests.py"

CARGO = '''#!{python}
import json, os, sys
root = os.environ["STAND_IN_ROOT"]
manifest = os.path.join(root, "package", "Cargo.toml")
if "metadata" in sys.argv:
    print(json.dumps({{"packages": [{{"manifest_path": manifest, "name": "package", "version": "0.1.0"}}],
                      "target_directory": os.path.join(root, "target")}}))
elif os.path.basename(sys.argv[0]) == "rustc":
    print(os.path.join(root, "sysroot"))
else:
    for name in json.loads(os.environ["STAND_IN_PLAN"]):
        print(json.dumps({{"reason": "compiler-artifact", "profile": {{"test": True}}, "manifest_path": manifest,
                          "package_id": "package 0.1.0", "executable": os.path.join(root, "target/debug/deps", name + "-0000"),
                          "target": {{"name": name, "kind": ["lib"], "src_path": os.path.join(root, "package/src/lib.rs")}}}}))
'''

# A test program with three tests. What `b::second` does is in STAND_IN_PLAN:
# "ok", "slow" (passes late), "fail" (fails every time) or "once" (fails the
# first time this process runs these tests).
PROGRAM = '''#!{python}
import hashlib, json, os, sys, time
args = sys.argv[1:]
name = os.path.basename(sys.argv[0]).rsplit("-", 1)[0]
tests = ["a::first", "b::second", "c::third"]
if "--list" in args:
    print("\\n".join(test + ": test" for test in tests))
    sys.exit(0)
chosen = [test for test in tests if test in args] if "--exact" in args else tests
plan = json.loads(os.environ["STAND_IN_PLAN"])[name]
start = time.time()
time.sleep(0.6 if plan == "slow" else 0.05)
fails = plan == "fail" and "b::second" in chosen
if plan == "once" and "b::second" in chosen:
    marker = os.path.join(os.environ["STAND_IN_ONCE"], hashlib.sha1((name + " ".join(chosen)).encode()).hexdigest())
    fails = not os.path.exists(marker)
    open(marker, "w").close()
print("running %d tests" % len(chosen))
for test in chosen:
    print("test %s ... %s" % (test, "FAILED" if fails and test == "b::second" else "ok"))
print("test result: %s. %d passed; %d failed; 0 ignored; 0 measured; %d filtered out; finished in 0.05s"
      % ("FAILED" if fails else "ok", len(chosen) - fails, fails, len(tests) - len(chosen)))
with open(os.environ["STAND_IN_TRACE"], "a") as stream:
    stream.write(json.dumps({{"program": name, "args": args, "start": start, "end": time.time()}}) + "\\n")
sys.exit(101 if fails else 0)
'''


class Fixture:
    def __init__(self, root):
        self.root = root
        for folder in ("bin", "package/src", "target/debug/deps", "sysroot/lib"):
            (root / folder).mkdir(parents=True)
        (root / "package/Cargo.toml").write_text('[package]\nname = "package"\nversion = "0.1.0"\n')
        cargo = root / "bin/cargo"
        cargo.write_text(CARGO.format(python=sys.executable))
        cargo.chmod(0o755)
        (root / "bin/rustc").symlink_to("cargo")
        self.program = PROGRAM.format(python=sys.executable)
        self.runs = 0

    def run(self, plan, *options, environment=None):
        self.runs += 1
        deps = self.root / "target/debug/deps"
        shutil.rmtree(deps)
        deps.mkdir()
        for name in plan:
            program = deps / f"{name}-0000"
            program.write_text(self.program)
            program.chmod(0o755)
        trace = self.root / f"trace-{self.runs}"
        once = self.root / f"once-{self.runs}"
        once.mkdir()
        self.logs = self.root / f"logs-{self.runs}"
        env = dict(os.environ, PATH=f"{self.root / 'bin'}{os.pathsep}{os.environ.get('PATH', '')}",
                   STAND_IN_ROOT=str(self.root), STAND_IN_PLAN=json.dumps(plan), STAND_IN_TRACE=str(trace),
                   STAND_IN_ONCE=str(once), **(environment or {}))
        for name in ("PARALLEL_TEST_JOBS", "PARALLEL_TEST_RETRY", "PARALLEL_TEST_CHUNK", "GITHUB_STEP_SUMMARY", "RUNNER_TEMP"):
            if name not in (environment or {}):
                env.pop(name, None)
        done = subprocess.run([sys.executable, str(TOOL), "--jobs", "3", "--log", str(self.logs), *options, "--", "--lib"],
                              cwd=self.root / "package", env=env, capture_output=True, text=True)
        ran = [json.loads(line) for line in trace.read_text().splitlines()] if trace.exists() else []
        timings = self.logs / "timings.json"
        return done, ran, json.loads(timings.read_text()) if timings.exists() else []


def times(ran, name):
    return len([r for r in ran if r["program"] == name])


def main():
    outcomes = []

    def expect(label, condition, detail=""):
        outcomes.append(bool(condition))
        print(("ok    " if condition else "WRONG ") + label + (f": {detail}" if detail and not condition else ""), flush=True)

    with tempfile.TemporaryDirectory() as folder:
        fixture = Fixture(Path(folder).resolve())
        steady = {"alpha": "slow", "beta": "ok", "gamma": "slow", "delta": "ok"}

        done, ran, timings = fixture.run(steady)
        expect("when every process passes, each runs once and nothing tells of a second run",
               done.returncode == 0 and len(ran) == 4 and "second run" not in done.stdout
               and "12 passed; 0 failed" in done.stdout and not any("second_run" in row for row in timings),
               (done.returncode, len(ran), done.stdout[-300:], done.stderr[-300:]))

        done, ran, timings = fixture.run({**steady, "beta": "once"})
        beta = sorted((r for r in ran if r["program"] == "beta"), key=lambda r: r["start"])
        others = [r for r in ran if r["program"] != "beta"]
        expect("a process that failed once runs once more, and the run passes",
               done.returncode == 0 and len(beta) == 2 and len(others) == 3,
               (done.returncode, len(beta), done.stdout[-400:], done.stderr[-300:]))
        expect("the second run starts when every other process has ended",
               len(beta) == 2 and beta[1]["start"] >= max(r["end"] for r in others + beta[:1]))
        expect("the totals are those of the last runs", "parallel-tests: 12 passed; 0 failed; 0 ignored" in done.stdout,
               done.stdout[-400:])
        expect("the test that had failed is listed at the end and as a warning",
               "parallel-tests: passed only on a second run (1):\n  [beta] b::second (beta-0000: exit status 101)" in done.stdout
               and "::warning title=Passed only on a second run::[beta] b::second failed in beta-0000" in done.stdout
               and "failed tests" not in done.stdout and "FAILED process" not in done.stdout, done.stdout[-500:])
        first, second = fixture.logs / "beta-0000.log", fixture.logs / "beta-0000.second.log"
        row = next((row for row in timings if row["process"] == "beta-0000"), {})
        expect("both runs keep their logs, and the timings name the second",
               first.is_file() and "b::second ... FAILED" in first.read_text()
               and second.is_file() and "b::second ... ok" in second.read_text()
               and row.get("problem") == "exit status 101"
               and row.get("second_run", {}).get("problem") == ""
               and row.get("second_run", {}).get("failed_first") == ["[beta] b::second"], row)

        done, ran, timings = fixture.run({**steady, "beta": "fail"})
        expect("a process that fails twice fails, with its failed test named",
               done.returncode == 1 and times(ran, "beta") == 2 and "parallel-tests: 11 passed; 1 failed" in done.stdout
               and "parallel-tests: failed tests (1):\n  [beta] b::second" in done.stdout
               and "parallel-tests: FAILED process beta-0000: exit status 101" in done.stdout
               and "passed only on a second run" not in done.stdout and "::warning" not in done.stdout,
               (done.returncode, times(ran, "beta"), done.stdout[-500:]))

        done, ran, timings = fixture.run({**steady, "beta": "once"}, "--retry", "0")
        expect("--retry 0 gives no second run", done.returncode == 1 and times(ran, "beta") == 1 and len(ran) == 4,
               (done.returncode, times(ran, "beta")))
        done, ran, timings = fixture.run({**steady, "beta": "once"}, environment={"PARALLEL_TEST_RETRY": "0"})
        expect("PARALLEL_TEST_RETRY=0 gives none either", done.returncode == 1 and times(ran, "beta") == 1,
               (done.returncode, times(ran, "beta")))

        many = {f"p{number}": "once" for number in range(1, 7)}
        done, ran, timings = fixture.run({**steady, **many})
        expect("six failed processes are not chance: none runs again",
               done.returncode == 1 and len(ran) == 10
               and "6 processes failed, more than 5: none gets a second run" in done.stdout,
               (done.returncode, len(ran), done.stdout[-300:]))
        done, ran, timings = fixture.run({**steady, **{name: "once" for name in list(many)[:5]}})
        expect("five still get their second runs", done.returncode == 0 and len(ran) == 14
               and "passed only on a second run (5):" in done.stdout, (done.returncode, len(ran)))

        done, ran, timings = fixture.run({**steady, "beta": "once"}, "--split", "beta", "--chunk", "2")
        beta = sorted((r for r in ran if r["program"] == "beta"), key=lambda r: r["start"])
        with_second = [r for r in beta if "b::second" in r["args"]]
        expect("of a split program, only the slice that failed runs once more, with the same tests",
               done.returncode == 0 and len(beta) == 3 and len(with_second) == 2
               and with_second[0]["args"] == with_second[1]["args"]
               and "[beta] b::second (beta-0000.1of2: exit status 101)" in done.stdout,
               (done.returncode, [r["args"] for r in beta], done.stdout[-300:]))

    wrong = outcomes.count(False)
    print(f"{len(outcomes)} checks of the test runner, {wrong} wrong")
    return 1 if wrong else 0


if __name__ == "__main__":
    sys.exit(main())
