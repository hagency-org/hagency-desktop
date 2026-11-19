#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../crates/hagency-client"
filter='domain::owner_resources::adoption::current::tests::'
tests=(cargo test --locked -p hagency-store --features shared-budget-execution,resource-adoption --lib "$filter")
test_list=$("${tests[@]}" -- --list)
echo "$test_list"
if ! grep -Eq "^${filter}.+: test$" <<< "$test_list"; then
    echo '::error::The current-profile test selector matched no tests.' >&2
    exit 1
fi
"${tests[@]}" -- --test-threads=1
