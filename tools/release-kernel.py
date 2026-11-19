#!/usr/bin/env python3
"""Keep the Octos kernel a release run builds, and take it in later runs.

Every installer carries one Octos kernel, built from the revision pinned in
packaging/octos.lock.json. Building it takes about 25 minutes of an installer
job and gives the same program for as long as five things stay the same: the
pinned revision, the build command, the target, the compiler and the kind of
runner. A release run therefore keeps the kernel it built as a workflow
artifact named after those five, and a later run with the same five takes it.

    find    name the kernel this job needs and look for a kept one
    accept  check the downloaded kernel and hand it to tools/package-octos.py
    keep    set aside the kernel this job built, to be uploaded

A kept kernel is taken only from a run of the release workflow itself, started
by hand or by a version tag, in this repository, on the main branch or on the
ref the current run was started on. A pull request, also one from a fork, can
upload an artifact under any name. Such an artifact is never taken: its run is
none of the above.

Taking a kernel only saves time. Whatever is missing or does not fit is named
in the log, and the job builds the kernel from source as it did before.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ".github/workflows/release.yml"
TRUSTED_BRANCH = "main"
RECEIPT = "kernel.json"

spec = importlib.util.spec_from_file_location("package_octos", Path(__file__).with_name("package-octos.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


def describe(runner, target=None, compiler=None):
    """The five things a kept kernel and the job that takes it must share."""
    lock = package.read_lock()
    target = (target or os.environ.get("RINX_KERNEL_TARGET") or os.environ.get("CARGO_BUILD_TARGET")
              or package.host_target())
    return {
        "revision": lock["revision"],
        "command": package.build_args(lock, target),
        "target": target,
        "compiler": compiler or subprocess.check_output(["rustc", "-V"], text=True).strip(),
        "runner": runner,
    }


def artifact_name(description):
    digest = hashlib.sha256(json.dumps(description, sort_keys=True).encode()).hexdigest()
    return "octos-kernel-" + description["target"] + "-" + digest[:16]


def kernel_file(description):
    return "octos.exe" if "windows" in description["target"] else "octos"


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def api(path):
    request = urllib.request.Request(
        os.environ.get("GITHUB_API_URL", "https://api.github.com") + path,
        headers={"Authorization": "Bearer " + os.environ["GITHUB_TOKEN"],
                 "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def refuse_artifact(artifact, repository_id, refs):
    """Why this artifact is not taken, judged by what the listing says of it."""
    made = artifact.get("workflow_run") or {}
    if artifact.get("expired"):
        return "it has expired"
    if made.get("repository_id") != repository_id or made.get("head_repository_id") != repository_id:
        return "its run belongs to another repository"
    if made.get("head_branch") not in refs:
        return "its run was on " + str(made.get("head_branch"))
    return None


def refuse_run(run, artifact, repository_id, refs):
    """Why this artifact is not taken, judged by the run that uploaded it."""
    if run.get("id") != (artifact.get("workflow_run") or {}).get("id"):
        return "its run could not be read"
    if run.get("path") != WORKFLOW:
        return "its run is not the release workflow"
    if run.get("event") not in ("workflow_dispatch", "push"):
        return "its run was started by " + str(run.get("event"))
    if run.get("head_branch") not in refs:
        return "its run was on " + str(run.get("head_branch"))
    owners = ((run.get("repository") or {}).get("id"), (run.get("head_repository") or {}).get("id"))
    if owners != (repository_id, repository_id):
        return "its run belongs to another repository"
    return None


def find(name, repository, repository_id, refs, fetch=api, say=print):
    """The newest kept kernel with this name that may be taken: (artifact, run), or None."""
    listing = fetch("/repos/" + repository + "/actions/artifacts?per_page=100&name=" + urllib.parse.quote(name))
    kept = sorted(listing.get("artifacts") or [], key=lambda a: a.get("created_at") or "", reverse=True)
    for artifact in kept:
        run_id = (artifact.get("workflow_run") or {}).get("id")
        reason = refuse_artifact(artifact, repository_id, refs)
        if not reason:
            reason = refuse_run(fetch("/repos/" + repository + "/actions/runs/" + str(run_id)),
                                artifact, repository_id, refs)
        if reason:
            say("Artifact %s of run %s is not taken: %s." % (artifact.get("id"), run_id, reason))
            continue
        return artifact["id"], run_id
    return None


def accept(directory, description, from_run):
    """The kernel in a downloaded artifact, or the reason it is not used."""
    receipts = sorted(Path(directory).rglob(RECEIPT)) if Path(directory).is_dir() else []
    if len(receipts) != 1:
        return None, "the download holds %d kernel records instead of one" % len(receipts)
    try:
        receipt = json.loads(receipts[0].read_text())
    except ValueError:
        return None, "its record cannot be read"
    if not isinstance(receipt, dict) or receipt.get("description") != description:
        return None, "it was built for something other than this job needs"
    if str(receipt.get("builtByRun")) != str(from_run):
        return None, "its record names run %s, the artifact belongs to run %s" % (receipt.get("builtByRun"), from_run)
    kernel = receipts[0].with_name(kernel_file(description))
    if not kernel.is_file() or sha(kernel) != receipt.get("sha256"):
        return None, "the file does not have the checksum in its record"
    # Artifacts do not keep the permission to run a file.
    kernel.chmod(0o755)
    return kernel.resolve(), None


def keep(description, out, run, commit, runtime=None):
    """Copy the kernel this job built to `out`, with its record. Returns the
    reason when there is nothing to keep."""
    runtime = runtime or ROOT / "dist/runtime"
    staged = runtime / ("octos-" + description["target"] + (".exe" if "windows" in description["target"] else ""))
    record = Path(str(staged) + ".json")
    if not staged.is_file() or not record.is_file():
        return "no kernel was packaged"
    record = json.loads(record.read_text())
    if record.get("source") != "pinned-source":
        return "this job took its kernel from release run %s" % record.get("builtByRun")
    if record.get("revision") != description["revision"] or sha(staged) != record.get("sha256"):
        return "the packaged kernel is not the one its record describes"
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    shutil.copy2(staged, out / kernel_file(description))
    (out / RECEIPT).write_text(json.dumps({
        "description": description, "sha256": record["sha256"],
        "builtByRun": run, "builtFromCommit": commit,
    }, indent=2) + "\n")
    return None


def publish(variable, pairs):
    """Give values to later steps: `variable` is GITHUB_OUTPUT or GITHUB_ENV."""
    if os.environ.get(variable):
        with open(os.environ[variable], "a") as stream:
            for key, value in pairs.items():
                stream.write("%s=%s\n" % (key, value))
    else:
        print(json.dumps(pairs))


def from_source(reason):
    print("::warning title=Octos kernel built from source::" + reason[:1].upper() + reason[1:]
          + ". This job builds the kernel from source.", flush=True)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["find", "accept", "keep"])
    parser.add_argument("--runner", required=True, help="The kind of runner this job is on, as the workflow names it")
    parser.add_argument("--dir", type=Path, help="accept: where the artifact was downloaded to")
    parser.add_argument("--from-run", help="accept: the run `find` named")
    parser.add_argument("--out", type=Path, help="keep: where to put the kernel for upload")
    args = parser.parse_args(argv)
    description = describe(args.runner)
    name = artifact_name(description)
    if args.command == "find":
        print("This job needs " + name + ":\n" + json.dumps(description, indent=2), flush=True)
        try:
            refs = {TRUSTED_BRANCH, os.environ["GITHUB_REF_NAME"]}
            found = find(name, os.environ["GITHUB_REPOSITORY"], int(os.environ["GITHUB_REPOSITORY_ID"]), refs)
        except Exception as error:  # A lookup that fails must not fail a release.
            return from_source("the kept kernels could not be listed (%s: %s)" % (type(error).__name__, error))
        if not found:
            print("No release run has kept this kernel. This job builds it from source and keeps it.", flush=True)
            return 0
        print("Taking artifact %s of release run %s." % found, flush=True)
        publish("GITHUB_OUTPUT", {"artifact_id": found[0], "run_id": found[1]})
    elif args.command == "accept":
        kernel, reason = accept(args.dir, description, args.from_run)
        if reason:
            return from_source("the kernel kept by run %s is not used: %s" % (args.from_run, reason))
        print("The kernel of release run %s is used: %s" % (args.from_run, kernel), flush=True)
        publish("GITHUB_ENV", {package.RELEASE_KERNEL: kernel, package.RELEASE_KERNEL_RUN: args.from_run})
    else:
        reason = keep(description, args.out, os.environ.get("GITHUB_RUN_ID"), os.environ.get("GITHUB_SHA"))
        if reason:
            print("Nothing to keep: " + reason + ".", flush=True)
            return 0
        print("Kept for later release runs as " + name + ".", flush=True)
        publish("GITHUB_OUTPUT", {"name": name})
    return 0


if __name__ == "__main__":
    sys.exit(main())
