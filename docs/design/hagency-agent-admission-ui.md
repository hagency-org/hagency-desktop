# Hagency project-agent admission UI checkpoint

The manager opens **Project → Project agents**, reads the current prerequisites,
and defines a text-only agent with a capability and token amount. A separate
review shows the assigned project administrator and original reservation period.
Only that assigned administrator receives approval/rejection controls. The
manager may cancel their request; neither the resource owner nor the coordinator
gets an additional approval role. Server administrators are not implicitly
assigned project administrators.

Approval is displayed as **Approved · executor setup pending**. This slice does
not claim a running agent, current free native capacity, a completed refund,
Matrix membership or runtime provisioning. The observed reservation limit is an
upper bound from the project's exact pinned setup allocation; native headroom
remains explicitly unknown. Current server mutation validation is authoritative.

## Closed contract and recovery

The six services are `hagency.agents.readiness`, `.list`, `.get`, `.request`,
`.decide` and `.cancel`. Native grants separate read, request, assigned-admin
decision and manager cancellation. No generic URL, token, owner, administrator,
provider, room or runtime input is accepted. List pagination filters the requested
project before its page bound and only permits manager/assigned-admin visibility.
Global Inbox reads use the same existing role scope. Optional `projectName`
remains compatible with older stored receipts.

The bundle hides the feature unless the server advertises `agent-admission-v1`.
The server only advertises it when all existing admission/setup/reservation
policy gates are enabled; no policy is enabled by this change. Readiness uses the
current setup's exact allocation and receipt, not an eligible replacement. The
coordinator's project-read visibility does not authorize the agent list.

All effects use the existing native-generated intent ID and durable native
journal. An interrupted or unverified reply retains that original command.
Retries first recover its latest canonical result under the original operation's
permission; generic HTTP/auth/network failures cannot prove non-application.
Explicit pre-effect rejection codes are terminal only on the first attempt.
After an earlier uncertain attempt, even such a later rejection remains unknown.
Successful recovery displays the latest state, including cancellation, without
repeating the original POST. Mutation errors clear stale UI state and preserve
IDs/drafts for refresh or original-command recovery.

The real HTTP test exposed an inherited cached-device authority gap in
`NativeManagement::authorized`. Its explicit SDK source check now runs after
renewal before reusing a management session; the original scope-loss-after-commit
test passes without modification. A committed outcome remains recoverable under
fresh same-account authority; the retired manager stays denied.

## Evidence and remaining acceptance

- Server: 191 real PostgreSQL/HTTP tests pass, including five new readiness,
  actor visibility, no-effect read, exact pinned allocation and filtered-paging
  regressions; strict all-target Clippy passes. Logs:
  `server/.run/agent-admission-readiness-postgres-3.log` and
  `server/.run/agent-admission-readiness-clippy.log`.
- Native client: four real loopback HTTP/SQLite custody tests pass. They cover lost
  reply/restart, source loss after commit, permissions, later rejection after an
  unknown attempt and malformed committed replies. One POST is preserved during
  recovery. Log: `client/.run/agent-admission-native-http-3.log`.
- Bundle: 36 actual-subtree/handler VM checks in EN/CN pass, plus the existing
  resource-detail 12 and Matrix-setup 12 checks. These use the pinned Makepad VM,
  inert widget prototypes and actual production callback code; they are not
  native screenshots. Package/catalog/i18n checks also pass.
- Native resource-detail release13 captures identified raw reasoning text `high`
  in Chinese; this bundle now displays `高` through the existing label mapping.
  Duplicate public resource cards still need authoritative user labels/context.

Native agent-flow screenshots, keyboard/layout acceptance and the complete real
manager→assigned-admin→executor flow remain outstanding. The read-only probe
`tools/hagency/native_management_agent_readiness.py` is authored and syntax-checked,
but has not run on a new admission build. It accepts only existing loopback
profiles/projects, never restarts or seeds capacity, and checks hidden/blocked
states without effects. Genuine qualified adoption, owner grant, project
reservation and verified Matrix setup are prerequisites for a full execution
acceptance run. No live server, flag, account, room or resource was changed by
this checkpoint.
