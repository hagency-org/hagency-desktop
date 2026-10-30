# Resource pool publication — 2026-10-28

The owner now submits one native **Publish resource pool** form containing eligible Matrix accounts, an exact token budget displayed in M, and a UTC expiry. The slider is bounded by the observed canonical remaining capacity; unknown capacity disables publication. Expiry presets (1 day, 7 days, end of month) are bounded by the current accounting period. Local catalogue listing is labeled separately from server publication.

`OwnerGrantAccess::publication_form` supplies read-only native facts before submission. Opening or refreshing this form creates no grant intent, hold or publication. One explicit native submission advances prepare → confirm the original reservation → review its publication → confirm its saved command. Each existing client boundary still rechecks the resource, owner, device, period, host lease, and available capacity. A resource identity/revision change stops continuation. Unknown, rejected, expired or closed outcomes stop continuation and retain their original IDs. Closing the window stops subsequent UI stages; an already pending operation retains its original custody.

Opening a published pool observes its existing original grant. Recovery and reopening never set the submission flag. No additional confirmation is required for successful intermediate stages, and no extra hold is created. The workflow does not launch an Agent or alter admission approval policy.

The shared token editor supports a slider plus a compact exact M input. Publication uses a real capacity bound; configuration editors have an expanding slider scale and do not misrepresent it as an account authorization limit. Typed values retain integer precision and are validated before publication.

## Amendment and renewal — 2026-10-29

`hagency.grants.review` also accepts, only beside `contributionId`:

- `succession: "amendment" | "renewal"` opens the native form for a successor of the pool's published grant: this month's new per-request limit (amendment), or next month's grant scheduled during the last 7 days of this month (renewal). The form shows the current limit → the new one, how many Agents keep running, the tokens Agents already hold (the lowest the new limit may take), and, for an amendment, that pending requests above the new limit cannot be approved. Applicants can be added but never removed. A renewal's delegation defaults to the end of the new month; an amendment keeps the current delegation. An unfinished successor (unattempted review, unknown outcome, saved but unpublished change) is shown instead of a new form until it is published, recovered or cancelled.

A successor requires the server to advertise `pool-grant-succession-v1`; otherwise the panel explains that the server must be updated. With that support, a `proposedTokens` beside this month's active grant no longer closes with `pool_grant_already_published`: the panel offers **Change per-request limit**, which opens the same native amendment form. The script still cannot commit anything: prepare, confirm and publication are native, exactly as for a new grant, and the close reply is the same public progress (its summary now also carries `grantId` and, for a successor, `succession`, with `successionState`).

The notes below document earlier implementation checkpoints; their separate-confirmation UI is superseded by this flow.

# Historical native owner grant review checkpoint

This is a native confirmation prerequisite, not activation of capacity allocation in the shipped Hagency app. The built-in bundle and service-capability catalog are unchanged. No normal session has a registry/adoption initializer or `hagency.grants.review` grant.

The proposed closed handoff is `hagency.grants.review {contributionId}` or `{intentId}`, exactly one selector. All other fields are rejected before lazy native initialization. A package cannot supply owner/provider identity, registry path, period, command/grant ID, raw receipt or native permission. The ordinary permission mapping deliberately grants no `ConsentGrants` for this service.

The host must eventually obtain an already-qualified canonical registry from the separately reviewed source-adoption coordinator, plus a distinct native management session authorized for `ReadResources`, `ConsentGrants`, and provider observation (`ReadProviders`/`ConnectProviders`). That future initializer is absent here. `OwnerGrantAccess::bind_registry` checks the exact live SDK/management session; it never opens a path or qualifies/imports old state. The Desktop handoff fails closed with `owner_source_adoption_required` when no trusted access exists.

The native flow first searches the original journal for this contribution. Existing intent IDs are recovered/reviewed; they are never silently replaced. For a new intent, editable choices are collected from native widgets, not Splash: coordinator, eligible managers, token amount, explicit UTC delegation expiry and coordinator self-approval. The next page displays native-authenticated owner and server, native profile, resource/model/revision, remaining/total resource budget, shared-account limit, exact UTC month/start/end, selected limits and original intent ID. A missing capacity observation is shown as unavailable, never zero.

The review captures one canonical resource/budget observation. Confirmation is fenced by exact resource/provider/profile/device/period authority plus a digest of the displayed budget facts; unrelated usage counters and observation timestamps are not in that digest. The digest is checked inside the grant transaction after immutable receipt replay and before any new effect. The dedicated `OwnerGrantReviewChanged` result is only emitted there; it restores the original journal record to review-required. All ambiguous outcomes remain applying/unknown and require canonical recovery. Successful confirmation reserves capacity locally; it does not publish a grant or launch an agent.

The host adds a restriction-only gate to native access. The original package lease's expiry/revocation and host retirement are checked in the real SQLite writer as well as around asynchronous boundaries. A bundle cannot acquire this gate or use it to replace the native SDK/provider authority.

Close/Back/Escape before confirmation only drop the opaque review. They do not decline/delete the journal, issue another command, or release a hold. Closing while an operation is pending leaves its original private custody intact. A fresh open or restart recovers the original journal. A changed account/session invalidates the old handle. The native UI keeps its controls outside the scrolling details body and explicitly supports Tab/Shift-Tab, Enter/Space on a focused action, and Escape.

Validation in this checkpoint uses actual isolated native SDK/HTTP/SQLite fixtures and the pinned Makepad VM/layout engine. It does not qualify real source adoption or exercise production accounts. Native compilation and unit layout checks are not a visual 9/10 acceptance claim. Root owns final native instrument captures.

Remaining activation work: qualified source adoption and registry initializer; independently reviewed capability disclosure; real native confirmation/publication E2E with that authority; explicit workflow for changing or closing a stale unattempted review. Merely reopening a review cannot change its immutable choices, period or binding.

## Original publication handoff

The native panel now offers a separate **Review publication → Publish to server** step after canonical local recovery proves the original hold. This requires native `PublishGrants` and `ReadIntents` in addition to `ReadResources`; it does not add either to the script handoff or shipped capability list. The exact native-generated publication command and authenticated contribution CAS are saved as an optional private field before review. Confirmation is persisted before using the existing `AllocationTransport` operation. Retry uses that command/CAS and the immutable canonical grant receipt, even across restart, lost replies, or a changed server result. The public progress contains only the original safe summary and observed server state/revision/time.

The extra host gate is composed with the existing SDK/device/profile gates and checked around transport awaits. A response arriving after host retirement stays unknown and its original pending custody remains available to a fresh authorized host. The initial local reservation remains held on rejection, loss or cancellation; publication cannot reserve another hold or move its period. Latest server state can be revoked, and recovery does not reactivate it.

Missing optional publication metadata is accepted only when native command history can exclude earlier publication for that grant. Matching pending/replied commands or acknowledged/scrubbed/unclassifiable grant-publication history fail closed before generating a new ID. The same check runs inside the new-intent transaction; exact original replay precedes it. Scrubbed historical custody needs a separately designed reconciliation path; the UI reports that block rather than guessing or replacing the command. No schema migration or historical receipt rewrite is performed.

Before a new publication POST, the native registry separately verifies current parent/resource eligibility. Historical receipt recovery does not imply an active hold: a closed parent or revoked resource blocks a new publication while its original result remains recoverable. A local close during an already in-flight remote POST is not atomically cancellable; the local permanent fence still denies subsequent execution, and the UI reports server observation rather than active local capacity.
