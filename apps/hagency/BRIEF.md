# Hagency workspaces and widget UX

User request, 2026-10-28: rebuild all Hagency mini-app capabilities with suitable
widgets and exceed 9.5/10 UX acceptance. Present Admin, resource Owner and Agent
manager through separate entry points; do not flatten all capabilities together.

## Product contract

- First entry selects a task workspace. Retain the last workspace per signed-in
  Matrix account and expose an explicit workspace switch. A person may use more
  than one workspace without signing out.
- Agent manager replaces the former project-manager journey: discover resource
  pools, request/manage Agents, inspect work and usage, request additional budget.
  Do not reintroduce the removed project-creation step.
- Resource Owner also coordinates supply: provider accounts, local resources,
  publication, capacity/permission approvals, execution devices and recovery.
- Server Admin: resource admission, server connections and administration.
  Workspace selection never grants authority; use current server capabilities
  and permission flags, and preserve all backend enforcement.
- Preserve identities, saved drafts, unresolved command IDs/payloads, original
  room recovery and owner-only permission/budget decisions. Remote business
  mutations are outside UX testing; use the offline native host and fixtures.
- Refactor every reachable Hagency management page and its native form bridges.
  The initial source inventory contains 87 page keys. Identify obsolete hidden
  compatibility routes separately; do not restore them to the navigation.

## Widget system

Use coherent cards, labeled form groups, status badges, bounded sliders with
exact number input, steppers for unbounded owner-defined quantities, proper
selection/toggle controls, progress stages and collapsible advanced sections.
Keep one primary next action per task. Field errors belong beside their field;
unknown capacity and unconfirmed external effects remain explicitly unknown.
Actions use the shared SVG family, theme tokens, font choices and keyboard
behavior. Avoid rebuilding active editors on each input event.

## Native acceptance and score

Apply octoscript-app-card-ux acceptance to this existing native Desktop host.
The local App Design Flow checkout supplies SCRIPT-API, CAPABILITIES and
flows/core/NATIVE-INSTRUMENT; its MODEL-VALIDATION.md is absent. Existing bundle
creation/publisher/listing steps are not repeated and no publication is requested.
The repository's compatible miniapp-package tool and catalog gate replace the
older App Hub binary that rejects hagency-management-v1.

Score the final source, not the historical 2026-10-23 candidate. Keep the five
existing equally weighted review dimensions: orientation/terminology,
role/task priority, journey continuity, visual hierarchy/density, and
feedback/recovery. Require a mean strictly greater than 9.5/10 and no unresolved
core-task, authority, keyboard, clipping or readability defect. Report reviewer
identity and whether this is an author review; never imply a user-study result.

Bind each reviewed family to current source/build hashes, actual full captures,
real native input and observed local state. Exercise EN/CN, light/dark/violet,
wide/narrow, empty/error/denied/loading, cancellation, theme changes and restart.
Retain failed runs. Treat fixture UI, real service reads and external business
execution as separate claims. A numeric score cannot waive a failed task.

## Implementation sequence

1. Preserve and validate the completed pool-allowance vertical slice.
2. Inventory routes, capability ownership, native bridges and control types.
3. Implement workspace entry/switch/retention and scoped navigation first.
4. Build reusable controls, then apply them to every capability family.
5. Exercise each family, fix defects, inspect pixels and score against the rubric.
6. Run the combined budget/chat gate, preserve unrelated Desktop work, and
   hand off the local Desktop with the actual coverage and remaining limitations.
