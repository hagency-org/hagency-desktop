#!/usr/bin/env python3
"""Validate real EN/CN native permission review in the isolated test profiles.

Requires newly built frozen binaries and unreviewed bundle digests in the manager
and coordinator fixtures. No grant file or host response is fabricated.
"""
import argparse
import json
from pathlib import Path
import time

from native_management_projects import select_card
from native_management_recovery import restart
from native_management_wizard import App, ROOT


def check(app, label, report):
    rows = app.snap()
    run = next(row for row in rows if row.get("i") == "hagency_run")
    assert 32 <= run["r"][1] and sum(run["r"][1::2]) <= 790
    permissions = next(row for row in rows if row.get("i") == "permissions")
    expected = json.loads((ROOT / "apps/hagency/bundle/manifest.json").read_text())["capabilities"]
    count = len(permissions["t"].splitlines())
    assert count == len(expected), f"Disclosed {count} of {len(expected)} manifest capabilities"
    assert not any(row.get("i") in ("path", "room", "local_key", "endpoint", "token") for row in rows)
    app.capture(label + "-permissions-top")
    app.request("/m", k="scroll", x=700, y=500, dy=100000, wait=0)
    time.sleep(.4)
    rows = app.snap()
    assert next(row for row in rows if row.get("i") == "hagency_run")["r"] == run["r"]
    ending = next(row for row in rows if "exact bundled version" in (row.get("t") or "")
                  or "此版本记住授权" in (row.get("t") or ""))
    assert 32 <= ending["r"][1] and ending["r"][1] + ending["r"][3] < run["r"][1]
    app.capture(label + "-permissions-bottom")
    app.click("hagency_cancel")
    app.wait("Hagency")
    assert any(row.get("i") == "catalog_list" for row in app.snap())
    select_card(app, "Hagency", "launch")
    assert any(row.get("i") == "hagency_run" for row in app.snap()), "Cancel must not remember consent"
    app.click("hagency_run")
    app.wait("Hagency")
    app.wait("@miniapp_" + label + "_1015:localhost:8088")
    assert not any(row.get("i") == "hagency_run" for row in app.snap())
    report["checks"].append(label + ": all permissions disclosed, footer fixed while scrolling, Cancel grants nothing, Run opens app")
    print("PASS " + report["checks"][-1], flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    report = {"passed": False, "checks": [], "scope": "actual native Hagency permissions review in English/light and Chinese/dark"}
    apps = []
    try:
        for role, port in [("manager", 8320), ("coordinator", 8319)]:
            run = ROOT / (".run/management-ui-" + role)
            # Binaries must already be replaced while stopped by the caller.
            app = restart(run, port, captures, accept_consent=False)
            apps.append(app)
            report.setdefault("binaries", {})[role] = app.binary_hash
            check(app, role, report)
        errors = []
        for app in apps:
            with (app.root / "native.log").open("rb") as log:
                log.seek(app.log_offset)
                errors += [line for line in log.read().decode(errors="replace").splitlines()
                           if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native script errors; inspect retained logs"
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error) or type(error).__name__
        if apps:
            try:
                apps[-1].capture("permissions-failed")
            except Exception:
                pass
        raise
    finally:
        report["trace"] = [app.trace for app in apps]
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
