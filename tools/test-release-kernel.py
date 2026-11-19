"""Checks of how a release run keeps and takes the Octos kernel (no compiler, no network)."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release_kernel", Path(__file__).with_name("release-kernel.py"))
kernel = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kernel)

REPOSITORY = 1403847449
MAC = "aarch64-apple-darwin"
WINDOWS = "x86_64-pc-windows-msvc"


def description(**changes):
    made = kernel.describe("macos-15", target=MAC, compiler="rustc 1.98.0 (stand-in)")
    made.update(changes)
    return made


def artifact(number, run, branch="main", created="2026-11-19T04:00:00Z", **changes):
    made = {"id": number, "expired": False, "created_at": created,
            "workflow_run": {"id": run, "repository_id": REPOSITORY, "head_repository_id": REPOSITORY,
                             "head_branch": branch}}
    made.update(changes)
    return made


def run(number, branch="main", event="workflow_dispatch", path=kernel.WORKFLOW, head=REPOSITORY):
    return {"id": number, "path": path, "event": event, "head_branch": branch,
            "repository": {"id": REPOSITORY}, "head_repository": {"id": head}}


class Api:
    """Stands in for the two questions `find` asks of GitHub."""
    def __init__(self, artifacts, runs, answers=None):
        self.artifacts, self.runs, self.asked = artifacts, {r["id"]: r for r in runs}, []
        self.runs.update(answers or {})  # What is answered when a run is asked for by number.

    def __call__(self, path):
        self.asked.append(path)
        if "/actions/artifacts?" in path:
            return {"artifacts": self.artifacts}
        return self.runs.get(int(path.rsplit("/", 1)[1]), {})


def find(api, refs=("main",)):
    said = []
    found = kernel.find("octos-kernel-x", "owner/name", REPOSITORY, set(refs), fetch=api, say=said.append)
    return found, said


class Naming(unittest.TestCase):
    def test_the_name_follows_each_of_the_five_things(self):
        name = kernel.artifact_name(description())
        self.assertEqual(name, kernel.artifact_name(description()))
        self.assertTrue(name.startswith("octos-kernel-" + MAC + "-"))
        for change in ({"revision": "0" * 40}, {"target": WINDOWS}, {"runner": "macos-26"},
                       {"compiler": "rustc 1.99.0 (stand-in)"},
                       {"command": description()["command"] + ["--features", "more"]}):
            with self.subTest(change=list(change)):
                self.assertNotEqual(name, kernel.artifact_name(description(**change)))

    def test_the_build_command_in_the_name_is_the_one_that_is_run(self):
        lock = kernel.package.read_lock()
        self.assertEqual(description()["command"], kernel.package.build_args(lock, MAC))
        self.assertEqual(description()["revision"], lock["revision"])


class Finding(unittest.TestCase):
    def test_the_newest_kernel_of_a_release_run_on_main_is_taken(self):
        api = Api([artifact(1, 10, created="2026-11-18T00:00:00Z"), artifact(2, 20, created="2026-11-19T00:00:00Z")],
                  [run(10), run(20)])
        self.assertEqual(find(api)[0], (2, 20))
        self.assertIn("name=octos-kernel-x", api.asked[0])

    def test_nothing_kept_means_nothing_taken(self):
        self.assertEqual(find(Api([], [])), (None, []))

    def test_an_artifact_that_is_not_a_main_release_run_s_is_passed_over(self):
        cases = {
            "expired": (artifact(1, 10, expired=True), run(10)),
            "a fork's": (artifact(1, 10, workflow_run={"id": 10, "repository_id": REPOSITORY,
                                                       "head_repository_id": 7, "head_branch": "main"}), run(10)),
            "another branch": (artifact(1, 10, branch="ci/experiment"), run(10, branch="ci/experiment")),
            "a pull request": (artifact(1, 10), run(10, event="pull_request")),
            "another workflow": (artifact(1, 10), run(10, path=".github/workflows/main.yml")),
            "a run of a fork that the listing calls ours": (artifact(1, 10), run(10, head=7)),
            "a run on a branch that the listing calls main": (artifact(1, 10), run(10, branch="ci/experiment")),
            "a run that cannot be read": (artifact(1, 10), {}),
            "a sound run that is not the one asked for": (artifact(1, 10), run(11)),
        }
        for case, (kept, made_by) in cases.items():
            with self.subTest(case=case):
                # An older, sound kernel behind it is still found.
                api = Api([kept, artifact(2, 20, created="2026-11-01T00:00:00Z")], [run(20)], answers={10: made_by})
                found, said = find(api)
                self.assertEqual(found, (2, 20))
                self.assertEqual(len(said), 1)
                self.assertIn("Artifact 1 of run 10 is not taken", said[0])
                # What the listing already rules out is not asked about.
                asked_about_it = any(path.endswith("/actions/runs/10") for path in api.asked)
                self.assertEqual(asked_about_it, case not in ("expired", "a fork's", "another branch"))

    def test_a_run_on_a_branch_takes_that_branch_s_kernel_and_main_does_not(self):
        api = Api([artifact(1, 10, branch="ci/experiment")], [run(10, branch="ci/experiment")])
        self.assertEqual(find(api, refs=("main", "ci/experiment"))[0], (1, 10))
        self.assertIsNone(find(api, refs=("main",))[0])

    def test_a_version_tag_s_run_is_taken_only_for_that_tag(self):
        api = Api([artifact(1, 10, branch="v0.5.0")], [run(10, branch="v0.5.0", event="push")])
        self.assertEqual(find(api, refs=("main", "v0.5.0"))[0], (1, 10))
        self.assertIsNone(find(api, refs=("main", "v0.5.1"))[0])

    def test_a_listing_that_fails_does_not_fail_the_job(self):
        saved = {key: os.environ.get(key) for key in ("GITHUB_REF_NAME", "GITHUB_OUTPUT", "GITHUB_TOKEN")}
        try:
            os.environ.pop("GITHUB_TOKEN", None)
            os.environ.pop("GITHUB_REF_NAME", None)
            with tempfile.TemporaryDirectory() as temporary:
                os.environ["GITHUB_OUTPUT"] = str(Path(temporary) / "output")
                wanted = description()
                describe, kernel.describe = kernel.describe, lambda runner: wanted
                # What it prints is held back: a runner would show the warning on the job that runs this check.
                printed = io.StringIO()
                try:
                    with contextlib.redirect_stdout(printed):
                        self.assertEqual(kernel.main(["find", "--runner", "macos-15"]), 0)
                finally:
                    kernel.describe = describe
                self.assertIn("::warning title=Octos kernel built from source::", printed.getvalue())
                self.assertIn("could not be listed", printed.getvalue())
                self.assertFalse(Path(os.environ["GITHUB_OUTPUT"]).exists())
        finally:
            for key, value in saved.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value


class KeepingAndTaking(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.runtime = self.root / "runtime"
        self.runtime.mkdir()
        (self.root / "LICENSE").write_text("Test license\n")

    def stage(self, wanted, content=b"a kernel", **options):
        built = self.root / "built"
        built.write_bytes(content)
        return kernel.package.stage(built, kernel.package.read_lock(), wanted["target"], self.root,
                                    self.runtime, **options)

    def kept(self, wanted, **options):
        self.stage(wanted, built_by_run="100", **options)
        out = Path(tempfile.mkdtemp(dir=self.root)) / "upload"
        self.assertIsNone(kernel.keep(wanted, out, "100", "c" * 40, runtime=self.runtime))
        for file in out.iterdir():  # As after an upload and a download.
            file.chmod(0o644)
        return out

    def test_a_kept_kernel_is_taken_and_can_be_run(self):
        for wanted in (description(), description(target=WINDOWS, runner="windows-2022")):
            with self.subTest(target=wanted["target"]):
                out = self.kept(wanted)
                file = "octos.exe" if wanted["target"] == WINDOWS else "octos"
                self.assertEqual(sorted(p.name for p in out.iterdir()), sorted([file, "kernel.json"]))
                taken, reason = kernel.accept(out, wanted, "100")
                self.assertIsNone(reason)
                self.assertEqual(taken, (out / file).resolve())
                self.assertEqual(taken.read_bytes(), b"a kernel")
                if os.name != "nt":
                    self.assertTrue(taken.stat().st_mode & stat.S_IXUSR)
                record = json.loads((out / "kernel.json").read_text())
                self.assertEqual(record["sha256"], hashlib.sha256(b"a kernel").hexdigest())
                self.assertEqual((record["builtByRun"], record["builtFromCommit"]), ("100", "c" * 40))

    def test_a_kernel_in_a_folder_of_the_download_is_found(self):
        out = self.kept(description())
        nested = self.root / "download" / "octos-kernel-x"
        nested.parent.mkdir()
        out.rename(nested)
        (self.root / "download" / "another-file").write_text("beside it\n")
        self.assertIsNone(kernel.accept(self.root / "download", description(), "100")[1])

    def test_a_kernel_that_does_not_fit_is_not_taken(self):
        cases = {
            "changed after it was kept": lambda out: (out / "octos").write_bytes(b"another kernel"),
            "missing": lambda out: (out / "octos").unlink(),
            "without a record": lambda out: (out / "kernel.json").unlink(),
            "with a record that cannot be read": lambda out: (out / "kernel.json").write_text("{"),
            "with a record that is not a record": lambda out: (out / "kernel.json").write_text("[]"),
        }
        for case, spoil in cases.items():
            with self.subTest(case=case):
                out = self.kept(description())
                spoil(out)
                taken, reason = kernel.accept(out, description(), "100")
                self.assertIsNone(taken)
                self.assertTrue(reason)
        out = self.kept(description())
        for case, (wanted, from_run) in {
            "built from another revision": (description(revision="0" * 40), "100"),
            "built by another compiler": (description(compiler="rustc 1.99.0 (stand-in)"), "100"),
            "named by another run than its artifact's": (description(), "101"),
        }.items():
            with self.subTest(case=case):
                taken, reason = kernel.accept(out, wanted, from_run)
                self.assertIsNone(taken)
                self.assertTrue(reason)
        self.assertEqual(kernel.accept(self.root / "nowhere", description(), "100")[0], None)

    def test_a_job_that_took_its_kernel_keeps_nothing(self):
        self.stage(description(), prebuilt=True, built_by_run="100")
        reason = kernel.keep(description(), self.root / "upload", "200", "c" * 40, runtime=self.runtime)
        self.assertIn("release run 100", reason)
        self.assertFalse((self.root / "upload").exists())

    def test_a_kernel_changed_after_packaging_is_not_kept(self):
        staged = self.stage(description())
        staged.write_bytes(b"signed in place")
        self.assertTrue(kernel.keep(description(), self.root / "upload", "200", "c" * 40, runtime=self.runtime))
        self.assertFalse((self.root / "upload").exists())

    def test_nothing_packaged_keeps_nothing(self):
        self.assertTrue(kernel.keep(description(), self.root / "upload", "200", "c" * 40, runtime=self.runtime))

    def test_the_packaged_record_names_the_run_that_built_the_kernel(self):
        staged = self.stage(description(), built_by_run="100")
        self.assertEqual(json.loads(Path(str(staged) + ".json").read_text())["builtByRun"], "100")
        staged = self.stage(description())
        self.assertIsNone(json.loads(Path(str(staged) + ".json").read_text())["builtByRun"])


if __name__ == "__main__":
    unittest.main()
