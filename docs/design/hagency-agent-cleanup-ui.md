# Agent removal and cleanup UI

The mini app exposes the server's `agent-cleanup-v1` protocol independently of
new-agent admission. Overview and project links reopen the cleanup index after
a restart, including when issuance is disabled. This is separate from work
history and from requesting or approving an agent.

The server remains authoritative: the original manager may cancel a request;
the currently assigned project administrator may revoke an approved request.
A coordinator, resource owner, or server administrator does not acquire that
project permission merely through their other role. Both writes require a
visible confirmation and the current request revision. They use the existing
native intent journal, original command ID, exact operation permission and
recovery route. Unknown outcomes stay pending; retry cannot replace the command.

`hagency.agents.removal.list` accepts only optional project ID, cursor and a
limit of 1–16. `hagency.agents.removal.get` accepts one request ID. Both require
`ReadAgentCleanup`. `hagency.agents.revoke` accepts command ID, request ID and
expected revision and requires `RevokeAgentRequests`; cancellation retains its
existing permission and wire bytes. Script inputs cannot supply an owner,
administrator, provider, receipt, URL, refund or executor authority. Typed native
responses drop nonpublic extras, check identity/cursor/order/numeric bounds and
recheck the current SDK account after HTTP.

The detail screen separates server restriction, native cleanup command delivery,
native/process observations, Matrix cleanup, and accounting. An old live process
record is labelled as an observation. Null consumption, retained liability and
cleanup accounting stay unknown; a recorded zero remains zero. Retained amounts
include spent tokens and unresolved liabilities, not available capacity. Later
usage reconciliation is shown independently of the original cleanup receipt.
No aggregate “fully removed”, refunded capacity, deleted history or currently
running status is inferred. The proposed identity is never labelled as an
actually provisioned agent.

Integration30 shows current progress first and places the original cleanup
receipt under Details. Matrix `fenced` plus an observed cleanup time is labelled
“Project-room exit observed”; `fenced` alone makes no membership claim. Historical
unknown process state remains unknown even when a later observation records a stop.
Applied and unacknowledged approved top-ups contribute to the conservative retained
floor while cleanup accounting is incomplete. Only a complete native fence can
lower that floor; known consumption remains an independent lower bound.

Validation: shipped Splash cleanup checks pass 43 EN/CN cases. Actual native
manager/admin review-cancel-revoke, latest-result refresh, Matrix membership read
and normal manager restart pass in the existing localhost fixture. Twelve native
captures were reviewed. The validation agent was already naturally fenced and
stopped before this test; live-process termination and final settlement are not
claimed. The original historical receipt and exhausted-agent ledger were preserved.
See `.run/project30-cleanup-ui-accepted/acceptance.json` and the current entry in
`hagency-management-progress.md` for exact builds and retained failed attempts.

`tools/hagency/native_management_agent_cleanup.py` is a read-only instrument probe
for an already-open isolated account, requiring real IDs and expected role
controls. It opens the receipt disclosure explicitly. The separate top-up flow
was implemented and exercised with a real provider in integration29. Complete
post-fence reconciliation, final settlement and the overall UX gate remain open.
