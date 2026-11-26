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
   less left, so the lead stays near the window. The activity lease of a
   working process is renewed once, when 20 seconds of it are left; that rule
   has to change with the lease's length and is in review.
3. **A check that cannot be made is not a refusal.** Every failed step is
   decided, not asked, or without an answer, and that is worked out in one
   place. Only a decided failure ends work. The other two wait and ask again,
   and a wait keeps no local session between attempts.
4. **The computer's own clock decides nothing.** An end is the time that was
   left when the answer arrived, counted down from the moment the request was
   sent.

For the room:

5. **A message that ended without a turn says for which Agent.** When a
   message went to several Agents, the notice has a line for each one that
   has something to say, with its name. An Agent that did not process the
   message is named even when it was the only one.
6. **Its sender can send it again.** "Send again" sends the same message as a
   new one that mentions only the Agents that did not process it, in the same
   thread. An Agent that already answered is not asked twice. The first
   message stays as it ended.

   The server decides where it is offered, and says so for each Agent in the
   message's status. It offers it only where nothing of the message can have
   run (the message expired, every start failed, or the Agent's process ended
   before the turn began), only to the message's sender, and only while the
   Agent is in the room and has not been stopped. It is not offered for a run
   whose outcome is unknown, which may have acted or cost something and has
   its review instead, nor for a message that is still waiting.

   It is never pressed by the program. Two devices of one person would each
   send it, and one message would become two runs.

   What is sent is the message as its sender last edited it. A message that
   was removed cannot be sent again. When the new message cannot be sent
   (the room's encryption is not confirmed, the Agent's device is not ready),
   the room says so and offers it again; nothing is put back into the
   composer, where the text would address every Agent it first did.

   One click sends once: a message that was sent again is not offered again
   by this window, also after the room was left and opened again. The window
   does not keep that across a restart, and another device does not know it.
   There the offer returns until the new message has a status of its own, and
   the new message stands in the room for its sender to see.

A server that does not yet describe each Agent of a message is read as it
always was: one line from the message's first row, in the words each state
always had, and no "Send again".

## What is left open

Whether accepted work that never began should be given to the next process
without anyone asking. The owner's policy says that what is confirmed not to
have run is run again, and for a start that never happened it is (three more
attempts). For accepted work it needs a second binding of one message on both
sides. The server's document states the case; "Send again" is what the sender
has until it is decided.

## Known bounds

- All starts on one computer share one lane. A start that waits holds it for
  at most the 30 seconds an offer lasts.
- The daemon keeps at most 16 local sessions, each used again while 30 of its
  60 seconds are left, and its bridge serves at most 64 managers.

## State

| Point | State |
|---|---|
| 2. Token refresh ahead of the expiry; one local session shared between calls | implemented |
| 5, 6. The notice names the Agent and offers "Send again" | implemented with this record; the name and the button need a server whose status names the Agent of each row and says whether the message can be sent again |
| 3. Three classes of failure | in review |
| 1. Leases follow the server's lease | Agent execution leases do; activity leases and the rest are in review |
