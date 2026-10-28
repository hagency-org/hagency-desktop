# Provider account management

## macOS credential storage repair

Real Codex authorization exposed `persist_failed`: the OAuth callback completed,
but Security.framework could not discover the default keychain while the child
process used a managed namespace as `HOME`. A read-only native probe returned
`-25307` with that namespace and `0` with the OS account home.

Codex children now obtain macOS `HOME` from the OS user database, while retaining
their explicitly selected `CODEX_HOME`, working directory and keyring-only
credential policy. Account login, model inspection and credential-reading launch
paths use the same helper. Other platforms retain their existing behavior. This
does not reset a keychain or fall back to plaintext credential files.

The repair passes 61 native tests, strict Clippy and 48 Desktop host tests; six
opt-in installed-provider tests remain skipped. All four original local test
roles run the repaired release. Inspection of the actual sign-in child confirms
the OS home and the original selected Codex namespace. The new authorization
attempt expired without a completed callback, so successful real credential
storage and resource enrollment are still pending. Evidence is retained under
`.run/management41-keychain-fix`.

## Earlier account-management acceptance

Integration39 runs on all four original localhost profiles with central
server32/schema21 unchanged. Schema92 adds selected-account enrollment and original-namespace execution;
explicit model inspection remains bound to the selected private account. Actual EN/CN signed-out responses, refresh and normal restart
pass without using default-profile credentials or changing login/resource/budget
state. Authenticated model catalogs remain offline-fixture evidence; completed
real authentication still needs the account holder to finish the provider page.
Integration36's actual authorization start/cancel acceptance is retained.
Evidence: `.run/management39-stage/checkpoint.json`.

The existing provider discovery service reports connected runtimes/model choices.
It is not the retained console's managed-account inventory. These are distinct:

- A managed namespace has its original account ID, ordinal, state, revision and
  profile. Login readiness comes only from the latest unexpired observed receipt.
  Preparing, uncertain or retired namespaces do not claim a usable login.
- A canonical resource binding records an adopted provider association. It has
  an opaque profile-specific ID and shows only resources currently granted to
  that profile. The saved association is not fresh authentication evidence.

`hagency.accounts.list` and `.get` request only `ReadAccounts`. Inputs contain
only opaque identifiers/cursors. Native workers recheck original authority and
return bounded pages without provider account IDs, credential references,
namespace paths, seats or authentication URLs. Reads neither probe/login to a
provider nor modify reservations. Resource details reuse the existing authorized
read path and local display names. Revoked resource grants immediately remove
the corresponding visible binding and make old detail links unavailable.

The EN/CN flow is Resources → Provider accounts → Account → Associated resource.
Loading, failure, empty and preparation-unknown states are distinct. Refresh
uses only read services. Late replies cannot replace another selected account
or a different page. An account's resource count is limited to the caller's
visibility; it is not a complete installation dependency or retirement check.

## Remaining retained behavior

MGT-002 preparation is locally accepted for newly created profile-owned spaces.
MGT-004 unused-account retirement is locally accepted in39; bound-account retirement
and provider logout remain unfinished. MGT-005 enrollment is
implemented and locally activated; real authenticated acceptance remains pending. Native
login is implemented, but completed real authentication remains unaccepted. The inventory slice does not mark MGT-001/003 full original parity
accepted without their actual namespace/import acceptance. In particular it
does not reopen an adopted private archive as a normal legacy store.

The old `UseLocal`/`Disconnect` provider service stops profile runtimes before
changing the connection. It must not become an unreviewed mini-app button or
stand in for namespace retirement. Original namespace retirement fences that
binding and unpublishes its resources; it does not prove remote withdrawal,
provider logout, stopped shared processes or settled reservations.

Native login is implemented. The next implementation needs current namespace
revision on retirement/enrollment and preserved uncertain outcomes. Canonical retirement also
needs installation-wide dependency checks under the original owner's authority,
followed by the existing project/agent fence and reconciliation transitions.
Only completed stop/usage evidence can clear uncertain liability. A profile's
paginated resource list cannot authorize that transition. Native unpublication
also needs the contribution amendment/re-admission protocol described in the
[resource plan](hagency-resource-detail-and-amendment.md).

Validation must cover correct-profile and wrong-profile behavior, stale/revoked
authority, failed or lost replies, restart recovery and real EN/CN native views.
Provider login and shutdown must be tested separately from the read-only account
inventory; a simulated provider is not live-provider acceptance.

## Preparation implementation accepted locally (35)

`hagency.accounts.prepare` takes only the original command ID. The host supplies
its current Matrix/profile/package/device authority; script input cannot choose
a filesystem path, ownership identity, provider executable or credentials. A
separate write grant does not imply account inventory access. The original
private intent is saved before the native worker reserves an ID. Its ownership
mapping and effect receipt commit with that reservation, before directory IO.
Only the original in-memory attempt can materialize the namespace. Later replay
reads the same ID, including after the effect/reply gap and a normal restart.
A prepared namespace is neither authentication evidence nor a resource grant.

Schema91 preserves older namespaces and does not assign them to an arbitrary
current user. Profile-scoped inventory now includes newly owned namespaces as
well as adopted resource associations. The migration/adoption and archive paths
explicitly recognize91; the read-only audit requires an ordinary ownership table
with its real columns and retains populated ownership as history. Actual native
UI, normal restart and activation pass; see the35 checkpoint for evidence.

## Native login implemented (36)

`hagency.accounts.login` accepts only the original command ID, account ID and
expected revision. A dedicated grant opens a native review. The original attempt
and command receipt commit atomically before the owner starts the finite Codex
app-server process from that namespace's `ManagedLaunch`. The native owner retains
process custody when a view closes; shutdown and adoption drain it before the store.
Same-command recovery reads the original attempt and cannot spawn another child.
Namespaces already bound to any installation resource reject a new login.

Only native memory holds the validated authorization URL. An explicit native button
opens it; Splash and journals receive no URL or credentials. Current SDK/profile/
package checks protect dispatch, polling and URL delivery. A pending login shadows
old readiness. Only a fresh observed subscription result establishes login readiness;
failed cancellation/stop remains uncertain. Preparing or signing in does not enroll,
publish or allocate resources. Offline fixtures verify successful completion, scope
loss and effect-gap restart; actual provider tests verify start/cancel only.

The next enrollment bridge must observe models and quota identity from the selected
original namespace, preserve its execution binding, and deduplicate physical quota
across multiple namespaces. It must not use the profile-default provider home or
treat namespace IDs as independent quota. See the retained integration36 enrollment
findings and the resource amendment plan.

## Selected-account model inspection accepted locally (37)

`hagency.accounts.models` accepts only account ID and expected revision under a
separate read permission. A retained original namespace handle gates metadata
observation; pending login, stale revision, wrong profile, retirement or lost SDK
scope refuse the result. The short-lived child is observed stopped before returning.
Only public model fields reach Splash. Read failures never claim that a command
was saved. Loading, signed-out, empty and populated states are distinct, and late
replies are checked against page, account and revision.

## Historical foundation qualification (38)

The isolated schema92 source separates physical provider quota from the private
execution namespace. Its atomic enrollment, original receipt recovery, quota
sharing, authorized account/resource association and missing-binding refusal pass
405 ordinary store tests (37 ignored). At this intermediate point it was not
activated; the remaining integration is recorded below. Execution
renewal must acquire current authority, while cleanup evidence must remain writable
when the namespace is unavailable. Full migration acceptance remains open.


## Selected-account enrollment locally activated (38)

The new `hagency.accounts.enroll` action reviews an exact original account ID and
revision, provider model name, qualified reasoning level and declared monthly
limit. It has a dedicated grant and native confirmation. No script approval flag,
credential, executable or path is accepted. The original intent precedes effect;
canonical receipt recovery precedes any new provider observation. The native
worker brackets fresh model inspection with account/quota observations and stops
the metadata child before accepting the proof. Unsupported public quota identity
is an explicit refusal, not a guessed email or namespace-based seat.

Schema92 retains the selected private execution namespace separately from quota
identity. Matching physical quotas share one canonical seat and ceiling. Metadata,
text and successor execution, connection verification and owner grant verification
use the recorded namespace; a missing binding never falls back to the profile
home. Live operations recheck current authority, while cleanup keeps original
custody. Bound account relogin/retirement remain refused pending a complete drain
and retirement protocol.

The mini app provides EN/CN model selection, declared-limit editing and native
review, followed by the existing resource detail screen. A resource starts
unpublished. Legacy profiles are directed to finish device setup before canonical
enrollment. Cancel closes the current review and reloads original saved requests;
it cannot establish that a previous attempt created no resource.

The page and callback VM checks pass; full native/store and focused checks are
recorded separately under `.run/management38-stage`. Final exact-source Desktop tests and release pass. All four original localhost
roles run38/schema92. Actual signed-out model checks, original settlement reads
and normal restart pass with preserved identities and business records. Successful
provider authentication and enrollment are still offline-fixture evidence; their
actual UI acceptance remains pending.

## Unused account-space retirement accepted locally (39)

`hagency.accounts.retire` requires its own capability and a native confirmation
bound to the original Matrix profile, account ID, revision and command ID. The
script cannot provide an approval flag, logout result, credential path or provider
command. Native code serializes the operation with account metadata work, rechecks
current SDK/profile authority and applies a finite confirmation deadline in the
queued writer.

The write transaction checks all legacy and canonical resource associations,
including ones outside the current profile's visible resource list, plus any
unfinished sign-in. It rejects stale revisions and accounts that are not active.
For an unused account, retirement, an explicit logout-unobserved observation and
the original management result commit together. Repeating the exact request after
a lost reply or restart returns its existing result. A receipt-write failure
rolls back the database. Conservatively fenced in-memory handles are not revived
on a failed write; recovery uses the original request and reopens the store normally.

The EN/CN account page opens native review, keeps cancel/expiry separate from an
unknown earlier result, and preserves pending original requests. The retired
view offers no sign-in or model-inspection action and explicitly retains provider
credentials. This slice does not perform provider logout, delete private data,
withdraw resources, settle agents or return token capacity. Bound-account drain
and historical namespace import remain unfinished. Exact-source native/store and Desktop qualification pass. All four original
localhost roles run39. Actual EN/CN native review/cancel/retire/refresh and normal
restart pass on two NEW empty test namespaces; the two original active namespaces
and all original resources/calls/budgets are preserved. Retired-detail captures
after restart are identical to the inspected originals. Evidence:
`.run/management39-stage/checkpoint.json` and `visual-review.json`.
