#!/usr/bin/env bash
# The Desktop checks on Linux, one part per call:
#   bash tools/linux-checks.sh <part>
#
# Each part runs the commands of one macOS job of .github/workflows/main.yml,
# unchanged except where a line below says so. A part goes on after a command
# fails, so that one run lists everything that fails; its last lines are a
# table of every command and of the failed tests, and it ends with an error
# when anything failed.
#
# The library workspace is run in four groups of packages instead of once
# with --workspace, so that the groups run side by side. Cargo therefore
# picks each group's dependency features for that group alone. Inside a
# group, tools/parallel-tests.py runs several test processes at a time.
#
# Parts: lib-hagency, lib-store, lib-runtime, lib-rest, lib-features (the
# library workspace), app and gate (the app's own checks and the regression
# gate, which the workflow runs only when asked).
set -uo pipefail

part="${1:?usage: linux-checks.sh <part>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
out="${RUNNER_TEMP:-/tmp}/linux-checks"
mkdir -p "$out"
: > "$out/steps.tsv"

# step <name> <folder> <command...>
step() {
    local name="$1" dir="$2"
    shift 2
    echo "::group::$name"
    echo "+ (in ${dir:-.}) $*"
    local start=$SECONDS
    (cd "$root/$dir" && "$@") 2>&1 | tee "$out/$name.log"
    local code=${PIPESTATUS[0]}
    echo "::endgroup::"
    printf '%s\t%s\t%s\n' "$name" "$code" "$((SECONDS - start))" >> "$out/steps.tsv"
    echo "linux-check $name exit=$code seconds=$((SECONDS - start))"
}

client=crates/hagency-client
# The two selectors the macOS job skips: they need a live qualified host.
skip=(--skip native_codex_real_app_server --skip native_two_agent_qualification_records_its_evidence)

runtime_probe() {
    step runtime-probe "$client" cargo build --locked -p hagency-runtime \
        --features shared-context --bin hagency-managed-runtime-probe
    cp "$root/$client/target/debug/hagency-managed-runtime-probe" "$out/hagency-library-runtime-probe" \
        && export HAGENCY_TEST_RUNTIME_PROBE="$out/hagency-library-runtime-probe"
}

library() {
    export RUST_TEST_THREADS=1 RUST_MIN_STACK=16777216
}
# cargo test with the same arguments, several test processes at a time.
tests=(python3 "$root/tools/parallel-tests.py" --)

case "$part" in
lib-hagency)
    library
    runtime_probe
    step tests "$client" "${tests[@]}" --locked -p hagency --all-targets -- "${skip[@]}"
    ;;
lib-store)
    library
    runtime_probe
    step tests "$client" "${tests[@]}" --locked -p hagency-store --all-targets -- "${skip[@]}"
    ;;
lib-runtime)
    library
    step tests "$client" "${tests[@]}" --locked -p hagency-runtime -p hagency-agent-local \
        --all-targets -- "${skip[@]}"
    ;;
lib-rest)
    library
    step linux-qualification-helper "$client" python3 native/scripts/test-qualify-linux-cgroup.py
    step tests "$client" "${tests[@]}" --locked --workspace --exclude hagency --exclude hagency-store \
        --exclude hagency-runtime --exclude hagency-agent-local --all-targets -- "${skip[@]}"
    ;;
lib-features)
    library
    runtime_probe
    step feature-tests "$client" "${tests[@]}" --locked -p hagency \
        --features shared-budget-execution,resource-adoption --lib
    step empty-device "$client" cargo test --locked -p hagency-store \
        --features shared-budget-execution,resource-adoption --lib \
        domain::owner_resources::adoption::empty_device::tests:: -- --test-threads=1
    ;;
app)
    step miniapp-core "" cargo test --locked --manifest-path crates/miniapp-core/Cargo.toml
    step miniapp-catalog "" cargo test --locked --manifest-path crates/miniapp-catalog/Cargo.toml
    step system-apps "" cargo test --locked --manifest-path crates/system-apps/Cargo.toml
    step package-system-apps "" bash tools/package-system-apps/check.sh
    step package-octos "" python3 tools/test_package_octos.py
    step release-icons "" python3 tools/check-release-icons.py
    step app-lib-tests "" cargo test --locked --features agent_chat,octosense-module --lib --no-fail-fast
    step app-build "" cargo build --locked --features agent_chat
    step app-minimal-check "" cargo check --locked --no-default-features --features agent_chat
    step i18n "" python3 tools/wechat-ux/check_i18n.py
    step theme-literals "" python3 tools/check_theme_literals.py
    ;;
gate)
    # The gate stops at its first failed check. Here its source is run with
    # that one line taken out, so that the report names every check that
    # fails on Linux. Nothing is written into the tree.
    cat > "$out/gate-runner.py" <<'PY'
import pathlib, sys
source = pathlib.Path(sys.argv[1]).resolve()
text = source.read_text()
stop = 'raise SystemExit(f"Inspect {log}; no release until this check passes")'
assert text.count(stop) == 1, "the gate's stop line changed"
sys.argv = [str(source), "--output", ".run/linux-regressions"]
exec(compile(text.replace(stop, "pass"), str(source), "exec"), {"__name__": "__main__", "__file__": str(source)})
PY
    step regression-gate "" python3 "$out/gate-runner.py" tools/check-hagency-regressions.py
    ;;
*)
    echo "unknown part: $part" >&2
    exit 2
    ;;
esac

python3 - "$out" "$root" "$part" <<'PY'
import json, os, pathlib, re, sys
out, root, part = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
lines = [f"## Linux checks: {part}", "", "| command | exit | minutes | passed | failed | ignored |", "|---|---|---|---|---|---|"]
failed_tests, errors, bad = [], [], False
for row in (out / "steps.tsv").read_text().splitlines():
    name, code, seconds = row.split("\t")
    text = (out / f"{name}.log").read_text(errors="replace")
    sums = [0, 0, 0]
    for p, f, i in re.findall(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", text):
        sums = [sums[0] + int(p), sums[1] + int(f), sums[2] + int(i)]
    lines.append(f"| {name} | {code} | {int(seconds) / 60:.1f} | {sums[0]} | {sums[1]} | {sums[2]} |")
    bad = bad or code != "0"
    target = ""
    for line in text.splitlines():
        found = re.search(r"Running (?:unittests )?(\S+) \(target/\S*/deps/([A-Za-z0-9_]+)-[0-9a-f]+\)", line)
        if found:
            target = f"{found.group(2)} {found.group(1)}"
        found = re.match(r"test (\S+) \.\.\. FAILED", line)
        if found:
            failed_tests.append(f"{name}: [{target}] {found.group(1)}")
        if re.match(r"error(\[E\d+\])?:", line) and len(errors) < 40:
            errors.append(f"{name}: {line[:200]}")
report = root / ".run/linux-regressions/report.json"
if part == "gate" and report.is_file():
    rows = json.loads(report.read_text())
    lines += ["", f"Regression gate: {sum(r['passed'] for r in rows)} of {len(rows)} checks passed."]
    lines += [f"- failed check: {r['check']}" for r in rows if not r["passed"]]
    bad = bad or any(not r["passed"] for r in rows)
if failed_tests:
    lines += ["", f"Failed tests ({len(failed_tests)}):"] + [f"- {t}" for t in failed_tests]
if errors:
    lines += ["", "Error lines:"] + [f"- {e}" for e in errors]
text = "\n".join(lines) + "\n"
print(text)
summary = os.environ.get("GITHUB_STEP_SUMMARY")
if summary:
    with open(summary, "a") as stream:
        stream.write(text)
sys.exit(1 if bad else 0)
PY
