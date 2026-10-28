# First resource on an empty device

Status: checkpoint44 is implemented, qualified and promoted. All four original
local role sessions now run checkpoint45, retaining this first-device flow. Device setup retains original SDK/device
bindings and adds the empty-device path without provider credentials or quota.
Read-only regression of the original settlement passes with its calls, usage,
allocation and receipt unchanged. Activation evidence is recorded under
`.run/management44-activate-{owner,manager,coordinator,admin}` and
`.run/management44-stage/acceptance-observe/report.json`. The subsequent45
activation and unchanged original-settlement receipts are in the corresponding
`.run/management45-activate-*` and `.run/management45-stage/acceptance-observe`.

The first-use flow passed isolated EN/light and CN/dark native-window checks,
including cancellation, setup, offline-provider enrollment and restart. These
are real Metal windows. Real OAuth persistence, whole-app UX and other-platform
acceptance remain open.

## Problem reproduced

A new Matrix profile can open Resources and prepare a provider account, but
canonical account enrollment requires device setup. The current setup route is
an existing-resource migration: it requires at least one original resource and
a verified original provider account. It also assumes a personal database exists.
Consequently an entirely new device cannot complete the canonical first-resource
flow. The native offline fixture had to create a legacy resource through the
older wizard before migration; that workaround is not the intended first-use UX.

Relevant boundaries are `NativeOwner::activation_snapshot_on_platform`,
`confirm_resource_activation`, `AdoptionInventory::collect_inner`,
`Session::prepare_account`, `NativeManagement::account_enrollment_ready` and
`namespace_enrollment_registry`. Preparing an account in the legacy store first
can introduce more state that the current migration does not transfer. That
namespace and its credentials must never be silently replaced.

## Intended mini-app flow

1. The signed-in Matrix user opens **Connect resources**. Native setup status
   identifies an already active device, an empty original device, an original
   migration, a saved setup in progress, or retained history requiring transfer.
   It does not infer empty state from an empty filtered resource list.
2. On an empty device, **Set up this device** opens a native review showing the
   authenticated Matrix identity and what will be initialized locally. This step
   has no model, account quota or contribution fields.
3. Confirming initializes native resource management. The result survives window
   closure and restart under the same original setup request. Cancellation before
   confirmation changes nothing. A lost result offers recovery of that request.
4. After native initialization, the mini app creates an account space, opens the
   provider authorization page, checks available models and enrolls the selected
   resource. The existing account-enrollment review supplies its shared account
   limit. Initialization itself does not supply a default account or capacity.
5. Resource association, server admission, coordinator allocation and project
   management continue through the existing authenticated mini-app flow.

The setup status must prevent starting legacy account preparation during this
first-use path. Existing account spaces remain visible and retained; if they
require transfer, the UI must explain that state and preserve their IDs instead
of presenting a second empty account as recovery. English/Chinese copy and Rinx
theme components remain the design baseline.

## Native and storage implementation

The first implemented prerequisite is `EmptyDeviceInventory::inspect`. It checks
the exact known source schema, profile identity and all tables on existing
read-only connections. A zero-resource source with accounts, allocations,
execution history, tasks or unknown schema is not an empty installation.
Validated terminal management custody is retained and included in its digest;
pending/uncertain commands block this path. An unused original seat without a
declaration is recorded as original metadata, never converted into account quota.
An optional existing personal database must match the original owner/profile and
contain no agent or usage history. Its identity and inbox settings are included
in the review digest. A missing personal database is represented as absent.

This inventory is descriptive, not authority. It accepts no JSON proof and does
not initialize databases, create a missing personal file, invoke a provider or
write a migration marker. Native filesystem custody must establish actual
absence, retain directory/file identities and recapture the inventory before any
write. The native implementation now retains the original file/directory handles,
physical identities, original domain lock and installation lease. A distinct
version-two `empty_device` binding records either the existing personal file or
the exact observed absence. The existing version-one populated binding is
unchanged. Creating a personal path after capture invalidates that observation.

`SourceQuiescence::review_empty_device` reads before the original owner is retired.
It checks the Matrix SDK session, native gate and host identity on both sides of
the bounded read. Cancellation leaves the original worker usable. After drain,
`validate_empty_device_review` requires the same native scope and rechecks both
physical binding and inventory. Even still-empty storage with changed settings
requires a new review. This native API is not yet connected to the mini-app.

The native fence now follows the existing source-drain and installation
writer-exclusion rules. It syncs byte-exact original main/WAL backups, publishes
the durable legacy refusal and then fences only databases actually present.
The version-two backup manifest records personal-source presence and the exact
plan/source digests. A missing personal source stays absent. Existing backup
bytes are checked on retry; invalid backups block writes and are never rebuilt
from already-fenced databases. This must also precede canonical activation so a
later personal opener cannot create a parallel legacy source at an absent path.

Persist an original native preparation before its first canonical write. The
store now uses a separate version-four `empty_device` plan in the existing
immutable, unique-per-profile adoption journal. It records the original
initialization ID, registry, physical source binding and inventory digest.
There is no provider, account or quota field. Existing v1/v2/v3 plan shapes and
digests are unchanged, and those APIs cannot interpret this plan as a provider
resource migration. A populated migration and an empty initialization cannot
reserve the same profile. No database schema migration is needed.

The store preparation API builds and validates without writes, commits only its
prepared record, and recovers that record using current native authority and the
original saved digest. A separate native-only transfer inspects original or
partly fenced sources, preserves the original inventory and writes archive
markers under retained physical custody. Only verified read-only archive
observations can advance the canonical record to `sources_fenced`. That phase
does not mean management is active; the later phases still reject access. No
resource, account, seat, quota or grant is created by preparation or fencing.

The native version-three descriptor shares the atomic create-once initializer
slot with the existing v1/v2 descriptors, whose serialization remains unchanged.
It is persisted before the canonical prepared record and binds the original
initialization, Matrix identity, device, registry, physical source and plan.
An interrupted native preparation resumes through the same original descriptor,
whether the canonical commit happened or not. Source-only startup reauthenticates
the SDK and validates the original unfenced or partly fenced state before any
legacy worker is opened. It does not create a replacement registry or bypass
changed source history. The desktop's sealed device-setup review now selects this
initializer explicitly; a script receives descriptive status, not its authority.

After restart, native initialization loads only the original recorded selection,
reauthenticates the current SDK profile/device, validates source custody and
canonical phase, and resumes the same operation. A descriptor is never sufficient
authority to create a replacement registry, manufacture absence, or adopt a
different source. Expired native consent does not authorize a new initialization.

## Canonical activation and active native restart

The version-four plan now proceeds through `prepared`, `sources_fenced`,
`registry_ready`, `custody_ready`, and `active`. Registry preparation checks that
this profile has no conflicting account, resource or task history. It creates no
provider identity, seat, resource, quota or capacity grant. A proof from the
original live private journal worker verifies retained terminal command custody;
caller JSON or an integer phase cannot substitute for that proof. Source and
custody evidence and the final descriptive result are checked against the exact
saved plan during recovery.

Active management uses the same canonical resource/account APIs and private
journal as populated migration. A distinct immutable task-history receipt binds
the original empty-device plan, allowing canonical task reads and writes after
activation. The original populated v1/v2/v3 task contracts remain distinct.
After activation, new pending commands legitimately change the private journal.
Restart verifies its permanent archive binding and active canonical plan rather
than comparing the mutable journal to its original empty snapshot again.

Native initialization shares the existing archive-worker handoff and retained
drain task with populated migration. The final selection is returned only after
that worker closes and the original source lock is reacquired. Both absent and
present empty personal sources are supported. Focused tests exercise native
owner shutdown/reopen, unchanged backups/descriptor, first account preparation,
first task creation, task reply recovery, and a pending request surviving another
restart. Account preparation intentionally creates an undeclared identity seat;
initialization alone creates none. Neither step grants account quota.

Those native API tests use synthetic Matrix authorization and isolated files.
The application checks described below additionally cover actual desktop windows
and process restarts. Neither suite substitutes for real provider authorization.

## Application wiring and validation

`DeviceSetupKind` distinguishes existing resources from an empty device. The
sealed native `DeviceSetupReview` retains the original observation until human
confirmation, rejects a quota for empty setup, and checks source identity and
inventory again before shutdown. `device_setup_phase` supports both original
populated migration and the version-four initializer during desktop reopen.
The original populated summary and quota contract remain unchanged.

The host reports `setupKind` with readiness and rejects account preparation
before setup is active. The mini app checks readiness before showing account
creation, routes a new resource through provider accounts, and retains existing
wizard/command recovery. Empty setup uses the existing native confirmation
window without a token input. EN/CN status and a completed heading distinguish
initialization from resource enrollment and shared capacity.

Validation includes 11 native activation tests; 49 desktop host tests and 12
adoption/window tests (overlapping sets); 84 account and 32 onboarding VM cases;
and full-bundle construction in both languages. Actual Metal windows verified
EN/CN cancellation, setup without account/resource/quota creation, one account
and one resource enrolled through offline provider metadata, and application
restart retaining their identities. Local `owner_resource_grants` are owner
access bindings; shared `owner_capacity_grants` remain empty.

The final CN/dark fixture uses no personal-database seeding flag or preliminary
legacy resource. Normal application startup creates its empty personal database;
the recorded `personalDatabasePresent` is true. The absent-personal-source case
is covered by the native API/process suite, not claimed as a desktop observation.
The stopped checkpoint43 populated-migration fixture also reopens with unchanged
canonical records and provider-call history. The final cosmetic change affects
only the native heading and EN/CN catalog entries; native and mini-app flow code
matches the full first-use test build.

Evidence: `empty-setup-build/qualification.json`,
`empty-setup-build/actual-ui-restart/report.json`,
`empty-heading-build/actual-ui/report.json`, and
`empty-heading-build/prior-migration/report.json` in the stage. These are hidden
real Metal windows, not GPU-less headless rendering. Real OAuth persistence,
original-role activation, other platforms, and whole-app UX acceptance are not
covered by these results.

## Isolated validation so far

- Store preparation and optional-source fencing pass 10 focused tests and 94
  adoption regressions; the original populated migration remains covered.
- Native descriptor/preparation validation passes 22 filtered cases (one separate
  installed-Codex keyring test remains ignored), then 27 source regressions.
- Native backup, permanent refusal, fencing and source-only restart pass 18
  focused cases and 31 source regressions, plus both strict configurations.
- The final raw-backup assertion avoids opening immutable backups with SQLite;
  it compares the original main/WAL bytes directly. The store's new exact lookup
  and phase reporting pass 10 focused tests and strict Clippy afterward.
- These suites overlap. They are native API/instance-restart tests against
  isolated files and a synthetic Matrix authorization server, not actual
  mini-app or real-provider E2E acceptance.

The active-handoff qualification in `empty-active-handoff-v2/qualification.json`
passes 15 focused storage cases, 99 full adoption cases, 13 private-journal cases,
33 native source-quiescence cases and both strict feature configurations for
`hagency-store` and `hagency`. The focused storage cases overlap the 99-case
suite. Source hashes remained unchanged throughout the combined qualification.
The new active native tests cover both personal-source variants and interruption
after registry/custody commits. Desktop UI testing has not yet run for this path.

Retained development failures: the first unrestricted parallel store run hit
bounded two-second inspection deadlines; serial qualification passes without
changing production deadlines. The first native test called a private helper
and was corrected to use public recovery. A later test incorrectly expected no
seat after explicitly preparing an account; it now checks zero seats before
account preparation and exactly one undeclared seat afterward. Strict Clippy
also identified a large task-binding enum; immutable plans now use shared Arc
references, and both strict configurations pass. The native debug test link
continues to emit the existing large unwind-section warning.

Evidence lives in `empty-native-preparation-v2`, `empty-source-fence`,
`empty-native-fencing`, `empty-fencing-final` and `empty-permanent-refusal` under
the stage. The first native preparation compile failure was a test helper import
at the wrong module depth; the failed report is retained. Running checkpoint43,
its four original Matrix sessions and server32 remain unchanged.

## Required acceptance before activation

- Truly empty original source, both absent and present empty personal storage;
  no provider RPC, synthetic resource, default quota or allocation during setup.
- Prepared account, resource, existing quota, personal history, unknown/missing
  schema, wrong profile/device, pending command and replaced-path refusals.
- Cancel, stale review and concurrent legacy writer; original data retained.
- Failure before preparation persistence, after persistence, after source fence,
  after canonical commit and after a lost private reply; exact restart recovery.
- Existing populated v1/v2/v3 migration regression and immutable original receipts.
- Actual native EN/light and CN/dark setup → account → model → enrollment flow
  from an empty isolated installation, without the test-only personal seeding
  option or a preliminary legacy resource. Verify normal restart and shared
  account identity/limits.
- Real provider authorization and enrollment remain a distinct user-assisted
  check; offline metadata tests must not be reported as real OAuth acceptance.

This work does not close the remaining full migration gaps: original account
import/drain/logout, resource duplication and central amendments, agent extras,
tool approvals, graph/board and project lifecycle parity, notifications, or
whole-app/platform acceptance.
