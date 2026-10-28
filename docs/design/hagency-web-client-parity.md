# Hagency web-client → Desktop parity

Implementation checkpoint: 2026-10-20. The identified web-client control gaps
now have Desktop screens, host routes and native implementations. The matrix
below records the supported behavior and its limits; it does not claim that every
legacy backend integration is active. Actual macOS Makepad-window checks now
cover the console reads, server probe, settings changes, catalogue publication
and unused-configuration retirement listed below. The macOS follow-up also
completed actual system-dialog import/export, live provider-model observation,
retained-input refresh and populated-data checks. New coding-agent execution was
not part of this work.

## Scope and baseline

The task is to port the existing Hagency web client's user-facing functions into
Hagency Desktop and its management mini-app, without requiring a local web
server. Existing first-class native Desktop controls count as available;
creating coding-agent engines or new telemetry is outside this port.

The original served console is the Next.js application in `hagency/mockup/`,
despite that directory's name. See its [parent README](../../../hagency/README.md),
[console README](../../../hagency/mockup/README.md) and
[navigation](../../../hagency/mockup/components/Rail.jsx). Checked upstream
`4a8a8ac25e43e645345fc25987261e0d7a644fcd` and local
`5dbef22dc5ad4e0bb1a886538406ec91a5893f9b` have the same `mockup` tree,
`0da651b6f4e52956bb57f1b8c47fb7292db93729`.

This matrix follows rendered pages and controls, not unused API routes. The
historical 104 Rust operations / 189 JavaScript entries / 70 owner commands are a
broader backend inventory, not the completion denominator. Source links below
refer to the real working trees, including the latest uncommitted implementation.

## Implemented feature matrix

**Implemented** means a reachable control and its native path exist. **Adapted**
means the original purpose is retained under the agreed signed-in management
model, with the difference stated. **Limited** identifies actual unavailable
behavior rather than treating a placeholder as a successful integration.

All new mini-app pages and their EN/CN labels are in the
[management bundle](../../apps/hagency/bundle/main.splash). Shared host
capabilities are declared in its [manifest](../../apps/hagency/bundle/manifest.json).

| Web surface / control | Desktop status and implementation | Supported behavior and limits |
|---|---|---|
| Resource inventory, configuration details and provider/model/reasoning setup | Implemented: existing Resources and onboarding wizard; [host integration](../../src/host/hagency.rs). | Names, new configurations, native provider authorization, declared limits, remaining/used amounts and saved-command recovery remain available. Editing an admitted immutable definition still uses its applicable amendment/re-admission flow; naming alone does not change it. |
| Resource catalogue publication / withdrawal | Implemented: resource-detail actions in the bundle invoke the existing publish service and explicit review. | Local catalogue publication, server contribution admission/withdrawal, profile retirement and provider-account retirement remain separate operations. The adopted store permits changes only on an unused configuration with one owner: its active capacity grants, unsettled children, legacy engagements or another active owner block a definition change. An unused sibling configuration on the same provider account remains editable. Standing offers govern new reservations without changing those pinned definitions. |
| Standing offers and capability qualification | Implemented: Capability and offer editor; [catalogue native adapter](../../../hagency-client/native/hagency/src/console/native/web_catalog.rs), [overview adapter](../../../hagency-client/native/hagency/src/console/native/web_overview.rs). | Role, count, per-engagement cap, declared rate and publication are editable. Qualification shows actual scoped model/framework/reasoning/family/tier data, not authorization to execute. New reservations enforce withdrawal, count and per-engagement cap as described below. |
| Resource daily rate and future-agent permission defaults | Implemented: catalogue settings and exact reviewed saves; [catalogue store](../../../hagency-client/native/hagency-store/src/domain/owner_resources/adoption/management/web_catalog.rs). | Daily rate is a declared term, explicitly not a newly implemented daily execution quota. Future defaults can be changed independently of publication; only first provisioning inherits them. Existing agents and approved policy snapshots are unchanged. |
| Delete a resource configuration / assign a preset | Adapted: Retire and Assign preset actions; [typed catalogue commands](../../../hagency-client/native/hagency-store/src/management/web_catalog.rs). | Retire removes this profile's access to an unused configuration while retaining receipts/history and other owners' access. Assignment uses the original binding transaction only for a pending unconfigured engagement; it does not rewrite an active agent or delete the shared provider account. |
| Shared subscription/account capacity | Implemented: Accounts overview and account/resource pagination; [account projection](../../../hagency-client/native/hagency/src/console/native/web_overview/accounts.rs). | Shows member resources, one shared quota, committed/remaining capacity, declared totals by period and oversubscription. Unknown quota stays unknown. A partial member page is explicitly labelled; its totals are not presented as the complete account. |
| Matrix connection, resource admission, projects and agent approval | Implemented: existing signed-in wizard, role-specific Inbox and project/agent management. | Server administrator approves projects; coordinator allocation and assigned project-administrator agent approval retain their distinct authority. The earlier four-role real-reply E2E remains scoped evidence, not a fresh acceptance of every new page. |
| External-server probe and connection maintenance | Implemented with compatibility limits: [host routes](../../src/host/hagency/web_connections.rs), [native sheets](../../src/miniapps/web_connections_ui.rs), [native maintenance](../../../hagency-client/native/hagency/src/console/native/web_connections/maintenance.rs). | Create/edit connection, probe, registration issue/reissue/export/import, credential replacement/withdrawal and representative verification are exposed. Secret files use native dialogs, not script/chat payloads. Imported legacy machine credentials do not start a Fleet runtime. See the applicability section. |
| Existing project-side allowance | Adapted: reviewed connection allowance edit and explicit applicability fields; [connection types](../../../hagency-client/native/hagency-store/src/management/web_connections.rs). | Retained as legacy-side configuration. It is not enforced by an inactive Fleet transport and does not limit canonical resource-pool allocations; the original web also exempted resource-pool requests. |
| Engagement allocation choice | Adapted: review the exact project-defined resource/model/reasoning/agent request and its approved allocation. | The original project-defined branch also hides its legacy candidate selector. No additional Hagency-owner approval or arbitrary substitution is introduced. Legacy non-project-defined candidate selection is not activated in this signed-in flow. |
| Trusted projects | Adapted: owner-scoped remembered trust list, add/remove/review; [catalogue implementation](../../../hagency-client/native/hagency-store/src/domain/owner_resources/adoption/management/web_catalog.rs). | Trust is remembered preference, not automatic server approval. It cannot bypass the server or assigned project administrator. |
| Workforce | Implemented: consolidated roster; [workforce projection](../../../hagency-client/native/hagency/src/console/native/web_overview/workforce.rs). | Joins agent → project → allocation → recorded usage → resource/account when exact retained bindings exist. It includes owner-local children even when the owner cannot list central project requests. Rows without central access do not offer unauthorized project-management navigation. Missing bindings remain explicit. |
| Usage dashboard | Implemented: period summaries, resource usage bars, fresh/cache-read tokens, recorded project-agent calls and allocation totals; [usage projection](../../../hagency-client/native/hagency/src/console/native/web_overview/usage.rs). | Uses existing measurements only. Fleet scans are bounded to 256 authorized resources with a completeness flag; agent accounting is separately bounded and displayed. Different periods are not combined as comparable totals. Agent-call accounting is not added to resource transcript usage twice. Busy time, task counts and historical series remain unavailable when unrecorded. |
| Global alerts and handling | Implemented: fleet counts, status/severity/agent-resource filters, paginated list, original detail/notes/transitions; [scoped alert query](../../../hagency-client/native/hagency-store/src/domain/ceiling_alerts/management.rs). | Counts cover authorized resources independently of the selected filter. Existing retained records and handling receipts remain authoritative. Alert deletion is not added; the original web deliberately refused it. |
| Terminal/activity pane | Limited: authenticated Terminal navigation and explicit unsupported state; [terminal projection](../../../hagency-client/native/hagency/src/console/native/web_overview.rs). | Current managed agents are headless and expose no existing tmux pane, so no terminal capture is claimed. Their work records remain available. No shell, pane launch or fabricated output was added to fill this gap. |
| Agent model/workspace/rules and execution permissions | Existing native controls retained: [Agents UI](../../src/hagency/ui.rs), mini-app personal settings, policy/tool-grant review and revocation. | Porting these controls does not extend the managed execution engine. The native ready-account → model enrollment/review/apply → fresh-model refusal → restart/recovery path passes with a metadata-only synthetic provider. This verifies the management path, not OAuth/inference or every provider/framework. |
| Invitations, messages, formatted content and replies | Existing native Matrix UI and HTTP(S) tabbed reader retained. | Ordinary invitation acceptance, chat, attachments and replies are available. Old backend-rendered `/msg/:id` routing passed 3/3 tests: exact original URL/query and view-token preservation, distinct tabs for identical IDs on different archive origins, and no native read authority inferred from originless IDs. The original archive server and its authorization remain required. |
| Theme and language | Existing shared theme plus EN/CN labels on new pages. | EN/CN callback/render checks pass. Actual macOS windows were captured and inspected; this is not every-platform theme acceptance. |

## Semantics that must not be conflated

### Offers affect future reservations

The [prospective catalogue rules](../../../hagency-client/native/hagency-store/src/domain/web_catalog.rs)
filter this owner's withdrawn roles and run inside the existing
[managed reservation transaction](../../../hagency-client/native/hagency-store/src/domain/owner_resources/capacity/managed_agents/reserve.rs).
A withdrawn offer refuses a new reservation; the per-engagement cap and count of
reserved, unfenced same-role children also constrain new reservations. Released
or fenced children do not consume that count. Original success/refusal receipts
are replayed before mutable offer checks, and existing children are not silently
fenced. Updating a remote admitted catalogue still requires explicit Reoffer.
Resource and offer daily rates remain declared terms, not a new daily meter.

### Imported credentials do not activate legacy Fleet execution

Native verification checks the representative with the actual imported/live
credential, including the existing pending-to-live fallback. Registration
custody, export and verification are functional controls; they do not establish
that the old machine poll/ack/update transport is running. The public connection
projection states `managementAuthority: signed_in_matrix` and
`importedTransportActive: false`.

Likewise, `sideBudgetFor()` in the original
[backend](../../../hagency/backend-v2.js) computes committed usage with
`legacyOnly: true`; its resource-pool request path explicitly bypasses that side
allowance. Desktop therefore states `allowanceScope: legacy_fleet_requests`,
`allowanceEnforced: false` and `resourcePoolAllocationsAffected: false`. Applying
this legacy allowance to the new resource-pool flow would change the original
semantics, not finish a missing frontend control.

### Project definitions and administrator decisions remain authoritative

The original [AgentAllocationChoice](../../../hagency/mockup/components/AgentAllocationChoice.jsx)
returns the project-defined resource/model/reasoning review before its legacy
candidate selector. Current project-defined requests retain that behavior.
Remembered trust cannot restore the old auto-approval shortcut: the user's
required server/project-administrator decision remains necessary.

## Validation checkpoint and compatibility limits

| Evidence | Result / scope |
|---|---|
| Desktop host route checks | 9 passed with the final closed command/cancellation routes: `.run/web-parity/desktop-tests-final.log`. |
| Legacy `/msg/:id` reader compatibility | 3/3 passed through headless Makepad action routing and the actual tab controller, including open/close of same-ID messages on different archive origins. Exact URL/query capabilities are retained. See [tests](../../src/shared/web_browser_message_links.rs), `.run/legacy-message-link-tests.log` and `.run/legacy-message-link-review.json`. This does not claim live archive-server availability. |
| Native web adapters | 14 passed including exact-model qualification and cancellation/lost-reply recovery: `hagency-client/.run/web-overview-validation/native-web-final-cancel-qualification.log`. |
| Store web behavior | 19 passed including atomic cancellation and prospective offer gates: `hagency-client/.run/web-overview-validation/store-web-final-cancel-qualification.log`. |
| Public mini-app callbacks | 96 passed, including saved-change cancellation, recoverable publication refusal and EN/CN rendering: `.run/web-parity/renderer-final-capacity/report.json`. |
| Connection EN/CN checks | 34 passed: `.run/web-parity/connections-final-2/report.json`; both full bundle roots also passed. |
| Private native export checks | 4 passed through the Desktop host test binary. The configuration-reader bound, encoding and before/after authority checks also pass: `.run/web-refresh-validation/native-configuration-reader.log`. |
| Page/focus/timer refresh | 84 EN/CN VM cases and 3 native lifecycle tests pass. Exact filters/cursors and retained native input/pending replies survive refresh; stale replies and hidden-page updates are rejected. `.run/web-refresh-validation/{callbacks-v3/report.json,native-lifecycle-v4.log}`. |
| Populated older accounts | Native fixture passes with 36 resources and 18 private account groups, retained offer terms, pagination, ownership, redaction and restart. Compatibility projection does not mutate legacy records. |
| Supported populated adoption | Five store history tests pass, with a stronger combined-v3 rerun preserving exact resource configuration, 21 tasks, 19 comments, alerts/notes, paging and restart. Explicit adoption is bounded; arbitrary historical tables are not silently migrated. `hagency-client/.run/web-overview-validation/store-history-v3-final-v2.log`. |
| Ready provider management | Native metadata-only provider fixture passes enrollment, exact model/reasoning apply with preserved quota, changed-model refusal and restart/receipt recovery. `hagency-client/.run/web-overview-validation/native-ready-model-final.log`. |
| Updated EN/CN catalogue UI | 100/100 VM checks pass, including legacy read-only capability terms and editable adopted catalogue controls. |
| Publication and retirement regressions | Publication 3, native adopted/lost-journal recovery 1, and legacy publication 1 passed. The final same-account sibling regression and catalogue subset passed 3 / 10: `hagency-client/.run/web-overview-validation/{store-publication-sibling-v1,store-catalog-after-sibling-v1}.log`. |
| Earlier actual macOS native windows | Release `274c6c1e3b8034ec9b8dbc16356fa47ea32d5c8525cac5fae6084222173f785e` built and restarted successfully, preserving the original account/device and schema 99. Scoped console reads, daily-rate save/reload and cancellation, trust add/remove, actual Matrix probe, connection creation/allowance edit, publication/withdrawal and unused-configuration retirement passed. See reports below. |
| Final macOS acceptance follow-up | Release `39d46d834cea819f32557218d8756aa910fd0b4116408c33402e87ab35fb123a` built and runs with the original active account/device. Actual Open/Cancel, selected JSON import and native confirmation, registration issue, Save/Cancel, private YAML export and credential withdrawal passed. Live signed-in Codex metadata and model setup/cancellation passed. Unsaved native text survived the refresh interval. `.run/web-parity/acceptance-followup/validation.json`. |

Actual UI evidence under `.run/web-parity/`:

- `activation-scoped-publication/report.json`: final release activation, same
  active account/device, consistent database backups and integrity checks.
- `console-final-scoped/report.json`: five console data views and the external
  connection wizard use actual native replies, with Metal PNG captures.
- `publication-final-v2/report.json`: withdraw, republish and retire the unused
  zero-capacity configuration, even while another configuration on the same
  provider account has active agents. Historical resource data remains. All
  existing capacity-grant and managed-agent rows stayed byte-identical.
- `native-v4/{daily-rate-roundtrip,trust-roundtrip,connection-roundtrip}.json`:
  save/reload a declared daily rate, remember/remove trust, probe the real local
  Matrix server, create its connection and edit the legacy allowance through
  native confirmation.
- `native-final/mutations-report.json`: the first check passed cancellation of
  a saved rate change and then exposed the old publication route. Its failed
  result is preserved. `publication-final/report.json` records the subsequent
  overly broad shared-account refusal. The final regression and actual-window
  result above address both failures; neither history was relabelled as passing.

The usage capacity bar is labelled **Capacity used / limit**, because its
existing figure includes reservations; it is not presented as consumed tokens.
Measured token and managed-agent call usage remain separately labelled.

The tests use an isolated local Matrix server and the existing provider account;
this is not a mini1 deployment. The instrument SDK lacks its optional
user-activity guard and reports that limitation in the trace. The macOS
follow-up used Accessibility to select a synthetic connection file and operate
the real Open/Save panels. Import required native confirmation. Registration
export contained the expected YAML fields and callback, used mode `0600`, and
was kept in the private test directory. The temporary home-directory import copy
was removed and the disposable connection's credentials were withdrawn.
Expired unconfirmed reviews were rejected normally.

The macOS picker now schedules `NSOpenPanel` directly through the same
`NSOperationQueue` helper as the native save panel, preserving worker-thread
bounded reads and current-owner checks. This avoids the vendor picker's extra
GCD dispatch; the earlier modal stack alone was not proof of a deadlock.

Follow-up evidence is under `.run/web-parity/acceptance-followup/`:

- `activation-final/report.json`: original account/device retained, stopped-role
  database backups and integrity checks, schema 99.
- `dialog-checks.json`: actual import/export, cancellation, expiry and cleanup.
- `provider-checks.json`: real retained Codex login observation and eligible
  models; resource setup opened and cancelled without a new configuration.
  This did not repeat OAuth or issue inference. Model apply and refusal/recovery
  are covered separately by the metadata-only native provider fixture.
- `refresh-live.json`: an unsaved connection allowance remained intact beyond
  one 15-second interval; no command was created, and leaving/reopening the page
  confirmed that the draft was not saved.
- `integrity-final.json`: allocations and agent definitions stayed identical.
  Strict whole-row identity did **not** pass: startup applied an already-issued
  server `agent.fence` command to one previously reserved test agent. The server
  command was issued at 03:34:15 UTC, before the 03:37:58 baseline and 03:43:27
  new-release startup. Existing worker code prioritizes pending fences. Its
  settlement remains unresolved, with 5,644 known consumed tokens and all 8,000
  allocated tokens retained because process/usage completion is unknown. The
  fence was not reversed and no capacity was refunded. This lifecycle event is
  recorded separately from the frontend checks.

The final resource and provider screenshots were captured from the actual
Makepad windows. Every-provider OAuth/inference acceptance is not claimed.

Old `/msg/:id` links are backend archive links, not a Next management route.
The [original bridge](../../../hagency/bridge-matrix.js#L9828) already sends the
full formatted body into Matrix and adds the archive URL. The existing reader
preserves that URL and its per-message capability; the
[archive backend](../../../hagency/backend-v2.js#L17050) still requires its
[original view-token, agent or API authorization](../../../hagency/backend-v2.js#L4462).
Unavailable localhost 8090 or retired 8084 archives cannot be reconstructed
from opaque message IDs, and no current Matrix credentials are injected.

Remaining compatibility limits are explicit: imported Fleet transport is
inactive and current managed agents have no terminal capture. Page/focus/
15-second refresh now has the VM and retained-native-widget evidence above,
based on the original [Data.jsx](../../../hagency/mockup/components/Data.jsx).
Populated old-owner reads and bounded v3 adoption have passed; arbitrary
historical database migration and every provider/framework live acceptance are
not implied. Other-platform acceptance is outside the current macOS scope.
Unknown measurements are never zeros.

## Outside this frontend port

The current served web client has no reachable task-graph editor, task board,
report/export generator or live oversight engine. Its old work/messages/repos
tabs were removed; fixture activity and unavailable live oversight in
[AgentTabs.jsx](../../../hagency/mockup/components/AgentTabs.jsx) are not rendered
production features to port.

New coding-agent engines, shared workspace/tool runtime architecture, graph
delivery, new notification/reminder systems, full native-CLI parity and blanket
historical-database migration are separate projects. Existing registration-file
import and current data visibility are included above. Template CRUD already
added is extra functionality, not a substitute for catalogue controls. The old
broader inventory and multi-day estimate do not determine this port's status.
