#!/usr/bin/env python3
"""Run `cargo test` selections with several test programs running side by side.

    python3 tools/parallel-tests.py [--jobs N] [--split NAME] [--first TEST] [--chunk N] [--retry 0|1]
        [--log DIR] -- <cargo test arguments> [--- <cargo test arguments of a second selection> ...]

The arguments after the first `--` are those of `cargo test`, including a
second `--` and the arguments of the test programs. `---` starts another
selection: all selections are built first, one after another, and their test
programs then share one queue. `--no-fail-fast` is implied: every program
runs, and the exit status is 1 when anything failed.

What is the same as `cargo test`: one build per selection with the same
arguments; the same test programs; each program started once, in its package's
folder, with the variables cargo sets and the same arguments. The tests of one
program therefore run in one process, in the same order as before, one after
another when RUST_TEST_THREADS=1 or `--test-threads=1` says so.

What is different: up to N programs run at the same time, so other test
processes use the machine while a test runs.

`--split NAME` (the name of a test target, e.g. the crate name of a library)
starts that one program several times, each time for a slice of at most
`--chunk` tests named exactly. Use it only for a program whose tests do not
depend on which other tests ran before them in the same process. Many suites
here do: the first test of a process that touches the installation's writer
root decides, for the whole process, whether a test root can still be chosen
(`hagency_platform::legacy_writer::test_support`). A slice that happens to
begin with a test that only looks then fails every later test that needs one.

`--first TEST` names a test that a split program runs before all its others
because its name sorts first: tests of one process run in the order of their
names. Every slice of a program that has this test gets it too, so each
process begins the way the whole program does. The `hagency` library has such
a test, which selects the writer root; that is what lets it be split. The run
stops when the named test is not the first of its program.

Why: the suites are serial inside a process because its tests share that
process's stores and deadlines. One job therefore used one core for most of
its time. Separate processes share none of that.

A process that ends with a failure runs once more (`--retry 0` or
PARALLEL_TEST_RETRY=0 turns that off). Many tests here start a program or a
server and give it a time limit, and on a busy machine one test in several
thousand now and then misses its limit. The second run starts when every
other process has ended, so it has the machine to itself. When it passes, the
process counts as passed, and the tests that failed the first time are listed
at the end and as warnings, so that a test which often needs its second run
is seen. What gets no second run: a process stopped at `--timeout`, one that
could not be started, and any process when more than SECOND_RUNS_AT_MOST
failed, because that many failures are not chance.
"""
import argparse
import json
import os
import pathlib
import queue
import re
import shutil
import subprocess
import sys
import threading
import time

RESULT = re.compile(
    r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out"
)
FAILED_TEST = re.compile(r"^test (\S+) \.\.\. FAILED", re.M)
SECOND_RUNS_AT_MOST = 5


def default_jobs():
    """On a small machine one core stays free for the servers and helpers the tests start."""
    cores = os.cpu_count() or 2
    if cores <= 2:
        return cores
    return cores - 1 if cores <= 4 else max(4, cores // 2)


# The options of `cargo test` whose value is the next argument.
CARGO_VALUES = {"-p", "--package", "--exclude", "-F", "--features", "--manifest-path", "--target", "--target-dir",
                "--profile", "--test", "--bin", "--example", "--bench", "-j", "--jobs", "--color", "--config",
                "-Z", "--message-format", "--lockfile-path"}
TARGETS = ("--lib", "--bins", "--bin", "--tests", "--test", "--examples", "--example", "--benches", "--bench",
           "--all-targets")


def split_arguments(arguments):
    """cargo's arguments and the test programs' arguments. `cargo test [options] [TESTNAME] [-- arguments]`:
    a test name before the `--` is a filter like one after it."""
    cargo, tests = arguments, []
    if "--" in arguments:
        at = arguments.index("--")
        cargo, tests = arguments[:at], arguments[at + 1 :]
    kept, names, index = [], [], 0
    while index < len(cargo):
        argument = cargo[index]
        if argument in CARGO_VALUES and index + 1 < len(cargo):
            kept += cargo[index : index + 2]
            index += 2
            continue
        (kept if argument.startswith("-") else names).append(argument)
        index += 1
    return kept, names + tests


def test_selection(test_arguments):
    """The name filters, the skips and the remaining arguments of the test programs."""
    filters, skips, rest = [], [], []
    exact = False
    index = 0
    while index < len(test_arguments):
        argument = test_arguments[index]
        if argument == "--skip":
            skips.append(test_arguments[index + 1])
            index += 2
            continue
        if argument.startswith("--skip="):
            skips.append(argument.split("=", 1)[1])
        elif argument == "--exact":
            exact = True
        elif argument.startswith("-"):
            rest.append(argument)
            # The options of the test harness that take a value in the next argument.
            if argument in ("--test-threads", "--format", "--color", "--logfile", "-Z", "--shuffle-seed") \
                    and index + 1 < len(test_arguments):
                rest.append(test_arguments[index + 1])
                index += 1
        else:
            filters.append(argument)
        index += 1
    return filters, skips, exact, rest


def selected(name, filters, skips, exact):
    """What the test harness itself does with filters and skips."""
    if exact:
        if filters and name not in filters:
            return False
        return name not in skips
    if filters and not any(f in name for f in filters):
        return False
    return not any(s in name for s in skips)


def build(cargo_arguments, folder):
    """Build the test programs; return them in cargo's order."""
    arguments = [a for a in cargo_arguments if a != "--no-fail-fast"]
    command = ["cargo", "test", *arguments, "--no-run", "--message-format=json-render-diagnostics"]
    print("+ " + " ".join(command), flush=True)
    process = subprocess.Popen(command, cwd=folder, stdout=subprocess.PIPE, text=True)
    programs = []
    for line in process.stdout:
        if not line.startswith("{"):
            continue
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("reason") != "compiler-artifact" or not message.get("executable"):
            continue
        if not message.get("profile", {}).get("test"):
            continue
        target = message["target"]
        programs.append(
            {
                "executable": message["executable"],
                "manifest": message["manifest_path"],
                "package": message["package_id"],
                "name": target["name"],
                "kind": target["kind"][0] if target.get("kind") else "",
                "source": target.get("src_path", ""),
            }
        )
    if process.wait() != 0:
        sys.exit(f"cargo test --no-run failed with status {process.returncode}")
    return programs


def packages(folder, cargo_arguments):
    """Package facts for the variables cargo gives a test program."""
    command = ["cargo", "metadata", "--format-version", "1", "--no-deps"]
    for flag in ("--locked", "--offline", "--frozen"):
        if flag in cargo_arguments:
            command.append(flag)
    for index, argument in enumerate(cargo_arguments):
        if argument == "--manifest-path" and index + 1 < len(cargo_arguments):
            command += ["--manifest-path", cargo_arguments[index + 1]]
        elif argument.startswith("--manifest-path="):
            command.append(argument)
    out = subprocess.run(command, cwd=folder, capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit("cargo metadata failed: " + out.stderr[-2000:])
    data = json.loads(out.stdout)
    return {p["manifest_path"]: p for p in data["packages"]}, data["target_directory"]


def environment(program, package, sysroot):
    """The variables `cargo test` sets when it starts a test program."""
    env = dict(os.environ)
    manifest = pathlib.Path(program["manifest"])
    env["CARGO_MANIFEST_DIR"] = str(manifest.parent)
    env["CARGO_MANIFEST_PATH"] = str(manifest)
    cargo = shutil.which("cargo")
    if cargo:
        env.setdefault("CARGO", cargo)
    if package:
        version = package.get("version", "")
        core, _, pre = version.partition("-")
        parts = (core.split(".") + ["", "", ""])[:3]
        env.update(
            {
                "CARGO_PKG_NAME": package.get("name", ""),
                "CARGO_PKG_VERSION": version,
                "CARGO_PKG_VERSION_MAJOR": parts[0],
                "CARGO_PKG_VERSION_MINOR": parts[1],
                "CARGO_PKG_VERSION_PATCH": parts[2],
                "CARGO_PKG_VERSION_PRE": pre,
                "CARGO_PKG_AUTHORS": ":".join(package.get("authors") or []),
                "CARGO_PKG_DESCRIPTION": package.get("description") or "",
                "CARGO_PKG_HOMEPAGE": package.get("homepage") or "",
                "CARGO_PKG_REPOSITORY": package.get("repository") or "",
                "CARGO_PKG_LICENSE": package.get("license") or "",
                "CARGO_PKG_LICENSE_FILE": package.get("license_file") or "",
                "CARGO_PKG_RUST_VERSION": package.get("rust_version") or "",
                "CARGO_PKG_README": package.get("readme") or "",
                "CARGO_CRATE_NAME": program["name"].replace("-", "_"),
            }
        )
    deps = str(pathlib.Path(program["executable"]).parent)
    profile = str(pathlib.Path(deps).parent)
    paths = [deps, profile] + ([os.path.join(sysroot, "lib")] if sysroot else [])
    if sys.platform == "darwin":
        variable, fallback = "DYLD_FALLBACK_LIBRARY_PATH", [os.path.expanduser("~/lib"), "/usr/local/lib", "/usr/lib"]
    elif os.name == "nt":
        variable, fallback = "PATH", []
    else:
        variable, fallback = "LD_LIBRARY_PATH", []
    existing = [p for p in env.get(variable, "").split(os.pathsep) if p] or fallback
    env[variable] = os.pathsep.join(paths + existing)
    return env


def without_harness(manifest):
    """The names of a package's targets that have their own main (`harness = false`)."""
    names = set()
    try:
        text = pathlib.Path(manifest).read_text()
    except OSError:
        return names
    for block in re.split(r"(?m)^\[", text):
        if re.search(r"(?m)^\s*harness\s*=\s*false", block):
            found = re.search(r'(?m)^\s*name\s*=\s*"([^"]+)"', block)
            if found:
                names.add(found.group(1))
    return names


def listed(program, env):
    """The names of a program's tests, or None when it has no test harness to ask."""
    if program["name"] in without_harness(program["manifest"]):
        return None
    try:
        out = subprocess.run(
            [program["executable"], "--list", "--format", "terse"],
            cwd=str(pathlib.Path(program["manifest"]).parent),
            env=env,
            capture_output=True,
            text=True,
            timeout=120,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if out.returncode != 0:
        return None
    names = []
    for line in out.stdout.splitlines():
        if line.endswith(": test") or line.endswith(": benchmark"):
            names.append(line.rsplit(": ", 1)[0])
        elif line.strip():
            return None
    return names


def shown(program, target_directory):
    """The line cargo prints before a test program, so that existing log readers find it."""
    relative = os.path.relpath(program["executable"], os.path.dirname(target_directory))
    manifest_folder = os.path.dirname(program["manifest"])
    source = os.path.relpath(program["source"], manifest_folder) if program["source"] else program["name"]
    prefix = "" if program["kind"] == "test" else "unittests "
    return f"     Running {prefix}{source} ({relative})"


def selections(arguments):
    """The `cargo test` argument lists, one per selection, separated by `---`."""
    found, current = [], []
    for argument in arguments:
        if argument == "---":
            found.append(current)
            current = []
        else:
            current.append(argument)
    found.append(current)
    if any(not selection for selection in found):
        sys.exit("parallel-tests: an empty selection (nothing before or after a `---`)")
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--jobs", type=int, default=int(os.environ.get("PARALLEL_TEST_JOBS") or 0),
                        help="test processes at a time")
    parser.add_argument("--split", action="append", default=[], metavar="NAME",
                        help="a test target whose tests may run in several processes (see the notes above)")
    parser.add_argument("--first", action="append", default=[], metavar="TEST",
                        help="a test that every slice of a split program runs first (see the notes above)")
    parser.add_argument("--chunk", type=int, default=int(os.environ.get("PARALLEL_TEST_CHUNK") or 40),
                        help="most tests one process of a split program is given (default 40)")
    parser.add_argument("--timeout", type=int, default=45 * 60, help="seconds one process may run")
    parser.add_argument("--retry", type=int, choices=(0, 1), default=None,
                        help="second runs a failed process gets (default 1, or PARALLEL_TEST_RETRY)")
    parser.add_argument("--log", default=None, help="folder for one log per process")
    parser.add_argument("cargo", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    arguments = options.cargo[1:] if options.cargo[:1] == ["--"] else options.cargo
    if not arguments:
        parser.error("no cargo test arguments")
    jobs = options.jobs or default_jobs()
    retry = options.retry if options.retry is not None else int(os.environ.get("PARALLEL_TEST_RETRY") or 1)
    split = {name for value in options.split for name in value.split(",") if name}
    first = {name for value in options.first for name in value.split(",") if name}
    folder = os.getcwd()

    chosen_sets = []
    for number, selection in enumerate(selections(arguments)):
        cargo_arguments, test_arguments = split_arguments(selection)
        if not any(a in TARGETS or a.split("=")[0] in TARGETS for a in cargo_arguments):
            sys.exit("parallel-tests: name the targets (--lib, --tests, --all-targets, ...). Without one, cargo "
                     "test also runs documentation tests, and this tool does not.")
        filters, skips, exact, rest = test_selection(test_arguments)
        chosen_sets.append({"number": number, "cargo": cargo_arguments, "tests": test_arguments, "filters": filters,
                            "skips": skips, "exact": exact, "rest": rest})

    started = time.time()
    for chosen in chosen_sets:
        chosen["programs"] = build(chosen["cargo"], folder)
    by_manifest, target_directory = packages(folder, chosen_sets[0]["cargo"])
    sysroot = subprocess.run(["rustc", "--print", "sysroot"], capture_output=True, text=True).stdout.strip()
    built = time.time() - started

    log_folder = pathlib.Path(options.log or os.path.join(os.environ.get("RUNNER_TEMP") or target_directory,
                                                           f"parallel-tests-{os.getpid()}"))
    log_folder.mkdir(parents=True, exist_ok=True)

    # One unit of work is one process: a whole test program, or one slice of a program named by --split.
    units = []
    listed_total = 0
    program_total = 0
    repeated = 0
    order = 0
    for chosen in chosen_sets:
        for program in chosen["programs"]:
            order += 1
            program_total += 1
            env = environment(program, by_manifest.get(program["manifest"]), sysroot)
            base = {"program": program, "env": env, "order": order, "set": chosen, "part": 0, "parts": 1}
            names = listed(program, env)
            if names is None:
                units.append({**base, "names": None, "expected": None, "weight": 1})
                continue
            wanted = [n for n in names if selected(n, chosen["filters"], chosen["skips"], chosen["exact"])]
            listed_total += len(wanted)
            if not wanted:
                # cargo would start the program and it would report that it ran nothing.
                continue
            if program["name"] not in split:
                units.append({**base, "names": None, "expected": len(wanted), "weight": len(wanted)})
                continue
            # The program's own first test goes into every slice, whatever the selection says.
            lead = [n for n in names if n in first]
            if lead and (len(lead) > 1 or names[0] != lead[0]):
                sys.exit(f"parallel-tests: {program['name']}: {lead[0]} must be the one test that runs first, but "
                         f"{names[0]} sorts before it. Rename it, or the test that now sorts first.")
            rest = [n for n in wanted if n not in lead]
            slices = [rest[i : i + options.chunk] for i in range(0, len(rest), options.chunk)] or [[]]
            repeated += len(lead) * len(slices) - len([n for n in wanted if n in lead])
            for part, piece in enumerate(slices):
                units.append({**base, "names": lead + piece, "expected": len(lead) + len(piece),
                              "weight": len(wanted), "part": part, "parts": len(slices)})

    # The programs with most tests first, so that the long ones do not start last.
    work = queue.Queue()
    for unit in sorted(units, key=lambda u: (-u["weight"], u["order"], u["part"])):
        work.put(unit)

    sliced = sorted({u["program"]["name"] for u in units if u["parts"] > 1})
    print(f"parallel-tests: {program_total} test programs, {listed_total} tests, {len(units)} processes, "
          f"{jobs} at a time" + (f"; in slices of at most {options.chunk} tests: {', '.join(sliced)}" if sliced else "")
          + (f"; {repeated} more runs of a program's first test, one per slice" if repeated else ""),
          flush=True)
    if not units:
        print("::warning::parallel-tests: this selection matches no test", flush=True)

    lock = threading.Lock()
    done = []

    def attempt(unit, second=False):
        program, chosen = unit["program"], unit["set"]
        command = [program["executable"]]
        if unit["names"] is None:
            command += chosen["tests"]
        else:
            command += ["--exact", *unit["names"], *chosen["rest"]]
        label = os.path.basename(program["executable"])
        if len(chosen_sets) > 1:
            label += f".set{chosen['number'] + 1}"
        if unit["parts"] > 1:
            label += f".{unit['part'] + 1}of{unit['parts']}"
        path = log_folder / (label + (".second.log" if second else ".log"))
        begin = time.time()
        timed_out = False
        with open(path, "w") as stream:
            process = subprocess.Popen(command, cwd=os.path.dirname(program["manifest"]), env=unit["env"],
                                       stdout=stream, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL)
            try:
                status = process.wait(timeout=options.timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                process.kill()
                status = process.wait()
        text = path.read_text(errors="replace")
        counts = [0, 0, 0]
        for found in RESULT.finditer(text):
            counts = [counts[0] + int(found.group(2)), counts[1] + int(found.group(3)), counts[2] + int(found.group(4))]
        problem = ""
        if timed_out:
            problem = f"no end after {options.timeout} s, stopped"
        elif status != 0:
            problem = f"exit status {status}"
        elif unit["expected"] is not None and sum(counts) < unit["expected"] and "--ignored" not in chosen["rest"]:
            # More is fine: a test that starts its own program again for one test adds that child's report.
            problem = f"{sum(counts)} tests reported, {unit['expected']} selected"
        record = {"unit": unit, "label": label, "seconds": time.time() - begin, "status": status, "counts": counts,
                  "problem": problem, "text": text, "timed_out": timed_out}
        with lock:
            if not second:
                done.append(record)
            print(shown(program, target_directory)
                  + (f"  [slice {unit['part'] + 1} of {unit['parts']}]" if unit["parts"] > 1 else "")
                  + ("  [second run]" if second else ""))
            print(text.rstrip("\n"))
            if problem:
                print(f"error: test process {label}: {'second run: ' if second else ''}{problem}")
            print(f"parallel-tests: {label} {'second run ' if second else ''}{'FAILED' if problem else 'ok'} "
                  f"in {record['seconds']:.1f} s" + ("" if second else f" ({len(done)} of {len(units)})"), flush=True)
        return record

    def worker():
        while True:
            try:
                unit = work.get_nowait()
            except queue.Empty:
                return
            try:
                attempt(unit)
            except Exception as error:  # a process that could not be started is a failure, not a crash of the run
                with lock:
                    done.append({"unit": unit, "label": os.path.basename(unit["program"]["executable"]),
                                 "seconds": 0.0, "status": None, "counts": [0, 0, 0],
                                 "problem": f"could not run: {error}", "text": "", "timed_out": False})
                    print(f"error: {unit['program']['executable']}: {error}", flush=True)

    running = time.time()
    threads = [threading.Thread(target=worker) for _ in range(max(1, jobs))]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    ran = time.time() - running

    # The second runs, one after another now that nothing else is running.
    once_more = [r for r in done if r["problem"] and r["status"] not in (None, 0) and not r["timed_out"]]
    again = 0.0
    if retry and len(once_more) > SECOND_RUNS_AT_MOST:
        print(f"parallel-tests: {len(once_more)} processes failed, more than {SECOND_RUNS_AT_MOST}: "
              "none gets a second run", flush=True)
    elif retry:
        for record in once_more:
            print(f"parallel-tests: {record['label']} failed, running it once more", flush=True)
            try:
                record["again"] = attempt(record["unit"], second=True)
            except Exception as error:
                record["again"] = {**record, "seconds": 0.0, "problem": f"could not run: {error}", "text": ""}
                print(f"error: {record['unit']['program']['executable']}: second run: {error}", flush=True)
            again += record["again"]["seconds"]

    def total(records, index):
        return sum(r["counts"][index] for r in records)

    def failed_names(record):
        return [f"[{record['unit']['program']['name']}] {name}" for name in FAILED_TEST.findall(record["text"])]

    serial = sum(r["seconds"] for r in done)
    # What counts for a process is its last run.
    first_runs, done = done, [r.get("again", r) for r in done]
    second_only = [r for r in first_runs if "again" in r and not r["again"]["problem"]]
    bad = [r for r in done if r["problem"]]
    failed_tests = []
    for record in done:
        failed_tests += failed_names(record)
    lines = [""]
    if len(chosen_sets) > 1:
        for chosen in chosen_sets:
            own = [r for r in done if r["unit"]["set"] is chosen]
            lines.append(f"parallel-tests: selection {chosen['number'] + 1} ({' '.join(chosen['cargo'])}): "
                         f"{total(own, 0)} passed; {total(own, 1)} failed; {total(own, 2)} ignored; "
                         f"{len(chosen['programs'])} test programs")
    lines += [
        f"parallel-tests: {total(done, 0)} passed; {total(done, 1)} failed; {total(done, 2)} ignored; "
        f"{program_total} test programs in {len(units)} processes, {jobs} at a time",
        f"parallel-tests: build {built / 60:.1f} min, tests {ran / 60:.1f} min "
        f"(one after another they took {serial / 60:.1f} min)"
        + (f", second runs {again / 60:.1f} min" if again else ""),
    ]
    slow = sorted(first_runs, key=lambda r: -r["seconds"])[:8]
    lines.append("parallel-tests: longest processes: " + ", ".join(f"{r['label']} {r['seconds']:.0f} s" for r in slow))
    if failed_tests:
        lines += [f"parallel-tests: failed tests ({len(failed_tests)}):"] + [f"  {t}" for t in failed_tests]
    for record in bad:
        lines.append(f"parallel-tests: FAILED process {record['label']}: {record['problem']}")
    if second_only:
        lines.append(f"parallel-tests: passed only on a second run ({len(second_only)}):")
        for record in second_only:
            names = failed_names(record) or [f"[{record['unit']['program']['name']}] no test named"]
            lines += [f"  {name} ({record['label']}: {record['problem']})" for name in names]
            for name in names:
                print(f"::warning title=Passed only on a second run::{name} failed in {record['label']} "
                      f"({record['problem']}) and passed when that process ran once more.", flush=True)
    print("\n".join(lines), flush=True)
    (log_folder / "timings.json").write_text(json.dumps(
        [{"process": r["label"], "seconds": round(r["seconds"], 1), "counts": r["counts"], "problem": r["problem"],
          **({"second_run": {"seconds": round(r["again"]["seconds"], 1), "problem": r["again"]["problem"],
                             "failed_first": failed_names(r)}} if "again" in r else {})}
         for r in sorted(first_runs, key=lambda r: -r["seconds"])], indent=1))
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as stream:
            stream.write("```\n" + "\n".join(lines).strip() + "\n```\n")
    sys.exit(1 if bad or total(done, 1) else 0)


if __name__ == "__main__":
    main()
