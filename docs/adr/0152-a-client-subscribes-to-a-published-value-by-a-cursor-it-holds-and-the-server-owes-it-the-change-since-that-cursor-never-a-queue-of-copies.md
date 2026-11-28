# ADR 0152 — A client subscribes to a published value by a cursor it holds, and the server owes it the change since that cursor, never a queue of copies

- Status: Accepted (2026-11-28) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after an independent review; amended on
  acceptance as listed in "Amended on acceptance". Amended the same day by the owner (first landed in ca877ecb830, and
  in the model's writing, and kept here over the review's text): D1 (a subscription to a machine that keeps one session
  names no key), D10 (`Authorized`, `Silent` per version, `Converges` with the session's publishing, and the code held
  to the model) ("Amended on acceptance", 14 to 16).
- Built: nothing. Its model (D10) is to be written and checked before any code, and the table of what each part taken
  out must make TLC find is a prediction until it is.
- Date: 2026-11-27
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: built on the backend a consumer needs, Rust first; the contract
  is general; what is special to one application stays in the application.
- Amends (each amended document gains an "Amended by: ADR 0152" line in the landing that moves it): `SCE_MESH.md` §8.1
  (the four wire values it reserves for a subscription paired with a snapshot and its changes are taken, D4; and its
  sentence that the snapshot and each delta reach the machine as events, which becomes the whole value as one event,
  D6), §22 (the sentence that refuses a pattern needing a subscription or a field, for the values of D1), §22.3 (what
  a response carries after the mailbox, D4) and §22.9 (a response covers the `content-digest` of the request it
  answers, which covers the cursors, D3); ADR 0113 D4 (that row of the table of what is signed); ADR 0117 D6 and D7
  (what a node keeps of a subscription, and what the surface's subscription is); ADR 0153 D3 and D4 (a changelog is a
  holder of grants, and a delivery from it is a crossing, D2); ADR 0109 D1 and D16 (the bounds and the metrics of D8);
  `docs/SCE_SERVER_DEPLOY.md`; `docs/SCE_ACCEPTED_SUBSET.md`, the appendix.
- Scope: the Rust host core `sce-rust-mesh` (the pool, the `https` server, the client core of ADR 0117 D1),
  `sce-rust-session-store` (the value table, D2) and `sce-build` (what the deployment declares). The browser shell has
  it through the client core. Every other backend refuses by name (D9).
- Related: ADR 0062 (readers own their cursors), ADR 0101 (one form of bytes per value), ADR 0107 (a keyed pool and
  its store), ADR 0109 D10 (the durable outbox), ADR 0113 (an open set of instances), ADR 0114 (positions, and a
  follower's position in its state), ADR 0116 (who a subscriber is), ADR 0117 (the client core and the browser node),
  ADR 0151 (a view, and its value per viewer), ADR 0153 (a blob, held by grants).

## Context

What exists:

- **The mailbox of the `https` binding** (`SCE_MESH.md` §22.3): what a server machine sends a client waits in the
  client's mailbox, numbered, until a later request acknowledges it (`Sce-Mesh-Ack`); a held exchange is answered when
  an envelope arrives (long polling); a response carries at most `max_response_bytes`. Under `receipt: durable` the
  mailbox is the client's outbox in the session store, appended by the write that keeps the sending session's
  checkpoint (ADR 0109 D10; `backends/rust/session-store/src/outbox.rs`, `Write::outbox`), and **a full outbox refuses
  the send**: each envelope is owed exactly, so a client that does not read pushes back on the session that sends to
  it. The client's side of the numbering is `backends/rust/mesh/src/https/client_mailbox.rs`. A client keeps at most
  two exchanges in flight (§22.3).
- **Subscriptions of other transports.** The envelope's patterns `EventSubscribe`, `EventUnsubscribe`, `EventNotify`
  and `FieldNotify` (`backends/rust/mesh/src/generated/pattern_kind.rs`, values 4 to 6 and 9) are realized where a
  connection or a discovery holds the subscriber: on a connection-oriented transport "a subscription lives exactly as
  long as its connection" (§10.4.4), and DDS keeps the last value for a reader that joins late (§8.2). A document
  names them by one vocabulary on every transport (`event.subscribe.X`, `event.unsubscribe.X`, `field.notify.X`;
  §8.1), so that a deployment alone chooses the middleware. The `https` binding has no subscription, and refuses "a
  pattern that needs a subscription or a field" (§22).
- **The four reserved values.** §8.1 reserves wire values 10 to 13 for "a subscription with an initial state snapshot
  delivered as a one-shot event, followed by delta-encoded change events", delivered to a machine as ordinary events,
  and rules out shared mutable state between sessions.
- **An open set of clients.** Under ADR 0113 a server knows its clients only as they arrive: an instance of an open
  machine, an incarnation that is offered, confirmed, leased and fenced. Its steps 3 to 6 landed (2026-11-22 to
  2026-11-28): the membership is in the store (`backends/rust/session-store/src/membership.rs`), and the write that
  keeps a run names each incarnation the run was made under (`Write::under`), refused whole when the store no longer
  holds one (`Conflict::Incarnation`).
- **Cursors.** ADR 0062's bounded log is read by cursors its readers own, and its writer never waits; ADR 0114 D4
  keeps a follower's position in its own state, written with what the entry did.
- **A store whose versions only rise.** Every write of a session moves its version, and "numbers the store hands out,
  a version or a fence, come from one counter of the store that only rises, so neither is ever given twice"
  (`backends/rust/session-store/src/lib.rs`, `SessionStore`).
- **Values per viewer.** ADR 0151 publishes a view's value per viewer, and the published variables of a machine are in
  its snapshot. A view's operator state is derived and not saved (ADR 0151 D4).
- **Blobs by grant.** ADR 0153 gives each holder of a blob a grant and a handle of its own, and translates a reference
  wherever it crosses from one holder to another (D3).
- **The page's surface.** ADR 0117 D7 generates "a subscription to the published snapshot" of the node the page holds.
  Nothing gives a client a value its server publishes.

So a client of a server cannot be told that a value changed. Copying each change into each subscriber's outbox would
use what exists, and would make the slowest subscriber's full outbox refuse the session's next send: one slow client
would delay all.

## The textbook structure

- **A subscriber to a value is owed its latest state, not its history.** The value has one writer, so its states are
  ordered by a version, and a reader that holds an earlier version converges by being given the latest (state-based
  replication: Shapiro, Preguica, Baquero and Zawirski 2011; state transfer rather than operation transfer: Saito and
  Shapiro 2005; DDS's `KEEP_LAST(1)` with `TRANSIENT_LOCAL`, which §8.2 already cites as the current-value contract).
  Intermediate versions a slow reader never saw are not lost: they are superseded. A reader that needs every version
  follows a log (ADR 0114).
- **Readers own their cursors, and the writer never waits on a reader** (Kleppmann 2017, chapter 11; ADR 0062).
  Fan-out is done by each reader, at its own pace, from state the writer keeps once.
- **A reader is sent what changed since the state it holds, and the whole state when that is no longer known**
  (delta-state replication: Almeida, Shoker and Baquero 2018). What changed since a position is the rows stamped after
  it and the keys removed after it: a log compacted by key, which keeps each key's latest version and each removal's
  marker for at least a stated time, the time a reader has to read the whole state (Kafka's compacted topics, their
  tombstones and `delete.retention.ms`). A reader whose position is older than the oldest marker kept reads the whole
  state again (Kubernetes' list and watch by resource version, whose too-old version is answered by a new list).
- **A snapshot may be read in pieces while the state changes**, each piece at its own position, and made exact by
  every change since the first piece's position (DBLog: Andreakis and Papapanagiotou 2020). The reader holds its place
  between pieces, as a Kubernetes list read in chunks is continued by a token its client holds.
- **The end knows what it holds** (Saltzer, Reed and Clark 1984): the client, not the server, says which version it
  has, as a conditional request names the validator of what its client holds (RFC 9110 section 13), and the server's
  record of a subscription is what was asked, not what was delivered.
- **Publish and subscribe decouple the parties in time, in space and in synchronization** (Eugster, Felber, Guerraoui
  and Kermarrec 2003): a publisher never waits on a subscriber.
- **Access is lacking unless it is declared** (Saltzer and Schroeder 1975).

## Decision

### D1. What may be subscribed is declared, and a subscription names a value, not a client

A machine entry of the deployment may carry `subscribable:`, a list of its values that clients may subscribe to: a
published variable (`sce:direction="out"`) or a view's variable (ADR 0151 D9), each with its **audience**:

- `audience: viewer`, for a view with a viewer: any member of a binding of the machine may subscribe, and is given the
  value for itself (D9). The view's filter is the authorization.
- `audience: [<machine>, ...]`: instances of the open machines listed may subscribe, and each is given the same value.
  For a value of a keyed pool this is every key's value; a value that must differ by reader is a view with a viewer.

A value with no `subscribable:` entry is subscribed by no one. The deployment names machines and never a client: the
subscribers are the instances of ADR 0113, whose names the build cannot know. A subscription names the machine, the
key of the session (as `key_from` reads it; none for a machine that keeps one session, `key_from: none`, ADR 0114 D4)
and the value.

### D2. A value has a generation, versions and a changelog of keyed rows

For each value that has a subscriber, and for a view each viewer that has one, the server keeps the value's
**changelog** in the session store, beside the outbox and the journal:

- **Rows by key.** A view's value is its rows by the result's key, and its `total`; a published map is its entries by
  key; any other published value is one row of an empty key. Each row is kept as its deterministic CBOR bytes (ADR
  0101) with the **version** at which it last changed: the session's version, which the write that keeps the state
  gives and which never repeats (`SessionStore`).
- **Removals.** A key removed is kept as a marker with the version of its removal and the time of the write that
  removed it (`Write::now_ms`). A marker is dropped when it is older than `tombstone_retention_ms`, or, the oldest
  first, when the value holds more than `tombstones` markers. Both are bounds by name with no default (D8): the time
  is how long a client has to finish a reset (D4), and the count bounds what the changelog stores (ADR 0109 D1). The
  **floor** is the version of the newest marker dropped: below it, the changelog cannot say what was removed. A marker
  dropped by the count before its time is reported (D8): the deployment's `tombstones` is then smaller than the
  removals of `tombstone_retention_ms`.
- **A generation**, 16 bytes drawn from the operating system's random source when the changelog starts, as a mailbox's
  incarnation is (§22.3). A changelog that is started again (its value's session was removed and started again, a view
  was given a new definition, a viewer was dropped and asked for again) has a new one, and a version of another
  generation means nothing in it.
- **Written with the state.** The rows and markers a run changed are written by the write that keeps the session's
  checkpoint (`Write::values`, beside `Write::outbox` and `Write::journal`), so a value's changelog is durable exactly
  when the state that made it is, and a restart keeps every version. For a view they are ADR 0151's output delta for
  each viewer kept; for a published map, its changed entries; for any other value, its one row when its bytes changed.
- **An output, not a saved state of the view.** ADR 0151 D4 saves no operator state of a view, and builds it again
  from the tables at a restore. The changelog is not that state: it is what the view gave, kept as the outbox keeps
  what a session sent, and nothing reads it to compute the view. It is held to agree with the tables: after a restore,
  the rows of each changelog are the value computed from the restored tables at the session's version. The store's kit
  holds that one write keeps the state and the changelog or neither (Order of work, step 3), and the server suite
  holds the agreement across a restart (step 5).
- **Nothing is written for bytes that did not change.** No version moves, and nothing is owed. A change outside a
  viewer's relations (ADR 0151 D7) is therefore not seen by the viewer even as a notification.
- **A blob a row names is held by the changelog's own grant** (ADR 0153). A row whose value reaches a `record:BlobRef`
  keeps the reference with `handle` empty, as a journal entry does (ADR 0153 D3), and the write that keeps the row
  gives the changelog a grant of each blob it names; a row changed or removed, and a changelog started again, end the
  grants of what it named. Delivering a row is a crossing: the server resolves each address under the changelog's
  grant, gives the subscriber's incarnation a grant and its own handle, leased as every client's grant is (ADR 0153
  D4), and delivers the row with that handle. No client is given a handle of the session's, and no blob a subscriber
  was given is collected while its grant lasts.

### D3. The client holds its cursor, and the server owes what is after it

- **A subscription is a record of the membership** (ADR 0113 D4): the instance, its incarnation, a number the client
  gives it, and the value it names. The client numbers its subscriptions in rising order within its incarnation, and
  subscribing is a request of the incarnation, recorded as ADR 0113 D5 records every request, so subscribing twice
  under one number is one subscription. The record holds the live subscriptions and the highest number made: a
  `ValueSubscribe` of a number at or below it that names no live subscription is answered `ValueEnd` and records
  nothing, so a repeat that arrives after its subscription ended does not start it again. The record ends with the
  incarnation, and with it every subscription the incarnation made: what is forgotten together is dropped together,
  and a change of person, which is a change of incarnation (ADR 0117 D6), takes its subscriptions with it.
- **A cursor is the client's**, in one of three forms:
  - `(g, c)`: the client holds the value whole, at version `c` of generation `g`;
  - `(g, f, k)`: the client is reading a reset of generation `g` whose first page was read at version `f`, and its
    next page starts after key `k` (Kubernetes' `continue` token, held by the client and not by the server);
  - none: a new subscription, or a client whose store was lost.
- **The cursors travel in the body, signed.** Every request carries, for each of the client's subscriptions, a
  `ValueSubscribe` naming its number, its value and its cursor (D4). The body is bound to the request's signature by
  its `Content-Digest` (RFC 9530), and a response covers the `content-digest` of the request it answers
  (`content-digest;req`, §22.9), so no intermediary moves a cursor forward and makes a client miss a change, and no
  response is taken as the answer to cursors its request did not name. The cursors are not in a header because an
  intermediary limits a header's size (RFC 9110 section 5.4; status 431, RFC 6585), and this binding exists to pass
  any intermediary that carries HTTPS. Both directions are signed only where the binding signs them: requests under
  `authenticator: key`, responses under the server's `sign:` and the client's `verify:` (§22.9). D9 refuses
  `subscribable:` on any other binding.
- **What is owed** to a subscription is the change since its cursor, or the whole value when that is not known
  (Almeida, Shoker and Baquero 2018):
  - to `(g, c)`, when `g` is the changelog's generation and `c` is at or above its floor: every row whose version is
    above `c` and every marker above `c`, up to the version of the session the response reads; otherwise a reset from
    the first key;
  - to `(g, f, k)`, when `g` is the changelog's generation and `f` is at or above its floor: the next page, from the
    first key after `k`; otherwise a reset from the first key;
  - to none: a reset from the first key.
  The changelog is indexed by version and by key, so what is owed is found in time proportional to what is owed, never
  to the value.
- **The server keeps nothing per subscriber but its record.** It keeps in memory which subscriptions are **due**,
  those whose value's latest version is above the cursor their client last named, and the record of this decision's
  first bullet in the store; for a value that names blobs, the grants of what it delivered (D2), each leased. No copy
  of a change is made for a subscriber.

### D4. It is carried by the exchange that carries the mailbox

The four values §8.1 reserves are taken by four patterns of the envelope:

| Value | Pattern | Direction | What it carries |
|---|---|---|---|
| 10 | `ValueSubscribe` | client to server | a subscription's number, the value it names and the cursor the client holds of it (D3); or, with no value, the number of one to end |
| 11 | `ValueReset` | server to client | one page of one value: its generation, the version of the session it was read at, the version the reset's first page was read at, the key range it covers, its rows in key order, and whether it is the last |
| 12 | `ValueChange` | server to client | one unit of one session (below): the version of the session it brings the client to, and for each value it moves, the version it moves it from, the rows changed and the keys removed between them |
| 13 | `ValueEnd` | server to client | that a subscription ended, and why, one of a closed set: ended by the client, refused (no audience admits it, the binding does not sign, a bound of D8, a number used before), the value gone |

Their data is the deterministic CBOR of a record of the binding's own schema. They are not numbered in the mailbox,
are not acknowledged by `Sce-Mesh-Ack`, and do not pass the §10.5 window: each states what the client holds or what it
is owed, and a repeat changes nothing. A value envelope lost on the way is owed again, because the cursor the client
names next has not moved. A `ValueSubscribe` of a number the incarnation already holds for the same value writes
nothing: it names the cursor.

- **A response carries the mailbox and what is owed, in one budget.** Its `max_response_bytes` is shared: the
  mailbox's envelopes first, then what is owed, the due sessions taken in turn so that one large value does not hold
  back the others. A unit and a page are each carried whole, and each part carries at least its first, even when it is
  larger than what remains of the budget, as the mailbox's first envelope is (§22.3), so neither starves the other. A
  held exchange is answered when an envelope arrives in the mailbox or a subscription becomes due.
- **A response reads each session at one version.** What it carries for the subscriptions of one session, its pages
  and its unit, is read at one version of that session.
- **A session's changes are one unit, all or none.** What brings the client to the version the response read for one
  session is one `ValueChange`: the change of each value of the session the client holds whole, from the version it
  holds, and the change of each value whose last page the response carries, from the version of its first page. Every
  value of the session the client holds whole moves to the unit's version, whether its rows changed or not, so a
  client holds the values of one session as one state of it. Values of different sessions are each at their own
  version; nothing orders two sessions.
- **A reset is read in pages.** Each page carries the key range it covers, and the next starts after its last key;
  between pages the client's cursor is `(g, f, k)`. A page that is not the last is carried on its own. The last is
  carried with its session's unit, which brings the value from `f` to the unit's version: the pages, each read at a
  version at or after `f`, with every change since `f`, are the value at the unit's version, as long as the floor has
  not passed `f` (D2), and a reset from the first key again if it has.
- **A request's cursors fit its body.** The deployment is refused where `max_subscriptions` cursors at their largest
  do not fit in the binding's `max_body_bytes` (D9).

### D5. One slow client delays none

- **The session's write never waits on a client.** It writes each changed value's changelog once per value and viewer,
  however many subscribe, and marks the subscriptions of that value due: work proportional to the change and to the
  number of subscriptions it concerns, each a constant.
- **A client's delivery is made at its own exchange**, from state the server keeps once, on the exchange's own task of
  the server's asynchronous runtime. A client that reads its response slowly holds its connection and nothing else; a
  client that is absent costs its subscription's record and no more.
- **No bound one client can fill is one another depends on.** The outbox's bound pushes back by design, because each
  of its envelopes is owed exactly (ADR 0109 D10). A value is owed only its latest state, so a slow client is given
  less and later, and never stops another client or the session: conflation is the contract here, not a loss.

### D6. A machine that subscribes uses the vocabulary of every transport

A subscriber that is a machine (the dialing side of a binding to the server, ADR 0113 D7) names a subscription as a
document does on every transport (§8.1), so the same document runs on every binding that carries it and the deployment
alone chooses the middleware. It subscribes by `<send event="event.subscribe.X">` on the binding, `X` the name of a
subscribable value, and ends the subscription by `event.unsubscribe.X`: written by the author, paired with the
subscribe at the state's exit by the build, or held for the machine's life by the deployment's `subscriptions:` list,
as on every transport (§13). Over `https` the build carries them as `ValueSubscribe` (pattern 10). The value reaches
the machine as `field.notify.X`, the current-value notification a SOME/IP field's notifier gives, whose data is the
whole value, of the type the publishing document declares, once for each unit or completed reset the client core takes
in that changes it. The pages and the changes are how the wire carries it, and the machine never sees them. A
`ValueEnd` the client did not ask for reaches the machine as `error.communication`, its data naming `X` and the
reason, as a refused subscription of another transport does (§8). §8.1's sentence that the snapshot and each delta
reach the machine as events is amended to this: the whole value, as one event, for each change the client core takes
in.

### D7. The client keeps what it holds, as a follower keeps its position

The client core (ADR 0117 D1) takes what a response owes it by one rule each, because two exchanges are in flight
(§22.3) and the network repeats, delays and reorders:

- **A unit** (`ValueChange`) is applied only when, for each value it moves, its `from` is at or below the version the
  client holds of that value and that version is below its `to`; a value whose pages are all in holds its first page's
  version. Otherwise the unit is dropped whole, and the cursors the client names next are owed it again. Since the
  client holds the values of one session at one version (D4), the rule takes or drops a unit as one.
- **A page** (`ValueReset`) is taken only when it continues the cursor the client holds now: a first page when that
  cursor is the one its request named, a later page when its generation, its first page's version and the start of its
  key range are those of the cursor `(g, f, k)`. Anything else is dropped. The pages are put together by their key
  ranges.
- **A value in a reset is presented to nothing** until the unit that carries its last page is applied.

Its effect is one stored write: the values a response moves and their cursors are kept together, the values of a unit
all or none, as a follower's position is kept with what the entry did (ADR 0114 D4). The browser shell keeps them in
IndexedDB under the incarnation (ADR 0117 D6), so a reload resumes from the cursor it kept, and a cleared store
subscribes again and is given a reset. The generated surface's subscription (ADR 0117 D7) presents the value of a
subscription and tells the page when its version moves; a value the page subscribes to on its server and a snapshot of
its own node are the same shape to it.

### D8. Bounds

`max_subscriptions`, how many subscriptions one instance holds; `max_viewers`, how many viewers of one view's value
are kept at once; `tombstones`, how many removal markers one changelog keeps; `tombstone_retention_ms`, how long a
removal marker is kept unless `tombstones` drops it first, which is how long a client has to finish a reset (D2). Each
has no default and is written by the deployment, as ADR 0113's `lease_ms` and `max_instances` are, and each is a bound
by name (ADR 0050) so a build moves it. A subscription past `max_subscriptions` or a viewer past `max_viewers` is
refused with `ValueEnd`, its reason the bound; a live subscription and a kept viewer are never evicted to make room.
The server's metrics (ADR 0109 D16) report, per value, the subscriptions due, the bytes owed, the resets given and
why, and the markers dropped by `tombstones` before `tombstone_retention_ms`.

### D9. Who a subscriber is, and the backends

The viewer of a subscription to a view with a viewer is the subscription's principal (ADR 0116 D1): the person its
incarnation acts for, or the instance where the view's viewer is `of="instance"`; a view of persons refuses a
subscription from an incarnation that acts for none. The audience is judged when the subscription is made and again at
every exchange that delivers it, so a delegation that ended (ADR 0116 D6) delivers nothing more: it ends the
incarnation, and the subscriptions with it.

Rust, as the server and as the dialing side, native and in the browser through the client core. Where the deployment
is read, `subscribable:` is refused by name: on a machine of another backend; on a transport other than `https`; on a
binding whose authenticator is not `key`, or that names no `sign:` on the server's side or no `verify:` on the
client's (D3), as ADR 0113 D4 already requires of an open machine's binding under `key`; on a machine whose sessions
no durable session store keeps, that is a machine that is not a keyed pool (ADR 0107 D3) or a keyed pool whose store
is not durable (ADR 0107 D6); where `max_subscriptions` cursors at their largest do not fit in the binding's
`max_body_bytes` (D4); and with an audience that names a machine that is not open. The subscriptions of the
connection-oriented transports (patterns 4 to 6 and 9) are as they are.

### D10. The protocol is a model before it is code

`docs/formal/ValueService.tla` states the promise:

- `Converges`: the session publishes every version it is asked to, whatever any client does; and for every version
  `v` of a value published while a client is subscribed to it, a client that keeps
  exchanging comes to hold the value at `v` or at a later version, or is told that the subscription ended, whatever
  the other clients do (under fairness of that client alone), as long as each reset it is given completes within
  `tombstone_retention_ms` and the value's removals within that time are at most `tombstones` (D2).
- `Exact`: what a client holds of a value at a version is the value published at that version.
- `Monotone`: within one generation a client's version of a value never goes back.
- `OneState`: the values of one session a client holds whole are of one version of that session.
- `Silent`: a client moves only to a version at which the value's bytes changed, so a change that leaves a viewer's
  value as it was gives that viewer nothing. (Stated per version: a client whose cursor is before a change and before
  the change that undoes it is owed both rows by D3, and is given bytes equal to what it held at a version that moved.)
- `Authorized`: nothing is delivered for a subscription after its incarnation ended, nor to a viewer its audience no
  longer admits (D9).

`docs/formal/Subscriptions.tla` refines it: a server that writes versions, crashes, drops markers by their time and by
their count and starts a changelog again; clients that subscribe, go absent, read slowly, lose their store and stop,
each with two exchanges in flight; a network that loses, repeats, delays and reorders; and an intermediary that
changes what is not signed; and incarnations that end, taking their subscriptions. It is checked by `sce-codegen
formal models` (ADR 0146) before any code. What each part taken out must
make TLC find:

| Taken out | What TLC must find |
|---|---|
| the removal markers | a client that holds a row that was removed (`Exact`) |
| the floor's check | the same, after the marker was dropped |
| the markers' time (markers bounded by their count alone) | a value that removes keys while a client reads its pages: the floor passes the first page every time, and the reset never completes (`Converges`) |
| the changelog written with the state | after a crash, a client that holds a version whose rows the server no longer has (`Exact`) |
| the generation | after a changelog is started again, a cursor read in another numbering (`Exact`) |
| the request's digest over the cursors, or the response's `content-digest;req` | a cursor moved forward, and a change never delivered (`Converges`) |
| the reset's cursor `(g, f, k)` | a reset whose pages restart at its first key at each exchange, and never complete (`Converges`) |
| the reset's catching up from its first page | a value assembled from pages of two versions (`Exact`) |
| the client's rule for a unit | a unit that comes late applied over a newer one (`Monotone`, `Exact`) |
| the client's rule for a page | a page of another reset, or one repeated, put into the value (`Exact`) |
| a session's changes as one unit | two values of one session held at two versions (`OneState`) |
| the value and its cursor in one stored write | after a client stops, a cursor past what it holds (`Exact`) |
| delivery from the changelog (a queue per client instead, bounded) | a client that never reads stops the writer, and the others (`Converges`) |
| nothing written for unchanged bytes | a change outside a viewer's relations delivered to it (`Silent`) |
| the subscriptions ended with their incarnation | a change delivered to an incarnation that ended, or to a person whose delegation ended (`Authorized`) |

The model is bounded: it does not cover the cost of what is owed, the pages' sizes, the cryptography of the signature,
or the grants of the blobs a row names, which are ADR 0153's.

**The code is held to the model** (ADR 0115 D2, D4 and D13). The server's due set, what a response carries, the units
and the reset's pages are traced from the runs of the server suite and checked as behaviours of `Subscriptions.tla`;
the client core's taking in of a unit or a page (D7) carries a Verus contract, as the `https` client core of ADR 0117
does: within a generation its cursor never goes back, and what it holds at a version is what it was given for that
version.

## Not in scope

Every version of a value (a journal's follower, ADR 0114); push to a page that is not running and transports other
than `https` (ADR 0117); a subscription to another server, and subscriptions forwarded between servers; a query a
client writes; consistency across two sessions' values; and any one application's values.

## Consequences

- A client of a public server is told when a value it may read changes, and resumes after an absence from what it
  holds, across a restart of either side.
- A server holds per subscriber a record, and the grants of the blobs it delivered, and nothing else, so its cost
  grows with the values and the viewers kept, not with how slowly anyone reads.
- A value that changes faster than a client reads is given to that client less often; a client that must see each
  change follows a journal instead.
- The outbox keeps its contract: what is owed exactly still pushes back, and a value no longer passes through it.
- A long absence costs a reset, whose size is the value's: the deployment's `tombstone_retention_ms` decides how long
  an absence is short, and its `tombstones` must hold the removals of that time.
- A document that subscribes is the same on every transport that carries the value.

## Alternatives rejected

- **Each change copied into each subscriber's outbox.** A full outbox refuses the session's send (ADR 0109 D10), so
  the slowest subscriber would stop the publisher; and the server would hold, per subscriber, every change since it
  last read.
- **A bounded log per value that forgets its oldest entries** (ADR 0062). It suits readers that must see each entry
  and can be told how many they missed. A subscriber of a value is owed the value, and a reader that lagged would have
  to be told to start again where the changelog gives it exactly what it lacks.
- **The cursor kept by the server.** The server would believe what it sent was received; the client knows what it
  holds, and a cursor the server keeps is lost or wrong after either side restarts. The same holds of a reset's place
  between pages.
- **The cursors in a header.** An intermediary limits a header's size, and `max_subscriptions` cursors with their
  generations would pass one and not another.
- **Removal markers bounded by their count alone.** A value that removes keys faster than a client reads its pages
  moves the floor past the first page every time, and the reset never completes.
- **A vocabulary of the binding's own for the machine** (`sce.subscribe`). It would run on one transport only, against
  §8.1's rule that the deployment alone chooses the middleware.
- **The whole value at each change.** Correct, and its cost is the value's size times the changes times the
  subscribers.
- **A second channel that stays open** (WebSocket, server-sent events). The binding exists so that any network that
  carries HTTPS requests carries SCE (§22); a held exchange already wakes a client when something is due.
- **A subscription that lives as long as its connection** (§10.4.4). An exchange of this binding is not a connection,
  and an absent client must resume.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. The model of D10, checked, with each configuration of its table failing as predicted, or the record amended by what
   TLC finds.
3. The store: `Write::values`, the changelog, its versions, markers, floor and generation, and the grants of the blobs
   its rows name, in memory and in SQLite, with its conformance kit
   (`backends/rust/session-store/tests/every_value_store_keeps_one_contract.rs`): a write refused for its fence or its
   version leaves no row of a value changed; one write keeps the state and the changelog or neither; what is owed
   after a version is exactly the rows and markers above it; a marker is dropped past its time and past the count, and
   the floor follows each; a version below the floor is answered as a reset; a restart keeps every version. Red
   without it: the store has no value table.
4. The build: `subscribable:`, its audiences, the bounds of D8, the four patterns in the envelope's document,
   `event.subscribe.X` and `field.notify.X` of a subscribable value carried over `https`, and each refusal of D9 with
   its appendix row. Red without it: `subscribable:` is an unknown key.
5. The server: the subscription records in the membership's store, the due set, what a response carries, the units and
   the reset's pages. In the server suite
   (`backends/rust/mesh/tests/a_value_reaches_every_subscriber_and_waits_for_none.rs`): 200 instances never listed,
   each subscribed to 5 values; one change reaches every one of them within the hold; one client that reads its
   response one byte at a time and one that stops polling leave the others' delivery and the session's write times
   within the bounds measured without them; the server killed between any two writes, and every client resuming
   without a reset; after a restart, each changelog's rows are the value computed from the restored tables; a client
   absent past its floor given one reset and then its changes; a value that removes keys at the rate its bounds admit
   while a client reads one page per exchange, and the client's reset completing; responses repeated and reordered
   between a client's two exchanges, and the client holding each value exactly; and the suite's runs traced against the
   model (D10).
6. The client core with its contract proved, and the native shell (D7), and the machine's events (D6); then the browser shell, with ADR 0117
   step 4.
7. The documents named in "Amends".

## Open questions

Decided on acceptance, each with its reason.

1. A change is sent as the rows changed and the keys removed, and a value that is not keyed as one row: the unit of a
   change is what a reader can apply without the history before it (a delta-state join, Almeida, Shoker and Baquero
   2018).
2. The cursor is the client's, rides every request in its body and is signed, rather than kept by the server: the
   client knows what it holds (D3), and a header's size is the intermediaries'.
3. A client is owed the latest version, not every version: a value is its state, and its history is a journal's (ADR
   0114).
4. `tombstones`, `tombstone_retention_ms`, `max_subscriptions` and `max_viewers` have no default: how long an absence
   is short, what it costs to store, and how many readers a server serves, are the deployment's, and no number here is
   guessed.
5. The changes owed to one session's subscriptions are delivered as one unit, and of two sessions each at their own
   version: a session is the unit of a consistent state, and two sessions share no order.

## Amended on acceptance

Each amendment is from the independent review of this record (2026-11-28), numbered as the review numbers them, and
was decided by the main session as stated.

1. Context and step 5: ADR 0113's membership and `Write::under` are built (its steps 3 to 6 landed), and step 5 no
   longer waits on ADR 0113 step 4 (review, amendment 1).
2. D3, D4 and D7: a third form of cursor, `(g, f, k)`, signed and held by the client, and each page carries its key
   range (review, amendment 2; the main session chose the cursor form).
3. D2, D8 and D10: `tombstones` is kept and `tombstone_retention_ms` is added, with no default, and `Converges` holds
   for a client whose reset completes within it, as Kafka's `delete.retention.ms` is (review, amendment 3; the main
   session chose a time beside the count).
4. D10: `Converges` is restated as "for every version published, the client comes to hold it or a later one, or the
   subscription ended" (review, amendment 4).
5. D4, D7 and D10: the owed changes of one session's subscriptions are one unit, all or none, kept by the client in
   one stored write, and `OneState` is added (review, amendment 5; the main session chose the unit over dropping the
   claim).
6. D7: a unit is applied only when `from` is at or below the version held and that is below `to`, else dropped; pages
   are put together by their key ranges (review, amendment 6).
7. D3 and D9: `subscribable:` is refused on a binding without the `key` authenticator and `sign:` and `verify:`, and
   the response covers the request's cursors in §22.9 (review, amendment 7; the main session decided to add
   `sce-mesh-values;req` to the covered list, and with the cursors moved into the body by amendment 8 the component
   that covers them is `content-digest;req`).
8. D3, D4 and D9: the cursors travel in the body, as `ValueSubscribe` envelopes covered by `Content-Digest` (RFC
   9530), and a deployment whose cursors do not fit `max_body_bytes` is refused (review, amendment 8; the main session
   chose the body over a header limit).
9. D6 and "Amends": a machine uses `event.subscribe.X`, `event.unsubscribe.X` and `field.notify.X`, and §8.1's
   sentence on snapshots and deltas is amended (review, amendment 9).
10. D9: the refusal names a machine whose sessions no durable store keeps, which a keyed pool with a durable store is
    not (review, amendment 10).
11. D2: the changelog is an output, as the outbox is, not a saved operator state (ADR 0151 D4), and after a restore
    its rows are the value computed from the tables (review, amendment 11).
12. D2, D3 and "Amends": the changelog is a holder of grants (ADR 0153) and translates a `BlobRef` to the subscriber's
    handle when it delivers (review, amendment 12; the main session chose the holder over a refusal).
13. The textbook structure and Sources: Almeida, Shoker and Baquero 2018 and Saito and Shapiro 2005 for D3, DBLog for
    D4 in place of ARIES, Kleppmann 2017 in place of Kreps 2013, and the sentence that the sources were not read is
    removed (review, amendment 13).
14. D1, by the owner (2026-11-28, ADR 0114 D4 as amended): a subscription to a machine that keeps one session names no
    key.
15. D10, by the owner and in the writing of the model: `Authorized` and its row; `Silent` stated per version, since
    its literal form fails under D3; `Converges` states that the session publishes, without which a queue per client
    that stops the writer would leave it true; the judge is `sce-codegen formal models`.
16. D10 and steps 5 and 6, by the owner: the code is held to the model by traces of the server suite and by a Verus
    contract on the client core.

## Sources

- P. S. Almeida, A. Shoker and C. Baquero, "Delta state replicated data types", Journal of Parallel and Distributed
  Computing, 2018: a replica is sent the deltas since the state its peer holds, and the whole state when those deltas
  are no longer kept (D3).
- Y. Saito and M. Shapiro, "Optimistic Replication", ACM Computing Surveys 37(1), 2005: state transfer rather than
  operation transfer.
- M. Shapiro, N. Preguica, C. Baquero and M. Zawirski, "Conflict-free Replicated Data Types", Symposium on
  Stabilization, Safety, and Security of Distributed Systems (SSS), 2011: state-based convergence.
- A. Andreakis and I. Papapanagiotou, "DBLog: A Watermark Based Change-Data-Capture Framework", arXiv, 2020: a
  snapshot read in chunks while the state changes, made exact by the log since the chunks were read (D4).
- M. Kleppmann, *Designing Data-Intensive Applications*, O'Reilly, 2017, chapter 11 "Stream Processing": readers that
  own their positions in a log.
- P. Th. Eugster, P. A. Felber, R. Guerraoui and A.-M. Kermarrec, "The Many Faces of Publish/Subscribe", ACM Computing
  Surveys 35(2), 2003.
- J. H. Saltzer, D. P. Reed and D. D. Clark, "End-to-End Arguments in System Design", ACM Transactions on Computer
  Systems 2(4), 1984.
- J. H. Saltzer and M. D. Schroeder, "The Protection of Information in Computer Systems", Proceedings of the IEEE
  63(9), 1975: fail-safe defaults.
- RFC 9110, HTTP Semantics, 2022 (section 5.4, a field's size; section 13, conditional requests); RFC 6585, Additional
  HTTP Status Codes, 2012 (431); RFC 9421, HTTP Message Signatures, 2024; RFC 9530, Digest Fields, 2024.
- The OMG DDS specification (`KEEP_LAST` and `TRANSIENT_LOCAL`, as `SCE_MESH.md` §8.2 states them); the Apache Kafka
  documentation (log compaction, and `delete.retention.ms`, the retention of a removal's marker); the Kubernetes API
  concepts (list and watch by resource version, the answer to a version too old, and a list read in chunks continued
  by a token). The documentation of Kafka and Kubernetes is cited for practice and not as a foundation.
