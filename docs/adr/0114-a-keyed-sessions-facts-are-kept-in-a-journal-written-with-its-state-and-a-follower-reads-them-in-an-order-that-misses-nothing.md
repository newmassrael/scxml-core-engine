# ADR 0114 — A keyed session's facts are kept in a journal written with its state, and a follower reads them in an order that misses nothing

- Status: Accepted (2026-11-22, by the owner). Its model (D7) was written and checked before any code. Amended
  2026-11-22 by the owner: D1 (how an entry is typed, what its send does, deciding and applying, a variable held to the
  facts) and D4 (a machine does not follow itself). Amended on landing step 4 (2026-11-27): D2 (a session's row is the
  machine and the key) and D4 (the reader's mark, the order of a key's entries, a follower that does not end). Amended
  2026-11-28 by the owner, at a consumer's request: D4 (a follower that keeps one session, `key_from: none`).
- Landed (2026-11-28): every step of "Order of work". The store (step 2) is `sce-rust-session-store`'s
  `JournalStore` in `MemoryStore` and `SqliteStore`, sealed by `sce-rust-crypto`'s `Sealed`; the build (step 3) is
  `sce-build`'s; the pool's writes and its follower (step 4) are `sce-rust-mesh`'s `pool` and `pool::follow`; and each
  document "Amends" names carries an "Amended by: ADR 0114" line (step 5), which `sce-build`'s
  `forge::journal::documents` holds it to.
- Amended by: ADR 0116 (D1 and D7): an entry records who its fact was said for, the instance and the person of the
  delegation the run that said it was made under (`Fact::actor`, `JournalEntry::actor`; `SqliteStore` layout 13,
  inside the record under `Sealed`), which the host core writes and no document does; an observed fact names no one.
  (D6, ADR 0116 D5): groups, roles and relations are facts of an observed stream, which a deciding machine follows; a
  fact a follower's session says records how far it had read each journal it follows (`Fact::read`, `SqliteStore`
  layout 14, inside the record under `Sealed`), and a request that carries a position of them waits in its session's
  inbox until the reader's mark has passed it (D4, `sessions.position_from`).
- Date: 2026-11-22
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: built on the backend a consumer needs, Rust first; the contract is
  general; what is special to one application stays in the application.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0114" line in the landing that moves it):
  `SCE_MESH.md` §10.11 and §14.4; ADR 0109 (D8: the seal; D10: the write that carries more than an outbox);
  `docs/SCE_ACCEPTED_SUBSET.md` §2.8, §2.15 and the appendix; `docs/SCE_SERVER_DEPLOY.md` sections 0, 1, 3 and 8;
  `docs/SCE_SEALED_RECORD.md`; `docs/SCE_HOST_FEATURES.md` section 9.
- Scope: the Rust host core and `sce-rust-session-store`; `sce-build` for what it reads. Every other backend refuses by
  name.
- Related: ADR 0086 (a bound is part of a type), ADR 0093 (an operation journal of a shared list), ADR 0101 and
  `sce:payload` (a record's deterministic CBOR), ADR 0107 (D6, the session store), ADR 0109 (D10, D13 to D15), ADR 0113.

## Context

What a server's sessions did is today in their saved states alone. A saved state is the present; a question about the
past, a read model over many sessions and a feed of what changed have nothing to read. The nearest parts are not it: the
audit log is for a reader outside the machine, hash-chained, redacted by its schema and retained by segments, and has no
read for a document; an outbox is one client's, and is deleted as it is acknowledged; a shared list's journal (ADR 0093)
is a replica's operations, not a server's facts.

ADR 0107 rejected "a journal of SCE's own as the reference store" for sessions, because a server's store needs
transactions over several records. This record keeps that: the journal is not the store of a session's state. It is a
record kept beside it, by the same transaction.

## The textbook structure

- **A log is the order in which facts became true; state and views are folds of it** (Kreps 2013; Helland 2015). A view
  keeps its position beside what it computed.
- **A write to two places is one transaction or is wrong** (the transactional outbox).
- **Two times** (Snodgrass 1999; Jensen and Snodgrass 1999): *valid time*, when a fact holds in the world, and
  *transaction time*, when it is current in the record: from the entry that states it to the entry that corrects it. A
  correction is a new record; none is ever changed.
- **Transaction time is an order, not a clock.** A clock can lag, and an answer "as of 14:00" given at 14:01 can be
  changed by a record another process stamps 13:59. Position is the order and the store's own.
- **A position may be taken before it is visible**: two transactions numbered 7 and 8 may commit 8 first. A reader that
  moves its cursor to 8 then never sees 7. The reader reads only below the point where nothing can still appear.

## Decision

### D1. A journal entry is made by a session, in the write that keeps its state

A machine entry may carry `journal: true`. A document of a keyed pool makes an entry with
`<send type="sce:journal" event="...">` of an event whose schema the document imports, its fields the entry's data: a
send of a declared schema's event, typed and lowered as one to another machine is, a list or a record given by the name
of what holds it (ADR 0086 section 5; `docs/SCE_ACCEPTED_SUBSET.md`, "A `<send>` of the event of a schema whose field is
composed"). (Amended 2026-11-22: it said "as `sce:request` types a request", which is text and holds neither a list nor
an absent field.) Such a send is not performed as it is executed: the entries one macrostep made are appended by the
write that keeps the checkpoint that holds it (`Write::journal`, beside `Write::outbox`), so an entry exists exactly
when the state that made it does. An entry is bytes: the record's deterministic CBOR with its schema's id and revision,
so an entry of an earlier revision is read by the lineage of ADR 0044 and ADR 0112 D5.

**To say a fact is to take it.** The one send does both: it makes the entry, and puts the event with the entry's data on
the session's internal queue, as `<raise>` does (§scxml-5.10.1 has its type `internal`), so the transitions that apply
the fact run in the same macrostep, before the checkpoint that carries the entry. A document cannot then record one
thing and apply another. The entry has no position until the write, so an applying transition reads none;
`_event.origin` is empty there. A restore replays nothing: the state is the checkpoint.

**Deciding is written twice, applying once.** A document that decides (it makes entries) and one that waits for an
observed entry (D6) differ in what they ask and in what may fail, and no syntax hides it: one's own fact is final as it
is said, another system's is not known until it comes. What they share is the transition that applies an event, which is
the same text whether the event came from the session's own send, from a followed journal or from an observed stream.

**A variable may be held to the facts.** A variable declared `sce:from="journal"` is assigned only in the initial data
and in a transition whose event is an entry's; any other assignment is refused where the document is read (an
algorithm's result as well, unless the call stands in such a transition). Its value is then a fold of the entries the
session said or took in, in their order, and the property is one a scenario can check (ADR 0115 D15): the same entries
given to a fresh session give the same value.

### D2. A stream is a session, and nobody else appends to it

An entry's stream is the machine and the session's key; a document names no stream. The one writer is the session that
holds the key's lease, and the store judges its fence before anything is written (ADR 0107 D6), so the expected version
of an event store is the fence and the session's version here, and a process that lost the key appends nothing. A fact
that concerns two sessions is made by each in its own stream, or by a request between them, as any such rule is.

**Amended on landing (2026-11-22, step 2).** A session is the machine and the key, and the store's `sessions` row is
keyed by the key alone, so two machines that keep their sessions in one file would share a row where they hold an equal
key. A write therefore names its machine for the journal alone (`Journal::machine`). The session's own identity is
brought to (machine, key) as the first part of step 4, in a layout of its own; until then a deployment that `follows:`
another machine keeps the two in two stores, and step 3 refuses one that names the same store.

**Amended on landing (2026-11-27, step 4).** A session's row, its lease and the notes an expiry wrote for it are kept by
the machine and the key (`SqliteStore` layout 12), and a handle keeps the sessions of one machine
(`SessionStore::machine`, `of_machine`), so the machines of a deployment keep their sessions in one store and an equal
key of two of them names two sessions. The interim of step 2 is lifted: a follower and the machine it follows keep their
sessions in the store the journal is in, and nothing refuses it. The build named no store to refuse by, and kept no such
refusal; what it refuses still holds: a follower of a machine of another device, whose journal is in a store its process
does not open ("Not in scope": a journal several stores share). The rows a file of an earlier layout held are the
machine the empty name stands for, the one machine of a store that kept one. A sealing store binds a session's record to
its machine as well as its key, so a record moved to another machine's equal key does not open.

### D3. Position, and the reader's horizon

The store gives each entry a position, above every position it gave before, inside the transaction that writes it. A
record sealed to its position (D8) is made there: the store is handed `FactData::AtPosition` and calls it with the
position, and a `Committer` refuses one, since it has no position to give. Positions are one sequence for the journal; a
gap is allowed and means nothing. A store states a **horizon** with every read: the position at or below which nothing
can still become visible. `JournalStore::after(position, limit)` answers entries above a position and at or below the
horizon, in order. SQLite writes one transaction at a time, so its horizon is its head. A store whose transactions
commit out of the order of their numbers reports a lower one, and a store that cannot tell is no store of this contract.
A store ends the transaction of a writer that is gone, as a database does the one of a connection that closed: one left
open keeps the horizon below it, and nothing past it is ever read (`Release`, D7). The conformance kit forces the
two-transaction case and the writer that is gone; a store whose horizon is wrong fails it.

### D4. A follower's position is part of its state

A machine entry may carry `follows: [<machine>]`. The pool delivers the followed journal, in order, as an external event
of the entry's name, its origin `journal://<machine>/<key>/<position>`, to the session that `key_from` names for it. An
entry of an observed stream (D6) has the origin `journal://<stream>/<position>`, two parts where a session's has three;
a stream and a machine of one deployment have different names. A machine does not follow itself: its own entries were
taken as they were said (D1), and would take effect twice; refused where the deployment is read. A session keeps the
position it has taken in of each journal it follows in its checkpoint, written with what the entry did, and takes an
entry at or below its position as a repeat and drops it. A delivery that is lost or repeated is delivered again or
dropped, so each entry takes effect once. What a view computed is the checkpoint, so a view needs no other snapshot, and
a view is rebuilt by following from position 0 under a new name. A follower that cannot keep up stops the reader at its
inbox's bound (ADR 0109 D1) and nothing is dropped.

**Amended on landing (2026-11-27, step 4).** The pool's reader (`Pool::follow`) is what D4 rests on, held to three rules
beside the session's position. It hands a session the entries of its key in the order of their positions: one handed and
lost (its session dropped before a durable checkpoint held it) is handed again before any later one of its key, and no
later one is handed while the key's session is in another place than the earlier one went to, judged under the lock the
hand-out is made under; a session that took an entry took every entry of its key below it, so its position drops only
repeats. Its mark (`JournalStore::keep_read`, kept by the follower's name in the store) is the position through which
every entry it read was taken in by a durable checkpoint, and a pool reads on from it when it starts: the consumer's
committed offset, a lower bound the idempotent receiver makes safe to repeat from (Kleppmann 2017, *Designing
Data-Intensive Applications*, chapter 11). A mark is never kept above the horizon. An entry of a key another process
holds is that process's to hand: it is taken in once that session's stored checkpoint holds its position, and handed
here once the lease runs out. An entry no session can be handed (no kind of entry the build described, data no value of
its record, no key) is told to the host and passed. A follower's document declares no top-level final state
(`mesh/codegen-journal-mismatch`): a session that ends takes its positions with it, and an entry handed again after its
end would start its key's session anew and take effect twice. The build tells the pool the record each followed kind of
entry is read by (`Sessions::follows`), and an entry's data is that record's deterministic CBOR read into the event's
data, as the event of a fact the session said itself carries it.

**Amended 2026-11-28 by the owner: a follower that keeps one session.** A keyed pool may declare
`sessions: { key_from: none }`: it keeps one session, and every input it takes is that session's, an entry of each
journal and observed stream it follows and a request a client sends alike. The read model of a whole deployment is such
a session (ADR 0151 D9): it follows journals and streams whose entries share no field, and a field of one value added
to every fact only so that it could be routed would make the facts carry the shape of the deployment that reads them.
- **Its key is the empty key.** A key read from an envelope is never empty (the pool refuses one, `bounded_key`), so
  the empty key is one no other source gives, and on a machine whose key comes from nothing no other source is read.
  Its row is the machine and that key (D2), and the origin of an entry of its own journal is
  `journal://<machine>//<position>`, the key an empty segment (RFC 3986 §3.3 allows one).
- **Its positions are the follower's.** It keeps one position for each journal and stream it follows, in its
  checkpoint, written with what the entry did, and the reader's rules above hold of it with one key. A request under
  `position_from:` waits in its inbox as in any session's.
- **What is refused is what already was.** `sessions:` is refused on an open machine (ADR 0113 D1), whose instances
  are several masters of one key; `key_from` names one source, so no machine reads a key from an envelope beside it.
- **It is the case the model checks.** The model's follower (D7) is one session that takes in every entry of the
  journal; the dispatch by a key field that the model leaves out is not used by it.

### D5. Two times, and reading as of

Every entry carries its position and the recorded moment: the store's clock, never below the one before it in the order
of commits. Where a store commits in the order of its numbers the two agree. Where it commits out of it they need not,
and `position_at` answers a position the horizon has passed that no entry recorded at or before the moment is above; it
may take in one recorded later, and the answer to the position it gives does not change. A schema may mark one `int64`
field `sce:valid-time="from"`, in milliseconds; an entry of a schema without it holds from its recorded moment. A read
names `(R, V)`: entries at or below position R whose valid time is at or before V, ordered by valid time and then
position. R is a position at or below the horizon. A moment of the clock is first turned into a position by the store
(`position_at`), and only one the horizon has passed is. The answer to an `(R, V)` never changes. A correction is an
entry; none is changed or removed.

A document asks for a page with `sce-host-journal-read` (stream, after, limit, R, V), whose entries are a value of a
union of the document's schemas (ADR 0092), bounded by the build's number for a page. A whole journal is a follower's to
read.

### D6. A stream the application observes

Some facts are another system's. `streams:` of the deployment declares an **observed** stream and its schemas; no
session appends to it. A read says which it asks: a session's stream or an observed one (`Stream::Session`,
`Stream::Observed`), which one name never is of both, so an observed name that is also a machine's is refused, at build
where the deployment is read (step 3) and by the store. The program, through an adapter that is the application's, gives
the store an entry with the **origin id** the other system gave it, and the store keeps one entry per (stream, origin
id), answering the one it holds for a repeat. A session that asks the other system for a change waits for the observed
entry that says the system accepted it, and does not take its own request for the fact. (ADR 0093's "taken in once
accepted" is the same thought for a shared list's copy; it is an analogy and no part of this contract.) The adapter, its
credentials and its retries are not SCE's.

### D7. The protocol is a model before it is code

`docs/formal/EventJournal.tla`, checked by `scripts/formal/check.sh`: two processes that take over each other's
sessions, crashes, transactions numbered in one order and committed in another, a follower and questions as of a moment.
It states what it takes as given. It checks `AppendOnce` (an input makes one entry), `NothingLost` (a state that was
written has its entries), `FollowedOnce` and `NonePassed` (each entry takes effect once and no follower passes one) and
`Stable` (an answer does not change), that every entry is followed in the end (`Followed`, under fairness), and that a
follower can always come to have taken in all there is (`CaughtUp`, `AG EF`). On 2026-11-22, two sessions, one input
each: 706,504 states; one session, two inputs: 18,088; no violation. Each part is shown to be needed:

| Taken out | What TLC finds |
|---|---|
| the fence | an input that makes two entries |
| one transaction for the entry and the state | a state whose entry a crash lost |
| the horizon | a follower that passes an entry; an answer that changes |
| the position written with the effect | an entry that takes effect twice |
| a position as the moment (a clock instead) | an answer that changes |
| the end of a stopped process's transactions | an entry that is never followed, and a state from which no follower can come to have taken in all there is |

The model is bounded and small. It does not cover the dispatch of one journal to many sessions by `key_from` (D4 states
its rule, and it is tested), the valid-time order, the observed stream or the seal.

### D8. At rest, erasure, size

- `Sealed` keeps an entry's data sealed, bound to its stream and position, and a stream by its keyed digest, as it keeps
  the rest (ADR 0109 D8). The position and the recorded moment stay in the clear; a valid time is inside the data, so a
  read by valid time opens the entries it reads. The re-seal pass covers the journal.
- **Erasure is not decided here.** Removing a person's data by destroying its key contradicts the present contract of
  the key ring (no call destroys a key, `docs/SCE_SERVER_DEPLOY.md` section 3.2), and it changes what existing users may
  rely on. It is a record of its own. This contract leaves room: an entry is opaque to the store, so what a later record
  seals under a person's key needs no change here.
- The journal has no retention: a record is kept until a record of its own decides what may be cut and what a reader
  as of a moment may then still ask. It grows with what the sessions say; its size is the deployment's, and metrics tell
  it.

## Not in scope

Relational queries over a journal (a kind of its own), subscription by a client (ADR 0113's own record), erasure,
retention, a journal several stores share, and any one application's streams.

## Consequences

- A document says a fact once, with the state that made it, and no store call stands between.
- A read model is a session that follows; it is saved, restored, bounded and rebuilt as any session is.
- A store for this contract must know the order in which its commits become visible. Some cannot, and they are refused.
- Nothing of a session that declares no journal moves: its write, its wire and its checkpoint are as they are.

## Alternatives rejected

- **Event sourcing: a session's state rebuilt by folding its journal.** It makes two sources of one truth and every
  restore a replay of the past. A saved state is already exact, versioned and migrated (ADR 0044, ADR 0109 D11).
- **An append host operation** (`<invoke>`, as the audit log has). It is a second write: a crash between it and the
  checkpoint loses or repeats the entry, which is what the model's second row finds.
- **An expected version the document writes.** The fence and the session's version say it, and a document that names one
  can be wrong.
- **A timestamp as the order.** The model's last row.
- **A cursor kept apart from the follower's state.** The model's fourth row.
- **Editing or deleting an entry to correct it.** It changes an answer already given.
- **A stream as a name the document chooses.** Two sessions would write one stream, and neither holds its lease.

## Order of work

Each step lands with a test that fails without it.

1. (2026-11-22) This record and its model, accepted.
2. (2026-11-22) The store contract, its memory and SQLite implementations (layout 8) and the conformance kit: the horizon with two
   transactions committed out of order, concurrent writers, a crash at every write, positions that rise, a read as of a
   position that later writes do not change, an origin id given twice. `Sealed` and its re-seal pass.
3. (2026-11-27) The build: `journal:`, `follows:`, `streams:`, `sce:journal` sends, `sce:valid-time`, `sce-host-journal-read`, and
   the refusals, each with its appendix row.
4. (2026-11-27) The pool: the write that carries entries, the follower and its positions. In the server suite: an input
   killed at every write makes one entry or none, and a follower of many sessions that is killed repeats and misses
   nothing.
5. (2026-11-28) The documents named in "Amends", held by a test to that list and to what the parts state.

## Open questions

Decided with the acceptance as recommended, each with its reason.

1. Whether a document may read another machine's journal by page (D5), or only its own and those it follows.
   Only those; a read of any other is a subscription's question.
2. Whether `position_at` is needed at first. Yes, since a person asks by date; it is the store's to answer.
