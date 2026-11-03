# ADR 0085 — A change is one signed unit, and a layer below the list delivers it once, whole and in causal order

- Status: Proposed (2026-11-03). Written at the owner's order of that day, which chose this structure over an
  earlier draft (recorded in "Alternatives rejected"), under the rule of 2026-10-26 and 2026-10-27: the most
  long-term correct structure, textbook in its foundations, cost ignored. The draft waits for acceptance; the
  questions it leaves to the owner are in "Open questions for the owner".
- Date: 2026-11-03
- Amends:
  - ADR 0046 sections 1 to 4, 6 and 7: a change of several operations is no longer parts held until whole; it is one
    message, and the list keeps no tombstones and no held parts;
  - ADR 0042 (the settle round, the digest and its walk leave the list for the delivery layer);
  - ADR 0083 sections 1.1, 1.2, 1.5, 2.2, 2.5, 2.6, 3.1, 4.3, 6 and the landing order: the journal reports one
    effector per change, recall takes a whole change, the history is given in causal order, the snapshot at a floor
    is a state, and step 2 is reworked to change units;
  - ADR 0079 (one peer core): the core is the list's delivery layer.
- Scope:
  - `stdlib/shared/record_list.scxml` and its schemas: `record_change` and `record_op` (new); `record_write`,
    `record_supersede`, `record_part`, `record_keep`, `record_node`, `record_visit`, `record_share`,
    `record_settle`, `record_propose`, `record_ack`, `record_floor`, `record_compact` and `record_answer` retired
    from the list; `record_made` revised; `record_room`, `record_refusal` read anew;
  - the event-schema grammar: an event schema's field may be a bounded list of records (section 6), in the forge
    AST, `sce-build`'s validation, every generated backend's payload codec, the Interpreter's lowering and the Mesh
    codec;
  - the Mesh core (`SCE_MESH.md` section 10): a causal delivery setting beside `ordering:` and `receipt:`;
  - `sce-rust-trust` (`message.rs`, `chain.rs`, `trust.rs`), `sce-rust-relay`, `sce-rust-peer`, `sce-rust-peer-capi`;
    `docs/SCE_SHARED_LIST_TRUST.md`, `docs/SCE_SHARED_LIST_RELAY.md`, `docs/SCE_SHARED_LIST_PEER_ABI.md`;
  - the list's scenarios and the seven drivers that replay them, the world tests of the list, the saved-shape pin,
    and every generated tree of the list.
- Related: ADR 0034 (a shared list is held by its members; the relay only relays), ADR 0044 (the revisions of a
  schema), ADR 0045 (a standard statechart that saves is a promise about its shape), ADR 0077 (a declared bound is
  part of a field's type), ADR 0078 (a consumer follows SCE one landing at a time), ADR 0080 (a device's relay token).

## Context

A change of more than one operation travels today as parts (ADR 0046 section 3): one `record_list.write` or
`record_list.supersede` per operation, each with `parts` and the change's stamp, which a receiver holds in `waiting`
until it holds every part, then takes in together. `record_list.arrive` decides that a part has arrived partly from
the receiver's registers: a part whose dot a tombstone names, or whose value is held, is `already`, is not held,
takes in what is held of its change at once, and is not counted (`stdlib/shared/record_list.scxml` at `bf3c2cafa5`,
lines 1059-1091; on the branch of ADR 0083 step 2, `adr0083-l2-journal`, lines 1251-1283).

Another change can produce that state, and the set the list is makes it happen:

- **(a) Two changes clear one value at once.** Bob writes over `x`; Alice, not having seen it, writes over `x` and
  `y` (two parts). A copy that took Bob's in is sent Alice's `wo x` first: `already`, nothing held, nothing taken, not
  counted. Then `wo y` is held as 1 of 2 and waits for a part that came and went. `y` stays live.
- **(b) A change arrives before the change it writes over.** O2 writes `12/1` and `12/2`; O3, made after seeing O2,
  writes over `12/2` and reaches a copy first. O2's `12/2` then arrives before its `12/1`: `already`, not counted;
  `12/1` waits for good.

A stuck part leaves only at a resync or a floor that passes it, and no floor passes it: the stuck copy answers a
settle with another hash, so no tombstone is forgotten anywhere until the host resyncs that copy. The rebuild of
ADR 0083 meets case (a) deterministically: it recalls in the order of the changes' stamps through the same `arrive`,
and the later of two concurrent clears is then `already`. ADR 0083 section 2.2's sentence "a tombstone recalled first
keeps the add out" holds for a change of one operation only.

Reading the defect against the textbooks shows a second thing: it is not a defect of `arrive` alone, but of where
the list's delivery is done.

- **A unit is cut into messages and glued back by the receiver.** A change is one update (one effector, one
  outcome, ADR 0083), sent as n messages, signed as n operations (`message.rs`: each `record_list.write` and
  `record_list.supersede` is a signed event of its own), stored as n records by the relay, and reassembled by every
  receiver. The receiver can hold half a change; a host can build half a change from the relay's records.
- **The list does its own delivery, without the delivery's metadata.** It deduplicates (a part "already taken in"),
  orders (a tombstone kept until a settle so that a write-over that arrives before its write still keeps it out),
  and repairs (the digest and its walk) from its own registers, because nothing below it says which updates it has
  received. Meanwhile two layers below it already number every update per author: the trust layer's chains (`Link {
  seq, prev, log, log_hash }`, `backends/rust/trust/src/message.rs:254`; numbers from 1 per device, no gap) and the
  Mesh core's per-source sequence (`SCE_MESH.md` section 10.6).

The defect was read from the code while landing ADR 0083 step 2; it was not observed in a run. It predates ADR 0083.

## The textbook structure

**An operation-based object assumes a delivery layer.** Shapiro, Preguiça, Baquero and Zawirski define an
operation-based CRDT by a prepare phase at the source and an effector applied at every replica, and assume that the
layer below delivers each effector exactly once and in causal order ("A comprehensive study of Convergent and
Commutative Replicated Data Types", INRIA RR-7506, 2011, section 2.2.2; "Conflict-free Replicated Data Types", SSS
2011, LNCS 6976). Under that assumption the observed-remove set needs no tombstones: a remove is delivered after every
add it observed, and removes exactly those tags (Specification 15 of the report).

**Causal broadcast is that layer.** A message carries its sender's identity, its number among the sender's messages
and the sender's vector of what it had delivered; a receiver delivers it when it is the next of its sender and its
vector is covered by what the receiver delivered, holds it otherwise, and discards one it delivered already (Birman
and Joseph, "Reliable Communication in the Presence of Failures", ACM TOCS 5(1), 1987; Birman, Schiper and Stephenson,
ACM TOCS 9(3), 1991). The vector is a version vector of per-author contiguous numbers (Lamport, "Time, Clocks, and the
Ordering of Events in a Distributed System", CACM 21(7), 1978; Mattern, "Virtual Time and Global States of
Distributed Systems", 1988, in *Parallel and Distributed Algorithms*, North-Holland, 1989): a pair (author, number)
is delivered exactly when the number is at most the vector's entry for the author. Gaps are repaired by exchanging
vectors and sending what the other lacks (anti-entropy: Demers et al., "Epidemic Algorithms for Replicated Database
Maintenance", PODC 1987).

**Tagged causal stable broadcast, the PO-log and compaction.** Baquero, Almeida and Shoker make the layer's
interface explicit ("Making Operation-based CRDTs Operation-based", DAIS 2014, LNCS 8460; "Pure Operation-Based
Replicated Data Types", arXiv:1710.04469, 2017): the layer delivers each operation with its tag (its vector
timestamp), and also reports **causal stability**, when an operation is known to be delivered at every replica. The
object keeps its recent operations with their tags (the PO-log) and, at stability, folds them into a plain state and
drops their metadata. The effect phase is then pure: it is given whole, causally ready operations, once each.

**A unit is a message.** An update is atomic because it is one message of the layer, not because the receiver
reassembles it. Where a frame is smaller than a message, fragmentation is the transport's, keyed by the message's
identity and invisible to everything above (Postel, RFC 791, 1981, fragment identification and offset).

Measured against this, the list has the effect right (`record_list.take` is idempotent by dot) and does the layer's
work above itself, with less information than the layer has. The decision moves the work down.

## Decision

**One change is one effector, one message and one signature. The list sends it as `record_list.change`, which
carries the change's operations in their order and nothing of its delivery. A layer below the list (the peer core
for a host that has one, the Mesh core for a copy that has none) tags each change with its dot and its causal
context, delivers it to every copy exactly once and only when every change in its context was delivered there,
repairs what was lost, and reports causal stability. The list keeps only the effect: no parts, no held changes, no
tombstones, no digest, no settle round. Compaction happens at causal stability, in the layer and the host's journal.**

### 1. One change is one message: `record_list.change`

1. **The event.** `record_list.change` (`record_change`, new) carries: `by` (the author's node), `wallTime` and
   `counter` (the change's stamp: the dot of every write in it, ADR 0046 section 1), and `ops`, a list of
   `record_op` (new: `kind` 0 for a write and 1 for a write-over, `id`, `field`, `value`, `text`, and for a
   write-over the dot it names, `wallTime`, `counter`, `nodeId`), in the fixed order of ADR 0046 section 3 (every
   write-over in the order of the source's `adds`, then every write in the order of the commands). `ops` is bounded
   by `shared_parts` (64 by default; the name stays, and now bounds the operations of one change only). An empty
   change is never sent (ADR 0046 section 1: a change that names nothing is no change).
2. **The source.** `record_list.close` builds the change as today and takes it in through `take` at once (the
   effect at the source, op-based); then a copy joined with `journal` reports it (section 7), and a copy without a
   journal sends it whole to its delivery layer on the route `#people` (section 3). Nothing else is sent for it.
3. **The receiver.** `record_list.change` from the delivery layer queues one `record_list.take` for each operation
   of `ops` in their order and the change is told to the parent once, after the last (ADR 0042's telling, unchanged).
   A receiver can neither receive nor build half a change, because no half of one exists on any wire.
4. **The one-operation forms retire.** `record_list.write` and `record_list.supersede` are no longer sent, by any
   copy, for a change of one operation either. Keeping them would leave two shapes of one fact with two signed
   events and two delivery paths, and a change of one operation would be the only change whose identity is not a
   message's; the cost of one shape is a list of one element. They stay readable in one place only: the core's
   one-time fold of a list's history signed before this decision (section 8.3). The revision-1 fixture `todo_peer`
   (ADR 0046 section 7) is rebuilt on `record_list.change` with the world replay that maps its afternoon.

### 2. The list keeps only the effect

1. **What goes.** `record_list.arrive`, `already`, `listed`, `count`, `waiting`, `kept`, `passing` and
   `record_list.pass`, the hub's relay, `late`, `tombstones`, `stable`, `digest` with `record_list.keep`,
   `record_list.resync`, `record_list.visit`, `record_list.node` and `record_list.share`, and the settle round
   (`record_list.settle`, `propose`, `ack`, `floor`, `compact`, `pending`, `members`, `answers`, `frozenFor`,
   `frozenHash`, `agreed`, the variable `floor`, `stale`). Each did the delivery layer's work: dedup, ordering by
   tombstone, repair by digest, and stability by agreement on a hash.
2. **No tombstones.** Under causal delivery a write-over is delivered after every write it names, so `take` removes
   the named value from `adds` and `values` at once (`OrsetLive` and `MvValueDrop`, as today) and keeps nothing: no
   later delivery can bring the value back, because a write is delivered once. A write-over whose value is already
   gone (case a: another change removed it) changes nothing. A write whose register is not held is added. ADR 0046
   section 2's observed-remove rule ("an edit not seen wins") is the same rule, now without the tombstone that paid
   for out-of-order delivery.
3. **What stays.** The commands and their judgement (`record_list.edit`, `clear`, `retire`, `close`, `staged`, the
   room, the refusals), `take` (one effect path, ADR 0083 section 1.5), `values`, `adds`, the clock (moved by every
   stamp taken, ADR 0083 section 1.6), the roster and roles, the telling, and the journal.
4. **The roster.** A change whose author the copy's roster does not let write is taken as an empty effect and counted
   in `denied`, on every copy, where only the hub judged it before: there is no hub relay left to judge for the
   others. On a host with a core, the core admits by the membership log before it delivers (ADR 0034), so the list's
   judgement is a second check that never fires there.
5. **The room.** `history` is read anew as the most changes the copy's delivery layer may hold not yet stable
   (section 4), which the host gives the copy from its layer; a change past it is refused `noRoomForHistory`, whose
   meaning ("a settle frees it") is kept: stability frees it. `digest` and `noRoomForDigest` are no longer used; the
   enum keeps the variant declared (a parent built earlier reads it) and the list never tells it.

### 3. Delivery is a layer below the list, with one rule in two places

**The tag.** The layer tags each change it accepts from its source with its **dot**, `(author, seq)`, the author's
node and its number among the author's changes, from 1 with no gap, and its **causal context**, the layer's version
vector of what it had delivered to that copy when it accepted the change (one `(author, seq)` entry per author it
delivered anything of; at most `shared_roster`). The list never sees the tag. A context taken when the change is
handed down may name a change delivered to the copy's queue that the list had not yet taken when it prepared; that
overstates the change's causal past, which only orders more than needed and is never unsafe.

**The rule, the same in both places.** For a copy whose delivered vector is `D`, a change tagged `(a, s)` with
context `C`:

- if `s <= D[a]`, it is delivered already: it is dropped;
- if `s = D[a] + 1` and `C[b] <= D[b]` for every author `b` other than `a`, it is delivered to the list, whole, and
  `D[a]` becomes `s`;
- otherwise it is held until it is deliverable; a gap is repaired by asking a peer for the changes the copy's vector
  lacks, never skipped.

The source's own change counts as delivered at the source when the layer tags it (`D[self]` becomes its `s`). A copy
that does not know its own highest number (restored from an older save, set up again from its seed) learns it from
its peers before it tags anything, so that no number is used twice (ADR 0083 section 3.4).

**Stability.** A change is stable when every member's delivered vector covers it. The stable cut is the pointwise
minimum of the latest delivered vector known from each member of the roster. Every member's vector is known from the
contexts of its changes and, for a member that has not written since, from a mark the layer sends of its own vector.

#### 3.1 For a host with a core: the core is the layer

- **The dot is the chain's link.** One change is one signed operation, so the chain's `seq` (`Link`, `message.rs:254`)
  is the change's number, and a device's chain is its column of the version vector. The core already numbers,
  verifies, stores and syncs per-author chains with no gap; what it adds is the context.
- **The context is signed in the link.** `Link` gains `context`, the core's delivered vector at signing, as a field
  of the link's bytes (`operation_bytes`), so that a relay cannot change the order a change is delivered in. It is
  delivery metadata and travels with the link, not in the event the list reads.
- **The core delivers by the rule.** A round, `Peer::history` (ADR 0083 section 2.5) and every hand-over of others'
  operations give the machine only deliverable changes, in a causal order; a change the core holds whose context it
  cannot meet waits in the core, which asks the relay for what it lacks. ADR 0083's history order (by change stamp,
  then author, then part) becomes a causal order (a linear extension of the contexts, ties broken by stamp then
  author), and is a condition of the result no longer, since the list is given only ready changes.
- **Stability.** A member's delivered vector is known from the context of its latest operation; a device that has
  not written for a while signs a mark of its vector, an operation of its chain that carries no change and that the
  core never delivers to a machine (the trust layer's mark kind, beside its membership entries).

#### 3.2 For a copy without a core: the Mesh core is the layer

- **A binding setting.** A route a deploy binds with `delivery: causal` (a Mesh-core setting beside `ordering:` and
  `receipt:`, `SCE_MESH.md` section 5, "Mesh-core delivery settings") is a causal broadcast group: the sending side
  stamps each envelope with the dot and the context (optional CBOR envelope fields, as `sequence_no` is, section
  10.6.3); the receiving side applies the rule above before dispatch, and keeps what it sent and delivered, until it
  is stable, to answer a peer's repair.
- **Repair, not skipping.** A causal route has no gap timeout: section 10.6's fast-forward drops a gap, which would
  lose an effector for good. The repair is anti-entropy: peers exchange vectors on a period and when a link is back,
  and each sends the changes the other's vector lacks. That is what `record_list.resync` and the digest walk did,
  done once in the layer for every document that needs it.
- **No machine-level relay.** The route `#people` is one group every copy sends to and reads; a transport without a
  topic is realised by the Mesh core forwarding envelopes at a relay node with their tags intact. A hub machine that
  passed changes on would re-send them under its own identity, which is why the hub's relay goes (section 2.1). The
  hub flag of `record_list.join` keeps no role in the list and is retired.
- **Dedup and ordering.** Section 10.5's dedup window and section 10.6's per-source buffer are subsumed on a causal
  route by the vector, which dedups exactly and orders causally; the two settings are refused beside `delivery:
  causal` on one binding, so a route has one delivery rule.

### 4. Compaction at causal stability

1. **The list.** Nothing to compact: it holds live values only. It has no floor and no `late`; the clock is raised
   by every stamp it takes.
2. **The layer's log is the PO-log.** What the layer keeps of changes not yet stable (the core's store of operations
   past the cut, the Mesh core's log for repair) is dropped at the stable cut. The core keeps the sealed records the
   relay holds as proof (ADR 0083 section 4) by the relay's own rules, which this decision does not change.
3. **The host's journal (ADR 0083 section 2.6)** is folded at the stable cut: the host takes a snapshot of the
   list's state at the cut (the live values with their dots, the clock), records it with the cut in one transaction,
   and drops the journal's changes at or below the cut. A rebuild gives the copy the snapshot, then the changes after
   the cut in causal order.
4. **A snapshot is a state, not changes.** ADR 0083 section 2.6 recalled the snapshot's live writes as recalled
   parts, before giving the floor. A live write of a change whose other parts were write-overs, or were written over
   since, was then held for parts that the snapshot does not hold, and the floor given next dropped it: a live value
   lost on every rebuild past a settle. Here the snapshot is given as `record_list.restore` (new, with the fields of
   `record_change` and `by` 0): its operations are the stable state's live values, taken by `take` as writes, told to
   the parent once. It is a state handed over, as ADR 0045 hands over a saved state, and no change identity is
   claimed for it.
5. **A copy behind the cut** (restored from a save older than the cut its peers compacted at) cannot be repaired
   from the logs; the layer says so, and its host gives it a snapshot at the cut and the changes after it, which
   replaces the list's `stale`.

### 5. Size: the change bound is the event's bound

A change is bounded where it is made (`shared_parts` operations, each with at most `merge_text` bytes of text;
`changeTooLarge` refuses one past it), so the largest `record_list.change` is a number the build knows. The bound
belongs to the event's type (ADR 0077) and is the machine's to hold:

- **The machine holds a whole change.** The generator derives the largest encoded payload of every event a machine
  accepts from its schemas' declared bounds. A backend whose event slot is sized by hand (C11's `SCE_MAX_DATA_LEN`,
  256 bytes by default, `backends/c/runtime/include/sce/types.h:76`) refuses at build time a slot smaller than the
  machine's largest payload (a static assertion the generated code carries), rather than truncating at run time. A
  deployment on a small device lowers `shared_parts` and `merge_text`, which lowers the change bound, and the build
  follows.
- **Fragmentation is the transport's.** A transport whose frame is smaller than a change (a datagram, a radio or a
  bus frame) fragments the envelope below the delivery layer, keyed by (dot, fragment number, fragment count),
  reassembles it before the layer sees it, and drops an incomplete one, which the layer then repairs as any loss.
  The list and the layer never see a fragment.

Fragmenting at the list's level is what this decision removes, so it is not offered as the general answer; raising the
slot to the change bound is, and fragmentation is kept for links that cannot carry the bound.

### 6. An event field may be a bounded list of records

An event schema's field is a scalar or an optional scalar today (docs/SCE_ACCEPTED_SUBSET.md, "Optional
variables"), and "an event cannot carry a list" (ADR 0042 section 2). `record_change.ops` needs a field of type
`list<record:Op>` whose element is a record of an event schema the schema imports, with a declared bound
(`sce:capacity`, `sce:limit`), carried as a JSON array of objects in `_event.data` and as a CBOR array by the Mesh
codec, decoded with the bound enforced (ADR 0077), and, under ADR 0044, `sce:since` with the empty list as its only
default. The trust layer encodes it in `operation_bytes` as its length and then each element's fields in their
declared order and kinds. The decision needs this and does not specify the grammar change in full: it is a property
of the toolchain, not of the list, and lands before the list moves (Landing order; Open question 1).

### 7. ADR 0083: the journal, recall and the core, at change units

1. **The journal.** A copy joined with `journal` reports each change it makes as one `record_list.made` (revised:
   the fields of `record_change`, with `command`), and every outcome as `record_list.closed`, as ADR 0083 section
   1.2 says. There is one report per change, not one per part; `ordinal` goes.
2. **Recall.** `record_list.recall` takes a whole change, with the fields of `record_change`: the host's own journal
   and the core's history give changes, already deduplicated and in causal order (section 3.1), and the list takes
   each through `take`. A hub is not special: there is no hub relay.
3. **The core.** `Peer::made` takes one change and signs one operation; `Kept::Taken` and the records of ADR 0083
   section 3 are per change; `Peer::history` gives changes in causal order with their tags read from the links.
4. **Two notes naming one write** (ADR 0083 section 4.3) become two changes of one author whose write-overs name one
   dot: admitted, idempotent, a link each.
5. **ADR 0083 step 2 is reworked.** The branch `adr0083-l2-journal` (not pushed) reports and recalls per part
   (`record_made` with `ordinal`; recall through `record_list.arrive`); it is reworked to change units in the landing
   of this decision, and lands with it, not before it.

### 8. Migration

1. **One protocol cut, one domain bump.** The signed events become `record_list.change` and the trust layer's mark;
   `record_list.write` and `record_list.supersede` are no longer signed by a core of this decision. That is the domain
   version `sce-trust/event/3` ADR 0083 section 6.2 already plans; it is made once, here, and covers ADR 0083's
   changes too. `Link` gains `context`, which is a new domain of the link's bytes as well.
2. **Lists without a core** (Mesh deployments, scenarios, world tests) hold no deployed data (the list is pre-release
   under ADR 0042, and no saved state that holds it can exist yet, ADR 0046 section 7): they move with the landing.
3. **A list whose relay holds operations of the second domain** (a deployed list: the measured device of ADR 0083 has
   765) is folded once, at its first start under this decision: the core reads every operation of the list it holds
   or fetches, of either kind, computes the state they make (the live writes are the writes no write-over names:
   the observed-remove rule is order-independent, so no order is needed), and gives it to the host as the list's
   first stable snapshot (section 4.4), at a cut of every chain's highest number. From then on only changes are
   accepted for that list; an operation of the second domain the core meets after the cut is refused and kept as
   proof, as ADR 0083 section 4 keeps a conflict. A device not yet moved is therefore cut off from the list until it
   moves, which a deployment plans (ADR 0078).
4. **Every lane.** The list's document and schemas, its saved shape (the pin in
   `sce-build/tests/fixtures/standard_saved_shapes.json` moves, ADR 0045), the committed generated trees (Rust, Go,
   Kotlin and Swift `static_datamodel`, `scripts/regen_static_datamodel_{rust,go,kotlin,swift}.sh`; the Go
   round-trip tree, `backends/go/forge-runtime/round_trip/generate.sh`, where its source set holds the list's
   schemas), and the lanes that build it at test time (Python, C++, C11, the Interpreter). The scenarios move as
   section 9 says, and every driver registers them: Rust (`backends/rust/tests/tests/static_scenarios.rs`,
   `backends/rust/scenarios/src/machines.rs`), Kotlin (`StaticScenarioTest.kt`), Python
   (`test_static_scenarios.py`), Go (`static_scenarios_test.go`), C11 (`test_static_scalars.c`), the Interpreter
   (`AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`); C++ derives its suite from the directory.
5. **What leaves with the list's delivery.** The scenarios of the walk and the settle (`record_list_hub`'s and
   `record_list_person`'s steps of `resync`, `node`, `share`, `settle`, `propose`, `ack`, `floor`), the parts steps
   of `record_list_change`, `record_list_retire`, `record_list_journal` and the three `record_list_recall*`, and the
   world tests that hold the walk or the settle (`a_resync_reaches_every_wall_time_and_every_field_number.rs`,
   `a_copy_restored_from_an_old_save_is_repaired_by_resync.rs`,
   `a_shared_calendar_forgets_what_its_copies_agree_on.rs` among them) are moved to the layer that now does the
   work (the Mesh core's causal route, or the core), not dropped: each property they hold is still held, one layer
   down.
6. **What a host changes** (ADR 0078): it gives `record_list.made` changes to `Peer::made` whole, recalls changes
   whole, gives the list its layer's room, folds its journal at the stable cut, and gives a snapshot as
   `record_list.restore`.

### 9. Verification

Every test below is written so that today's rule turns it red, which the landing checks once by running it against
today's list (parts over a route with no causal layer, the per-part recall) before the new structure is trusted.

1. **The cross product, in the world.** `backends/rust/world`, a simulated network over the list's copies, for one
   change C of three operations (two write-overs and a write) and its neighbours, over every product of: the order
   the network delivers C's message relative to the changes around it; concurrent clears (each write-over's target
   removed first by a concurrent change: four subsets); out-of-causal-order arrival (a later change that writes over
   C's write reaching a copy before C); redelivery (none, C twice, C again after the cut); a stable cut (none, one
   before C, one past C); and a rebuild by recall (none, from the journal and the history after the cut, from a
   snapshot at the cut). Run once through the core (a relay, three devices) and once through the Mesh causal route
   (three copies, no core). Every schedule ends with every copy's `values` and `adds` equal, each register told once
   per change, and the clock at or past every stamp held; a rebuild ends with the state before it. On today's rule
   the schedules of cases (a) and (b), and the snapshot past a settle, fail.
2. **The rule, per layer.** In `sce-rust-trust`/`sce-rust-peer`: a change is not delivered before its context, is
   delivered once whatever the relay repeats, and a gap is fetched, never skipped. In the Mesh core (C++ and Rust):
   the same rule on a `delivery: causal` route, repair by vectors, and the refusal of `ordering:` or a dedup window
   beside it.
3. **The list, on every lane.** New scenarios `record_list_concurrent_clears` (case a given as two concurrent changes
   in both orders: `y` gone, nothing held, the parent told once), `record_list_out_of_causal_order_is_not_given` (a
   change and the one it writes over, given in their causal order, the only order a layer gives), and
   `record_list_restore` (a snapshot at a cut, then changes after it, a live value of a change whose write-overs the
   cut forgot kept). They replay on Rust, Kotlin, Python, Go, C++, C11 and the Interpreter.
4. **The bound.** A C11 build of the list with `SCE_MAX_DATA_LEN` below its largest `record_list.change` is refused at
   build time; one at the bound carries a change of `shared_parts` operations whole.
5. **The fold.** A core given a relay's history of the second domain (the recorded scenario of ADR 0083 section 6.3,
   with multi-part changes and concurrent clears) folds it into the same snapshot whatever order it reads it in.
6. **Mutation casefiles** under `sce-build/tests/mutations` for the delivery rule's three branches in each layer, the
   stability cut, the build-time bound, and `take`'s write-over without a tombstone.

## Landing order

1. **The event-schema list field** (section 6), on every backend, with its own scenarios; nothing of the list moves.
2. **The relay** (ADR 0083 landing 1, protocol 2 and `conflict`), unchanged and independent.
3. **The Mesh core's causal route** (section 3.2), with its tests; nothing uses it yet.
4. **The list at change units, with ADR 0083 step 2 reworked**: `record_list.change`, the effect-only list, the
   journal per change, recall and restore, the scenarios on seven lanes, the world tests over the causal route, the
   saved-shape pin, every generated tree regenerated. The branch `adr0083-l2-journal` is reworked into this landing.
5. **The core** (ADR 0083 landing 3 at change units): one operation per change, `Link.context`, delivery by the rule,
   stability and its mark, history in causal order, the fold of the second domain, `sce-trust/event/3` once, ABI 7.
6. **The hosts**, one landing each (ADR 0078); a host that pins SCE takes landings 4 and 5 together.

## Consequences

- A change is atomic on the wire, in the signature, in the relay's store, in the journal and in every copy; no copy
  or host can hold or build half of one.
- The list is the effect and nothing else; it holds no tombstones, which ADR 0046 section 2 kept until a settle.
- Dedup, causal order, repair and stability are one layer's, with one rule, in the core and in the Mesh core.
- A settle needs no round of hashes among the copies: stability is read from the vectors the layer keeps.
- One more list-valued event field kind in the toolchain, on every backend.
- A deployed list moves across one protocol cut, folded once into a snapshot; a device not moved is cut off until it
  moves.

## Alternatives rejected

- **The earlier draft: record each part's arrival by its change's stamp and a signed `ordinal`** (an earlier draft of
  this decision, written the same day and withdrawn before it was accepted). It fixed cases (a) and (b) by giving
  the list a delivery layer of its own (an arrival list per change, a list of changes taken whole, a forgetting rule
  at the floor). Rejected as the wrong layer: it kept a unit cut into n messages and n signatures and reassembled
  by every receiver, built a second dedup and buffering mechanism above two layers that already number every update
  per author, and kept the tombstones that pay for non-causal delivery. The textbook places dedup, causal order and
  stability below the object, where the metadata is.
- **Keep the one-operation `write` and `supersede` beside `record_list.change`** for a change of one. Two shapes of
  one fact, two signed events, two delivery paths (section 1.4).
- **Fragment a change at the list's level when an event slot is small.** It is the parts machinery again. A bound too
  small for a change is a build error; a frame too small is the transport's (section 5).
- **Keep the list's digest walk as the repair for Mesh copies.** It repairs from a state hash because the layer below
  knew nothing; with a vector the layer repairs exactly what is missing, for every document, not for the list alone.
- **Explicit dependencies instead of a version vector** (each change names the dots of the changes whose values it
  writes over). Smaller, but it orders only what a change names, needs a map from a value's dot to its change's
  number, and gives no stability; the vector is bounded by the roster and gives all three.
- **Keep the settle round of hashes.** Agreement on a hash of the state is a stability test for a layer that has no
  vectors; it stalls behind any copy that differs, as case (a) showed.

## Open questions for the owner

1. **The list-valued event field: its own ADR or a section of this one.** It changes the grammar of every event
   schema and every backend's codec, and its rules (bounds, revisions, defaults, the trust encoding) are not the
   list's. Recommendation: its own ADR, landed first (Landing order 1), with this ADR citing it.
2. **The context's form: a version vector (as decided) or explicit dependencies.** Recommendation: the vector, for
   exact dedup, causal order and stability from one structure, bounded by `shared_roster` (section 3; Alternatives).
3. **The Mesh hub: retire the machine-level relay (as decided) or keep it with tags carried in the payload.**
   Keeping it would put the layer's tag into the list's event so that a machine can pass it on, which is the layer
   leaking upward. Recommendation: retire it; `#people` is a group, and a topic-less transport is forwarded by the
   Mesh core with tags intact.
4. **The fold of a deployed list's second-domain history at the protocol cut** (section 8.3), which cuts off a
   device not yet moved. The alternative is reading both domains for a period, which brings back a second shape and
   an unordered history. Recommendation: the fold and the cut, announced with the landing (ADR 0078).
5. **The C11 bound: refuse a small slot at build time (as decided) or let C11 carry large payloads out of line.**
   Recommendation: the build-time refusal now (it is ADR 0077's rule), and out-of-line payload storage as a C11
   backend decision of its own if a device needs changes larger than it can slot.
