#!/usr/bin/env python3
"""Recover a real project request after a lost reply and administrator decision.

Only the dedicated localhost manager fixture is crashed. PostgreSQL and SQLite
locks delay real effects; neither fabricates a response nor inserts success.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import sqlite3
import subprocess
import uuid

from native_management_projects import fill_request, idle, no_decision, select_card
from native_management_recovery import Database, restart, until
from native_management_wizard import App, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reconnection-report", type=Path, required=True)
    parser.add_argument("--server-run", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = json.loads(args.reconnection_report.read_text())
    assert source["passed"] is True
    resource = source["resourceLabel"]
    assert re.fullmatch(r"Recovery MacBook [a-f0-9]{8}", resource)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    run = ROOT / ".run/management-ui-manager"
    account = "@miniapp_manager_1015:localhost:8088"
    manager = App(run, 8320, captures)
    admin = App(ROOT / ".run/management-ui", 8317, captures)
    apps = [manager, admin]
    database = Database(args.server_run, "hagency-latest-test-postgres-1")
    name = "Miniapp project " + uuid.uuid4().hex[:8]
    report = {"passed": False, "checks": [], "projectName": name,
              "scope": "project request custody, lost reply, latest approved result and damaged draft",
              "binarySha256": manager.binary_hash}
    writer = None

    def passed(label):
        report["checks"].append(label)
        print("PASS " + label, flush=True)

    try:
        assert manager.binary_hash == admin.binary_hash, "Mixed native binaries"
        manager.wait(account)
        admin.wait("@admin:localhost:8088")
        fill_request(manager, resource, name)
        manager.click("project_review")
        manager.wait("Submit project request")
        paths = list((run / "profile/native-owner").glob("*/owned-agent-owners/management_*/domain.sqlite3"))
        assert len(paths) == 1, "Ambiguous test manager ledger"

        def pending():
            with sqlite3.connect(f"file:{paths[0]}?mode=ro", uri=True, timeout=.2) as db:
                return db.execute("SELECT command_id,input FROM management_intents WHERE state='pending' "
                                  "AND operation=? AND json_extract(input,'$.name')=?",
                                  ('"project_request"', name)).fetchall()

        assert not pending()
        database.hold()
        manager.click("project_submit")
        rows = until(pending)
        assert len(rows) == 1
        command_id, raw = rows[0]
        assert re.fullmatch(r"[a-zA-Z0-9_-]+", command_id)
        assert json.loads(raw)["command_id"] == command_id
        report["commandId"] = command_id
        report["inputSha256"] = hashlib.sha256(raw.encode()).hexdigest()
        passed("project request is durably held before the server mutation")

        writer = sqlite3.connect(paths[0], timeout=2)
        writer.execute("BEGIN IMMEDIATE")
        database.release()
        receipt_count = lambda: database.sql("SELECT count(*) FROM hagency_agent_v1.management_commands "
                                            "WHERE command_id='" + command_id + "';")
        until(lambda: receipt_count() == "1")
        assert writer.execute("SELECT state,reply FROM management_intents WHERE command_id=?",
                              (command_id,)).fetchone() == ("pending", None)
        state = json.loads((run / "state.json").read_text())
        assert manager.request("/s")["pid"] == state["pid"]
        command = subprocess.check_output(["ps", "-p", str(state["pid"]), "-o", "command="], text=True).strip()
        assert command == str(run / "bin/rinx"), "Refusing to kill an unrelated process"
        os.kill(state["pid"], signal.SIGKILL)
        writer.rollback()
        writer.close()
        writer = None
        passed("one server request committed before the manager crashed without a native reply")

        admin.click("inbox")
        idle(admin)
        select_card(admin, name, "open")
        admin.click("project_approve")
        admin.wait("Confirm project approval")
        admin.click("project_confirm")
        admin.wait("Project request approved")
        passed("administrator decided the original request while the manager was offline")

        draft = run / "profile/miniapps" / account.encode().hex() / "im.palpo.operations/onboarding-v1.json"
        report["draftBeforeSha256"] = hashlib.sha256(draft.read_bytes()).hexdigest()
        draft.write_text('{"version":1,"projectDraft":')
        assert pending()[0][0] == command_id
        manager = restart(run, 8320, captures)
        apps.append(manager)
        assert manager.binary_hash == report["binarySha256"]
        manager.wait("Saved requests")
        manager.wait("Retry original request")
        manager.capture("project-recovery-pending")
        manager.click("recover_native")
        manager.wait(name)
        manager.wait("Project request approved")
        no_decision(manager)
        manager.capture("project-recovered-latest-result")

        def acknowledged():
            with sqlite3.connect(f"file:{paths[0]}?mode=ro", uri=True) as db:
                return db.execute("SELECT state,input,reply FROM management_intents WHERE command_id=?",
                                  (command_id,)).fetchone() == ("acknowledged", None, None)

        until(acknowledged)
        assert receipt_count() == "1"
        assert database.sql("SELECT count(*) FROM hagency_agent_v1.management_projects WHERE name='" + name + "';") == "1"
        generations = database.sql("SELECT c.device_generation,d.generation FROM hagency_agent_v1.management_commands c "
                                   "JOIN hagency_agent_v1.devices d ON d.id=c.device_id WHERE c.command_id='" + command_id + "';")
        original, current = map(int, generations.split("|"))
        assert current > original, "Restart must rotate real authorization"
        report["executorGenerations"] = {"original": original, "current": current}
        assert not pending()
        assert manager.draft(account)["command"] is None
        passed("recovery survives draft damage and authorization rotation, returns latest decision and creates no duplicate")
        errors = []
        for app in apps:
            with (app.root / "native.log").open("rb") as log:
                log.seek(app.log_offset)
                errors += [line for line in log.read().decode(errors="replace").splitlines()
                           if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native runtime errors; inspect retained logs"
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
        raise
    finally:
        database.release()
        if writer:
            writer.rollback()
            writer.close()
        report["trace"] = [app.trace for app in apps]
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
