#!/usr/bin/env python3
"""Inspect an existing agent request's status in an already-open native fixture.

Start on the real request detail. The probe does not create a request, seed
capacity, approve, change accounts/flags or restart. Expected stage text comes
from the operator's real fixture; qualification is never a running assertion.
"""
import argparse
import hashlib
import json
from pathlib import Path
import time

from native_management_agent_readiness import saved_command
from native_management_projects import idle
from native_management_wizard import App


COPY = {
    "en": ("Agent status", "Agent approval", "Native agent reservation",
           "Matrix presence", "Runtime qualification", "Work history and usage",
           "Shared-agent work records are not available in this interface yet.",
           "Qualification checks the provider process. It does not prove that the agent is currently online or performing work."),
    "zh-CN": ("Agent 状态", "Agent 审批", "原生 Agent 预留", "Matrix 成员状态",
              "运行环境资格检查", "任务历史与用量", "此界面暂不提供共享 Agent 的任务记录。",
              "资格检查用于验证模型服务进程，并不证明 Agent 当前在线或正在执行任务。"),
}
MUTATIONS = {"agent_new", "agent_submit", "agent_confirm", "agent_approve",
             "agent_reject", "agent_cancel", "execute", "top_up", "remove"}


def top(app):
    app.request("/m", k="scroll", x=750, y=650, dy=-100000, wait=0)
    time.sleep(.15)


def inspect(app, args, capture):
    top(app)
    app.wait(COPY[args.language][0])
    app.wait(args.agent_name)
    app.capture(capture + "-top")
    texts, controls = set(), set()
    reached_bottom = False
    for _ in range(28):
        rows = app.snap()
        texts.update(row.get("t", "") for row in rows)
        controls.update(row.get("i", "") for row in rows)
        footer = next((row for row in rows if row.get("t") == COPY[args.language][6]), None)
        if footer and footer["r"][3] > 0 and 32 <= footer["r"][1] and footer["r"][1] + footer["r"][3] <= 780:
            reached_bottom = True
            app.capture(capture + "-history")
            break
        app.request("/m", k="scroll", x=750, y=650, dy=160, wait=0)
        time.sleep(.15)
    assert reached_bottom, "Work-history availability section is not reachable"
    assert not controls.intersection(MUTATIONS), "Read-only status offers mutation controls"
    for value in COPY[args.language] + (args.assignment, args.membership, args.runtime):
        assert value in texts, "Missing rendered status: " + value
    if args.agent_id:
        assert args.agent_id in texts, "Recorded native agent ID differs from the real fixture"


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--run", type=Path, required=True)
    p.add_argument("--port", type=int, required=True)
    p.add_argument("--account", required=True)
    p.add_argument("--agent-name", required=True)
    p.add_argument("--agent-id", help="Actual recorded native agent ID, when an assignment exists")
    p.add_argument("--language", choices=COPY, required=True)
    p.add_argument("--assignment", required=True, help="Exact actual translated reservation-stage text")
    p.add_argument("--membership", required=True, help="Exact actual translated Matrix-stage text")
    p.add_argument("--runtime", required=True, help="Exact actual translated qualification-stage text")
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    assert args.account.startswith("@") and args.account.endswith(":localhost:8088"), "Use isolated loopback accounts"
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    app = App(args.run, args.port, captures)
    report = {"passed": False, "checks": [], "binarySha256": app.binary_hash,
              "probeSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "account": args.account, "agentName": args.agent_name, "agentId": args.agent_id,
              "scope": "read-only recorded stages, refresh and Back; no work, usage or execution claim"}
    try:
        app.wait(args.account)
        idle(app)
        app.wait(args.agent_name)
        original = saved_command(app, args.account)
        app.click("agent_status_open")
        idle(app)
        inspect(app, args, "agent-status")
        report["checks"].append("existing request exposes recorded stages and honest unavailable work history")
        top(app)
        app.click("agent_status_refresh")
        idle(app)
        inspect(app, args, "agent-status-refreshed")
        report["checks"].append("fresh server reads preserve expected real fixture facts without mutation controls")
        top(app)
        app.click("agent_status_back")
        idle(app)
        app.wait(args.agent_name)
        assert any(row.get("i") == "agent_list_back" for row in app.snap()), "Back did not return to request detail"
        app.wait(args.account)
        assert saved_command(app, args.account) == original, "Status reads changed original command or draft presence"
        report["checks"].append("request, account and original command are preserved")
        with (app.root / "native.log").open("rb") as f:
            f.seek(app.log_offset)
            errors = [line for line in f.read().decode(errors="replace").splitlines()
                      if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native errors; inspect retained native.log"
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
        raise
    finally:
        report["trace"] = app.trace
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
