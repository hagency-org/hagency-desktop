#!/usr/bin/env python3
"""Release checks for budget authority and chat regressions; no live account calls.

python3 tools/check-hagency-regressions.py --output .run/regressions
Includes the in-tree Client library HTTP loss/replay tests.
Server PostgreSQL integration tests must separately use a disposable database.

--jobs N runs up to N checks side by side; the default is one after another.
The commands, their logs and the report are the same either way. Cargo itself
lets only one command at a time build into a target folder, so side by side
it is the tests that overlap, not the builds. Two commands that build the
programs of the same package never overlap: Cargo keeps one copy of such a
program per target folder and replaces it when the features differ.
Side by side as one after another, a check gets what the checks listed
before it built for it (the script VM, the two probe programs) and nothing
of a check listed after it.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
CLIENT = ROOT / "crates/hagency-client"


def build_seconds(text):
    """How long Cargo says it took until everything was built, from a check's
    log. Side by side this includes waiting for another command's build."""
    total = 0.0
    # Where Cargo is told to colour its output, as in CI, "Finished" is wrapped
    # in escape codes.
    text = re.sub(r"\x1b\[[0-9;]*m", "", text)
    for minutes, seconds in re.findall(r"^\s*Finished .* in (?:(\d+)m )?([\d.]+)s\s*$", text, re.M):
        total += int(minutes or 0) * 60 + float(seconds)
    return total


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cargo-config", type=Path,
                        help="Optional pinned framework config for the Desktop root checks")
    parser.add_argument("--start-at", help="Resume at this failed check; earlier recorded passes must be unaffected by the fix")
    parser.add_argument("--jobs", type=int, default=int(os.environ.get("HAGENCY_REGRESSION_JOBS") or 1),
                        help="How many checks run side by side (default 1, or HAGENCY_REGRESSION_JOBS)")
    parser.add_argument("--second-run", action="store_true",
                        help="Run a check once more when its tests ran and one failed; it passes if that run "
                             "passes, and the report and the end of the output name it. For the checks of a "
                             "pull request. A release leaves this out: there every check passes the first time.")
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be at least 1")
    if args.jobs > 1 and args.cargo_config:
        # That mode compares Cargo.lock before and after each command.
        parser.error("--cargo-config runs the checks one after another; leave --jobs at 1")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.setdefault("RUST_MIN_STACK", "16777216")
    previous = {}
    if args.start_at:
        previous = {row['check']: row for row in json.loads((output / 'report.json').read_text())}
    reached_start = not bool(args.start_at)
    started = time.monotonic()
    pool = ThreadPoolExecutor(max_workers=args.jobs)
    lock = threading.Lock()
    rows = {}        # place in the list -> report row, for every check that ended
    logs = {}        # place in the list -> its log file
    futures = {}     # check name -> its future
    failed = []      # places of the checks that failed
    programs = {}    # (folder, package) -> lock held while its programs are in use
    handed = {}      # variables a check hands to the checks listed after it
    handing = []     # the checks that hand something on: later ones wait for them
    second = []      # the checks that passed only on their second run, and the tests that had failed
    order = 0

    def stopped(index):
        # Checks start in list order. Once one fails, none listed after it
        # starts, and every check listed before it still runs to its end: so
        # the first failed check in the list is the one to resume at.
        with lock:
            return bool(failed) and index > min(failed)

    def record(index, row):
        with lock:
            rows[index] = row
            if not row["passed"]:
                failed.append(index)
            (output / "report.json").write_text(json.dumps([rows[i] for i in sorted(rows)], indent=2) + "\n")
            took = row.get("seconds")
            print(row["check"], "PASS" if row["passed"] else "FAIL", "" if took is None else f"{took:.0f}s", flush=True)

    def builds_programs_of(command, cwd):
        # An integration test starts its package's programs from the one place
        # Cargo keeps them. A command that builds them with other features
        # replaces them there, so such commands take turns per package.
        if command[0] != "cargo" or not {"--test", "--all-targets", "--bin"} & set(command):
            return None
        package = command[command.index("-p") + 1] if "-p" in command else ""
        manifest = command[command.index("--manifest-path") + 1] if "--manifest-path" in command else ""
        return (str(cwd), package, manifest)

    def execute(index, *check):
        try:
            return check_once(index, *check)
        except BaseException:
            # Not a failed check but a fault of this script: start nothing more.
            with lock:
                failed.append(index)
            raise

    def check_once(index, name, command, cwd, rust_tests, run_env, native_lock, needs, late, hands_on):
        for need in needs:
            futures[need].result()
        if late:
            with lock:
                run_env.update(handed)
        key = builds_programs_of(command, cwd)
        with lock:
            turn = programs.setdefault(key, threading.Lock()) if key else None
        if turn:
            turn.acquire()
        try:
            if stopped(index):
                return None
            log = logs[index]
            begin = time.monotonic()

            def once():
                with log.open("w") as stream:
                    try:
                        return subprocess.run(command, cwd=cwd, env=run_env, stdout=stream, stderr=subprocess.STDOUT).returncode
                    except OSError as error:
                        stream.write(f"Could not start {command[0]}: {error}\n")
                        return 127

            code = once()
            text = log.read_text()
            failed_first = None
            if code != 0 and args.second_run and re.search(r"^test result: FAILED\.", text, re.M):
                # The tests ran and one failed: not a build that failed, which
                # would fail again. One test now and then misses a time limit
                # while other checks use the machine. Still this package's turn.
                failed_first = re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)
                print(f"{name} tests failed, running it once more: {', '.join(failed_first) or 'no test named'}", flush=True)
                log.replace(output / (name + ".first.log"))
                code = once()
                text = log.read_text()
            took = time.monotonic() - begin
            if code == 0 and hands_on:
                # Still this package's turn: what was built is what is kept.
                try:
                    hands_on(text)
                except ValueError as error:
                    with log.open("a") as stream:
                        stream.write(f"\n{error}\n")
                    code = 1
        finally:
            if turn:
                turn.release()
        count = sum(int(n) for n in re.findall(r"test result: ok\. (\d+) passed;", text))
        unchanged = native_lock is None or native_lock == (ROOT / 'Cargo.lock').read_bytes().split(b'[[patch.unused]]', 1)[0]
        passed = code == 0 and unchanged and (not rust_tests or count > 0)
        if not unchanged:
            with log.open('a') as stream:
                stream.write('\nNative framework check changed resolved Cargo.lock packages; reconcile and review before retrying.\n')
        row = {"check": name, "passed": passed, "tests": count if rust_tests else None,
               "seconds": round(took, 1), "build_seconds": round(build_seconds(text), 1)}
        if failed_first is not None:
            row["failed_first"] = failed_first
            if passed:
                with lock:
                    second.append((index, name, failed_first))
        record(index, row)
        return text

    def name_second_runs():
        with lock:
            found = sorted(second)
            second.clear()
        for _, name, tests in found:
            print(f"Passed only on a second run: {name} ({', '.join(tests) or 'no test named'})", flush=True)
            print(f"::warning title=Passed only on a second run::{name}: {', '.join(tests) or 'no test named'} "
                  "failed and passed when the check ran once more.", flush=True)

    def finish():
        """Wait for every check started so far; stop here if one failed."""
        for future in list(futures.values()):
            future.result()
        with lock:
            bad = sorted(failed)
        if not bad:
            return
        name_second_runs()
        pool.shutdown()
        log = logs[bad[0]]
        # Failed-check files disappear with ephemeral CI runners.
        print(f"Last 200 lines of {log.name}:", flush=True)
        print("\n".join(log.read_text().splitlines()[-200:]), flush=True)
        if len(bad) > 1:
            print("Also failed:", ", ".join(rows[i]["check"] for i in bad[1:]), flush=True)
        raise SystemExit(f"Inspect {log}; no release until this check passes")

    def run(name, command, cwd=ROOT, rust_tests=False, needs=None, hands_on=None):
        """Start one check. `needs` names a check that must end first.
        `hands_on` is called with the check's log once it has passed, fills
        `handed` for every check listed after this one, and raises ValueError
        when the log lacks what it looks for. Returns the kept log of a check
        that an earlier run passed and this run reuses, otherwise nothing."""
        nonlocal reached_start, order
        index = order
        order += 1
        if not reached_start:
            reached_start = name == args.start_at
            if not reached_start:
                prior = previous.get(name)
                log = output / (name + '.log')
                if not prior or not prior['passed'] or not log.is_file():
                    raise SystemExit(f'Cannot reuse an unproven check: {name}')
                with lock:
                    rows[index] = prior
                print(name, 'REUSED', flush=True)
                return log.read_text()
        # Standalone tool/crate workspaces retain their own pinned lockfiles.
        # The native-accessibility candidate uses the reviewed framework patch
        # only for its integrating Desktop workspace.
        native_lock = None
        if args.cargo_config and cwd == ROOT and command[0] == "cargo" and "--manifest-path" not in command:
            # Cargo reports a lock update for overlapping Git/path patch-unused
            # entries even when resolving writes identical bytes. Keep the
            # dependency invariant explicit: offline resolution plus an exact
            # comparison of every resolved package (unused patch bookkeeping
            # is outside that graph), rather than waiving the gate.
            native_lock = (ROOT / 'Cargo.lock').read_bytes().split(b'[[patch.unused]]', 1)[0]
            command = [part for part in command if part != '--locked']
            if '--offline' not in command:
                command.insert(1, '--offline')
            command = [command[0], "--config", str(args.cargo_config.resolve()), *command[1:]]
        logs[index] = output / (name + ".log")
        # What a check listed before this one handed on, and nothing of a later one.
        with lock:
            run_env = {**env, **handed}
        if command[0] == "cargo" and "CARGO_TARGET_DIR" not in run_env and "--manifest-path" not in command:
            # Swift/Clang modules store absolute cache paths. Reusing the same
            # target through its worktree symlink and its real path otherwise
            # loads each module twice and can crash the compiler.
            target = cwd / "target"
            if target.is_symlink():
                run_env["CARGO_TARGET_DIR"] = str(target.resolve())
        # A check reused from an earlier run is not waited for.
        needs = [need for need in ([needs] if needs else []) + handing if need in futures]
        futures[name] = pool.submit(execute, index, name, command, cwd, rust_tests, run_env, native_lock,
                                    needs, bool(handing), hands_on)
        if hands_on:
            handing.append(name)
        if args.jobs == 1:
            # One after another, a failed check stops the list where it failed.
            futures[name].result()
            finish()
        return None

    run("native-minimal-agent-chat", ["cargo", "check", "--locked", "--no-default-features", "--features", "agent_chat"])
    run("owner-source-pins", ["python3", "tools/check-owner-source-pins.py"])
    run("owner-source-boundary", ["python3", "tools/test-owner-source-pins.py"])
    run("package", ["bash", "tools/package-system-apps/check.sh"])
    run("script-vm", ["cargo", "build", "--locked", "--manifest-path", "tools/hagency-script-review/Cargo.toml"])
    vm = ROOT / "tools/hagency-script-review/target/debug/hagency-script-review"
    for name in ["check_full_bundle", "check_agent_budget", "check_agent_control", "check_agent_work", "check_pool_application", "check_entry_workspaces", "check_account_identity", "check_accounts", "check_owner_tasks", "check_encrypted_approvals", "check_pool_fixes", "check_shared_web", "check_settings_stability", "check_background_observations", "check_publication_first", "check_owner_dashboard_approvals", "check_interrupted_calls", "check_admin_admission"]:
        run(name, ["python3", f"apps/hagency/tests/{name}.py", str(vm), "--output", str(output / name)], needs="script-vm")
    # Keep these together: a budget fix must not erase rendering, activity or answer actions.
    for name, pattern in [
        ("budget-host", "host::hagency::budget"),
        ("host-stack", "host::hagency::cache_tests::host_request_"),
        ("token-units", "token_unit_tests"),
        ("native-resource-units", "host::hagency::enrollment"),
        ("publication-host", "host::hagency::grants"),
        ("recurring-period-display", "host::hagency::pools::recurring_display_tests"),
        ("whitelist-accounts", "host::hagency::grant_people"),
        ("friend-search", "home::mobile::new_chat_tests"),
        ("space-management", "home::space_management::tests"),
        ("room-creation-journal", "home::creation_journal::tests"),
        ("publication-ui", "grant_ui::"),
        ("resource-ceiling-ui", "resource_edit_ui::tests"),
        ("agent-chat", "agent_chat"),
        ("current-agent-registry", "agent_access::"),
        ("miniapp-native-overlays", "app::back_navigation_tests::"),
        ("agent-sidebar-palette", "navigation_tab_bar::rail_tests::"),
        ("agent-question-cards", "hagency::pending_questions::tests"),
        ("agent-run-cards", "hagency::run_controls::tests"),
        ("shared-worker", "hagency::backend::shared_worker::tests"),
        ("chat-mermaid", "shared::chat_mermaid::tests"),
        ("send-session-expiry", "expired_send_queue_token_prompts_for_session_recovery"),
        ("device-key-recovery", "settings::crypto_settings"),
        ("device-verification-scopes", "verification_modal::tests"),
        ("device-recovery-session", "matrix_context::tests::recovery_session_pin"),
        ("recovery-room-resume", "crypto_recovery_resume_tests"),
        ("recovery-settings-resume", "recovery_closing_settings"),
        ("matrix-link-navigation", "matrix_links::tests"),
        ("link-preview", "link_preview"),
        ("linkify", "linkify"),
        ("activity", "shared_permission_notice"),
        ("unknown-work-review", "hagency::work_review::tests"),
        ("permission-error-classification", "hagency::permission_failure::"),
        ("permission-polling-policy", "home::permission_retry::tests"),
        ("device-sessions", "settings::device_sessions::tests"),
        ("system-notifications", "system_notifications::tests"),
        ("media-playback", "shared::attachment_download::playback_tests"),
        ("space-overflow", "home::spaces_bar::overflow_tests"),
        ("theme", "theme::"),
        ("theme-contrast", "semantic_token_pairs_meet_wcag_aa"),
        ("chat-information", "home::mobile_chat_info"),
        ("agent-invitations", "hagency::backend::binding_recovery_tests"),
        ("modal-navigation", "app::back_navigation_tests"),
        ("upload-preview", "shared::file_upload_modal::tests"),
        ("private-share-storage", "temp_storage::tests"),
        ("private-share-presentation", "shared::attachment_download::"),
        ("temporary-quota", "transfer_budget::tests"),
        ("attachment-downloads", "attachment_transport::tests"),
        ("media-previews", "media_transport::tests"),
        ("sdk-media-cache", "media_cache_reader::tests"),
        ("image-memory", "memory_budget::"),
        ("encoded-media-cache", "media_cache::"),
        ("avatar-memory", "avatar_cache::tests"),
        ("timeline-update-retention", "timeline_updates::tests"),
        ("timeline-residency", "sliding_sync::retention_tests"),
        ("timeline-draft-retention", "retention_ui_tests"),
        ("image-viewer-lifetime", "shared::image_viewer::lifetime_tests"),
        ("compact-mentions", "shared::mention"),
        ("chat-information-layout", "desktop_information_columns_tests"),
        ("system-update-disclosures", "home::room_screen::system_updates::"),
        ("typing-expiry", "room::typing_notice::"),
        ("typing-broadcast", "room::room_input_bar::typing_broadcast_tests"),
        ("composer-send", "composer_send_tests"),
        ("trace-backpressure", "trace_backpressure_tests"),
        ("chat-layout", "home::main_desktop_ui::chat_layout_tests"),
        ("miniapp-dock", "mini_apps_side_tests"),
        ("owner-approval-navigation", "owner_approvals::tests"),
        ("owner-dashboard-host", "hagency::owner_dashboard::tests"),
        ("owner-hosting", "hagency::hosting::tests"),
        ("current-device-setup-ui", "adoption_ui::"),
        ("current-device-setup-host", "host::hagency::adoption"),
        ("shared-worker-status", "shared_protocol_"),
        ("runtime-start-contract", "runtime_start_button_body_matches_the_native_request_parser"),
        ("current-data-directory", "data_directory_tests::"),
        ("removed-resource-services", "removed_resource_services_are_not_dispatchable"),
        ("owner-daemon-handoff", "hagency::backend::retirement_tests"),
        ("owner-renewal-warnings", "renewal_warning_tests"),
        ("article-editor", "article_app::"),
    ]:
        run(name, ["cargo", "test", "--locked", "--lib", "--features", "onboarding-test-support", pattern], rust_tests=True)
    run("native-share-bridge", ["cargo", "test", "--locked", "-p", "robius-share", "--lib"], rust_tests=True)
    if sys.platform == "darwin":
        run("apple-chat-fonts", ["cargo", "test", "--locked", "--lib", "--features", "onboarding-test-support", "apple_fonts", "--", "--test-threads=1"], rust_tests=True)
        run("apple-system-fonts", ["cargo", "test", "--locked", "--manifest-path", "crates/article-makepad/Cargo.toml", "--lib", "apple_fonts", "--", "--test-threads=1"], rust_tests=True)
    run("answer-export", ["cargo", "test", "--locked", "--manifest-path", "crates/answer-export/Cargo.toml", "--lib"], rust_tests=True)
    # Separate workspace: Desktop's --lib gate does not execute the
    # owner-dashboard crate's own tests.
    run("owner-dashboard", ["cargo", "test", "--locked", "--manifest-path", "crates/owner-dashboard/Cargo.toml", "--lib"], rust_tests=True)
    run("native-question-protocol", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "agent_questions::tests"], cwd=CLIENT, rust_tests=True)
    run("native-run-card-protocol-recovery", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "run_controls"], cwd=CLIENT, rust_tests=True)
    run("native-run-controls", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--lib", "controls::"], cwd=CLIENT, rust_tests=True)
    run("native-shared-run-controls", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--test", "session", "run_controls::"], cwd=CLIENT, rust_tests=True)
    run("native-claude-run-controls", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--test", "runtime", "controls::"], cwd=CLIENT, rust_tests=True)
    run("native-claude-long-stream", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--test", "runtime", "long_stream::"], cwd=CLIENT, rust_tests=True)
    run("native-run-preview", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--lib", "live_text::"], cwd=CLIENT, rust_tests=True)
    run("native-cooperative-host-files", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--lib", "codex::host_files::tests"], cwd=CLIENT, rust_tests=True)
    run("native-postpaid-personal", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--all-targets", "postpaid"], cwd=CLIENT, rust_tests=True)
    run("native-personal-room-bridge", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--lib", "room_"], cwd=CLIENT, rust_tests=True)
    run("native-personal-claude-room", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--test", "runtime", "room::"], cwd=CLIENT, rust_tests=True)
    run("native-question-recovery", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "shared_execution::questions::", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-question-recovery-auth", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "question_recovery_requires_authenticated_events_in_encrypted_rooms"], cwd=CLIENT, rust_tests=True)
    run("native-question-adapter", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--all-targets", "user_input"], cwd=CLIENT, rust_tests=True)
    run("native-room-history", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "console::native::shared_execution::history::", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-room-history-fields", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "console::owned_runtime::e2ee::history::tests", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-key-wakeup", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "shared_work_sync_wakes", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-room-history-protocol", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--test", "session", "native_room_history", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-server-management", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "server_management", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-adopted-server-administration", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "server_administration", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-server-journal", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "server_journal", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-profile", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "current_profile", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-management-contracts", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "current_management_", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-approval-cards", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--test", "approvals", "card::", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-approval-delivery", ["cargo", "test", "--locked", "-p", "hagency-matrix", "--lib", "approval_", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-room-custody", ["cargo", "test", "--locked", "-p", "hagency-matrix", "--lib", "native_provisioning_inline_rooms", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-token-reattach", ["cargo", "test", "--locked", "-p", "hagency-matrix", "--test", "token_provision", "native_token_account_reattach_only_reads", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-profile-edit-contract", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "personal_model_digest_protocol", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-device-setup", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "device_setup::tests", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-management-intents", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--test", "management_intents", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-profile-identity", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "current_profile::", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-owner-daemon", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "daemon::", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-owner-telemetry", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "console::native::telemetry::", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-metering-ledger", ["cargo", "test", "--locked", "-p", "hagency-metering", "--test", "dashboard"], cwd=CLIENT, rust_tests=True)
    run("native-confirmation-deadlines", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "daemon::bridge::reviews::deadline_tests"], cwd=CLIENT, rust_tests=True)
    run("native-recurring-quota", ["cargo", "test", "--locked", "-p", "hagency-core", "--lib", "recurring_quota"], cwd=CLIENT, rust_tests=True)
    run("native-pool-wire", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "split_pool_limits_tests"], cwd=CLIENT, rust_tests=True)
    run("native-pool-publication", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "native_allocation_transport_", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-recurring-transport", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "recurring_transport_tests"], cwd=CLIENT, rust_tests=True)
    run("native-room-workspaces", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--lib", "room_workspace"], cwd=CLIENT, rust_tests=True)
    run("native-room-workspace-launch", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "shared_tools_first_launch", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-room-workspace-protocol", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--test", "managed_qualification", "shared_room_workspace", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-room-workspace-approvals", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "shared_execution::approvals::tests", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    for name, package, pattern in [
        ("native-agent-permissions", "hagency-store", "execution_policy"),
        ("native-agent-web-search", "hagency-store", "web_policy"),
        ("native-pool-policy-inheritance", "hagency-store", "pool_agent_inherits_owner_permissions_and_network"),
        ("native-pool-owner-renewal", "hagency", "pool_owner_renewal_policy"),
        ("native-permission-recovery", "hagency", "execution_policy"),
    ]:
        run(name, ["cargo", "test", "--locked", "-p", package, "--features", "shared-budget-execution,resource-adoption", "--lib", pattern, "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-owner-restart", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "owner_host::tests", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    for name, pattern in [("native-publication-form", "publication_form"), ("native-publication-recovery", "native_grant_publication_")]:
        run(name, ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", pattern, "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-model-qualification", ["cargo", "test", "--locked", "-p", "hagency-core", "--test", "qualification"], cwd=CLIENT, rust_tests=True)
    run("native-model-catalog", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "resource_onboarding::tests"], cwd=CLIENT, rust_tests=True)
    run("native-resource-creation", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "additional_configuration_", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    def keep(executable, fresh):
        # Later checks build this package with other features, and Cargo then
        # replaces the program where it stands. The checks that are handed the
        # program by path get this build of it, whatever is built beside them.
        kept = output / "programs" / Path(executable).name
        if fresh or not kept.is_file():
            kept.parent.mkdir(exist_ok=True)
            shutil.copy2(executable, kept)
        return str(kept)

    def hand_on_probes(text, fresh=True):
        artifacts = [json.loads(line) for line in text.splitlines() if line.startswith("{")]
        found = {}
        for variable, target, kind in [
            ("HAGENCY_TEST_RUNTIME_PROBE", "hagency-managed-runtime-probe", "context"),
            ("HAGENCY_TEST_CLAUDE_RUNTIME_PROBE", "hagency-runtime-probe", "Claude"),
        ]:
            built = [item["executable"] for item in artifacts if item.get("reason") == "compiler-artifact" and item.get("target", {}).get("name") == target and item.get("executable")]
            if len(built) != 1:
                raise ValueError(f"Expected one freshly built offline {kind} probe")
            found[variable] = built[0]
        with lock:
            handed.update({variable: keep(path, fresh) for variable, path in found.items()})

    # The checks listed before this build never see its two programs; the
    # checks listed after it wait for it and are handed the kept copies.
    reused = run("native-context-probe", ["cargo", "build", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--bin", "hagency-managed-runtime-probe", "--bin", "hagency-runtime-probe", "--message-format=json"], cwd=CLIENT, hands_on=hand_on_probes)
    if reused is not None:
        try:
            hand_on_probes(reused, fresh=False)
        except ValueError as error:
            raise SystemExit(str(error))
    run("native-claude-runtime-protocol", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--lib", "--test", "claude", "--test", "claude_control", "--test", "claude_owned", "--test", "claude_session", "--test", "claude_usage", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-shared-claude-custody", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "shared_claude", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-shared-claude-lease", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--test", "claude_owned", "shared_claude"], cwd=CLIENT, rust_tests=True)
    run("native-shared-claude-approval", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "shared_claude_encrypted_approval"], cwd=CLIENT, rust_tests=True)
    run("native-shared-claude-host", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "native_shared_claude_full_host", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    for name, pattern in [
        ("native-claude-renewal", "claude_run::authority::tests"),
        ("native-claude-host-errors", "claude_run::host_tests"),
        ("native-claude-owner-time-policy", "claude_run::policy::tests"),
        ("native-claude-workspace-lifetime", "claude_workspace::"),
        ("native-control-cancel-requeue", "canceled_action_membership_lookup_requeues_the_entire_taken_batch"),
        ("native-current-runtime-intents", "obsolete_runtime_intents_do_not_restore_or_block_a_fresh_start_or_stop"),
        ("native-current-service-options", "obsolete_service_preferences_reset_before_current_options_are_saved"),
        ("native-fleet-atomic-identity", "native_fleet_identity"),
        ("native-current-tool-receipts", "native_tool_grant"),
        ("native-current-binding-receipts", "native_approval_binding"),
        ("native-current-policy-receipts", "native_execution_policy"),
        ("native-shared-worker-header", "shared_protocol_"),
        ("native-current-graph-json", "current_graph_json_"),
        ("native-current-connection", "current_connection_requires_explicit_reconnect_and_keeps_approval_and_custody"),
        ("native-current-contribution", "current_contribution_provenance_is_durable_before_http_and_survives_rejection_and_lost_reply"),
        ("native-current-namespace", "native_namespace_launch_after_current_setup_uses_selected_home_and_stops"),
        ("native-current-provider-quota", "native_idle_quota_after_current_setup_is_account_bound_and_revocable"),
    ]:
        run(name, ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", pattern, "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    for name, pattern in [
        ("native-claude-stop-settlement", "native_shared_claude_daemon_stop_collects_final_output_usage"),
        ("native-claude-idle-settlement", "native_shared_claude_explicit_owner_idle_policy_settles_silent_child"),
    ]:
        run(name, ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", pattern, "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-shared-session", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "shared_native_session_", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-cold-resume-usage", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--lib", "codex::rollout_usage::tests"], cwd=CLIENT, rust_tests=True)
    run("native-personal-cold-usage", ["cargo", "test", "--locked", "-p", "hagency-agent-local", "--lib", "codex::native_usage::tests"], cwd=CLIENT, rust_tests=True)
    run("native-renewable-activity", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--lib", "renewable_activity_keeps_exact_epochs"], cwd=CLIENT, rust_tests=True)
    run("native-run-card-startup", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "native_shared_run_card_device_delay_does_not_block_native_turn", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-postpaid-shared", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "postpaid_", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    for name, package, pattern in [
        ("native-known-usage-recovery", "hagency-store", "recurring_"),
        ("native-never-started-store-recovery", "hagency-store", "shared_demand_v2_"),
        ("native-never-started-message-recovery", "hagency", "native_demand_never_started_attempt_retries_same_event_exactly_once"),
        ("native-demand-cold-account", "hagency", "cold_account_lookup_completes_the_first_demand"),
        ("native-demand-slow-binding", "hagency", "slow_demand_binding_renews_before_the_first_work_ack"),
        ("native-unanswered-exchange", "hagency", "again_when"),
        ("native-two-minute-lease", "hagency", "holds_a_two_minute_lease"),
        ("native-demand-late-stop-recovery", "hagency", "native_demand_late_stop_and_logout_keep_unknown_state_and_deny_stale_offer"),
    ]:
        run(name, ["cargo", "test", "--locked", "-p", package, "--features", "shared-budget-execution,resource-adoption", "--lib", pattern, "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-question-execution", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "native_shared_question_", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-room-history-input", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution", "--lib", "encrypted_shared_input_is_pinned", "--", "--ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-current-input-boundary", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "input_boundary_tests", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-execution-feature-contract", ["cargo", "test", "--locked", "-p", "hagency", "--no-default-features", "--features", "shared-execution,resource-adoption", "--lib", "every_execution_feature_enables_the_current_native_context_contract", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-minimal-context-refusal", ["cargo", "test", "--locked", "-p", "hagency", "--no-default-features", "--features", "shared-execution,resource-adoption", "--lib", "native_shared_execution_requires_v3_before_call_admission", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-thread-context", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution", "--lib", "shared_context_", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-thread-context-http", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "native_shared_context_", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-first-turn-permissions", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "native_shared_owner_policy_applies_before_the_first_model_call", "--", "--ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    if sys.platform == "darwin":
        run("native-macos-exit-signal", ["cargo", "test", "--locked", "-p", "hagency-platform", "--lib", "native_macos_signal_denial"], cwd=CLIENT, rust_tests=True)
    run("native-interrupted-calls", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "native_shared_interrupted_call", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-admitted-call-cleanup", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution", "--lib", "shared_successor_call_failed_start", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-codex-control", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--test", "session", "native_codex_control", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-encrypted-execution", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "native_shared_encrypted_", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-demand-promotion-recovery", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "native_demand_recovered_promotion", "--", "--include-ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    for name, pattern in [("native-budget", "agent_budget"), ("native-http-replay", "budget_http")]:
        run(name, ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", pattern, "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-account-inventory", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "account_inventory", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-owner-pool-limits", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "automatic_account_quota_does_not_couple_pools_or_erase_unknown_usage", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-capacity", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution", "--lib", "pool_agent_topup_requires_owner_keeps_unknown_usage_and_conserves_siblings", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-room-documents", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "documents::tests", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-pdf-parser", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--test", "documents", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-chat-file-protocol", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--test", "session", "native_room_file", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-chat-file-service", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "chat_files_", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-chat-file-execution", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "native_shared_chat_files_cross_", "--", "--ignored", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-claude-room-tools", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--test", "claude_session", "room_mcp", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-claude-usage", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--test", "claude_usage"], cwd=CLIENT, rust_tests=True)
    run("native-claude-provider-identity", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "claude_identity"], cwd=CLIENT, rust_tests=True)
    run("native-claude-account-binding", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "claude_native_provider_binding"], cwd=CLIENT, rust_tests=True)
    run("native-claude-process-identity", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--test", "claude_owned", "native_claude_account_observation"], cwd=CLIENT, rust_tests=True)
    run("native-claude-interrupt", ["cargo", "test", "--locked", "-p", "hagency-runtime", "--test", "claude_session", "--test", "claude_owned", "native_claude_interrupt", "--", "--test-threads=1"], cwd=CLIENT, rust_tests=True)
    run("native-postpaid-host", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution,resource-adoption", "--lib", "--", "--test-threads=1", "typed_http_postpaid_overrun", "typed_http_runtime_unreported_failure", "typed_http_runtime_budget_notice"], cwd=CLIENT, rust_tests=True)
    if not reached_start:
        raise SystemExit(f'Unknown start check: {args.start_at}')
    finish()
    pool.shutdown()
    name_second_runs()
    timed = [row for row in rows.values() if row.get("seconds") is not None]
    built = sum(row.get("build_seconds") or 0 for row in timed)
    print(f"{len(rows)} checks passed in {(time.monotonic() - started) / 60:.1f} min, {args.jobs} at a time. "
          f"Added up, they took {built / 60:.1f} min until Cargo had built what they run"
          f"{' (waiting for another command to build included)' if args.jobs > 1 else ''} "
          f"and {(sum(row['seconds'] for row in timed) - built) / 60:.1f} min to run it. The longest:", flush=True)
    for row in sorted(timed, key=lambda row: -row["seconds"])[:8]:
        print(f"  {row['seconds']:6.0f}s  {row['check']}  ({row.get('build_seconds') or 0:.0f}s of it until built)", flush=True)


if __name__ == "__main__":
    main()
