# Desktop durable knowledge

## 2026-10-28 — resource publication and workspace entry

The native publication form reads capacity before owner consent. Its one submit action advances the original reservation and publication; recovery never enables this continuation. Published pool status takes precedence over an expired connection diagnostic. Diagnostics are optional and collapsed. The shared token editor uses a slider and a fixed 110px exact M input. Do not use host `script_apply_eval!` to resize a TextInput owned by a Splash isolate: cross-VM script object handles can become invalid during typing. Keep input layout in its widget definition.

The desktop sidebar `new_workspace` button dispatches the existing `HagencyAction::NewProject` native Matrix Space form. Resource project/allocation requests remain disabled; Matrix collaboration workspaces are independent of pool Agent applications.

## 2026-10-13 — execution device configuration

`hagency-desktop` uses the sibling `hagency-client/native/hagency` dependency. The CI checkout pin in `.github/actions/setup-hagency-client/action.yml` must include the NativeOwner configuration wrappers whenever Desktop calls them. A local sibling build alone cannot verify that checkout pin.

NativeOwner contracts: `AgentConfigurationStatus` returns `{configuration: metadata}`; `ExecutionEnvironment` returns raw `{providers, configuration}`; `AgentConfiguration` wraps only the approved Local/Provider/RuntimeProvider/PrepareAgentWorkspace commands and returns the raw operation result. The server schema is defined in `../hagency-server/docs/design/2026-10-13-cross-device-agent-configuration.zh-CN.md` and its review.

Remote model and Agent policy edits target the assigned device; provider catalogs, references, reasoning choices and workspaces must never fall back to the viewing device. Room/requester policy, reset, provider login/logout/disconnect, service controls and approvals remain local. Metadata polls use a separate UI request fence, preserve mutation feedback, and cannot clear a save's busy state.

`local_revision_conflict` is the client policy CAS error. `invalid_local_policy` also covers target workspace canonicalization failures. `configuration_outcome_unknown` and malformed configuration responses do not prove that an attempted write failed; retain the original idempotency key. A confirmed success or terminal validation failure ends that intent, so an A → B → A model sequence creates a new final write rather than replaying the first receipt.


## 2026-10-28 — Agent answer actions and local export

`AgentReply` has its own body/folding widget, so controls attached only to `content.message` do not cover managed Agent answers. Keep the answer action row inside AgentReply, but route clicks through the containing Message's current `MessageDetails` and RoomScreen event lookup. Copy, forwarding and exports must use the complete current Matrix body rather than visible HTML, the folded preview, or a stream fragment. The action row is hidden while the live marker remains set and cleared on recycled/hidden rows.

Markdown export keeps the UTF-8 message bytes. `crates/answer-export` parses Markdown and uses Parley system font fallback plus Krilla PDF encoding for local A4 exports with text, tables, source links and pagination. PDF work runs off the UI thread and uses the existing native save dialog; no URL or image is fetched. A missing system glyph is an explicit export error instead of a silently blank character. PDFs are not screenshots. MuPDF is used for acceptance text extraction because lopdf 0.36's ToUnicode parser cannot read Krilla's generated CMaps; lopdf remains useful for structural checks.


2026-10-28 owner direct funding: Do not model a resource owner funding their own
Agent as a pending request requiring a second human approval. New clients send
ownerFunded=true and approveImmediately=true only after an explicit amount confirmation;
canOwnerFundDirect on the budget read advertises the server behavior. Old missing/false
flags retain old pending semantics and byte-identical command digests. The server validates
the actual resource owner and budget snapshot, then records the request and its approval
inside one DB transaction using the existing decision checks and SQL evidence triggers.
It returns the original request receipt plus the latest approved top-up. No schema change.
Recovery reuses the original command ID; it must never issue a replacement top-up.
Native direct funding requires both RequestAgentTopUps and DecideAgentTopUps, including
before HTTP, after HTTP and saved-command replay. A script field is not owner authority.
The device still checks the full resource/account budget, siblings and retained unknown
usage. Authorization, device application and task execution remain distinct observations.
The MiniApp's budget is a server accounting view; its consumption field is not a live
provider meter, and null native remaining must never be presented as zero.

Desktop regression command: python3 tools/check-hagency-regressions.py --output .run/regressions
--client ../hagency-client (one shell line). CI and installer releases run the Desktop
checks; release packaging depends on their success. Keep budget/role/replay checks with
Markdown, previews, activity and answer actions/export. The checker rejects empty Rust
test matches. Native HTTP fixtures need RUST_MIN_STACK=16777216. Server tests require a
disposable PostgreSQL database; never use the production recovery database. Evidence:
/Users/ychen/home/hagency-desktop/.run/direct-owner-20261028/.


2026-10-28: Shared-thread delivery validation must compare immutable original Matrix
content, never regenerate it through the current presentation renderer. Optional
formattedBody is receipt evidence only: omit when absent to preserve old bytes, validate
closed body/thread/format fields plus original content digest and source/work bindings,
and strip HTML only after validation from provider input. Rich evidence needs coordinated
server/native rollout (old strict clients reject the added field). Previously saved rich
receipts without evidence remain unsupported; never rewrite their immutable digest.
River preflight had6 receipts/0rich so that limitation does not affect this deployment.
Run the combined Desktop regression gate with --client to include real offline-probe
context tests (both filters, --include-ignored); run server shared_context_ against a
separate disposable PostgreSQL DB. Context admission failure must emit bounded stage/code
without raw prompts or credentials. Historical cancelled work stays terminal; a new
explicit chat message creates the next attempt. No live model turn was used for this fix.


Agent execution/permission indicators belong in the conversation PortalList, not
RoomScreen header or a pinned composer banner. Own the stateful widgets outside recycled
list templates and draw one virtual tail row; keep Matrix event indices/read receipts
independent. Redraw parent when a previously undrawn status appears/disappears. Offscreen
status keeps non-input polling but must not receive stale pointer/scroll hits. Regression
geometry tests live in src/home/room_screen/agent_status_tests.rs (agent_chat filter).
Headless cached views require DrawEvent.redraw_all=true when testing successive layouts.
For missing answers, verify delivery AND inspect the actual UI: main-room timelines hide
thread replies; main summary is not the full answer. Do not equate process stopped with
failure after a completed/replied work receipt, or assume sent means currently visible.


2026-10-28: For a chat UI preview restart, preserve the separate resource-owner Desktop
process and canonical store; restart only the requesting user's profile when sufficient.
If live work completes during the build/restart, distinguish screenshot acceptance of
the completed chat from headless coverage of running/approval states; do not generate a
new live model request solely to make an animation appear.


2026-10-28: Keep answer actions as SVG icon targets with localized tooltips and keyboard
handling. Their NavigationBarButtonAction must be routed through the parent Message's
current binding, including folded replies. Do not restore text-only action pills.
Article editor belongs in Mini Apps, not a duplicate primary rail shortcut. Appearance
is reachable from the rail palette button and Settings; both reuse AppearanceSettings.
Default platform page/container roles may be identical (macOS #ececec): explicit app
surface separation is owned by theme::Selection, never by hard-coded page consumers.
Preserve custom/host overrides and test all light/dark/accent combinations, including
state retention/restart; the combined release gate now includes theme and contrast.


Room information defaults collapsed on new/restored room or thread selection, at
both wide and narrow widths. Do not force chat_info_requested=true in room setup.
Keep the header toggle available, including history/media controls while collapsed.
Legacy private approval-room discovery is compatibility functionality; do not expose
it as a routine room-information step for resource-pool agents. Their permission
requests are handled by the inline chat card. Keep approval authorization unchanged
when simplifying navigation. The chat-information and desktop_information_columns
regressions are part of the combined release gate.


Chat LinkPreview uses responsive1/2/3-column wrapping based on its own available
width and message font size. Collapsed means one row, not a fixed card count. Keep
expansion across resize, preserve child identity/cache/original click URLs, and
recompute fixed child widths after script/theme reapply. Expand/collapse controls
are NavigationBarButton SVG chevrons with localized tooltips; route their actions
through NavigationBarButton, not Button. Native offline fixture covers all three
widths, expansion/collapse, original link targets, thumbnails and theme reloading.


2026-10-28 font audit correction: the Apple typography startup log does NOT
prove PingFang is used for English/Chinese chat. A probe linked to the current
release libraries and using production registration followed by ensure_fonts_loaded
and FontFamily::get_or_shape found IBMPlexSans-Text/SmBld for English,
LXGWWenKai-Regular/Bold for Chinese prose, and LiberationMono-Regular for English
code. The Apple installer prepends native members only in the loader; script
FontFamily members stay unchanged. Current ensure_fonts_loaded checks the expected
member count, detects a mismatch and rebuilds from script members, dropping the
injected primary. This audit did not fix or change product fonts. Existing Apple
font tests based on earlier loader assumptions are insufficient; future repair
needs post-draw glyph selection and theme-reload coverage. Evidence:
.run/font-audit-20261028/report.json and selected-fonts.txt.


2026-10-28 font audit repaired: do not inject extra primary fonts only into
Fonts loader caches. Declare them as script FontFamily members, so draw-time
expected-member reconciliation, lazy fallback loading and theme reload preserve
them. Register both base appearances before widgets_mod selects one. Explicit
stylesheet font overrides still win. For the installed PingFang TTC, the local
in-memory resource points collection entry zero at the selected SC face, keeping
tables intact; no dependency hash duplication, OS font mutation or shipped font
copy. Check actual shaped-glyph PostScript names after drawing/reloading, not a
startup log. The macOS gate includes apple-chat-fonts and apple-system-fonts.
Current Edward preview uses PingFangSC-Regular/Semibold for prose; English code
keeps LiberationMono-Regular. Native font fixture inspects actual retained labels.
Use native_theme_release.py --isolated-installation for onboarding-test-support
binaries; the authority root must be a new/private child, not a755 evidence root.


2026-10-28 uniform web previews: grid columns depend on available width and
message font size, not URL count. A single URL occupies one cell; all cards
reserve196 logical px at11pt body size, scaling upward with the message font.
Use a Fill summary above the Fit website footer so missing/short descriptions
do not change card height or footer alignment. Invalidate cached width after
changing URL children, including growth at an unchanged container width.
AgentReply uses NavigationBarButton SVG chevrons with localized Show more/less
tooltips. Route clicks through that widget type, keep fold state per reply, and
retain the full answer for copy/forward/export. Native offline fixtures exercise
metadata sparsity, images, one/two/three columns, reuse and actual fold clicks.

2026-10-28 pool application limits: use the pool's publishedTokens as the
per-request authorization ceiling, intersecting a known nativeAvailableTokens.
A null native balance is unknown, not an available reservation. Dedicated Agent
budget top-ups do not enlarge the pool's original published grant. Validate
positive whole tokens and capability membership before creating a room, then
read the latest pool/revisions again before each submission phase. Retain raw
out-of-range drafts; never silently clamp their typed amount. Definitive request
rejections return to editable form and reuse its original room; unresolved
commands retain their exact original ID/payload and block edits/new submissions.

The pinned Makepad Slider has no script change callback. RinxAmountSlider bridges
real Slider actions into the owning Splash isolate. Keep amount editing in place:
rebuilding the whole page on each keystroke/drag loses focus and interrupts the
interaction. Use the slider's set_value and the local MiniAppButton set_enabled
adapter for incremental updates. Initialize the button's disabled animation from
its enabled field, so blocked controls also look disabled. Run the shipped-form
VM check in the combined regression gate, plus tools/hagency/native_pool_application.py
against the release hagency_visual offline host for real pointer/keyboard input,
continuous text entry, theme reload, narrow layout and changing server limits.


### 2026-10-28 — Workspace entries and shared mini-app widgets

- Workspaces are navigation, not authorization. Agent manager, resource Owner
  (also coordinator), and server Admin have separate navigation and inbox queries.
  Save the selection alongside the account-scoped draft; never replace unresolved
  commands or room IDs during a switch. Reset inbox filters when changing roles.
- Register shared widgets in both the Splash isolate and the host VM before
  native confirmation UI modules use them. Isolate-only registration lets the
  mini app render while breaking Desktop's native widget registrations.
- Do not call `script_apply_eval!` through a raw `vm.cx_mut()` borrow inside a
  ScriptHook; it re-enters an already-held VM. Prefer direct setters, such as
  `TextInputRef::set_empty_text`, or correctly park the VM with `with_cx_mut`.
- A compound amount editor must emit Actions for native confirmation drafts as
  well as queue its Splash callback in the owning isolate. Otherwise theme
  reapplication can restore a stale native draft. Its step buttons need explicit
  keyboard focus because RinxButton defaults to `grab_key_focus: false`.
- Wrap each repeated named choice in a distinct View. Repeating a named button
  directly in one parent's loop leaves only the final choice in native widgets;
  inert VM objects do not catch this. Native model review asserts all reasoning
  levels are present.
- Slider-derived widgets do not expose `visible`; put conditional progress
  meters in a View. Update text/slider/button state without rebuilding the active
  editor. Clearing an enrollment validation notice must not redraw the form.
- Native acceptance scripts fail on all `[E]` messages, preserve input traces
  across process restarts, and scroll offscreen zero-bounds controls into view.
  Fixture-injected initial pages validate UI, not external business execution.
## Hagency widget font scaling — 2026-10-28

Do not hardcode text controls to 44 points after adopting the shared theme.
Use `max(44, mod.widgets.RINX_CONTROL_HEIGHT)` for buttons, number editor parts,
text fields and switches. A 44-point minimum is a target-size floor, not an
upper bound. Exercise a 1.5 typography scale through `theme::packages::apply`
on the offline native host, checking both retained draft text and actual bounds.
