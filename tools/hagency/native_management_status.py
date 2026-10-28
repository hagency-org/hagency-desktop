#!/usr/bin/env python3
"""Check actual list/detail status consistency and five-minute expiry, in EN/CN.

Requires the dedicated loopback owner and Chinese/dark coordinator sessions,
an admitted resource from the recovery/reconnection probe, and real provider
access. Makes no database writes and injects no UI state, host replies or clock.
"""
import argparse
import json
from pathlib import Path
import re
import time

from native_management_reconnection import open_connection
from native_management_recovery import until
from native_management_wizard import App, ROOT


def row_status(app, label):
    rows = app.snap()
    title = next((row for row in rows if row.get("t") == label), None)
    if title is None:
        return None
    following = sorted((row for row in rows if row.get("t") and
                        0 < row["r"][1] - title["r"][1] < 70),
                       key=lambda row: row["r"][1])
    if not following:
        return None
    status = following[0]
    # Inspection also contains children outside the scrolling viewport. A
    # clipped status must not satisfy a visible-expiry check or its capture.
    # These fixtures use the same 1000x800 native window as App.click.
    if not all(240 <= row["r"][1] and row["r"][1] + row["r"][3] <= 760
               for row in (title, status)):
        return None
    return status["t"]


def find_row(app, page, label, expected):
    app.click(page)
    until(lambda: not any(value in app.texts() for value in ("Updating…", "正在更新…")))
    for _ in range(32):
        status = row_status(app, label)
        if status is not None:
            assert expected in status, f"Wrong {page} status: {status!r}"
            return
        app.request("/m", k="scroll", x=750, y=650, dy=180, wait=0)
        time.sleep(.2)
    raise AssertionError("Target row was not reachable: " + page)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reconnection-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    previous = json.loads(args.reconnection_report.read_text())
    assert previous["passed"] is True
    label = previous["resourceLabel"]
    assert re.fullmatch(r"Recovery MacBook [a-f0-9]{8}", label)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    owner = App(ROOT / ".run/management-ui-owner", 8318, captures)
    coordinator = App(ROOT / ".run/management-ui-coordinator", 8319, captures)
    report = {"passed": False, "checks": [], "resourceLabel": label,
              "binaries": {"owner": owner.binary_hash, "coordinator": coordinator.binary_hash}}
    apps = [owner, coordinator]
    def passed(name):
        report["checks"].append(name)
        print("PASS " + name, flush=True)
    try:
        assert owner.binary_hash == coordinator.binary_hash
        owner.wait("@miniapp_owner_1015:localhost:8088")
        coordinator.wait("@miniapp_coordinator_1015:localhost:8088")
        open_connection(owner, label)
        if any(row.get("i") == "resume" for row in owner.snap()):
            owner.click("resume")
            # Resume disappears while the native provider check is still in
            # flight. Wait for its result, not merely the missing old button.
            until(lambda: any(row.get("i") == "verify" for row in owner.snap()))
        owner.click("verify")
        owner.wait("Connection verified")
        started = time.monotonic()
        owner.capture("owner-detail-verified")
        find_row(owner, "connections", label, "Connection verified")
        owner.capture("owner-list-verified")
        find_row(owner, "overview", label, "Connection verified")
        passed("English Overview, Connections and detail show the same verified state")
        find_row(coordinator, "overview", label, "连接已验证")
        find_row(coordinator, "connections", label, "连接已验证")
        open_connection(coordinator, label, "准入审批与连接验证是两个步骤")
        coordinator.wait("连接已验证")
        coordinator.wait("资源主人可使用登记的设备重新检查连接。")
        assert "资源主人需要使用登记的执行设备完成验证。" not in coordinator.texts()
        assert not any(row.get("i") in ("approve", "reject", "resume", "verify", "revoke")
                       for row in coordinator.snap())
        coordinator.capture("coordinator-detail-cn-dark-verified")
        find_row(coordinator, "connections", label, "连接已验证")
        coordinator.capture("coordinator-list-cn-dark-verified")
        passed("Chinese coordinator sees the same state without owner/admin actions")
        print("Waiting for the actual five-minute deadline; neither list is refreshed", flush=True)
        deadline = started + 330
        next_progress = time.monotonic() + 45
        while time.monotonic() < deadline:
            owner_status = row_status(owner, label)
            coordinator_status = row_status(coordinator, label)
            if (owner_status and "Connection needs a fresh check" in owner_status and
                    coordinator_status and "连接需要重新检查" in coordinator_status):
                break
            if time.monotonic() >= next_progress:
                print(f"Deadline wait: {round(time.monotonic() - started)} seconds", flush=True)
                next_progress += 45
            time.sleep(2)
        else:
            raise AssertionError("A visible list retained the expired verified state")
        report["expiryObservedAfterSeconds"] = round(time.monotonic() - started, 2)
        assert report["expiryObservedAfterSeconds"] >= 290, "Verification expired unexpectedly early"
        owner.capture("owner-overview-expired")
        coordinator.capture("coordinator-list-cn-dark-expired")
        passed("visible Overview and Connections expire without navigation or refresh")
        open_connection(owner, label)
        owner.wait("Connection needs a fresh check")
        passed("detail agrees with the server after the list expiry")
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
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
