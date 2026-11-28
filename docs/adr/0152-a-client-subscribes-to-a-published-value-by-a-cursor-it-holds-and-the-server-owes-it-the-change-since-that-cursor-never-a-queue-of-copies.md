# ADR 0152 — A client subscribes to a published value by a cursor it holds, and the server owes it the change since that cursor, never a queue of copies

- Status: Accepted (2026-11-28, by the owner), as amended at its review the same day: D1 (a subscription to a machine
  that keeps one session names no key), D9 (which machine keeps no session store, stated), D10 (`Authorized`, and the
  code held to the model by traces and by the client core's contract). The passages its sources were cited for were
  checked at acceptance (Sources). Nothing of it is built; its model (D10) is to be written and checked before any
  code, and the table of what each part taken out must make TLC find is a prediction until it is.
- Date: 2026-11-27
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: built on the backend a consumer needs, Rust first; the contract is
  general; what is special to one application stays in the application.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0152" line in the landing that moves it):
  `SCE_MESH.md` §8.1 (the four wire values it reserves for a subscription paired with a snapshot and its changes are
  taken, D4), §22 (the sentence that refuses a pattern needing a subscription, for the values of D1), §22.3 (what a
  response carries after the mailbox, D4) and §22.9 (the cursors are signed, D3); ADR 0113 D4 (one more row of the
  table of what is signed); ADR 0117 D6 and D7 (what a node keeps of a subscription, and what the surface's
  subscription is); ADR 0109 D1 and D16 (the bounds and the metrics of D8); `docs/SCE_SERVER_DEPLOY.md`;
  `docs/SCE_ACCEPTED_SUBSET.md`, the appendix.
- Scope: the Rust host core `sce-rust-mesh` (the pool, the `https` server, the client core of ADR 0117 D1),
  `sce-rust-session-store` (the value table, D2) and `sce-build` (what the deployment declares). The browser shell
  has it through the client core. Every other backend refuses by name (D9).
- Related: ADR 0062 (readers own their cursors), ADR 0101 (one form of bytes per value), ADR 0109 D10 (the durable
  outbox), ADR 0113 (an open set of instances), ADR 0114 (positions, and a follower's position in its state), ADR 0116
  (who a subscriber is), ADR 0117 (the client core and the browser node), ADR 0151 (a view, and its value per viewer).

## Context

What exists:

- **The mailbox of the `https` binding** (`SCE_MESH.md` §22.3): what a server machine sends a client waits in the
  client's mailbox, numbered, until a later request acknowledges it (`Sce-Mesh-Ack`); a held exchange is answered
  when an envelope arrives (long polling); a response carries at most `max_response_bytes`. Under `receipt:
  durable` the mailbox is the client's outbox in the session store, appended by the write that keeps the sending
  session's checkpoint (ADR 0109 D10; `backends/rust/session-store/src/outbox.rs`, `Write::outbox`), and **a full
  outbox refuses the send**: each envelope is owed exactly, so a client that does not read pushes back on the session
  that sends to it. The client's side of the numbering is `backends/rust/mesh/src/https/client_mailbox.rs`.
- **Subscriptions of other transports.** The envelope's patterns `EventSubscribe`, `EventUnsubscribe`,
  `EventNotify` and `FieldNotify` (`backends/rust/mesh/src/generated/pattern_kind.rs`, values 4 to 6 and 9) are
  realized where a connection or a discovery holds the subscriber: on a connection-oriented transport "a subscription
  lives exactly as long as its connection" (§10.4.4), and DDS keeps the last value for a reader that joins late
  (§8.2). The `https` binding has neither, and refuses "a pattern that needs a subscription or a field" (§22).
- **The four reserved values.** §8.1 reserves wire values 10 to 13 for "a subscription with an initial state snapshot
  delivered as a one-shot event, followed by delta-encoded change events", delivered to a machine as ordinary events,
  and rules out shared mutable state between sessions.
- **An open set of clients.** Under ADR 0113 a server knows its clients only as they arrive: an instance of an open
  machine, an incarnation that is offered, confirmed, leased and fenced. Its step 4, the membership in the store, is
  not built (`backends/rust/session-store/src` has no membership, and no write names `Write::under`).
- **Cursors.** ADR 0062's bounded log is read by cursors its readers own, and its writer never waits; ADR 0114 D4 keeps
  a follower's position in its own state, written with what the entry did.
- **A store whose versions only rise.** Every write of a session moves its version, and "numbers the store hands out,
  a version or a fence, come from one counter of the store that only rises, so neither is ever given twice"
  (`backends/rust/session-store/src/lib.rs`, `SessionStore`).
- **Values per viewer.** ADR 0151 publishes a view's value per viewer, and the published variables of a machine are
  in its snapshot.
- **The page's surface.** ADR 0117 D7 generates "a subscription to the published snapshot" of the node the page holds.
  Nothing gives a client a value its server publishes.

So a client of a server cannot be told that a value changed. Copying each change into each subscriber's outbox would
use what exists, and would make the slowest subscriber's full outbox refuse the session's next send: one slow client
would delay all.

## The textbook structure

- **A subscriber to a value is owed its latest state, not its history.** The value has one writer, so its states are
  ordered by a version, and a reader that holds an earlier version converges by being given the latest (state-based
  replication: Shapiro, Preguica, Baquero and Zawirski 2011; DDS's `KEEP_LAST(1)` with `TRANSIENT_LOCAL`, which
  §8.2 already cites as the current-value contract). Intermediate versions a slow reader never saw are not lost: they
  are superseded. A reader that needs every version follows a log (ADR 0114).
- **Readers own their cursors, and the writer never waits on a reader** (Kreps 2013; ADR 0062). Fan-out is done by
  each reader, at its own pace, from state the writer keeps once.
- **A change since a position is the rows stamped after it, and the keys removed after it**: a log compacted by key,
  which keeps each key's latest version and each removal's marker for a stated time, and a reader whose position is
  older than the oldest marker kept reads the whole state again (Kafka's compacted topics and their tombstones;
  Kubernetes' list and watch by resource version, whose too-old version is answered by a new list).
- **A snapshot may be read in pieces while the state changes**, and made exact by then reading every change since the
  oldest piece (a fuzzy checkpoint, made consistent by the log: Mohan et al., ARIES, 1992).
- **The end knows what it holds** (Saltzer, Reed and Clark 1984): the client, not the server, says which version it
  has, and the server's record of a subscription is what was asked, not what was delivered.
- **Publish and subscribe decouple the parties in time, in space and in synchronization** (Eugster, Felber, Guerraoui
  and Kermarrec 2003): a publisher never waits on a subscriber.
- **Access is lacking unless it is declared** (Saltzer and Schroeder 1975).

## Decision

### D1. What may be subscribed is declared, and a subscription names a value, not a client

A machine entry of the deployment may carry `subscribable:`, a list of its values that clients may subscribe to:
a published variable (`sce:direction="out"`) or a view's variable (ADR 0151 D9), each with its **audience**:

- `audience: viewer`, for a view with a viewer: any member of a binding of the machine may subscribe, and is given
  the value for itself (D9). The view's filter is the authorization.
- `audience: [<machine>, ...]`: instances of the open machines listed may subscribe, and each is given the same
  value. For a value of a keyed pool this is every key's value; a value that must differ by reader is a view with a
  viewer.

A value with no `subscribable:` entry is subscribed by no one. The deployment names machines and never a client: the
subscribers are the instances of ADR 0113, whose names the build cannot know. A subscription names the machine, the
key of the session (as `key_from` reads it; none for a machine that keeps one session, `key_from: none`, ADR 0114 D4)
and the value.

### D2. A value has a generation, versions and a changelog of keyed rows

For each value that has a subscriber, and for a view each viewer that has one, the server keeps the value's
**changelog** in the session store, beside the outbox and the journal:

- **Rows by key.** A view's value is its rows by the result's key, and its `total`; a published map is its entries
  by key; any other published value is one row of an empty key. Each row is kept as its deterministic CBOR bytes (ADR
  0101) with the **version** at which it last changed: the session's version, which the write that keeps the state
  gives and which never repeats (`SessionStore`).
- **Removals.** A key removed is kept as a marker with the version of its removal, at most `tombstones` of them for
  the value, a bound by name with no default (D8). The **floor** is the version of the newest marker dropped: below
  it, the changelog cannot say what was removed.
- **A generation**, 16 bytes drawn from the operating system's random source when the changelog starts, as a
  mailbox's incarnation is (§22.3). A changelog that is started again (its value's session was removed and started
  again, a view was given a new definition, a viewer was dropped and asked for again) has a new one, and a version of
  another generation means nothing in it.
- **Written with the state.** The rows and markers a run changed are written by the write that keeps the session's
  checkpoint (`Write::values`, beside `Write::outbox` and `Write::journal`), so a value's changelog is durable
  exactly when the state that made it is, and a restart keeps every version. For a view they are ADR 0151's output
  delta for each viewer kept; for a published map, its changed entries; for any other value, its one row when its
  bytes changed.
- **Nothing is written for bytes that did not change.** No version moves, and nothing is owed. A change outside a
  viewer's relations (ADR 0151 D7) is therefore not seen by the viewer even as a notification.

### D3. The client holds its cursor, and the server owes what is after it

- **A subscription is a record of the membership** (ADR 0113 D4): the instance, its incarnation, a number the client
  gives it, and the value it names. Subscribing is a request of the incarnation, numbered and recorded as ADR 0113 D5
  records every request, so subscribing twice under one number is one subscription. The record ends with the
  incarnation, and with it every subscription the incarnation made: what is forgotten together is dropped together,
  and a change of person, which is a change of incarnation (ADR 0117 D6), takes its subscriptions with it.
- **A cursor is the client's**: the generation and the version of what it holds of the value. Every request names the
  cursor of each of its subscriptions, in `Sce-Mesh-Values`, which is one more row of the table of what is signed
  (ADR 0113 D4), so no intermediary moves a cursor forward and makes a client miss a change.
- **What is owed** to a subscription with cursor `(g, c)` is: when `g` is the changelog's generation and `c` is at
  or above its floor, every row whose version is above `c` and every marker above `c`, up to the changelog's
  latest version; otherwise the whole value, as a reset. The changelog is indexed by version, so what is owed is
  found in time proportional to what is owed, never to the value.
- **The server keeps nothing per subscriber but its record.** It keeps in memory which subscriptions are **due**,
  those whose value's latest version is above the cursor their client last named, and the record of D3's first bullet
  in the store. No copy of a change is made for a subscriber.

### D4. It is carried by the exchange that carries the mailbox

The four values §8.1 reserves are taken by four patterns of the envelope:

| Value | Pattern | Direction | What it carries |
|---|---|---|---|
| 10 | `ValueSubscribe` | client to server | the subscription's number and the value it names; or, with no value, the number of one to end |
| 11 | `ValueReset` | server to client | a page of the whole value, in key order, at one version, and whether it is the last |
| 12 | `ValueChange` | server to client | the rows changed and the keys removed from one version to another |
| 13 | `ValueEnd` | server to client | that a subscription ended, and why: ended by the client, refused (no audience admits it, the bound of D8), the value gone |

Their data is the deterministic CBOR of a record of the binding's own schema. They are not numbered in the mailbox and
are not acknowledged by `Sce-Mesh-Ack`: a value envelope lost on the way is owed again, because the cursor the
client names next has not moved.

- **A response carries the mailbox and what is owed, in one budget.** Its `max_response_bytes` is shared: the
  mailbox's envelopes first, then the owed values, due subscriptions taken in turn so that one large value does not
  hold back the others, and each part carries at least one envelope when it has one, so neither starves the other. A
  held exchange is answered when an envelope arrives in the mailbox or a subscription becomes due.
- **A reset is read in pages**, each at the version it was read at. The client holds the value whole when it has the
  last page, and its cursor is then the oldest page's version, so the next exchange is owed every change since: the
  pages together with those changes are exact, as long as the floor has not passed that version, and a reset again if
  it has.
- **Values of one session are given at one version within a response**, so a client that holds two values of one
  session holds them as one state of it. Values of different sessions are each at their own version; nothing orders
  two sessions.

### D5. One slow client delays none

- **The session's write never waits on a client.** It writes each changed value's changelog once per value and
  viewer, however many subscribe, and marks the subscriptions of that value due: work proportional to the change and
  to the number of subscriptions it concerns, each a constant.
- **A client's delivery is made at its own exchange**, from state the server keeps once, on the exchange's own task of
  the server's asynchronous runtime. A client that reads its response slowly holds its connection and nothing else; a
  client that is absent costs its subscription's record and no more.
- **No bound one client can fill is one another depends on.** The outbox's bound pushes back by design, because each
  of its envelopes is owed exactly (ADR 0109 D10). A value is owed only its latest state, so a slow client is given
  less and later, and never stops another client or the session: conflation is the contract here, not a loss.

### D6. A machine that subscribes is given events

A subscriber that is a machine (the dialing side of a binding to the server, ADR 0113 D7) is given the value as an
external event, as §8.1 says a subscription is delivered: `<value>.changed`, whose data is the whole value, of the
type the publishing document declares, once for each version the client core takes in. The changes and the pages are
how the wire carries it, and the machine never sees them. A machine subscribes by a `<send>` of the binding's
standard event `sce.subscribe` naming the value, and ends one by `sce.unsubscribe`; `ValueEnd` is delivered as
`<value>.ended` with its reason.

### D7. The client keeps what it holds, as a follower keeps its position

The client core (ADR 0117 D1) takes a page or a change and the cursor it moves to in one input, and its effect is one
stored write of both: the value and its cursor are kept together, as a follower's position is kept with what the entry
did (ADR 0114 D4). The browser shell keeps them in IndexedDB under the incarnation (ADR 0117 D6), so a reload resumes
from the cursor it kept, and a cleared store subscribes again and is given a reset. The generated surface's
subscription (ADR 0117 D7) presents the value of a subscription and tells the page when its version moves; a value
the page subscribes to on its server and a snapshot of its own node are the same shape to it.

### D8. Bounds

`max_subscriptions`, how many subscriptions one instance holds; `max_viewers`, how many viewers of one view's value
are kept at once; `tombstones`, how many removal markers one changelog keeps. Each has no default and is written by
the deployment, as ADR 0113's `lease_ms` and `max_instances` are, and each is a bound by name (ADR 0050) so a
build moves it. A subscription past `max_subscriptions` or a viewer past `max_viewers` is refused with
`ValueEnd`, its reason the bound; a live subscription and a kept viewer are never evicted to make room. The server's
metrics (ADR 0109 D16) report, per value, the subscriptions due, the bytes owed, and the resets given and why.

### D9. Who a subscriber is, and the backends

The viewer of a subscription to a view with a viewer is the subscription's principal (ADR 0116 D1): the person its
incarnation acts for, or the instance where the view's viewer is `of="instance"`; a view of persons refuses a
subscription from an incarnation that acts for none. The audience is judged when the subscription is made and again at
every exchange that delivers it, so a delegation that ended (ADR 0116 D6) delivers nothing more: it ends the
incarnation, and the subscriptions with it.

Rust, as the server and as the dialing side, native and in the browser through the client core. `subscribable:` on a
machine of another backend, on a transport other than `https`, or on a machine that is no keyed pool (ADR 0107), whose
sessions no session store keeps and whose values therefore have no changelog, and an audience that names a machine that is not open, are refused by name where the
deployment is read; the subscriptions of the connection-oriented transports (patterns 4 to 6 and 9)
are as they are.

### D10. The protocol is a model before it is code

`docs/formal/ValueService.tla` states the promise: a client that keeps exchanging comes to hold the latest version of
each value it subscribes to, whatever the other clients do (`Converges`, under fairness of that client alone); what
a client holds at a version is the value published at that version (`Exact`); within one generation a client's
version never goes back (`Monotone`); a client is given nothing for a version whose bytes equal what it holds
(`Silent`); nothing is delivered for a subscription after its incarnation ended, nor to a viewer its audience no longer
admits (`Authorized`). `docs/formal/Subscriptions.tla` refines it: a server that writes versions, crashes, drops markers past
its floor and starts a changelog again; clients that subscribe, go absent, read slowly, lose their store and stop; a
network that loses, repeats, delays and reorders; and an intermediary that changes what is not signed. It is checked by
`scripts/formal/check.sh` before any code. What each part taken out must make TLC find:

| Taken out | What TLC must find |
|---|---|
| the removal markers | a client that holds a row that was removed (`Exact`) |
| the floor's check | the same, after the marker was dropped |
| the changelog written with the state | after a crash, a client that holds a version whose rows the server no longer has (`Exact`) |
| the generation | after a changelog is started again, a cursor read in another numbering (`Exact`) |
| the signature over the cursors | a cursor moved forward, and a change never delivered (`Converges`) |
| the reset's catching up from its oldest page | a value assembled from pages of two versions (`Exact`) |
| the value and its cursor in one stored write | after a client stops, a cursor past what it holds (`Exact`) |
| delivery from the changelog (a queue per client instead, bounded) | a client that never reads stops the writer, and the others (`Converges`) |
| nothing written for unchanged bytes | a change outside a viewer's relations delivered to it (`Silent`) |
| the subscriptions ended with their incarnation | a change delivered to an incarnation that ended, or to a person whose delegation ended (`Authorized`) |

The model is bounded: it does not cover the cost of what is owed, the pages' sizes, or the cryptography of the
signature.

**The code is held to the model** (ADR 0115 D2, D4 and D13). The server's due set, what a response carries and the
reset's pages are traced from the runs of the server suite and checked as behaviours of `Subscriptions.tla`; the client
core's taking in of a page or a change (D7) carries a Verus contract, as the `https` client core of ADR 0117 does:
within a generation its cursor never goes back, and what it holds at a version is what it was given for that version.

## Not in scope

Every version of a value (a journal's follower, ADR 0114); push to a page that is not running and transports other than
`https` (ADR 0117); a subscription to another server, and subscriptions forwarded between servers; a query a client
writes; consistency across two sessions' values; and any one application's values.

## Consequences

- A client of a public server is told when a value it may read changes, and resumes after an absence from what it
  holds, across a restart of either side.
- A server holds per subscriber a record and nothing else, so its cost grows with the values and the viewers kept, not
  with how slowly anyone reads.
- A value that changes faster than a client reads is given to that client less often; a client that must see each
  change follows a journal instead.
- The outbox keeps its contract: what is owed exactly still pushes back, and a value no longer passes through it.
- A long absence costs a reset, whose size is the value's: the deployment's `tombstones` decides how long an absence
  is short.

## Alternatives rejected

- **Each change copied into each subscriber's outbox.** A full outbox refuses the session's send (ADR 0109 D10), so
  the slowest subscriber would stop the publisher; and the server would hold, per subscriber, every change since it
  last read.
- **A bounded log per value that forgets its oldest entries** (ADR 0062). It suits readers that must see each entry
  and can be told how many they missed. A subscriber of a value is owed the value, and a reader that lagged would
  have to be told to start again where the changelog gives it exactly what it lacks.
- **The cursor kept by the server.** The server would believe what it sent was received; the client knows what it
  holds, and a cursor the server keeps is lost or wrong after either side restarts.
- **The whole value at each change.** Correct, and its cost is the value's size times the changes times the
  subscribers.
- **A second channel that stays open** (WebSocket, server-sent events). The binding exists so that any network that
  carries HTTPS requests carries SCE (§22); a held exchange already wakes a client when something is due.
- **A subscription that lives as long as its connection** (§10.4.4). An exchange of this binding is not a connection,
  and an absent client must resume.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. The model of D10, checked, with each configuration of its table failing as predicted, or the record amended by
   what TLC finds.
3. The store: `Write::values`, the changelog, its versions, markers, floor and generation, in memory and in SQLite,
   with its conformance kit (`backends/rust/session-store/tests/every_value_store_keeps_one_contract.rs`): a write
   refused for its fence or its version leaves no row of a value changed; what is owed after a version is exactly the
   rows and markers above it; a version below the floor is answered as a reset; a restart keeps every version. Red
   without it: the store has no value table.
4. The build: `subscribable:`, its audiences, the bounds of D8, the four patterns in the envelope's document, and
   each refusal with its appendix row. Red without it: `subscribable:` is an unknown key.
5. The server: the subscription records in the membership's store, the due set, what a response carries, the reset's
   pages, and their traces checked against the model (D10). It needs ADR 0113 step 4. In the server suite
   (`backends/rust/mesh/tests/a_value_reaches_every_subscriber_and_waits_for_none.rs`): 200 instances never listed,
   each subscribed to 5 values; one change reaches every one of them within the hold; one client that reads its
   response one byte at a time and one that stops polling leave the others' delivery and the session's write times
   within the bounds measured without them; the server killed between any two writes, and every client resuming
   without a reset; a client absent past its floor given one reset and then its changes.
6. The client core with its contract proved, and the native shell (D7), and the machine's events (D6); then the browser shell, with ADR 0117
   step 4.
7. The documents named in "Amends".

## Open questions

Decided as recommended, for the owner's acceptance, each with its reason.

1. A change is sent as the rows changed and the keys removed, and a value that is not keyed as one row: the unit of a
   change is what a reader can apply without the history before it.
2. The cursor rides every request and is signed, rather than kept by the server: the client knows what it holds
   (D3).
3. A client is owed the latest version, not every version: a value is its state, and its history is a journal's
   (ADR 0114).
4. `tombstones`, `max_subscriptions` and `max_viewers` have no default: how long an absence is short, and how many
   readers a server serves, are the deployment's, and no number here is guessed.
5. Values of one session are delivered at one version within a response, and of two sessions at their own: a session
   is the unit of a consistent state, and two sessions share no order.

## Sources

Shapiro, Preguica, Baquero and Zawirski 2011 (conflict-free replicated data types; state-based convergence); Kreps 2013
(the log); Eugster, Felber, Guerraoui and Kermarrec 2003 (the many faces of publish and subscribe); Mohan, Haderle,
Lindsay, Pirahesh and Schwarz 1992 (ARIES; fuzzy checkpoints); Saltzer, Reed and Clark 1984 (end-to-end arguments);
Saltzer and Schroeder 1975 (fail-safe defaults); the OMG DDS specification (`KEEP_LAST` and `TRANSIENT_LOCAL`, as
`SCE_MESH.md` §8.2 states them); the Apache Kafka documentation (log compaction, and the retention of a removal's
marker); the Kubernetes API concepts (list and watch by resource version, and the answer to a version too old). They
are cited from the texts as known. The documentation of Kafka and Kubernetes is cited for practice and not as a
foundation; both passages were checked at acceptance (2026-11-28): Kafka keeps a removal's marker for
`delete.retention.ms`, and a consumer that reaches the head of the log within it sees every removal; a Kubernetes watch
from a version older than the history kept is answered `410 Gone`, and the client lists again.
