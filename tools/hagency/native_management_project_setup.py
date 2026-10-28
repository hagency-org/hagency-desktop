#!/usr/bin/env python3
"""Validate setup readiness and administrator assignment through real native UI.

Uses the isolated localhost fixture and an approved project from the project
probe. Reservations stay disabled: this checks truthful readiness and role
boundaries, not room creation or the full allocation/agent lifecycle. All
mutations use the actual mini app; PostgreSQL is inspected read-only.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

from native_management_projects import idle, project
from native_management_recovery import Database, restart, until
from native_management_wizard import App, ROOT


MANAGER = "@miniapp_manager_1015:localhost:8088"
EFFECT_CONTROLS = {"setup_begin", "setup_continue", "setup_rebase", "setup_cancel"}


def controls(app):
    return {row.get("i") for row in app.snap()}


def setup(app, name):
    project(app, name)
    app.click("project_setup_open")
    idle(app)
    app.wait(name)
    app.wait("Matrix")


def no_capacity_effects(app):
    assert not EFFECT_CONTROLS.intersection(controls(app)), "Setup effects offered without capacity"


def rooms_not_started(app, chinese=False):
    # The first probe passed role checks but screenshot review caught this
    # missing child. Inspect rendered widgets, excluding the script source.
    app.wait("尚未开始创建房间" if chinese else "Room creation has not begun")


def top(app):
    app.request("/m", k="scroll", x=750, y=650, dy=-100000, wait=0)


def confirmation_visible(app):
    rows = [row for row in app.snap() if row.get("i") in {
        "setup_confirmation_cancel", "setup_confirmation_apply"}]
    # Instrumentation returns the clipped rectangle. A 22 px sliver can still
    # satisfy the window bounds; these default-theme buttons are at least 40 px.
    assert len(rows) == 2 and all(
        row["r"][3] >= 40 and row["r"][1] >= 32
        and row["r"][1] + row["r"][3] <= 780
        for row in rows), "Administrator confirmation controls clipped in initial viewport"


def assigned(app, mxid, label="Administrator"):
    def shown():
        rows = app.snap()
        fields = [row for row in rows if row.get("t") == label]
        return any(value.get("t") == mxid
                   and value["r"][0] >= field["r"][0] + 100
                   and abs(value["r"][1] - field["r"][1]) < 24
                   for field in fields for value in rows)
    until(shown)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-report", type=Path, required=True)
    parser.add_argument("--server-run", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = json.loads(args.project_report.read_text())
    assert source["passed"] is True
    name, project_id = source["projectName"], source["approvedProjectId"]
    assert re.fullmatch(r"Miniapp project [a-f0-9]{8}", name)
    assert re.fullmatch(r"[A-Za-z0-9_-]{1,128}", project_id)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    manager = App(ROOT / ".run/management-ui-manager", 8320, captures)
    admin = App(ROOT / ".run/management-ui", 8317, captures)
    coordinator = App(ROOT / ".run/management-ui-coordinator", 8319, captures)
    apps = [manager, admin, coordinator]
    database = Database(args.server_run, "hagency-latest-test-postgres-1")
    report = {"passed": False, "checks": [], "projectName": name, "projectId": project_id,
              "scope": "real setup readiness and assignment; no reserved capacity, Matrix creation or agent execution",
              "probeSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "binaries": {app.root.name: app.binary_hash for app in apps}}

    def authority():
        text = database.sql(
            "SELECT json_build_object('mxid',u.mxid,'revision',a.revision) "
            "FROM hagency_agent_v1.management_project_authorities a "
            "JOIN hagency_agent_v1.users u ON u.id=a.administrator_user_id "
            "WHERE a.plan_id='" + project_id + "';")
        return json.loads(text) if text else None

    def effects():
        return database.sql(
            "SELECT json_build_object('setups',(SELECT count(*) FROM hagency_agent_v1.management_project_setups),"
            "'projects',(SELECT count(*) FROM hagency_agent_v1.projects),"
            "'agents',(SELECT count(*) FROM hagency_agent_v1.agents),"
            "'bindings',(SELECT count(*) FROM hagency_agent_v1.bindings));")

    def passed(label):
        report["checks"].append(label)
        print("PASS " + label, flush=True)

    try:
        assert len(set(report["binaries"].values())) == 1, "Mixed native builds"
        assert authority() is None, "Use a fresh approved project for this probe"
        baseline = effects()
        setup(manager, name)
        manager.wait("No verified current reservation")
        manager.wait("Not assigned")
        rooms_not_started(manager)
        assert "setup_assign" not in controls(manager)
        no_capacity_effects(manager)
        top(manager)
        manager.capture("manager-requirements")
        passed("manager sees missing reservation and administrator without permission to start setup")

        setup(admin, name)
        rooms_not_started(admin)
        admin.field("setup_admin_input", MANAGER)
        admin.click("setup_assign")
        admin.wait("Confirm")
        confirmation_visible(admin)
        assert authority() is None, "Review already changed project authority"
        admin.click("setup_confirmation_cancel")
        idle(admin)
        assert authority() is None, "Cancelling confirmation changed project authority"
        passed("administrator review and cancellation leave authority unchanged")

        top(admin)
        admin.click("setup_assign")
        admin.wait("Confirm")
        confirmation_visible(admin)
        admin.capture("admin-assignment-confirmation")
        admin.click("setup_confirmation_apply")
        idle(admin)
        assigned(admin, MANAGER)
        assert authority() == {"mxid": MANAGER, "revision": 1}
        assert effects() == baseline, "Administrator assignment created setup or executable state"
        no_capacity_effects(admin)
        passed("server administrator assigns the real manager once without creating rooms or an agent")

        manager.click("setup_refresh")
        idle(manager)
        assigned(manager, MANAGER)
        rooms_not_started(manager)
        manager.wait("No verified current reservation")
        assert "setup_assign" not in controls(manager)
        no_capacity_effects(manager)
        top(manager)
        manager.capture("manager-assigned")
        manager.request("/gq")
        manager = restart(ROOT / ".run/management-ui-manager", 8320, captures)
        apps.append(manager)
        manager.wait(MANAGER)
        setup(manager, name)
        assigned(manager, MANAGER)
        manager.wait("No verified current reservation")
        no_capacity_effects(manager)
        rooms_not_started(manager)
        assert authority() == {"mxid": MANAGER, "revision": 1}
        passed("manager refresh and restart preserve latest assignment without treating it as capacity")

        setup(coordinator, name)
        coordinator.wait("当前没有已验证的资源预留")
        assigned(coordinator, MANAGER, "项目管理员")
        rooms_not_started(coordinator, chinese=True)
        assert "setup_assign" not in controls(coordinator)
        no_capacity_effects(coordinator)
        top(coordinator)
        coordinator.capture("coordinator-read-only-cn-dark")
        coordinator.click("setup_back")
        idle(coordinator)
        coordinator.wait(name)
        coordinator.wait("项目申请已批准")
        passed("coordinator reads Chinese setup status and returns to the exact project without mutation controls")

        setup(manager, name + " withdrawn")
        rooms_not_started(manager)
        assert "setup_assign" not in controls(manager)
        no_capacity_effects(manager)
        manager.capture("manager-withdrawn-setup")
        assert effects() == baseline
        passed("withdrawn project retains access to setup status without actionable setup controls")
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
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
