#!/usr/bin/env python3
"""Check native admission/executor status and contextual project navigation.

Uses the existing loopback owner and Chinese/dark coordinator fixtures. The
owner performs a real provider/connection check; no clock, rows or host replies
are injected. Project and connection navigation is read-only.
"""
import argparse
import json
from pathlib import Path
import re

from native_management_projects import idle, project
from native_management_reconnection import open_connection
from native_management_recovery import until
from native_management_wizard import App, ROOT


def readonly(app):
    assert not any(row.get("i") in ("approve", "reject", "resume", "verify", "revoke")
                   for row in app.snap()), "Coordinator received an owner/admin control"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = json.loads(args.project_report.read_text())
    assert source["passed"] is True
    label, name = source["resourceLabel"], source["projectName"]
    assert re.fullmatch(r"Recovery MacBook [a-f0-9]{8}", label)
    assert re.fullmatch(r"Miniapp project [a-f0-9]{8}", name)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    owner = App(ROOT / ".run/management-ui-owner", 8318, captures)
    coordinator = App(ROOT / ".run/management-ui-coordinator", 8319, captures)
    apps = [owner, coordinator]
    report = {"passed": False, "checks": [], "resourceLabel": label, "projectName": name,
              "binaries": {app.root.name: app.binary_hash for app in apps},
              "scope": "connection status and navigation; allocation/execution excluded"}

    def passed(check):
        report["checks"].append(check)
        print("PASS " + check, flush=True)

    try:
        assert owner.binary_hash == coordinator.binary_hash
        owner.wait("@miniapp_owner_1015:localhost:8088")
        coordinator.wait("@miniapp_coordinator_1015:localhost:8088")
        open_connection(owner, label)
        if any(row.get("i") == "resume" for row in owner.snap()):
            owner.click("resume")
            until(lambda: any(row.get("i") == "verify" for row in owner.snap()))
        owner.click("verify")
        owner.wait("Connection verified")
        for text in ("Server admission", "Admission approved", "Executor check",
                     "Executor and provider binding checked", "Keep the connection current"):
            owner.wait(text)
        owner.capture("owner-connection-verified-en-light")
        passed("owner sees distinct admitted and verified states after a real connection check")

        project(coordinator, name)
        coordinator.click("project_connection")
        coordinator.wait(label)
        for text in ("项目 · " + name, "连接已验证", "服务器准入", "准入已批准",
                     "执行设备检查", "已检查执行设备与关联模型服务"):
            coordinator.wait(text)
        readonly(coordinator)
        coordinator.capture("coordinator-project-connection-cn-dark")
        coordinator.click("refresh")
        idle(coordinator)
        coordinator.wait("项目 · " + name)
        coordinator.click("connection_project_back")
        coordinator.wait(name)
        coordinator.wait("项目申请已批准")
        assert not any(row.get("i") == "connection_project_back" for row in coordinator.snap())
        passed("Chinese coordinator retains project context across refresh and returns to current result")

        open_connection(coordinator, label, "准入审批与连接验证是两个步骤")
        coordinator.wait("连接已验证")
        readonly(coordinator)
        assert not any(row.get("i") == "connection_project_back" for row in coordinator.snap())
        assert "项目 · " + name not in coordinator.texts()
        coordinator.capture("coordinator-direct-connection-cn-dark")
        passed("normal connection navigation clears old project context and stays read-only")

        errors = []
        for app in apps:
            with (app.root / "native.log").open("rb") as log:
                log.seek(app.log_offset)
                errors += [line for line in log.read().decode(errors="replace").splitlines()
                           if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native errors; inspect retained logs"
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
        raise
    finally:
        report["trace"] = [app.trace for app in apps]
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
