#!/usr/bin/env python3
"""Crash the isolated owner after a real server commit, then recover via UI.

Requires the signed-in loopback fixture used by native_management_wizard.py.
Only the supplied test profile is killed/corrupted. A PostgreSQL table lock and
a temporary SQLite writer lock control timing; neither supplies a fake reply
or writes a success record. The owner is restarted with its frozen test binary.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import select
import signal
import socket
import sqlite3
import subprocess
import time
from urllib.parse import urlsplit
from urllib.error import HTTPError
import uuid

from native_management_wizard import App, ROOT


def until(test, timeout=25):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = test()
        if value:
            return value
        time.sleep(.1)
    raise AssertionError("Timed out waiting for recovery test condition")


class Database:
    def __init__(self, server_run, container):
        server_run = server_run.resolve()
        pid = int((server_run / "server.pid").read_text())
        command = subprocess.check_output(["ps", "-p", str(pid), "-o", "command="], text=True)
        assert str(server_run / "bin/hagency-server") in command, "Not the isolated server"
        config = (server_run / "hagency.toml").read_text()
        url = urlsplit(re.search(r'^database_url\s*=\s*"([^"]+)"', config, re.M).group(1))
        assert url.hostname == "127.0.0.1" and url.port == 55438, "Local fixture only"
        assert container == "hagency-latest-test-postgres-1", "Unexpected database container"
        user = subprocess.check_output(["docker", "exec", container, "printenv", "POSTGRES_USER"], text=True).strip()
        self.command = ["docker", "exec", "-i", container, "psql", "-X", "-Atq", "-U", user,
                        "-d", url.path[1:], "-v", "ON_ERROR_STOP=1"]
        self.lock = None

    def sql(self, statement):
        return subprocess.check_output(self.command, input=statement, text=True, timeout=10).strip()

    def hold(self):
        self.lock = subprocess.Popen(self.command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE, text=True, bufsize=1)
        self.lock.stdin.write("BEGIN; SET LOCAL idle_in_transaction_session_timeout='60s'; "
                              "LOCK TABLE hagency_agent_v1.management_contributions IN ACCESS EXCLUSIVE MODE; "
                              "SELECT 'recovery_lock_held';\n")
        self.lock.stdin.flush()
        assert select.select([self.lock.stdout], [], [], 10)[0], "Database lock unavailable"
        assert self.lock.stdout.readline().strip() == "recovery_lock_held"

    def release(self):
        if self.lock:
            try:
                self.lock.communicate("ROLLBACK;\n", timeout=10)
            finally:
                if self.lock.poll() is None:
                    self.lock.kill()
                self.lock = None


def restart(run, port, output, accept_consent=True):
    def port_closed():
        with socket.socket() as probe:
            return probe.connect_ex(("127.0.0.1", port)) != 0
    # SIGKILL delivery and socket teardown are asynchronous on macOS.
    until(port_closed, timeout=10)
    env = {**os.environ, "HAGENCY_DESKTOP_DATA_DIR": str(run / "profile"),
           "MAKEPAD_REMOTE": str(port), "MAKEPAD_HIDE_WINDOWS": "1", "MAKEPAD_NO_FOCUS": "1",
           "RUST_BACKTRACE": "1", "HAGENCY_INSTRUMENT_LOGIN_URL": str(run / "authorization-url")}
    for key in ("NO_PROXY", "no_proxy"):
        env[key] = ",".join(filter(None, [env.get(key, ""), "127.0.0.1,localhost,::1"]))
    with (run / "native.log").open("a") as log:
        process = subprocess.Popen([str(run / "bin/rinx")], cwd=ROOT, env=env,
                                   stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                                   start_new_session=True)
    (run / "state.json").write_text(json.dumps({"pid": process.pid, "user_seq": None}))

    def attach():
        assert process.poll() is None, "Restarted app exited"
        try:
            app = App(run, port, output)
            if app.request("/s").get("w"):
                return app
        except OSError:
            pass
        return None
    app = until(attach)
    (run / "state.json").write_text(json.dumps({"pid": process.pid, "user_seq": app._user_seq}))
    return open_test_miniapp(app, accept_consent)


def open_test_miniapp(app, accept_consent=True):
    run = app.root
    role = "admin" if run == ROOT / ".run/management-ui" else run.name.removeprefix("management-ui-")
    assert role in ("admin", "owner", "coordinator", "manager"), "Dedicated SDK fixture required"
    account = "@admin:localhost:8088" if role == "admin" else f"@miniapp_{role}_1015:localhost:8088"
    # The sidebar can render before SDK restoration finishes. Opening a
    # privileged mini app at that point does not exercise its signed-in flow.
    def account_visible():
        try:
            rows = app.request("/snap", w=0)["s"]
        except HTTPError as error:
            # The window can be registered before its first inspectable draw.
            if error.code == 404:
                return False
            raise
        if any(row.get("t") == account for row in rows):
            return True
        sidebar = next((row for row in rows if row.get("i") == "persistent_sidebar_frame"), None)
        toggle = next((row for row in rows if row.get("i") == "toggle_sidebar"), None)
        if sidebar and toggle and 0 < sidebar["r"][2] < 100:
            x, y, w, h = toggle["r"]
            app.request("/click", w=0, x=x+w/2, y=y+h/2, wait=0)
            time.sleep(.25)
        return False
    until(account_visible)
    def open_launcher():
        if any(window.get("i") == 1 for window in app.request("/s").get("w", [])):
            return True
        rows = app.request("/snap", w=0)["s"]
        button = next((row for row in rows if row.get("i") == "open_miniapps" and row["r"][2] > 0), None)
        if button:
            x, y, w, h = button["r"]
            app.request("/click", w=0, x=x+w/2, y=y+h/2, wait=0)
            # SDK restoration can reflow the sidebar between snapshot and
            # click. Confirm that a window actually opened before proceeding.
            time.sleep(.25)
            return any(window.get("i") == 1 for window in app.request("/s").get("w", []))
    until(open_launcher)
    app.wait("Hagency")
    rows = app.snap()
    label = next(row for row in rows if row.get("i") == "name" and row.get("t") == "Hagency")
    launches = [row for row in rows if row.get("i") == "launch" and abs(row["r"][1]-label["r"][1]) < 40]
    assert len(launches) == 1
    x, y, w, h = launches[0]["r"]
    app.request("/click", x=x+w/2, y=y+h/2, wait=0)
    # Consent belongs to the actual installed bundle and exact permissions.
    until(lambda: any(row.get("i") in ("run", "hagency_run", "overview", "retry") for row in app.snap()))
    if not accept_consent:
        return app
    if any(row.get("i") == "hagency_run" for row in app.snap()):
        app.click("hagency_run")
    elif any(row.get("i") == "run" for row in app.snap()):
        app.click("run")
    return app


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--owner-run", type=Path, required=True)
    parser.add_argument("--owner-port", type=int, default=8318)
    parser.add_argument("--server-run", type=Path, required=True)
    parser.add_argument("--postgres-container", default="hagency-latest-test-postgres-1")
    parser.add_argument("--owner", required=True)
    parser.add_argument("--coordinator", required=True)
    parser.add_argument("--model", default="GPT-5.6-Sol")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    run = args.owner_run.resolve()
    assert run == ROOT / ".run/management-ui-owner", "Only the dedicated owner test profile may be crashed"
    for account in (args.owner, args.coordinator):
        assert account.startswith("@miniapp_") and account.endswith(":localhost:8088")
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    captures = args.output / "captures"
    captures.mkdir()
    report = {"passed": False, "checks": [], "scope": "real committed admission with lost reply and damaged draft"}
    apps = []
    database = Database(args.server_run, args.postgres_container)
    writer = None
    app = App(run, args.owner_port, captures)
    apps.append(app)
    log_offset = (run / "native.log").stat().st_size
    report["binarySha256"] = app.binary_hash
    def passed(name):
        report["checks"].append(name)
        print("PASS " + name, flush=True)
    try:
        app.wait(args.owner)
        app.click("resources")
        app.wait("All capabilities")
        app.click("add")
        app.wait("Start with your server")
        app.click("next")
        app.wait("Choose models")
        app.click("choose")
        app.wait(args.model)
        app.click(args.model)
        app.click("High")
        app.field("budget", "234567")
        app.click("create")
        app.wait("Request server admission")
        label = "Recovery MacBook " + uuid.uuid4().hex[:8]
        report["resourceLabel"] = label
        app.field("resource_name", label)
        app.field("coordinator", args.coordinator)
        paths = list((run / "profile/native-owner").glob("*/owned-agent-owners/management_*/domain.sqlite3"))
        assert len(paths) == 1, "Ambiguous owner ledger"
        def pending():
            with sqlite3.connect(f"file:{paths[0]}?mode=ro", uri=True, timeout=.2) as db:
                return db.execute("SELECT command_id,input FROM management_intents WHERE state='pending' AND operation=? AND json_extract(input,'$.label')=?",
                                  ('"contribute"', label)).fetchall()
        assert not pending(), "Fixture already has an unfinished admission"
        database.hold()
        app.click("submit")
        rows = until(pending)
        assert len(rows) == 1
        command_id, raw = rows[0]
        command = json.loads(raw)
        assert command["label"] == label and command["command_id"] == command_id
        assert re.fullmatch(r"[a-zA-Z0-9_-]+", command_id)
        report["commandId"] = command_id
        report["inputSha256"] = hashlib.sha256(raw.encode()).hexdigest()
        passed("native intent is durable before the server mutation proceeds")
        # Block only local reply recording, after native custody exists.
        writer = sqlite3.connect(paths[0], timeout=2)
        writer.execute("BEGIN IMMEDIATE")
        database.release()
        receipt_count = lambda: database.sql("SELECT count(*) FROM hagency_agent_v1.management_commands WHERE command_id='" + command_id + "';")
        until(lambda: receipt_count() == "1")
        assert writer.execute("SELECT state,reply FROM management_intents WHERE command_id=?", (command_id,)).fetchone() == ("pending", None)
        state = json.loads((run / "state.json").read_text())
        assert app.request("/s")["pid"] == state["pid"]
        process_command = subprocess.check_output(["ps", "-p", str(state["pid"]), "-o", "command="], text=True).strip()
        assert process_command == str(run / "bin/rinx"), "Unexpected process; refusing kill"
        os.kill(state["pid"], signal.SIGKILL)
        writer.rollback()
        writer.close()
        writer = None
        passed("server committed once while the native reply remained unrecorded, then the test app crashed")
        draft = run / "profile/miniapps" / args.owner.encode().hex() / "im.palpo.operations/onboarding-v1.json"
        report["draftBeforeSha256"] = hashlib.sha256(draft.read_bytes()).hexdigest()
        draft.write_text('{"version":1,"wizard":')
        assert pending()[0][0] == command_id
        app = restart(run, args.owner_port, captures)
        apps.append(app)
        assert app.binary_hash == report["binarySha256"]
        app.wait("Saved requests")
        app.wait("Retry original request")
        app.capture("damaged-draft-native-recovery")
        passed("a corrupted page draft does not hide the pending native request after restart")
        app.click("recover_native")
        app.wait(label)
        app.wait("Awaiting administrator")
        app.capture("original-request-recovered")
        def acknowledged():
            with sqlite3.connect(f"file:{paths[0]}?mode=ro", uri=True) as db:
                return db.execute("SELECT state,input,reply FROM management_intents WHERE command_id=?", (command_id,)).fetchone() == ("acknowledged", None, None)
        until(acknowledged)
        assert receipt_count() == "1"
        assert database.sql("SELECT count(*) FROM hagency_agent_v1.management_contributions WHERE label='" + label + "';") == "1"
        generations = database.sql("SELECT c.device_generation,d.generation FROM hagency_agent_v1.management_commands c JOIN hagency_agent_v1.devices d ON d.id=c.device_id WHERE c.command_id='" + command_id + "';")
        original_generation, current_generation = map(int, generations.split("|"))
        assert current_generation > original_generation, "Restart must exercise real authorization rotation"
        report["executorGenerations"] = {"original": original_generation, "current": current_generation}
        assert not pending()
        assert app.draft(args.owner)["command"] is None
        passed("UI recovery reused the original ID, created no duplicate and acknowledged the recorded result")
        with (run / "native.log").open("rb") as log:
            log.seek(log_offset)
            errors = [line for line in log.read().decode(errors="replace").splitlines()
                      if "[E]" in line or "panicked at" in line or "on_render closure failed" in line]
        report["nativeScriptErrors"] = len(errors)
        assert not errors, "Native runtime errors during recovery; inspect retained log"
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
