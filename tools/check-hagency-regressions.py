#!/usr/bin/env python3
"""Release checks for budget authority and chat regressions; no live account calls.

python3 tools/check-hagency-regressions.py --output .run/regressions
Add --client ../hagency-client to include native HTTP loss/replay tests.
Server PostgreSQL integration tests must separately use a disposable database.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--client", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.setdefault("RUST_MIN_STACK", "16777216")
    results = []

    def run(name, command, cwd=ROOT, rust_tests=False):
        log = output / (name + ".log")
        with log.open("w") as stream:
            result = subprocess.run(command, cwd=cwd, env=env, stdout=stream, stderr=subprocess.STDOUT)
        text = log.read_text()
        count = sum(int(n) for n in re.findall(r"test result: ok\. (\d+) passed;", text))
        passed = result.returncode == 0 and (not rust_tests or count > 0)
        results.append({"check": name, "passed": passed, "tests": count if rust_tests else None})
        (output / "report.json").write_text(json.dumps(results, indent=2) + "\n")
        print(name, "PASS" if passed else "FAIL", flush=True)
        if not passed:
            raise SystemExit(f"Inspect {log}; no release until this check passes")
        return text

    run("package", ["bash", "tools/package-system-apps/check.sh"])
    run("script-vm", ["cargo", "build", "--locked", "--manifest-path", "tools/hagency-script-review/Cargo.toml"])
    vm = ROOT / "tools/hagency-script-review/target/debug/hagency-script-review"
    for name in ["check_full_bundle", "check_agent_budget", "check_pool_application", "check_entry_workspaces", "check_account_identity", "check_accounts"]:
        run(name, ["python3", f"apps/hagency/tests/{name}.py", str(vm), "--output", str(output / name)])
    # Keep these together: a budget fix must not erase rendering, activity or answer actions.
    for name, pattern in [
        ("budget-host", "host::hagency::budget"),
        ("agent-chat", "agent_chat"),
        ("link-preview", "link_preview"),
        ("linkify", "linkify"),
        ("activity", "shared_permission_notice"),
        ("theme", "theme::"),
        ("theme-contrast", "semantic_token_pairs_meet_wcag_aa"),
        ("chat-information", "home::mobile_chat_info"),
        ("chat-information-layout", "desktop_information_columns_tests"),
    ]:
        run(name, ["cargo", "test", "--locked", "--lib", "--features", "onboarding-test-support", pattern], rust_tests=True)
    if sys.platform == "darwin":
        run("apple-chat-fonts", ["cargo", "test", "--locked", "--lib", "--features", "onboarding-test-support", "apple_fonts", "--", "--test-threads=1"], rust_tests=True)
        run("apple-system-fonts", ["cargo", "test", "--locked", "--manifest-path", "crates/article-makepad/Cargo.toml", "--lib", "apple_fonts", "--", "--test-threads=1"], rust_tests=True)
    run("answer-export", ["cargo", "test", "--locked", "--manifest-path", "crates/answer-export/Cargo.toml", "--lib"], rust_tests=True)
    if args.client:
        probe_build = run("native-context-probe", ["cargo", "build", "--locked", "-p", "hagency-runtime", "--features", "shared-context", "--bin", "hagency-managed-runtime-probe", "--message-format=json"], cwd=args.client.resolve())
        artifacts = [json.loads(line) for line in probe_build.splitlines() if line.startswith("{")]
        probes = [item["executable"] for item in artifacts if item.get("reason") == "compiler-artifact" and item.get("target", {}).get("name") == "hagency-managed-runtime-probe" and item.get("executable")]
        if len(probes) != 1:
            raise SystemExit("Expected one freshly built offline context probe")
        env["HAGENCY_TEST_RUNTIME_PROBE"] = probes[0]
        run("native-thread-context", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution", "--lib", "shared_context_", "--", "--include-ignored", "--test-threads=1"], cwd=args.client.resolve(), rust_tests=True)
        run("native-thread-context-http", ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", "native_shared_context_", "--", "--include-ignored", "--test-threads=1"], cwd=args.client.resolve(), rust_tests=True)
        for name, pattern in [("native-budget", "agent_budget"), ("native-http-replay", "budget_http")]:
            run(name, ["cargo", "test", "--locked", "-p", "hagency", "--features", "shared-budget-execution", "--lib", pattern, "--", "--test-threads=1"], cwd=args.client.resolve(), rust_tests=True)
        run("native-account-inventory", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution,resource-adoption", "--lib", "account_inventory", "--", "--test-threads=1"], cwd=args.client.resolve(), rust_tests=True)
        run("native-capacity", ["cargo", "test", "--locked", "-p", "hagency-store", "--features", "shared-budget-execution", "--lib", "pool_agent_topup_requires_owner_keeps_unknown_usage_and_conserves_siblings", "--", "--test-threads=1"], cwd=args.client.resolve(), rust_tests=True)


if __name__ == "__main__":
    main()
