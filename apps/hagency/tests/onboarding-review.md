# Native resource onboarding checkpoint

The Resources page opens **Set up shared capacity**. It reads native readiness,
then explains three separate steps: move the original local resource management,
obtain server admission, and review a local capacity reservation followed by
server publication. Each step uses actual native state. Unknown capacity remains
unknown; admission, migration and provider qualification never imply a running
agent.

## Boundary

| Mini-app service | Closed input | Native authority |
| --- | --- | --- |
| `hagency.onboarding.status` | `{}` | Reads current device migration mode and descriptive original grant IDs. |
| `hagency.onboarding.review` | `{}` | Opens the native panel; ordinary Host request execution refuses confirmation. |
| `hagency.grants.review` | Exactly one original `intentId` or admitted `contributionId` | Opens the separate existing native grant panel. It revalidates canonical resource/provider/account/period history. |

These capabilities do not map to `ConsentGrants` or
`PrepareSourceQuiescence`. The native migration panel alone collects the explicit
shared monthly account limit and displays the owner, Matrix server, complete
resource IDs/models and original migration identity. A second native action
confirms moving resources. The migration does not reserve project tokens or
start agents. Native source provenance and provider identity are neither
script-provided nor editable here.

Closing before confirmation returns `review_closed`. Closing while an operation
is in flight returns `outcome_unknown`; the retained native task and immutable
original journal own recovery. Logout/session retirement denies further UI
actions. A stale review must reopen the original review, not generate new IDs.
Migration completion requires a real canonical Resources read. The following
capacity panel separately displays remaining/total native budget, exact UTC
period, manager/coordinator restrictions and publication confirmation.

The wizard offers original grant recovery before new admitted-connection
reviews, filters connection rows to the signed-in owner, follows existing
connection pagination and rechecks verification expiry from server timestamps.
A replacement device can see inventory but has no grant review unless native
original-device custody is present. Unsupported source history gives a concrete
blocked explanation and retains access to resource/connection records.

## Resource owner readiness

The project-agent prerequisites card groups the project, its single project
manager, current access, and specific blocker. The manager also handles agent
approvals; no separate project-administrator assignment is needed. A
resource owner without request visibility gets an explicit next step back to the
project's resource connection; capacity coordination belongs to the owner; agent approval belongs to the manager.
The server's `canViewRequests`/`canRequest` flags remain authoritative. This
replaces the flat role/status block measured at 7.87 for CN dark in integration
14/15. The revised screen has not yet received integrated native visual
acceptance; its score is not claimed to exceed 9.

## Evidence and limits

The tracked test runner and reduced pinned-VM regression are documented in
`README.md`. The actual Splash subtree checks cover EN/CN routes, no script
confirmation inputs, paging, original-ID recovery, unavailable/unknown states
and command preservation. Native Rust tests cover closed request shapes,
permission separation, bounded ceiling input, close outcomes and Makepad Cx
footer bounds with long EN/CN content at phone/desktop sizes. Cx bounds are not
pixel captures. The resource-adoption backend has its own isolated restart,
original-plan and grant-publication tests maintained by its owner.

Before acceptance, rebuild the combined feature-enabled Desktop checkpoint and
use isolated owner/manager fixtures to inspect the complete windows:

1. Resources → shared capacity → native review. Verify no token default and
   exact current native owner/server/resource list; close before confirm.
2. Enter an explicit shared ceiling, inspect the second review, then confirm only
   in the designated isolated migration fixture. Recover the same original plan
   after an interrupted response or restart.
3. Verify active inventory, separately open an admitted owner's native grant
   review, and check original-intent recovery after closing/reopening the mini
   app. A replacement device must remain inventory-only.
4. Exercise Escape/Back and keyboard Tab/Shift-Tab. At the supported small and
   desktop sizes, all footer controls must remain fully reachable.
5. Check CN dark resource owner prerequisites and whole-window captures with the
   unchanged UX scorer. Do not crop or add filler to satisfy the threshold.

No live server, fixture process, provider credential, real ledger or production
installation is changed by this UI checkpoint. Full managed work history,
quota-change requests, removal and unsupported source adoption remain separate
parity work; no successful running-agent claim is made here.

The syntax-checked `tools/hagency/native_management_onboarding.py` automates
status, all three stages, refresh and Back against an already-open loopback
fixture. Optional `--open-legacy-review` is restricted to an observed fresh legacy
state and closes the initial native panel without entering a ceiling. It does
not confirm or resume an original migration. The probe has not yet been run on
an integrated binary.
