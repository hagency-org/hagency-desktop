# Pending shared contract release

Exact source from companion App Hub commit `9854614` on
`feat/palpo-miniapp-contract`, based on App Hub 2bcb898. Adds the ADR 0010 exact service grants and their
consent language as version 1.2.0. No admission or integrity rule is relaxed.
Remove the root, standalone miniapp-catalog, system-apps and miniapp-package
Cargo patches when the same shared contract is published. All four consumers
require at least version 1.2 so build, admission and packaging agree.

The additive `palpo.agents.control` grant and consent text match companion App
Hub commit `6180376` on `fix/palpo-project-consent`. It authorizes bounded
project-scoped lifecycle intents; the backend still checks current authority.

The notification read/settings grants match App Hub commit `8ff0654`. The native
host supplies a device time-zone suggestion; only an explicit settings save
changes the account preference.

The vendored manifest, Palpo service registry, policy and growth tests now match
App Hub `6180376`, including the closed native signup/agent navigation targets
and verified My Actions room feature gates. Native integration was adapted from
Rinx `a9aaa2f0`, `8d2ffbd4` and `ee3836bb`; the superseded Node workflow-view
normalizer is intentionally not used with the ADR 0011 Rust projections.

Scoped rename consent now matches App Hub `a93073d`; the same lifecycle grant
is checked for the project owner/resource owner/current coordinator. Runtime
`coordinatorAgentProfileV1` negotiation is required before displaying rename.

Project setup recovery consent matches App Hub `2e2ad26`. The Rust endpoint
checks current project/engagement roles and requires an existing approved grant;
it cannot create a replacement approval.

Native project room selection and its required host feature match App Hub
`978a28f`. Only the selected joined room crosses the Rinx picker boundary;
Palpo independently checks current owner authority before binding it.

Desktop Windows integrity repair (2026-10-13): bundle digest paths now join
filesystem components with `/`, matching the original Unix producer encoding
without rewriting literal backslashes in Unix filenames. Manifest hashes and
admission rules are unchanged. A fixed nested-bundle digest regression runs in
every installer job. Preserve this repair until the shared release includes it.

Embedded Hagency management (2026-10-15, work in progress): adds the exact
`hagency.*` registry and required `hagency-management-v1` feature. No Palpo
capability is upgraded or aliased. Desktop admits these services only for its
verified compiled bundle. This local additive extension still needs upstream
contract/harness synchronization before publishing a bundle to App Hub.

Project plan operations (2026-10-16, work in progress): adds exact catalog and
project read/request/decision/revocation capabilities. Resource admission
permission does not imply project approval. Project approval records intent; it
does not allocate tokens, create a Matrix project, or authorize agent execution.
The native host preserves command custody and consults current server roles on
recovery. Servers advertise `project-requests-v1` before the mini app enables it.

Resource detail (2026-10-16, work in progress): `hagency.resources.get` is a
separate read-only grant for bounded native configuration/budget/usage
observations. It does not authorize configuration, publication or allocation.
The built-in resource detail screen requests this grant. Editing remains disabled
until durable native provenance or an amendment protocol establishes authority.

The Hagency agent-admission slice adds six closed services: readiness, list, get,
request, decide and cancel. Read, request, assigned-project-admin decision and
manager cancellation have separate native grants. There is no execution, runtime,
provider, room creation or arbitrary-URL command in this surface. The app hides
these views unless the server advertises `agent-admission-v1`; current server
roles and exact revisions remain authoritative at every mutation.

Additional configurations (2026-10-18, work in progress): the exact
`hagency.resources.create` capability requests native review and creates a new
locally published configuration on the existing account/seat. It cannot supply
account identity, account quota, paths, publication flags or server grants.
Generic dispatch refuses it without native confirmation; original-command
recovery preserves the returned new resource ID.

Resource re-admission (2026-10-19, work in progress): exact history and reoffer
grants are registered in both the service registry and manifest capability list.
History remains read-only; reoffer creates a separate administrator decision for
a rejected, withdrawn or expired connection. Neither grants capacity or provider
execution. The existing exact-service policy test covers both grants and still
requires `hagency-management-v1`; server feature discovery and current role/
revision checks remain independent. This is an embedded Desktop extension, not
a published App Hub contract.
