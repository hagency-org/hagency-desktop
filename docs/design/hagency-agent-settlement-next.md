# Agent settlement after removal

Status: integration32 native core/HTTP transport, central persistence/routes and
the paired Desktop worker are qualified. Integration33 adds the typed human
read/request services, durable retry intent and EN/CN mini-app view in isolated
paired source. Three actual loopback/SQLite recovery tests, 31 new VM checks,
43 cleanup regressions and two full bundle checks pass. Native501 ordinary
cases,21 management store checks, paired Desktop filters and release pass.
The accepted33 settlement flow is retained in running Desktop39 on all four
existing localhost profiles with server32/schema21;39 read-only regression passes. Actual EN/light and CN/dark pending/complete views, single-command
delivery and normal-restart recovery pass for the already revoked validation
agent. Full retained-management migration remains open.

## Problem and boundaries

The original `agent.fence` receipt is immutable. For an agent that has issued
work, that receipt deliberately reports incomplete usage and unknown process
state. Later stopped observations and completed usage must not rewrite it.
Integration30 correctly keeps the original reservation plus approved budget
increases held while those facts are incomplete, but has no fresh per-agent
settlement operation.

A new `agent.settle` command observes only the original permanently fenced
agent. Its capability is `hagency.managed_agents.settlement.v1`. It references
the exact original fence command and its receipt digest. The native adapter
requires the original owner/profile/device and current finite authenticated
cleanup authority; expiry of the original grant does not authorize new work.
Recovery returns a previously stored result and cannot create an executable
lease. This operation cannot start a provider, increase credit, release a
project/grant, renew the grant or reassign an agent.

## Native implementation

The settlement writer validates the original project history, grant receipt,
reservation digest, permanent semantic fence and original fence receipt inside
one SQLite transaction. A present child must be exactly the original permanently
fenced child. An absent child requires the original `absent_fenced` tombstone.

One bounded child liability collector now serves project and agent accounting.
It includes the full applied budget chain, known usage, outstanding calls,
overflow/unknown usage, original shared process custody, every successor and
issued qualification-process custody. Only each process's own retained stop
observation can clear its process uncertainty. Complete usage remains a separate
requirement. Prepared calls left behind a fence remain unresolved.

The new receipt records known consumption, retained liability, unresolved
liability, completeness, process state, exact applied budget head, original
period and a digest of the liability summary. Fully observed overrun remains
charged even when it exceeds the ceiling. The receipt carries no spendable
balance. Its ledger revision is globally increasing. Receipt storage and revision
advance commit together using the existing append-only capacity receipt journal;
there is no native schema change. A failed save rolls back the revision.

Each command returns its exact original result across retries and restart.
Newly arrived usage needs a new command/result. Old fence and settlement receipts
remain unchanged. Existing parent manifest fields/encoding remain the same;
conservative results change only where the additional custody or prepared-call
check finds unresolved evidence.

## Paired implementation

Integration32 adds central schema21 with immutable command, issuance and result
journals, finite independent executor tickets, exact recovery and a latest-proof
projection. Current project managers and assigned project administrators can
read/request observations; the original owner/device uploads its native result.
Server-administrator status alone grants no project-agent approval authority.

A new fence result queues one initial settlement. Historical receipt replay and
schema upgrades do not enroll old agents. Already enrolled agents receive one
new observation when newer work/process/budget evidence arrives; polling alone
never creates a repeated command. Pending/lost budget results prevent a lower
central retained balance. A matching late budget receipt can make an existing
proof current; a newer observation invalidates the old proof and schedules a new
one. All original receipts remain unchanged. Global native revision collision
checks include settlement records in both arrival orders. Known usage uses the
per-agent maximum of work, fence and settlement evidence before summing agents,
so the same consumption is not counted twice.

The native transport checks server discovery, current SDK scope, original device,
closed command/digest and finite cleanup authority. It first recovers the exact
saved local proof. Missing historical proof does not execute a new observation.
Lost POST/ack, restart and changed SDK generation recover original evidence only.
The native shared worker polls settlement after process/fence reconciliation,
with a separate cursor and the existing account lifetime. It can continue after
the mini-app closes; it cannot launch provider work through this adapter.

## User-facing implementation and remaining acceptance

1. **Implemented and qualified in33.** Typed
   `hagency.agents.settlement.get` and `.request` host services are added. The
   request must use the existing durable management-intent machinery with a
   distinct `AgentSettlementRequest` intent and explicit package permission.
   Preserve its exact command ID, request revision and fence receipt digest;
   recover an unknown response under current SDK authority instead of issuing
   a second logical request. The current central POST already replays that exact
   input under current role visibility. Integrate that with `with_intent` and
   pending-intent recovery explicitly; the generic recovery route now
   recognizes this new operation. A locally cached successful request
   must still recheck central visibility before a former administrator can read
   it. Do not expose executor tickets or original reserve payloads.
2. **Implemented and native EN/CN pixels inspected in33.** The current removal
   page now uses the shared theme and EN/CN settlement flow. Show
   the latest settlement independently from the original cleanup receipt and
   Matrix room exit. Explain pending device delivery, unresolved liability and
   stale proof caused by newer evidence. Move the original fence completeness
   message into original-receipt details. A stopped process or room exit alone
   must never read as completed accounting.
3. **Implemented and VM-checked in33.** Offer a bounded recheck only when the
   central projection says `canRequest`.
   Disable it while a command is pending or the latest proof is current. Refresh
   remains a read. Use server discovery to keep older servers usable.
4. **Local acceptance passes.** Actual native EN/light and CN/dark captures
   show the latest result separately from the immutable original. Exactly one
   request/result survives refresh and a normal manager restart. Ledger revision125
   reports16161 known consumed/retained and0 unresolved. Original four calls,
   the4096-token increase and the original exhausted agent are unchanged.
5. **Activated only on localhost.** Normal shutdown and consistent backups
   preserved existing sessions and configuration. Both grants had already expired;
   all validation processes had already stopped. No live provider-stop claim,
   refund, new provider work, grant renewal or production deployment is implied.
   Evidence is in `.run/management33-stage/checkpoint.json` and its referenced
   acceptance reports. The standalone App Hub gate does not support this required
   host feature; the paired Desktop package is restamped and checked instead.

This slice is one part of full retained-management migration. It does not close
provider management, policy/approval bindings, rich Boards, resource amendment,
other inventory gaps, cross-platform acceptance or the whole-app UX gate.
