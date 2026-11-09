# ADR 0085 — A change is one signed unit, and a layer below the list delivers it once, whole and in causal order

- Status: Accepted (2026-11-03). Written at the owner's order of that day, which chose this structure over an
  earlier draft (recorded in "Alternatives rejected"), under the rule of 2026-10-26 and 2026-10-27: the most
  long-term correct structure, textbook in its foundations, cost ignored. The owner accepted the draft the same day
  with the answers recorded in "Owner's answers". Amended after acceptance on 2026-11-08, at the owner's order: the
  Mesh core's causal route is written to the owner's answers Q1 to Q8 of that day (sections 3 and 3.2 rewritten,
  section 3.3 new, section 9.2, "Landing order", "Owner's answers"). The author of a dot is a replica, membership
  is a numbered view, a copy keeps its own highest number in its journal and begins a new incarnation when it cannot
  know it, and landing 3 is split into a codec step and one step per Mesh core. Amended again on 2026-11-09, under
  the same rule: landing 4 is split into lettered steps by what depends on what, the core's delivery by the rule and
  its signing of a change move from landing 5 into landing 4, which cannot be green without them, and the
  prerequisites landing 4 waits on in ADR 0086 are named ("Landing order", "Owner's answers").
- Date: 2026-11-03; amended 2026-11-08 and 2026-11-09
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
  - the Mesh core (`SCE_MESH.md` section 10): a causal delivery setting beside `ordering:` and `receipt:`, its
    group and view, the envelope's tag entries and the control message `causalVector`, the standard Mesh algorithms
    of the rule (`stdlib/mesh`), and the four Mesh cores (Rust, C++, Kotlin, Swift);
  - the CBOR codec (`sce-build/src/forge/cbor_codec.rs` and every backend's CBOR codec): an entry may be a bounded
    list of records (answer Q1);
  - `sce-rust-trust` (`message.rs`, `chain.rs`, `trust.rs`), `sce-rust-relay`, `sce-rust-peer`, `sce-rust-peer-capi`;
    `docs/SCE_SHARED_LIST_TRUST.md`, `docs/SCE_SHARED_LIST_RELAY.md`, `docs/SCE_SHARED_LIST_PEER_ABI.md`;
  - the list's scenarios and the seven drivers that replay them, the world tests of the list, the saved-shape pin,
    and every generated tree of the list.
- Related: ADR 0034 (a shared list is held by its members; the relay only relays), ADR 0044 (the revisions of a
  schema), ADR 0045 (a standard statechart that saves is a promise about its shape), ADR 0077 (a declared bound is
  part of a field's type), ADR 0078 (a consumer follows SCE one landing at a time), ADR 0080 (a device's relay token),
  ADR 0086 (an event's field may hold a bounded list: landing 1 of this decision), ADR 0093 (a copy's durable state is
  its journal, and its own change is taken in once its layer accepts it).

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

(Amended 2026-11-08, answers Q2, Q3, Q6 and Q7: the author is a replica, the members are a numbered view, the
source's change is answered by a verdict, and a copy's own highest number is durable. Section 3.3 is new.)

**The tag.** The layer tags each change it accepts from its source with its **dot**, `(author, seq)`: the author is
the source's **replica identity**, one incarnation of one member of the group (section 3.3), and `seq` is the change's
number among that replica's changes, from 1 with no gap. It also tags it with its **causal context**, the layer's
version vector of what it had delivered to that copy when it accepted the change: one `(author, seq)` entry per
replica of the current view it delivered anything of, so at most the view's size (section 3.3), which `shared_roster`
bounds for the list. The list never sees the tag. A context taken when the change is handed down may name a change
delivered to the copy's queue that the list had not yet taken when it prepared; that overstates the change's causal
past, which only orders more than needed and is never unsafe.

**The rule, the same in both places.** For a copy whose delivered vector is `D`, a change tagged `(a, s)` with
context `C`:

- if `s <= D[a]`, it is delivered already: it is dropped;
- if `s = D[a] + 1` and `C[b] <= D[b]` for every author `b` other than `a`, it is delivered to the list, whole, and
  `D[a]` becomes `s`;
- otherwise it is held until it is deliverable; a gap is repaired by asking a peer for the changes the copy's vector
  lacks, never skipped.

**The source's own change: offer and verdict.** The layer answers each change its source offers with a verdict
(ADR 0093 section 3.1), and the copy takes the change in only after it. The core's verdicts are ADR 0093 section
6.4's. The Mesh route answers `Accepted(dot)`, the dot it tagged the change with, or `Refused(reason)`. An accepted
change counts as delivered at the source when it is tagged (`D[self]` becomes its `s`). A refused change takes no
number and is taken in by no copy; a layer whose log has no room left for a change not yet stable refuses it, and
the list tells that refusal as `noRoomForHistory` (section 2.5).

**A copy's own number is durable.** On the Mesh route the host records each accepted change with its dot in the
copy's journal, in the transaction of ADR 0093 section 1.3, and the layer releases the change to no peer before that
record is committed. A restart reads the replica identity and its highest number from the journal and continues
after it. A crash between the tag and the commit loses a number no peer has seen and no copy took in, so using it
again names no second change. A copy that cannot know its highest number (restored from an older save, or set up
again from its seed) does not ask its peers for it: it begins a new incarnation, with a new replica identity
(section 3.3). The core's numbers are its chain's, which it keeps in its sealed state (section 3.1).

**Stability.** A change is stable when every member of the current view has delivered it. The stable cut is the
pointwise minimum, over the replicas of the current view, of the latest delivered vector known from each; a member
whose vector is not known yet holds its column of the cut at 0. Every member's vector is known from the contexts of
its changes and, for a member that has not written since, from a mark the layer sends of its own vector.

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

(Rewritten 2026-11-08 to answers Q1 to Q8.)

- **A binding setting that names its group.** A route a deploy binds with `delivery: causal` (a Mesh-core setting
  beside `ordering:` and `receipt:`, `SCE_MESH.md` section 5, "Mesh-core delivery settings") is a causal broadcast
  group, and names the group and its initial view:

  ```yaml
  bindings:
    "#people":
      transport: wss
      delivery: causal
      group:
        name: people
        view: [phone/phone_list, laptop/laptop_list, tablet/tablet_list]   # the initial view, epoch 0
  ```

  `group:` is the binding's own key. `members:` keeps its one meaning, the members of a DDS or Zenoh pool
  (`SCE_MESH.md` section 14.4), and is not read on a causal binding. Each entry of `view` names one instance the
  deploy declares, as `<device>/<machine>`. A machine name is one entry of the whole deploy
  (`mesh/deploy-duplicate-machine`), so the copies of one document are machines of their own, named apart, each
  with the same `source:`. Every member sends to the route and reads it.
- **The tag on the envelope.** The sending side stamps each envelope with the tag, as optional CBOR envelope entries,
  as `sequence_no` is (section 10.6.3): the author's replica identity, the number, the view's epoch, and the context
  as a bounded list of `(author, seq)` records in the envelope's own CBOR map. A CBOR codec entry holds a scalar
  only today, so the codec gains entries of a bounded list of records first (Landing order, step 3a; answer Q1). A
  context written as bytes inside a byte string is not offered: it would be a second encoding of one value, outside
  the type grammar every backend generates from.
- **The rule before dispatch.** The receiving side applies the rule of section 3 before the engine sees anything. An
  envelope tagged with another epoch than the receiver's current view, or by a replica the view does not hold, is
  not delivered and is raised as `error.communication` (a row of `SCE_MESH.md` section 16.7 that the landing adds).
- **One control message, `causalVector`.** A new variant of `PatternKind` (`stdlib/mesh/pattern_kind.scxml`) is
  the layer's only control message and never reaches an engine. It carries its sender's replica identity, the view's
  epoch and its delivered vector, and it serves two ends: the **anti-entropy summary**, to which a receiver answers
  with every change of its log that the vector lacks, sent as the original envelopes with their tags intact; and the
  **mark** of a member that has not written since, which the receiver records as that member's latest known vector
  for the stable cut.
- **Repair, not skipping.** A causal route has no gap timeout: section 10.6's fast-forward drops a gap, which would
  lose an effector for good. Each member sends `causalVector` on a period and when a link is back. That is what
  `record_list.resync` and the digest walk did, done once in the layer for every document that needs it.
- **Retention until stable.** The layer keeps every change it sent and every change it delivered until the stable
  cut passes it, to answer repairs, and drops it there (section 4.2). The log's room is a bound the deployment sets;
  an offer the log has no room for is refused (section 3, "offer and verdict").
- **No machine-level relay.** The route `#people` is one group every copy sends to and reads; a transport without a
  topic is realised by the Mesh core forwarding envelopes at a relay node with their tags intact. A hub machine that
  passed changes on would re-send them under its own identity, which is why the hub's relay goes (section 2.1). The
  hub flag of `record_list.join` keeps no role in the list and is retired.
- **One delivery rule per binding.** Section 10.5's dedup window and section 10.6's per-source buffer are subsumed
  on a causal route by the vector, which dedups exactly and orders causally. A causal binding skips the dedup window
  (the build's `needs_dedup` is false for it). `ordering:` beside `delivery: causal` on one binding is refused with
  a new code, `mesh/deploy-delivery-conflict`, and so is a machine's `dedup:` section when every binding it would
  reach is causal: that section would read as configured and filter nothing, which is why `window_size: 0` is
  refused too (section 10.5).
- **One rule on every core.** The rule, the merge of a delivered vector, what a vector lacks and the stable cut are
  written once, as standard Mesh algorithms (`stdlib/mesh`), and generated for each language; each core keeps the
  state they are applied to and the envelopes themselves, as it does for sections 10.5 and 10.6. Four cores take the
  route: Rust (the reference), then C++, Kotlin and Swift, all before landing 4. Until its core has landed, a
  language refuses `delivery: causal` at build time, by name; no build ignores the setting.

#### 3.3 Views and incarnations

(New 2026-11-08, answers Q2, Q3 and Q6.)

1. **A replica is one incarnation of one member.** The author of a dot is a replica identity that is unique within
   the group and never used again (ADR 0093 section 4). It is not the machine name, the envelope's `source`
   (`SCE_MESH.md` section 10.9): a machine name names a kind, and two instances of one machine would share a column
   of every vector, while a refusal of the duplicate would forbid running a machine more than once. It is not
   `routing_id` either (section 10.9): a router draws that at each construction, so it changes at every restart
   while a replica's numbers continue, and it keeps its own role, the self-filter of an echo. The replica identity is
   a 16-byte UUID v7 the layer draws when an incarnation begins, kept in the copy's journal (ADR 0093 section 1.1)
   from that moment, beside the highest number the incarnation used. Section 10.9's invariant 5 keys a sender's
   window and buffer on `source` and foresaw a second axis for a machine hosted more than once; the causal route
   keys on the replica identity, a stable axis.
2. **A copy that cannot know its own highest number begins a new incarnation.** A normal restart continues from its
   journal (section 3). A copy restored from an older save, or set up again from its seed, draws a new replica
   identity, so no dot is used twice, which is crash recovery with incarnation numbers. Asking the peers for the
   highest number is unsafe: the change with that number may be held only by a member that is offline. The members
   that answer report a lower number, the copy gives its next change the dot of the change that member holds, and
   when the member comes back one dot names two contents. The rule then drops whichever arrives second as delivered
   already, so copies that took the first never take the second, and the group diverges for good. No answer from
   the members reachable now bounds what an unreachable one holds; the copy's own journal does, and a fresh identity
   needs no bound. The old incarnation's changes stay delivered, and its column stays in the view until a view
   change removes it.
3. **Membership is a numbered view, `(epoch, members)`.** As in virtual synchrony (Birman and Joseph 1987; Chockler,
   Keidar and Vitenberg, "Group Communication Specifications: A Comprehensive Study", ACM Computing Surveys 33(4),
   2001), the group's members are a view with an epoch. The deploy gives the initial view, epoch 0. Each member of a
   view is held by one incarnation at a time, whose replica identity the member's first envelope of the epoch
   presents. Every tagged envelope and every `causalVector` carries its epoch; stability and the cut are computed
   over the current view's members; a context holds at most one entry per replica of the view.
4. **Changing the view may land later; the model is view-based from the start.** Join and leave (and a member's new
   incarnation, which is the old incarnation's leave and the new one's join) are a protocol of their own (Landing
   order, step 7). The data model, the envelope and the rule carry the epoch from landing 3 on, so that protocol adds
   messages and changes none. Until it lands, epoch 0 is the only view: a second replica identity for a member is
   not delivered, so a copy restored from an older save or set up from its seed is cut off from its group, and its
   old incarnation's column holds the stable cut where it stopped. A deployment plans for that until step 7.

For a host with a core, ADR 0093 section 4.1 governs a device's incarnations: a device set up from its seed alone
joins as a new node, and one that holds an older sealed state waits for its own chain. That wait is on the relay,
which holds every link a chain published, since a link reaches every other device through it; on a Mesh route no
one place holds every change, which is why the copy there does not wait (decided while writing, below).

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
of the toolchain, not of the list, and lands before the list moves (Landing order; Owner's answers, 1).

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
   delivered once whatever the relay repeats, and a gap is fetched, never skipped. In the Mesh core, on each of the
   four cores as it lands (amended 2026-11-08), the tests below; each names the change to the rule that turns it red,
   which the landing checks once against that mutant:
   - **Drop.** A change with `s <= D[a]`, given again after it was delivered, is not delivered a second time. Red if
     `<=` becomes `<`.
   - **Deliver.** The next change of an author whose context is covered is delivered once and raises `D[a]` to `s`,
     so that author's following change is delivered next. Red if `D[a]` is not raised.
   - **Hold, a gap of the author.** A change with `s > D[a] + 1` is held, and delivered once the change before it is.
     Red if `s = D[a] + 1` is weakened to `s > D[a]`.
   - **Hold, an uncovered context.** A change whose context names a change of another author not yet delivered is
     held until that change is. Red if the check `C[b] <= D[b]` is removed.
   - **No gap timeout.** A held change stays held however long the world's time runs, and is delivered only after
     the change before it is repaired. Red if a gap timeout fast-forwards the author's column.
   - **Repair by vector.** A copy that lost a change receives it after one `causalVector` exchange, on the period
     and when its link is back. Red if what a vector lacks is computed as empty, or if a link that comes back sends
     no vector.
   - **The stable cut.** The cut is the pointwise minimum over the current view, with a member not heard from at 0.
     Red if the minimum becomes a maximum, or if a member whose vector is unknown is left out.
   - **The mark.** A member that has not written moves the cut by its `causalVector` alone. Red if a received
     vector is not recorded as that member's latest.
   - **Retention.** The log drops a change once the cut passes it and not before, and a repair asked for a change
     the log still holds is answered. Red if the log is pruned at the copy's own delivered vector instead of the cut.
   - **The verdict.** An offer the log has no room for is answered `Refused` and takes no number, so the copy's next
     accepted change is delivered at every peer. Red if the number is taken before the refusal.
   - **Write-ahead.** No accepted change reaches a peer before the host commits its journal record. Red if the core
     transmits at the tag.
   - **Incarnation.** A copy started without a trustworthy journal tags under a new replica identity, and a world in
     which the old incarnation's last change sits at an offline member ends with no dot naming two contents. Red if
     the copy reuses its previous identity, or continues from the highest number its reachable peers report.
   - **The view.** An envelope of another epoch, or of a replica the view does not hold, is not delivered and is
     raised. Red if the epoch check is removed.
   - **The settings.** `ordering:` beside `delivery: causal`, and a machine's `dedup:` section that reaches only
     causal bindings, are refused with `mesh/deploy-delivery-conflict`; a causal binding's `needs_dedup` is false.
     Red if either refusal is removed, or if the build's per-binding policy ignores `delivery:`.
   - **The language refusal.** A language whose core has not landed refuses `delivery: causal` by name. Red if the
     refusal is removed, which builds the binding as a plain route.
   - **The wire.** An envelope with the tag entries is written to the same bytes by every core (one golden vector).
     Red if one core numbers an entry's key otherwise.
   - **The world.** In `backends/rust/world`, three copies on links that lose, duplicate and reorder deliver every
     change exactly once, in an order that is a linear extension of the contexts, with no gap skipped, and their
     logs drop to the cut once the links are quiet. Red if the hold branch delivers, or if repair is removed (a lost
     change then never arrives).
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
3. **The Mesh core's causal route** (sections 3.2 and 3.3), with its tests; nothing uses it yet. Amended 2026-11-08
   (answers Q1 and Q8) into lettered steps, so that landings 4 to 6, which other decisions cite by number, keep their
   numbers:
   - **3a. CBOR codec entries of a bounded list of records** (answer Q1), before every core: ADR 0086's one type
     grammar applied to a CBOR codec's entry, in `sce-build/src/forge/cbor_codec.rs` (whose entries take a scalar
     only today) and in the CBOR codec of every backend that generates one (Rust, Kotlin, C++, Go, Python, C11,
     Swift), with C11's sizing, fixtures and conformance. The envelope does not move in this step.
   - **3b. The route on the Rust core, the reference**: the setting with its group and view, its refusals and
     `mesh/deploy-delivery-conflict`; the envelope's tag entries and `causalVector`; the standard Mesh algorithms of
     the rule, the merge, what a vector lacks and the cut; the Rust core's offer and verdict, write-ahead release,
     repair, retention, cut and mark, and its replica identity and incarnations; the tests of section 9.2 and the
     world test. Every Mesh tree is regenerated from `stdlib/mesh` in the same step, and the other three languages
     refuse `delivery: causal` by name.
   - **3c. C++**, **3d. Kotlin**, **3e. Swift**: each core applies the same generated algorithms, takes the tests of
     section 9.2, and lifts its language's refusal. All three land before landing 4.
4. **The list at change units, with ADR 0083 step 2 reworked**: `record_list.change`, the effect-only list, the
   journal per change, recall and restore, the scenarios on seven lanes, the world tests over the causal route, the
   saved-shape pin, every generated tree regenerated. The branch `adr0083-l2-journal` is reworked into this landing.
   Amended 2026-11-09 into lettered steps, as landing 3 was, so that landings 5 to 7 keep their numbers. Each step
   leaves every lane green and no protocol half moved. The split follows two facts read from the tree at
   `8de85a1ec8`:
   - **The list and the core speak one wire.** The world tests through the core run the list itself
     (`backends/rust/world/tests/common/devices.rs`: the machine's sends are given to `Peer::outbound`, and what a
     round brings back is raised into it), and the trust layer signs `record_list.write` and
     `record_list.supersede` (`backends/rust/trust/src/message.rs:34-38`). A list that sends `record_list.change`
     while its core signs parts leaves those tests red; a core that signs both leaves two shapes of one fact alive
     (section 1.4). The list's wire and the core's signing of a change are therefore one step (4c).
   - **The effect without tombstones is correct only over causal delivery.** The core's relay may reorder and may
     answer less than it holds (`Dishonesty` in the same file): a write-over given before the write it names leaves
     that write live when it arrives, once no tombstone keeps it out. The core's delivery by the rule therefore
     lands before the list loses its tombstones (4b), not after it in landing 5.

   Two prerequisites are ADR 0086's own steps, and landing 4c waits on both:
   - **Composed payloads on C11 and Swift** (ADR 0086 step 3, its C11 and Swift lanes). `ComposedLowering::of`
     (`sce-build/src/forge/event_schema_check.rs:281`) lowers a read, a copy and a send of a composed payload on
     Rust, Kotlin, Go, Python and C++ only, and C11 and Swift refuse such a statechart by name. The list reads
     `_event.data.ops` and sends `ops`; it is built on C11 (`test_static_scalars.c`) and has a committed Swift tree.
   - **C11's slot** (ADR 0086 step 5). C11's event slot is sized by hand at 256 bytes
     (`backends/c/runtime/include/sce/types.h:76`), and the largest `record_list.change` of the list as written is
     47,900 bytes (4a pins the number). Step 5 derives the slot from the machine's largest payload. It carries the
     build-time refusal of section 5, and the C11 lane builds the list with `shared_parts` and `merge_text`
     lowered.

   The steps:
   - **4a. The change's schemas and their sizes.** `record_op` and `record_change` (`record_list.change`), with
     `ops` typed `list<record:Op>[1..shared_parts=64]`, so an empty change cannot be written on any wire (section
     1.1). Nothing imports them yet and no protocol moves; a generated tree changes only in the generator's source
     hash its header carries, since the standard library is part of that hash. Their sizes are pinned on
     every backend that generates a schema unit, as the number section 5 says the build knows and ADR 0086 step 5
     sizes C11's slot by. Files: `stdlib/shared/record_op.scxml`, `stdlib/shared/record_change.scxml`,
     `sce-build/tests/a_change_of_the_shared_list_is_one_event_of_a_known_size.rs`.
   - **4b. The core delivers by the rule** (section 3.1), over the operations it signs today: `Link.context`, the
     core's delivered vector at signing, in the link's bytes (a new domain of the link); every round, the history
     and every hand-over give the machine only operations whose context it has delivered, in a linear extension of
     the contexts; a held operation waits in the core, which asks the relay for what it lacks; nothing is skipped.
     The rule is the layer's and does not read the event kind, so it lands before the list moves, and the list's
     tombstones are then redundant until 4c drops them. Tests: section 9.2's three tests of the core, and a
     round over a relay that withholds a write from its feed and gives its write-over, in which the core holds
     the write-over, asks for the write, and tells both in causal order. The recorded scenario of the C ABI is
     recorded again, and the six lanes that replay it (C, C++, Python, Go, Kotlin, Swift) replay it. Files:
     `backends/rust/trust/src/{message,trust}.rs`, `backends/rust/relay/src/client.rs`,
     `backends/rust/peer-capi/tests/fixtures/two_devices_and_a_relay.json`, the trust tests that pin bytes
     (`a_byte_format_does_not_drift.rs`), `docs/SCE_SHARED_LIST_TRUST.md`, `docs/SCE_SHARED_LIST_RELAY.md` section
     6. Decided while landing it (2026-11-09): the core tells its machine operations when a round ends, by the
     delivered vector it keeps beside its operations and saves in its own text, and not as it takes each in; a
     context entry naming what no machine is ever told (past a freeze, or past the last operation the
     administrators accepted of a device that may write no more) is met once the machine was told the device up
     to there, since the rule would otherwise hold an honest device's operations behind a frozen one for good; an
     operation sealed under an epoch before the first one a host holds the key of, which a member that joined
     later is given no history of, is passed over and told never, so what follows it in its chain is not held for
     it (and is not told later should an older key come); a
     context names neither its own author nor a device the log had not granted, and is refused otherwise; and an
     operation the Mesh endpoint admits (`Admission`, a copy on a Mesh route) is not held by the core, since on a
     causal binding the Mesh core's own layer delivers it (section 3.2).
   - **4c. The list at change units, on both layers.** After 4a, 4b and both prerequisites. The list of sections 1
     and 2: `record_list.change` sent on `#people` and taken whole, the effect without tombstones, digest, settle or
     hub relay, the hub flag of `record_list.join` retired, and the schemas section 2.1 lists retired. In the same
     step the trust layer signs one `record_list.change` as one operation under `sce-trust/event/3`, its
     `operation_bytes` holding `ops` as section 6 says, and the core gives and takes `record_list.change`; the
     shape of the events an outcome holds changes, so the ABI takes the next number by its version rule
     (`docs/SCE_SHARED_LIST_PEER_ABI.md`, the paragraph on numbers). The scenarios move as section 8.5 says:
     the parts steps of `record_list_change` and `record_list_retire` are rewritten as changes, the walk and settle
     steps of `record_list_hub` and `record_list_person` leave the list (each property is held by landing 3's
     tests of the Mesh core and 4b's of the core), and `record_list_concurrent_clears` and
     `record_list_out_of_causal_order_is_not_given` (section 9.3) are new; all replay on the seven lanes. The world
     tests: the devices of `common/devices.rs` on change units; the Mesh tests of the walk and the settle
     (`a_resync_reaches_every_wall_time_and_every_field_number.rs`,
     `a_copy_restored_from_an_old_save_is_repaired_by_resync.rs`,
     `a_shared_calendar_forgets_what_its_copies_agree_on.rs`) moved onto a `delivery: causal` binding, with what
     each held stated of the layer; `todo_peer` rebuilt (section 1.4). The saved-shape pin moves, a mutation
     casefile holds `take`'s write-over without a tombstone, and every generated tree of the list is regenerated.
     Files beyond those named: `stdlib/shared/record_list.scxml` and its schemas, `record_list_parent.scxml` and
     `todo_peer.scxml` under `sce-build/tests/fixtures/static_datamodel`, the scenarios and their seven drivers
     (section 8.4), the relay's, the peer's and the MLS crate's tests that build operations, and
     `sce-build/tests/fixtures/standard_saved_shapes.json`.
   - **4d. The journal per change, recall, restore and the verdict.** ADR 0083 step 2, reworked from
     `adr0083-l2-journal` to change units (section 7), with ADR 0093 step 2 folded in as that ADR asks: `journal` on
     `record_join` (default `true`), `command` on the three commands, `record_list.made` per change (the fields of
     `record_change` and `command`), `record_list.closed`, the offer and its verdict with the four outcomes,
     provisional changes, withdrawal, `record_list.raise`, `record_list.recall` and `record_list.restore` (section
     4.4), and the own write-over's clock (ADR 0083 section 1.6). Every throwaway copy (the scenarios, the world
     tests of 4c, the devices of `common/devices.rs` until landing 5 gives the core `Peer::made`) joins with
     `journal: false`. Scenarios on seven lanes: `record_list_journal`, `record_list_recall`,
     `record_list_restore` (section 9.3), and ADR 0093's `record_list_verdict`, `record_list_provisional` and
     `record_list_raise`. The saved-shape pin moves again and every generated tree is regenerated. Files: the
     schemas named, `stdlib/shared/record_list.scxml`, the scenarios and their drivers,
     `docs/SCE_ACCEPTED_SUBSET.md` where it lists the list's events.
   - **4e. The cross product in the world, over the causal route** (section 9.1, the Mesh half): three copies with
     no core, each with a host journal that gives every `record_list.made` to its Mesh core's offer and gives the
     verdict back, over every product section 9.1 names, the rebuild by recall and by restore included. Its
     negative arm, kept for good as ADR 0093 step 1 keeps its own, runs the same schedules over a binding without
     `delivery: causal`, and must end with copies that differ: that is the run against a rule with no causal
     layer that section 9 asks for. Files: `backends/rust/world/tests/` (a new test and its harness in
     `common/`).
5. **The core** (ADR 0083 landing 3 at change units): the outbox, stability and its mark, history in causal order,
   the fold of the second domain, the next ABI number. Amended 2026-11-09: one operation per change,
   `sce-trust/event/3` and `Link.context` with delivery by the rule moved into landing 4 (steps 4b and 4c).
6. **The hosts**, one landing each (ADR 0078); a host that pins SCE takes landings 4 and 5 together.
7. **View changes on the Mesh route** (section 3.3.4, added 2026-11-08): join, leave and a member's new incarnation,
   on every core, with their tests. It may land after landing 4; until it does, a member restored from an older save
   or set up from its seed is cut off from its group, and a deployment of a causal route plans for that.

## Consequences

- A change is atomic on the wire, in the signature, in the relay's store, in the journal and in every copy; no copy
  or host can hold or build half of one.
- The list is the effect and nothing else; it holds no tombstones, which ADR 0046 section 2 kept until a settle.
- Dedup, causal order, repair and stability are one layer's, with one rule, in the core and in the Mesh core.
- A settle needs no round of hashes among the copies: stability is read from the vectors the layer keeps.
- One more list-valued event field kind in the toolchain, on every backend, and a CBOR codec entry of a bounded list
  of records (landing 3a).
- On the Mesh route a dot's author is a replica of a numbered view; a copy that lost its journal begins a new
  incarnation and, until view changes land (landing 7), is cut off from its group.
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
- **The deploy's machine name as the author of a dot** (answer Q2). Two instances of one machine would share a
  column of every vector; refusing the duplicate would forbid running a machine more than once.
- **Learn one's own highest number from the peers before tagging** (the text of section 3 before the amendment of
  2026-11-08; answer Q6). The change with that number may be held only by a member that is offline, so the peers'
  answer can be too low, and a number used twice names two changes by one dot.
- **The context as bytes inside a byte string** (answer Q1). A second encoding of one value, outside the type
  grammar every backend generates from.
- **A fixed roster without an epoch** (answer Q3). A change of membership would then change the meaning of every
  vector in flight; a numbered view says which members a vector and a cut are over.
- **Keep the settle round of hashes.** Agreement on a hash of the state is a stability test for a layer that has no
  vectors; it stalls behind any copy that differs, as case (a) showed.

## Owner's answers

The draft asked five questions, each with a recommendation. The owner accepted the ADR on 2026-11-03 with every
recommendation as the answer; the sections named are written to the answers.

1. **The list-valued event field is an ADR of its own, landed first** (Landing order 1). It changes the grammar of
   every event schema and every backend's codec, and its rules (bounds, revisions, defaults, the trust encoding) are
   not the list's; section 6 states only what this decision needs of it, and this ADR cites it once it exists. Its
   number is assigned when it is written.
2. **The causal context is a version vector**, not a set of explicit dependencies: one structure gives exact dedup,
   causal order and stability, bounded by `shared_roster` (section 3; "Alternatives rejected").
3. **The Mesh hub's machine-level relay is retired.** `#people` is one group every copy sends to and reads, and a
   transport without a topic is forwarded by the Mesh core with the tags intact; carrying the tags in the list's
   event so that a machine could pass them on would be the layer leaking upward (section 3.2).
4. **A deployed list's second-domain history is folded once at a hard protocol cut**, announced with the landing
   (ADR 0078): a device not yet moved is cut off from the list until it moves, and no copy reads both domains for a
   period (section 8.3).
5. **The C11 bound is a build-time refusal now** (ADR 0077's rule: a slot smaller than the machine's largest payload
   is refused when the machine is built). Out-of-line payload storage is not part of this decision; it is a later,
   separate C11 backend decision, taken only if a device needs changes larger than it can slot (section 5).

No question is left open.

Scoping landing 3 raised eight more questions. The owner answered them on 2026-11-08, under the same rule (the
structure that is correct longest and is the textbook one, cost ignored); Q2, Q3 and Q6 differ from the
recommendations put to the owner, and Q8 is wider than recommended. Sections 3, 3.2, 3.3, 9.2 and the landing order
are written to them.

- **Q1. The context is a bounded list of records in the envelope's CBOR map.** CBOR codec entries gain that type in
  a step of their own before the route (landing 3a), as ADR 0086's one type grammar applied to the codec. A context
  carried as bytes inside a byte string is not acceptable.
- **Q2. The author of a dot is the instance's stable replica identity, not the deploy's machine name.** A machine
  name is a kind, not a replica: two instances of one machine would share a column, and refusing the duplicate would
  forbid scale-out. The identity is unique within the group and never used again (ADR 0093), and section 3.3.1 says
  how it relates to `routing_id`, which is not stable across a restart.
- **Q3. A causal binding names its group, whose membership is a numbered view `(epoch, members)`**, as in virtual
  synchrony. The deploy gives the initial view, epoch 0; stability and the cut are computed over the current view.
  The protocol that changes the view may land later (landing 7), but the data model, the envelope and the rule are
  view-based from the start. The binding's key does not collide with `members:`, which names a DDS or Zenoh pool.
- **Q4. One control message of the layer, `causalVector`**, a new `PatternKind`: both the anti-entropy summary and
  the mark of a member that has not written.
- **Q5. `ordering:` beside `delivery: causal` on one binding is refused with `mesh/deploy-delivery-conflict`.** A
  causal binding skips the dedup window, and a machine's `dedup:` section that reaches only causal bindings is
  refused with the same code.
- **Q6. A copy keeps its own highest number durably in its journal, written before the change is released** (ADR
  0093: the journal is the durable state). A normal restart continues from the journal; a copy restored from an
  older save or set up again from its seed does not wait for its peers, and joins as a new incarnation with a new
  replica identity, so no number is used again (crash recovery with incarnation numbers). This replaces the sentence
  of section 3 by which such a copy learned its number from its peers: an offline member may hold its latest change.
- **Q7. A causal send is answered `Accepted(dot)` or `Refused(reason)`**, under ADR 0093's offer and verdict; a
  full retention log is a refusal, told as `noRoomForHistory`.
- **Q8. The rule is written once, as `stdlib/mesh` algorithms, and generated per language.** All four Mesh cores
  land before landing 4: Rust, the reference, then C++, Kotlin and Swift. Until its core has landed, a language
  refuses `delivery: causal` at build time.

Decided while writing, within those answers, and open to the owner's review: the binding's keys `group:`, with
`name` and `view` (section 3.2); the replica identity as a 16-byte UUID v7 drawn when an incarnation begins and kept
in the journal (section 3.3.1); a member of a view bound to the replica identity its first envelope of the epoch
presents, a second identity being a view change (section 3.3.3); the epoch carried by every tagged envelope; a
member not heard from holding its column of the cut at 0; the layer releasing an accepted change only once the
host's journal record is committed (section 3); landing 3's steps lettered so that landings 4 to 6 keep their
numbers, and view changes as landing 7; and Q6 read for the Mesh route alone, a device with a core keeping ADR 0093
section 4.1's wait for its own chain, which waits on the relay that every published link reached (section 3.3).

Decided while splitting landing 4 (2026-11-09), under the same rule and open to the owner's review:

- **The core's delivery by the rule and its signing of a change move into landing 4.** Landing 4 as written, with
  the core left to landing 5, is red: the list's wire and the core's signing are one wire, and the effect without
  tombstones needs the core to deliver causally ("Landing order", item 4). Taking the core with the list keeps
  every step green and no protocol half moved. Landing 5 keeps its number and the rest of its work.
- **No step of parallel change.** A core that signs both the parts and the change for a while, or a list that
  takes `record_list.change` beside the parts, is the textbook migration between deployed parties, and here no
  host pins main between landings 4 and 5 (landing 6). It would leave two shapes of one fact on main, which
  section 1.4 refuses.
- **The tombstones go in the step that moves the wire, not after it.** Keeping them while the list moves to
  change units would keep the settle round that forgets them, whose messages are wire too.
- **The prerequisites stay ADR 0086's steps.** Composed payloads on C11 and Swift and C11's derived slot are
  properties of the toolchain (Owner's answers, 1), so this ADR names them and does not take them. Section 5's
  build-time refusal is ADR 0086 step 5's derived slot. No second assertion is added here.
- **A write's dot is carried once.** In `record_op` a write's `wallTime`, `counter` and `nodeId` are not read: a
  write's dot is its change's stamp (ADR 0046 section 1), carried once in `record_change`, so no copy of it can
  disagree with it. A sender writes 0 there. A union of a write and a write-over would say it in the type, and is
  not used while unions are lowered on Rust alone (ADR 0092).
- **`ops` holds at least one operation** (`[1..shared_parts=64]`, ADR 0086 section 15), so "an empty change is
  never sent" (section 1.1) is a property of the type that every reader refuses (`tooFew`).
- **An event new in this landing begins at revision 1.** `record_list.recall`, first shipped in step 4d, carries
  ADR 0093's `provisional` in its first revision, not as a revision 2 of an event no build ever sent.
- **The ABI number moves when an outcome's events change shape** (4c), by the ABI's own rule, and again when
  landing 5 withholds what a host has not acknowledged. ADR 0083's "version 7" is read as the next number at that
  landing.
- **The world cross product keeps its negative arm** (4e). The check against "today's rule" that section 9 asks
  for cannot be run on a tree that no longer holds that rule. A permanent arm over a binding with no causal layer
  is a check that keeps running.

Decided while landing 4c (2026-11-09), under the same rule and open to the owner's review:

- **A staged command is a record of its own, `record_staged`.** A command is not an operation: a clear and a
  retire name what is to be written over, and the values they write over are the ones the copy holds when the
  change ends, which only then become write-overs. Typing `staged` by `record_op` would give `kind` two meanings,
  and keeping `record_part` for it would keep the parts' fields (`by`, `atWallTime`, `parts`) alive in the list.
  `record_command` numbers the commands as `record_staged`'s `kind` does.
- **The change in hand is one list, taken and sent.** `taking` holds the change's operations in their wire form
  (a write's dot 0, section 1.1), the source takes them in and then sends that same list as `ops`, and a peer's
  change is copied into it whole. The write's dot is read from the change in hand (`dot`, its stamp with its
  author), which every operation of one change is taken under before anything else reaches the copy. One list is
  one shape: nothing builds a second form of a change to send.
- **The clock moves once per change.** A peer's change moves it by `hlc_receive` of the change's stamp, once,
  after every operation of it is queued, so a clock that cannot move ends a block that has queued the change
  already. A change of the copy's own node, which a copy started again from an older save is given back, raises
  it to the stamp without a tick, as a copy's own write did. The list moved it once per part before, which counted
  one receive per operation of one message; the textbook clock (Kulkarni et al. 2014) receives a message once.
- **A write the copy holds already moves nothing.** It is kept though the layer never gives a change twice: it is
  one comparison, and it makes a copy's own change given back to it after a restore harmless. It does not make
  the list idempotent across a write-over: a write given again after the write-over that named it would come back,
  which is why exactly-once delivery is the layer's duty and not the list's (section 2.2).
- **The roster is judged on every copy, a peer's change included.** A change whose author the roster does not let
  write is taken in as no change and counted in `denied` (section 2.4); with no hub left, no copy judges for the
  others.
- **`record_room` keeps its three fields; the list reads `values` alone.** `history` is the layer's room, which
  the layer's verdict refuses past (section 3, landing 4d), and `digest` is read by no copy. A field dropped
  from the schema would refuse nothing (a key a schema does not declare is stepped over, ADR 0086 section 4.1)
  but would break the parent that forwards it; it stays declared and documented as unread.
- **`record_list_hub` is retired.** With no hub, what it held that stays the list's (the roster judged on a peer's
  change) moves into `record_list_person`, and the walk and the settle leave with the list's delivery (section
  8.5). `record_settled`, the hash a settle compared, leaves the standard library with the settle round.
- **The two scenarios of section 9.3 are written where they are observed.** `record_list_concurrent_clears` runs on
  `record_list_parent`, so that "told once" is a count the parent keeps; `record_list_out_of_causal_order_is_not_given`
  runs on the list itself, whose `values` and `adds` are read. The run that turns the first red under the old rule
  is the same schedule given to the list at `7ff8257b04` in parts (Alice's change as two `record_list.supersede`
  of `parts` 2): its write-over of x is `already` and its write-over of y is held for good, so y stays live and the
  parent is never told it empty. The second holds the order the layer gives; its old-rule failure is case (b), an
  order the layer never gives, and is held by the negative arm of 4e.
- **`sce-trust/event/3` is deterministic CBOR, and so is the operation.** Section 6 asks for the ops list "as its
  length and then each element's fields in their declared order and kinds"; ADR 0086 section 9 says that from this
  ADR's one domain bump the bytes a signature covers are the deterministic CBOR of `[domain, list, event,
  payload]`, the operation the CBOR of `[operation domain, message bytes, link]`, and the length prefixes go. The
  bump moved into 4c (amendment of 2026-11-09), so the encoding moves with it: one domain names one encoding, and a
  second bump for CBOR at landing 5 would be the two cuts this ADR refuses. A CBOR array is its length and then its
  elements, so section 6 holds; a record is a map keyed in the order of the keys' encodings, which ADR 0086 section
  9.2 fixes and which is no less deterministic than declared order. The operation domain keeps its number,
  `sce-trust/operation/2`: 4b's form of it (texts with their lengths) was never on main, and 4b and 4c land
  together. The vectors of `a_byte_format_does_not_drift` were made by an encoder written apart from the crate's
  (CBOR by hand, SHA-256, Ed25519 and XChaCha20-Poly1305 of another library), which gave the old vectors back first.
- **The table holds the shape, not the build's bounds.** `message::CHANGE_TYPE` and `message::OP` type each field
  as `record_change` and `record_op` do, and give a text and the list no bound: a deployment moves `shared_parts`
  and `merge_text` (`--limit`), and a write's text travels sealed, longer than the text it seals. The build and the
  list refuse a change past its bounds; the trust layer refuses what the type itself says (an empty change, a
  number outside its integer) and a change that writes one register twice, whose two writes would be one write of
  two contents sealed under one nonce. The world test holds the table to the schemas by name, order and type.
- **A change is named by its dot and kept by its bytes.** `Signing::subject` is the CBOR of `by`, `wallTime` and
  `counter`; what a host keeps a change by is the bytes it is signed as, its content opened. A second content under
  one dot is refused at the source and held as a conflict (`Why::Repeats`) elsewhere: every write of a change is
  sealed under its register and the dot, so two changes at one dot are a nonce used twice. The notes' table and
  their provenance go: no copy sends a write-over again as itself, so no host puts an author back on one.
- **Each write is sealed apart, under new domains.** The nonce and the associated data are made from the write's
  register and its change's dot (`message::write_stamp`), under `sce-trust/nonce/2` and `sce-trust/content/2`.
  The bytes of that stamp are those of a write sent alone under the second event domain with the same register and
  stamp, and one epoch's key may hold both across the cut (section 8.3), so the domains move to keep the nonces
  apart. A write-over carries no content and is not sealed. A change that holds no write waits for the epoch's key
  as one that holds writes does: a change is sealed whole under one epoch.
- **The core's settlement and its floor stay until landing 5.** The list keeps no tombstones and reads no
  `record_list.floor`, so the floor a host may still raise changes nothing; the checkpoints, the settlement made of
  them and `sce_peer_floor_event` are the core's word of stability, which landing 5's stable cut and mark replace,
  with the ABI's next number. Removing them here would move the six lanes' bindings twice for one landing.
- **The ABI is version 7.** What `sce_peer_outbound` takes and the events an outcome holds change shape, which the
  ABI's rule makes a new number; the lanes that pin the number (Python, Kotlin) move with the header.
