# Shared-agent status and remaining management parity

This checkpoint adds a functioning read-only status page after an agent request.
It does not activate execution, provider access, adoption or additional server
flags. Agent approval belongs to the assigned project administrator; there is
no second resource-owner approval.

## Implemented contract

`hagency.agents.status {id}` requires `ReadAgentRequests` and `ReadAgentStatus`.
The native `management_server/agent_status.rs` adapter reads the exact request,
then its fixed `/assignment`, `/matrix` and `/runtime` routes. Each stage retains
its own observation timestamp. These reads can observe legitimate advancement;
the projection does not claim one atomic snapshot. Exact immutable request/agent
identity is checked, and SDK/profile/package authority is checked after every
HTTP await and before returning. Private receipts, digests and executor tokens
are excluded from the host projection.

Only a documented policy-disabled 503 becomes “Not enabled on this server”. An
explicit successful null is “No stage result recorded”. Network, authorization
and malformed-response failures clear the old status and remain errors. Refresh
and Back keep the original request and do not enter the command journal.

The EN/CN page shows approval, reservation, Matrix membership and qualification.
Qualification and a historical `processState:live` observation are not current
online/work evidence. Requested tokens are not measured consumption or remaining
capacity. The project page now describes actual setup/admission discovery rather
than always claiming these interfaces are unavailable.

## Original functions and exact remaining work

| Original local function | Available shared-agent contract | Next complete UI slice |
| --- | --- | --- |
| `console/agent_detail.rs`: agent/resource/room/current dispatch/recent tasks | The three stage reads above expose reservation and recorded setup; no live dispatch | Read-only status and request-scoped work history implemented; add separately authorized response content only if a real contract is added |
| `console/usage.rs`: engagements, allocation/spent/remaining totals; native `quota_status` | Schema13 shared-work query/index returns per-call observations; old qualification remains metadata only | Bounded history/read-only usage view implemented below; aggregate resource totals and top-ups remain separate |
| Native `approve_coordinator_top_up` / `raise_allocation` | No shared-agent top-up request/decision/receipt protocol | Manager requests a delta for the same agent and period; assigned project admin decides; native exact revision/account/allocation checks reserve the delta once; only accepted receipt updates capacity, unknown retains original command/hold |
| Native `control_coordinator_agent`, `settle_coordinator_agent`, `retry_cleanup`; local `agents.rs` removal/undelete | Manager cancellation exists; assigned admin `POST agent-requests/{id}/revoke` accepts original `commandId` and `expectedRevision`; cleanup is separate | Add projected `canRevoke` plus durable native command/recovery and explicit confirmation. Show revocation, runtime stop, Matrix removal and final settlement separately; do not call cancellation a refund or expose legacy undelete |

History must be scoped to the shared request/project and existing manager/admin
visibility. It must not fall back to a personal ledger or assume a coordinator
can read another project's work. A list needs bounded pagination, explicit
unknown/partial usage, stable work identity, and separately authorized result
content. The backend owner is implementing the first shared text-work protocol;
the history checkpoint below consumes its bounded read-only projection.

## Evidence and limits

- Four real loopback HTTP/SDK tests: client `.run/agent-status-native-http.log`.
  They cover independent stage advancement, safe projection, disabled/no-record
  versus errors, SDK retirement after each read, grants and malformed identity.
- 24 pinned-VM status/navigation/discovery-copy checks in English and Chinese:
  Desktop `.run/agent-status-vm-final`; existing 36 admission checks also pass.
- Shared native strict Clippy: client `.run/adoption-grant-handoff-clippy.log`.
- Desktop closed-input/permission/manifest test passes with `resource-adoption`:
  `.run/agent-status-desktop-host-final.log` (one test; existing native adoption
  entrypoint dead-code and macOS linker unwind warnings remain).
- Parser, package, catalog and localization checks pass for the new bundle.

These are protocol, authority and script-object results. The new status page has
not yet been accepted in a rebuilt native window. There is no new UX score or
claim that the full shared agent lifecycle is finished. Real capture acceptance
must use a real request; it must not seed fabricated native capacity.

## Schema13 work-history checkpoint

`hagency.agents.work.list {projectId?,after?,limit?}` and
`hagency.agents.work {id,after?,limit?}` require only the distinct native
`ReadAgentWork` grant. Both limits are 1–16 (default 16). Their closed fixed routes
are POST `agent-work/list` and `agent-requests/{id}/work/query`; neither is a work
submission and neither enters the command journal. The server independently
checks the permanent project manager or **current assigned** project administrator.
Resource ownership, coordination or server administration alone grants no history
access. The native adapter rechecks the SDK/profile/package after every HTTP read.

The index includes only requests with actual shared-work records, with public
request/project IDs and project/agent names. Visibility and project filtering
precede pagination. It is discoverable from Overview after restart, and from a
project with an exact project filter. The `shared-work-history-v1` feature is
independent of admission/setup/reservation issuance; disabling new agent requests
must not hide past work. Existing status `/runtime` remains unchanged and is not
an alias for the newer `/execution` observation.

The native typed projection excludes prompts, response bodies, executor tokens
and receipts. It bounds page length/IDs/display fields and exact integer values,
checks cursor advancement, and preserves null timestamps and usage. Host-rendered
UTC labels come from the same numeric timestamps. First/Next and Refresh replace
a bounded page; they do not accumulate an unbounded timeline or replay a command.
Pages are ordered by opaque record/request ID, not presented as chronological.

Each call shows recorded work state separately from Matrix reply delivery.
`usage:null` is unmeasured; zero is shown only when explicitly observed. A completed
response may have incomplete usage. `retainedTokens` includes known consumed
amounts: complete consumption of 17 tokens retains 17 in accounting, not zero.
Unknown retained accounting stays unknown. The UI never adds per-call values into
resource free capacity and never calls a historical started/completed record a
currently running agent. Auth/network failures clear prior displayed results and
remain errors; policy-disabled/old servers have explicit unsupported copy.

Evidence in this checkpoint:

- Client `.run/native-agent-work-tests-final.log`: five real loopback HTTP/SDK tests,
  covering index/query paging, separate grants, SDK loss, null/partial/complete
  usage, secret stripping, invalid/cyclic cursors and no journal/effect writes.
- Desktop `.run/agent-work-vm/work-final`: 36 pinned-VM actual subtree/callback tests
  in English/Chinese; previous status24 and admission36 also pass.
- Desktop `.run/agent-work-desktop-host-passed.log`: two closed host/grant/date tests pass.
- Desktop Clippy exits0 with no errors or warnings in the owned history adapter;
  `.run/agent-work-desktop-clippy-scope.json` records436 warnings in inherited,
  dependency or other concurrent source. This is not a repository-wide strict-clean claim.
- `tools/hagency/native_management_agent_work.py` is an authored read-only native
  probe (hidden/actual empty/actual record). It has not been executed for this
  checkpoint. It never creates requests/work/rooms, changes flags/accounts or
  restarts a process. Full-window captures and the unchanged UX gate remain root
  integration work; synthetic VM objects are not native screenshots.

Remaining parity includes manager top-up requests, assigned-admin top-up decisions,
revocation/removal with distinct native stop/Matrix cleanup/settlement results,
authorized reply content, and separately observed finite activity (`/execution`).
No execution, adoption or server policy is activated by this read-only checkpoint.

## Next dependency after this history checkpoint

The missing usable bridge is the coordinator allocation screen: a server-approved
project already creates a `project_allocation` request when an eligible published
parent grant exists. The coordinator must review that exact allocation and its
period, then approve/reject through the central revision-checked decision. An
approval queues native work; it is not a reservation. The existing native
`AllocationTransport` can apply the reservation through the actual owner ledger,
but no trusted adopted-owner runner is activated by a Splash service. This needs
a separate native executor handoff; mapping `ExecuteAllocations` into a bundle
permission would bypass that boundary.

Current `GET projects/{id}/allocation` picks the highest opaque ID, so Inbox
selection must gain an exact allocation-ID read instead of treating that result
as the chosen request. Grant remaining/total needs an explicit observed
projection; a historical parent receipt or central pending request cannot prove
current native available capacity. After that usable flow, assigned-admin revoke
is the next bounded existing write contract. Top-up remains a separate new
request/decision/native-delta protocol.
