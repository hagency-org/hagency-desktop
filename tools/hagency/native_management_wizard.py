#!/usr/bin/env python3
"""Drive the real admission wizard in two isolated, signed-in native instances.

Prepare owner/admin test profiles against the loopback development server and
leave the owner's wizard at Request server admission with a saved resource.
This probe enters fields and makes real admission decisions; it never injects
host replies or credentials. Instances are left running for inspection.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/wechat-ux/live"))
from native_probe import NativeApp


class App(NativeApp):
    def __init__(self, run, port, output):
        super().__init__(run, port, auto_login=False)
        self.output = output
        saved = json.loads((self.root / "state.json").read_text())
        self._user_seq = saved["user_seq"]
        self._activity_checked = self._user_seq is not None
        assert self.request("/s")["pid"] == saved["pid"], "Different native process"
        binary = self.root / "bin/rinx"
        self.binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
        self.log_offset = (self.root / "native.log").stat().st_size

    def request(self, route, **params):
        if route in {"/snap", "/g", "/click", "/m", "/k", "/t"}:
            params.setdefault("w", 1)
        return super().request(route, **params)

    def texts(self):
        return [w.get("t") or "" for w in self.snap()]

    def snap(self):
        # Splash's root inspector entry contains the complete source program.
        # It is not visible UI text and must never satisfy an assertion.
        return [w for w in super().snap() if w.get("i") != "card"]

    def wait(self, text, timeout=25):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if any(text in value for value in self.texts()):
                return
            time.sleep(.15)
        raise AssertionError("Native text not reached: " + text)

    def click(self, name):
        for attempt in range(12):
            rows = [w for w in self.snap() if w.get("i") == name or w.get("t") == name]
            if rows:
                x, y, width, height = rows[0]["r"]
                if y >= 32 and y + height <= 790:
                    self.request("/click", x=x + width/2, y=y + height/2, wait=0)
                    time.sleep(.25)
                    return
            self.request("/m", k="scroll", x=750, y=650, dy=250, wait=0)
            time.sleep(.2)
        raise AssertionError("Native control not reachable: " + name)

    def field(self, name, value):
        self.click(name)
        self.request("/k", c="A", cmd=1, wait=0)
        self.request("/t", t=value, wait=0)
        self.wait(value)

    def draft(self, account):
        path = self.root / "profile/miniapps" / account.encode().hex() / "im.palpo.operations/onboarding-v1.json"
        def unique(pairs):
            keys = [k for k, _ in pairs]
            assert len(keys) == len(set(keys)), "Draft contains duplicate property keys"
            return dict(pairs)
        return json.loads(path.read_text(), object_pairs_hook=unique)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--owner-run", type=Path, required=True)
    parser.add_argument("--admin-run", type=Path, required=True)
    parser.add_argument("--owner-port", type=int, default=8318)
    parser.add_argument("--admin-port", type=int, default=8317)
    parser.add_argument("--owner", required=True)
    parser.add_argument("--admin", required=True)
    parser.add_argument("--coordinator", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--enroll-model", help="Also create a resource using an already connected provider")
    parser.add_argument("--verify-connection", action="store_true", help="Verify the admitted resource from the owner's original executor")
    parser.add_argument("--verify-expiry", action="store_true", help="Wait for the real five-minute expiry, then verify again")
    args = parser.parse_args()
    if args.verify_expiry and not args.verify_connection:
        parser.error("--verify-expiry requires --verify-connection")
    for account in (args.owner, args.admin, args.coordinator):
        assert account.startswith("@") and account.endswith(":localhost:8088"), "Loopback test accounts required"
    assert len({args.owner, args.admin, args.coordinator}) == 3, "Separate roles required"
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    report = {"passed": False, "checks": [], "scope": "resource admission only"}
    owner = App(args.owner_run, args.owner_port, args.output)
    admin = App(args.admin_run, args.admin_port, args.output)
    report["binaries"] = {"owner": owner.binary_hash, "admin": admin.binary_hash}
    def passed(name):
        report["checks"].append(name)
        print("PASS " + name, flush=True)
    try:
        owner.wait(args.owner)
        admin.wait(args.admin)
        if args.enroll_model:
            owner.click("resources")
            owner.wait("All capabilities")
            owner.click("add")
            owner.wait("Start with your server")
            owner.click("next")
            owner.wait("Choose models")
            owner.click("choose")
            owner.wait(args.enroll_model)
            owner.click(args.enroll_model)
            owner.click("High")
            owner.field("budget", "NaN")
            owner.click("create")
            owner.wait("Enter a whole number of tokens")
            assert owner.draft(args.owner)["command"] is None
            owner.field("budget", "123456")
            assert owner.draft(args.owner)["wizard"]["tokens"] == "123456"
            owner.capture("resource-definition")
            owner.click("create")
            owner.wait("Request server admission")
            heading = next(w for w in owner.snap() if w.get("t") == "Request server admission")
            assert heading["r"][1] < 400, "New wizard step retained previous scroll offset"
            passed("valid model enrollment, edited capacity, numeric validation and new-step scroll reset")
        owner.wait("Request server admission")
        label = "Owner MacBook " + uuid.uuid4().hex[:8]
        report["resourceLabel"] = label
        owner.field("resource_name", label)
        owner.field("coordinator", "invalid-coordinator")
        draft = owner.draft(args.owner)
        assert draft["wizard"]["label"] == label
        assert draft["wizard"]["coordinator"] == "invalid-coordinator"
        passed("text edits update unique draft properties")
        owner.click("submit")
        owner.wait("Enter the coordinator's full Matrix ID")
        assert owner.draft(args.owner)["command"] is None
        passed("invalid coordinator is correctable without a stuck saved command")
        owner.field("coordinator", args.coordinator)
        owner.click("submit")
        owner.wait("Awaiting administrator")
        owner.capture("owner-pending")
        assert owner.draft(args.owner)["command"] is None
        passed("owner submits resource admission through the native host")
        admin.click("inbox")
        admin.wait(args.owner)
        admin.click("Review request")
        admin.wait(label)
        admin.wait(args.coordinator)
        admin.click("approve")
        admin.wait("Confirm admission approval")
        admin.capture("admin-review")
        admin.click("confirm")
        admin.wait("Admission approved")
        assert not any(w.get("i") == "approve" for w in admin.snap())
        passed("designated administrator approves and completed card loses decision controls")
        owner.click("refresh")
        owner.wait("Admission approved")
        owner.wait("Next: verify the executor connection")
        assert not any(w.get("i") in ("approve", "reject") for w in owner.snap())
        owner.capture("owner-approved")
        passed("owner sees latest decision without obtaining administrator controls or false readiness")
        if args.verify_connection:
            assert not any(w.get("i") == "verify" for w in admin.snap()), "Admin cannot impersonate executor"
            owner.click("verify")
            owner.wait("Connection verified")
            owner.wait("no agent has been started")
            draft = owner.draft(args.owner)
            assert draft["command"] is None
            contribution = draft["wizard"]["admission"]["contribution"]
            assert contribution["state"] == "admitted"
            assert contribution["verification"]["status"] == "verified"
            assert contribution["revision"] == 3
            owner.capture("owner-verified")
            admin.click("refresh")
            admin.wait("Connection verified")
            assert not any(w.get("i") == "verify" for w in admin.snap())
            admin.capture("admin-verified")
            report["verification"] = contribution["verification"]
            report["scope"] = "resource enrollment, admission and executor connection verification"
            passed("original executor verifies actual provider binding and admin sees the latest result")
            if args.verify_expiry:
                owner.wait("Connection needs a fresh check", timeout=310)
                owner.capture("owner-verification-expired")
                # The UI conservatively expires one second before server time.
                time.sleep(1.2)
                owner.click("refresh")
                owner.wait("Connection needs a fresh check")
                # Wait for the real server reply to replace the saved draft.
                deadline = time.monotonic() + 25
                while time.monotonic() < deadline:
                    latest = owner.draft(args.owner)["wizard"]["admission"]["contribution"]
                    if latest["verification"]["status"] == "stale":
                        break
                    time.sleep(.15)
                assert latest["verification"]["status"] == "stale"
                owner.click("verify")
                owner.wait("Connection verified")
                latest = owner.draft(args.owner)["wizard"]["admission"]["contribution"]
                assert latest["revision"] == 4
                assert latest["verification"]["verifiedAtMs"] > contribution["verification"]["verifiedAtMs"]
                owner.capture("owner-reverified")
                passed("real-time expiry removes verified status and a new native check renews it")
        errors = []
        for app in (owner, admin):
            with (app.root / "native.log").open("rb") as log:
                log.seek(app.log_offset)
                errors += [line for line in log.read().decode(errors="replace").splitlines()
                           if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native runtime logged script errors during the test"
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
        raise
    finally:
        report["trace"] = {"owner": owner.trace, "admin": admin.trace}
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
