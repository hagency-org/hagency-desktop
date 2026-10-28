# Desktop progress

## 2026-10-13 — desktop issue #2

Final design was saved and independently reviewed before implementation, on branch `codex/issue-2-cross-device-config`. Added cross-device model and Agent-wide policy configuration to the existing native pages. Initial metadata automatically requests and polls the assigned device environment; cached snapshots remain read-only while offline. Device name, connectivity, heartbeat/observation time, provider installation/version/readiness/account verification and target workspace are displayed. Remote provider connect explains its owner-wide effect. Login/disconnect/runtime/approval/reset/Room/requester operations are blocked remotely.

Requests are fenced by identity, UI request, Agent and assignment generation/device. Switching Agent/account clears visible catalogs/profile/workspace. Separate background metadata polling preserves mutation feedback. Mutation intent keys survive same-panel refresh/selection on unknown or transport errors, and are removed after confirmed success or terminal failure. Keys are in-memory UI state; full Desktop restart does not restore a requester-side pending intent. Worker-side receipt persistence is provided by client.

Focused Makepad widget and routing tests cover target-only catalogs, precise Octos reference selection, offline cached settings, online saves, target workspace/version label, scope allowlist, stale metadata and idempotency intent transitions. Initial full library snapshot passed 458 tests with 2 existing ignored tests; final snapshot/build results will be recorded below. The English/Chinese catalog checker passes with no missing keys. Root owns final client CI SHA pin and integrated A/B-process validation; physical multi-device/provider-inference validation is not claimed here.

Final frozen-source verification:

- `cargo test --locked --lib`: exit 0; 458 passed, 0 failed, 2 ignored. The ignored tests are the pre-existing backend interop fixture and isolated Palpo Moments fixture; they are not counted as passing.
- `cargo build --locked --bin rinx`: exit 0. Existing Cargo patch/duplicate-package warnings and the macOS large unwind-table linker warning remain; no new compilation errors.
- `python3 tools/wechat-ux/check_i18n.py`: exit 0; 1821 catalog entries, 1430 translated call sites, 0 missing keys.
- `git diff --check`: exit 0.

Independent Desktop review corrected first-load metadata errors, preserved online state for validation/CAS errors and quiet metadata polls, and verified generation changes clear old catalogs/workspaces. Request intent fingerprints also include the execution device ID/generation. Client review observation about input+result envelope response bounds was forwarded to client owner and resolved there. No further Desktop-specific actionable review finding remains.

Root pinned CI to client `33c75d3a645f4732a086ece5fa8297fc9553a419`, and rebuilt the locked Desktop binary against that exact client source after its final Agent-specific busy correction. Cargo.lock has no dependency changes. Final real embedded Server/Pasion/Palpo with two NativeOwner processes passed, including B without any provider executables, A validation/persistence, both reopens, idempotent result and exact runtime Ledger resolver under a real active owner-direct binding. This uses a declared provider fixture and makes no physical-device or actual inference claim.


## 2026-10-28 — Answer copy, forwarding, Markdown and PDF

Completed persistent Copy / Forward / Save Markdown / Export PDF controls on completed Agent replies, including folded replies. The parent Message binds each action to the current event; RoomScreen resolves the complete current Matrix body. Forwarding opens the existing message/recipient picker and retains its explicit Send step. Markdown export preserves the original UTF-8 bytes, while local Parley/Krilla PDF export provides embedded system fonts, selectable Chinese/emoji, headings, code, tables, source links and pagination through the native save dialog. Work runs off the UI thread; no model or remote rendering request is made.

Validation: 5 native answer-widget tests and 2 export tests passed; independent MuPDF acceptance retained all 120 paragraph markers plus Chinese, emoji, code, table content, links and the final marker across 4 pages. On Edward's existing 小白 answer, the collapsed Copy button copied all 13,153 source bytes, the recipient chooser opened with one selected message and Send disabled until a recipient is chosen (no message sent), the native Markdown save matched the source exactly, and native PDF save produced 6 pages with the final paragraph and 26 link regions. Both exported files remain in Downloads. The existing malformed Markdown in the historical source remains unchanged.

Built with locked dependencies and default + onboarding-test-support; preserved all prior locked dependency versions. Updated admin/manager/owner processes to `.run/answer-actions-20261028/rinx-isolated`, backing up each profile before restart and retaining the original owner canonical directory inode (208631362). Owner call, budget-delta and permission-policy hashes remained unchanged; integrity and foreign-key checks passed. No new model work or budget was requested. Evidence: `.run/answer-actions-20261028/validation.json`, screenshots and export fixtures. New English/Chinese keys pass scoped parity; the global catalog checker already failed before this task (19 unmatched Chinese keys and unrelated missing management descriptions), which is recorded rather than counted as passing.


2026-10-28: Owner direct funding and regression prevention deployed. dspfac can enter an
amount and confirm once (optional note); no second self-approval. Manager requests retain
resource-owner approval. The explicit approveImmediately+ownerFunded request executes both
records atomically under the same transaction, real owner authority, exact budget snapshot
and existing SQL approval triggers. Retry/lost reply reuses the original request and delta;
old omitted/false fields preserve saved command bytes. Native requires both request and
decision capabilities. UI distinguishes authorization/device application from task execution,
labels server-confirmed consumption honestly, and does not label every account an applicant.
Desktop main CI and installer release dependency now run the combined budget/chat regression
gate; server CI already includes ignored PostgreSQL tests and now sets the tested 16MiB stack.
117 Rust, 36 EN/CN bundle, 20 PostgreSQL tests and independent 4-page PDF acceptance pass.
Actual dspfac UI reviewed through confirmation only, then cancelled; no live top-up/model
call. Same budgets, calls, tool policies, app-service identity and canonical inode208631362.
Server 20261028T170141Z; all 3 default+onboarding-test-support Desktops use .run/direct-owner-final-20261028/rinx-isolated.
Evidence: /Users/ychen/home/hagency-desktop/.run/direct-owner-20261028/validation.json,
owner-confirmation.png, owner-budget.png and scoped.diff. Owner left on 小白 budget page.


2026-10-28: 小白 H100 follow-up cancellation fixed and deployed (server 20261028T173555Z,
all three Desktop profiles .run/xiaobai-context-20261028/rinx-isolated). The request was
accepted/acknowledged but stopped before context freeze/model admission: the server had
regenerated historical plain Matrix content with the newer Markdown renderer, invalidating
immutable delivery evidence. Compare original closed content instead; new rich receipts
carry exact optional formattedBody evidence, validated natively against supported JSON
encodings. Omitted field preserves old receipt bytes/digests. Validate first, then exclude
HTML from provider input while retaining original Markdown. No history rewrite or new
migration. Native context preparation now logs bounded stage/error code at WARN.
All 16 combined release checks pass: 135 Rust tests plus bundle checks, and 15 server
context tests pass including 13 disposable PostgreSQL tests. Native offline subprocess
follow-ups are included in the release gate using a freshly built actual probe. Six live
historical receipts passed read-only content/provenance checks before and after upgrade.
Server readiness/Matrix startup roundtrip and restored Desktop sessions verified. Existing
new owner-funded +1M at17:25:25UTC was independently applied: ceiling9M revision5,
available1,907,955; 8 calls, known251,453, retained7,092,045, policies and owner canonical
inode208631362 preserved. No new prompt, top-up, hold release or cancelled-work replay.
Edward remains in 小白 chat; cancelled “和H100比较呢” needs a new explicit message.
Pasion signup enabled/contact-not-required settings and Appservice identity preserved.
Evidence: /Users/ychen/home/hagency-desktop/.run/xiaobai-context-20261028/validation.json;
investigation: /Users/ychen/home/hagency-client/.run/xiaobai-check-20261028-latest/handoff.json.


2026-10-28: Local-only agent chat status placement fix (explicit user request: do not
 deploy/restart). RoomScreen now owns persistent agent_status widgets and draws them as
the final PortalList row. Runtime/activity/permission state scrolls with messages, no
longer consumes space beneath the chat header or above the composer. Existing polling,
20fps spinner, exact room/thread/account scope and approve-once/session/deny remain.
Visibility changes redraw the timeline; offscreen pointer/scroll input is suppressed.
Two headless geometry regressions cover360/840/1280px, sidebar, inline approvals, history
scroll, appended messages/widget identity and completed-row removal. Included by the
existing agent_chat release filter. Five existing column-layout tests also pass.
All16 check categories passed after one native-context HTTP retry (15s fixture-stop timeout;
same unchanged native source then4/4 in5.68s). Preserve first-failure log; not a clean first
pass.142 Rust tests plus bundle checks. No binary promotion, restart, server/config edit,
Matrix message, model call or budget change. Evidence: .run/inline-agent-status-20261028/.
Read-only diagnosis during work: 小白 completed fresh H100 comparison and sent3,296chars
at10:45:43 PDT,69,478tokens settled; event$na92WdO8qN0NV3aQs4mqs3EmhTdayrj-tLDetXPuDfc.
Actual Edward screenshot shows full answer/table/actions in 小白·话题. MainRoom intentionally
filters threaded replies and displays summaries; this task did not change thread routing.
Cannot infer earlier synchronization/scroll timing solely from the current screenshot.


2026-10-28: User subsequently requested Desktop restart to inspect the inline status UI.
Built the tested sources in release mode with onboarding-test-support; source hashes
matched the validation manifest. Backed up Edward's stopped admin profile, then restarted
only Edward on .run/inline-agent-status-preview-20261028/rinx-isolated (PID48266,
port64288). Owner PID38364 and other Desktop PID38366 remained alive and unchanged;
server release20261028T173555Z was untouched. Actual new-window screenshot confirms
小白·话题 restored with the latest16-GPU answer, Markdown, preview thumbnail and
copy/forward/Markdown/PDF buttons. Latest work swork_01m61tpdqextrz4k6mwj7veraj completed,
usageComplete=true, reply=sent, runtime inactive. No active banner remains. Running
animation/approval placement was covered by headless tests, not observed live after
restart because the task had already completed. No new prompt, budget change or hold
release. Evidence: .run/inline-agent-status-preview-20261028/restart.json and
edward-chat-after-restart.png. Original native fixture timeout remains recorded.


2026-10-28: User requested SVG answer actions, removal of the duplicate article
editor rail entry, and restoration of visible theme/surface hierarchy. Copy, forward,
Markdown and PDF now use 36px NavigationBarButton targets with 18px SVGs, translated
hover hints and existing keyboard behavior. Parent message action routing, complete
answer exports and streaming guards remain covered by the updated regression test.
Article editor remains available through Mini Apps; its rail entry was removed.
A palette icon at the bottom of the desktop rail opens the shared AppearanceSettings,
including light/dark/system, teal/violet, and the existing customer theme studio.
The stock macOS palette had identical page/container colors (#ececec). The standalone
palette now separates page, panel and field in both modes. Navigation colors derive
from the active theme; conversation sidebar uses the page role. External host styles
and custom packages retain ownership. Theme separation and contrast checks now run
in the chat/budget release gate.18 check categories /145 Rust tests passed, plus native
four-theme switching, draft/heap retention, production dropdowns, restart persistence,
custom import/preview/cancel/apply/undo/recovery and hosted ownership checks. Theme
literal and whitespace guards pass. The optional global i18n check still reports19
pre-existing Chinese-only catalog keys; no catalog changes were made and all labels
used here already exist in both catalogs. Evidence: .run/chat-polish-20261028/validation.json.
Built release and restarted only Edward (PID60797, port64288), preserving ownerPID38364.
Actual chat screenshot shows the four SVG actions, retained Markdown/link previews,
palette entry, dark rail and distinct sidebar/chat surfaces. Live selection remains
light/violet; dark mode was inspected in the native fixture. Another test Desktop
(managerPID38366) was found exited during the restart script's final check; its
metadata was unchanged, no quit was sent to it, cause/time unknown, and it was not
relaunched. Direct rechecks confirmed Edward and owner healthy. Server unchanged.
No Matrix send, model call, budget change or reservation release. Visual capture:
.run/chat-polish-20261028/edward-chat-after.png. Further UI input stopped when the
live window geometry changed during review.


2026-10-28: Removed the legacy Approval rooms shortcut from the room information
panel; existing approval protocol/settings compatibility and in-chat permission cards
remain intact. RoomScreen now starts collapsed on room/thread selection/restoration;
only an explicit information-button click opens it. Updated the real room-selection
layout test at500/840/1280px, including a second room after opening the first pane,
and the overview/member row-routing test. Added both existing test groups to the
combined release gate. All20 checks passed,161 Rust tests; theme literal/whitespace
checks passed. Release built and only Edward restarted,PID64673/port64288 on
.run/room-info-default-20261028/rinx-isolated. OwnerPID38364 stayed live; the manager
profile was already exited in preflight and was left untouched. Actual screenshot
shows restored 小白 thread at1160px with the right panel closed. One explicit click
opens the panel without Approval rooms; another closes it. Left the pane closed.
Markdown, SVG actions and link previews remain visible. No server/config/permission
policy/budget edits, model calls or chat sends. Evidence:
.run/room-info-default-20261028/validation.json and restart.json.


2026-10-28: Chat web-link previews now lay out one, two or three cards per row
from the available message width, scaling the minimum card width with text size.
Collapsed previews show one row; expansion survives resizing and shows all links.
Replaced text fold buttons with SVG chevrons, localized tooltips and existing
NavigationBarButton keyboard behavior. Retained cards, thumbnails, original URLs
and theme reload state. Added actual headless reflow coverage with seven varied
cards, plus native 240/500/760px pointer/layout checks in the offline fixture.
All20 combined gate categories passed,162 Rust tests; native fixture passed16 checks
including card recycling, thumbnail, fonts/theme reload, message bounds and articles.
Visually inspected native1/2/3-column captures. Release built and only Edward
restarted,PID68971/port64288; ownerPID38364 unchanged, previously exited manager
left untouched. Live 小白 thread shows three cards per row; expand shows all six
links in two rows and collapse restores one row. Right pane stays closed, Markdown
and SVG answer actions retained. No server changes, chat sends or model calls.
Evidence: .run/link-preview-grid-20261028/validation.json and restart.json.


2026-10-28: Fixed Apple font registration being overwritten on draw. The old
installer only prepended native loader-cache entries while script FontFamily
members still named the bundled fonts; member-count reconciliation rebuilt the
family and dropped PingFang. The installer now declares a system-font resource
member in both light/dark theme bases, retaining fallback members and lazy flags.
Selected installed TTC faces are exposed at collection index zero in memory;
font tables/checksums are unchanged, with no copied font assets on disk or shipped.
Explicit theme overrides including mono keep precedence; code/icons are untouched.
Added actual chat draw/shaped-glyph coverage for English/Chinese regular/bold,
resource generation changes, retained widgets across light/dark reloads, fresh
contexts and monospaced selection. Shared tests verify face metrics, glyphs and
invalid directories. Both suites now run in the macOS combined release gate.
All22 gate categories passed,169 Rust tests. Native link-card fixture passed17
checks including actual selected glyph font names before/after scaling/reload.
Full release light/dark startup passed with custom themes retained. Its old
harness initially stopped at the test binary's isolation guard; added opt-in
isolated-installation argument with a fresh private child directory and reran.
Only Edward Desktop restarted,PID81955/port64288; ownerPID38364 remained live,
previously exited manager left untouched. Actual 小白 chat screenshot confirms
new typography with Markdown,3-column previews,SVG fold/actions and closed right
pane retained. No server changes, chat sends, provider calls or budget edits.
Evidence: .run/font-fix-20261028/validation.json and edward-chat-after.png.


2026-10-28 uniform web preview cards and SVG answer folding completed.
All cards use a common196px height at the default11pt body size, scaling with
text. Fill summary anchors website footers; grid width no longer depends on
URL count, so one link stays the same width as a cell in a fuller row. Existing
responsive1/2/3 columns, images, original click URLs and expansion are retained.
Reset layout cache when adding/reusing URL children at unchanged container width.
Replaced AgentReply Show more/less text buttons with SVG down/up chevrons and
localized tooltips. Rich text and full-answer copy/forward/Markdown/PDF remain.
Extended real-draw regression coverage and offline native pointer checks. All22
release gate categories passed,169 Rust tests;18 native checks passed. Initial
example build caught a private helper call; fixture now supplies explicit HTML,
then release build and native validation passed. Edward alone restarted at
PID24114/port64288. Actual 小白 chat screenshot verifies equal card dimensions,
aligned footers, SVG fold/actions, rich text and closed right panel. Owner and
server unchanged. Evidence: .run/uniform-preview-20261028/validation.json and
edward-chat-after.png; scoped.diff records only this task's changes.

## 2026-10-28 — Hagency workspaces and native widgets

Implemented a task-workspace entry for Agent manager, resource Owner/coordinator,
and server Admin. The current Matrix account can switch workspace without
signing out; its last choice survives restart. Navigation, primary actions and
request lists are scoped by workspace. Entry selection grants no authority;
server Admin requires the server capability. Owner capacity requests and Admin
resource admission use separate queries. Saved commands and existing room/draft
identities remain intact. Removed project setup is not reintroduced.

Reworked shared management presentation with form groups, selected choices,
stateful switches, progress bars, SVG disclosures and exact numeric steppers.
Pool application retains the bounded slider, presets and fresh limit check
before room creation/submission. Native capacity and migration confirmations use
the same number editor. Text controls grow with theme metrics from a 44-point
minimum; account identity has its own row for narrow windows and large text.
Native repairs cover host/isolate registration, keyboard step buttons, repeated
model choices, editor focus and retained drafts during theme changes.

Evidence: `.run/hagency-widgets-20261028/validation.json`, with source/build hashes,
original native captures, input traces, regression results and author review.
The review concerns representative macOS core journeys, not all 87 historical
route/state combinations, a user study or live server E2E. Existing extracted-test
failures are retained with pre-change comparisons. No live budget, approval,
message, room or execution mutation was made. The running Edward and dspfac
Desktop processes remain unchanged; this is a local implementation and build.
