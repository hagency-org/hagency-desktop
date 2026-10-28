# Original-resource onboarding validation

The Resources → Onboarding action opens a native confirmation panel. Script input is `{}`: paths, provider identities, migration IDs, ceiling values and confirmation booleans are rejected. The native panel displays the current SDK account, homeserver and original resources; an absent account-wide monthly ceiling must be entered explicitly there. Its final confirmation creates one retained operation. Closing a UI waiter does not cancel or duplicate the original migration.

This checkpoint is opt-in. `resource-adoption` enables the production native adapter and review panel. `onboarding-test-support` additionally enables the explicit installation-root bootstrap below. Neither feature is enabled by default. The fixture feature is for an isolated test binary, not a normal distributed release.

## Build and start the isolated fixture

From the Desktop checkout:

```sh
CARGO_BUILD_JOBS=2 cargo build -p rinx --release --features onboarding-test-support
install -d -m 700 "$PWD/.run/onboarding-installation"
install -d -m 700 "$PWD/.run/onboarding-installation/profile"
HAGENCY_DESKTOP_DATA_DIR="$PWD/.run/onboarding-installation/profile" \
  ./target/release/rinx \
  --hagency-isolated-installation-root "$PWD/.run/onboarding-installation" \
  --hagency-seed-empty-personal-fixture
```

Use an isolated local test Matrix server/account. The app's ordinary profile selector is separate from the installation authority root: every fixture process must pass the **same absolute installation-root argument**, even when it uses a different profile directory. The CLI initializes that root before the app starts; changing it or selecting it after any canonical/legacy authority opens is rejected. Production builds have no environment-variable authority override. The selected root contains both `authority/` and `canonical/`.

For Makepad instrumentation, add the existing `MAKEPAD_REMOTE` environment variable with an unused port outside the running 8317–8320 fixture set. Do not reuse or restart those running profiles for this test. Preserve logs privately; do not print SDK tokens, provider account identity, email or credential references.

The optional empty-personal-fixture flag is a narrowly synthetic bootstrap. After real SDK authorization it may create an absent, empty scoped personal ledger **only when the NativeOwner state is physically inside this explicitly selected test installation root**. Existing personal DBs are returned untouched; unknown or populated histories still block adoption. This flag does not declare a real user's missing history empty.

## Obtain original evidence

Either enroll the original resources using the real Resources flow in this new fixture, or stop/drain a separately authorized test owner and copy its complete test profile into `profile/` while its SQLite files are closed. Keep every original resource ID, Seat declaration, completed admission intent/reply and private provider selection. Do not manufacture replacement canonical resources, rewrite pending outcomes, or remove a real history to make the test pass. A copy of a currently running SQLite/WAL profile is not an accepted snapshot.

The first supported inventory is the whole selected source profile on one proven provider account/Seat. Two original resources on that Seat are supported, but their ceilings are not added together. Shared signed-in Codex identity must be confirmed by the existing metadata-only native provider adapter; unknown provider identity remains unsupported. No model turn or provider login is performed by migration.

## Validate the visible flow

1. Sign in through the normal SDK flow, open Hagency → Resources, and verify both original resources and their original connection/admission details.
2. Open Onboarding. Check native account/server/resource identity. Enter the explicit shared monthly ceiling if the original Seat had none. Confirm in the native panel.
3. The original source is drained and backed up, its legacy writer is fenced, original definitions and completed private admission custody are adopted, and the active canonical Resources list is read before success is returned. Both resource IDs must remain unchanged and share one account budget.
4. Restart the same binary with the same profile and installation-root argument. The create-once descriptor selects the original plan; current SDK authorization supplies authority. An incomplete plan opens recovery-only, retaining its original migration ID and ceiling. An active plan reopens canonical Resources.
5. For an admitted owner contribution, open the separate native grant review. Confirm the displayed resource, coordinator, eligible managers, period and budget. Publication retains its own original native and central command IDs. A lost publication reply is recovered before any retry.
6. Reopen with a replacement stable central device only in a separate authorized fixture scenario: canonical resource inventory remains readable, while original private grant review is unavailable. Missing descriptors are not reconstructed from the replacement device.

Closing the panel, package retirement or logout is not permission to infer rollback. Retained task results and the durable descriptor determine whether to recover. Unknown histories, changed source revisions, busy physical source locks, an expired confirmation or unqualified platform remain explicit blockers. Observing selected roots does not prove that undiscovered external roots are complete.

## Source-backed regression entry points

Client:

```sh
CARGO_BUILD_JOBS=2 cargo test -p hagency --lib \
  --features resource-adoption,managed-runtime native_confirmation_ -- --nocapture
CARGO_BUILD_JOBS=2 cargo test -p hagency --lib \
  --features resource-adoption,managed-runtime resource_activation::tests -- --nocapture
```

These use real HTTP authentication, SQLite source/canonical ledgers, source locks/backups/fences and separate processes sharing one explicit test root. The provider identity proof in these deterministic tests is synthetic; the visible Desktop fixture must separately exercise the real metadata adapter. They cover two original resources, prepared-plan restart, post-descriptor authority loss, original admission/grant/publication recovery and replacement-device denial.

Desktop:

```sh
CARGO_BUILD_JOBS=2 cargo test -p rinx --lib \
  --features onboarding-test-support hagency::backend:: -- --nocapture
CARGO_BUILD_JOBS=2 cargo test -p rinx --lib \
  --features onboarding-test-support adoption_ui -- --nocapture
```

The backend regressions exercise retained waiter custody and real native revocation barriers, including logout during drain. Native panel tests draw the actual Makepad widgets at phone and desktop dimensions in English and Chinese. Passing these checks does not replace the actual instrumented visible-flow acceptance.
