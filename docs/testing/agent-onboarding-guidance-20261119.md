# First-account Agent guidance — November 19, 2026

A hidden current-main Desktop JIT signed in through real Pasion PKCE and Matrix
sync against Server `9a9cbad5195f72d583b760f627d9eba608b9067f`, using its exact
CI OCI image in an isolated Docker project with three databases. The Desktop
base was `71d6e9ad` with the small bundle-copy change in this PR.

Reproduction: use a fresh account whose local hosting is not set up, with Server
resource-pool capability gates disabled. Open Agent, Agent manager, My Agents.
Before the fix, no Local Agents section appears and the sole explanation says
that the server needs an update, even though the current server has the feature
but disabled its optional configuration.

After the fix, the same real session shows the local-hosting prerequisite and
points to the existing Hosting header and subsequent Add Local Agent entry.
The separate resource-pool notice asks the administrator to check settings and
version, without asserting an upgrade is the only cause. The new copy was
rendered and semantically inspected in English and Simplified Chinese; real GPU
captures were checked. The unsigned bundle digest was refreshed and the JIT
startup validated it. No hosting Setup or model execution was triggered.

The pinned current-main baseline also passed all 50 `hagency::ui::tests` under
Makepad JIT, including actual Octos profile selection, model choices, stale
approvals, stop/readiness and unknown-result retry. These component tests do not
claim a full first-Agent inference flow or a matched Buzz benchmark.

Testing reused the existing Makepad runner, helper and native libraries. All 19
native package source/version/feature records matched the cache; fail-closed
Cargo and C-tool adapters only copied verified precompiled artifacts. No native
application or dependency compilation occurred. An idle remote host, private
profile, daemon home and explicit isolated installation root were used. User
JIT processes and installed OS hosting were preserved.

Private screenshots, profiles, device material and full SDK logs remain under
ignored `.run/`. The website audit separately records the 35-feature comparison,
published/main boundary and remaining end-to-end acceptance gaps. This copy fix
does not claim a Desktop UX score of 90 or superiority to Buzz.
