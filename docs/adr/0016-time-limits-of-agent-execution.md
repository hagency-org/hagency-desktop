# ADR 0016: Time limits of Agent execution, the daemon's side

- Date: 2026-11-26
- Status: Accepted. The table at the end says what is implemented.
- Scope: Hagency Desktop and its embedded Hagency Client. The rules themselves
  are kept in one place, in `hagency-server`:
  [Time limits of Agent execution](https://github.com/hagency-org/hagency-server/blob/main/docs/design/hagency-time-limits.md).
  This record says what they mean for the owner daemon and for the room.

## Context

The server hands an owner's computer authority that ends by itself: a session,
leases for its Agents, commands to start a process, tickets, activity leases.
Each end had a number of its own, and three failures in production came from
the way they met. The last one is the one people saw: a message sent to two
Agents was answered by one, and under it stood "This message was not
processed; it will not retry automatically", with no word on which Agent and
nothing to press.

## Decision

The daemon follows the rules of the server's document. For the daemon they
come to this:

1. **A lease ends when the server's lease ends.** What the daemon holds (the
   session, an Agent's execution lease, the activity lease of a working
   process) follows the session's authorization window, up to two minutes.
   What it is offered (a start, a ticket, a command from a poll) has 30
   seconds and is offered again when missed. The daemon's local session bounds
   one request, never the run it belongs to.
2. **Renew early.** An Agent's execution lease is renewed every 5 seconds, or
   after half of what is left when that is sooner. The access token is
   refreshed as soon as a proof has lost 10 seconds of the window (at most
   once a minute for that reason) and always when a proof has 20 seconds or
   less left, so the lead stays near the window.

   The activity lease of a working process, in review (Desktop #243 and
   #251): renewed ten seconds after its grant, no later than twenty seconds
   before its end and no sooner than five seconds after its grant; a lease
   of twenty seconds or less is renewed half-way through. Its end is the
   earlier of the end counted from the request and the end on this
   computer's wall clock.
   All four drivers of a process take that moment from one function,
   `io::renewal_moment`. Main still renews it once, when 20 seconds of it
   are left, and the Claude driver when 15 seconds of a 30-second lease are
   left.
3. **A check that cannot be made is not a refusal.** Every failed step is
   decided, not asked, or without an answer, and that is worked out in one
   place. Only a decided failure ends work. The other two wait and ask again,
   and a wait keeps no local session between attempts.

   The Matrix sign-in is a protocol of its own, and the daemon's custody of
   it sorts its failures where they happen. A decided one ends the session:
   the homeserver refusing the token as unknown or missing, the sign-in
   server refusing the refresh (an invalid grant or client), a sign-in of
   another account or device, or a sign-in server that is not this server's.
   A timeout, or any other failure, is without an answer and waits.

   A refusal ends no more than its own message. The daemon saves each start
   of an Agent's process before it asks for it, and a saved start holds the
   place after the process it follows for the rest of the allowance period.
   The server may still make that start, so it is the one start the daemon
   asks for after that process; the server makes no second one either. When
   the server refused a start, the first one of a message or one that runs
   it again, and the message then ended (it waited out its time, it was
   stopped, or it was ended because the Agent had moved on), the start gives
   that place back as soon as the daemon reads the end, and the Agent's next
   message starts. It gives it back only when the message's status names no
   start the server made from it and this computer holds no command issued
   for it; the server starts nothing for a message that ended. Until the
   message ends, a refused start keeps its place and is sent again
   unchanged, unless the server refused it for the Agent's applied budget
   (below). An Agent with an applied budget gives the place back the same
   way, and a start that gave its place back before the budget was applied
   holds none after that. The daemon reads a message's status in
   both protocols, the budget one and the one before it, and each names only
   the starts of its own protocol. It keeps the end each one shows, so a
   start of either kind gives its place back whichever end it read first.
   The starts of an Agent with an applied budget have no allowance period:
   one that does not give its place back holds it for as long as the
   Agent's reservation lasts.

   The daemon decides no new start in the protocol before the budget, first
   or retry, for an Agent this computer has applied a budget to, also when
   that protocol offers the message, and it starts no such Agent in that
   protocol for no message either. A message that comes before the budget
   is reported waits for the report, which this computer sends right after
   applying the budget. A start decided before the budget was applied is
   still sent. Once the budget is reported, the server refuses it
   (`shared_budget_execution_required`), and refuses it again every time,
   since an applied budget stays applied. It looks for a start of the
   message that it made before it reads the budget, so the refusal also
   says that it made none. The daemon keeps that refusal, with the start
   and the digest of the request it answered, and the start gives its place
   back at once and is never sent again. A message whose first start was
   refused is then taken up by the budget protocol, as if no start had been
   decided for it. A message whose retry was refused has a start the server
   made, so the budget protocol ends it, and the retry holds no place a
   later message needs. No other refusal gives a place back before its
   message ends: the server's other refusals of a start can change from one
   try to the next, or name a start it made.

   In review (Desktop #246): an owned Agent's own steps and the session's
   renewal are read by the same rule. A refusal this computer makes itself
   is decided and says why: an answer with no time, on a computer whose
   clock is behind, ends the run as `execution_clock_behind` instead of being
   asked again until the lease is gone, and the Agent's panel then says that
   this computer's clock is wrong and what to change (#255). An answer that
   does not carry the
   server's code is not a decision, so a 401 or 403 without one no longer
   signs the owner out; the proof in hand is kept and the renewal asked
   again.
4. **The end of a lease is the server's.** The daemon holds it two ways:
   counted from the moment it sent the request, with the time the server put
   in its answer, and read against its wall clock, which keeps running while
   the computer sleeps (the counted time does not). The earlier of the two
   ends the lease. The wall clock may only end a lease early: it extends none
   and refuses none.

   In review (Desktop #246): an Agent's execution lease and the
   authorization of each reply of an owned Agent are counted from their
   request with the answer's date, and the session's proof is held against
   both ends, so a proof slept through is not used for its last awake
   seconds. Whether the token needs refreshing is decided on the end the
   server counted, not on what the wall clock left of it. Counting takes the
   time in the answer's `Date` header, which the server process or a proxy
   in front of it writes, against the end the server wrote from its database
   clock: it assumes the two clocks agree to within a second.

For the room:

5. **A message that ended without a turn says for which Agent.** When a
   message went to several Agents, the notice has a line for each one that
   has something to say, with its name. An Agent that did not process the
   message is named even when it was the only one. A message that arrived
   while its Agent was stopped says that the Agent is paused; once the Agent
   runs again, its sender reads that the Agent did not process it.
6. **Its sender can send it again.** "Send again" sends the same message as a
   new one that mentions only the Agents that did not process it, in the same
   thread. An Agent that already answered is not asked twice. The first
   message stays as it ended.

   The server decides where it is offered, and says so for each Agent in the
   message's status. It offers it for a message that ended without a turn:
   one that waited out its time, one that was stopped while it waited or
   arrived while the Agent was stopped, one whose start failed for good (an
   Agent with an applied budget included, whose protocol tries no start
   again), and one whose process ended before the turn began. It offers it
   only to the message's sender, and only while the Agent is in the room and
   has not been stopped. A message that waits for the Agent's allowance is
   not offered while it waits: its notice says so, and shows the Agent's
   owner or manager the pool request that would let it go on. When the
   allowance period turns before that, the message has ended without a turn
   and is offered like the others. Nor is it offered for a run whose outcome
   is unknown, which may have acted or cost something and has its review
   instead, nor for a message that is still waiting.

   A message that the computer blocked because it could not count the
   Agent's earlier usage cannot run in that period, whatever anyone does.
   When the server ends it at once (`endReason`), its notice says so, and it
   is offered only once the Agent's next period has begun, never for an
   Agent without periods; until then its notice says that a copy would end
   the same way. When the server lets it wait out its time, its notice says
   that it cannot start and that it ends within a day.

   It is offered on the message itself, on every message the room's status
   lists that can be sent again, and the notice at the end of the timeline
   offers it for the newest message too. Both read the same status, so an
   earlier message keeps its offer when a newer one arrives, and a click on
   either sends once. The newest read of the status that lists a message,
   of the room or of one of its threads, decides whether it is offered: an
   older read of a thread does not keep an offer that a newer read of the
   room no longer makes. A reply in a thread is offered in its thread. The
   room's timeline does not show the reply and cannot send it again, so the
   room's notice names the Agent and says to open the thread.

   It is never pressed by the program. Two devices of one person would each
   send it, and one message would become two runs.

   What is sent is the message as its sender last edited it, read from the
   room as it is shown; it is never taken from the stored first version. When
   the new message cannot be sent now (the message is not loaded or not yet
   readable, the room's encryption is not confirmed, the Agent's device is
   not ready), the room says to try again in a moment and offers it again.
   A message that was removed cannot be sent again: the server does not learn
   of a removal, so it is offered until its sender presses once, and the room
   then says that it can no longer be sent and stops offering it until the
   app starts again. Nothing is put back into the composer, where the text
   would address every Agent it first did.

   One click sends once: a message that was sent again is not offered again
   by this window, also after the room was left and opened again. The new
   message names the one it was sent for (`org.hagency.sent_again` in its
   content, encrypted with the rest in an encrypted room). Every device of
   its sender reads that name from the sender's own messages: from each one
   the room's timeline, or a thread's, holds when it is opened, from each
   one that arrives later, and, for a new message that has not left the
   device yet, from the device's send queue, which keeps it across a
   restart. A device that has not loaded the new message, or one whose new
   message was removed, offers the first one again; the new message stands
   in the room for its sender to see. A new message whose sending is
   cancelled on its device before it left keeps the first one from being
   offered by that window until the app starts again.

   The server does not read that name: its status keeps offering the first
   message, and only the sender's devices stop. In an encrypted room it
   could not, because the name is inside the encrypted content, so there the
   server cannot link a message sent again to its first one, and an
   operator cannot tell it from a new request.

A server that does not yet describe each Agent of a message is read as it
always was: one line from the message's first row, in the words each state
always had, and no "Send again".

## What is left open

Whether a message whose process ended before its turn began should be run
again without anyone asking. The owner's policy says that what is confirmed
not to have run is run again. For a start that never happened it is: a
message has its first attempt and at most three more. On main, three kinds
of start are not tried again and end the message, as the server's document
says: one whose process reported and then ended before it was given the
message, any start of an Agent with an applied budget, and any start on a
computer whose daemon does not ask for retries. Work that a process was
given and accepted, and whose process then ended before the turn began, is
not run again either. Their sender can send the message again.

Option A, in review as Desktop #249 and server #102, runs such a message
again, whether or not its process had been given it:

- The daemon opts in. Each demand start, poll, recover and block request
  says `reoffers` while the server lists demand retries and the re-offer
  (`shared-demand-retry-v2`). The server lists the re-offer only while its
  operator has switched it on; it is off unless configured. A start is
  always sent with the bytes it was first prepared with.
- The daemon sends the stop of such a process as a request for the
  message's next retry, with its evidence that the process stopped. The
  server opens a retry only at that request, and decides by its own record
  of the turn's start: work whose turn it never recorded it takes back from
  the process and shows the retry; the stop of a turn it recorded leaves
  the message with that process. When the server shows the retry, the
  daemon releases its binding of the work and starts the retry.
- The server's decision rests on one premise about the computer: it begins
  a turn only when the answer to its start request says that the server's
  record of the start was made now. The daemon meets it in three places.
  The start step of shared execution (`context_io::start`, and the same
  step for an Agent with a budget) can ask only with the attempt that the
  store gives out after it has saved that the start was asked for, and the
  store gives out no second attempt for the same work. The store's issue
  of a shared call (`issue_shared_call`) issues the call only for an
  answer that says the record was made now (`newlyStarted`), for the same
  activity and work, and saves that answer before it hands the call back.
  The issued call (`SharedCallCustody`) is the only thing that sends a
  turn's text to the CLI, for Codex, Claude and Octos alike. An answer for
  a record made earlier, or no answer at all, therefore leaves the call
  unissued: nothing reaches the CLI, and the work's outcome stays unknown.
  A store test reads the sources and fails when any of this changes.
- For work the process had been given, this takes a second binding of one
  message on both sides. Each side keeps the first binding as a released
  copy, and released work is never bound again.
- A message that waits for its retry ends if the daemon does not come back
  for it in time: 3 minutes after the later of the end of the grace before
  its first process is given up and the daemon's first request to run it
  again. Its block payload then says `retry_lapsed`.
- A retry the daemon prepares while the server no longer lists the
  re-offer goes out without `reoffers`, and the message's later stops are
  reported as before.
- An Agent with an applied budget keeps the behaviour of main: the server
  takes nothing back from it, and the daemon asks for no retry for it.

"Send again" is what the sender has until the owner decides.

## Known bounds

- The steps of a computer's worker pass run one after another, so a start
  that waits for its first report holds every later step for at most the
  command's start window (30 seconds).
- A start the server refused for a message it still offers keeps its place,
  and the Agent's later messages wait behind that message, until it ends: at
  the latest when it has waited a day, or when the allowance period turns.
  A start refused for the Agent's applied budget is the one exception: it
  gives its place back at once (rule 3).
- An Agent with an applied budget can also be started for no message, after
  the process it follows. No message's end shows that the server will never
  make such a start, and the server keeps no record of one it refused, so
  it never gives its place back. The daemon makes no such start: the one
  function that asks for one is built into tests only. If one were made
  and refused, every later message after the same process would wait for
  as long as the Agent's reservation lasts. Only the end of the
  reservation, or the same start sent again and made, would end the wait;
  there is no other way out.
- A reader who did not send a message that arrived while its Agent was
  stopped keeps reading that the Agent is paused after it runs again.
- A message is offered while the room's status lists it. The status holds
  the 16 newest rows its reader is shown in the room, one for each Agent of
  a message: rows of the reader's own messages and of every message to an
  Agent whose computer is the reader's. In an encrypted room, rows of such
  messages that wait for an outdated or absent daemon to read them count
  too. A message that ended without a turn is no longer offered once 16
  newer such rows stand above it, which for the owner of a busy Agent can
  be soon.
- In review (Desktop #243 and #251): a computer whose clock is ahead holds
  what its clock leaves of each lease, and renews it by rule 2 before that
  runs out. Only a clock ahead by the whole lease loses it, and every
  process then ends at its first lease. After a sleep through a lease's
  end, the turn ends at its next renewal moment, at most ten seconds of
  awake time; the store refuses its records, and the commit gate its
  messages, from the wall-clock end on. On main, such a computer still
  loses as much of every lease as its clock is ahead, and ahead by the
  lease less the ten seconds to its renewal (20 seconds), no process
  outlives its first lease; and after a sleep through a lease's end, a
  turn on main runs on for the counted time that was left when the
  computer slept: 20 to 30 seconds with main's 30-second leases, and 110
  to 120 seconds with #243's two-minute leases if #251 did not land with
  it.
- On main, the answer that grants an Agent's execution lease is read without
  its time, so for that lease the wall clock is the only witness, and a
  computer whose clock is behind by more than a second is refused a
  full-length lease. In review (Desktop #246) the lease is counted from its
  request with the answer's date. Only an answer without a date, behind a
  proxy that drops it, still leaves the wall clock as the only witness; on a
  computer whose clock is behind, the run then ends as
  `execution_clock_behind` and says that the clock is behind.
- An Agent's execution lease that its daemon did not release stays with
  that daemon for up to 120 seconds from its last renewal. The same device
  under the same sign-in takes it back at once. Another device, or this one
  after a new sign-in, has to wait for it: on main a room's restore gives up
  after 35 seconds and the service tries again later; in review (Desktop
  #246) it waits up to 125 seconds, and a daemon that stops sends its
  release up to four times within 15 seconds while it gets no answer.
- A request to the server is given up after 15 seconds, 10 of them for
  connecting, whatever the window. A renewal that hangs that long, and the
  2 seconds before it is asked again, use 17 seconds of its lease: a
  30-second lease renewed with 20 seconds left has 3 seconds for one more
  try, and one renewed later has none; under a 120-second window about six
  tries fit. The window is the tolerance for a server that does not answer;
  the timeouts are not scaled to it.
- The daemon keeps at most 16 local sessions, each used again while 30 of its
  60 seconds are left, and its bridge serves at most 64 managers.
- The daemon's store only moves forward. Every store migration raises the
  store's schema version, and a build that finds a newer store than it knows
  refuses to open it ("state format is corrupt or newer than this
  binary"): no Agent on that computer runs again until a build at least as
  new is installed. A host is rolled back by a newer build that undoes the
  change, never by installing an older one. #252's, #256's and #259's place
  records (schemas 114, 115 and 116) are such steps, and so is every store
  migration in review here. A store migration is known by its file name and
  the SHA-256 of its SQL, not by its number alone. The store records both
  for each migration it runs, and every start compares them with the
  build's own list. A store that ran another build's migration of the same
  number, such as an earlier draft's, is refused by name ("store migration
  N was made by another build") before any later migration runs over it.
  A store from before these records takes the build's own for the versions
  it has, once: released builds come from the main branch, where a
  migration's number, name and SQL never change once it lands. A store
  that a draft build migrated must not be opened by a later release. Since
  Desktop #261 the release refuses it by name wherever the draft's step
  differs from its own, and no Agent on that computer runs until the store
  is replaced. A store migrated by a draft build from before #261 has no
  records and cannot be told apart: the release takes its own steps for
  the draft's. A build from a draft pull request therefore runs only on a
  daemon home made for it and thrown away afterwards.

## State

"Implemented" means merged into the main branch. Which release carries a
point is in the release notes. Pull requests named "in review" are open.

| Point | State |
|---|---|
| 2. Token refresh ahead of the expiry; one local session shared between calls | implemented |
| 2. One renewal moment for the activity lease, in all four drivers | in review (Desktop #243 and #251) |
| 5, 6. The notice names the Agent and offers "Send again", and so does each message that can be sent again | implemented (Desktop #244, #245, #248; server #99); the name and the button need a server whose status names the Agent of each row and says whether the message can be sent again |
| 3. Three classes of failure | in review (Desktop #242, #247; an owned Agent's steps and the session's renewal: #246). The server's list of the 503 answers that are decisions is pinned by tests (server #101) |
| 3. A refused start gives its place back when its message ends | implemented (Desktop #252); for an Agent with an applied budget, implemented (Desktop #256) |
| 3. A message whose process ended before its turn began is run again (option A, "What is left open") | in review (Desktop #249, server #102), waiting for the owner's decision |
| 3. No new start in the protocol before the budget for an Agent with an applied budget; a start the server refused for that budget gives its place back at once | implemented (Desktop #259) |
| 1. Leases follow the server's lease | Agent execution leases do. A session's end only grows, and an Agent is fenced only when its session is revoked (server #98, #103); each fence is recorded with its reason (server #100). Activity leases follow the window in review (Desktop #243, server #97). What is offered keeps 30 seconds |
| 4. The wall clock only ends a lease early | holds for a ticket and a start window; not yet for the session's proof, whose wall-clock end is in review (Desktop #246). For the activity lease it is in review (Desktop #243 and #251); on main the store still refuses an activity lease that reads longer than allowed by the wall clock, which a slow clock makes of every fresh lease. The Agent execution lease, the authorization of an owned Agent's reply and the session's proof: in review (Desktop #246) |
