# Hagency mini-app migration progress

## Web-client implementation checkpoint — 2026-10-20

The user clarified that this task ports the existing Hagency web client into
Hagency Desktop. The [current feature matrix](hagency-web-client-parity.md)
supersedes the historical remaining-work lists below. Catalogue/standing offers,
declared daily rates, profile-scoped configuration retirement and assignment,
future-agent defaults, remembered trust, connection maintenance, and consolidated
workforce/account/capability/usage/alert views now have reachable implementations.
The final macOS release and actual Makepad-window checks passed. The implemented
controls do not claim that every legacy backend integration is active.

Focused checks pass: 9 host routes, 14 native adapters, 19 store web cases,
96 EN/CN mini-app callbacks, 34 connection callbacks and 4 private exports.
Publication recovery/legacy checks also pass; the final same-account sibling
regression plus catalogue rerun pass 3/10. The final release is
`274c6c1e3b8034ec9b8dbc16356fa47ea32d5c8525cac5fae6084222173f785e`.
Actual native UI verifies console data, connection probe/create/edit, daily
rate save/cancel, trust add/remove, publication/withdrawal and unused resource
retirement. Existing capacity grants and managed-agent rows remain unchanged.
Exact reports and explicit limitations are in the current feature matrix.

Imported Fleet transport stays inactive, side allowances remain legacy-only,
daily rates remain declared terms, and remembered trust cannot bypass project
administrators. New reservations enforce saved offer withdrawal, count and
per-engagement limits. An active allocation pins its resource definition;
unused sibling configurations on the same account remain editable.

Coding-agent engines, graph delivery, full native-CLI parity and general legacy
migration are outside this frontend task. Earlier E2E evidence below remains
valid only within its stated scope and is not complete web-port acceptance.

## Current delivery checkpoint — 2026-10-20

The native onboarding and real shared-text reply flow now passes end to end on the isolated localhost Matrix server. Existing provider credentials and the original Matrix session/device were preserved. This is not a mini1 deployment or full management-feature acceptance.

The release `59ff68fb834979b1ff48ecf5d9de97930245a2d13ad6ea21967aa354d9a8d0b9` runs in the original owner profile. It fixes private provider namespaces skipping restrictive feature discovery, which previously caused the final execution-policy check to reject startup. Discovery now runs for private namespaces too; the final independent identity/policy check remains enforced. Ordinary desktop builds include `shared-budget-execution` by default.

Using four distinct Matrix roles, actual native UI validation completed resource enrollment, server admission, connection verification, a 16,000-token owner grant, a 12,000-token project request/approval/allocation, Matrix space/room creation and membership verification, and an 8,000-token agent request approved by the assigned project administrator. Two real Codex replies were then delivered to the project room. Both Matrix reply event IDs were checked and both native calls settled; native and server usage match at **5,644 of 8,000 tokens**, leaving 2,356. The second turn needed no restart or run-control change. Actual Metal captures show both replies.

The earlier failed event remains in history as `head_changed`. Read-only evidence proved it had no work, calls, event pins, bindings, replies, or consumed tokens, and that its failed processes stopped. Existing Pause/Enable controls established a fresh control revision before one explicit new request; the old request was not rewritten or silently replayed. Reopening that exact terminal event remains unsupported, and a retained terminal demand can block subsequent messages in its old control revision.

Agent-template create/edit/disable/delete passed actual native UI checks with original command IDs and canonical receipts. The latest release also fixes the internal namespace leaking into the template resource label; its actual window now shows the readable model. Focused template Rust tests passed 16 cases, template callback tests 98, graph regressions 136, and full-bundle checks 2. This does not establish populated legacy-template import.

Personal model/workspace/rules configuration is implemented with exact saved-command recovery, model-profile revision checks, and private path/rule custody in native code. Focused native/store checks passed 33 cases, desktop host/layout checks 3, and new configuration callbacks 74. The actual Chinese/dark native rules and workspace sheets opened and cancelled without configuration writes; the original personal database was byte-identical afterward. The model selector correctly displayed its unauthenticated empty state. A ready-model apply flow was not exercised in that dedicated fixture. Minor English placeholder/newline issues remain recorded; no whole-app 9/10 UX acceptance is claimed.

The validated source is now in the real `hagency-client`, `hagency-desktop`, and `hagency-server` Git working trees. Guarded promotions copied 202 client files, 45 desktop files, and 77 server files, retaining backups and excluding unrelated changes. Source equivalence and `git diff --check` pass; client/desktop offline locked metadata checks pass. No commit, PR, or merge was performed at this checkpoint.

Build work now reuses one source tree and cached targets with eight jobs. The latest incremental desktop release took **3m22s**, compared with the previous **10m43s** build. These are observed build times, not an application-performance benchmark.

Current acceptance and exact evidence hashes: `.run/management46-stage/core-flow-live/acceptance.json`. Supporting state includes `flow.json`, `runtime-fix-flow.json`, `runtime-fix-second-turn.json`, `zero-work-proof.json`, `runtime-owner-activation/report.json`, and `template-flow.json`. Personal window evidence is in `.run/management46-stage/operator-graph-bindings-v1/personal-native-sheets-actual-v1/completion.json`. Promotion manifests are `.run/management46-final-promotion-20261019T192420Z/manifest.json` and the server's `.run/server32-promotion-20261019T192607Z/manifest.json`.

**The historical full-backend migration is broader than this task.** Shared coding/workspace execution, graph delivery and blanket legacy-history migration are not unfinished web-client porting. Current frontend behavior, explicit compatibility limits and remaining cross-platform acceptance are recorded in the web-client matrix above. The historical sections below preserve earlier evidence and unresolved broader work; their release numbers and missing-feature lists do not supersede the current implementation checkpoint.

## Candidate46: tool-approval management under validation

The staged native API lists resource-scoped tool-request status, projects
readable saved-rule descriptions with their original owner/project, and revokes
an exact reviewed grant. Revocation checks current authority and retains the
original command/revision for recovery. Long Unicode descriptions are preserved;
list pagination bounds encoded bytes, and command receipts omit private rule text.

Qualification `tool-approval-native-v2` passes 52 full approval tests (including
seven new focused cases), two adopted-resource authority cases, two NativeOwner /
loopback restart cases and both strict configurations. Desktop v1 passes 59 host
tests, four grant UI tests and release compilation. Its native source matches the
separately qualified native input. Earlier fixture and VM harness failures remain
recorded in their original evidence directories.

Actual hidden macOS Metal windows now pass English/light and Chinese/dark
18-request pagination, original rule details, review/cancel, two distinct exact
revocations, refresh and normal restart. The other agent's saved permission,
original rows and original four Desktop sessions/server remain unchanged.
`tool-approval-actual-v1/report.json` records the flow; eight captures were viewed
and reviewed in its `visual-review.json`. This is synthetic history on a loopback
Matrix account with an offline provider, not a real provider/private-decision E2E.
The fixture has been inserted once; never reseed or replay its two revocations.

Inspection reproduced expired pending requests being labeled as awaiting approval.
The candidate now compares their deadline with the host observation time, shows
“Approval window ended” / “审批已截止”, preserves recorded decisions and states
that statuses reflect the last refresh. Ten focused deadline checks and all 84
EN/CN tool UI cases pass. The UI-only replacement release
`tool-approval-desktop-v2` and its read-only actual-window continuation both pass.
All eight EN/CN captures were inspected; `tool-approval-actual-v2/visual-review.json`
records the corrected labels and preserved effects. No fixture was seeded again
and no additional permission was revoked.

The next candidate now includes readable agent/project context, fresh request
details and host-resolved navigation into the current account's native approval
inbox. Its 132 EN/CN VM cases and both complete-bundle roots pass. Native
qualification passes 53 full approval cases (including eight focused cases), two
adopted-resource cases, two NativeOwner/restart cases and both strict feature
configurations. Paired Desktop qualification passes 61 host tests and four grant
UI tests and release compilation. Actual hidden macOS Metal windows pass
English/light and Chinese/dark request context, fresh detail, saved-rule navigation
and project-filtered native approval-inbox opening. All eight captures were
inspected. Original records remain unchanged; the fixture was not seeded again.
The synthetic project has no joined approval room, so this verifies the empty
inbox destination, not exact-request selection or a private approval decision.
Evidence is under `tool-request-context-*`, including the actual probe report
and its separate visual review.

Whole-app 9/10 UX is not accepted. Original populated
approval-history transfer, next-dispatch execution-policy behavior and the full
private approval workflow remain open. The runtime-policy audit now also records
the original read-only dispatch guard and why setting the retained native YOLO
boolean alone cannot implement it. No candidate is promoted to the original45/32
installations. Evidence above is under `.run/management46-stage/`.

## Independent execution-policy runtime candidate

`execution-policy-runtime-probe-v1` implements the original Codex policy modes
in runtime settings and validates their returned values. Read-only always takes
precedence over an owner-selected unrestricted mode. A consumed warm process can
bind the dispatch settings only before starting/resuming a thread, with the same
cwd/model/effort; active-thread changes are refused. Three new focused cases,
all 57 session cases and default all-target runtime Clippy pass. The new cases
are included in that total. This is an isolated copy, not the running or paired
Desktop source.

The separate `execution-policy-dispatch-probe-v1` now captures the selected
policy in the same transaction as task start. A pre-start settings change refuses
the stale preparation; later changes preserve the running task's captured policy.
Four focused cases are included in ten owned-dispatch cases. Its final v3 run
passes 38 selected store cases (14 domain, ten owned dispatch, eight engagement
retention and six execution retention), plus default/enabled all-target Clippy.
The new snapshot is included in the existing ended-engagement cleanup; live
custody still prevents removal. An independent original-schema94 executable
started an attempt that the new schema95 code then migrated and checked: the
original fingerprint survived owner edits and offline reopen, while normal
runtime restart still fenced it. See `execution-policy-legacy-v1/attempt-v2`.

The separate `execution-policy-host-probe-v1/source` now binds the captured
policy to cold and consumed-warm processes. Its focused process checks pass,
including managed-account isolation and refusal of unexpected approval requests
in unrestricted mode. An earlier full run returned `CleanupUnknown` after a
completed protocol; five focused repetitions and a later complete 54-case run
passed, but the intermittent cleanup failure is not resolved.

This candidate also implements resource-scoped agent policy listing, current
policy reads and revision-checked edits with the original command retained for
recovery. The first four store cases and two actual NativeOwner/restart cases
pass. The staged Desktop now includes English/Chinese list, choice, review,
save and recovery pages using the existing theme: 72 policy VM cases, 132 existing
tool-management cases and both full bundle roots pass. These VM checks use inert
widgets and do not claim native pixels or provider execution.

Broader integration found schema95 compatibility omissions in source adoption,
empty-device setup, private archive journals and read-only ledger inspection.
The candidate updates those explicit version ranges and requires the new table's
shape; rewind fixtures remove it when reconstructing an older schema. Native
qualification `management-api-v5` now passes all 268 cases (58 approvals,
112 adoption, 32 reconciliation, two NativeOwner/restart, 54 owned-process and
ten warm-runtime cases), plus strict default and enabled-feature Clippy. The
paired Desktop `desktop-v2` passes 63 host tests, four grant UI tests and release
compilation (SHA `4ae3675872c2f6580bf1f23b756aafd36361e7d4069f08bd4c09c772e44eedf6`).
Actual hidden macOS Metal windows now pass list/choice/review/cancel, two distinct
policy saves, refresh and two normal restarts in EN/light and CN/dark. All eight
captures were viewed; `actual-v1/visual-review.json` records scoped visual checks.
Exactly two policy receipts were created; protected rows, the original four
Desktop45 sessions and server32 remain unchanged. This offline synthetic-agent
flow is not real-provider or whole-app9/10 acceptance.

A separate `execution-policy-defaults-v1` candidate now preserves the original
resource serialization/revision when no default exists, accepts a typed optional
Codex default, and inherits it only on the first successful new-agent provision.
Existing explicit agent choices and previously completed effects remain unchanged.
All64 selected native cases (four new core/store, two resource-configuration
and58 approvals) plus strict Clippy pass. Its separate mini-app
copy adds direct same-agent saved-permission navigation and passes 80 EN/CN VM
checks. These newer changes are not part of the Desktop build above. The resource
default editor, original-history adoption and native visual acceptance remain open.

The resource-default editor candidate now passes 121 selected native cases
(114 adoption, five resource-detail, and actual NativeOwner create/edit), plus
strict default and enabled-feature checks. Its 188 EN/CN VM checks and both full
bundle roots pass. Paired desktop63 host and four grant-UI checks plus release compilation now pass.
Actual EN/CN default editing, creation, cancel and normal restart pass in
`actual-v6/report.json`; exactly two edits and one creation were consumed.
Native qualification is `execution-policy-default-editor-v1/native-v4`;
the earlier missing test features, test initializers and supervisor-probe setup
failures remain recorded. No real provider was used.

Integration of read-only ordinary host dispatch, populated original
policy/history transfer and actual provider acceptance remain open. Shared
managed text-only execution keeps its separate admission contract. None of
these policy changes is promoted to the original installations. The source
integration map is `.run/management46-stage/tool-policy-dispatch-integration-map.json`.

An independent `execution-readonly-v1` candidate now removes the ordinary Host's
blanket refusal of nonexclusive workspaces and derives read-only runtime settings
from the immutable dispatch lease. Saved unrestricted policy cannot override it.
A real-pipe regression first reproduced Admission/NotStarted, then passed all four
managed/unmanaged and standard/unrestricted combinations. The complete55 owned
execution cases and strict all-target Clippy pass. Original policy snapshots,
lease cleanup and selected-account isolation are checked. These are offline
protocol tests, not actual-provider sandbox acceptance. This candidate is separate
from the qualified resource-default Desktop; no source is promoted.
The prior intermittent process-cleanup failure is still unresolved.

## Candidate46: approval-room management

The isolated candidate exposes resource-scoped approval-room inspection and
reviewed removal. A removal revokes the binding's saved tool grants in the same
transaction and retains the original command result for recovery. A durable
incarnation counter prevents removing and observing a room again from reviving
old approvals. Current SDK, package and resource authority remain required.

Actual hidden macOS Metal windows passed English/light and Chinese/dark list,
review, cancel, removal and normal restart. Eight captures were inspected. Two
submitted removals produced exactly two receipts and revoked the two fixture
grants; protected resource/accounting rows stayed unchanged after explicit fixture
setup. These are synthetic room records, without provider inference or real
Matrix room creation. Evidence: `.run/management46-stage/approval-binding-scoped-acceptance.json`.

Agent and project filters are now implemented in staged source. Project filtering
uses the project ID or project Matrix room, distinct from the private approval
room, and applies before pagination. The native approval family passes45 tests
and both strict configurations; the mini-app passes72 VM checks and two full
bundle roots. These scopes overlap other recorded checks. The desktop filter
build passed57 host tests, four grant UI tests and release compilation. Actual
EN/light and CN/dark filtering, 16+4 pagination, project-filtered16+3 pagination,
scroll reset, review/cancel and normal restart pass against20 synthetic links;
all protected records and the original two removal receipts remain unchanged.
The two earlier viewport-assumption failures are retained. Seven captures were
inspected, revealing narrow stacked labels. Their two width corrections pass
the same72 VM checks and two bundle roots. The layout-only release and actual
EN/CN native-window recheck now pass, with all eight captures inspected; scoped
acceptance is `approval-binding-filter-scoped-acceptance.json`. Actual functional evidence is
`approval-binding-filter-actual-v3/report.json`. Evidence is under
`approval-binding-filter-native-v2` and `approval-binding-filter-applied-vm-v3`
in the same stage directory.

Populated original approval-history transfer, tool-rule management and private
approval decisions remain unfinished. Source inspection confirms that the original
web permission page also needs readable saved rules and next-dispatch policy
editing; the retained native grant summary alone does not restore that behavior.
The audit is `tool-approval-port-source-audit.json` in the stage directory.
The original desktop45/server32 installations are unchanged. Candidate46 is not
promoted, and full migration, real OAuth persistence and whole-app UX remain open.

## Running checkpoint: integration45 (additional resource configurations)

The mini app now creates another configuration from an authorized source using
the same account and shared quota. Source configuration and existing grants stay
unchanged. A distinct native review shows source and proposed limits, and success
opens the new resource. Local publication is separate from server admission.
The prior44 first-device setup remains available; the prior43 system-HOME
keychain repair remains in the release. Real OAuth persistence still awaits a
new completed browser authorization and is not claimed by the synthetic tests.

The paired release passed50 host tests,11 native confirmation-layout tests,
46 creation VM cases,42 existing editor regressions, two bundle roots and the
translation audit. Native source has its separate focused/process and strict
feature-configuration qualification. These counts are overlapping scopes.
Actual hidden Metal windows pass EN/light cancel/create, creation from a locally
published source, committed-effect/private-reply-loss recovery after normal
restart, and CN/dark cancel/zero/clear plus another normal restart.12 original
captures were inspected. The fixture uses offline provider metadata and an
existing loopback Matrix account; it is stopped. Two test-navigation failures
are retained with their selector corrections; product source did not change,
and already completed resource creation was not replayed with a fresh ID.

The33 accepted source paths are promoted locally with backups. All four original
role sessions run release SHA256
`bd09d046df4165547d229ddad56ba5b5d0b247a7e1e9917e7f2b419fef41cf80`
with unchanged SDK/device identities. The original EN/CN settlement readback
passes: ledger125, known/retained16161, unresolved0, stopped execution and
original calls/budgets/top-up/exhausted agent unchanged. Local server32/schema21
is unchanged. No commit, push, merge, AppHub publication or production deployment.

Evidence: `.run/management45-stage/create-ui-build-v1/{qualification.json,actual-ui/report.json,visual-review.json}`,
`.run/management45-promotion/manifest.json`, `.run/management45-activate-*` and
`.run/management45-stage/acceptance-observe/report.json`. Native qualification is
`create-native-v6/qualification.json`; earlier native regression scopes remain
recorded separately. MGT-064 now has a named macOS scope and selectors in the
parity inventory. This is not whole-app9/10 or all-platform acceptance.

Remaining work includes admitted-resource amendment and publication/withdrawal,
resource deletion/daily limits, bound-account lifecycle/import, tool approvals,
remaining agent/project/work/notification functions, alternate runtime adapters,
real OAuth enrollment and full original JavaScript/platform acceptance. The
full desktop migration goal remains active and incomplete.

## Historical checkpoint: integration43 (resource provenance and local editing)

The source candidate records exact creation proof and irreversible first offers
in schema93, and adds a closed canonical resource editor. Model/reasoning and
resource-ceiling edits preserve resource/account/seat identity and all original
allocations. A native managed-namespace adapter checks fresh provider metadata,
retains the original command, and recovers committed edits before provider RPC.
The baseline passed 528 tests; the final native recovery/shutdown amendment passed
76 tests and both strict Clippy configurations against unchanged store sources.
Three opt-in real-provider tests remain skipped. Evidence is under
`.run/management43-stage`, including the exact qualified source delta.

A further isolated amendment adds current edit eligibility and explicit blocker
reasons in the same read snapshot as resource detail and local name. It passed 18 resource
editor/detail/name tests, 49 dependent native owner tests and both strict Clippy
configurations. Separate evidence is under `editor-eligibility-v8`; these counts
overlap earlier suites.

The closed mini-app editor and EN/CN native review are now implemented and
qualified. Native review reads current values before enabling Save; cancelling
does not write. Saving rechecks account/model availability and canonical
eligibility. The resource ID, account namespace, shared quota and allocations
are retained. Recovery presents the saved original result before provider RPC.

The v9 integration passed 49 native owner tests, both strict native Clippy
configurations, 49 Desktop host tests and three native consent geometry tests.
The v10 presentation amendment passed 42 editor VM cases, two complete-bundle
cases, the i18n audit and release build. Its native source is unchanged from v9.
These overlapping suites must not be added to the earlier totals.

Actual hidden macOS Metal windows pass EN/light and CN/dark draft, cancel and
save, plus a lost-reply restart using the exact original command. Recovery does
not repeat the canonical change or contact the offline metadata provider. The
shared 700-token account limit stays unchanged. Earlier v9 native tests also
cover zero and explicit ceiling clearing. Five final captures were inspected;
the monthly label and translated reasoning are legible and consent controls
remain reachable. This is targeted editor acceptance, not whole-app 9/10 or
mobile validation. A test-driver child-reaping timeout was corrected without
resubmitting the saved edit; its failed report is retained.

The exact 56 native and 16 Desktop source paths are promoted locally with
backups. The qualified release SHA256 is
`ffae33ca60c18a7cb7118b3fb3eb52e65dea8dd30c16922b65fcdedad7c35c62`.
Evidence and source archive: `.run/management43-stage/editor-ui-v10`; promotion:
`.run/management43-promotion/manifest.json`. The dedicated offline fixture8324
is stopped. All four original sessions now run43 with their original SDK/device
bindings. SQLite-consistent canonical/private backups were saved before each
normal restart. Opened databases use schema93; the owner's unopened archived
private journal retains its original schema79. EN/CN read-only settlement
regression passes: ledger125, known/retained16161, unresolved0, processes stopped,
original calls, budgets, top-up and exhausted agent unchanged. Server32 is
unchanged. Activation evidence: `local-activation.json` and
`acceptance-observe/report.json` in the same stage. No commit, push, merge or
production deployment.

Real OAuth completion/enrollment, explicit resource duplication, central
amendment/publication and the other retained-feature gaps remain open. The fresh
empty-installation path also needs a clear create-first-resource action: current
shared-capacity review can show a generic failure before the first resource
exists. Full migration remains incomplete.

## Historical checkpoint: integration42 (resource alert history transfer)

Explicit native migration now preserves populated resource alert history. Its v3
plan includes the original alert IDs, raw detail, status, assignment, suppression,
notes and completed handling receipts. Saved v1/v2 plans keep their original
scope. Import is atomic, rejects collisions and changed reviews, and resumes the
original migration after interruption.

Store adoption checks70, native activation checks6, source-recovery checks8 and
both strict native configurations pass. Desktop adoption checks5 and host checks48
pass before a context-label correction. The corrected release is rebuilt from
unchanged qualified native source; actual EN/light and CN/dark captures verify
"Resource alerts" / "资源告警" instead of the unrelated Markdown label.

Actual hidden native Metal interaction passes on the original independent
synthetic fixture: review/cancel, exact2-alert/3-note/4-receipt migration, one
new handling decision, visible saved note, refresh and normal restart. Original
legacy rows and all resource/account/budget rows remain unchanged. The metadata
provider is explicitly offline; this is not real OAuth or inference acceptance.
Two test-driver mistakes (an abbreviated Chinese label and an offscreen note)
were corrected without repeating a submitted decision. Their evidence is retained.

The25 native and3 Desktop paths are promoted locally with backups. All four
original roles run schema92 and binary SHA256
`4977c6e6fd17b845b8b151983108260a50cdc313ee284f4a8482fa5056fde969`,
including the macOS keychain repair below. Original SDK/device bindings are
preserved. The existing EN/CN settlement regression passes: ledger125,
known/retained16161, unresolved0, all processes stopped, original calls/top-up
and exhausted agent unchanged. Server32/schema21 is unchanged.

Evidence: `.run/management42-stage/checkpoint.json`, `actual-acceptance.json`,
`visual-review.json`, `local-activation.json` and `acceptance-observe/report.json`.
Real provider authentication/enrollment, the remaining retained-feature gaps,
Matrix notifications/reminders and whole-app/platform acceptance remain open.
No commit, push, merge or production deployment. Full migration is incomplete.

## Current validation: macOS provider sign-in repair

Real user authorization exposed `persist_failed`: Codex completed OAuth but
Security.framework could not find the default keychain. A read-only native
probe reproduces status -25307 with the isolated account HOME and status 0
with the OS account home. The keychain is unlocked; both direct and system
proxy connectivity checks succeed.

The isolated `management41-keychain-fix` candidate selects macOS HOME from the
OS user database for Codex children while retaining their exact CODEX_HOME,
working directory, environment allowlist and keyring-only credential policy.
Other platforms keep their prior HOME behavior. Login, catalog and relevant
credential-reading launch paths share the helper. The repair passes 61 native tests (six opt-in installed-provider tests remain
skipped), strict Clippy, and 48 Desktop host tests. The release executable is built and the exact 11 changed paths are promoted
locally with backups. The repair was activated on all four original roles as binary
`0bdacf14d56ee0066bd3891b9402b64dd5a48d10a17524e5b067c1e15bc1b7cf`;
checkpoint42 now includes that same repair and retains their SDK/device bindings. Actual provider-child inspection
confirms OS HOME with the original selected CODEX_HOME. Attempt 4 opened its native authorization page but expired after the 10-minute
authorization deadline without a completed callback. Real credential saving
and resource enrollment are not yet confirmed; the original account namespace
is retained for a user-initiated retry. Evidence is under
`.run/management41-keychain-fix`. No production deployment or commit.

Additional native keychain verification uses a unique temporary generic-password
item, never OAuth credentials: the old isolated HOME reproduces -25307 without a
write; the repaired OS HOME successfully adds and reads back the item, deletes it,
and confirms it is absent (-25300). System keychain settings are unchanged and no
test item remains. Evidence: `.run/management43-stage/keychain-write-check/results.json`.
This verifies the storage environment; real OAuth completion remains pending.


## Historical checkpoint: integration41 (embedded resource alert observation)

The embedded Rust owner now observes resource alerts without opening the mini app
or running the local management web server. It shares the retained reconciler,
checks current SDK/profile and exact resource authority inside each transaction,
and prunes only that resource's expired resolved alerts. It visits bounded pages,
uses the retained hourly interval after a complete pass, and joins on shutdown.

Both strict native configurations pass. Store alert checks8, retained alert
regressions11, native alert checks3, owner lifecycle checks34 and management drain
checks3 pass (filters may overlap). Desktop host tests48 and release build pass
against the unchanged qualified source.

Actual Desktop startup acceptance passes with only the main window open: the
observer removes one expired synthetic row while original alerts, notes, four
triage receipts, resources, execution and budget records remain identical. Opening
the mini app afterward shows the original CN/dark resolved and suppressed records;
refresh creates no extra command. The isolated fixture then shuts down normally.
This verifies background startup/retention, not a real-provider overrun or an
hour-long scheduling soak.

Reviewed changes are promoted locally with backups. All four original roles run
41/schema92, binary SHA256
`b05811ed315f1d1a10ccbf8cc5b1b314380dc2ea0cdc5b254c2ce1ff283b4b54`.
Their restart captures match the inspected40 captures. EN/CN original settlement
regression and visual review pass: ledger125, known/retained16161, unresolved0,
stopped processes and unchanged original calls/top-up. Central server32/schema21
is unchanged. Evidence is under `.run/management41-stage`, including
`actual-observer/report.json`, `local-activation.json`, `schema-check.json`,
`visual-review.json` and `local-visual-review.json`.

The next isolated source is42 for explicit populated alert-history transfer.
Legacy-history adoption and Matrix notification/reminder delivery remain open,
alongside real provider authentication/enrollment and the other retained-feature
and platform acceptance gaps. No production change, provider call, commit, push
or merge. Full migration remains incomplete.

## Historical checkpoint: integration40 (resource alert read and triage)

Resources now exposes retained alert history, bounded filters and notes, and the
original legal handling actions. Each decision has a review step and an immutable
original-command receipt. Refresh and recovery do not repeat it. Native authority
checks remain scoped to the current SDK profile and exact resource grant.

The final source passes both strict native configurations, focused store checks5,
retained alert regressions11, native SDK recovery checks2 and Desktop host tests48.
The release build passes. The EN/CN action VM regression passes42 after exposing
and fixing repeated widget IDs that had collapsed four actions into one.

Actual native EN/light and CN/dark interaction covers acknowledge, assign to self,
resolve, suppress, cancel, refresh and normal restart. Four submitted decisions
produce exactly four original receipts. The synthetic fixture has no provider
credentials or calls; resources, execution and budget records remain unchanged.
The failed UI/test-driver evidence and subsequent corrections are retained.

Reviewed source is promoted locally with backups. All four original localhost
roles run40/schema92, binary SHA256
`02f5ea15297a6d784018d654f6ea361d2fe17bdb92ce4caa884bb679e42cafd3`.
Original settlement regression remains ledger125, known/retained16161, unresolved0
and processes stopped. Actual restart and settlement captures were reviewed;
fresh observation timestamps change some pixels without changing the receipts.
Central server32/schema21 remains unchanged. Evidence is under
`.run/management40-stage`, including `checkpoint.json`, `actual-acceptance.json`,
`visual-review.json`, `local-activation.json` and `acceptance-observe`.

Integration41 subsequently accepts the embedded background observer as recorded
above.
Populated legacy alert adoption, Matrix notification/reminder delivery, real
provider authentication/enrollment and full retained-feature/platform acceptance
remain open. No production change, provider inference, commit, push or merge.
Native captures use hidden Metal windows, not GPU-less headless rendering.

## Historical checkpoint: integration39 (unused account-space retirement)

The native and EN/CN mini-app source now exposes original-request retirement
through a dedicated capability and a native confirmation. The transaction checks
profile ownership, expected revision, all installation resource associations and
pending sign-in before fencing the namespace. The transition, logout-unobserved
observation and original result commit together; recovery returns that result.
Retirement does not call the provider, delete credentials or free allocated tokens.
Bound namespaces remain refused pending a complete resource/agent drain protocol.

Two strict native configurations pass. Store regression passes411 ordinary tests
with37 ignored. VM account views pass72, retirement callbacks22, existing enrollment
callbacks24 and full bundle roots2. Translation check reports0 missing. Full native regression passes518 with68 ignored; the three retained legacy
retirement integration tests also pass. Exact-source Desktop tests/release are
complete: backend37, host47, execution10 and geometry9 pass (overlapping filters),
including360×640 native confirmation in both languages; the release build passes.

Exact reviewed changes are promoted locally with backups. All four original
localhost roles run39/schema92, binary SHA256
`9305aed95627b7f42af673609ed0cf2ef350899e30fd41b5ea55ced3ad228355`.
Actual EN/light and CN/dark creation of a NEW empty test namespace, native
review/cancel/retire, refresh and normal restart pass. Both retired-detail restart
captures match the inspected originals byte-for-byte. Original active account IDs,
Matrix sessions, resources, calls and budgets remain unchanged. The original
settlement remains ledger125, known/retained16161, unresolved0 and processes stopped.
Central server32/schema21 remains unchanged. Evidence is in
`.run/management39-stage/checkpoint.json`, qualification reports,
`acceptance-retirement-*`, `acceptance-observe` and `visual-review.json`.

These are actual hidden native Metal captures, not GPU-less headless tests.
Completed real authentication/enrollment, bound-account retirement, full retained
feature parity and whole-app UX/platform gates remain open. No production change,
provider login/inference, commit, push or merge. Full migration remains incomplete.

## Historical checkpoint: integration38 (selected-account resource enrollment)

Isolated38 now implements the full selected-account enrollment slice: choose an
original private account and available model, declare a monthly limit, review in
a native confirmation, then save an unpublished canonical resource. The mini app
gets no provider credentials or filesystem paths. Native verification rechecks
account identity, public quota identity and the selected model around a finite
metadata child, and observes that child stopped before accepting its proof.

Schema92 records the canonical resource-to-namespace binding separately from the
physical quota identity. Resource, namespace binding, profile grant and original
result commit atomically. Different namespaces using the same physical provider
quota share one ceiling. Saved-command recovery reads the exact original result
before making any provider RPC. A missing recorded namespace refuses default-home
fallback. Bound accounts cannot silently relogin or retire via the old direct APIs.

Metadata qualification, first text execution, successor execution, budget
successors and connection verification use that original namespace. Current SDK
scope is checked throughout; cleanup retains independent custody. Offline process
probes verify both metadata and text initialization, the selected HOME/CODEX_HOME,
absence of ambient API keys, observed process stop and revoked-handle rejection.
These probes do not perform real authentication or inference.

Full native regression passes516 ordinary tests with68 ignored; store regression
passes405 with37 ignored. Two strict configurations pass. Six later native file
changes add readiness and recovery guards; their focused qualification is recorded
separately in `readiness-qualification` and `readiness-final3-qualification`, without presenting the older full run as
an exact-source final run. The early guard refuses legacy-store enrollment before
saving an intent. The saved-command recovery probe now passes with no additional provider RPC.
It executes as an awaited task, matching Desktop dispatch, after a debug-only
nested fixture stack overflow. The original stack limit and assertions remain. The unchanged store does not require a repeated full run.

The EN/CN UI distinguishes device setup from enrollment, and cancelled review
reloads saved original requests instead of claiming no earlier resource exists.
Account view checks pass64, enrollment callbacks24, model callbacks16, login
callbacks10 and shipped full-bundle roots2. An initial populated-model layout
failure in the pinned script VM was repaired and the failed evidence retained.
The first release test groups passed backend37, host46, execution10 and geometry8
(overlapping filters), including a narrow-window confirmation with a fixed footer.
That superseded release compilation was stopped before the final corrections;
final exact-source Desktop qualification passes the same four filters and the release build.

Evidence is under `.run/management38-stage`: `native-final-qualification`,
`readiness-qualification`, `ui-accounts-final3`, `ui-enrollment-final3`,
`ui-full-final3`, `review`, and `failures-and-corrections.json`. Exact patches replay
against37 for54 native paths and12 Desktop paths. The reviewed paths are promoted locally with backups; all four original localhost
roles run38/schema92, binary SHA256
`5052b5a9a0a5b687c0883b3ddf102826aa4c71660f9bf3b55aa4e159bef89a0f`.
Actual EN/light and CN/dark signed-out model checks and normal restart pass,
with all four restart captures identical to their inspected originals. Existing
Matrix sessions, original account IDs and business records are preserved.
The read-only settlement regression remains ledger125, known/retained16161,
unresolved0, usage complete and processes stopped. Central server32/schema21 is
unchanged. Evidence includes both `acceptance-models-*` reports,
`acceptance-observe`, `schema-and-preservation.json` and `visual-review.json`.
These are hidden native Metal captures, not GPU-less headless tests. Actual
authenticated new-enrollment UI and completed real provider login remain unaccepted. No production change,
provider inference, commit, push or merge. Full104-operation migration remains open.

## Historical checkpoint: integration37 (selected-account model inspection accepted locally)

`hagency.accounts.models` has a separate read grant and observes only the original
selected namespace. Ownership, revision, pending login, namespace lifetime and
current SDK authority are checked around the short-lived metadata child. It must
stop before success. Public replies contain model choices without email, credential
paths, authorization URLs or quota proof. Reads do not sign in, infer, enroll or
change readiness. Late replies cannot replace another selected account or page.

Strict native checks pass in both configurations. Store regression passes398 with
37 ignored; native regression passes511 with68 ignored. Desktop backend37, host45,
execution10 and geometry7 filters pass (filters overlap), and the release build
passes. Script checks: account views54, model callbacks16, login callbacks10 and
full bundle roots2. Translation check:0 missing. Authenticated catalogs are tested
with an offline protocol child, not a completed real provider authentication.

The exact reviewed paths are promoted with backups. All four original localhost
roles run Desktop SHA256
`9854677c106d465a65e525f392d277656a8d8282355f12b78b46583a2f8d537e`.
Native schema91 and central server32/schema21 remain unchanged. Actual EN/light
and CN/dark selected-account reads correctly show the signed-out message without
default-profile fallback or a fabricated empty catalog. Refresh clears stale
observations. Normal restarts preserve the original account IDs and Matrix
sessions; the four resulting captures are byte-identical to the inspected views.
No login attempt or inference is started. Original resources, grants, calls,
budgets and top-up are unchanged. Settlement remains ledger125, known/retained
16161, unresolved0, usage complete and original processes stopped.

Evidence: `.run/management37-stage/checkpoint.json`, both qualification reports,
`acceptance-models-observe/report.json`, `acceptance-models-restart/report.json`,
`acceptance-observe/report.json` and `visual-review.json`. These are actual hidden
native Metal windows, not GPU-less headless tests. No production change, commit,
push or merge. Full104-operation migration and whole-app UX/platform gates remain
unfinished.

## Historical checkpoint: integration36 (provider sign-in and cancellation accepted locally)

The mini app now offers login for the selected original private Codex account.
A separate capability opens native review, followed by explicit provider-page
opening and completion/cancellation status. The original attempt and command
receipt commit together before spawning. Saved-command recovery only observes
that attempt. Current SDK, profile and grant checks protect the process and
native authorization delivery. Login does not enroll or allocate resources.

Strict native checks pass with and without shared execution. Store regression
passes395 ordinary tests with37 ignored; native regression passes508 with68
ignored. Desktop backend37, host44, execution10 and geometry7 filters pass
(filters overlap), and the release build passes. Script VM checks pass: account
views44, preparation8, login callbacks10 and full bundle roots2. Offline provider
fixtures test completion and authority loss; they are not real authentication.

Exact reviewed changes were promoted with baseline guards and backups. All four
original localhost profiles run Desktop SHA256
`21122007de5abcafc56014f03d55e3775b821bcecc3aa994f680fde0be66214c`.
Normal activation preserves existing Matrix sessions; server32/schema21 is
unchanged. Actual EN/light and CN/dark review/cancel creates no attempt before
Start. Two original provider authorization attempts then start and cancel
successfully, recording stopped outcomes in their own namespaces. Refresh
neither repeats login nor marks the accounts authenticated. No browser, credential
entry or model inference is involved. Post-cancellation normal restart preserves each original cancelled attempt; no
login is repeated. The original settlement regression also passes: ledger125,
known/retained16161, unresolved0, stopped processes and one original result.
Original resources, grants, agent calls, budgets and top-up are unchanged.

Evidence: `.run/management36-stage/checkpoint.json`, both qualification reports,
`acceptance-login-review/report.json`, `acceptance-login-cancel/report.json`,
`acceptance-login-restart/report.json`, `acceptance-observe/report.json` and
`visual-review.json`. Earlier failed attempts remain recorded. These are actual
hidden native Metal windows, not GPU-less headless tests. Account-holder
completion of fresh authentication, retirement/enrollment/import and full
retained feature parity/UX/platform acceptance remain unfinished. No production
change, commit, push or merge.

## Historical checkpoint: integration35 (private account-space preparation accepted locally)

The mini app now creates an empty private Codex account space owned by the current
Matrix profile, with EN/CN review, cancel and original-request recovery. The
reservation, immutable ownership and command receipt commit before directory
creation. Recovery returns the same account; it does not repeat an uncertain
filesystem operation. Schema91 preserves older unscoped namespaces without
assigning them to a current user. This is not provider login or enrolled capacity.

Strict checks pass with and without shared execution. Native regression passes
504 ordinary tests with68 ignored. Store regression passes390 with37 ignored;
that completed run is reused only after verifying exact unchanged store and
dependency sources. Desktop backend37, host43, execution9 and geometry6 filters
pass (filters overlap), as does the release build. VM checks pass:44 account
views,8 recovery cases,10 naming regressions and2 full bundle roots.

Regression found and fixed a real stack overflow in saved-request recovery.
Boxing each selected operation's future reduces its size from45656 to3984 bytes;
the original reproducer and a16KiB bound pass. Earlier schema audit/rewind,
callback recovery, compilation and stack failures remain recorded in
`.run/management35-stage/failures-and-corrections.json`; they are not passes.

The reviewed35 native and8 Desktop paths were promoted with exact baseline
guards and backups. All four original localhost profiles run Desktop SHA256
`5427e396cecdb5f36eb0e07f58c4203663e4ad3ce461bbc4aacd79944397bcd1`.
Central remains qualified32/schema21. Actual EN/light owner and CN/dark
coordinator review/cancel/create/refresh and cross-profile isolation pass.
Normal restarts preserve exactly one original account per profile and the
existing Matrix sessions. Login remains explicitly unverified. Original
provider observations, resources, grants, agent calls and budgets are unchanged.
The settlement regression passes: ledger125, known/retained16161, unresolved0,
usage complete, processes stopped, exactly one original result.

Evidence: `.run/management35-stage/checkpoint.json`,
`acceptance-preparation/report.json`, `acceptance-restarted-preparation/report.json`,
`acceptance-observe/report.json` and `visual-review.json`. These are actual hidden
native Metal windows, not GPU-less headless tests. No provider call, production
change, commit, push or merge. Fresh provider login, retirement/enrollment,
populated import, full104-operation parity and whole-app9/10/platform acceptance
remain unfinished. See [provider accounts](hagency-provider-accounts.md).

## Historical checkpoint: integration34 (read-only provider accounts accepted locally)

The paired34 source implements `hagency.accounts.list/get` with a dedicated read
grant, original managed-namespace state/readiness, profile-scoped canonical
bindings, associated resources and bounded pagination. EN/CN pages distinguish
saved association from current provider authentication. Refresh is read-only;
late replies cannot replace a different page or selected account.

Strict native checks pass in both configurations. Native regression passes501
ordinary cases with68 ignored; store regression passes384 ordinary cases with37
ignored. The3 new SQLite tests are included in384. Desktop backend37, host42,
execution9 and geometry6 checks pass (overlapping filters); the paired release
passes. Script VM account checks34 and full bundle roots2 pass. Initial native
and script failures are retained with their subsequent corrections in the stage.

Exact reviewed patches reproduce the12 native and7 Desktop changed paths.
They are promoted with baseline guards and backups. At checkpoint34, all four original localhost
profiles ran Desktop SHA256
`d76705f16db6384bf10c13f2f92850ae4ab269d83276f83b9b8aa04bc3733a17`;
the server remains qualified32/schema21. Normal restarts preserve the SDK sessions.
Actual EN/light owner account list/detail/refresh/resource navigation and CN/dark
other-profile empty state pass. Another normal owner restart preserves the same
account ID and resources. UI reads leave calls, budgets, grants and account rows
unchanged. The original settlement readback also passes in EN/CN: ledger125,
known/retained16161, unresolved0, stopped, complete usage, one original result.

Evidence: `.run/management34-stage/checkpoint.json`, `acceptance-accounts/report.json`,
`acceptance-restarted-accounts/report.json`, `acceptance-observe/report.json` and
`visual-review.json`. These are actual hidden native Metal windows, not GPU-less
headless tests. No new provider call, production change, commit, push or merge.
Account preparation/login/retirement/enrollment, populated namespace acceptance,
full104-operation parity and whole-app9/10/platform acceptance remain unfinished.
See [provider accounts](hagency-provider-accounts.md).

## Historical checkpoint: integration33 (human settlement flow accepted locally)

The isolated paired source at `.run/management33-stage` adds typed settlement
read/request services with separate package grants, a durable original human
intent and exact POST replay under current central visibility. The server's
settlement command ID is distinct from the original human request ID; retry and
acknowledgement retain the latter. Executor payloads/tickets are not projected
to the mini app.

The EN/CN removal page now shows latest settlement separately from immutable
cleanup history. Pending delivery, unresolved usage, current proof and newer
unreconciled evidence have distinct labels. Refresh is read-only; recheck follows
the server's current permission and blocks while an original command is pending.
Old servers retain the previous cleanup presentation. Original fence accounting
moves into Details when the latest-settlement feature is present.

31 new VM checks, 43 existing cleanup VM checks and two complete bundle-root
checks pass. Three actual loopback HTTP/SQLite tests pass, including normal
suspend/reopen after a lost reply, fresh role checks on cached success, SDK scope
loss and read-only grants. The initial restart fixture kept an old owner lock,
then used logout instead of suspend, then lacked the current-device resume route;
those failed attempts remain in the stage. Two excess closing braces in the new
script section were caught by full-bundle/root checks and repaired before the
passing runs. Neither VM checks nor HTTP fixtures are native visual acceptance.

Enabled and default strict native checks pass after collapsing one nested
conditional. Full native regression passes501 ordinary cases with68 existing
ignored cases;21 management store checks pass. The7 native paths are promoted
with exact baseline checks and backups in `.run/management33-promotion/native`.
Paired Desktop backend37, host41, execution9 and native-component6 checks
pass (overlapping filters). The6 Desktop paths are promoted after baseline
checks and backups. The final release and paired local acceptance pass. The
initial Desktop attempt used the wrong dependency patch URL and
failed offline before compilation; it now uses the exact qualified32 dependency
source. A subsequent build was stopped before the native lint repair. Logs and
source manifests for all attempts are retained.

`octo doctor` passes, but the standalone App Hub gate refuses the bundle's
required `hagency-management-v1` host feature. This is not a successful App Hub
validation. The bundle is restamped through the Desktop's paired contract tool;
actual host checks and native UI are the applicable local acceptance path.
No App Hub publication, platform claim or external submission is requested.

At checkpoint33, all four original localhost role profiles ran Desktop SHA256
`e0d9d62ed05be6db7ed5a722428a88f9024d9431a58e04785cf622738ad33cce`.
The localhost server runs qualified32 SHA256
`166ef6a77306545bc72abf68d8a9890b6d48a01e372c2fcda292f0490a7af433`,
schema21. Normal restarts preserve the original SDK sessions/account/device
bindings; consistent SQLite and PostgreSQL backups precede activation. Schema
upgrade does not enroll historical agents for settlement. The first activation
comparison detected rotating runtime leases; comparison against the restored
backup confirmed that only lease hash/expiry changed, with business history
unchanged. Both attempts and the diagnosis remain recorded.

Actual native EN/light manager and CN/dark assigned-project-administrator
acceptance passes. One click queues one observation for the already revoked
validation agent. A repeated disabled click does not create another request.
The owner worker records exactly one fresh settlement at ledger revision125:
known consumed16161, retained16161, unresolved0, complete usage and stopped
processes. The previous conservative retained amount was28096. The original
incomplete fence receipt remains byte-identical and visible separately under
Details. A normal manager restart reads the same single result. Both languages'
pending and complete native Metal captures were inspected. Disabled-button
styling and the whole-app9/10 gate are not accepted by this scoped check.

Evidence: `.run/management33-stage/checkpoint.json`, `acceptance-queue/report.json`,
`acceptance-observe/report.json`, `acceptance-restart-observe/report.json` and
`visual-review.json`; role restart evidence is under `.run/integration33-restart-*`.
No new provider call, grant renewal, agent recreation, exhausted-agent replay,
production change, commit, push or merge occurred. Existing calls, top-up and
budgets remain unchanged. The11935 reduction in uncertain liability is not a
spendable balance or refund; the grant remains expired. The whole migration,
original104-operation acceptance,9/10 whole-app UX and mobile/platform gates
remain incomplete.

## Historical checkpoint: integration32 (paired settlement core qualified, UI/E2E pending)

Central schema 21 in the isolated paired source now retains separate settlement commands/results and finite
executor tickets. New fence results queue an observation automatically; replayed
historical fences and schema migration do not enroll old agents. New evidence
queues a further observation only for already enrolled agents. Current managers
and assigned project administrators can read/request settlement; original
owner/device authority is required to upload native evidence.

Latest complete proof can lower the central retained floor only when its exact
budget head is known, no budget result is pending and no newer process/work
observation contradicts it. Late consumption raises the floor. Work, fence and
settlement amounts are combined with a per-agent maximum, not added twice.
Original receipts and global ledger revision uniqueness are preserved. Pending
and applied top-up results remain distinct even when their HTTP replies arrive
out of order.

The native HTTP adapter recovers its original local proof before doing any work,
uses current SDK authority and finite leases, and cannot create a proof from a
historical delivery. Lost upload/ack, restart, changed SDK generation, missing
capability, wrong device and host retirement have coverage using real loopback
HTTP and SQLite. The Desktop shared worker polls it after cleanup/process
reconciliation with an independent cursor; this runs outside the mini-app view.

Qualification currently passes both strict native configurations and 498 native
ordinary tests, with 68 existing ignored cases. The 5 new transport tests are a
subset of that 498. Store source is byte-identical to qualified 31; its 381 ordinary
and 37 supervised offline tests are prior evidence, not newly rerun tests.
Central 11 focused tests pass, including schema 20 rollback/reopen without old-agent
auto-enrollment, transactional enqueue rollback and pending/late budget evidence.
Paired Desktop backend 37, host 40, execution 9 and geometry 6 checks pass (overlapping
filters), and the release build passes. Server broad regression passes 346 cases
with one existing finite-ticket test failure: its host-clock wait did not prove
the original PostgreSQL ticket had expired. The corrected test waits for that
unchanged ticket's actual database deadline under a bounded monotonic timeout,
then verifies rejection by the production endpoint. It passes separately. Only
that test file changed after the broad run; production source is identical.
Server all-target strict lint and release build pass. This covers 347 distinct
cases across runs, not one clean full-suite run. The 11 focused cases are included
in that coverage, not additional distinct tests.

Earlier failed logs remain: the first native fixture queried a nonexistent
receipt `operation` column; it now checks its exact command ID. Older PostgreSQL
rewind helpers initially left the new journal/view behind, causing cascading
fixture failures; they now remove schema 21 objects before rewinding. The enqueue
fault test initially retried a rotated original executor ticket, then incorrectly
asked a still-leased command to poll again; it now uploads the original proof
using its fresh historical recovery ticket. The superseded broad run was stopped
and its isolated test database removed before the final broad run.

Review patches under `.run/management32-stage/review` reproduce exact source from
qualified 31 (native/server) and 30 (Desktop); the combined native patch also applies
to qualified 29. All changed paths apply cleanly in independent temporary trees.
The 21 native/client paths have been promoted after exact baseline checks, with
backups in `.run/management32-promotion/native`. The one Desktop worker path
is also promoted after its paired checks, with a backup in `desktop` beside it.
No live activation, provider request, grant renewal, commit, push,
merge or production/mini1 deployment has occurred in this slice.

The latest-settlement mini-app view and durable manual-recheck intent remain
unfinished, as does real paired localhost acceptance. The next steps are in
[agent settlement](hagency-agent-settlement-next.md). Full 104-operation acceptance,
EN/CN whole-app UX and platform gates remain open; this settlement slice does not
complete the entire migration. Running profiles/server remain integration30.

Evidence is consolidated in `.run/management32-stage/checkpoint.json`,
`native-qualification/qualification.json`, `desktop-qualification/qualification.json`
and `hagency-server/.run/management32-stage/server-qualification-final/qualification.json`.
Qualified Desktop SHA256 is
`c29302fda879cdfdecc9826acde0bfbce6f9c6c178bbce79d0a0c8d25790718d`;
server SHA256 is
`166ef6a77306545bc72abf68d8a9890b6d48a01e372c2fcda292f0490a7af433`.
The 30-file `server32-after30.patch` applies exactly to the qualified running30
source. Server main is older and dirty, so it remains unmodified; do not treat
that patch as an automatic overwrite of server main. No actual Makepad capture
or live paired settlement is claimed for 32; its UI is unchanged from 30.

## Historical checkpoint: integration31 (native settlement core qualified)

A fresh `agent.settle` operation now observes the exact original permanently
fenced agent without rewriting its original fence receipt. It uses the shared
bounded liability calculation, full applied budget chain and original/successor
process-stop evidence. Late complete usage can reduce the reported hold; unknown
usage, process custody and overrun remain retained. The result is append-only,
with atomic ledger revision, exact replay/recovery and original owner/device
checks. It does not issue provider work, grant credit or release the parent.

Native source is isolated in `.run/management31-stage/hagency-client`, based on
the qualified29 source. Both enabled/default strict native checks pass. Complete
store validation passes381 ordinary and37 explicitly supervised offline cases;
native validation passes493 ordinary cases with68 ignored. The focused11 cases
are part of that coverage, not additional unique tests. The supervised probes
are actual offline children, not real provider requests. The first new fixture
used an execution ID where a native call ID was required; its failed log is
retained and the corrected focused/full runs pass.

Central31 adds closed settlement wire types and a synthetic canonical fixture
only. Its two focused tests and agent-service strict check pass. A separate
native public-API executable accepts exactly the same central fixture; this is
wire compatibility, not authenticated HTTP or ledger-proof acceptance. No central
schema change, settlement route, background adapter or mini-app presentation is
implemented yet. Full PostgreSQL tests were not rerun for these unused types.

The13-path native patch reproduces exact source from qualified29 in an independent
temporary directory. Evidence: `.run/management31-stage/native-checkpoint/manifest.json`,
`native-qualification/qualification.json`, `wire-qualification.json` and
`hagency-server/.run/management31-stage/protocol-qualification.json`.
The implementation plan is [agent settlement](hagency-agent-settlement-next.md).

All running profiles/server remain on integration30 below. Main client source is
still29. No live agent, grant, provider call, top-up or account was changed; no
commit, push, merge or deployment occurred. Next work is paired central storage,
finite recovery transport, conservative latest-balance projection, native worker
and EN/CN mini-app flow, followed by real E2E on only the already revoked
validation agent. Full retained-management parity, UX and mobile gates remain open.

## Historical checkpoint: integration30 (bounded cleanup acceptance complete)

All four original localhost role profiles now run Desktop
`80539992d6336471e2993215e0f113bd30476d5c744a93cf76cff96a836c8d98`.
Normal restart preserved SDK sessions, account/device bindings and consistent
SQLite backups. The server remains schema 20 with unchanged configuration,
running `fdcffb10ec186e07e018a2670e8b12b3d83cdb25539851ff877e0820d36f75b8`.
No production or mini1 deployment occurred.

Cleanup now includes applied and possibly applied top-ups in the conservative
retained amount when the original fence receipt is incomplete. A complete native
fence remains authoritative for confirmed unused capacity. The UI separates
server restriction, latest process observation, confirmed Matrix room exit and
accounting. Original delivery/process observations and request identity are
available under Details. Approval, rejection and cancellation have distinct
budget explanations; an applied increase no longer says it is awaiting application.

Actual EN/light manager and CN/dark assigned-project-administrator acceptance
passes: review/cancel has no write; the existing validation request is revoked
once (revision 2 → 3); the manager reads the latest revoked result and retains it
after normal restart. Matrix independently returns membership `leave`. The
original exhausted agent ledger is unchanged, as are the validation agent's four
calls, one 4096-token delta, original 24000 reservation and 16161 known consumed.
The cleanup view retains 28096; this includes spent/uncertain amounts and is not
available capacity or a refund. Both grants had already expired naturally before
this slice, and all processes were already stopped. This does not prove termination
of a live provider process or complete final settlement.

Checks: cleanup VM 43, budget VM 28, full-bundle EN/CN 2; Desktop backend 37,
host 40, execution 9 and geometry 6. These filters overlap. Server broad regression
passed 337 cases with one new test-fixture expectation failure. The corrected
cleanup filter passes both cases; all-target strict lint and release build pass.
That covers 338 distinct cases across runs, not a single clean full-suite rerun.
Production source was unchanged by the test-only correction. The repeated Desktop
compile was traced to absent Git metadata in the isolated copy; remaining filters
used the exact already-built test binary, with unchanged source hashes.

Evidence: `.run/project30-cleanup-ui-accepted/acceptance.json` and twelve reviewed
whole-window native Metal captures, plus `restart-observation.json`. Hidden native
windows are not GPU-less headless. The first read-only UI probe remains recorded:
it incorrectly treated rotating work-delivery lease hash/expiry as immutable
business history. A comparison against the pre-activation PostgreSQL dump showed
only those two fields changed; the corrected probe retains all business fields.

Four qualified Desktop bundle/test files were promoted with exact baseline checks
and backups in `.run/management30-promotion`. Native source remains integration29.
Server source is still separate under `hagency-server/.run/management30-stage`;
`server30-after20.patch` applies to the qualified schema20 source, not the older
main working tree. No commit, push or merge was made.

Next remains post-fence usage reconciliation and settlement. Full 104-operation
parity, the overall EN/CN 9/10 UX gate and mobile validation remain open. The older
request caption still conflates approved admission with pending executor setup;
its state taxonomy should be corrected independently of cleanup observations.

## Historical checkpoint: integration29 (isolated activation complete)

Integration29 is running in all four original localhost role profiles; active SDK
journals, devices and account bindings are preserved. Each normal shutdown was
followed by a consistent SQLite backup. The qualified Desktop binary is
`81ad0bd67467b39ddbb6133c99b6e07cf9c47ad84d3ed25d13e5581a2aa59f6b`.
Server20 activation also passes, preserving all original row counts and history
hashes. Its thirteen new accounting/execution tables were empty at activation.
Only the original localhost fixture was changed; production and mini1 were not.

Final source qualification: store372 ordinary plus35 explicitly supervised offline
cases pass with serial store execution. Native493 ordinary cases pass at two
threads, with68 ignored; the separately run shared-process filter passes52 cases,
including supervised offline children. Enabled/default strict lint passes.
Desktop backend37, host40, execution9 and geometry6 pass, as do budget VM26 and
full-bundle EN/CN2. Server20 passes all336 real-PostgreSQL cases and strict lint.
These overlapping filters must not be added into a unique test count.

Earlier failures remain recorded: bounded store adoption scans under concurrent
load, a historical schema83 test fixture retaining future89/90 tables, and a stale
Desktop manifest allow-list assertion. The serial store rerun changed no source
or production deadline; the native fixture and explicit six-capability assertion
were corrected before the final full reruns. Source, binaries, patches and logs
are frozen in `.run/management29-stage/qualified-checkpoint/`.

Actual EN manager / CN assigned-project-admin top-up acceptance now passes. The
existing approved validation agent receives exactly one4096-token increase:
original24000 plus4096 gives effective28096. Review/cancel creates no request,
the manager has no approval controls, the assigned project administrator approves,
and the original execution device returns one applied receipt. With its mini-app
closed, one new Matrix event executes once under the exact v3 budget head and
returns `BUDGET_READY`. It consumes5183 tokens, bringing complete native usage to
16161 and ledger remaining to11935. The actual native thread and refreshed budget
view were inspected. A normal owner restart preserves the original active SDK
journal/device, all four settled calls and the sole delta without new work/reply.
The original exhausted4096/4813/0 agent remains unchanged.

Evidence: `.run/project29-acceptance/acceptance.json`, with nine reviewed actual
native captures. This is hidden native/Metal, not GPU-less. The message was sent
through authenticated Matrix transport; native chat authoring is not claimed.
The first probe's project-card scroll selector failure remains retained and
created no request. Remaining-on-device telemetry is still unavailable in the
central mini-app;11935 above is verified native-ledger arithmetic. Approval copy
and overall layout still require UX work; no9/10 or full-parity claim is made.

The v3 journal retains original Matrix events, budget blocks, Run revisions and
uncertain start identities. Applied budget heads are append-only; original
reservations and previous usage are not rewritten. The background worker checks
Stop before budget/demand/process work. Offline validation is not actual provider
acceptance. Manual successor review/start remains on the earlier v2 path.

The accepted111 changed source paths have been promoted into the existing Desktop
and client working trees after exact baseline/hash checks and backups. All14 native
packages resolve to the local paired client; locked offline metadata and both
working-tree diff checks pass. Frozen source/evidence remains in
`.run/management29-stage`; server source remains separately staged. No commit,
push or merge has been made.
Full104-operation parity and the overall EN/CN UX gate remain open. Older dated
entries below are historical and do not override this current status.

Updated: 2026-10-17. Branch: `feat/hagency-management-miniapp`.

The full migration remains in progress. Real-provider same-thread recall and new-thread isolation now pass on the separately approved validation agent, with the mini-app closed. Integration28 also passes populated local task migration, editing, comment pagination and normal restart in an independent synthetic fixture. These bounded checks do not establish full original-management parity. No migration branch has been committed, pushed or merged.

Integration28 (`59b57e4cb6f8f932ab4f7377dff5190d699e7a8d6cedc0ace40cb5b02f72c11a`) composes the task-history88 wizard with27b. The combined native27 and Desktop adoption5 checks, native strict lint and release build pass. Actual native EN cancellation and CN confirmation preserve/import three original tasks and two comments, including IDs, parent, statuses, timestamps, labels and historical authors. A subsequent UI edit and one new signed-in comment persist after normal restart; there are still exactly three tasks and three comments, with unchanged archive identity/provenance and no server admission, capacity grant or provider call. The original SDK session/device remains active. Seven native Metal captures were viewed. The test is window-hidden, not GPU-less. Evidence: `.run/onboarding-task-history28/acceptance.json` and `restart28/report.json`. Initial read-only probe assumptions and a comment-pagination selector failure remain recorded; recovery did not resubmit either write. Original rich Board and populated project/engagement migration remain open. The accepted source has now been applied to the two working trees with original-file hash checks and backups (115 paths). The local sibling Cargo dependency is retained; offline locked metadata resolves all14 native packages to it. Both working-tree diff checks pass. This is not a Git commit or release.

Server20 now passes the complete336-test real-PostgreSQL suite and agent-service/backend all-target strict lint. The exact386-file source remained unchanged throughout qualification. The database-clock response read precedes final reauthentication; the expiry test waits on the actual database deadline. Earlier failed runs remain preserved. The isolated checkpoint is `/Users/ychen/home/hagency-server/.run/shared-budget-execution-checkpoint/manifest.json`, SHA256 `2a039e92dbcf5ab7fe16a9063be19b5b9627d2de474ab512f268860b3ab8129f`. Its36-file patch reproduces exact source from central19. No live activation occurred.

Integration29 is in progress in `.run/management29-stage`, separate from the qualified28 main worktrees. It combines native89 accounting, native90 v3 custody, and the human top-up UI. The v3 call writer now uses the exact applied budget head while preserving original assignment/call identity and old overrun; a later head denies fresh issue but permits old issued evidence collection. Definitive central never-started evidence plus native non-issuance can cancel only its exact prepared hold. Durable original startup and restart recovery are implemented. Seventeen focused offline tests pass before the latest migration audit additions; these include real supervised offline children, not real provider requests. Earlier strict native/store lint passed; newly added v3 HTTP/process adapters compile, but paired execution remains unqualified and the full source remains incomplete. The combined full-bundle EN/CN2 and human-budget VM26 checks pass. This is not visual acceptance or operational top-up E2E.

Native90's first complete store run failed279/85 with34 ignored after the new schema was introduced. Diagnosed failures include the archival supported-version bounds and historical migration fixtures retaining future tables. The exact schema90 shapes, retained-history audit, compatibility range and fixture rewinds are corrected, and the complete rerun passes366/0 with34 ignored. Exact store source hashes are unchanged during that run; the old result remains retained. V3 durable-demand reassessment/control, paired HTTP/background integration and actual manager→admin→executor top-up validation remain open.

Integration27b now passes real same-thread recall and new-thread isolation on the existing approved24,000-token validation agent. The executor owner upgraded to `9527996a3bbf7e57404fff48aac94c2343d3d031b90c1f7bb0380fb7424ed9c8` with the exact original account/device binding and a consistent pre-upgrade SQLite backup. With its mini-app closed, two distinct new events consumed3,675 and2,120 tokens, delivered the expected Matrix replies, and retained exact native/server context and input digests. Its cumulative usage is10,978 with13,022 remaining. The earlier cancelled event was not replayed or changed; the original exhausted4096/4813 agent ledger is unchanged. Actual Rinx thread captures for both replies were viewed. This is narrow real-provider continuity acceptance, not full retained-feature parity. Evidence: `.run/project27b-context-transport-resumed/report.json` and `.run/project27b-context-recipient/report.json`. The first post-fix probe stopped after delivery because its cached SDK token rotated; read-only recovery used the current SDK session without resending the event.

Integration27a is qualified (`de447c7ce5fc8317c3a6b501f6743878e17bc91db3e02c3d029f64c413b480ca`). It combines the staged upstream Desktop changes, native context87, retained-heap locale updates and the running Splash root-layout fix. All four role upgrades preserve their exact active SDK journals, original devices and account bindings. Each upgrade first makes a consistent SQLite backup after normal shutdown. Server18 now enables durable intake, demand issuance and thread context on localhost only. A normal recovery read renewed only the original completed work result-ticket lease; all other original history and accounting remain unchanged against the backup. The original exhausted assignment remains manual. The first real multi-turn attempt failed as detailed below; it is not accepted.

Integration27 passed EN/CN script, native locale1, host36, shared-worker18, execution9, geometry6 and catalog8 filters, but actual captures exposed a root-layout regression. The full-parent rebake regression fails on27 and passes on27a. Final27a release checks pass locale/startup6, geometry6 and translation1; the release build succeeds. The unchanged bundle/native checks retain their original receipts instead of being represented as reruns.

Actual27a EN→CN→EN testing now passes three checks in the native Metal window: vertical/full-width layout, both unsaved task fields retained, and cancellation with no task, comment, Start or provider call. Four screenshots were viewed. The authoritative isolated run is `.run/management27a-native-i18n-isolated/report.json`; visual review is `.run/management27a-qualification/visual-review.json`. An expired mini-app lease and a subsequent case-sensitive automation selector error remain recorded separately. Renewing the mini-app lease did not replace the Matrix account or device. This narrow regression acceptance is not the whole-app9/10 UX gate.

The three delegated implementation agents have stopped because of usage limits; their source checkpoints and evidence are preserved and root is continuing. Task-history88 and its bounded native wizard now have combined build and populated native acceptance as recorded above. Budget19/89 accounting is frozen and inactive; native90 now includes call/start/refusal custody and offline checks, but remains an unfinished operational top-up executor. Full retained-feature parity remains unfinished.

The first real context run exposed a protocol mismatch (subsequently fixed and accepted in27b above). A separately requested24,000-token agent was approved through the assigned administrator's native Chinese UI, then automatically armed without another owner approval. With the owner mini-app closed, its first real provider turn completed once, consumed5,183 tokens and delivered `CONTEXT_READY` in the original Matrix thread. The follow-up was cancelled before a provider call; its original event/work records remain preserved and have not been replayed. That initial continuity/isolation attempt failed; the subsequent distinct-event27b acceptance above supersedes this failure without replaying its cancelled event.

The server stores the SHA of the exact outbound Matrix JSON bytes. Native context validation incorrectly required the hash of a sorted-key reconstruction. The actual insertion-order record reproduces the failure in a store regression. The isolated87a amendment verifies both existing encodings of the same closed body/thread content while still rejecting changed body, thread or unknown digest. Ten context store checks, three native offline process checks, three custody checks and enabled/default strict checks pass. The first native attempt lacked its required explicit offline probe setting and remains recorded as a harness failure; the correctly configured run passes. These supervised offline processes are not real provider validation. Paired integration27b passed execution9 and the release build; the subsequent actual continuity/isolation acceptance is recorded above. No schema change or historical digest rewrite is needed. Evidence: `../hagency-client/.run/context-wire87a-stage/`, `.run/project27a-context-agent-request/`, `.run/project27a-context-transport/`. The original exhausted agent remains4096/4813/0; this new agent has18,817 tokens remaining after one settled call.

The frozen task-history wizard now additionally passes all five Desktop Rust tests, including EN/CN preview wording and the long-history footer layout. Its original source receipt is preserved, with additive evidence in `.run/task-history-adoption-wizard-checkpoint/desktop-rust-qualification.json`. Actual populated migration still needs a dedicated isolated fixture and UI acceptance.

The exact26a release passed36 host,18 shared-worker,9 execution,6 geometry and8
catalog tests. English/Chinese VM checks pass acknowledgement18, task writers26,
task reads10, personal settings52, control22 and full-root2. Native qualification
records377 cases:367 inherited from byte-identical26 production plus10 final
Manager cases, including129 supervised offline children. Enabled/default strict
checks pass; offline children are not real provider turns. Source identity and
split evidence are in `.run/management26a-qualification/{desktop,native}.json`.

Actual hidden native/Metal testing passes10 task lifecycle checks, including
12 consecutive saves beyond the previous eight-unacknowledged-result limit,
exact assignee filtering, status changes, comments and explicit deletion. The
single temporary task and comment were deleted through the UI. Three further
checks pass resource local-name review/cancel, save/refresh and restoration.
Published resource configuration, original grants and the two Start intents,
one work and one4813-token provider call remain unchanged. Screenshots were
viewed. This is native/Metal instrumentation, not GPU-less testing.

Evidence: `.run/management26a-native-task-lifecycle-accepted/report.json`, its
linked original/resumed probe records, `.run/management26a-native-resource-name`,
`.run/integration26a-restart-{owner,manager,admin,coordinator}` and
`.run/management26a-qualification/activation-final.json`. Visibility mistakes in
the automation are preserved: results were below expanded filters, then the
probe tried Clear filters while collapsed. Corrected continuation checked the
same task and exact ledger, without repeating creation or provider execution.

Task forms now put their primary action in the first viewport, collapse optional
fields, and automatically acknowledge the exact confirmed terminal result before
navigation. Lost acknowledgement retains the saved result; unknown saves do not
become new requests. Create/edit/detail/delete captures confirm these changes.
Expanded filters still push feedback below the fold. Populated task migration,
full original Board behavior, complete EN/CN theme acceptance and the9/10 UX gate
remain open. A separately staged migration88 is adding exact original task and
comment transfer, including prepare-response-loss recovery before fencing.

Personal-policy editing, explicit Run/Pause and durable Stop/demand recovery are
compiled into26a and source-qualified. Actual personal-agent policy editing and
continuous-provider operation remain unaccepted. A local resource name changes
only display metadata, not the admitted model/capability/limit definition.

Context work remains separate and default-off. Server18 source/release has11
focused cases and28 final affected cases passing plus backend strict checks.
The broader run was107/108 with a retained clock-fixture failure; no full-suite
pass is claimed. Native87 has store329 and focused actual offline-process tests
passing; its final cancellation fixture correction and source freeze are complete.
Native/server context require a paired Desktop build and actual multi-turn
validation. A new top-up19/89 lane is implementing append-only accounting;
approval alone will not count as usable execution credit.

Latest upstream is also staged separately: Desktop52d88b20 includes brand assets
and transport dependency fixes; Serverd685c069 includes login-theme fixes.
Desktop lockfile merge and all14 paired native dependency identities are checked,
and release icons pass. Its unpublished native management APIs still require an
explicit local Cargo override until a qualified client revision is published.
Server composition passes10 asset and16 browser cases plus wasm/backend checks;
frontend strict retains the same133 diagnostics as its unchanged baseline.
Server18 upstream composition has replaced17 locally. Its activation preserved
configuration bytes, original ledger hashes and all original counts; five new
context tables remain empty. Evidence:
`../hagency-server/.run/management-schema18-context-off-activation/activation.json`.
Desktop upstream/context87 integration27 now builds and passes native functional
checks. Its retained-heap locale hook corrects the previously untranslated open
mini-app without replaying startup or losing callbacks. Actual screenshot review
found the separate root-layout regression described above;27a now passes the actual-window regression.
Server19 protocol-only budget accounting is frozen, with operational Server20
and native89/90 still isolated. Approval alone is not usable execution credit.

Earlier failed checks remain retained. Integration26 host expectations required
explicit fresh sign-in after logout, and Manager fixtures had used permanent
drain before attempting another job.26a corrects those tests without weakening
production authority or changing Manager code. The prior Task acknowledgement
regression fails on26 and passes18/18 plus the actual12-save sequence on26a.
Previous debug geometry failures at the unchanged64ms deadline remain recorded;
release geometry passing does not establish unconditional startup performance.

Previous isolated server was schema17
(`149b2d132f977f190c7eabc3e7c13a2a79f12f7db3e719c4f9bb47950987866b`).
The release binary matches351 frozen source files. Qualification is split:
294/295 in the prior full run (one stale feature-list assertion), then all20
final affected PostgreSQL/HTTP tests, the four-policy discovery case, strict
Clippy and backend check passed. The full295 was not repeated. A test-only
host/container timestamp race was corrected using fixture PostgreSQL time;
failed logs remain alongside the corrected runs.

The17 localhost activation preserved the previous binary/config and a PostgreSQL
backup. Original row counts, contributions and execution-history hashes are
unchanged; configuration is byte-identical. Durable intake and demand issuance
remain disabled. The existing assignment was initialized as manual with unknown
migration provenance; both new intake and demand tables are empty. This upgrade
does not start an agent or change its budget. Evidence:
`../hagency-server/.run/management-schema17-demand-off-activation/activation.json`.

Actual attempt3 uses the same original approved Agent, assignment and4096-token
allocation. Review/cancel creates no intent; explicit Start adds one new process
attempt while preserving stopped attempts1/2. Closing the actual mini-app window
leaves that process active for36 seconds and renews its authority. One native
chat mention then produces exactly one work and the expected provider reply.
The existing manager session reads the delivered Matrix event from the exact
agent sender and original thread root; native Rinx displays the response in the
thread. The process then stops as expected for the current one-turn design.
Evidence: `.run/project23-original-start-after-tickets/report.json`,
`.run/project23-agent-work-result/report.json`,
`.run/project23-agent-reply-event/report.json` and
`.run/project23-recipient-ui/report.json`. Captures were viewed.

**Budget limitation observed in the real call:** allocated4096, actual consumed
and retained4813, remaining0, overrun717. Native and central ledgers retain the
full cost; no refund, replacement allocation or top-up was created. Native review
refuses another Start and leaves the original two saved Start intents and one
work unchanged. The reservation is an admission/accounting limit, not a hard
provider token cutoff. Integration24 replaces the ambiguous generic refusal
with exact allocated, consumed, retained, remaining and overrun values; its
actual native UI passed while preserving the original ledger. Top-up and hard
provider-limit semantics remain open. Earlier refusal evidence:
`.run/project23-exhausted-review/report.json`. Two failed probe expectations are
retained there: an unsettled post-open navigation click and expecting a numeric
review when the app instead correctly refused preparation.

The earlier integration22 attempt2 failure remains recorded: background Start
replay/poll replaced its single result ticket during real provider initialization,
so qualification upload returned409 and cleanup recorded stopped. Resume recovered
that same stopped attempt without a duplicate intent/process. The schema15 fix
and actual23 attempt resolve this startup block; prior evidence remains under
`.run/project22-original-start/` and `.run/project22-original-start-recovery/`.

The first owner restart probe hit unsettled sidebar coordinates and opened
Contacts; the corrected layout-stability probe opened the separate mini-app
without another login or resource recreation. Both failed and recovered receipts
are retained under `.run/integration22-restart-owner*`.

Previous isolated release was integration21f
(`863809bc52865a2aa3aca5e3a388be14639f30deca63d55c5c26540911557164`).
Administrator, project manager and resource owner now pass real native launch
and normal restart with their exact active session journals, device generations
and account bindings unchanged. The coordinator also passes same-account
recovery with its original binding preserved. Those four fixtures subsequently
passed the integration22 session-preservation checks described above.
Earlier owner workflow acceptance used
integration21e, a dev-profile functional instrumentation build
(`4ff30191e0da31a14bf6049e61e0db311e2cee46b6c6dbf84ba6ac0965a7f917`).
Same-account OAuth recovery (two checks) and normal close/reopen (three checks)
retain the original adopted installation and resources. The one-shot sign-in
correction passes14 focused native regressions,60 login tests, strict native
Clippy and27 exact Desktop backend tests. The subsequent outer stale-session
correction now passes actual reconnect: the exact saved Verify command records
one receipt, and both original resources become connection verified (two checks).
No admission or resource was recreated.

Native capacity review and cancel pass two checks with zero reservations.
Continuing that same saved intent now passes three checks: the original resource
reserves exactly100000 tokens once, and separate publication returns Active with
the reviewed coordinator, eligible manager and expiry. Canonical reservation
count is one. Captures were viewed. The first publication probe is retained as
a failed harness attempt: it incorrectly expected blank choices instead of the
correctly restored saved review. No app bypass or replacement grant was used.
Evidence: `.run/onboarding21d-reauthenticate2/`, `.run/onboarding21d-restart/`,
`.run/onboarding21e-reconnect/`, `.run/onboarding21e-grant-review/`,
`.run/onboarding21e-grant/` and `.run/onboarding21e-grant2/`.

The same original grant now supports a real approved project and native project
reservation: the manager requests54321 tokens, the server administrator approves
the project, and the coordinator authorizes the reservation within the original
100000 grant. The server administrator assigns the existing coordinator account
as this project's administrator. Review/cancel, actual reservation, assignment
and manager readiness pass four native checks. No second resource-owner approval
or replacement grant was used. Evidence: `.run/project21f-original-grant/` and
`.run/project21f-original-allocation3/`.

The manager created one private Matrix Space and room. The assigned administrator
accepted both original invitations through native Rinx controls. Continue then
joined the configured service and verified those exact original rooms, preserving
the creation journal. The paired schema13 service-join server correction passes
237 PostgreSQL/HTTP tests and strict Clippy, and was first activated only on localhost8088
(binary `4fce5f10709fe508f985f2092b2a81f1499f7d6554ccb046fd2b8bb82e2a73f6`).
Evidence: `.run/project21f-matrix-created/`,
`.run/project21f-native-invite-acceptance3/` and
`.run/project21f-matrix-confirm/`. The verified-project capture was viewed.

The manager then requested one text-only4096-token agent, and its assigned
project administrator approved that same request. Three actual checks pass;
approval/status captures were viewed. Agent reservation, Matrix provisioning and
shared execution were enabled only for the isolated test server. The original
agent received a native reservation, joined the original room and qualified an
actual provider process. However, that process stopped and was permanently fenced
after approximately30 seconds idle, before any prompt was submitted. The guarded
send probe refused to send; work count is zero. Native regression subsequently
reproduced a stale local Session during a delayed HTTP reply across the initial
deadline. A separate completion regression exposed premature process/pipe closure
before pending Matrix reply reconciliation. Both corrections are frozen and
qualified with12 actual offline-child native tests,60 authority tests, ordinary
owned-session lifecycle1/1 and codec9/9; strict lint passes. Central exact-schema13
and paired-schema14 renewal tests pass without a server production change. These
regressions establish concrete defects, not a forensic proof of the original
live stop. Integration22 subsequently exposed the independent result-ticket race.
Both startup blocks are now resolved for the actual integration23 one-turn
acceptance documented above; continuous chat remains separate.
Evidence: `.run/project21f-agent-request2/`,
`.run/project21f-agent-work-send/`, and Server
`.run/management-shared-v1-activation21f/`. No complete execution E2E pass is claimed.

Substantial original management parity remains: provider/resource creation and
amendment, publication/withdrawal and usage totals; full agent start/stop/resume,
tools/workspace policy and token top-ups; tasks/boards/graphs and durable alerts,
notifications and reminders. Stopped-process successor custody has separately
tested schema14/server and schema82/native checkpoints; explicit Desktop start
and retained-worker integration are now live in integration22 on localhost.
Their first actual attempt exposed the result-ticket race; schema15 plus
integration23 now pass the actual same-assignment successor and one reply.
The separately qualified UI overlays now include all six capability controls,
individually selectable saved setup commands, and resource local-name editing
with revision checks and durable original-command recovery. Naming does not
amend published model/capability/limit definitions. Its store tests pass266 with
19 explicitly ignored; native pixels remain unverified. Personal settings also
remain a separate overlay. Full role/fault/restart acceptance and
EN/CN light/dark9/10 visual acceptance remain open. There is no reliable overall
completion percentage or finish estimate.

Integration22 composition is complete: immutable21f plus six Desktop overlays,
native schema81/82/83 and the separately qualified caller/discovery corrections.
The combined schema14 server includes the service-join fix and passes249 real
PostgreSQL/HTTP tests and strict Clippy. It is active only on localhost8088,
binary`12324b8c1b73c360ae2c186e8e80ef4f72c81c097c7e24ce43854018773567c9`.
Successor issuance was subsequently enabled for this isolated E2E. All four
Desktop22 role sessions retain their original device generations and account
bindings. Read-only receipts under`.run/original22-before/`,
`.run/original22-after-schema14/` and
`.run/original22-after-desktop-restarts/` preserve the intermediate checks.

**Continuous chat remains a substantive gap.** Each current shared process
handles one text turn, stays owned until the reply is sent/blocked, then is
reaped. Another turn needs a new explicit start intent under the same approved
assignment. This is not another agent approval, but it is not normal continuous
chat. Messages arriving while stopped/offline are not durably queued for a later
attempt. Full parity requires assignment-scoped durable intake and deduplicated
work-triggered execution, with current budget/role/device checks and no additional
human approval for every turn. A successful one-turn recovery test cannot close
that requirement.

The existing manager profile now passes same-account21e recovery. Administrator
OAuth also succeeds and creates an active resume journal, but reopening the
mini app exposes a startup failure: the full-size Splash slot contains the exact
229550-byte shipped source and zero script children. Its native log records
`script time budget exceeded`; maximizing does not recover it. This dev-profile
failure is retained at `.run/management-role21e-admin3/` and
`.run/admin21e-blank-investigation/`. Immutable21f built in release mode in6m49s
under the unchanged64ms isolate-entry budget. It composes separately tested
v1 process custody, the typed service-join Confirm client, execution observation
and state-specific copy. Actual administrator, manager and owner roots render;
their captures were viewed. Evidence: `.run/management-role21f-admin/`,
`.run/management-role21f-manager/` and `.run/management-role21f-owner/`.
A separate native visible-error/retry correction now passes three actual-widget
tests, including a real script budget exhaustion and repeated reopen; it is not
yet integrated or pixel-accepted. A separate invitation-detail correction passes
three widget tests for updates to the selected room without losing pending
actions. Agent capability choices also exposed a repeated widget ID: only the
last capability rendered. Its separate patch passes VM negative/positive tests
and two actual-widget checks. Those startup, invitation and capability corrections are now included in
the live22 binary; personal settings remains separate. Coordinator recovery evidence is
`.run/management-role21f-coordinator-reauth2/`.

Earlier attempts:
Integration20 built successfully (SHA256
`5d52afab50da9caa782a963ad55a8f83b159c5d1c229009aca17a5029ecd20bb`)
and replaced only the isolated owner fixture on port8321. It contains the narrow
adopted-resource reconnect and fenced-provider-directory fixes. Native regression
covers original-ID resume, a lost reply, separate verification and unchanged
approval. Actual reconnect has not passed yet: launching the newer mini-app
revealed a blank page. A missing brace in the allocation button swallowed the
remaining root widgets. The complete shipped-bundle test reproduces this with
zero root children and no interpreter error; the corrected EN/CN bundles pass.
Integration20b freezes that minimal fix and is running only on8321 (binary
`b9da636eee59560fe489bdc6697bd0a1eac7c80e59375c53a250d41df04c593a`).
Its actual mini-app root renders again. Actual reconnect still failed: the
provider observation could outlive the initially captured short authorization,
and verification reused that retired session after obtaining a fresh one.
The original resource/provider configuration is unchanged. An isolated test
reproduces the failure and passes with the fresh same-context session; actual
reconnect awaits the next integrated build. Evidence:
`.run/onboarding20-launch/`, `.run/blank-root-checkpoint/`, and
`.run/miniapp-blank-investigation/`, and `.run/onboarding20b-reconnect/`.
The two original resources are unchanged;
no capacity grant was created. Earlier failed grant evidence remains at
`.run/onboarding18-grant2/`.

The Desktop native worker now polls original agent reservation/fencing, Matrix
readiness and text execution with independent cursors. Its20 backend tests pass
and the checkpoint is `.run/shared-worker-checkpoint-2/`. The exact immutable20
plus native bridge passes9 execution tests and the original three-process
handoff. This is still default-off and has not performed an actual shared model
turn or Matrix reply. A separate restart blocker is being repaired: ordinary
close previously revoked native enrollment, rotating device generation on the
next start and fencing prior grants. New private original-session recovery and
suspend-versus-explicit-logout APIs pass434 native tests (26 explicit ignored),
58 focused login regressions, strict native Clippy and25 Desktop backend tests.
The Desktop checkpoint is `.run/desktop-session-lifecycle-checkpoint/`.
These are under immutable21 composition; no live activation or full restart
acceptance is claimed. Stopped-process successor custody remains
a separate missing protocol.

The central dispatcher stack-overflow fix now passes the full231 PostgreSQL/HTTP
tests and strict all-target Clippy at the default test-thread stack size. It
splits the same route arms into bounded boxed dispatch groups; no stack-size
increase or route contract change is used. The release artifact is frozen at
Server `.run/management-schema13-231-checkpoint/`, SHA256
`7a1a1986425d0b4f1c5fb23b291f6af315298670050aa9ce606d130c5013d5c9`.
The additive current-device proof endpoint then passed233 PostgreSQL/HTTP tests,
strict Clippy and release build. Its frozen binary
`4f3f6edf1cfe6ee1ad6a8a30433114a0290adf8c6a9a416c64c17f325910e85d`
is now active only on localhost8088, at schema13. Existing configuration, row
counts and original contribution identities/revisions/generations are preserved.
Shared execution and agent executor/Matrix/runtime provisioning remain disabled.
Evidence: Server `.run/management-schema13-resume-activation/` and
`.run/stack-overflow-investigation/full-after.log` and `clippy-after.log`.

Exact integration21 composes shared execution, cleanup, lifecycle/session resume
and the fresh-session reconnect correction. Integration21b adds one missing
Desktop lockfile dependency edge only. The first full native run passed427
with one heartbeat timing failure and26 ignored; the same binary's focused
heartbeat test then passed, and its four-thread full rerun passed428/428
(26 ignored). No lease duration or assertion was weakened. Integration21c adds
same-account recovery from the native pane and mini app. Its exact Desktop
backend suite passes25/25 and its release build is frozen at
`.run/management-integration-build-21c/`, SHA256
`76dd3a40559a6b6478d57357424b09d258e689856e87b5de677d3e3d450d3018`.
It was not activated: a new regression proved an earlier explicit sign-in flag
can be reused after a later authenticated remote revocation in the same
service. The one-shot native/desktop correction is the separately qualified21d
overlay described above. No capacity publication or shared execution is accepted.
Pre-resume installations must use same-account reauthentication with their
existing OAuth client/device, not full logout followed by new OAuth registration.

Full parity still requires the actual resource→grant→project→agent→provider work
→Matrix reply flow, remaining management actions (top-ups/removal/tools/
workspaces/tasks/settings) and fault/role/visual acceptance. The
[feature-family matrix](hagency-management-feature-matrix.md) covers all104
original native operations exactly once. Their acceptance records remain
pending; newer flow tests are not a completion percentage.

Integration18 now passes the actual isolated original-resource migration
(three checks) and subsequent process-restart acceptance (three checks).
Binary SHA256:
`2f7fac9e391f8498cf135ad7c16de333dce117953374a6ed7b666957890796f2`.
Both original resources retain their full IDs and 123456 monthly ceilings;
the owner explicitly confirmed one 400000 shared monthly account ceiling.
The migrated native inventory is readable before and after restart, and no
second migration confirmation is offered. Actual provider identity verification
completed. Evidence: `.run/onboarding18-migration/` and
`.run/onboarding18-restart3/`. Earlier probe failures are retained: a list-ID
assertion incorrectly expected full IDs instead of abbreviations, and the
existing selector needed to scroll an offscreen detail button into layout.
An overlapping grant probe was stopped before its capacity-review capture or
confirmation; grant testing is now strictly sequential. Capacity reservation
and publication acceptance remain pending. No real shared model turn occurred.

Earlier retest (2026-10-16): integration17 built successfully and ran only in
the same isolated owner fixture on port8321. Binary SHA256:
`684e3bc031daa026cf0cd10ac0f4fcaf9528d8257869bf727013087c8e9adc02`.
Its real native review now renders both original resource identities, the
explicit 400000 monthly ceiling and the confirmation button without maximizing
the window. The capture was viewed. Migration itself still fails with the safe
code `owner_authorization_required`; no success is claimed. Two native provider
identity callers incorrectly supplied the complete cookie header where the
authentication API expects its raw value. A loopback regression reproduces that
refusal and verifies the corrected calls reach the next provider gate. The
minimal fix is being prepared for integration18; test resources remain intact.
Evidence: `.run/onboarding17-migration/`.

The server fairness checkpoint passes 222/222 PostgreSQL/HTTP tests, strict
Clippy and a backend build. Its frozen binary SHA256 is
`bbd4573cc50ecc73d8d4e2791326f4fb3ffad442ccac88481dc212521c81d98d`;
root independently verified the artifact. It has not replaced the schema11
live isolated server. The retained native allocation worker and sealed
original-device access are now implemented in source and under test. Its
cancelled shutdown waiter retains the original task handle; ledger operations
reuse the existing owner grant and reservation receipts. This source work is
excluded from integration17/18 and is not yet an actual allocation E2E pass.
The other active work is the coordinator allocation UI and real native text
execution bridge. Full management parity remains open.

Current parallel work (2026-10-16): the original-resource onboarding/native
confirmation checkpoint is frozen as integration16 and built in 6m30s from
immutable Client/Desktop source. Its binary is
`c1b5099c90225993b8dc4a8fc78f5af1f9714f3639c5cbb2aae0b607a3c63840`.
The frontend bundle is
`082377fca9dc656e7b36394b8b3d2e84d834503dd87771020c5d36cfb290a7f3`.
Its 17 frontend and 15 native checkpoint source hashes match the snapshot.
The matching native checkpoint passes three confirmation/restart tests, two
stale/expired review tests, 14 Desktop backend lifetime/revocation tests and
three grant tests. The frozen onboarding VM passes 24 checks. Visible adoption,
native grant publication and restart acceptance remain pending; these source
tests do not establish UI E2E. Root created a fresh SDK test profile under
`.run/onboarding-installation16` (port8321), with an explicit isolated
installation root and no copied running database. Normal OAuth login and native
provider consent completed. Two original resources were enrolled, approved by
the existing test administrator and connection-verified using real UI controls.
The exact request card is selected by its name; the existing unrelated pending
request remains untouched. Evidence: `.run/onboarding16-original-resources/`.
The initial native review/cancel probe passes four checks with zero script
errors (`.run/onboarding16-native-review/`).

Native acceptance then exposed a real defect: Choose→Loading→Review updates
widget state but does not redraw the separate window. The confirm button and
exact review remain invisible until an actual window maximize. With that
diagnostic workaround explicitly recorded, one confirmation was attempted;
it returned an unconfirmed result, with no canonical registry/descriptor seen.
No migration success or grant publication is claimed. Source resources and
private test history are retained. UI and native agents are investigating
separately; `.run/onboarding16-migration/` retains failure evidence. The four
integration15 binaries and schema11 server/configuration remain unchanged;
only the two authorized test admissions were added through UI.

All three agents are active on the next checkpoint: the retained native
text-execution bridge, server work/usage/Matrix reply custody, and read-only
shared work history. A read-only history index is being added so previous work
remains reachable after restart when new agent admission is disabled. The
server's schema13 artifact is now frozen after 221/221 PostgreSQL/HTTP tests,
30 focused protocol tests and strict lint. Its SHA256 is
`6788d459532f854e5af242fd3c4165c43efb8cada175d8a796f5e1371d5b3060`.
The first renewal401 failure is retained. A confirmed host/database clock-domain
comparison defect was corrected; the exact clock offset of that first failure
was not captured, so its cause is not claimed conclusively. No schema13
activation or actual model turn has occurred. Fair multi-agent runtime polling
is being fixed in a separate checkpoint.

Previous parallel checkpoint: root owns integration and the isolated live
fixtures. The completed status and
layout checkpoints are isolated in `.run/management-integration-source-15/`;
the exact combined source passes 102 EN/CN script checks and four native HTTP
status tests. Release 15 built in 6m14s from the stable
`.run/management-integration-buildtree/` path. All four isolated account fixtures
now run binary `65e17982fc057139df3c01c1a36ed08607430edb7b963655b0d8c0582cde0802`,
bundle `e65a2cc4bbc4476ee4c77e77a3eaa8acf4116739a036f4329d00a3532ac43d5f`.
Nine native role/prerequisite checks and three resource-detail/navigation checks
pass, with zero script errors and unchanged central counts. All nine captures
were viewed. Resource selection now checks the full native ID even when model
and budget are identical. Evidence is in `.run/management-agent-readiness-native15/`
and `.run/management-resource-detail-native15-en/`. The unchanged scorer reports
resource screens 9.52–9.67; coordinator Chinese readiness remains 7.87 and is
being improved in the next UI checkpoint. Native status pixels still need a
real admitted request. No current wizard or unfinished execution changes are
included in this build; no owner source was migrated. The server agent separately
reports schema12 process/activity tests 205/205 and strict lint passing, with a
frozen artifact retained but not activated on the live isolated server.

Current integration checkpoint (2026-10-16): the isolated localhost server runs
schema 11 and binary
`e287770bb13b137f9da51335d4c3245bdb21b9618bd18461bb160ac2b5cbf9b3`.
Its 191 PostgreSQL/HTTP tests and strict lint pass. The binary upgrade preserved
all eight checked workflow/domain counts and configuration. Subsequently, only
project reservations and agent admission were enabled alongside project setup
for native readiness testing. Executor, Matrix provisioning, runtime and parent
settlement remain off. The private configuration/database backups and activation
records are in `hagency-server/.run/management-agent-{admission-upgrade,readiness-enable}/`.

Previous integration baseline: Desktop release 14 built in 11m01s from the immutable source in
`.run/management-integration-source-14/`. Its binary is
`4c379fd97aaef8effe461567c02a01226e22e1270b1e82211682a3927a127eb0`,
with bundle `ec32e8d63c4be8675f0f1836fd0234e15cc487638468bef63424c7caee7c74cc`.
At that checkpoint the manager, administrator and coordinator fixtures ran it;
the owner remained on release 13. The exact frozen client source passes all four admission HTTP tests.

Real Makepad/Metal native checks pass: two feature-disabled checks and three
readiness/navigation checks for each of the manager, administrator and coordinator.
All six central state counts stay unchanged; startup and script error counts are
zero. All seven captures were viewed. Reports are in
`.run/management-agent-hidden-native14/` and
`.run/management-agent-readiness-native14c/`. These verify missing prerequisites,
role restrictions, refresh and return navigation; they do not submit or approve
an agent or run work. The first two probe failures are retained: read-only
coordinator has no onboarding draft, and its extra connection section puts the
Project agents button below the viewport. The amended probe preserves missing
versus existing drafts and scrolls to the actual button. Immutable source14 was
not changed; the exact probe patches/hashes are retained in
`.run/agent-readiness-native14-probe-amendment/`.

The unchanged visual scorer reports agent-readiness English 9.13/9.00, Chinese
dark 7.87, and return-project 9.67/9.61/9.17. Overall 9/10 acceptance remains open.
A separate post14 layout checkpoint adds visible resource identity and clearer
setup/connection structure; its English resource native captures now pass in
release15 as recorded above. Chinese resource and setup layout acceptance remains
pending. The stale blanket project-unavailability sentence identified during
visual review is fixed and verified in the release15 project captures.

Release 14 adds typed agent requests, assigned-project-administrator decisions
and manager cancellation. Approval is shown as waiting for executor setup.
Its cached management authorization rechecks the current SDK session before use;
a committed command remains recoverable under fresh authority without a duplicate
POST. Positive request/decision native UI acceptance is still outstanding.

Adopted-resource management backing now passes its isolated migration/reopen,
profile-filtering and original private-journal tests. The trusted Desktop handoff
from migration to native grant consent/publication is still being connected.
The actual selected shared Codex provider now passes three consecutive metadata
qualification runs under the existing observed-supervisor contract: stable
account, verified restricted policy, exact process cleanup and unchanged provider
selection/auth/configuration bytes. An overly short new per-RPC timeout was
corrected to use the remaining original bounded lease. Evidence is in
`hagency-client/.run/managed-runtime-selected-provider-final/`. Fifteen focused transport tests now pass, including separate post-start expiry
and startup-expiry cases; eight discovery and thirteen store tests also pass. Neither metadata qualification nor the
fixture-process tests establish real agent work.
No real owner profile has been migrated, and the complete resource → project →
agent → work workflow remains unverified. The detailed checkpoints below retain
earlier evidence and failures; they are not all descriptions of the current build.

Latest integrated project/setup checkpoint: four isolated native role fixtures
ran release 12,
`887ec51d6017d349a3d488207a9197e06b4757185657093d98acc353447f5c39`,
with bundle `23be7cd74789a9c57521580a463a3af7b66c9bd2c51f416a371555e88d8210d8`.
The real project-flow and setup probes each pass six checks with zero script
errors (`.run/management-integration-project-e2e-12b/` and
`.run/management-project-setup-e2e-12/`). All eleven full-window captures were
viewed. Setup tests now assert the rendered room-creation status in both
languages, including after a manager restart. The earlier missing-status defect
is fixed, with a failing release-7 regression and a reduced actual script-VM
reproduction retained. The tracked `tools/hagency-script-review` helper also
passes twelve checks of the actual setup subtree and recovery field identities.

UI acceptance is still open. The unchanged scorer reports project English
9.37–9.67, Chinese project 9.17, and contextual Chinese connection 8.93. Setup
English scores 9.48–9.52 and Chinese dark mode 8.53. Release 9 fixes the partly
clipped administrator confirmation row. The stronger native check now requires
its full height inside the viewport; the real capture was also reviewed.
Scores do not replace visual review. Release-7's original five captures, 8.38 Chinese
score and missing-status evidence remain retained. Exact base commits, changed
source hashes, source overlays and the final enrollment-expiry amendment for
release 9 are retained in `.run/management-integration-source-9/`. Release 12's
exact immutable source is in `.run/management-integration-source-12/`.

A separate real UI logout probe found a new defect: the Account page skipped
the logout action when optional cached profile metadata was absent, despite an
authenticated SDK session. The failing evidence is retained in
`.run/management-logout-e2e-1/`. Release 10 built from the immutable release-9
source plus the narrow fix: logout no longer depends on that metadata, and
account display can use the authenticated SDK user ID. Its binary is
`67c6dc79d47b4758388ca42923c83d735e040d6f937e0409ed773c66ccced0f8`.
Only the isolated manager fixture was updated. All three real UI actions pass:
cancel preserves a resource read; logout closes the mini-app window; fresh
browser/SDK sign-in reopens management and reads resources. Three captures were
viewed. The overall test remains red because expected account-shutdown task
cancellations were logged as bugs. Evidence is in `.run/management-logout-e2e-2/`.
Release 11 adds account-epoch-aware cancellation handling, preserving unexpected
cancellation and panic errors. Its binary is
`def14a5053162b8f3c87f2b9c743dc24aba1f5759330c93eea69e3780dbf76be`.
The unchanged strict error gate now passes: three real UI checks, zero native
errors, and all three full-window captures viewed in
`.run/management-logout-e2e-3/`. The resource check waits for the asynchronous
read to complete; static filters or an updating/error notice cannot satisfy it.
This verifies the isolated manager's logout and fresh login, not resource
migration or shared agent execution.

Release 12 built from an immutable snapshot with a narrow layout update:
project requirements now show their blocking reason beside the relevant fields,
and contextual connection details use the same field alignment as other details.
Its bundle digest is
`23be7cd74789a9c57521580a463a3af7b66c9bd2c51f416a371555e88d8210d8`.
Twelve actual script-VM subtree checks and the translation audit pass. Its native
captures and unchanged scores are recorded above. The first project run exposed
a probe timing race: the SDK identity appeared while initial management reads
were still updating, and navigation correctly did not proceed. The failure is
retained in `.run/management-integration-project-e2e-12/`. The probe now waits for
the visible updating state to clear before navigation, including after restart;
it does not use a longer blind sleep or relax functional assertions.

The previous release 6 connection probe passes three checks with zero script errors
(`.run/management-connection-detail-e2e-3/`), and the project regression passes
all six checks with zero script errors (`.run/management-integration-project-e2e-6/`).
All nine full-window captures were viewed. English/light connection scores 9.60,
Chinese/dark direct connection 9.00, and the contextual connection 8.93. The
visual threshold is still not met; earlier lower-scoring revisions are retained.
These probes use actual hidden Makepad/Metal windows, not a GPU-less headless renderer.

This release fixes immediate account retirement, same-account SDK replacement,
cancelled/concurrent shutdown cleanup, and retired mini-app manager caches.
Eight real-native lifecycle tests, three client-slot tests and seven host tests
pass. The cache fixture proves that dropping the retired manager releases the
profile lock and permits fresh authorization; the old manager stays denied.
These are isolated native/loopback regressions, not an actual logout UI E2E.

The earlier central schema 8 checkpoint passed 151 PostgreSQL/HTTP tests and strict all-target Clippy;
its frozen binary is `86a131047154fc4c9e9896fc9fc13ca54499b52340033e60ab156e425550ca62`.
It adds assigned-project-administrator agent admission with durable commands and
immutable execution intents. Those intents cannot yet be delivered to an
executor or start work. Native child reservation accounting now has a separately
tested, unwired implementation; no transport activates it. Mini1 remains untouched.

The earlier paired desktop/server setup checkpoint had the following evidence:
156 PostgreSQL/HTTP tests and strict server Clippy pass; its server binary is
`20ea7a97a341c641bd52d985f94e6c395ad5467516905dba18a033fdd263f471`.
Readiness supplies the exact eligible reservation and begin pins its ID as well
as its revision. This closes a race where replacement reservations had equal
per-record revisions. Old omitted-ID commands may recover recorded receipts;
they cannot silently create a new setup. The matching native adapter passes
24 tests; its nine typed host services pass 12 host tests and one capability
policy test. The running mini-app bundle, version 0.3.0 and digest
`a83bb9851e483314ba984354fe1680a9dd2cc76bd19de05b6c30ae190a3a4ebc`,
ships the eight used setup grants. Parser/helper checks, catalog tests (8/8),
host tests (12/12), and the translation audit pass. Release 8 built successfully;
the role probes above pass their functional assertions but UI acceptance remains open.

The isolated localhost server previously ran that paired schema 8 binary. Its pre-upgrade
database/configuration/binary backup and before/after counts are retained in
`hagency-server/.run/management-schema8-upgrade/`; all eight checked workflow/domain
table counts were preserved. Only project setup is enabled for readiness and
administrator-assignment testing. Project reservations, parent settlement and
agent admission remain disabled. No real Matrix setup, capacity reservation or
shared provider execution has yet passed the joint native workflow.

The source census passes 15 native tests, 11 domain audit tests and 8 personal
audit tests, with strict native Clippy. It observes existing sources without
normal ledger initialization, respects root locks and preserves source evidence.
This is inspection only: adoption, old-writer fencing and the canonical-resource
adapter are still implementation work. Release 8 includes the extracted
profile-directory helper, so Matrix/configuration journals no longer initialize
the personal agent ledger merely to find their directory.

Release 8 includes schema 76 managed call/provision custody, exact reviewed grant
periods and consent-deadline checks, archive-journal/authority extraction and the
cancellation-safe management writer drain. Its frozen store source passes all
181 library tests. The managed custody types have no production execution
constructors; these additions do not activate resource migration, allocation or
shared agent execution.

The writer drain retains the original shutdown acknowledgement across cancelled
and concurrent callers. A failed or closed attempt cannot become success because
its channel later closes. New management initialization checks retirement before
and after blocking filesystem work. Three real-domain tests, nineteen shared
Matrix regressions and strict native all-target Clippy passed at that checkpoint.
This acknowledges only the domain repository's drop; migration still needs
provider/runtime acknowledgements and physical source-lock reacquisition.

Newer source work is separate from release 8: native grant-intent recovery,
retained provider/runtime shutdown, installation authority leases and central
schema 9 agent-executor delivery. The completed checkpoints are described below. The
installation lease module alone passed seven tests, including actual separate
process exclusion, and strict store Clippy. Its first NativeManagement wiring
covers enrollment, configuration and publication only; personal/CLI legacy
writers remain uncovered. No source fence, import, destination activation or
end-to-end shared-agent result is claimed.

## Server integration and pending source checkpoint

The localhost fixture previously ran the frozen schema 9 server
`56e192a43e4a0dbc515a0af1e6fbc89e3b06e7da0de5ff6416855fcce4f9f0f4`.
All 169 PostgreSQL/HTTP tests, strict service all-target Clippy and the backend
build pass. The upgrade retained counts in all eight checked workflow/domain
tables; the previous binary/configuration and database dump are under
`hagency-server/.run/management-schema9-upgrade/`. Only setup readiness and
administrator assignment remain enabled. Executor delivery, agent admission,
project reservations and parent settlement remain off in this runtime.

The isolated localhost fixture now runs schema 11, binary
`0f7b2f6fab63fd8585fb672d9372751412b39a4c7b1930f0a99397b0209d50f5`.
The upgrade preserved all eight checked workflow/domain table counts. Its
private database/configuration backup, previous binary and activation evidence
are in `hagency-server/.run/management-schema11-upgrade/`. Only project setup is
enabled; reservation, settlement, admission, executor, Matrix identity and
runtime effects remain disabled. Mini1 and production were not changed.
The existing release-12 admin, manager and coordinator windows then passed
three read-only compatibility checks against schema 11 with zero native errors.
Assignment revision and all checked executable-state counts stayed unchanged;
all three captures were viewed. Evidence is retained in
`.run/management-schema11-native-compatibility/`.

Schema 9 adds exact agent reserve/fence delivery and receipt recovery, permanent
manager-owned suspended identities, pending assignments, transactional cleanup
obligations and isolation from personal-agent entry points. It does not provision
a Matrix identity, start a provider or run a chat turn. The paired native
reserve/fence adapter passes twenty transport tests (eight new managed-agent
cases and twelve existing allocation cases). Its canonical store checkpoint
passes all 201 library tests and strict Clippy. Schema 77 keeps permanent agent
fence records so a delayed reserve cannot recreate a stopped agent. These tests
use isolated stores and loopback HTTP; the adapter is not enabled in the
running workflow.

Newer native source has four passing real SQLite queue tests covering
installation leases across enrollment/configuration/publication, twelve passing
existing management regressions, and strict store Clippy. Its lease guard is
retained by the queued command through commit or dequeue; session retirement
still applies independently. This covers those three paths only. The grant
intent journal checkpoint passes seventeen native cases and strict native
Clippy, including restart, lost journal, canonical recovery and queued retirement.
The initial trusted enrollment factory now passes nine cases, with seventeen
grant-journal cases: 26 total, plus strict native/store Clippy. A genuine failing
regression caught a proof expiring during the final SDK await. The corrected
factory completes its awaits before constructing the finite provider-bound
proof; the regression now passes. Tests use a synthetic SDK/provider, real
loopback HTTP and isolated SQLite. Production grant consent UI and source
adoption remain required before activation.

A native screenshot proved the setup confirmation buttons were clipped to
22 pixels despite appearing within the window bounds. The tightened probe
requires full button height, retaining both the original inadequate assertion
and the real failing check in `.run/management-setup-confirmation-regression-{1,2}/`.
The source now puts confirmation immediately after the project heading and
resets scroll on entering/leaving that review. Parser checks, twelve actual
setup-subtree checks, package/catalog validation and the translation audit pass.
Bundle `2f0c6dc53b6288642050f8cd11a0ad2e85da1a04fbb546e9be5dfb1cc6717d2f`
is included in release 9; the stronger real native regression passes. Chinese
visual acceptance remains pending.

## Implemented foundation

Additional source checkpoints (not included in the release 9–12 desktop):

- Central schema 10 freezes Matrix membership separately from provider runtime
  qualification. All 176 PostgreSQL/HTTP tests, strict service Clippy and the
  backend build pass. Native store checks pass 204 tests, including unchanged
  main/WAL evidence; four Matrix transport tests pass. The frozen server binary
  is `17e3b3f81d67d331bddfd5eb264fac5e8cd7284f71d8413a4059059e1c42011f`.
  Tests use loopback Matrix HTTP, not a deployed homeserver. Agents remain
  suspended and cannot run work.
- Central schema 11 adds exact provision/stop commands with immutable bindings,
  finite leases and observation-only recovery. Its 186 PostgreSQL/HTTP tests,
  strict service Clippy and locked backend build pass. The frozen artifact is
  `0f7b2f6fab63fd8585fb672d9372751412b39a4c7b1930f0a99397b0209d50f5`
  in `hagency-server/.run/management-schema11-build/`, with 89 source hashes.
  It is active only in the isolated localhost fixture with effects disabled as
  listed above. Native retained-process qualification and cleanup remain in
  progress; these server tests do not prove a runnable agent.
- Legacy normal domain/personal openers now retain an installation guard for
  their entire database lifetime, including initialization and shutdown. The
  source checkpoint is `hagency-client/.run/legacy-writer-lifetime-checkpoint.json`.
  Normal-opener/subprocess checks pass 6/6; personal tests pass 86 with three
  ignored; the bounded-concurrency full store run passes 204. The broad native
  initial default-concurrency run had 356 passed, four failed and five ignored.
  The failure logs remain retained. Two HTTP-renewal fixtures now explicitly
  hold the original response rather than racing short sleep/deadline windows;
  the 17-registration capacity fixture permits its real SQLite setup to finish
  without expiring earlier registrations. Production limits are unchanged.
  A frozen native test executable now passes 371 tests with five ignored at
  four test threads, including all four formerly failing tests, in
  `hagency-client/.run/native-concurrency-regression-1/`. This is a bounded
  concurrency pass, not a new default-concurrency result. Combined native
  library/tests strict Clippy also passes.
  Only macOS has runtime adoption qualification; cross-compilation does not
  qualify another platform. A subsequent isolated physical migration checkpoint
  now passes: two original resources retain their IDs on their original shared
  Seat, with a single 700-token account headroom and no new commitments. It
  backs up original main/WAL bytes, fences old writers, restarts before canonical
  acknowledgement, and recovers the exact migration through the SDK-authorized
  recovery-only opener. Missing migration IDs and cross-profile reads are denied.
  Two physical/subprocess checks, 14 store-phase tests and six lock/refusal tests
  pass; strict store/platform all-target Clippy passes. Source hashes and logs
  are in `hagency-client/.run/native-resource-adoption-physical-checkpoint.json`.
  No real owner profile was migrated. The subsequent adopted-management
  checkpoint now supplies canonical profile-filtered Resources and the original
  private journal. The trusted Desktop handoff to consent/publication remains
  in progress; backing alone is not yet a usable end-to-end grant flow.
- Native grant publication retains the original private command and CAS before
  HTTP, with separate review/confirmation and observation-only recovery after
  an unknown outcome. The current checkpoint passes 11 native publication tests,
  24 allocation transport tests and one archive-authorizer test; earlier broader
  owner-grant/enrollment coverage passes 40 tests. Pinned Makepad layout tests
  check complete footer bounds at phone
  and desktop sizes; these are not rendered screenshot acceptance. The same final
  Desktop test executable passes 15 grant tests, two host-cache tests and eight
  retirement tests with explicit isolated installation authority. Whole-tree
  strict Desktop Clippy still reports 436 diagnostics outside the owned grant
  files; native/store strict Clippy passes. Production
  adoption and the real native consent flow remain closed. Publication has no
  normal production initializer until source custody and consent are connected.

The next parallel slices are durable source adoption, exact grant publication
and recovery, and retained provider-process qualification/stop. No shared
prompt execution or completed lifecycle E2E is claimed by these prerequisites.

Release 13 built from immutable release 12 plus only the read-only
resource-detail slice and navigation fixes. Its binary is
`f3478fb806ce2b1e2e03eb97cb2e38724eabf1c6c0fcd16fffdc88d2109c9df4`;
only the isolated owner fixture is updated. The other three role fixtures remain
on release 12. Its bundle is
`dcc6b9a52e1bf69f92d5ae4bb39633526df6f1cd1d0cc8510e2fbe8cb8df8a67`.
The exact overlay and source commits are in
`.run/management-integration-source-13/immutable.json`. The frozen store passes
all five resource-detail tests; the source host, package/catalog, EN/CN script-VM
and translation checks pass. The actual owner UI passes three checks each in
English/light and Chinese/dark: details, refresh and Back preserve the visible
model/budget, capability-filtered list, account and original saved command.
All six full-window captures were viewed; there are zero native errors, including
the initial launch and both presentation restarts. Reports and unchanged scores
are in `.run/management-resource-detail-e2e-13-{en,cn}/`. Detail captures score
9.49–9.67; the English list scores 9.48 and Chinese list 8.28. Overall UX acceptance
remains open. The owner is restored to English/light. The fixture presentation
switch does not test the settings UI. Editing,
resource migration and agent runtime are not enabled by this build. Existing
same-model resource cards can still be visually indistinguishable; the probe
uses an explicit occurrence and does not infer a hidden resource identity.
The Chinese detail in release 13 still shows the raw reasoning value `high`;
release 14 includes its display translation without changing the wire value.

The newer managed-runtime checkpoint has 11 passing authenticated HTTP/SQLite/
supervised-process tests and 12 canonical store tests, with combined native
feature strict Clippy passing. It covers cancelled waits, reply loss, fixed
deadlines, logout, concurrent stop and unknown cleanup without releasing capacity.
Hashes and logs are retained in
`hagency-client/.run/managed-runtime-paired-checkpoint/`. These use a test process,
not a real model turn. Read-only inspection found the existing owner uses a
shared Codex profile, which the first dedicated-profile launcher rejects;
supervised shared-profile discovery and real-provider qualification remain work.

Release 12 places the reservation/administrator blocker alongside those
requirements and aligns the contextual project name with the other connection
fields. Its real-window results are recorded above; it does not meet 9/10 in
all locales.

- Client source: `39e990c8`, with reviewed resource/core/store/metering changes
  from `hagency-rs@1212833a`, common ancestor `e51a0b1`. Current workspace
  dependencies and the existing execution-path compatibility change are retained.
- Restored coordinator reservations, delegation revisions, project setup receipts,
  lifecycle/refusal records, final settlements, legacy adoption support and Octos
  usage evidence. Delegated executor integration remains pending.
- SDK-authorized `NativeOwner` opens no OAuth callback listener. Standalone
  browser authorization retains its callback. Provider/account/workspace calls
  now use typed native dispatch. Other existing personal commands still use the
  in-process HTTP adapter and remain scheduled for typed extraction.
- `hagency_store::management` provides direct typed resource list/configure/
  create/publish services. One writer snapshot includes configuration, budget,
  usage evidence and observation time. Filters and pagination are bounded.
- Management binds account epoch, package digest/instance and enrolled device
  generation. Writes use original revocable writer gates. Account/package
  retirement fences access; scripts receive no credentials.
- Schema 70 adds atomic command receipts and authenticated profile binding.
  Retrying after a lost reply or restart returns the original result. A reused
  key with different arguments conflicts. Failed receipt persistence rolls back
  the resource change. Populated unscoped ledgers require explicit migration.
- `NativeOwner::management` opens the direct service on a blocking worker,
  deriving its ledger directory from the verified server/subject/owner profile.
  Package closure, owner retirement and shutdown revoke its sessions.
- Fixed concurrent creation of private account directories: an `AlreadyExists`
  race succeeds only after ownership, permissions and symlink checks pass.
- Added first-resource enrollment from the native provider catalog. A short-lived
  native selection binds account, device, exact inputs and provider reference.
  Resource and command receipt commit atomically, the resource starts unpublished,
  and scripts never receive credential references. Capability filters describe
  eligible unpublished definitions without publishing them to the public catalog.
- Added central resource admission, designated administrator approve/reject,
  owner/admin revocation, revision checks, role-scoped durable Inbox and atomic
  notification intents. Approval reports `admitted`; it does not claim a verified
  connection, reserve capacity or create an agent. The server uses device-scoped
  authorization and preserves separate agent/executor ownership constraints.
- Added exact native host services and contract-version negotiation. Only the
  verified built-in digest can obtain management capabilities. Downloaded bundles
  and legacy Palpo bundles remain denied. Account/package closure retires the
  management handle without quitting explicitly authorized engine work.
- Added native provider connection confirmation with shared controls, SVG and
  English/Chinese text. Splash requests only a runtime; it cannot approve itself.
  Consent expires, is consumed once and is checked again after async waits.
  Back/Cancel does not connect; an already connected account is left running.

Provider connection cancellation/acceptance and positive Codex enrollment have
now passed real native UI checks with a separate owner account. Fresh-login
flows and trusted workspace selection remain. Claude's current model aliases
are not asserted to satisfy exact model qualification. Octos configuration
discovery is reported separately from verified provider-account authentication.
The central outbox currently records intent only; delivery/reminders,
re-admission/recovery, projects, allocation and shared execution remain
implementation work. The separate executor verification slice now passes the
native workflow described below.

## Native wizard validation in progress

The first real run reached the local server as `@admin:localhost:8088` and read
its actual connected Codex provider. Native testing caught and is correcting a
missing SVG module import, forward references in script callbacks, and repeated
model widget IDs that hid choices. A debug cold start also exhausted the script
time budget; the release build must be tested without increasing that limit.

The production bundle provides overview, resource capability filters, measured
capacity, connection/admission pages, latest-state Inbox, and the initial guided
onboarding stages. Saved commands persist before submission and retry with their
original ID. Approval and executor verification have separate states; neither
claims allocated capacity or a running agent.

The first actual overview capture scored **9.18/10** with the unchanged mechanical
scorer, but this is not UI acceptance: the visual review found excess chrome and
the workflow still had runtime failures. Capture and raw score are retained in
`.run/management-ui/overview-v1.png` and `overview-v1.ux.json`. Every final screen
and both locales still need functional/visual verification. The older App Hub
CLI rejects the new contract feature; its `octo check` failure remains recorded,
while the current built-in packer validates and seals the actual bundle.

Real follow-up testing caught duplicate JSON keys from string-indexed updates
to named Splash properties: fields looked filled but submission read the old
empty values. Explicit field writes now update both submission and the saved
draft. A reproducible native probe checks unique keys, correctable invalid
coordinator input, actual owner submission, administrator confirmation and the
owner's latest result without administrator controls:
`tools/hagency/native_management_wizard.py`. The first passing run is
`.run/management-admission-e2e-1/report.json`. All three captured windows were
visually inspected; unchanged mechanical scores were 9.12, 9.32 and 9.22.
The confirmation controls were below the fold despite that score; the next
revision replaces decision buttons with inline confirmation to keep it reachable.

The old shared scroll container carried a model page's offset into the next
step. Attempts to construct then immediately render nested dynamic views failed
because rendering queues their insertion. `RinxPageScroll` now keeps a real
scroll view and exposes an explicit route reset; same-page refresh retains its
offset. Earlier blank-page/script errors and failed build attempts remain in
the native/build logs rather than being counted as passing runs.

Provider model choices now intersect the current provider catalog with the
original shared-resource qualification policy. Unknown models remain in the
native projection but are not offered as enrollable choices. The three focused
projection/policy/recovery tests and strict engine lint pass, recorded in client
`miniapp-provider-policy-tests-1.log` and `miniapp-provider-policy-clippy-1.log`.

Draft save/readback has been exercised through normal restarts; this is not a
power-loss durability claim. The current Splash filesystem write is not atomic
or fsynced. Native command custody and recovery after interrupted disk writes
remain required before the full durable-workflow acceptance gate.

The expanded native run `.run/management-admission-e2e-3/report.json` passes
six checks with **zero native script errors**, using release instrument build 9.
It creates an actual Codex resource with an edited 123,456-token limit, rejects
NaN locally, verifies a new step resets scroll, corrects invalid coordinator
input, and completes the two-account approval flow. The four whole-window
captures score 9.12–9.34; the administrator's confirmation controls now fit.
Run 2 exposed an enrollment callback reading a nonexistent `admission` field;
that field is now read only for central admission operations. The probe also
excludes the Splash root's source text from visible-text assertions; earlier
run 1 is retained as preliminary evidence, not equivalent strict coverage.

Normal owner restart retained the approved resource and server result. Chinese
light-theme resource/status/detail captures scored 9.08–9.22. The Chinese dark
status capture scored **8.13**, so the full UX gate remains open. Dark contrast
passes strongly; the scorer reports lower content/alignment/rhythm scores.
The unchanged scorer and original images remain in `.run/management-ui-owner/`.
Visual review also found an English host header/Run button in Chinese mode;
their existing catalog translations have been wired up for the next build.

## Inventory and evidence

Executor connection verification now has a central device-bound acknowledgement
and a direct native provider/ledger check. The server migration preserves admitted
connections and adds expiring verification observations. Replayed command receipts
retain their original deadline; revoked devices, another owner's device, another
device of the same owner and changed resource revisions are rejected. The UI's
verified status is scheduled to expire and re-reads the current server result
after a write; it does not treat this as project allocation or agent execution.

Focused verification checks pass: server `miniapp-verification-postgres-2.log`
(84 tests, zero ignored), strict server/native lint, client management store
(11 tests) and native adapter suite (16 tests), desktop host boundary (3 tests),
contract capability coverage and built-in catalog (8 tests). The i18n audit has
1,847 keys and no missing translations. These do not yet establish UI success:
native verification E2E runs 1 and 2 reached admission but failed to display the
verification controls. Ordinary helper calls did not append widgets to the
render target, and nested `on_render` callbacks are not automatically invoked.
The controls have been moved into the existing page render tree. Run 3 now passes
all eight checks with zero script errors using release instrument build 4:
`.run/management-verification-e2e-3/report.json`. It creates a resource, completes
the separate-owner/admin admission flow, verifies the actual native provider
binding, waits for the real five-minute expiry, confirms the server reports
stale status, and verifies again with a new command/revision. No artificial clock
or host responses were injected. English whole-window verification captures
score 9.18 and the approved state 9.40; the raw scores and images are retained.

A third actual SDK login as `@miniapp_coordinator_1015:localhost:8088` reads its
assigned connection and has no owner verification, administrator decision or
revocation controls. Its Chinese dark-theme capture was inspected and scores
8.47, so the overall 9/10 gate still fails. The translated native mini-app header
and window title now render correctly. The test setup first lacked Playwright
on system Python; the existing build-tools environment completed real Pasion
SSO. An initial capture-copy path error was also retained; the role assertions
and capture were rerun successfully in the separate capture directory.

`hagency-management-parity.json` records 104 original native actions, 189 original
JavaScript routes, and 70 current owner commands. There are 61 exact normalized
method/path matches between the old JavaScript and native console. These are
mapping candidates, not proof of equivalent behavior or authority. Unmapped
agent protocols and operator actions must be reconciled before parity acceptance.
The original framework registry also names Hermes and Codex ACP alongside
Claude, Codex and Octos. `launchable:false` in their manifests applies to the
tmux launcher; their documented `hagency acp-up` path must still be inventoried.
Hermes native support and Codex ACP replacement parity remain explicit work.

Initial restored core/store/metering run: 711 passed, 73 explicitly ignored,
zero failed. Ignored legacy parity cases remain visible gaps. Subsequent work
adds management tests and schema fixtures; the final store rerun is recorded
separately when complete.

Focused evidence in `hagency-client/.run/`:

- `miniapp-management-tests-3.log`: eight direct-service tests passed, including
  concurrent retries, restart, rollback, profile isolation and directory races.
- `miniapp-sdk-tests-6.log`: 13 SDK authorization tests passed, including opening
  the direct resource service and refusing renewal of a closed package.
- `miniapp-browser-mode-test.log`: standalone callback compatibility passed.
- `miniapp-engine-clippy-3.log`: current engine/store library Clippy passed with
  warnings denied.
- `miniapp-domain-tests-6.log`: 622 passed, zero failed, 47 explicitly ignored
  across 68 store test binaries/doc-test runs, including the eight new tests.
- `miniapp-foundation-evidence.json`: source-tree fingerprint and test-log hashes.

Later focused evidence (does not retroactively extend that foundation fingerprint):

- Client `miniapp-sdk-central-tests.log`: 15 SDK tests passed, including exact
  central routes, device grants, package retirement and typed provider dispatch.
- Client `miniapp-enrollment-store-tests-2.log`: ten management tests passed,
  including first resource, concurrent/restarted replay and native-proof scope.
- Client `miniapp-provider-projection-tests.log`: private-field filtering passed.
- Client `miniapp-provider-projection-tests-2.log`: also confirms a disconnected
  provider is never shown as account-verified, even if the adapter flag says true.
- Client `miniapp-provider-consent-test-1.log`: read-only, expired and closed
  provider connection attempts denied before any CLI invocation.
- Client `miniapp-onboarding-clippy-2.log`: strict engine/store library lint passed.
- Client `miniapp-onboarding-tests-final.log`: 18 passed, zero ignored (16 SDK
  authorization cases plus two provider projection/recovery cases). A failed
  reply after a possible provider change reports outcome-unknown instead of
  falsely asserting rollback. `miniapp-onboarding-clippy-final.log` passes on
  this final native source.
- Server `miniapp-management-postgres-1.log`: all 83 agent-service tests passed
  against a dedicated PostgreSQL database, with zero ignored. Covers rollback,
  concurrent/restarted requests, roles, expiry, revocation and actual API routing.
- Server `miniapp-management-clippy-1.log` and `miniapp-server-host-check.log`:
  strict service lint and full backend check passed.
- Desktop `miniapp-host-tests-2.log`: two host contract/consent tests passed.
- Desktop `miniapp-host-ui-tests-2.log`: six tests passed, including actual
  MiniAppsPanel script evaluation/navigation and translation preservation. The
  first run used the public `miniapps` alias rather than the compiled module name
  `octoscript_apps` and selected only five tests; it was not counted as UI coverage.
- Desktop `miniapp-management-contract-tests-3.log`: 34 contract/doc tests passed.
- Desktop `miniapp-catalog-management-tests-4.log`: 19 passed, one existing ignored,
  with provider list/enroll/connect capabilities included. The third invocation
  used the root workspace; this crate is a separate workspace, so that command
  failed before testing. The corrected run uses its own manifest and locked graph.
- Desktop `miniapp-provider-i18n-2.log`: 1,827 English/Chinese keys, 1,437 explicit
  translation sites, zero missing. The first parity check failed because the new
  English keys were missing; both catalogs are now updated, with the failure kept.

These are service/build checks, not real provider execution or MacBook UI E2E.
The debug desktop test linker reports a large unwind-table warning; the binary
still links. Existing ignored legacy tests remain gaps, not passing coverage.

Earlier failures remain recorded. They exposed schema fixtures that kept new
tables while lowering `user_version`, a private-directory startup race, and
concurrent commands contending on an authority gate. Fixtures now reconstruct
the old schema, directory creation remains checked, and each package has a
bounded serial mutation queue. Failed attempts do not count as passes.

## Remaining implementation gates

1. Finish typed service extraction and original-action mapping, preserving
   provider setup, accounts, connections, lifecycle, tasks/graphs, approvals,
   usage, alerts and recovery.
2. Finish connection recovery for changed resources/other devices, coordinator allocation/project workflows,
   notification delivery/reminders and explicit delegated executor authorization.
   Preserve the existing personal-owner foreign key and reply/tool/cleanup checks.
3. Finish provider/workspace interactions and implement the complete shared-theme
   English/Chinese wizard UI using the new host services.
4. Complete migration, packaging, real MacBook workflow/failure/provider E2E,
   Makepad instrumentation, and unchanged UX scorer >=9/10 for each required
   representative screen. Keep separate physical mobile validation gates.

## Native command recovery validation (2026-10-15, continuing)

The native journal and exact pending/retry/acknowledge host services are now
implemented. The first real crash run committed a server admission, held the
native reply unrecorded, killed the isolated app and corrupted its page draft.
It then exposed a real design error: SDK restart rotates executor authorization
generation, which made a generation-keyed journal invisible. Earlier store tests
had recreated the same generation and were insufficient. Failed run 1 and the
unreleased generation-keyed fixture rows remain preserved as evidence; they are
not counted as a passing recovery workflow or migrated by the test harness.

Request identity now survives authorization rotation for the same owner and
installation. Current generation checks still fence every operation, and central
receipt recovery returns the latest visible result without restoring execution
permission. Native resource receipts repair an interrupted reply before another
provider check. A new validation failure cannot discard an older uncertain
outcome. Restarted admission/verification still needs explicit executor
re-admission work; receipt recovery alone does not make an old executor grant
current.

The full store run before the continuity correction passed 628 tests with 47
existing ignores (`miniapp-intent-full-store-tests-2.log`). The first run caught
two old-schema fixtures that failed to remove the new table before downgrading;
those fixtures were corrected. The final focused store run adds the actual
resource-receipt recovery case and passes 15 checks
(`miniapp-intent-store-tests-7.log`). SDK/native suites each pass 17 checks
(`miniapp-intent-sdk-tests-7.log`, `miniapp-intent-native-tests-7.log`), and strict
native Clippy passes. One earlier parallel SDK run exceeded an existing one-second
fixture wait under build load; the failed log remains, and the unchanged timing
assertions pass in the sequential rerun.

Server `miniapp-intent-postgres-4.log` passes 85 tests with none ignored, including
real HTTP routing, same-installation recovery across rotation, changed-argument
conflicts, foreign-device denial, stale-authority fencing and revocation. Strict
server lint and the full backend build pass. Earlier compile errors and the test
request missing the required native Host header remain recorded. The final
native crash rerun is still pending at this checkpoint; no overall migration or
UX acceptance is claimed.

The corrected real native crash rerun now passes all four assertions with zero
script errors (`.run/management-recovery-e2e-2/report.json`). The actual server
committed while SQLite reply recording was locked; the owned test process was
killed, its draft deliberately truncated, and the same frozen binary restarted.
Executor generation changed from 14 to 15. The Saved requests UI recovered the
original command, central state contains exactly one contribution and one receipt,
and acknowledgement removed the active payload while preserving its tombstone.
Both whole-window captures were inspected. The first harness version also needed
to wait for asynchronous socket teardown and allow a previously consented app to
start without a second Run prompt. No fake host response or success row was used.
This passing run uses a newly created journal entry with the corrected stable
identity; it does not claim migration of the earlier unreleased failed-run entry.

Desktop host boundary tests pass four checks, exact contract coverage one, and
built-in catalog eight. The i18n audit reports 1,850 keys and zero missing entries.
The overall UI gate remains open: the previous Chinese dark capture scored 8.47.
No production deployment, PR, merge or full migration acceptance has occurred.

The unchanged scorer (`Octoscript-OH/tools/uxscore.py`, SHA-256
`14d9018b310ac885f87493367ff813581a1c303bc4c4ee209c4ca90b4b8ee286`)
scores the new recovery and recovered-result windows 9.64 and 9.11 respectively.
Raw scores are stored alongside their real captures. The recovery source/binary
fingerprint is `.run/management-recovery-evidence.json`; the overall UX gate
continues to fail until the remaining locales/themes/screens are accepted.

## Explicit executor reconnection (native validation passed)

`hagency.connections.resume` is now a separate exact capability. The original
owner/installation checks its unchanged resource and provider before reattaching
the current authorization generation. It keeps the request/decision, advances
revisions, records an audit/receipt/outbox intent atomically, and leaves old
verification stale. Expired or closed actions cannot be reopened. Another device
cannot use this path, and read/verification capabilities do not imply reconnection.
The native provider witness is shared internally while retaining separate exact
permissions. This closes the fresh-device-check step for the same installation
and unchanged resource. Other-device/resource replacement remains separate work;
it is not authorized by this operation.

The final server suite passes 87 tests with none ignored
(`miniapp-resume-postgres-2.log`), including expiry and real HTTP routing. Strict
server lint, the backend build and release instrument desktop build pass.
Focused store tests pass 15, SDK
boundary tests pass 17, strict native lint passes, and i18n reports 1,851 keys with
zero missing. The sealed bundle and release desktop rebuild include the new
permission and English/Chinese reconnection controls. Recovery explanatory copy
now describes resuming an unfinished request without exposing storage/command-ID
implementation details in the product UI.

The two-account native rerun passes three workflow checks with zero script errors
(`.run/management-reconnection-e2e-2/report.json`). The owner reconnects the
original recovered request, the designated administrator approves its current
revision, and the owner verifies the actual provider. After a second restart,
explicit reconnection preserves that approval and requires fresh verification.
Read-only database assertions confirm one contribution and one approval decision.
Both instances use release binary SHA-256
`2bb2d3f8266d22afc6764ec9dcbfd6c9e9785d5cc4a65ec4428db46f1282d2f8`.

The first probe failed because the target label was below the visible scroll
region; the probe now scrolls real controls before matching. An initial admin
launch also preceded SDK account restoration and was retried after the account
was available. These failed attempts remain retained. No state or host-response
injection was used to make the passing rerun succeed.

All distinct whole-window reconnection captures were visually inspected. The
unchanged scorer reports 9.54 for pending/reconnected states and 9.48 for verified
states. The two verified captures are byte-identical; they represent separate
successful checks before and after restart. Raw scores and captures accompany
the report. Desktop host tests pass four, contract coverage one, and catalog
tests eight. This validates reconnection, not project allocation, delegated
execution, notification delivery or the full migration. The overall locale/theme
UX acceptance gate remains open.

## List verification status correction

Review found that Overview and Connections formatted only the admission state,
so they showed "Admission approved" even when detail correctly showed "Connection
verified". Both lists now use the same status projection as detail. List responses,
including later pages, schedule expiration from the server's observed deadline;
leaving a list open does not retain an indefinitely verified label.

The bundle was resealed (`miniapp-status-bundle-stamp-1.log`), catalog validation
passes (`miniapp-status-catalog-check-1.log`), and the instrument release rebuild
passes (`miniapp-status-desktop-build-1.log`). Actual native English owner and
Chinese dark coordinator sessions use SHA-256
`4749e0f157f9540e6015d1a3729841ef1636c72d62e96c6505ace332cf0d7471`.
The new probe is `tools/hagency/native_management_status.py`. Its first two
attempts exposed probe timing/scroll assumptions: disappearing reconnect controls
did not imply completed provider checks, and selecting the current route retains
its scroll offset. Both attempts remain recorded as failures. The probe now waits
for the next available action and scrolls to the list heading before searching.
An earlier launch probe also timed out before the mini-app became ready; the
actual account and mini-app were subsequently confirmed before testing.

Initial captures in `.run/management-status-e2e-3/captures/` were inspected.
English detail/list scores are 9.48/9.13; Chinese dark detail/list scores are
8.61/8.37 with the unchanged scorer. The list captures include expected clipping
at the scroll viewport, and are functional evidence rather than a claim that all
list layouts have passed UX acceptance. Visual review still finds excessive
vertical space, weak hierarchy between status and role metadata, and a coordinator
hint that says the owner must verify even after a successful verification. These
remain product UX work; no scorer adjustment or passing full-UI claim is made.

The completed status rerun passes all four assertions with zero native script
errors (`.run/management-status-e2e-3/report.json`). English Overview and Chinese
Connections expire without refresh/navigation at 298.93 seconds, using the
existing one-second conservative display margin, then detail agrees with the
actual server. All six captures were inspected. The expired English Overview
scores 8.91 and the Chinese list 8.38; those lower-scoring states also remain
inside the open UI acceptance gate.

## Parallel project implementation and current validation

At the user's request, three sub-agents now own the project backend, native
bridge and mini-app UI, with root handling integration and actual native E2E.
This does not change the full-parity gates above. The next complete milestone is
project request/decision; allocation and delegated execution are still separate
unfinished protocols.

The hierarchy revision makes the connection state prominent, groups role metadata
below the primary action, removes duplicate label padding and marks the active
navigation section. A verified owner sees **Check again**. The coordinator now
sees a renewal hint instead of being told the already-completed check is pending.
All colors/fonts come from the shared theme. Real owner and Chinese dark
coordinator sessions passed four checks with zero script errors in
`.run/management-hierarchy-e2e-1/report.json`, including actual expiry after
299.91 seconds without navigation or refresh. Release SHA-256 is
`bd313af3e74193484c9f848da141681aa90337014fa7bbc0d1a22b9459401d89`.
Inspected whole-window captures score 9.33/9.53 for English detail/list, 9.40 for
expired Overview, 8.60 for Chinese dark detail and 8.73 for both Chinese list
states. The scorer is unchanged. The overall 9/10 UX gate still fails.

The new project backend provides admitted-resource metadata/capability discovery,
project request/list/detail, designated-server-administrator approval/rejection,
manager/admin withdrawal, current named Inbox cards, and command recovery.
Schema v2→v3 preserves resource admissions and verification. Approval explicitly
means `awaiting_allocation`: it creates no executable Matrix project, reservation,
agent or binding. Resource withdrawal closes its pending/approved plans atomically.
Responses are bounded by serialized size as well as item count.

The actual PostgreSQL/HTTP suite passes 98 tests with none ignored, including
concurrent decisions, resource withdrawal, expiry, atomic rollback, old-schema
migration, role isolation, replay and escaped-text pagination
(`hagency-server/.run/miniapp-project-postgres-3.log`). Strict service lint and
the full backend build pass. The native bridge has distinct exact capabilities,
closed typed arguments and journal operations. SQLite close/reopen and grant
isolation tests pass four checks; Desktop host checks pass five; exact contract
coverage passes. Native console regression passes 141, with four existing ignores
(`hagency-client/.run/miniapp-project-native-final-tests.log`); strict lint passes.
One earlier concurrently loaded SDK run hit an existing one-second startup wait;
its failure is retained, and the serial rerun passed all 18 SDK checks.

The bundle adds a three-step English/Chinese project request wizard, named Inbox
entries, role-specific decisions, explicit allocation status and draft recovery.
An older recovered result cannot erase a different newer draft; conflicting
command identity remains an uncertain outcome. Package/catalog validation and
Makepad parser/helper checks pass. These do not establish actual integrated UI
success on their own. The completed actual native run now passes all five checks
using separate manager, administrator and coordinator SDK accounts
(`.run/management-project-e2e-1/report.json`): draft recovery after restart,
administrator approval, coordinator read-only access, administrator rejection,
and manager withdrawal updating an already-open administrator card. Database
assertions confirm approval creates no executable project, agent or binding.
There were zero native script errors. All three instances use release SHA-256
`86c3d7fef8776583e524b824c6acca4e784e48c7f06dfd9b566680e2c5b6bfe6`.
The isolated localhost backend uses SHA-256
`ce1d4db86d2a17b3aad5762ef1b138f3f89dc91b55289cc77ab53ac543b5e567`.

Actual older-server testing exposed a missing-field script error that static
parsing missed. Optional features, Inbox names and partial stored drafts now use
the pinned Makepad VM's tested destructuring defaults. The corrected native
compatibility run hides Projects and keeps Inbox usable on the old server with
zero script errors (`.run/management-project-launch-3/compatibility.json`).
Earlier failed compatibility and launcher probes remain retained; the helper
now waits for SDK account restoration and the real mini-app window rather than
assuming a click succeeded during sidebar layout. Six dynamic permission
descriptions also lacked Chinese translations. The audit now includes this
previously unscanned path: 1,857 entries, 1,444 literal calls and 26 permission
descriptions, with zero missing (`miniapp-project-i18n-dynamic-2.log`).

All five whole-window project captures were visually inspected. The unchanged
scorer reports 9.21 for administrator confirmation, 9.37 for manager review,
9.18 for pending and 9.33 for approved. Chinese dark coordinator detail scores
8.29. The overall 9/10 UX gate therefore remains open; screenshots and raw scores
are retained in `.run/management-project-e2e-1/`. The sealed bundle tested here is
`4384e5fa25dd6e5cac891c6499a43d75560b2cdc0c4c8ba766cc3bd599395158`.
No deployment to mini1, production release, merge or full migration acceptance
is claimed.

The separate real project crash test also passes all four assertions with zero
script errors (`.run/management-project-recovery-e2e-1/report.json`). A PostgreSQL
lock first confirms the native request is durable; a SQLite writer lock then
blocks only local reply recording while the actual server commits. The owned
manager fixture is killed, and the administrator approves the original project
while that manager is offline. After deliberately damaging only the fixture's
page draft and restarting the same frozen release, Saved requests recovers the
latest approved result across authorization-generation rotation. Exactly one
project and one command receipt remain, and acknowledgement removes the active
native payload. Both real captures were visually inspected. This exercises
project recovery itself, beyond the earlier resource-admission recovery.

The next parallel slice addresses shared native resource accounting, central
project allocation and the remaining locale/theme UX gaps. Review found that
different Matrix profiles can associate the same local provider account with
independent resource ledgers. Before shared allocation can be accepted, one
authoritative owner ledger must enforce explicit profile grants and conserve
capacity across profiles/processes. This is a confirmed accounting design gap;
the request-only tests above do not reserve capacity or demonstrate an actual
overspend. See the client/server allocation design reviews for the required
authority, reservation, recovery and migration boundaries.

## Parallel allocation and native UI checkpoint

The revised project/detail bundle
`67d3423100e4402f013093e0fed991175cf7306f684bbe3bd4883c413ed780f4`
passed all five actual native project checks with zero script errors in
`.run/management-project-e2e-3/report.json`. All three role instances used release
`208a5a60cb520f680472f4ea8ff09d4c04db4093a748067f527b05ba84ee611d`.
All five captures were inspected. English scores are 9.36–9.59; Chinese dark
coordinator detail is 8.55, improved from 8.29 but still below the required gate.
The coordinator's resource-connection drilldown also passed using the exact
project and contribution, with no owner/admin mutation controls
(`.run/management-project-drilldown-e2e-1/report.json`).

The first real permission-review probe caught an omitted `storage` disclosure:
24 descriptions were shown for 25 requested capabilities. Source now includes
that description in English and Chinese, with a scrollable review and fixed
Cancel/Run footer. The failed probe is retained in
`.run/management-consent-e2e-1/`; the corrected source requires the second release
build and actual permission-review rerun before it is accepted. Translation
coverage passes for 1,860 entries, 1,451 literal call sites and all 27 known
permission descriptions (`miniapp-project-layout-i18n-3.log`).

The revised connection detail and lists passed four real status/expiry checks
with zero script errors (`.run/management-layout-status-e2e-1/report.json`),
observing the real deadline after 299.76 seconds. All six captures were inspected.
Some list targets were partially clipped; the probe is now stricter about fully
visible labels/statuses and its rerun remains separate evidence. English captures
score 9.42–9.55; Chinese dark captures score 8.51–8.64. No overall UX pass is claimed.

The central allocation foundation passed 110 real PostgreSQL/HTTP tests and
strict lint before the next parent-settlement slice. The native shared registry
passed 13 ledger tests, two OS-owner-root tests and 73 existing regressions;
five pre-existing usage-oracle tests remain ignored. These are separate component
results. Allocation remains disabled and unconnected to the production mini app.
Verified provider identity, explicit legacy-ledger reconciliation, native owner
consent/transport, parent-grant settlement and renewal, project provisioning and
agent/work lifecycle integration still require implementation and acceptance.

Three parallel owners now address provider identity, central parent settlement,
and the remaining UI hierarchy/navigation gaps. Root owns integration and actual
native validation. mini1 and the Article app have not been changed.

The corrected permission screen now passes its actual native English/light and
Chinese/dark run (`.run/management-consent-e2e-2/report.json`, zero script errors).
It discloses all 25 manifest capabilities, keeps Run fixed while scrolling, and
returns to the review after Cancel without saving consent. All four captures were
inspected; unchanged whole-window scores are 9.92–10.00. This test uses release
`164fe8cbdac819c5d12d2eeefb6c1b552d7a7be3ed6687f1739037dcd46b0e50`
and the same `67d34231…` bundle. The stricter visible-row status probe also passes
all four checks, observing expiry at 300.15 seconds
(`.run/management-layout-status-e2e-2/report.json`).

The next hierarchy/navigation bundle
`266e3617ebda4ca0d8f35cb7cd71b4f908d745998e94ced782b7cdb69cc5d9dd`
passes six actual native project checks with zero script errors
(`.run/management-project-e2e-4/report.json`). This adds the coordinator's exact
resource drilldown and return to the refreshed project. All four role fixtures
now run release `0ff23a8ad729b90b9f85f2599d09eb9a2c33c34a12ab3bf9f356a3922e87e948`.
All six captures were inspected. Chinese dark project detail now scores 9.17;
English screens score 9.37–9.66. Chinese dark connection detail remains 8.47, so
the overall UX gate still has not passed.

The native provider adapter now derives a non-serializable identity proof from
consistent authenticated Codex usage-account IDs after pinned-origin and current
scope checks. Thirteen provider regressions (including six new identity checks),
shared-account concurrency and strict library Clippy pass; three unrelated
installed-provider tests remain ignored. A separate real signed-in read-only
probe confirms supported identity, stable observations and the expected origin
(`hagency-client/.run/provider-identity-live-probe/result.json`). It starts no
model turn and reveals no raw identity/credential. Claude/Octos still lack the
required quota-account proof; no email/path fallback is accepted. Execution-time
account binding and legacy-ledger reconciliation remain unfinished.

The default-off central parent settlement checkpoint passed 122 real
PostgreSQL/HTTP tests, strict lint and a backend build. Independent review found
and fixed a parent/child revision collision that could hide held capacity; both
receipt arrival orders are covered. Native parent cleanup is being validated
separately and no end-to-end allocation/settlement or production activation is
claimed by these component results.

## Parallel integration checkpoint

The native owner-registry parent close/settlement writer now passes 23 focused
tests; its previously tested broader library/integration suites also pass with
five existing usage-oracle tests ignored. It remains unwired to the mini app.
The profile-ledger reconciliation audit passes seven focused checks, including
unchanged main/WAL bytes and retained execution history. It always returns
`adoption: not_assessed`: no live ledger was migrated or activated. Its final
library/management suites and strict store/native lint pass. These are component
results, not an allocation or shared-agent E2E result.

The bounded central project-setup slice passes 133 PostgreSQL/HTTP tests, strict
Clippy and a server build. It verifies real Matrix room state and protects managed
projects from generic personal-agent binding bypasses. The validated schema-6
binary is `9a612f8b9e99171f9f89eb7edb29f3e881a1e6ff010eb1fc7be8640161806349`.
After a private full database backup, the isolated localhost server upgraded from
schema 3 to 6. Counts for all eight checked workflow/domain tables are unchanged
(`hagency-server/.run/management-schema6-upgrade/{before,after}.json`). The first
shutdown probe allowed only ten seconds and expired while Matrix sync requests
were draining; the process subsequently exited normally before replacement.
Allocation, parent settlement and project setup remain disabled. mini1 was not
accessed. Native backward-compatibility testing is running separately.

All six stricter status/expiry captures from
`.run/management-layout-status-e2e-2/` have now been visually inspected. The next
connection-detail bundle `b18c828ddfb0f537f42d1e9e581af1b8bc951e09c4bc68128e57117567c56e09`
separates server admission from executor verification and shows the originating
project. Parser, i18n, package and system catalog checks pass. Only rendering
changed; mutation, authority, draft and recovery helpers match the prior bundle.
Native rebuild/capture validation remains pending; no new UX score is claimed.

The schema-6 localhost compatibility run now passes all six native project-flow
checks with zero script errors (`.run/management-schema6-project-e2e-1/report.json`).
All six whole-window captures were inspected. It uses the frozen `0ff23a8a…`
desktop / `266e3617…` bundle, not the pending connection-layout bundle.
The typed native Matrix creation extraction passes 13 focused and seven existing
route tests; two pre-existing live-fixture tests remain ignored. It preserves
original journal/marker bytes, fences account changes around awaits, and refuses
to repeat an uncertain createRoom POST. Managed creation is a separate pending
adapter. The next server cancel/rebase slice passes 140 PostgreSQL/HTTP tests;
its final conformance/lint/build checkpoint and native integration are separate.


## Connection detail and synchronous account retirement checkpoint

The `b18c828d…` connection-layout bundle is now running in the frozen release
instrument build `4f87e94f9f847480873bb1b6ad309f3bdb57810c55c5ce0211f2546788e8462a`.
All three targeted native checks pass with zero script errors in
`.run/management-connection-detail-e2e-1/report.json`: real owner verification,
Chinese coordinator project context across refresh/back, and ordinary navigation
clearing old project context while remaining read-only. All three 2000×1600
whole-window captures were viewed. The unchanged scorer reports English/light
9.60, Chinese/dark direct connection 9.00, and Chinese/dark project connection
8.99. The last score remains below the requested 9/10 gate; it is not rounded up
or treated as full workflow acceptance.

Independent integration review found a gap between accepting an account
transition and asynchronously closing native authority. Desktop now registers
the native owner before SDK authorization starts, publishes switch/logout/token
expiry before synchronous retirement, and prevents a new service during session
close. Registered-but-unpublished initialization is also collected during
shutdown. Isolated real-native constructor/queue/cancellation regressions are
being validated; these source edits are newer than the running build above.

Server schema 7 cancel/rebase/admin-replacement finished with 140 PostgreSQL/HTTP
checks, canonical fixture validation, strict all-target Clippy and a server build
(`f40559bcfb176747fd6bbb7145ad1f8a0ae058a198da2c039ef317b739a4b4f7`).
It has not replaced the schema-6 localhost runtime. The managed native setup
adapter and central assigned-project-admin agent admission are now separate
parallel implementation tasks. Owner-registry source reconciliation, full shared
execution, workflow delivery/reminders and original-feature parity remain open.

## Resource default editor candidate

`execution-policy-default-editor-v1` extends the isolated native resource
create/edit APIs with an optional typed default. Omission preserves the source
default and old receipt serialization; native review shows current/proposed
permissions. Resource detail, EN/CN forms and native confirmation text are
implemented. The existing provenance, account, scope and revision gates remain
in force. Both shipped-form suites pass54 VM cases each and both full-bundle
roots pass. Native adoption, original-command recovery and NativeOwner checks
are running; paired Desktop compilation and actual window acceptance remain
pending. No source or running original installation has been promoted.


## Resource default editor actual-window acceptance

The frozen default-editor Desktop passes63 host checks, four grant UI checks and
release compilation (`ca8a0963ea56bc4c1ec8fee12c1a31cc21c0361d4f8c33af18b56930304f7ef3`).
The actual EN/light and CN/dark flow now completes editing, native confirmation,
cancel, creation with an inherited default and explicit override, and normal
restart. Exactly two resource edits and one creation were committed. Original
resource values, shared quota, capacity grants and agent policies remain intact;
the new resource adds its required ownership grant. All three final captures
were inspected. Evidence: `.run/management46-stage/execution-policy-default-editor-v1/scoped-acceptance.json`.

Earlier failures were in the acceptance harness: static text outside the viewport,
an incorrect assertion forbidding the new ownership grant, and stale selected-card
state carried across scroll snapshots. Each failure is retained. Exact-card
navigation now resets selection for each snapshot and requires a fully visible
button. Completed mutations were not replayed. The isolated fixture is stopped;
the original four Desktop sessions and server are still running.

This is offline provider evidence. Real OAuth/provider execution, integration of
the separately qualified read-only change, populated-history transfer and full
UX acceptance remain pending. Stacked resource actions and form clipping remain
layout work; these captures do not establish a whole-app9/10 score.


## Read-only selection and warm-runtime correction

Following the dispatch path beyond the original executor test exposed two remaining
refusals: the production host-profile SQL skipped nonexclusive leases, and a
consumed warm binding required exclusivity. Both failures were reproduced before
editing production code. The selection predicate now admits Codex read-only work;
other providers retain their exclusive requirement. Warm admission retains its
engagement, resource and workspace identity checks, while dispatch settings derive
read-only access from the actual lease.

`execution-readonly-selection-v1/qualification-v2` passes17 selector,11 warm and55
owned-process cases plus strict all-target Clippy. The cold read-only fixture now
uses production profile selection for managed and unmanaged accounts. Warm tests
reuse the initialized process, apply read-only thread/turn settings despite saved
unrestricted policy, retain the captured policy and clean up workspace leases.
Negative checks cover foreign workspace/device, dirty workspace, conflicting
writers, other providers and revocation. The first corrected warm run was refused
because its test profile omitted the existing opt-in for its plaintext project;
that fixture error is retained and corrected without relaxing the product guard.
These are offline real-pipe tests, not an actual provider sandbox claim.

A paired desktop candidate groups resource actions in the existing wrapping row
and highlights selected default permissions. Its108 EN/CN form checks and both
full-bundle roots pass; the first desktop passes63 host and four grant UI tests.
A sequential composition job waits for that release, preserves its original six
native files, applies the qualified selection/warm changes, builds the next
revision and runs a no-mutation actual-window probe. No original installation
has been replaced. Full migration, workspace recovery and real OAuth/provider
acceptance remain open.

Workspace recovery candidate46 now implements resource-scoped workspace list/detail and a reviewed clear-dirty action through native management and the mini app, with no local HTTP server. The write checks current ownership of every associated resource, active leases, unresolved exclusive dispatches and an exact-attempt revision, then stores its receipt atomically. Retrying an original command does not clear a later failure. Initial workspace tests pass14 (9 new); EN/CN VM checks pass78, existing policy checks80, and full-bundle root checks2. The unsigned bundle was restamped using its matching closed host contract. Full native qualification is running and the desktop build is queued behind the prior policy integration. Actual populated-workspace clear/recovery acceptance is still missing; the prepared native-window script covers only the existing fixture’s empty workspace list. Original45/32 sessions and real-provider authorization remain untouched. Evidence: `.run/management46-stage/workspace-recovery-v1/status.json`.


## Interrupted-task native migration in progress

The integrated policy desktop passed its release qualification (63 host + 4 permission UI tests), actual EN/light and zh-CN/dark input checks, and inspection of all eight native captures. The resource action row and selected default permissions are visible; this does not establish whole-app UX acceptance. Evidence: `.run/management46-stage/execution-policy-integration-v1/actual-layout-v2/visual-review.json`.

Workspace recovery passed all 69 native cases, strict checks, desktop host65, permission UI4 tests and release compilation. Actual EN/light and CN/dark resource entry, empty-list, refresh and back navigation passed; all four native captures were inspected. This validates the empty state only. A populated reviewed release remains an explicit acceptance gap.

The new `.run/management46-stage/stopped-recovery-v1` candidate adds native stopped-task list/inspection, a short-lived exact-attempt review, and continue / accept completed / keep blocked decisions. The console and mini app share the original domain transaction helpers. The mini app uses closed typed host services and an English/Chinese inspect → choose → confirm flow. No local HTTP server is introduced. Original receipts are scoped to account, installation and profile, and saved outcomes recheck current shared-resource ownership. Continue reconstructs message custody from persisted inbox records instead of copying an old serialized envelope; a regression exposed and verified this correction.

Frozen native qualification now passes32 tests plus strict all-target checks: inventory encoding1, owned completion23, adopted workspace5 and saved native intents3. UI80, workspace regression78 and both full-bundle roots also pass. The desktop candidate is compiling with unchanged frozen sources. Large inventories are paged; roots, symlink targets, raw payloads and inspection credentials are not returned to the mini app. Actual populated recovery windows, real providers, original-history transfer and full migration parity remain pending. The original four role sessions and original server have not been replaced.

The stopped-task fixture contains three interrupted attempts and one idle dirty workspace, generated through native test APIs without provider inference or Matrix requests. Its 24 populated tables and source digest are recorded; importing into a copy of the isolated database preserved existing rows and passed foreign-key checks. The actual-window probe is prepared for all three decisions, confirmation cancellation, reviewed workspace release and restart. It has not run yet. A reproduced encoded-inventory limit failure was corrected by bounding serialized JSON bytes as well as item count; the boundary regression is included in the32 passing native tests.

## Populated recovery render correction

The stopped-task desktop-v1 passed67 host tests,4 permission UI tests and release compilation with unchanged sources. Its actual native window passed relative file inventory, quarantined workspace release refusal and exact-workspace task navigation. The first review committed and was acknowledged, but the decision page failed to render: the two TextInput instances used an unsupported `enabled` property. The VM widget stubs did not detect that property error. No recovery decision was committed. The failed report and screenshot are retained in `stopped-recovery-v1/actual-recovery-v1`.

Both inputs now use the existing Makepad `is_read_only` property and retain guarded callbacks. Real full-bundle widget tests cover EN/CN construction and busy/pending read-only behavior, with a separate reproduction of the old render failure. The corrected bundle is restamped; desktop-v2 qualification is running. Its actual continuation preserves the acknowledged review receipt, does not reseed the database, and will make fresh explicit reviews before exercising the three decisions and workspace release. Full acceptance remains pending.

## Populated recovery acceptance checkpoint

The corrected desktop-v2 passes two real-widget regressions, 67 host tests,
four permission UI tests and release compilation. Its binary is
`a7d009bb022e951bd62036ff14b026648a8ca37a5c5573c57a22f07b7bd5e91c`;
all 3,368 source-file hashes still match the qualified source snapshot.
Native qualification remains 32 tests plus strict checks.

Actual hidden macOS Metal windows now complete English/light Continue and
Chinese/dark Accept completed and Keep blocked. Each flow exercises inspection,
selection, confirmation cancellation and exactly one committed decision. Continue
queues the remaining instruction with the same task, session and workspace;
it does not run provider inference. The idle workspace review/cancel/clear flow
also commits once. A normal restart preserves all three decisions, shows no
unresolved attempts and shows the cleared workspace as Ready with no recovery
action. Every canonical table remains unchanged during restart. All original
rows survive; management commands increase from 11 to 19: four reviews, three
decisions and one workspace release. All 18 combined captures were viewed.

Two test assertions required correction without replaying mutations. The first
locator rejected a fully visible button ending at y=741 because its artificial
cutoff was 740. After release, the expected database delta omitted the existing
`workspace_dirty_releases` timestamp table. Read-only verification confirmed that
timestamp matches the saved command result and that only the selected workspace
flag changed. The failed reports are retained alongside the successful restart
verification in
`.run/management46-stage/stopped-recovery-v1/scoped-acceptance.json`.

This accepts the synthetic offline recovery slice only. The fixture is stopped;
the original four desktop role sessions and server32 remain running and have not
been replaced. Original populated-history migration, task-graph management,
remaining lifecycle features, real OAuth/provider acceptance and final platform
acceptance remain open. UX review also retains the misleading generic wait hint
on a quarantined workspace and the first-viewport Continue form layout as
follow-up work; no whole-app 9/10 score is claimed.

## Recovery layout correction and task-graph domain extraction

The separate `recovery-ux-v1` candidate reduces the two recovery inputs from
80 to 64 pixels and suppresses the generic wait hint when a workspace is
quarantined. Its 160 EN/CN flow cases and two full-bundle roots pass. Ten actual
Metal captures were inspected using the complete before/after mini-app source,
the existing qualified Rinx widget library and inert services. At 1000×800,
the Continue button changes from a clipped 25-pixel box to its full 44 pixels;
quarantined workspaces offer inspection guidance while ordinary active leases
retain the wait message. This is layout evidence, not another business recovery
run or acceptance of the pending release binary. Two capture-cleanup failures
remain in their original reports; the aggregate visual review records them.

Formal real-widget regression initially exposed an offscreen test-fixture
problem: its default Fit-height Splash had no established viewport and clamped
multiline inputs to zero height. The actual desktop host uses Fill height.
An independent test binary using that host layout passes all ten cases. The
test fixture now matches the real host; product source did not change for this
correction. Formal regression passes all ten cases. The first successful run
stopped before release because its Python count check compared tuples with lists;
that harness failure is retained in `desktop-v3`. Corrected orchestration and
release qualification are recorded under `recovery-ux-v1/desktop-v4`.

Task-graph work has started in `operator-graph-domain-v1`. The retained domain
has been extracted from its HTTP adapter, retaining graph IDs, dependency
conditions, name-addressed messages, message counters and cancellation history.
A direct no-listener regression exposed a retained restart bug: graph documents
were read through `read_secret`, which refuses anything larger than 512 bytes.
The old loader then silently substituted an empty graph map. The failing run is
retained under `operator-graph-domain-v1/qualification-v1`. The candidate now uses
private file handles with document-sized read limits, rejects invalid existing
documents instead of starting with empty history, bounds message writes to the
same readable size and refuses an exhausted message counter without wrapping.
Regression covers reopen without redispatch, conditional downstream dispatch,
cancellation without erasing completed nodes, malformed/oversized documents,
duplicate normalized graph IDs, private-file checks and counter exhaustion.
All five direct domain cases, five original HTTP graph contracts and strict
all-target checks now pass with unchanged frozen source hashes. The original
HTTP fixture initially failed before graph initialization because the SDK resume
journal canonicalizes a state directory that it had not created. The fixture
now provisions that private directory before Console loads, matching production;
the failed report is retained. Scoped evidence is
`operator-graph-domain-v1/scoped-acceptance.json`.
This does not expose a mini-app service: current authority, document adoption,
durable command recovery and the graph UI still need implementation.

The recovery UX release now builds successfully, with SHA
`88cc70cf9c8f44862cb4f1738ac1595a4b75039ffb08821accc97538151fa35d`.
Its read-only actual-window startup passes: all three saved recovery decisions
and the one workspace release remain, no unresolved task or recovery action is
offered, and every canonical table is unchanged. Both new-release captures were
inspected, in addition to the ten layout captures. All 3,368 qualified source
hashes still match. The fixture is stopped and its prior binary restored; no
business mutation was replayed. Aggregate evidence is
`recovery-ux-v1/scoped-acceptance.json`. The separate graph correction is not
included in this desktop binary.

Both candidates remain staged. Original role sessions and server32 are
unchanged; task-graph mini-app/authority/adoption, other retained lifecycle
features, full migration, real OAuth persistence and whole-app UX acceptance
remain open.


## Retained graph inventory and real provider credential verification

The new `operator-graph-history-v1` source pair builds on the qualified graph
engine and recovery UX candidates. It adds a native-only, read-only inventory
of `task_graphs.json`, `messages.json` and `.msg_counter`. Exact bytes, unknown
fields, IDs and counters are retained privately; public review contains counts
and a digest, without message contents or credential-bearing fields. Duplicate
keys/IDs, ambiguous dispatch links, oversized documents and unsafe files refuse
review. Missing files are distinct from empty files, and changed bytes, replaced
file handles or a replaced root invalidate the original observation.

Review occurs before SDK retirement. After the existing source drain, validation
requires that same owner, device, profile, registry and finite host gate. The
first test run exposed an incorrect attempt to use the already-retired SDK;
that implementation was corrected, not bypassed. The failed report is retained
in `qualification-v1`. `qualification-v2` passes graph/domain10, source-custody38,
original HTTP5 and strict all-target checks with resource adoption enabled.
The default-feature strict all-target check also passes (`strict-default-v1`).
This is descriptive inventory only: graph writer fencing, canonical import,
principal/project mappings, reviewed graph commands, mini-app UI and delivery
remain incomplete. Empty SQL-history evidence is never treated as JSON-history
evidence. Neither this source pair nor a new desktop binary has been activated.

The user completed a fresh real Codex authorization in the original owner45
session (attempt6). Actual native UI confirmed “Account signed in”. Two
subsequent metadata inspections each launched a fresh provider child and read
the same private account successfully. All nine observed resource/execution
business tables remained unchanged, and all three acceptance captures were
inspected. Evidence: `real-owner-persistence-v2/scoped-acceptance.json`.
This verifies real credential persistence across provider child processes;
it does not claim a full desktop restart, resource enrollment or inference.
No new OAuth is needed; retain the signed-in account.

The live catalog also exposed a separate enrollment gap: it contains
`gpt-6.1-sol` and `gpt-6-astra`, while the shared embedded Codex role policy only
contains `gpt-5.6-sol`. Both returned models therefore show “Not qualified for
shared resources”. The policy was not silently widened. The initial persistence
probe incorrectly expected an enrollment button; its failure is retained in
`real-owner-persistence-v1`, and v2 checks actual model reads independently of
resource qualification. Full migration and real enrollment remain unfinished.


## Current Codex model compatibility and retained graph writer exclusion

`current-codex-models-v1` explicitly adds `gpt-6.1-sol` and `gpt-6-astra`
to the one embedded qualification policy. Their supported efforts were checked
against the official OpenAI model pages on 2026-10-19; availability was already
observed in the original authenticated account. Hagency retains its own role
floors: low/lightweight, medium/medium, high/xhigh/max/strong. These are local
qualification rules, not provider quality guarantees. Enrollment still requires
the exact live account catalog; unknown models and unsupported efforts stay
unqualified. Old model profiles, defaults, resources and shared quota are retained.

Native regression passes55, including the original JavaScript policy fixtures,
new-model enrollment, same-account quota and original-receipt recovery after
reopen. Twenty-four unrelated pre-existing core cases remain ignored. Default
and resource-adoption strict all-target checks pass. The mini-app VM passes44
EN/CN cases; desktop release tests pass15 (one host, four enrollment widgets,
ten recovery widgets). Four real Metal layout captures were inspected. Two
harness mistakes are retained in their failed reports: the initial new VM test
compared the draft's model object with a string, and the first layout probe
expected an offscreen second model before scrolling. Corrected maintained VM
checks and the explicit scroll probe pass; product UI needed no changes.

Release SHA is `8f9bf978a3f382f07f232e1ce8ea4f4c7f14fe670f9663d94af62ca3a697996b`.
The actual new release was started against the preserved recovery fixture,
viewed, then stopped. All canonical tables are unchanged, no business action
was replayed, and its previous binary was restored. Both release captures were
inspected. The only post-build source change corrects the Python test assertion;
`source-qualification.json` verifies all3375 tracked snapshot entries and that
compiled inputs are unchanged. Scoped evidence is
`current-codex-models-v1/scoped-acceptance.json`.

`operator-graph-writer-v1` then reproduced a separate missing exclusion: a live
GraphStore did not prevent exclusive installation migration. The store now
retains the existing cooperative writer admission across all graph, message and
counter operations. Failed opens release it; a published migration refusal
prevents new legacy admission. Graph/domain13, source-custody38, retained HTTP5
and both strict configurations pass. This source-only change is not in the
model desktop binary. Its scoped evidence is
`operator-graph-writer-v1/scoped-acceptance.json`.

Both candidates remain staged. Existing four role sessions/server32 and the
successfully signed-in real provider account remain unchanged. The original
owner database is schema93; the candidate uses95. No new OAuth is required,
but real-account desktop upgrade and resource enrollment are still unverified.
Canonical graph import, downlevel/external-writer coverage, principal/project
mapping, reviewed commands, graph mini-app UI and runtime delivery remain open.


## Operator graph history archive — scoped backend acceptance

The staged `operator-graph-archive-v1` now copies the exact selected-root
`task_graphs.json`, `messages.json` and `.msg_counter` into private canonical
history. Schema96 stores a manifest plus immutable1MiB chunks. A copy remains
`copying` and unreadable until one final transaction verifies every byte and the
original source, then marks it `archived`. The same original command can resume
after interruption/restart; changed inputs or a second key cannot replace it.
Original IDs, unknown fields, private message tokens and the exact counter stay
in the private archive. Only bounded counts/digests leave the native review.

A separate native `ArchiveOperatorGraphHistory` permission is required. The
writer retains the original profile, source handles and cooperative installation
exclusion. Each chunk rechecks file identity and authority; the sealed commit
checks the full source contents, avoiding a whole-file reread for every chunk.
The full64MiB document passes both store and native source-custody tests. No
source file, runtime task, message delivery, capacity or agent is created or
modified by archival. This is an immutable history copy, not active graph
adoption or a permanent fence on legacy/downlevel writers.

Validation passes517 tests:457 complete-store library tests,13 graph-domain
tests,41 native source-custody tests,5 retained HTTP contracts and1 schema93
approval-upgrade test. The store suite retains21 existing ignored cases. Both
`hagency` and `hagency-store` pass strict all-targets Clippy with default and
`resource-adoption,test-support` configurations. The paired unchanged mini-app
and desktop host pass release-profile `cargo check`; no new release binary or
archive UI was launched. The existing desktop `ToolApprovalTarget` visibility
warning remains. Evidence: `.run/management46-stage/operator-graph-archive-v1/`
`scoped-acceptance.json`, `qualification-v4/qualification.json`,
`extra-qualification-v2/qualification.json`, `desktop-check-v1/qualification.json`.

Earlier failures are retained: a non-Clone test helper, explicit source-schema
allowlists, synthetic downgrade fixtures retaining later tables, and one old
current-schema assertion. Production source remained unchanged during final
native qualification; the subsequent one-line integration-test assertion fix
is documented in `source-qualification.json` and passed its targeted test plus
store Clippy. The original approval context/generation assertions remain.

No source promotion, PR, deployment, activation or original-session restart
was performed. The four Desktop45 role sessions, server32 and successful real
OAuth remain untouched. The next source base is this paired candidate. Active
graph import, principal/project mapping, legacy-writer retirement, reviewed
graph/node commands, mini-app pages, runtime delivery and full E2E remain open.
Real-account resource enrollment and the schema93 desktop upgrade are also
still unverified. This acceptance does not mark MGT-045–049 or the full migration
complete.


## Task graph history reader — mini-app integration

`operator-graph-reader-v1` builds on the paired archive candidate. It adds four
closed, read-only host services for archive discovery, graph pages, graph nodes
and one node's dependencies. The native service reads only the sealed graph
document, bounded to4MiB; it never reads the64MiB message document. Parsing runs
on a blocking worker, outside the UI thread. Each request checks the current
package/profile authority before and after reading. The new
`ReadOperatorGraphHistory` permission is separate from both task reads and
archive creation. Legacy storage reports unavailable history rather than an
empty successful archive list.

The Hagency overview now opens four EN/CN history pages using the existing Rinx
widgets and theme. Graph and node pages contain at most16 rows. Dependencies
render16 at a time. Navigation retains the original archive/graph/node IDs and
list cursors, preserves existing saved commands, and rejects mismatched or stale
replies. Incomplete copies have no Open action. Recorded owners/agents and
progress are explicitly historical; these names grant no current project or
runtime authority. Result bodies, conditions and message identifiers are not
projected; their presence is indicated while the original bytes remain private.

The focused native tests pass34 cases:14 archive regressions,19 graph-domain
cases and1 separate session-permission test. The full-bundle VM passes50 EN/CN
state/navigation/pagination cases plus2 root-construction checks. Eight actual
macOS Metal captures were inspected, covering four pages in EN/light and
CN/dark. These use the unchanged Rinx widget/theme renderer and inert fixture
services; they do not prove a native business E2E or a new release startup.
Evidence is under `.run/management46-stage/operator-graph-reader-v1/`, including
`vm-v5/report.json`, `full-root-v1/report.json` and
`actual-layout-v1/visual-review.json`.

Four strict native configurations and final default/enabled release-profile
desktop checks pass. The exact capability-isolation contract passes as well.
Native-v2 removes an unused synthetic historical timestamp from the summary;
a regression proves missing source timestamps never become observation time.
`source-qualification.json` records the three late-review files. The initial
optimized full-app test build was stopped before tests because its added host
unit merely mirrored registration/serde. That test was removed; final
`desktop-v2/qualification.json` contains the authoritative compile and contract
results. No host unit-test pass or new executable is claimed.
The first three VM runs retain test-harness failures (missing-property access,
unsupported string helper and string versus identifier dictionary keys); the
corrected harness passes all50 cases. The initial packager runner failed because
copyfile omitted executable modes. Nine original modes were restored, and the
newly compiled packager successfully stamped the bundle with its four exact
capabilities. The review patch contains19 changed files.

No source promotion, PR, release activation, real account enrollment, or
original-session restart has occurred. Active graph adoption, permanent legacy
writer retirement, current project/principal mapping, reviewed creation and
node updates/cancellation, dispatch and complete E2E remain open. History
browsing does not complete MGT-045–049 or the full management migration.

## Canonical task graph transactions — native implementation

`operator-graph-transitions-v1` continues from the qualified history reader.
The original file-backed graph store and the new native canonical adapter now
use the same cancellation, node-patch, dependency advancement and dispatch
message builder. Conditions, complete structured dependency results, recorded
assignees and terminal history retain their original semantics. The file store
keeps its cooperative writer guard and counter → message → graph write order.

Schema97 adds account-scoped graphs, immutable command receipts and an outbox.
Native create/update/cancel commits graph changes, queued messages and the
original result in one SQLite transaction. Revision conflicts fail before the
transition runs. Exact command recovery precedes current-version checks, so a
lost reply or restart returns the original receipt even after later changes.
Message IDs are stable per scope/graph/node; replay does not regenerate private
message tokens. Cancellation retains message bytes and suppresses pending work.
Authority retirement, callback errors or SQLite failure roll back the whole
transaction. Source files are not reopened or rewritten by this adapter.

This outbox has only `pending_binding` and `suppressed` states. It does not send
Matrix messages, wake an agent or launch inference. The legacy graph field
`dispatched` still records a durable local intent; the native snapshot explicitly
reports that runtime delivery is not authorized. Inspection of the original
JavaScript found broadcast/push-notify effects that the retained Rust graph
file writer lacked. Actual delivery, including large structured dependency
results and human/personal/shared-agent targets, remains required for parity.

Validation passed554 distinct cases:472 full store tests,35 graph-domain tests,
41 migration-source tests,5 original HTTP graph contract tests and1 schema93
approval-upgrade regression. The12 focused journal tests are included in the
472 rather than counted twice. There are21 existing ignored store tests. All
four strict native configurations and both default/enabled desktop release-
profile compile checks pass with unchanged source snapshots. Evidence:
`.run/management46-stage/operator-graph-transitions-v1/native-v2/qualification.json`,
`desktop-v1/qualification.json` and `scoped-acceptance.json`.

Review caught and corrected arbitrary JSON results being passed through the
integer-only signed DTO encoder, and control-character IDs that would fail only
when a dependent node became ready. Regressions cover fractional conditions,
large integer/opaque result fields, Unicode IDs, nullable patches and early ID
refusal. The first integration-test compile used a private fixture constructor;
its failure remains in native-v1. Native-v2 uses the supported isolated fixture
entry point. The review patch contains38 source changes, including20 existing
downgrade-fixture adjustments; the mini-app and desktop files are unchanged.
No new UI captures or Makepad runtime validation are claimed this round.

Original four Desktop45 sessions and server32 remain alive. The original owner
binary hash is unchanged, and its database has not been upgraded. No new release,
source promotion, commit/PR or activation occurred. The next integration is
current recipient binding and reviewed mini-app operations, followed by real
stable-ID dispatch/result feedback, active legacy graph transfer and writer
retirement. Full original management parity, original-owner upgrade and real
resource enrollment/E2E remain open; MGT-045–049 remain pending.


## Reviewed task graph mini-app workflows

`operator-graph-workflows-v1` adds eight closed host services and seven EN/CN
mini-app pages for current graph listing, detail, node inspection, multi-node
creation, node updates, cancellation and saved-command review/recovery. All
writes use native prepare → review → apply-original-command-ID. Current account,
profile and package authority are native-derived. The original permission is
checked again on apply; lost responses retain the original command and receipt.
Assignee text remains a label and does not grant delivery authority.

Native result and condition JSON is retained as exact text for review/editing,
including fractional values, large integers and nested structures. Graph
requests have separately bounded larger intent storage without expanding the
legacy command limit. Read pages contain at most 16 graph/node summaries.
Recorded graph states explicitly distinguish local intent from actual delivery.
The mini-app keeps the existing Rinx theme, SVG controls and saved-command flow.

Qualification passed 542 distinct Rust cases: 492 store/management tests,
46 native graph/task/intent tests, three tests of the shipping host router and
one capability contract test. There are 21 existing ignored store tests. All
four strict native configurations and both desktop release-profile compile
configurations pass. The 214 VM checks include 136 current workflow checks and
78 retained full-root/history/task checks. Fourteen actual Metal page captures
cover seven pages in English/light and Chinese/dark. They validate layout only,
not business E2E; they precede the final callback-only text-length correction.

The Unicode regression reproduced four bilingual failures: the script string
length is UTF-8 bytes while native limits count Unicode scalar values. The
editor now uses the supported `to_chars().len()` for semantic text limits and
retains byte limits for JSON. All 136 workflow checks pass after the correction.
The failed intermediate `chars()` attempt remains recorded. Earlier fixture
and strict-check failures are retained in the scoped acceptance report.

Evidence is under `.run/management46-stage/operator-graph-workflows-v1/`:
`scoped-acceptance.json`, `reviewable-changes.patch`, `native-v3/qualification.json`,
`store-v1/qualification.json`, `desktop-v3/qualification.json`,
`ui-vm-v6/report.json` and `actual-layout-v2/report.json`. Source snapshots were
verified after qualification. This stage changes 19 native files and seven
desktop files relative to the prior candidate.

The next-stage source audit is recorded in `next-stage-review.json`. Real
recipient identity/permission binding and agent delivery/result correlation
remain unfinished. Active old graph import must retain original message IDs
and prevent redelivery; current historical labels cannot be treated as account
authority. Downlevel JavaScript ignores cooperative locks and recovers from
unreadable JSON, so an invalid-file tombstone alone cannot retire its writer.

MGT-045–049 remain pending for full parity. No new release, source promotion,
commit/PR or runtime activation occurred. The four original Desktop45 sessions,
server32 and original owner binary remain unchanged. Full original E2E,
original-owner schema upgrade and real resource enrollment remain outstanding.


## Reviewed task graph recipient binding

`operator-graph-bindings-v1` adds current-account selection of shared project
agents, personal agents/bindings and joined Matrix members. Four closed host
services expose selection, observation, current binding and reviewed binding
preparation. Native observations retain exact account, server, recipient, room
and relevant generation/state evidence. Changes require a fresh review; recovery
uses the original command and committed receipt, including after remote drift.
Schema98 commits the binding, graph revision and receipt together without
rewriting private outbox message IDs or payloads. Saving a binding does not send
work. A Matrix member selection does not certify a real-world human.

The EN/CN mini-app presents readable identities and keeps exact JSON behind
Technical details. Unbound, saved, stale, refused and uncertain states remain
distinct. Existing Rinx themes, SVG controls and saved-request recovery are used.

Scoped qualification passes 561 Rust cases: 502 store/management, 54 native
graph/task/intent, four shipping-host router and one capability-contract test.
There are 21 existing ignored store tests. All four strict native configurations
and both desktop compile configurations pass. The 360 VM checks pass; fourteen
actual Metal layout states were captured and inspected in EN/light and CN/dark.
These inert layout fixtures and loopback native tests are not real business E2E.

Qualification exposed a native stack overflow after adding optional recipient
resolution to graph commands. Boxing the large nested futures fixed it without
increasing stack limits; all 54 native cases then passed. Two default-feature
test-gating errors were also corrected. Original failed reports are preserved.
The final change after desktop/native qualification consists only of four cfg
attributes on test fixture fields/initializers; final default strict passes.

Evidence: `.run/management46-stage/operator-graph-bindings-v1/scoped-acceptance.json`,
`reviewable-changes.patch` and `delivery-next-review.json`. The latter maps six
delivery implementation groups and eighteen recovery/security tests. Actual
transport, recipient crypto, execution/result/cancellation correlation, active
legacy adoption and the other management-family gaps remain unfinished.
MGT-045–049 remain pending for full parity. No new release, source promotion,
commit/PR or activation occurred; original Desktop45 sessions and server32 remain
running with the original owner binary unchanged.
