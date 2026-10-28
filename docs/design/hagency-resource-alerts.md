# Resource alerts in the Hagency mini app

Status: integration40 read/triage, integration41 embedded observation and
integration42 explicit history transfer are qualified and promoted locally;
all four original test roles run42. Actual
EN/light, CN/dark and normal restart acceptance pass for the40 UI.
The first actual capture found repeated action IDs collapsing four choices into
one; each action now has its own View. The new VM regression failed before the
fix and passes afterward (42/42). This is the retained alert read/triage slice,
not full alert parity.

## Journey

Resources → resource details → Resource alerts keeps the original resource
identity. All states and open-only filters have bounded pagination. Open an alert
to see its observation, impact, runbook, recovery condition and paged notes.
Choose a legal handling action, review it, optionally add a note, then save.
The host supplies the current Matrix actor. Assignment means “Assign to me”;
suppression uses the retained24-hour default. Resolution records a decision,
not proof that the underlying condition has gone away. Refresh reads current
state and does not resubmit a decision. Original commands survive lost replies.
The views use the existing reloadable theme, SVG controls and EN/CN text.

## Native contract

The package asks for `hagency.alerts.list`, `.get` and `.triage`. Read and triage
permissions are separate. Each request includes an exact resource ID; scripts
cannot supply SQL, a path, owner/profile authority, actor or provider credentials.
Legacy stores remain private to the current SDK profile. Canonical requests
recheck the original active resource grant inside the queued operation.

The retained legal-transition map is shared with the original console. A digest
of the complete alert observation and latest note sequence is its review version.
A new sweep, operator edit or note invalidates a stale review. The state change,
note and exact original command receipt commit in one immediate transaction.
Original-result recovery rechecks current authority and never reapplies a state
change, even after a later observation or retention of the source alert.

Only public numeric budget evidence is projected from raw detail JSON. Original
operator text remains scoped to the authorized resource. Pages contain at most16
alerts or notes. Triage never changes reservations, resources, agents or budgets.

## Acceptance boundaries

Store regressions cover scoped reads, wrong resource, lost grants, pagination,
queued authority loss, revision changes, legal transitions, suppression retention,
receipt rollback, original-result recovery and normal reopen. Native SDK tests
exercise actual loopback identity and SQLite restart recovery with synthetic data.
VM checks cover shipped EN/CN views and original-command callbacks; they are not
native screenshots. Actual hidden Metal interaction/capture also passes on a
dedicated non-administrator localhost fixture. Acknowledge, assign, resolve and
suppress create exactly four original receipts; cancellation, refresh and restart
create none. The fixture has no provider credentials or calls. Original resource,
execution and budget rows are preserved. Root inspected both languages and the
restarted resolved state. Evidence is under `.run/management40-stage`.

Integration41 shares the old `bootstrap::start_ceiling_sweep` reconciler through scoped native observation;
the retained accounting projection already includes canonical shared liability.
Integration42 accepts bounded populated alert/notes history through an explicit
v3 native review and transfer. Saved v1/v2 migrations retain their original scope.
Matrix notifications/reminders, richer JavaScript alert metadata operations and
whole-app/platform acceptance remain open.

## Embedded observer (integration41, accepted locally)

A separate native source change now shares the retained reconciler and observes
one authorized resource per writer transaction. The global sweep remains for the
original console. Canonical observation and pruning recheck the exact resource
grant inside the transaction; a user cannot prune another resource's alert history.
The producer owns a separate native-only observation permission, not mini-app triage.

Desktop starts the observer with the SDK-backed owner. It reopens existing
management history even with no mini-app window, without creating an empty database
on every login. It visits at most16 resources per page, yields between pages, and
uses the retained hourly interval after a complete pass. Account retirement cancels
the original task; shutdown joins it before draining the management writer.

Both strict native configurations pass. Store alert checks8 and retained alert
regressions11 pass, including access/retention isolation, authority-loss rollback
and canonical shared-call overrun without freeing unknown liability. Native alert
checks3 pass, including reopening without a mini-app management package and
stopping writes after normal suspension. Owner lifecycle checks34 and management
drain checks3 also pass; filters may overlap. Desktop host checks48 and release
build pass. Actual Desktop startup removes one expired synthetic alert while
only the main window is open; original alerts, notes, four triage commands,
resources, executions and budgets remain identical. The CN/dark mini app then
opens the preserved resolved record, refreshes without another command, and
shuts down normally. Root inspected the actual native captures.

Source is promoted with backups and activated on the four original localhost
roles. Session/device identities, schema92 and the original settlement remain
unchanged. Evidence is in `.run/management41-stage`. This does not claim a live
provider overrun, hour-long scheduling soak, GPU-less headless rendering or mobile
acceptance. Legacy populated-history adoption and Matrix notification/reminder
delivery remain separate unfinished work.


## Explicit history transfer (integration42, accepted locally)

Fresh native adoption reviews task and alert counts, then binds their exact
history digests. The v3 plan retains all original alert columns and note sequence,
author and text, including bounded raw detail that is never projected into the
mini app. Completed `alert.triage` receipts remain in original private custody.
Source changes, malformed/unknown contracts, oversize histories and destination
collisions are refused. Import and receipts are atomic; activation allows normal
subsequent observation, retention and triage without invalidating the original
immutable migration. Saved v1/v2 scopes and bytes stay unchanged.

Native qualification covers70 store adoption cases,6 activation cases,8 source
recovery cases and strict checks. Desktop checks5/48 pass before a small label
correction; the rebuilt final release passes actual EN/CN native acceptance.
The original synthetic fixture preserves2 alerts,3 notes and4 old receipts,
then creates exactly1 new handling receipt and1 note. Refresh/restart retain
that result and the exact SDK binding. Original legacy rows, resources and budgets
remain unchanged. Captures are real hidden Metal windows, inspected by the root
agent. The test uses a closed offline metadata stub, not provider credentials or
inference. Evidence: `.run/management42-stage/actual-acceptance.json` and
`visual-review.json`. All four original local roles run42; production is unchanged.
