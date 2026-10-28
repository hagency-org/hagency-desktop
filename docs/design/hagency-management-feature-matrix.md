# Hagency management feature-family audit

**Historical backend inventory; current web-port status is separate.** This
2026-10-19 family audit includes retained native APIs that the served web console
did not expose. Its operation counts, pending records and checkpoint45 release
references are historical, not today's frontend completion checklist.

See the [2026-10-20 web-client implementation matrix](hagency-web-client-parity.md)
for the actual served web pages, implemented Desktop controls, legacy-only
semantics and completed macOS native-window checks. Focused checks pass: 9 host,
14 native, 19 store, 96 public EN/CN callbacks, 34 connection callbacks and
4 private exports. Publication/retirement regressions also pass, including the
shared-account sibling case. Actual desktop UI verifies the console reads,
connection probe/create/edit, saved-change cancellation, trust changes, catalogue
publication and unused configuration retirement. The matrix links exact reports
and states the legacy/runtime and cross-platform limits. This does not
retroactively accept every retained backend operation below. New coding-agent
engines and graph execution remain outside the user's web-client port.

The [progress log](hagency-management-progress.md) preserves the later real
four-role onboarding/reply E2E and source promotion; checkpoint45 is no longer a
current-release claim. Historical rows below retain their original evidence
rather than being relabelled as new full-migration acceptance.

Historical audit date: 2026-10-19 (integrated execution policy, workspace recovery and stopped-task candidates). This compares the retained Rust console, original JavaScript UI/API, current native owner services and current mini-app source. It is not release acceptance.

The [inventory](hagency-management-parity.json) has **104 retained-native operations, 189 JavaScript entries and 70 owner commands**. They overlap, so these are not 363 independent features. MGT-064 has scoped macOS acceptance; the other 103 records remain pending. This is an acceptance inventory, not a percentage of implemented code. The Tasks records now separately map eight operations with actual native verification in an adopted empty-history profile; assignee navigation has source/VM evidence and Board has limited-projection evidence. Integration28 additionally accepts a populated synthetic history: three original tasks/two comments, one later edit/comment and exact restart preservation. Original rich Board behavior and populated project/engagement migration remain gaps. These scoped checks do not establish full original parity.

Baselines: `hagency-rs@1212833a7bfe3b17f6f1d69fc8d528aca6ac0f36`, `hagency@4a8a8ac25e43e645345fc25987261e0d7a644fcd`, and `hagency-client@39e990c8d483d6bd29b4135742f486e7b83ad137`. The exact Rust checkout and JavaScript UI at those revisions were inspected. Links below point to retained working sources; inventory entries preserve exact original routes/revisions.

**Implemented** means production source and a concrete mini-app action exist. **Read-only** means only the stated projection is exposed. **Missing** means the original function lacks a service, screen or suitable authority protocol. **Unverified** means the complete actual native flow has not passed; default-disabled code, simulated providers and VM tests are not live acceptance.

## All 104 retained-native operations

Every MGT identifier belongs to exactly one row below. Current behavior and remaining original behavior are separate.

| Inventory | Retained family/source | Current mini-app | Missing or unverified original behavior |
|---|---|---|---|
| MGT-001–005 | [Provider accounts](../../../hagency-client/native/hagency/src/console/accounts.rs) | Trusted provider connection/onboarding,34 inventory/detail,35 private account-space preparation,36 native login and37 selected-account model inspection exist. Actual EN/CN review, authorization start/cancel, signed-out model checks, refresh and normal restart pass. | Populated original namespace/import acceptance, retirement/enrollment, completed real authentication and logout remain unfinished. An empty namespace is not authentication or capacity. Evidence: `.run/management37-stage/checkpoint.json`. Locally activated38 implements selected-account enrollment, original namespace execution/provider verification and the EN/CN native-review wizard. Four-role activation, signed-out checks and restart pass; authenticated enrollment acceptance and real login remain open. Locally accepted39 adds native-confirmed unused-account retirement, original-result recovery and actual EN/CN cancel/retire/refresh/restart. Bound-account drain/retirement and provider logout remain open. |
| MGT-006–011 | [Agent extras](../../../hagency-client/native/hagency/src/console/agent_extras.rs) | No equivalent actions. | Launch environment, undelete, avatar, delivery events, message detail/suppression. Work history has no message bodies. |
| MGT-012–024 | [Lifecycle / uncertain dispatch](../../../hagency-client/native/hagency/src/console/agents.rs) | Managed request/decision, stage views and same-assignment successors are implemented; integration30 accepts bounded native revoke/refresh/restart and Matrix room exit. | Stopped-dispatch recovery now passes native32 plus strict, real-widget2/host67/grant4/release and actual EN/CN inspect, choose, confirm, cancel, all three decisions and restart (`stopped-recovery-v1/scoped-acceptance.json`). Continue retains the task/session/workspace and queues the remaining instruction. This uses synthetic offline attempts; original populated history and full lifecycle acceptance remain open. Original preset/start/stop/refuse remain open. Actual27b same-thread recall and new-thread isolation pass with the mini-app closed on an already approved validation agent. Durable intake/demand issuance are enabled on the isolated server18. Integration33 accepts post-fence accounting for the already stopped validation agent. Termination of a live provider during cleanup, offline outage/recovery and full original lifecycle parity remain unverified. |
| MGT-025–026 | [Alerts](../../../hagency-client/native/hagency/src/console/alerts.rs) | Resource-scoped history, details, notes and original-command triage accepted in EN/CN and restart (40). Embedded observer41 passes actual Desktop startup with the mini app closed. Explicit v3 history transfer42 preserves original alerts/notes/receipts and accepts continued handling plus restart in a synthetic native fixture. | Richer metadata, arbitrary operator assignment/suppression, real-provider acceptance and Matrix notification delivery remain open. |
| MGT-027–028 | [Approval bindings](../../../hagency-client/native/hagency/src/console/approval_bindings.rs) | Candidate46 implements resource-scoped inspection, exact agent/room review, atomic removal and saved-grant revocation. EN/light and CN/dark list, cancel, remove and restart pass in actual Metal windows; eight captures reviewed. Evidence: `.run/management46-stage/approval-binding-scoped-acceptance.json`. | Agent/project filters, pagination, scroll reset, cancel and restart pass in actual EN/CN Metal windows (`approval-binding-filter-actual-v3`). The two label-width corrections pass the layout-only release and EN/CN native-window recheck; all eight captures are inspected (`approval-binding-filter-scoped-acceptance.json`). Populated original approval-history transfer and full original-flow acceptance remain open. Source is staged; original running sessions remain45. |
| MGT-029–031 | [Tool approvals](../../../hagency-client/native/hagency/src/console/approvals.rs) | Candidate46 implements scoped request status, readable saved rules and exact reviewed revocation/recovery. Actual EN/CN pagination, cancel/revoke/refresh/restart pass. Corrected expiration labels and agent/project context, request details, saved-rule navigation and opening the project-filtered native approval inbox also pass actual native-window checks; captures were inspected. Evidence: `tool-approval-actual-v2` and `tool-request-context-*` under `.run/management46-stage/`. | The synthetic project has no joined approval room: inbox navigation proves the destination, not selection or decision of an actual private request. Populated original history transfer, real-provider tool execution and the complete private approval workflow remain unaccepted. Original running sessions remain45. |
| MGT-032–033 | Server associations (original `associations.rs`; absent from current client tree) | Resource admission/verification/reconnect implemented. | Original association/import/credential semantics; adopted reconnect and original grant publication now pass isolated native UI checks; original import/credential parity remains pending. |
| MGT-034–041 | [Engagement / settlement](../../../hagency-client/native/hagency/src/console/engagements.rs) | New coordinator allocation and cleanup have isolated coverage. | Original candidates/audit/settlement UI and cleanup retry. New receipts do not prove every legacy transition. |
| MGT-042–044 | [Execution policy / workspace](../../../hagency-client/native/hagency/src/console/exec_policy.rs) | Candidate46 scoped agent permissions pass actual EN/CN save, cancel, refresh and restart. Resource default editing/creation now pass native121, strict, Desktop63+4+release and actual EN/CN windows (`execution-policy-default-editor-v1/scoped-acceptance.json`). Read-only execution now also passes production host selection, managed/unmanaged cold processes and consumed warm runtime: selector17, owned55, warm11 and strict (`execution-readonly-selection-v1/qualification-v2`). | Integrated policy desktop passes63 host,4 grant UI checks, release and actual EN/CN windows; all8 captures inspected (`execution-policy-integration-v1`). Workspace recovery passes69 native, strict, Desktop65+4+release and actual EN/CN empty-state navigation. The integrated stopped-recovery candidate also passes populated quarantine refusal, exact-task navigation, one idle workspace release after review/cancel, and restart. All18 combined recovery captures were inspected; one command receipt and matching release timestamp were saved without replay. Original populated approval/policy history transfer, full UX and real-provider policy acceptance remain open. Shared managed text-only agents retain their separate contract. Candidate sources are not promoted to original sessions. |
| MGT-045–049 | [Task graphs](../../../hagency-client/native/hagency/src/console/graphs.rs) | Staged `operator-graph-bindings-v1` retains reviewed graph workflows and adds current shared/personal/Matrix-member selection, exact native observations and atomic reviewed binding. 561 Rust cases, 360 VM checks, 14 inspected Metal layout states, four strict native configurations and both desktop checks pass. | Actual delivery, recipient crypto, execution/results/cancellation, active original graph adoption, legacy-writer retirement and full business E2E remain open. Binding alone does not send work. No new release, source promotion or activation. |
| MGT-050–051 | [Matrix invites](../../../hagency-client/native/hagency/src/console/invites.rs) | Original project Space and room invitations accepted through native Rinx in the actual four-role flow; setup verifies membership. | Retained console invite semantics and rejection/recovery parity remain unaccepted. The selected-invitation widget correction is integrated in live22; retained rejection/recovery parity still needs acceptance. |
| MGT-052–054 | [Matrix diagnostics](../../../hagency-client/native/hagency/src/console/matrix_diag.rs) | Scoped connection/setup verification shown. | Explicit probe/reach/callback diagnostics and retained refusals. |
| MGT-055–057 | [Offer book](../../../hagency-client/native/hagency/src/console/offer_book.rs) | Admitted-resource catalog, capability filter and detail are read-only. | Original previews, role counts/tiers and contribution parity. Catalog does not claim physical free capacity. |
| MGT-058 | [Registration import](../../../hagency-client/native/hagency/src/console/palpo_import.rs) | Native source adoption retains original resources/journals; actual migration passed. | External registration import is not equivalent to adoption. No synthetic legacy registration for managed execution. |
| MGT-059–060 | [Project sides](../../../hagency-client/native/hagency/src/console/project_sides.rs) | Project requests/list/detail/server-admin decisions implemented. | Original side configuration/edit. Approval is neither allocation nor a created Matrix project. |
| MGT-061–062 | [Resource configuration](../../../hagency-client/native/hagency/src/console/resource_configuration.rs) | Read-only detail and local naming accepted. Checkpoint43 adds durable creation/first-offer provenance and native model/reasoning/resource-ceiling editing for positively proven never-offered resources; EN/CN cancel/save and exact lost-reply restart pass while account, seat and allocations are retained. | Admitted-resource amendment, broader capability editing and full original configuration parity remain unfinished. Legacy rows without creation proof stay blocked; a local name does not amend a published definition. |
| MGT-063–066 | [Resources/budget/publication](../../../hagency-client/native/hagency/src/console/resources.rs) | Read-only list/detail/filter; remaining/total native budget and exact ID disambiguation. Checkpoint45 additionally qualifies [new configurations](hagency-additional-resource-configuration.md) with separate native review, preserved source/shared account quota, EN/CN actual-window creation and original-command lost-reply restart recovery. MGT-064 has scoped macOS acceptance. Isolated checkpoint46 also accepts native quota publication, server withdrawal, zero-consumption canonical closure, fresh re-admission/new grant and EN/CN restart with the original history retained. | Admitted-resource amendment, daily cap, reusable definitions and actual nonzero-liability closure remain unfinished. Native unpublish is not server withdrawal; full resource-family parity is not accepted. |
| MGT-067–071 | Coordinator engagements (original `server_engagements.rs`; absent from current client tree) | Allocation read/decision/release UI and native runner source. Actual coordinator approval and54321 native reservation within the original100000 owner grant pass. | Original delegation/resource APIs and release/recovery parity remain distinct and unaccepted. No second owner approval per project/agent. |
| MGT-072–074 | [Local setup](../../../hagency-client/native/hagency/src/console/setup.rs) | Onboarding/adoption and separate native grant review implemented. | Actual migration and original grant review/publication passed; the complete project/agent lifecycle is still under validation. Fresh provider/platform coverage and original offer creation remain open. |
| MGT-075–076 | [Side allocation/budget](../../../hagency-client/native/hagency/src/console/side_budget.rs) | Explicit-period parent/project allocation projections. | Original side allocation semantics unaccepted. Original totals are not current physical free capacity. |
| MGT-077–084 | [Side/project lifecycle](../../../hagency-client/native/hagency/src/console/side_lifecycle.rs) | Setup readiness/assignment/begin/continue/cancel/rebase source with original journals. Actual original Space/room creation, administrator invitations, service join and verification passed. | Old credential/archive/deactivate/reactivate/delete missing. Cancel cannot erase an in-flight returned room. Full original lifecycle parity remains unaccepted. |
| MGT-085 | [Registration file](../../../hagency-client/native/hagency/src/console/side_registration.rs) | Owner-only connection download exists. | Original side registration-file parity unaccepted. Secrets stay outside chat/script projections. |
| MGT-086–088 | [Streams](../../../hagency-client/native/hagency/src/console/stream.rs) | Explicit refresh and saved command recovery. | Stream/snapshot/events, notification delivery/reminders/retries. |
| MGT-089–098 | [Tasks / board](../../../hagency-client/native/hagency/src/console/tasks.rs) | Integration24 exposes original task/board reads. Canonical84 supports exact proven-empty adopted histories; the original owner passed actual native read/ledger-preservation checks. | Full original parity remains unaccepted; integration28 imports a populated synthetic task history through actual native review and preserves exact IDs, fields and comments. Actual25a CRUD/accept/transition/comments pass in a proven-empty adopted history; assignment/filter edge cases remain separately qualified. Actual26a adds exact-assignee filtering, first-viewport save and12 consecutive edits with terminal acknowledgement. Actual28 populated task import/edit/comment/restart now passes. Rich Board behavior, populated project/engagement migration and full UX acceptance remain open. Model calls are not operator tasks. |
| MGT-099–101 | [Engagement usage](../../../hagency-client/native/hagency/src/console/usage.rs) | Resource budgets and nullable/partial per-call usage. | Original reports/totals and full resource→project→agent usage. Completed work does not imply complete usage or a refund. |
| MGT-102–104 | [Runtime detection](../../../hagency-client/native/hagency/src/fleet_views.rs) | Native provider detection/connection and resource capability projections. | Full framework matrix and each original adapter's version/refusal compatibility need explicit evidence. |

## Evidence for the newer flow

| Slice | Source / evidence | Acceptance boundary |
|---|---|---|
| Resource detail | [host](../../src/host/hagency/resources.rs), [probe](../../tools/hagency/native_management_resource_detail.py); `.run/management-resource-detail-native15-en` | Actual exact-ID/detail/refresh/Back passed. No resource editing acceptance. |
| Project plans/setup | [host](../../src/host/hagency/setup.rs), [probe](../../tools/hagency/native_management_project_setup.py); `.run/project21f-original-grant`, `.run/project21f-matrix-created`, `.run/project21f-native-invite-acceptance3`, `.run/project21f-matrix-confirm` | Actual server-admin project approval, assigned-admin membership, original Space/room creation and service verification passed. Original archive/deactivation/deletion parity remains missing. |
| Agent admission/status | [native admission](../../../hagency-client/native/hagency/src/console/native/management_server/agents.rs), [status](../../../hagency-client/native/hagency/src/console/native/management_server/agent_status.rs); `.run/project21f-agent-request2`, `.run/project21f-agent-work-send` | Actual manager request and assigned-project-admin approval passed without a second owner approval. Original reservation, puppet membership and actual process qualification observed. Integration23 plus schema15 now pass original same-assignment Start, closed-miniapp renewal, one real provider call, exact Matrix event and recipient thread rendering. Complete cleanup remains unverified. Narrow actual27b same-thread recall/new-thread isolation passes, without claiming complete offline lifecycle coverage. Actual usage4813 exceeds allocated4096; full cost is retained and next Start is refused, but there is no hard provider cutoff. |
| Adoption / grant review | [native activation](../../../hagency-client/native/hagency/src/console/native/resource_activation.rs), [native review](../../src/miniapps/grant_ui.rs); `.run/onboarding18-migration`, `.run/onboarding21e-reconnect`, `.run/onboarding21e-grant2` | Actual migration, two original-resource reconnects and exactly-once100000 grant publication passed. Earlier failures are retained; no complete shared-agent execution claim. |
| Coordinator allocation | [native adapter](../../../hagency-client/native/hagency/src/console/native/management_server/allocations.rs), [host](../../src/host/hagency/allocations.rs); `.run/project21f-original-allocation3` | Actual review/cancel, coordinator decision, original native reservation, assigned administrator and manager readiness pass four checks. Release/recovery and retained original API parity remain unaccepted. |
| Work history | [native adapter](../../../hagency-client/native/hagency/src/console/native/management_server/agent_work.rs), [VM tests](../../apps/hagency/tests/check_agent_work.py) | Bounded index/history and nullable usage tested. No task-board/graph/original-message parity. |
| Actual shared reply | `.run/project23-original-start-after-tickets`, `.run/project23-agent-work-result`, `.run/project23-agent-reply-event`, `.run/project23-recipient-ui`, `.run/project23-exhausted-review` | One real provider turn and exact recipient rendering passed. Budget is admission accounting:4096 allocated,4813 consumed/retained,0 remaining; next review refuses Start. No continuous-chat, hard token ceiling or top-up acceptance. |
| Tasks read and precise process review | `.run/management24-qualification`, `.run/management24-native-read-and-state2`, `.run/integration24-restart-*` | Exact composition:20 focused native/store,34 Desktop/catalog and12 VM cases passed. Four actual owner UI checks and four original-session restarts passed; no new work/call/Start intent. Empty-history read only; Tasks/board layout defects and full UX gate remain open. |
| Tasks writers | `.run/management25a-native-task-lifecycle`, `.run/management25a-qualification`, `.run/integration25a-restart-*` | Seven actual native checks passed, temporary task/comment removed and original provider ledger unchanged. Four original sessions preserved. Later integration28 accepts bounded populated task import/edit/comment/restart. Full original-operation acceptance and9/10 UX remain open. |
| Populated task migration | `.run/onboarding-task-history28/acceptance.json`, `.run/onboarding-task-history28/restart28/report.json` | Native27 + Desktop5 checks and release build pass. Actual EN/CN native wizard, original3-task/2-comment import, one edit/comment, pagination, parent navigation and restart pass. Exact original archives/provenance retained; no capacity grant or provider work. This is a synthetic local-history fixture, not populated project/engagement or rich Board parity. |
| Continuous context | `.run/project27b-context-transport-resumed/report.json`, `.run/project27b-context-recipient/report.json` | Two distinct new real-provider turns pass same-thread recall/new-thread isolation with mini-app closed; native/server digests agree and actual recipient threads were viewed. Approved24,000-token validation agent totals10,978 consumed/13,022 remaining. Original exhausted agent and cancelled event unchanged. No full outage/recovery or top-up acceptance. |
| Agent top-up | `.run/project29-acceptance/acceptance.json`, `.run/project29-topup-ui-accepted/report.json`, `.run/project29-topup-provider/report.json`, `.run/project29-topup-restart/report.json` | Actual EN manager request/CN assigned-admin approval, single4096 native delta and one real-provider v3 turn with mini-app closed pass. Native recipient and updated usage are visible. Normal restart preserves one delta/four calls without replay. Effective28096, consumed16161, ledger remaining11935; original exhausted agent untouched. Central UI remaining telemetry, full block/recovery provider acceptance and overall UX remain open. |
| Cleanup | [design](hagency-agent-cleanup-ui.md), [probe](../../tools/hagency/native_management_agent_cleanup.py); `.run/project30-cleanup-ui-accepted/acceptance.json` | Actual EN manager/CN assigned-admin review/cancel, one revoke, refresh and normal restart pass, with twelve reviewed native Metal captures and independently observed Matrix `leave`. Both grants had already expired and all processes had stopped. Top-up-aware retained floor28096 and known usage16161 remain intact. Integration33 subsequently accepts final accounting for this already stopped agent; live-process termination remains open. |
| Post-fence settlement | [design](hagency-agent-settlement-next.md); `.run/management32-stage` | Immutable native proof, SDK-authenticated HTTP transport and Desktop background polling are implemented and promoted after native/paired Desktop checks. Central schema21, finite tickets, role-bound routes, conservative latest-proof accounting and new-evidence rechecks are qualified in isolated source. Native498 ordinary tests pass. Server346 broad cases pass; one expiry-test timing assumption is corrected and passes separately (347 covered across runs). Both paired release builds pass. Isolated33 adds the typed human view/recheck, original-intent recovery and EN/CN states: native501 ordinary cases,21 management store checks,31 settlement VM cases,43 cleanup VM cases and2 full bundle roots pass. Paired Desktop checks and release pass. Checkpoint33 accepted the flow with server32/schema21; all four profiles now run34, whose read-only EN/CN settlement regression also passes. Actual EN/light and CN/dark single-request pending/complete views and normal restart pass: ledger125, known/retained16161, unresolved0, original fence/calls/top-up unchanged. Evidence: `.run/management33-stage/checkpoint.json` and `acceptance-restart-observe/report.json`. No new provider call or whole-app UX/platform acceptance. |
| Personal-agent settings | [bounded scope](hagency-personal-agent-settings.md), [native writer](../../../hagency-client/native/hagency/src/console/native/personal_agent_policy.rs), [VM tests](../../apps/hagency/tests/check_personal_settings.py) | Original personal read/explicit device-observation/policy-edit source, loopback recovery/CAS tests and EN/CN VM checks. Scoped actual native budget observation/review/cancel/save/re-observation and EN/light, CN/dark restart now pass (`.run/management46-stage/personal-policy-scoped-acceptance.json`): one successful policy write, revision1, 250 tokens monthly. The original label-locator failure remains recorded; read-only continuation proves persistence without resubmission. Actual lost-reply/stale-CAS acceptance, model/workspace editing, custom-rule editor and shared settings remain missing; candidate46 clears the obsolete post-save observation notice and passes54 VM checks; actual post-save notice revalidation remains pending. |
| First device setup | [scope and evidence](hagency-fresh-device-setup.md); `.run/management44-stage/empty-setup-build`, `.run/management44-stage/empty-heading-build` | Distinct native empty-device setup, status/reopen integration and account-preparation gating implemented. Actual EN/CN cancel, setup, one offline-provider account/resource enrollment and process restart pass. Final CN/dark setup uses no personal seeding flag or preliminary legacy resource; populated checkpoint43 fixture reopens with canonical records and provider calls unchanged. Strict native checks, 11 activation tests, 49 host and 12 adoption tests (overlapping), 84 account/32 onboarding VM cases and both bundle roots pass. Source is promoted; all four original sessions now run45 with exact SDK/device identities retained. Read-only original-settlement regression passes. Real OAuth persistence and whole-app/platform acceptance remain open. |

These are current source checkpoints, not a new release claim. Working source after immutable19 includes peer runtime/reconnect work. The server stack-overflow regression was subsequently fixed by the root agent; its final231-test suite and strict lint passed. Earlier failed evidence is retained and is not replaced by focused-test claims. The unchanged 9/10 UX gate also remains open: coordinator readiness scored7.87 in release15; newer layouts need fresh native captures.

## Settings implementation and remaining work

1. **Read-only personal settings implemented in source:** use existing `Command::Agents` and `AgentConfigurationStatus` only. The central list filters `execution_kind='personal'`. Keep a separate Personal agents route and exact agent generation/device matching. Show observed model/reasoning/web-search, configured-workspace boolean, request/tool policy, policy revision, budget/usage and observation age. No credential references/filesystem paths. Plain refresh excludes `ExecutionEnvironment`: it submits a remote configuration request despite its name. A separately reviewed `personal_agents.observe` operation now journals an original environment request explicitly; its output excludes credentials/paths.
2. **Personal policy writes implemented in source:** exact original device/generation/idempotency custody, execution-device revision CAS and an explicit mini-app review. Native actual E2E remains pending. Custom rules require explicit reviewed replacement; no implicit rewrite. `LocalAction::AgentPolicy` opens personal SQLite; adopted archives must not be reopened. `PrepareAgentWorkspace` creates state. Never expose generic Command/URL/path/credential inputs. A cached observation is not a current authorization.
3. **Managed settings:** require a distinct versioned definition/policy amendment. Approved managed definitions currently fix tools disabled; personal APIs must not target shared identities. Template edits cannot silently rewrite approved agents.
4. **Top-up:** integration29 now has an append-only native delta, assigned-project-admin approval and collection within the original parent grant/period, with bounded actual UI/provider/restart acceptance above. Original reservations and consumed usage stay unchanged. Legacy `raise_allocation` is not this protocol. Central remaining telemetry, actual budget-blocked event recovery and full parity are still open; cleanup remains separate from refunds.

Owner anchors: `OWN-001–003`, `OWN-041–050`, `OWN-059`. Sources: [native configuration](../../../hagency-client/native/hagency/src/console/native/configuration.rs), [personal owner policy](../../../hagency-client/native/hagency/src/console/owned_agents.rs), [existing Desktop editor](../../src/hagency/ui.rs), [server configuration](../../../hagency-server/crates/agent-service/src/execution_configuration.rs). Original `ExecutionPermissions.jsx` includes YOLO, persisted execution grants and revocation; `AgentTabs.jsx` deliberately shows unavailable live activity instead of joining sample logs to real agents.

Resource editing separately needs durable contribution provenance and amendment/re-admission: revisions hash configuration/publication, central contributions pin the old revision and permanently key owner/device/resource. Paginated absence or scrubbed journal history cannot prove never-contributed, including other profiles/servers. Keep resource deletion, provider retirement, native unpublish and server withdrawal separate.

Original acceptance still requires exact operation/refusal semantics, current-role success/wrong-role denial, stale-CAS checks, original-ID lost-reply/restart recovery where applicable, and actual full-window EN/CN light/dark rendering. This family audit does not change those records.


## Resource re-admission foundation candidate46

The running release remains45 and the local server remains32. Candidate46 is
staged; it has not changed the original sessions or their accounting. Schema22
adds immutable per-admission resource snapshots, exact old-request visibility,
owner/device/revision-checked resubmission after rejection, expiry or withdrawal,
and mandatory recorded native parent closure for every previous grant. A new
submission requires a new administrator decision. Retained liabilities remain on
the original grants. Legacy snapshots are explicitly marked `legacy_current`.

Earlier server evidence: `hagency-server/.run/management46-stage/qualification-v6`.
That full run passed355 cases and failed one older ACL assertion that expected an
empty successful page for a later coordinator. The correct response hides the
old project with `project_request_not_found`. Only that test assertion changed;
production-source hashes remained identical. All4 allocation UI cases and8
re-admission cases then passed, as did strict all-target Clippy and the release
build. The failed run is retained. After adding the unpublished-legacy-receipt
regression and admission guard below, `qualification-v7` passed all357 cases in
one fresh isolated-database run, strict all-target Clippy and the release build,
with unchanged source hashes. The running server was not replaced.

The native transport passes18 focused cases and both feature configurations'
strict checks (`.run/management46-stage/native-v2`). The desktop bridge passes51
host tests (`host-v1`); these preceded the two later internal enrollment changes.
The mini app does not yet declare or expose the new services.

Native provider observation now checks the exact admission request and both
revisions before returning a proof. Its HTTP regression failed before the fix
and passed afterward (`native-admission-v3`). New enrollment journals also retain
the original admission request across restarts. Legacy records remain byte-identical
and may continue only at their original revision2 approval; a later admission
cannot silently adopt their identity. Thirteen enrollment and20 grant/recovery
cases and both strict configurations pass (`native-admission-v5`). Only these
two native enrollment source files were promoted to the Client working tree;
verification in that main source tree is recorded separately in
`.run/management46-native-admission-promotion`. Running sessions remain45.

Explicit new-round consent, old-grant fencing, resource amendment and actual
mini-app acceptance remain required. This foundation does not close resource
editing or publication parity. Concrete findings and next steps are in
`.run/management46-stage/next-work.json`.


## 2026-10-19: Original-admission grant confirmation and cancellation

New native grant intents persist the original admission request identity without
rewriting legacy digests. Prepare, re-review and confirmation check the current
authenticated admission; a cached confirmation cannot reserve capacity after its
request or observed revisions change. Re-review may refresh revisions of the same
admission while keeping the original command, grant and intent IDs. Legacy
unbound journals remain limited to the initial revision2 approval.

The native cancellation API separately reviews and closes only an unattempted
intent. It works after withdrawal, owner reopen and accounting-period expiry
without provider login. Closing its review has no effect. Applying/unknown and
recorded grants retain their original recovery path and receipts; cancellation
cannot refund a hold or reinterpret an unknown outcome. Desktop now exposes a
separate cancellation review and explicit confirmation; closing the review keeps
the original pending. The five Desktop source files have been promoted locally;
running sessions still use45.

The stale-confirmation regression failed before the fix. The final candidate
and promoted main source each pass the same62 grant/enrollment/publication tests
and default/enabled strict Clippy checks; these counts overlap, not124 distinct
cases. Earlier runs retained one test-child stack overflow. The connection
fixture now uses a separate executor task as Desktop does, keeping the ordinary
stack limit and all assertions. These checks use synthetic provider/Matrix
authentication, actual loopback HTTP and SQLite, NativeOwner reopen, and existing
subprocess cases; they are not real-provider OAuth or full mini-app E2E.

Evidence: Desktop `.run/management46-stage/native-consent-v3/qualification.json`
and `.run/management46-consent-promotion/main-v1/qualification.json`; selected
source hashes/backups are in that promotion directory. Running desktop45 and
server32 were not replaced. Explicit new-round receipts, local old-grant fencing,
resource amendment and the mini-app workflow remain unfinished. In particular,
a current resource hash must not stand in for an original admission identity
when publishing a previously unpublished v1 receipt after re-admission.

The server regression reproduced that exact old-receipt acceptance before the
fix. Candidate46 now rejects a new v1 publication once the contribution has a
prior admission request, while retaining original-command and existing-receipt
recovery. This is an interim restriction: legitimate new-round publications also
need the unfinished versioned, admission-bound protocol. Evidence is server
`.run/management46-stage/admission-receipt-regression-v1` and `qualification-v7`.

Desktop candidate qualification passes53 host tests, four grant UI tests
(including12 layout combinations), and a release build. Actual macOS Metal
windows in isolated profiles verify: owner admission and separate administrator
approval; closing both reviews without changing the original; keyboard-confirmed
cancellation; explicit new limits retaining the cancelled original; and Chinese
dark-mode cancellation after restart and synthetic provider sign-out without a
provider call. Original intent fields and the selected canonical resource/capacity
tables remain exact. The provider is an offline metadata fixture; this does not
validate real OAuth or inference. Earlier harness failures are retained: the
passing keyboard run waits for each Tab frame before Enter. Long details exceed
the first viewport, so this is not whole-app UX or a9/10 acceptance.

Evidence: `.run/management46-stage/consent-ui-v3/qualification.json`,
`consent-actual-ui-v2/report.json` (three passing setup/close checks before its
harness failure), `consent-actual-ui-v5/report.json` and `visual-review.json`.
The five-file promotion, backups and main-composition check are recorded under
`.run/management46-consent-ui-promotion`; candidate and main checks must remain
separate because other staged re-admission changes have not been promoted.

The promoted Desktop composition now passes52 host tests and four grant UI tests
against the main Client dependencies. Evidence:
`.run/management46-consent-ui-promotion/main-v1/qualification.json`. This is a
source/test qualification; the running application was not replaced.

Candidate46 additionally implements admission-bound v2 grant receipts, preserving
the bytes and recovery of original v1 receipts. A new enrollment appends a distinct
admission entry. Its atomic capacity writer requires committed closure evidence
for every earlier grant and continues charging retained child liabilities. The
private journal must independently reconcile every original intent; a missing
receipt for an attempted operation remains unknown. The native protocol passes
291 resource tests (37 ignored),66 grant-family tests and default/enabled strict
checks in `.run/management46-stage/round-native-v2/qualification.json`. These are
overlapping scoped tests, not acceptance of all original management functions.

The next candidate adds this reconciliation before a reoffer sends HTTP, and
refuses new grant preparation when server allocation support is disabled.
`round-native-v3` passes68 grant-family tests and291 resource tests (37 ignored).
Its adopted-management test confirms that a
pending original prevents HTTP, explicit cancellation permits a separate new
submit, and replay of the refused original keeps its original result. These
tests overlap the v2 suite. The later v4 strict check exposed stale check-profile
metadata: copying prepared source with its old modification time let Cargo reuse
the earlier interface. The test-profile metadata contains the new methods and
the functional tests exercised them; the v3 enabled strict result needs separate
revalidation. The diagnosis is retained in `round-native-v4/cache-diagnosis.json`
and `round-native-v3/verification-correction.json`. A final confirmation-time
policy recheck covering both v1 and v2 originals, with refreshed source mtimes,
passes68 grant-family tests and both strict configurations in `round-native-v5`.
Fresh check metadata confirms the new APIs; the earlier correction remains recorded.

Server `qualification-v8` passes359 tests with no failures/ignored cases,
strict Clippy and release compilation. Paired native/server acceptance remains
pending. Desktop must still distinguish closed historical grants from the
current admission, expose the complete withdraw/reoffer/amendment flow and pass
actual-window/restart acceptance. The v2 protocol has not been promoted or
activated, and real OAuth persistence remains unverified.


Candidate46 now also projects canonical per-grant closure without changing the
original Recorded intent or receipt. Desktop selection keeps current pending,
unknown or active originals ahead of closed history, and separately checks new
admission approval before offering new choices. The view distinguishes retained
amounts, confirmed consumption and unresolved liability. These source changes
are staged only. Resource-store validation passes293 cases (37 ignored), both
strict native configurations pass,34 EN/CN onboarding VM cases and both full
bundle root checks pass, and the bundle is restamped against the Desktop host's
pinned contract. The first native grant-family run passed67 and failed one
existing recovery regression: a valid immutable closure with a restored legacy
active bit must remain readable. Descriptive lookup now keeps the committed
settlement authoritative while new-admission/reoffer retain their independent
active-bit checks. The original failure is retained in `closed-native-v1`; the
paired Desktop build was interrupted before changing its dependency. The fix is
verified in `closed-native-v2`: the targeted regression,293 resource tests
(37 ignored),68 grant-family tests and both strict configurations pass.
`closed-desktop-v2` passes55 host and4 grant-UI tests; its intermediate release
was stopped to combine the completed history/reoffer UI into one candidate.
Actual window and paired re-admission/restart acceptance are pending. Real OAuth
credential persistence and full original-operation parity remain unverified.

Candidate46 history/reoffer is implemented in the embedded EN/CN mini app.
Server `qualification-v10` passes all359 tests, strict Clippy and release build;
Desktop `readmission-desktop-v1` passes55 host tests,4 grant-UI tests and release
build. The v9 feature-list and database-clock test failures remain recorded.
The first actual owner/admin flow on isolated loopback8388 passes resource
setup, original approval, owner verification, restart and read-only admission
history (`readmission-actual-ui-v1/setup-complete.json`). Original profiles and
running server32 are unchanged. The provider supplies offline metadata only.

The actual withdrawal/reoffer probe then found a production parsing bug:
`connections.get` returns `{admission, observedAtMs}` but the new page required
an absent `protocol` field. Its VM fixture had repeated that incorrect field.
With the real wire shape,24 of48 earlier cases fail; correcting the parser and
adding missing/mismatched admission checks gives54 of54 passing cases and two
full-root checks (`readmission-wire-fix`). The bundle is restamped. The release containing this parser fix built successfully. The next native
attempt (`readmission-actual-ui-v2`) exposed an additional form-render failure:
the two new TextInputs used the Button-only `enabled` property. They now use
TextInput's documented `is_read_only`, preserving the existing callback guards.
The updated56-case VM suite and two full roots pass (`readmission-input-fix`);
release v3 is qualified. Both actual failures are retained. The actual v3 probe
submits one new admission, injects a private reply-write failure after the server
commit and recovers the same command after restart. Its final visible-text
assertion misses the second record below the viewport. Continuation v4 uses
native scrolling, confirms both original history records after another restart
and approves only the second request through the separate administrator session.
This accepts the no-grant readmission/recovery flow in this isolated fixture,
not the closed-old-grant → new-grant flow or full migration.

The next actual capacity review (`readmission-grant-actual-v1`) exposes a first
grant bootstrap deadlock: session discovery advertises admission-bound issuance
only when project capacity already exists, while native grant preparation needs
that discovery before creating capacity. The candidate separates the explicit
issuance policy from live project availability; both server and native focused
regressions reproduce the failure before the fix and pass afterward. Native
qualification passes69 grant-family cases and strict checks in both configurations.
Server v11 passes274 management cases,3 API discovery cases, strict Clippy and
release compilation; the earlier v10 full359-case run remains separate evidence.
Desktop passes the real reoffer-input widget regression,55 host cases and4 grant
UI cases and its paired release passes. The isolated installation retains the
same database and owner/admin profiles. Actual native-window probes now pass
first publication, withdrawal and canonical zero-consumption closure, a fresh
third admission with separate administrator approval and a distinct admission-bound
grant, and normal restart without duplicate records. Chinese/dark read-only
restart also passes. Evidence: `readmission-grants-scoped-acceptance.json`.
The original closed receipt and new active grant remain separate. Actual
nonzero-liability closure, real provider inference and full migration are outside
this scoped acceptance. Reopening a published grant still initially shows a
local reservation; a separate read-only status lookup is under qualification.

The installed Codex passes `codex-keychain-recheck-v1`: reads only a fresh empty
owner keyring and reopens signed out. The separate `codex-keyring-write-v1`
probe now also writes a dummy API-key-shaped value through Codex's keyring-only
storage in a fresh private CODEX_HOME, reads it after process restart, logs out
and confirms another restart is signed out. Outbound networking is denied by
the macOS sandbox; no real credential, auth.json fallback or inference is used.
This verifies local keyring write/read/delete, not real OAuth token persistence.
The existing checkpoint45 owner session uses the same qualified provider-home
code. Its expired original attempt4 has been recovered and acknowledged, and
fresh attempt5 has opened the actual OpenAI account-selection page in Safari
(`real-owner-login-v3`). Attempt5 subsequently expired; real OAuth persistence
remains unverified. Read-only inspection confirms the protected original business
records are unchanged; no resource/budget mutation or inference occurred. Admitted-resource
amendment, original missing management functions and full migration acceptance
remain required.

Real provider login update: `real-owner-persistence-v2/scoped-acceptance.json` verifies actual OAuth and two fresh private provider-process catalog reads on owner45. It does not verify real resource enrollment. The current catalog returns `gpt-6.1-sol` and `gpt-6-astra`; the embedded Codex role policy recognizes only `gpt-5.6-sol`, so enrollment remains blocked by qualification. Keep this account signed in.

The current model catalog gap is resolved in staged `current-codex-models-v1`:
explicit Sol/Astra qualification, native55, VM44 and desktop15 checks pass;
the release builds and its read-only actual startup preserves all fixture data.
The model and enrollment views were inspected in EN/light and CN/dark.
Original sessions still run Desktop45; real-account resource enrollment and
the schema93-to95 desktop upgrade remain unverified. No new OAuth is needed.
