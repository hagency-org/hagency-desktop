#!/usr/bin/env python3
"""Read a real native resource detail without changing accounts or resources.

Use an already signed-in loopback fixture with the Hagency mini app open. This
probe never starts/restarts a process, changes language/theme or issues a
management mutation. An optional capability selects the test's starting filter;
Back must preserve that filtered list. Run separately in EN/CN fixtures prepared
by the operator. Captures are full native windows, never crops or mock replies.
"""
import argparse
import hashlib
import json
from pathlib import Path
import time

from native_management_projects import idle
from native_management_wizard import App


COPY = {
    "en": ("All capabilities", "Resource on this device", "Model and capabilities",
           "Native budget snapshot", "Available / limit", "Observed usage",
           "Publication and configuration", "Configuration editing is not available",
           "This is the native ledger, not a provider quota check.",
           "Unknown means no measurement is recorded."),
    "zh-CN": ("全部能力", "此设备上的资源", "模型与能力", "原生账本预算快照",
              "可用 / 上限", "已观测使用量", "发布与配置", "暂不支持编辑配置",
              "这里显示原生账本，不是模型服务商的实时额度。", "未知表示尚无观测记录。"),
}
MUTATIONS = {"create", "submit", "approve", "reject", "confirm", "verify", "revoke",
             "resource_edit", "resource_save", "configure", "publish", "offer"}


def top(app):
    app.request("/m", k="scroll", x=750, y=650, dy=-100000, wait=0)
    time.sleep(.2)


def resource_cards(app):
    """Fingerprint rendered public titles/identity labels on the current page."""
    top(app)
    # Splash retains the rendered page's child geometries, including children
    # below the viewport. Do not use the complete program inspector entry.
    rows = app.snap()
    titles = [row.get("t", "") for row in rows if " · " in row.get("t", "")
              and row.get("i") not in ("card",)]
    return sorted(titles), sum(row.get("i") == "resource_detail" for row in rows)


def choose_resource(app, title, expected_budget, occurrence, resource_id=None):
    """Select visible facts; exact identity is checked again on the detail page.

    Short labels are display hints only. A colliding visible abbreviation falls
    back to the full ID in the app. Legacy builds still support explicit ordinal
    selection, without claiming an independently observed identity.
    """
    expected_ids = None
    if resource_id is not None:
        short = resource_id if len(resource_id) <= 16 else resource_id[:4] + "…" + resource_id[-8:]
        expected_ids = {prefix + value for prefix in ("Resource ID · ", "资源标识 · ")
                        for value in (resource_id, short)}
    top(app)
    chosen_budget = None
    for _ in range(40):
        rows = app.snap()
        headings = sorted([row for row in rows if row.get("t") == title], key=lambda row: row["r"][1])
        candidates = []
        for index, heading in enumerate(headings):
            low = heading["r"][1]
            high = headings[index + 1]["r"][1] if index + 1 < len(headings) else float("inf")
            buttons = [row for row in rows if row.get("i") == "resource_detail" and low < row["r"][1] < high]
            if not buttons:
                continue
            button = min(buttons, key=lambda row: row["r"][1])
            amounts = [row.get("t", "") for row in rows
                       if low < row["r"][1] < button["r"][1] and " / " in row.get("t", "")
                       and row.get("t") not in ("Available / monthly token limit", "可用额度 / 每月 token 上限", "Available / limit", "可用 / 上限")]
            if len(amounts) != 1:
                raise AssertionError("Cannot identify this resource card's public budget")
            labels = [row.get("t", "") for row in rows
                      if low < row["r"][1] < button["r"][1]
                      and row.get("t", "").startswith(("Resource ID · ", "资源标识 · "))]
            if expected_ids is not None and (len(labels) != 1 or labels[0] not in expected_ids):
                continue
            if expected_budget is None or amounts[0] == expected_budget:
                candidates.append((button, amounts[0], labels[0] if len(labels) == 1 else None))
        if occurrence is None:
            assert len(candidates) == 1, "Ambiguous resource selector; specify --resource-id, --expected-budget or --occurrence"
            selected = 0
        else:
            selected = occurrence
            assert selected < len(candidates), "Explicit resource occurrence is unavailable"
        button, budget, identity_label = candidates[selected]
        if chosen_budget is not None:
            assert budget == chosen_budget, "Selected public budget changed during navigation"
        chosen_budget = budget
        x, y, width, height = button["r"]
        if 32 <= y and y + height <= 790:
            app.request("/click", x=x + width/2, y=y + height/2, wait=0)
            time.sleep(.2)
            return budget, identity_label
        app.request("/m", k="scroll", x=750, y=650, dy=120, wait=0)
        time.sleep(.15)
    raise AssertionError("Selected resource card is not reachable")


def inspect_detail(app, language, name):
    top(app)
    app.wait(COPY[language][1])
    app.wait(COPY[language][3])
    app.capture(name + "-top")
    texts, controls = set(), set()
    reached_bottom = False
    for _ in range(24):
        rows = app.snap()
        texts.update(row.get("t", "") for row in rows)
        controls.update(row.get("i", "") for row in rows)
        button = next((row for row in rows if row.get("i") == "resource_connections"), None)
        if button and 32 <= button["r"][1] and button["r"][1] + button["r"][3] <= 790:
            app.capture(name + "-configuration")
            reached_bottom = True
            break
        app.request("/m", k="scroll", x=750, y=650, dy=180, wait=0)
        time.sleep(.15)
    assert reached_bottom, "Resource configuration/footer is not reachable"
    assert not controls.intersection(MUTATIONS), "Read-only detail exposes mutation control"
    for text in COPY[language][1:]:
        assert any(text in value for value in texts), "Missing actual detail text: " + text
    return texts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--account", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--framework", required=True)
    parser.add_argument("--language", choices=COPY, required=True)
    parser.add_argument("--capability", choices=("coding", "documentation"))
    parser.add_argument("--expected-budget", help="Select/assert exact displayed amount, e.g. '123456 / 123456'")
    parser.add_argument("--resource-id", help="Select the visible shortened/full ID and assert the exact full ID on detail and refresh")
    parser.add_argument("--occurrence", type=int, help="Explicit zero-based occurrence among model/framework/budget matches")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.occurrence is not None and args.occurrence < 0:
        parser.error("--occurrence must be non-negative")
    assert args.account.startswith("@") and args.account.endswith(":localhost:8088"), "Isolated loopback account required"
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    app = App(args.run, args.port, captures)
    report = {"passed": False, "checks": [], "binarySha256": app.binary_hash,
              "probeSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "language": args.language, "account": args.account,
              "capability": args.capability or "existing filter",
              "scope": "real native read-only resource detail, refresh and Back; no editor, provider check, admission or execution"}

    def passed(check):
        report["checks"].append(check)
        print("PASS " + check, flush=True)

    try:
        app.wait(args.account)
        idle(app)  # Navigation is intentionally disabled during the initial read.
        original_command = app.draft(args.account).get("command")
        app.click("resources")
        idle(app)
        app.wait(COPY[args.language][0])
        if args.capability:
            app.click("coding" if args.capability == "coding" else "docs")
            idle(app)
        title = args.model + " · " + args.framework
        app.wait(title)
        before = resource_cards(app)
        assert before[1] > 0, "No resource detail controls"
        selected_budget, identity_label = choose_resource(app, title, args.expected_budget, args.occurrence, args.resource_id)
        report["publicSelector"] = {"title": title, "budget": selected_budget, "occurrence": args.occurrence, "identityLabel": identity_label, "expectedResourceId": args.resource_id}
        idle(app)
        app.wait(args.model)
        texts = inspect_detail(app, args.language, "resource-detail-" + args.language)
        assert selected_budget + " tokens" in texts or selected_budget + " token" in texts
        if args.resource_id is not None:
            assert args.resource_id in texts, "Full native resource ID differs from the selected card"
        passed("native detail exposes model, budget, measured-usage caveats and disabled editing explanation")

        top(app)
        app.click("resource_refresh")
        idle(app)
        app.wait(args.model)
        assert any(row.get("i") == "resource_back" for row in app.snap())
        refreshed = app.texts()
        assert selected_budget + " tokens" in refreshed or selected_budget + " token" in refreshed, "Refresh changed the selected public budget"
        if args.resource_id is not None:
            assert args.resource_id in refreshed, "Refresh changed the full native resource ID"
        passed("refresh preserves selected public facts and does not enter a management command")

        app.click("resource_back")
        idle(app)
        app.wait(COPY[args.language][0])
        assert resource_cards(app) == before, "Back changed the filtered resource page"
        app.wait(args.account)
        assert app.draft(args.account).get("command") == original_command, "Saved command changed during read-only navigation"
        app.capture("resource-list-return-" + args.language)
        passed("Back retains resource filter/list and exact saved command without changing account")

        with (app.root / "native.log").open("rb") as log:
            log.seek(app.log_offset)
            errors = [line for line in log.read().decode(errors="replace").splitlines()
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
