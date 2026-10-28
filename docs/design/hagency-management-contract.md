# Embedded Hagency management contract

Date: 2026-10-16
Status: Accepted implementation direction; implementation and acceptance pending.

The authorized migration plan is `rinx/docs/design/hagency-management-miniapp-migration.md`.
Hagency Desktop supplies one Rinx client, one embedded Rust engine, and one
OctoScript management application. No local management web server or second
Hagency sign-in is required. Central Matrix/Pasion/Appservice deployment remains.

## Decisions

1. Preserve the personal-agent model and restore shared resource contribution.
   An agent's human owner is permanent. An execution assignment delegates a
   particular agent/project/Room to an enrolled resource executor; it does not
   make the executor the human owner or share that owner's login credentials.
2. Reuse the original native resource ledger and coordinator contract. Import
   reviewed changes from `hagency-rs@1212833a` into the current client, retaining
   its new owner engine. Do not introduce a second local allocation ledger.
3. Keep the existing personal-device foreign key and authorization path. Add
   explicit shared-execution authorization, exercised by both application checks
   and database constraints. A shared assignment must be consulted when claiming
   work, authorizing tools, replying, stopping, revoking and cleaning up.
4. Central admission belongs to the designated server administrator. Allocation
   belongs to the resource's explicitly delegated coordinator under the configured
   project policy. Project managers request capacity; membership or Room power
   alone never confers allocation authority. The executor automatically enforces
   the accepted grant; it must not ask the human resource owner for the same
   allocation approval a second time.
   Within an allocated project, agent admission belongs to the administrator
   assigned to that project, as requested by the user. Resource owners and
   coordinators do not approve each agent again. Legacy coordinator agent methods
   need an explicit authority adapter plus automatic capacity checks; retaining
   their implementation must not restore the superseded human approval flow.
5. Use typed Rust management services directly. HTTP console handlers become
   adapters to those services. Preserve original command/recovery receipts when
   extracting personal-agent services; calling the central API directly is not
   equivalent to the local command-custody protocol.
6. SDK login uses the existing signed-in Matrix session and opens no callback
   socket. Provider credential and workspace selection flows remain trusted host
   operations. No provider credential, Matrix refresh token, Appservice secret,
   unrestricted command or caller-selected URL is exposed to Splash.
7. The mini app uses exact versioned service grants and bounded closed requests.
   The host binds account epoch, package digest/instance and device generation;
   the script cannot choose its actor. Expected object revisions and idempotency
   keys are required for durable writes. Recheck authorization at the writer and
   suppress private replies after account/package revocation.
8. Reuse `im.palpo.operations` for continuity only with the new management contract
   feature and verified built-in bundle digest. Old Palpo capabilities do not
   acquire new authority. Preserve account-scoped deep links and drafts through
   an explicit versioned migration. Do not remove the catalog ban globally.
9. Work continues after closing the mini-app window. Account switching retires
   the selected owner service until independent executor enrollment is implemented.
   Quit drains/stops execution with visible status. Background execution must
   have an independently revocable enrollment, never a hidden human session.
10. Existing native management shortcuts deep-link into one canonical bundle.
    Shared theme tokens, SVG icons, English/Chinese and desktop separate-window
    behavior apply to every new screen. Article editor code is outside scope.

## Wire and custody rules

The first project implementation is an approval plan. A signed-in project
manager selects admitted resource metadata and requests a monthly token amount.
Only the configured server administrator can approve or reject that plan. The
result `approved` plus `allocationStatus: awaiting_allocation` grants no executor
capacity, creates no Matrix project/agent/binding and starts no provider process.
The resource coordinator can inspect assigned plans but cannot approve project
admission merely because they coordinate the selected resource. Withdrawal is
available to the requesting manager or server administrator. Resource withdrawal
closes dependent pending/approved plans in the same central transaction.

Servers advertise `project-requests-v1`; old clients continue to receive only
resource admissions unless they opt into project Inbox records. Catalog metadata
omits private provider credentials, device/resource identifiers and unconfirmed
token balances. The admission wizard explicitly explains its server-local
discovery visibility. Local resource publication and central catalog admission
remain different operations until a versioned reconciliation protocol exists.

Project mutations use separate exact capabilities and journal operation names.
Recovery returns the latest currently visible plan using the original mutation
grant, including a later rejection or withdrawal. It cannot restore revoked
authority. Inbox records identify the actual project and resource, and opening an
old card resolves the current revision before offering a decision.

The new protocol is `hagency-management-v1`; service families are
`hagency.session`, `hagency.providers`, `hagency.resources`,
`hagency.connections`, `hagency.projects`, `hagency.agents`, `hagency.inbox`,
`hagency.work`, and `hagency.activity`. Individual allowed service names are
declared in the shared application contract, not matched by a prefix.

Native host context is non-serializable. Script requests contain only operation
arguments and opaque references. Responses contain bounded projections and
separate capability/availability information. Unsupported operations return an
explicit unavailable result; they are not substituted with successful mock data.

Provider connection uses the exact `hagency.providers.connect` grant plus a
native confirmation. The request accepts only a supported runtime. The host
shows its own account/runtime identity and consumes a non-serializable, expiring
confirmation once. Script-supplied approval flags, account IDs, paths, URLs and
credentials are rejected. Back/Cancel performs no provider change. A queued
acceptance rechecks scope and expiry before calling the original native provider
service. Connecting associates an already signed-in local provider; fresh
provider login, workspace selection and remote-device connection remain separate
implementation work. A successful connection does not publish or start anything.

Mutation identity binds operation, owner, installation, target, payload digest
and expected revisions. Authorization also binds the current account/package
scope and executor generation; reauthorization does not erase an earlier command
identity. Explicit execution grants and verification observations retain their
original executor generation. Repeating the same key reads the original result;
a different payload conflicts. Queue acceptance is not successful application.
Pending, claimed, succeeded, failed, outcome-unknown, expired and revoked states
remain distinguishable.

The native SQLite journal takes custody before polling an effect. It retains at
most eight active commands per owner/installation and keeps acknowledged identity
and digest tombstones. Splash drafts are convenience state, not durable custody.
The exact pending/retry/acknowledge services return only bounded summaries; retry
uses native saved arguments and checks the original operation capability again.
Unknown outcomes cannot be acknowledged. A recorded native resource receipt can
repair a lost UI reply without asking the provider to repeat its effect. Central
recovery reads the same owner's same-installation receipt under fresh server
authorization and resolves the latest visible admission. It cannot renew a
verification or transfer executor authority. A new validation failure after an
uncertain attempt does not by itself establish that the earlier attempt failed.

Central decision and outbox publication commit together. Executor reservation
and result receipts commit before acknowledgement. Lost acknowledgement causes
lookup/reconfirmation, never a second reservation or new agent. Revocation denies
new work immediately and retains cleanup until the executor confirms termination.

Connection verification is a separate acknowledgement after admission. Only the
original owner's enrolled executor device and generation may send it. The native
adapter checks the current provider catalog/authentication and the original local
resource revision, runtime, model, reasoning and provider binding. The request
contains stable identifiers and revisions, never script-asserted readiness or
provider credentials. The central server trusts the enrolled executor's report;
this is not remote hardware attestation or proof of a successful model turn.
Verification expires after five minutes and becomes stale on revocation or
binding/revision changes. Replaying its receipt does not renew that deadline;
clients read the latest object after replay. Verification grants no publication,
reservation, allocation or execution authority.

A restarted installation may explicitly reconnect through
`hagency.connections.resume`. Fresh native provider/resource proof, current
owner/device authorization and the unchanged admitted resource revision are
required. This advances the connection and request revisions, so an old approval
form must refresh. It preserves the original request and any completed admission
decision; it neither renews verification nor transfers execution grants. Expired,
rejected and revoked requests cannot be reopened by reconnecting. Another device
of the same owner cannot claim this operation. After an approved connection is
reattached, a separate verification is still required.

For settled capacity, total = consumed + reserved unused + available. Unknown
usage retains the hold. Retirement refunds only measured unused reservation.
Resources shared across server engagements reserve against the same local ledger.
The central catalog reports observation age and never invents current capacity.

Inbox is durable independent of notifications. Opening or dismissing a card does
not decide it. Old links resolve the current object/revision. Reminder delivery
stops on a decision or expiry; delivery failures remain visible and retryable.

## Delivery and validation

The action inventory in `hagency-management-parity.json` is an implementation
checklist. `pending` means neither implemented nor verified in the mini app.
Restoring domain code or passing an old console test does not change that status.
Original JavaScript-only actions are separately reconciled against the retained
native inventory before any full-parity claim.

Validation uses an isolated real server on this MacBook, distinct administrator,
resource owner/coordinator and project manager accounts, actual provider turns,
restarts, lost replies, concurrent capacity requests and unauthorized attempts.
No production user data is migrated during development validation.

The real production Splash bundle is driven with Makepad instrumentation.
Capture entire native windows, inspect them, and run the existing UX scorer
unchanged. Retain every attempt, raw scores, image/scorer hashes and functional
checks. Each representative new screen must reach 9/10 in required language and
theme states. Passing the pixel score alone does not prove usable workflows.

Desktop E2E is the requested initial device gate. Android/OpenHarmony management,
suspended notifications and device-specific behavior remain explicit release
gates; desktop evidence must not be relabeled as physical mobile validation.
