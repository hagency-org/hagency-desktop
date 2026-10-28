#!/usr/bin/env python3
"""Check management-console reads in a real signed-in Makepad window.

Uses the normal UI, original host replies and actual PNG captures. This is not
a provider-execution test. The supplied run must already have a native
instrument session and a separately opened Hagency mini-app window. The report
records whether that SDK provides the user-activity guard.
"""
import argparse
import json
import time
from pathlib import Path

from native_management_logout import WindowsApp


class ConsoleApp(WindowsApp):
    def control(self, identity):
        for _ in range(35):
            rows = [r for r in self.snap()
                    if r.get("i") == identity or r.get("t") == identity]
            height = next(w["sz"][1] for w in self.request("/s")["w"]
                          if w["i"] == self.window)
            if rows:
                x, y, w, h = rows[0]["r"]
                if h >= 20 and y >= 76 and y + h <= height - 12:
                    self.request("/click", x=x+w/2, y=y+h/2, wait=0)
                    time.sleep(.3)
                    return
                delta = -230 if y < 76 else 230
            else:
                delta = 230
            self.request("/m", k="scroll", x=750, y=500, dy=delta, wait=0)
            time.sleep(.18)
        raise AssertionError("Unreachable native control: " + identity)

    def navigation(self, identity):
        rows = [r for r in self.snap() if r.get("i") == identity]
        assert len(rows) == 1, "Ambiguous top-level navigation"
        x, y, w, h = rows[0]["r"]
        self.request("/click", x=x+w/2, y=y+h/2, wait=0)
        time.sleep(.35)

    def wait_data(self, alternatives):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            rows = self.snap()
            if any(marker in (r.get("t") or "")
                   for marker in alternatives for r in rows):
                assert not any(r.get("i") == "notice" and r.get("t")
                               for r in rows), "Console read has a notice"
                return
            time.sleep(.15)
        raise AssertionError("No native data result: " + repr(alternatives))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--port", type=int, default=8321)
    parser.add_argument("--account", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, mode=0o700, exist_ok=False)
    app = ConsoleApp(args.run, args.port, args.output)
    report = {"passed": False, "binarySha256": app.binary_hash,
              "account": args.account, "checks": [],
              "scope": "native console reads; no simulated host replies"}
    try:
        app.wait(args.account)
        app.navigation("overview")
        app.wait("Management console")
        app.capture("overview")
        screens = [
            ("Workforce", ["Requested allocation", "No agents in this view."]),
            ("Capabilities and offers", ["Family / tier", "No resource configurations yet."]),
            ("Usage", ["Authorized resources"]),
            ("Shared account capacity", ["Remaining / account quota", "No provider accounts yet."]),
            ("Fleet alerts", ["All alerts / matching"]),
        ]
        for label, markers in screens:
            app.control(label)
            app.wait_data(markers)
            name = label.lower().replace(" ", "-")
            app.capture(name)
            (args.output / (name + ".json")).write_text(
                json.dumps(app.snap(), ensure_ascii=False, indent=2))
            report["checks"].append(label + " returned real scoped data")
            print("PASS " + label, flush=True)
        app.navigation("connections")
        app.control("web_external_connections")
        app.wait("External Matrix configuration")
        app.control("web_connection_new")
        app.wait("Check server")
        app.capture("external-server-wizard")
        app.control("web_server_cancel")
        report["checks"].append("External-server wizard opens and returns without a write")
        app.navigation("overview")
        app.wait("Management console")
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
        try:
            app.capture("not-completed")
        except Exception:
            pass
        raise
    finally:
        report["trace"] = app.trace
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
