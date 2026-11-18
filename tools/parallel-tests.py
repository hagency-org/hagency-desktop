#!/usr/bin/env python3
"""Run one `cargo test` selection as several test processes side by side.

    python3 tools/parallel-tests.py [--jobs N] [--chunk N] [--log DIR] -- <cargo test arguments>

The arguments after the first `--` are those of `cargo test`, including a
second `--` and the arguments of the test programs. `--no-fail-fast` is
implied: every test runs, and the exit status is 1 when anything failed.

What is the same as `cargo test`: one build with the same arguments; the same
test programs; every test of every program; each program started in its
package's folder with the variables cargo sets; the tests inside one process
run one after another when RUST_TEST_THREADS=1 or `--test-threads=1` says so.

What is different: a program with many tests is started several times, each
time for one slice of its tests (named exactly), and up to N processes run at
once. A test therefore shares its process with fewer tests than before, and
other test processes run on the machine at the same time.

Why: the suites are serial because the tests of one process share that
process's stores and deadlines. That leaves all cores but one idle for most of
a job. Separate processes share none of that.
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


def default_jobs():
    cores = os.cpu_count() or 2
    return cores if cores <= 4 else max(4, cores // 2)


# The options of `cargo test` whose value is the next argument.
CARGO_VALUES = {"-p", "--package", "--exclude", "-F", "--features", "--manifest-path", "--target", "--target-dir",
                "--profile", "--test", "--bin", "--example", "--bench", "-j", "--jobs", "--color", "--config",
                "-Z", "--message-format", "--lockfile-path"}


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


def environment(program, package, target_directory, sysroot):
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


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--jobs", type=int, default=int(os.environ.get("PARALLEL_TEST_JOBS") or 0))
    parser.add_argument("--chunk", type=int, default=int(os.environ.get("PARALLEL_TEST_CHUNK") or 40),
                        help="most tests one process is given (default 40)")
    parser.add_argument("--timeout", type=int, default=45 * 60, help="seconds one process may run")
    parser.add_argument("--log", default=None, help="folder for one log per process")
    parser.add_argument("cargo", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    arguments = options.cargo[1:] if options.cargo[:1] == ["--"] else options.cargo
    if not arguments:
        parser.error("no cargo test arguments")
    jobs = options.jobs or default_jobs()
    folder = os.getcwd()
    cargo_arguments, test_arguments = split_arguments(arguments)
    filters, skips, exact, rest = test_selection(test_arguments)
    targets = ("--lib", "--bins", "--bin", "--tests", "--test", "--examples", "--example", "--benches", "--bench",
               "--all-targets")
    if not any(a in targets or a.split("=")[0] in targets for a in cargo_arguments):
        sys.exit("parallel-tests: name the targets (--lib, --tests, --all-targets, ...). Without one, cargo test "
                 "also runs documentation tests, and this tool does not.")

    started = time.time()
    programs = build(cargo_arguments, folder)
    by_manifest, target_directory = packages(folder, cargo_arguments)
    sysroot = subprocess.run(["rustc", "--print", "sysroot"], capture_output=True, text=True).stdout.strip()
    built = time.time() - started

    log_folder = pathlib.Path(options.log or os.path.join(os.environ.get("RUNNER_TEMP") or target_directory,
                                                           f"parallel-tests-{os.getpid()}"))
    log_folder.mkdir(parents=True, exist_ok=True)

    # One unit of work is one process: a slice of a program's tests, or a whole program without a harness.
    units = []
    listed_total = 0
    for number, program in enumerate(programs):
        env = environment(program, by_manifest.get(program["manifest"]), target_directory, sysroot)
        names = listed(program, env)
        if names is None:
            units.append({"program": program, "env": env, "names": None, "order": number, "part": 0, "parts": 1})
            continue
        chosen = [n for n in names if selected(n, filters, skips, exact)]
        listed_total += len(chosen)
        if not chosen:
            # cargo would start the program and it would report that it ran nothing.
            continue
        slices = [chosen[i : i + options.chunk] for i in range(0, len(chosen), options.chunk)]
        for part, piece in enumerate(slices):
            units.append({"program": program, "env": env, "names": piece, "order": number, "part": part,
                          "parts": len(slices), "filtered": len(names) - len(chosen)})

    # The programs with most tests first, so that the long ones do not start last.
    weight = {}
    for unit in units:
        weight[unit["order"]] = weight.get(unit["order"], 0) + (len(unit["names"]) if unit["names"] else 1)
    work = queue.Queue()
    for unit in sorted(units, key=lambda u: (-weight[u["order"]], u["order"], u["part"])):
        work.put(unit)

    print(f"parallel-tests: {len(programs)} test programs, {listed_total} tests, {len(units)} processes, "
          f"{jobs} at a time, at most {options.chunk} tests each", flush=True)
    if not units:
        print("::warning::parallel-tests: this selection matches no test", flush=True)

    lock = threading.Lock()
    done = []

    def run(unit):
        program = unit["program"]
        command = [program["executable"]]
        if unit["names"] is None:
            command += test_arguments
        else:
            command += ["--exact", *unit["names"], *rest]
        label = f"{os.path.basename(program['executable'])}.{unit['part'] + 1}of{unit['parts']}"
        path = log_folder / (label + ".log")
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
        expected = len(unit["names"]) if unit["names"] is not None else None
        problem = ""
        if timed_out:
            problem = f"no end after {options.timeout} s, stopped"
        elif status != 0:
            problem = f"exit status {status}"
        elif expected is not None and sum(counts) != expected and "--ignored" not in rest:
            problem = f"{sum(counts)} tests reported, {expected} named"
        record = {"unit": unit, "label": label, "seconds": time.time() - begin, "status": status, "counts": counts,
                  "problem": problem, "text": text}
        with lock:
            done.append(record)
            print(shown(program, target_directory)
                  + (f"  [slice {unit['part'] + 1} of {unit['parts']}]" if unit["parts"] > 1 else ""))
            print(text.rstrip("\n"))
            if problem:
                print(f"error: test process {label}: {problem}")
            print(f"parallel-tests: {label} {'FAILED' if problem else 'ok'} in {record['seconds']:.1f} s "
                  f"({len(done)} of {len(units)})", flush=True)

    def worker():
        while True:
            try:
                unit = work.get_nowait()
            except queue.Empty:
                return
            try:
                run(unit)
            except Exception as error:  # a process that could not be started is a failure, not a crash of the run
                with lock:
                    done.append({"unit": unit, "label": os.path.basename(unit["program"]["executable"]),
                                 "seconds": 0.0, "status": -1, "counts": [0, 0, 0],
                                 "problem": f"could not run: {error}", "text": ""})
                    print(f"error: {unit['program']['executable']}: {error}", flush=True)

    running = time.time()
    threads = [threading.Thread(target=worker) for _ in range(max(1, jobs))]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    ran = time.time() - running

    passed = sum(r["counts"][0] for r in done)
    failed = sum(r["counts"][1] for r in done)
    ignored = sum(r["counts"][2] for r in done)
    serial = sum(r["seconds"] for r in done)
    bad = [r for r in done if r["problem"]]
    failed_tests = []
    for record in done:
        for name in re.findall(r"^test (\S+) \.\.\. FAILED", record["text"], re.M):
            failed_tests.append(f"[{record['unit']['program']['name']}] {name}")
    lines = [
        "",
        f"parallel-tests: {passed} passed; {failed} failed; {ignored} ignored; {len(programs)} test programs "
        f"in {len(units)} processes, {jobs} at a time",
        f"parallel-tests: build {built / 60:.1f} min, tests {ran / 60:.1f} min "
        f"(one after another they took {serial / 60:.1f} min)",
    ]
    slow = sorted(done, key=lambda r: -r["seconds"])[:8]
    lines.append("parallel-tests: longest processes: " + ", ".join(f"{r['label']} {r['seconds']:.0f} s" for r in slow))
    if failed_tests:
        lines += [f"parallel-tests: failed tests ({len(failed_tests)}):"] + [f"  {t}" for t in failed_tests]
    for record in bad:
        lines.append(f"parallel-tests: FAILED process {record['label']}: {record['problem']}")
    print("\n".join(lines), flush=True)
    (log_folder / "timings.json").write_text(json.dumps(
        [{"process": r["label"], "seconds": round(r["seconds"], 1), "counts": r["counts"], "problem": r["problem"]}
         for r in sorted(done, key=lambda r: -r["seconds"])], indent=1))
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as stream:
            stream.write("```\n" + "\n".join(lines).strip() + "\n```\n")
    sys.exit(1 if bad or failed else 0)


if __name__ == "__main__":
    main()
