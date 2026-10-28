#!/usr/bin/env python3
"""Exercise real project requests in three distinct native SDK sessions.

Requires the isolated localhost fixture and an admitted contribution. All writes
go through the built-in Splash bundle and typed native host. PostgreSQL is read
only here and confirms approval creates neither an executable project nor agent.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import time
import uuid

from native_management_recovery import Database, restart, until
from native_management_wizard import App, ROOT


def idle(app):
    until(lambda: not any(value in app.texts() for value in ("Updating…", "正在更新…")))


def select_card(app, label, control):
    """Click the actual control under a visible, uniquely named card."""
    app.request("/m", k="scroll", x=750, y=650, dy=-100000, wait=0)
    for _ in range(50):
        rows = app.snap()
        title = next((row for row in rows if row.get("t") == label), None)
        if title:
            choices = [row for row in rows if row.get("i") == control
                       and 0 <= row["r"][1] - title["r"][1] < 260]
            if choices:
                button = min(choices, key=lambda row: row["r"][1])
                x, y, width, height = button["r"]
                if 32 <= y and y + height <= 790:
                    app.request("/click", x=x+width/2, y=y+height/2, wait=0)
                    time.sleep(.2)
                    return
        app.request("/m", k="scroll", x=750, y=650, dy=120, wait=0)
        time.sleep(.15)
    raise AssertionError("Named card was not reachable: " + label)


def project(app, name):
    app.click("projects")
    idle(app)
    select_card(app, name, "open_project")
    app.wait(name)
    until(lambda: not any(row.get("i") == "open_project" for row in app.snap()))


def no_decision(app):
    assert not any(row.get("i") in ("project_approve", "project_reject", "project_confirm")
                   for row in app.snap()), "Non-administrator received project decision controls"


def fill_request(manager, resource, name):
    manager.click("projects")
    idle(manager)
    manager.click("new_project")
    manager.wait("Choose a resource")
    idle(manager)
    manager.click("catalog_coding")
    idle(manager)
    select_card(manager, resource, "choose_resource")
    manager.wait("Project name")
    manager.field("project_name", name)
    manager.field("project_purpose", "Validate a separate project manager and server administrator")
    manager.field("project_tokens", "54321")


def submit(manager):
    manager.click("project_review")
    manager.wait("Submit project request")
    manager.click("project_submit")
    manager.wait("Awaiting project approval")
    no_decision(manager)


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
    manager = App(ROOT / ".run/management-ui-manager", 8320, captures)
    admin = App(ROOT / ".run/management-ui", 8317, captures)
    coordinator = App(ROOT / ".run/management-ui-coordinator", 8319, captures)
    apps = [manager, admin, coordinator]
    database = Database(args.server_run, "hagency-latest-test-postgres-1")
    name = "Miniapp project " + uuid.uuid4().hex[:8]
    report = {"passed": False, "checks": [], "projectName": name, "resourceLabel": resource,
              "scope": "project intent approval only; allocation and agent execution excluded",
              "probeSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "binaries": {app.root.name: app.binary_hash for app in apps}}

    def counts():
        return database.sql("SELECT json_build_object('projects',(SELECT count(*) FROM hagency_agent_v1.projects),"
                            "'agents',(SELECT count(*) FROM hagency_agent_v1.agents),"
                            "'bindings',(SELECT count(*) FROM hagency_agent_v1.bindings));")

    def stored(label):
        assert re.fullmatch(r"Miniapp project [a-f0-9]{8}( declined| withdrawn)?", label)
        result = database.sql("SELECT json_build_object('id',p.id,'state',p.state,'request',p.request_id,"
                              "'tokens',p.requested_tokens,'requestState',r.state,'revision',p.revision) "
                              "FROM hagency_agent_v1.management_projects p JOIN hagency_agent_v1.management_requests r "
                              "ON r.id=p.request_id WHERE p.name='" + label + "';")
        return json.loads(result) if result else None

    def passed(label):
        report["checks"].append(label)
        print("PASS " + label, flush=True)

    try:
        assert len(set(report["binaries"].values())) == 1, "Mixed native builds"
        manager.wait("@miniapp_manager_1015:localhost:8088")
        admin.wait("@admin:localhost:8088")
        coordinator.wait("@miniapp_coordinator_1015:localhost:8088")
        for app in apps:
            idle(app)
        baseline = counts()
        fill_request(manager, resource, name)
        manager.field("project_tokens", "NaN")
        manager.click("project_review")
        manager.wait("Enter a project name")
        assert stored(name) is None
        manager.field("project_tokens", "54321")
        manager.click("project_review")
        manager.wait("Submit project request")
        manager.capture("manager-review")
        assert manager.draft("@miniapp_manager_1015:localhost:8088")["projectDraft"]["name"] == name
        manager.request("/gq")
        manager = restart(ROOT / ".run/management-ui-manager", 8320, captures)
        apps.append(manager)
        manager.wait("@miniapp_manager_1015:localhost:8088")
        # SDK identity appears before the initial management reads finish.
        # Navigate after the visible updating state clears, as other steps do.
        idle(manager)
        manager.click("projects")
        manager.wait("Continue saved draft")
        manager.click("continue_project")
        manager.wait("Submit project request")
        manager.wait(name)
        manager.click("project_submit")
        manager.wait("Awaiting project approval")
        no_decision(manager)
        manager.capture("manager-pending")
        assert stored(name)["tokens"] == 54321
        passed("manager validates amount, restores draft after restart and submits exactly one request")

        admin.click("inbox")
        idle(admin)
        select_card(admin, name, "open")
        admin.wait(name)
        admin.click("project_approve")
        admin.wait("Confirm project approval")
        admin.capture("admin-confirmation")
        admin.click("project_confirm")
        admin.wait("Project request approved")
        assert not any(row.get("i") in ("project_approve", "project_reject") for row in admin.snap())
        manager.click("refresh")
        manager.wait("Project request approved")
        manager.wait("Approval does not reserve capacity")
        manager.wait("Approval does not create an agent")
        no_decision(manager)
        manager.capture("manager-approved")
        approved = stored(name)
        assert approved["state"] == "approved" and approved["requestState"] == "approved"
        assert counts() == baseline, "Plan approval created executable state"
        report["approvedProjectId"] = approved["id"]
        passed("only administrator approves; manager sees latest approval without allocation or agent creation")

        project(coordinator, name)
        coordinator.wait("项目申请已批准")
        no_decision(coordinator)
        assert not any(row.get("i") == "project_withdraw" for row in coordinator.snap())
        coordinator.capture("coordinator-approved-cn-dark")
        passed("coordinator sees assigned plan in Chinese without approval or withdrawal authority")

        coordinator.click("project_connection")
        idle(coordinator)
        coordinator.wait(resource)
        assert not any(row.get("i") in ("approve", "reject", "verify", "resume", "revoke")
                       for row in coordinator.snap())
        coordinator.capture("coordinator-resource-connection-cn-dark")
        coordinator.click("connection_project_back")
        idle(coordinator)
        coordinator.wait(name)
        coordinator.wait("项目申请已批准")
        no_decision(coordinator)
        passed("coordinator reads the exact resource connection and returns to the current project without owner controls")

        rejected = name + " declined"
        fill_request(manager, resource, rejected)
        submit(manager)
        admin.click("inbox")
        idle(admin)
        select_card(admin, rejected, "open")
        admin.click("project_reject")
        admin.wait("Confirm project rejection")
        admin.click("project_confirm")
        admin.wait("Project request declined")
        manager.click("refresh")
        manager.wait("Project request declined")
        no_decision(manager)
        assert stored(rejected)["state"] == "rejected"
        passed("administrator rejection closes the manager request")

        withdrawn = name + " withdrawn"
        fill_request(manager, resource, withdrawn)
        submit(manager)
        admin.click("inbox")
        idle(admin)
        select_card(admin, withdrawn, "open")
        admin.wait(withdrawn)
        manager.click("project_withdraw")
        manager.wait("Withdraw this project request?")
        manager.click("project_confirm")
        manager.wait("Project request withdrawn")
        admin.click("refresh")
        admin.wait("Project request withdrawn")
        assert not any(row.get("i") in ("project_approve", "project_reject") for row in admin.snap())
        assert stored(withdrawn)["state"] == "revoked"
        assert counts() == baseline
        passed("manager withdrawal updates an already-open administrator card and removes obsolete actions")
        errors = []
        for app in apps:
            with (app.root / "native.log").open("rb") as log:
                log.seek(app.log_offset)
                errors += [line for line in log.read().decode(errors="replace").splitlines()
                           if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native script error; inspect retained logs"
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
        raise
    finally:
        report["trace"] = [app.trace for app in apps]
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
