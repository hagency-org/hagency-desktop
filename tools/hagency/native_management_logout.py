#!/usr/bin/env python3
"""Exercise real logout/re-login with an open Hagency mini app on localhost.

Use the dedicated manager fixture only. Requires Playwright and the private
test-accounts.json from the isolated server run. Credentials go solely through
the normal isolated browser login and are never saved in the report/trace.
Leaves the manager signed in with the mini app open for further checks.
"""
import argparse
import hashlib
import json
from pathlib import Path
import time
from urllib.parse import urlparse

from native_management_recovery import open_test_miniapp, until
from native_management_wizard import App, ROOT


MANAGER = "@miniapp_manager_1015:localhost:8088"


class WindowsApp(App):
    window = 1

    def request(self, route, **params):
        if route in {"/snap", "/g", "/click", "/m", "/k", "/t"}:
            params.setdefault("w", self.window)
        return super().request(route, **params)

    def click_visible(self, identity):
        for _ in range(15):
            height = next(w["sz"][1] for w in self.request("/s")["w"]
                          if w["i"] == self.window)
            rows = [r for r in self.snap() if r.get("i") == identity
                    or r.get("t") == identity]
            if len(rows) == 1:
                x, y, w, h = rows[0]["r"]
                if h >= 28 and y >= 32 and y + h <= height - 12:
                    self.request("/click", x=x+w/2, y=y+h/2, wait=0)
                    time.sleep(.25)
                    return
            self.request("/m", k="scroll", x=750, y=650, dy=220, wait=0)
            time.sleep(.2)
        raise AssertionError("Native control unavailable: " + identity)

    def wait_resource_read(self):
        self.wait("All capabilities")
        # Filters render before the asynchronous read finishes. A failed or
        # still-running request must not pass using that static page chrome.
        until(lambda: not any(r.get("i") == "notice" and r.get("t")
                              for r in self.snap()))
        self.wait(MANAGER)


def browser_login(run, server_run):
    from playwright.sync_api import sync_playwright

    url = (run / "authorization-url").read_text()
    parsed = urlparse(url)
    assert parsed.scheme == "http" and parsed.hostname == "127.0.0.1"
    assert parsed.port == 8088 and parsed.path.startswith("/auth/")
    user = json.loads((server_run / "test-accounts.json").read_text())["manager"]
    assert "@" + user["username"] + ":localhost:8088" == MANAGER
    with sync_playwright() as engine:
        browser = engine.chromium.launch(
            executable_path="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            headless=True)
        try:
            page = browser.new_page()
            page.goto(url, wait_until="domcontentloaded")
            submitted = False
            deadline = time.monotonic() + 75
            while time.monotonic() < deadline:
                username = page.get_by_placeholder("Username or email", exact=True)
                if not submitted and username.is_visible():
                    username.fill(user["username"])
                    page.locator("input[type=password]").fill(user["password"])
                    page.get_by_role("button", name="Sign in", exact=True).click()
                    submitted = True
                current = urlparse(page.url)
                if submitted and current.hostname == "127.0.0.1" and current.port != 8088:
                    return
                allow = page.get_by_role("button", name="Allow", exact=True)
                if allow.is_visible():
                    allow.click()
                page.wait_for_timeout(250)
            raise AssertionError("Native OAuth callback not reached")
        finally:
            browser.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-run", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    # Check the browser dependency before affecting an existing test session.
    from playwright.sync_api import sync_playwright
    assert callable(sync_playwright)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    run = ROOT / ".run/management-ui-manager"
    app = WindowsApp(run, 8320, captures)
    report = {"passed": False, "checks": [], "binarySha256": app.binary_hash,
              "probeSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "scope": "real localhost UI logout and fresh SDK login; no resource or agent execution"}

    def passed(label):
        report["checks"].append(label)
        print("PASS " + label, flush=True)

    try:
        app.wait(MANAGER)
        app.click("overview")
        app.wait(MANAGER)
        app.capture("management-before-logout")
        app.window = 0
        app.click_visible("open_settings")
        app.click_visible("category_account_button")
        app.click_visible("logout_button")
        app.wait("Are you sure you want to logout?")
        app.click_visible("cancel_button")
        app.window = 1
        app.click("resources")
        app.wait_resource_read()
        passed("cancelling logout preserves the signed-in mini app and a real resource read")

        app.window = 0
        app.click_visible("logout_button")
        app.wait("Are you sure you want to logout?")
        app.click_visible("Log out now")
        app.wait("Sign in to Hagency", timeout=90)
        until(lambda: all(w["i"] != 1 for w in app.request("/s")["w"]))
        assert not any(r.get("t") == MANAGER and r.get("i") == "user" for r in app.snap())
        app.capture("signed-out-miniapp-closed")
        passed("completed logout closes the separate mini-app window and shows sign-in")

        authorization = run / "authorization-url"
        if authorization.exists():
            # create_new protects the instrument login handoff; discard only
            # this fixture's already-used localhost URL before a fresh login.
            old = urlparse(authorization.read_text())
            assert old.hostname == "127.0.0.1" and old.port == 8088
            authorization.unlink()
        app.click_visible("homeserver_input")
        app.request("/k", c="A", cmd=1, wait=0)
        app.request("/t", t="http://127.0.0.1:8088", wait=0)
        app.click_visible("continue_server_button")
        app.wait("Continue with single sign-on")
        app.click_visible("browser_login_button")
        until(authorization.exists)
        browser_login(run, args.server_run)
        app.wait(MANAGER, timeout=45)
        app.window = 1
        open_test_miniapp(app)
        app.wait(MANAGER)
        app.click("resources")
        app.wait_resource_read()
        app.capture("fresh-session-management")
        passed("fresh browser/SDK login reopens management and completes a resource read")

        log = (run / "native.log").read_bytes()[app.log_offset:].decode(errors="replace")
        errors = [line for line in log.splitlines() if "[E]" in line
                  or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native errors; inspect private fixture log"
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error) or type(error).__name__
        try:
            app.capture("logout-failed")
        except Exception:
            pass
        raise
    finally:
        report["trace"] = app.trace
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
