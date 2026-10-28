#!/usr/bin/env python3
"""Reconnect an actual recovered request, approve it, then restart/reconnect again."""
import argparse
import json
from pathlib import Path
import re
import time

from native_management_recovery import Database, restart, until
from native_management_wizard import App, ROOT


def open_connection(app, label, ready_text="Admission and connection verification are separate."):
    app.click("connections")
    # Selecting the current route preserves its position. Start the search at
    # the top even when this helper is called from an already-scrolled list.
    app.request("/m", k="scroll", x=750, y=650, dy=-100000, wait=0)
    app.wait(ready_text)
    until(lambda: not any(value in app.texts() for value in ("Updating…", "正在更新…")))
    for _ in range(30):
        rows = app.snap()
        title = next((row for row in rows if row.get("t") == label), None)
        if title is None:
            app.request("/m", k="scroll", x=750, y=650, dy=250, wait=0)
            time.sleep(.2)
            continue
        buttons = [row for row in rows if row.get("i") == "open" and
                   0 <= row["r"][1] - title["r"][1] < 180]
        if not buttons:
            app.request("/m", k="scroll", x=750, y=650, dy=100, wait=0)
            time.sleep(.2)
            continue
        button = min(buttons, key=lambda row: row["r"][1])
        x, y, width, height = button["r"]
        if 32 <= y and y + height <= 790:
            app.request("/click", x=x+width/2, y=y+height/2, wait=0)
            app.wait(label)
            until(lambda: not any(row.get("i") == "open" for row in app.snap()))
            return
        app.request("/m", k="scroll", x=750, y=650, dy=250 if y > 790 else -250, wait=0)
        time.sleep(.2)
    raise AssertionError("Target connection is not reachable")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--owner-run", type=Path, required=True)
    parser.add_argument("--admin-run", type=Path, required=True)
    parser.add_argument("--server-run", type=Path, required=True)
    parser.add_argument("--recovery-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    assert args.owner_run.resolve() == ROOT / ".run/management-ui-owner"
    assert args.admin_run.resolve() == ROOT / ".run/management-ui"
    recovered = json.loads(args.recovery_report.read_text())
    assert recovered["passed"] is True
    label = recovered["resourceLabel"]
    assert re.fullmatch(r"Recovery MacBook [a-f0-9]{8}", label)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    owner = App(args.owner_run, 8318, captures)
    admin = App(args.admin_run, 8317, captures)
    database = Database(args.server_run, "hagency-latest-test-postgres-1")
    report = {"passed": False, "checks": [], "resourceLabel": label,
              "binaries": {"owner": owner.binary_hash, "admin": admin.binary_hash}}
    apps = [owner, admin]
    log_offsets = {app.root: app.log_offset for app in apps}
    def current():
        return json.loads(database.sql("SELECT json_build_object('id',c.id,'state',c.state,'generation',c.executor_generation,'request',r.id,'requestState',r.state,'requestRevision',r.revision) FROM hagency_agent_v1.management_contributions c JOIN hagency_agent_v1.management_requests r ON r.id=c.request_id WHERE c.label='" + label + "';"))
    def passed(name):
        report["checks"].append(name)
        print("PASS " + name, flush=True)
    try:
        open_connection(owner, label)
        owner.wait("Check and reconnect")
        before = current()
        assert before["state"] == "pending_admission"
        owner.capture("pending-reconnect")
        owner.click("resume")
        until(lambda: current()["generation"] > before["generation"])
        until(lambda: not any(row.get("i") == "resume" for row in owner.snap()))
        owner.wait("Awaiting administrator")
        after = current()
        assert after["request"] == before["request"]
        assert after["requestRevision"] == before["requestRevision"] + 1
        passed("same executor reconnects its original pending request without duplicate admission")
        open_connection(admin, label)
        assert not any(row.get("i") in ("resume", "verify") for row in admin.snap())
        admin.click("approve")
        admin.wait("Confirm admission approval")
        admin.click("confirm")
        admin.wait("Admission approved")
        owner.click("refresh")
        owner.wait("Admission approved")
        owner.click("verify")
        owner.wait("Connection verified")
        owner.capture("reconnected-and-verified")
        approved = current()
        assert approved["requestState"] == "approved"
        passed("administrator approves the current revision and only the owner verifies the provider")
        owner.request("/gq")
        owner = restart(args.owner_run.resolve(), 8318, captures)
        apps.append(owner)
        assert owner.binary_hash == report["binaries"]["owner"]
        open_connection(owner, label)
        owner.wait("Check and reconnect")
        owner.click("resume")
        until(lambda: current()["generation"] > approved["generation"])
        owner.wait("Connection needs a fresh check")
        assert current()["requestState"] == "approved"
        owner.capture("approval-retained-verification-stale")
        owner.click("verify")
        owner.wait("Connection verified")
        owner.capture("verified-after-second-restart")
        admin.click("refresh")
        admin.wait("Connection verified")
        assert not any(row.get("i") in ("approve", "reject", "resume", "verify") for row in admin.snap())
        identity = current()["id"]
        assert re.fullmatch(r"[a-zA-Z0-9_-]+", identity)
        assert database.sql("SELECT count(*) FROM hagency_agent_v1.management_commands WHERE operation='request.decide' AND result::jsonb->'contribution'->>'id'='" + identity + "';") == "1"
        assert database.sql("SELECT count(*) FROM hagency_agent_v1.management_contributions WHERE label='" + label + "';") == "1"
        passed("another restart retains approval, requires a fresh check and creates no second decision")
        errors = []
        for root, offset in log_offsets.items():
            with (root / "native.log").open("rb") as log:
                log.seek(offset)
                errors += [line for line in log.read().decode(errors="replace").splitlines()
                           if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native script errors; inspect retained logs"
        report["final"] = current()
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
        raise
    finally:
        report["trace"] = [app.trace for app in apps]
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
