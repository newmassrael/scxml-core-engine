# ADR 0093 — A copy's durable state is its journal of operations, and its own change is taken in once its layer accepts it

- Status: Proposed (2026-11-07), awaiting the owner's acceptance. Written at the owner's order of that day, under the
  rule of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored.
  Nothing of it has landed; "Landing order" lists the steps, each with the test that holds it.
- Date: 2026-11-07
- Amends, once accepted (each amended ADR gains an "Amended by: ADR 0093" line in the landing that moves it):
  - ADR 0046 section 4 ("What a copy does to itself never waits on what it sends"): a copy that has a delivery layer
    takes its own change in only after the layer's verdict on it; section 5 gains the reason `peerRefused`;
  - ADR 0079 section 7 (already amended by ADR 0083 section 6.4) and the status of its step 3: the premise that a
    replay regenerates the same writes and the core recognises them is withdrawn as a guarantee of any kind;
  - ADR 0083 sections 1.3 (the order of a change's take, report and send), 1.5 (recall), 2.1 (what the host commits
    and when), 2.2 (the rebuild), 3.1 (`Peer::made`) and 3.5 (`Kept::Held`);
  - ADR 0085 section 1.2 (the source takes its change in at once) and section 3 (the source's own change counts as
    delivered when the layer tags it, which here is also when the source takes it in).
- Scope:
  - `stdlib/shared/record_list.scxml` and its schemas: `record_verdict`, `record_stamp` and the enum `record_outcome`
    (new); `record_refused` revised; `record_refusal` gains `peerRefused`; `record_list.recall` (ADR 0083, at change
    units by ADR 0085) gains `provisional`; the list's scenarios and the seven drivers that replay them;
  - `sce-rust-trust` (`Trust::sign_waiting`), `sce-rust-relay` (`client::Round`), `sce-rust-peer` (`Peer`,
    `Outcome`), `sce-rust-peer-capi` (`include/sce_peer.h`, ABI version 6 today) and the Kotlin binding
    (`backends/kotlin/peer`);
  - `docs/SCE_SHARED_LIST_PEER_ABI.md` (sections 4, 5, 6, 9), `docs/SCE_SHARED_LIST_TRUST.md` (section 5);
  - the world tests of the list (`backends/rust/world`), the saved-shape pin, and every generated tree of the list.
- Related:
  - ADR 0034 (a shared list is held by its members; a device that signs two histories is frozen);
  - ADR 0042 (the shared record list is a standard part), ADR 0044 (the revisions of a schema), ADR 0045 (a standard
    statechart that saves is a promise about its shape);
  - ADR 0078 (a consumer follows SCE one landing at a time, and each API change says how);
  - ADR 0080 (a device's relay token, and the invitation that makes a device a node of a list);
  - ADR 0083 (a host records what a change made, and a rebuild never makes it again) and ADR 0085 (a change is one
    signed unit, and a layer below the list delivers it once, whole and in causal order), both accepted on
    2026-11-03; this ADR completes them and does not replace them.

## Context

### What a consumer measured

On 2026-11-07 a consumer reported a host application on a phone, one of two devices of a list held through a relay,
whose copy of `sce:std/shared/record_list` showed a row the person had retired, and values no other copy held. The
host rebuilds its machine at every start by giving it its own command log again (57 commands), as hosts do until
ADR 0083 lands. The core's answers to the 370 sends the replay produced were:

- 307 held (`Kept::Held`: the operation the core had signed for that write or note already);
- 46 writes refused, each with the trust layer's reason "its stamp goes back along this device's chain (a write at N
  has a later time)";
- 17 supersedes signed as new operations and published.

The same command log replayed on a desktop with no other device gives the expected list (one live row, no register
with two values, the retirement applied). The phone's copy had taken each of the 46 refused writes in before it sent
it, so they stayed in it: registers held a value of this device that no other copy held beside the value the live
run had written, and a row whose retirement had written over the live run's values was shown again with them. The
other device showed the retired row too; what reached it (a published supersede, or an earlier rebuild's write
that the core admitted) is not known from the report. The measurements are the consumer's; this repository holds
none of them, and this ADR rests on their kinds, not on their numbers.

### The mechanism, read from the tree

Every location below was read at `682a4375ed`, whose cited files are the same as at `3f6b32bd88`. The consumer's
note cited the same places; where its line numbers differed, the corrected ones are given and the difference is said.

1. **A change is stamped at close from a clock that peers move.** `HlcSend(clock, closingNow, node)`
   (`stdlib/shared/record_list.scxml:626`) keeps the greater of `now` and the clock's wall time and raises the
   counter when they are equal (`stdlib/merge/hlc_send.scxml:29-34`; the note cited the file alone). The clock is
   raised by every peer write and write-over the copy takes in, `HlcReceive` at `record_list.scxml:724-728` and
   `:783-787`. So a change's stamp is a function of the command and of the peer operations taken in before it. A
   peer whose wall clock runs ahead moves every later stamp of this device to the peer's time.
2. **A change writes over what the copy holds.** Close counts (`record_list.scxml:591-601`) and then builds
   (`:630-652`) a write-over of every value of `adds` in a register the change names. The note cited `:583-596` and
   `:629-650`, which cut the first loop in its middle; the loops are as given here.
3. **The core knows a write by its register, stamp and content.** A write's subject is the bytes of its fields whose
   role is not `Change` (`backends/rust/trust/src/message.rs:361-370`): `id`, `field`, `value`, `text`, `wallTime`,
   `counter`, `nodeId` (`message.rs:123-129`). `seal_write` returns the operation it holds for that subject
   (`backends/rust/trust/src/trust.rs:2560-2570`). ADR 0079 section 7 called the key "register and stamp"; the code
   adds the content, which changes nothing here, since the stamp already differs.
4. **A write whose stamp goes back along its author's chain is refused.** `Chain::probe` notes `Why::GoesBack` for a
   write with an earlier time than one before it on the chain (`backends/rust/trust/src/chain.rs:355-383`), and
   `make_operation` refuses it with the reason the consumer saw (`trust.rs:2060-2062`). A replay without the peer's
   operations stamps the device's writes at the commands' own `now`, earlier than the stamps the live run took after
   the peer's, so every write past the first such is refused: these are the 46.
5. **A note carries no time the chain checks.** Only a write has a stamp on the chain (`trust.rs:2025-2032`). A note
   names the write it removes, and one whose subject the core does not hold is signed as the device's next operation
   (`seal_note`, `trust.rs:2084-2106`; the note cited `:2084-2104`). A replayed change that writes over a write the
   replay re-made names a dot no other copy holds: a new subject, a new signed note. These are the 17.
6. **The copy takes its change in before it sends it.** Close queues every `record_list.take` before any
   `record_list.pass` (`record_list.scxml:678-683`, explained at `:548-555`); the note cited `:676-683`, whose first
   lines copy `taking` into `passing`. ADR 0046 section 4 made that order the rule, so that a send that fails where it
   is made cannot leave a copy without its own change. ADR 0083 section 1.3 keeps the order (take, then report, then
   pass) and ADR 0085 section 1.2 keeps it (take at once, then report). No ADR gives a refusal by the layer a place
   to come back to.
7. **A copy rebuilt from an older save takes its own writes back and raises its clock to them**
   (`record_list.scxml:711-721`, the clause at `:730-733`; the note cited `:708-728`). That path is what lets a
   rebuilt copy take the live run's writes back by a resync, beside the replayed writes the core refused: one
   register then holds two values of one author.

Three more things make a replayed command produce another operation than it did, on top of the stamp: the set of
values a change writes over (item 2); the parent statechart's own judgement of a command, which reads its own state
(a command that edited another device's row is refused when that row has not arrived yet, and the counters a parent
keeps then move); and a new build, whose parent or list may judge the same command otherwise. Only the first is
in the list; all three are a command run a second time against a state that is not the one it first ran against.

### What is decided already, what has landed, and what is left

ADR 0083 diagnosed the same root on 2026-11-03 from another measurement (182 new operations after one restart, no
refusal), and decided that the host records each change's effector and a rebuild recalls effectors and never runs
the generator again. ADR 0085 made a change one signed message, moved delivery, dedup, causal order and stability
into a layer below the list, and named that layer's log the PO-log. Both are accepted. Of their landings, only the
relay's protocol version 2 is on `main` (`backends/rust/relay/src/protocol.rs:22`); the list, the core and the ABI
(version 6, `backends/rust/peer-capi/include/sce_peer.h:38`) are as before. ADR 0083's Context also settled a
question the consumer's note left open: the world test that holds ADR 0079 section 7
(`a_copy_built_again_from_its_own_commands_writes_nothing_twice`,
`backends/rust/world/tests/a_shared_list_runs_through_a_relay_with_no_hub_machine.rs:362`) gives the rebuilt copy
no other device's operation between its commands, which is the one schedule under which the replay reproduces
itself.

This measurement shows three things those two ADRs leave open:

1. **The source takes in what its layer refuses.** The 46 refused writes are in the copy, and nothing in ADR 0083 or
   0085 prevents the same shape after they land: the source takes its change in at close and reports it after. A
   change the core refuses at once (`Kept::Refused`: another content under a signed stamp, a person the log does not
   let write, a frozen chain) or drops later (`Outcome::dropped`, `backends/rust/peer/src/lib.rs:197`) then lives in
   one copy. The present answer to a drop is `Outcome::rebuild`, a rebuild of the whole machine for one change; under
   ADR 0083 that is a recall of the whole history, and before it a command replay, which is this defect.
2. **A stamp is not held above the device's own chain.** A refused write is a stamp that went back along the
   device's own chain. ADR 0083 section 1.6 raises the clock to every own stamp a rebuild recalls, which covers what
   the host's journal holds and not what the core holds beyond it (a journal older than the chain, a journal lost).
3. **The parent's own state.** ADR 0083 records the list's effectors. A parent statechart that keeps state of its
   own derived from commands (counters, an undo stack, what a person deleted on this device) is still rebuilt by
   running commands, unless the host records what each command made for the parent too.

## The textbook structure

**State machine replication and its condition.** Replicas that start in one state and apply the same deterministic
commands in the same order end in the same state (Schneider 1990). A command whose result depends on anything not
in the ordered input is not such a command. In this list the input that decides a stamp and a write-over set
includes the arrival times of other devices' operations, which no log orders with the commands.

**Log-based rollback recovery.** Re-executing a process from its log reproduces its state only under piecewise
determinism: every nondeterministic event the process consumed is logged, and the replay is fed the logged outcome
(Elnozahy, Alvisi, Wang and Johnson 2002). A message a process sent before a failure is an
output to the outside world; a recovery that re-executes and sends it otherwise breaks every receiver that acted on
the first (the output commit problem of the same survey). A signed operation published to a relay is exactly such an
output.

**Operation-based CRDTs: prepare once, effect everywhere, and a log of operations as the state.** An update is
prepared once, at its source, reading the source's state, and its effector is applied at every replica, the source
included (Shapiro, Preguiça, Baquero and Zawirski 2011). The source's effect and the broadcast are one atomic step in
that model, which assumes a broadcast that never refuses. Baquero, Almeida and Shoker (2014, 2017) make the replica's
state the partially ordered log of the operations it delivered (the PO-log), tagged by a causal broadcast, and fold
it into a plain state at causal stability. The operation, once prepared, is a fact; nothing prepares it again.

**Write-ahead.** The record of a change is on stable storage before the change is made visible, and recovery redoes
from the log, never from the inputs that led to it (Gray and Reuter 1993, chapter 9; Mohan et al. 1992).

**Tentative and committed writes.** A replica that applies its own write before the write is committed holds it
tentatively, and undoes it when it is rejected, by an undo log of the tentative writes, without rebuilding the
replica from scratch (Terry et al. 1995; Petersen et al. 1997).

**Unique dots, and an incarnation that lost its state.** A dot names one event of one replica, the pair of the
replica's identifier and a counter it never reuses, and correctness of causality tracking rests on a dot naming one
event only (Almeida, Baquero, Gonçalves, Preguiça and Fonte 2014; operation identifiers of Kleppmann and Beresford
2017). A replica that lost the state its counter lives in cannot know which counters it used; the only identifier
that cannot collide with one it used is a fresh one. A hybrid logical clock is a valid counter for this purpose when
it is raised past every timestamp the replica has issued (Kulkarni et al. 2014), since it then never issues one
twice and respects happened-before (Lamport 1978).

**Compensation.** A history that has been published is not taken back; its effect is undone by a new action that
compensates for it (Garcia-Molina and Salem 1987).

Measured against these, the list has the effect right (`record_list.take`, one path for a copy's own operations and a
peer's), ADR 0083 and 0085 put the PO-log and the write-ahead record in place, and two pieces are missing: the
source's effect conditioned on the layer's acceptance, and a dot rule that holds across incarnations of a device.

## Decision

**A copy's durable state is its journal of operations, written ahead: the PO-log of Baquero et al. Operations are
made once, when a command is first judged, and recorded; a rebuild applies recorded operations and effect records,
and never runs a command again. A copy that has a delivery layer takes its own change in only after the layer has
answered for it: accepted or held, it is journalled and taken in; refused, nothing is taken in and the parent is told
`peerRefused`; waiting, it is taken in provisionally, and a later drop is withdrawn by a journal entry and an undo
record, not by a rebuild. A copy is (list, node); a node is never used by two incarnations; a new stamp exceeds every
stamp of the device in its journal and in its core's chain.**

### 1. The durable state is the journal of operations

1. **What the journal holds.** For each copy, in the host's storage, append-only, in the order the host wrote it:
   - each change of the device's own that its layer accepted or held (ADR 0085's `record_list.change` with the
     closing command's identifier, ADR 0083 section 1.2), and the core's record of its signed bytes (ADR 0083
     section 3.1);
   - each change of the device's own that its layer answered waiting, marked provisional;
   - each confirmation of a provisional change, and each withdrawal (section 3.3);
   - the stable snapshot and its cut (ADR 0085 section 4.3), after which the entries before the cut are dropped.
   The other devices' changes are not copied into it: the layer holds them, the core as its history
   (`Peer::history`, ADR 0083 section 2.5) and the Mesh core as its log for repair (ADR 0085 section 4.2), and a
   second durable copy of them would be a second original that can disagree with the first.
2. **The copy is the fold of the journal and the layer's history.** A machine's saved state, where a host keeps one,
   is a cache of that fold and is thrown away when its shape changes (ADR 0045). The settle floor of ADR 0083 section
   2.6 is the stable cut of ADR 0085 section 4.
3. **Write-ahead at the source.** The host commits a change of its own, its verdict and the core's record in one
   transaction before it gives the copy the verdict (section 3.1). The copy's effect, and every register the parent
   is told, comes after the commit, so nothing the person sees as saved is missing from the journal after a crash.
   This narrows ADR 0083 section 2.1, which committed after the machine was quiet.

### 2. Operations are made once; a rebuild applies them and runs no command

1. **The list.** A rebuild is ADR 0083 section 2.2 at change units (ADR 0085 section 7.2): restore the core, give it
   the journal's changes it does not hold (each is held or kept again, never prepared again), give a fresh machine
   the stable snapshot as `record_list.restore` (ADR 0085 section 4.4) and then every change of the history and of
   the journal after the cut as `record_list.recall`, own provisional changes with `provisional: true` (section 3.4),
   then `record_list.raise` (section 4.3). Nothing is sent while it does; no change is closed.
2. **The parent.** A command's judgement by the parent is a prepare as much as the list's close is. The host keeps,
   with each command, the parent's effect record: what the parent's judgement made (accepted or refused, and the
   identifiers, counters and other state it assigned). The parent is rebuilt from those records or from a saved state
   of it (ADR 0045), never by judging its commands again. The list tells the parent everything a parent needs to
   record (the registers, the verdict of each change, section 3); what the parent keeps beside that is the
   application's, and SCE does not prescribe its shape (Open question 3).
3. **The command log stays the record of intent.** It is kept for audit and undo, read by no rebuild. A new build
   rebuilds from the same operations and effect records and therefore holds what the old one held, whatever the new
   build would judge a command to be.

### 3. The source's effect comes after its layer's verdict

The list has a delivery layer when it is joined with `journal: true` (ADR 0083 section 1.2): the host's core (ADR
0085 section 3.1), the host's Mesh core on a causal route (ADR 0085 section 3.2), or, for a copy on one device with
no peer at all, a host that accepts everything. A copy joined with `journal: false` (a throwaway test copy) has no
layer to ask and takes its change in at close, as today.

1. **Offer and verdict.** `record_list.close` judges and stamps the change as today, builds it, and does not take it
   in. It holds it in `offered` (one change) and reports it to the host's journal route as `record_list.made` (ADR
   0083 section 1.2, one per change by ADR 0085 section 7.1): the report is the offer. The host gives the change to
   its layer (`Peer::made`, section 6), commits the journal entry (section 1.3), and gives the copy
   `record_list.verdict` (`record_verdict`: the change's `wallTime` and `counter`, and `outcome`, a value of the new
   enum `record_outcome`: `accepted` 1, `held` 2, `waiting` 3, `refused` 4, with `none` 0 declared and never sent).
   The core answers in the same call that took the change (`Kept`), so a host gives the verdict at once; the list's
   correctness does not rest on that, only on the next rule.
2. **One offer at a time.** A host gives a copy no command while an offer is outstanding: it runs the copy until it
   is quiet after each command (ADR 0083 section 2.1), and the verdict is part of that. A command a copy is given
   while it holds an offer is refused by `error.execution` and counted in `refusals`, a host's fault said where it
   happens rather than a change prepared on a state that lacks the one before it. Peer changes the layer delivers
   while an offer is outstanding are taken in as always: they are concurrent with the offered change, whose
   write-over set was fixed at close, and the effect path is order-independent.
3. **The four outcomes.**
   - `accepted`, `held`: the copy queues the change's takes, then reports `record_list.closed` (ADR 0083), and the
     parent is told each register once (ADR 0042's telling).
   - `refused`: the copy takes nothing in and drops the offer. Its registers, values and digest are what they were
     before the close. It reports `record_list.closed` with `reason` `peerRefused` and tells its parent
     `record_list.refused` with `reason` `peerRefused` (value 9 of `record_refusal`) and `withdrawn` false. The clock
     stays at the stamp: a stamp is never given twice, even to a change no copy holds.
   - `waiting`: the copy takes the change in, provisionally: it holds the change's stamp and an undo record in
     `provisional` (below), tells the parent each register, and tells it `record_list.pending` (`record_stamp`:
     the change's `wallTime`, `counter` and closing `command`). A provisional change is confirmed by `record_list.confirm` (`record_stamp`), when the layer signs it
     (`Outcome::taken`); the copy then drops its undo record and tells the parent `record_list.confirmed`.
4. **Withdrawal, by an undo record, never by a rebuild.** When the layer drops a waiting change, the host journals the
   withdrawal and gives the copy `record_list.withdraw` (`record_stamp`) naming it. The copy withdraws it and every
   provisional change after it, latest first, and tells the parent each register the withdrawals moved, once, and
   `record_list.refused` with `peerRefused` and `withdrawn` true for each change withdrawn.
   - **Why the suffix.** The core keeps every send after one that waits waiting, in order (ADR 0079's landing of
     2026-11-02, "A send the core cannot sign yet is kept"), so every change after a provisional one is provisional;
     each was prepared on a state that held the one before it, and its parent's judgement may have read that state.
     Withdrawing only the change that was dropped would leave changes in every copy prepared on a state no copy
     holds. The core withdraws the same suffix (section 6.2), so the copy and the layer agree on what is withdrawn.
   - **The undo record.** A provisional change's undo record holds each value its write-overs took out (register,
     dot, value, text), at most `shared_parts`. Withdrawing a change removes the values its writes added (every value
     whose dot is the change's stamp under this node) and gives back each value of its undo record. A write-over taken
     after the provisional change, from any author, that names a dot an undo record holds strikes that dot from the
     record: another change took the value out too, and the value stays out. The rule's property: after a
     withdrawal, the copy holds what a copy that never took the withdrawn changes holds after taking every other
     change it took, in any order (Landing order, step 2).
   - **The bound.** `provisional` holds at most `shared_provisional` changes (a build limit, 8 by default); a change
     closed while it is full is refused `noRoomForHistory`, the reason ADR 0085 section 2.5 reads as the changes a
     copy's layer holds not yet stable, which a provisional change is.
5. **Recall of a provisional change.** A rebuild recalls an own change the journal marks provisional with
   `provisional: true`; the copy takes it in and builds its undo record as at the verdict, so a withdrawal after a
   restart behaves as one before it. A history gives the device's own waiting changes after every change they follow
   causally, as ADR 0085's causal order already places them.

### 4. Identity: a copy is (list, node), a node has one incarnation, a stamp exceeds every own stamp

1. **The replica's identifier is (list, node).** The node is the one the membership log names the device by (ADR
   0080 section 4). It is never used by a second incarnation of the device: an incarnation that lost both its
   journal and its sealed state (ADR 0079 step 7), including one set up again from its seed alone, joins the list as
   a new node, granted by an administrator as any new device is. An incarnation that holds a sealed state, even an
   older one (a passphrase backup, ADR 0079 section 6), keeps its node behind the wait for its own chain (ADR 0079
   step 3; ADR 0083 section 3.4).
2. **Two dots, one order.** The list names a write by its dot, (node, HLC stamp) (ADR 0046 section 1); the layer
   names a change by (author, seq), its number on the author's chain (ADR 0085 section 3.1). The chain's rule that a
   write's time does not go back along it (`chain.rs:355-383`) is the statement that the two orders agree. A stamp
   that goes back is therefore not a fault of the core to be worked around but a dot rule the source broke.
3. **The stamp floor.** A new stamp exceeds every stamp of the device in its journal and in its core's chain. The
   journal's part is ADR 0083 section 1.6 (recall raises the clock to every own stamp). The chain's part is
   `record_list.raise` (`record_stamp`): the host gives it after every rebuild and whenever the core reports a higher
   own stamp (`Peer::own_stamp`, section 6), and the copy raises its clock to it as it does to its own write (the
   maximum, `record_list.scxml:730-733`), never by `HlcReceive`, since it is no peer's.
4. **Before the core has read back its own chain.** The core signs nothing then (ADR 0079 step 3), so every change
   closed is answered `waiting` and is provisional (section 3.3). When the round that reads the chain back finds own
   operations beyond what the core held, the host gives `record_list.raise` with the new floor; any provisional change
   whose stamp goes back is dropped by the core with its suffix and withdrawn by the copy (section 3.4). So no stamp
   below the floor is ever confirmed, and a device that writes offline right after a start keeps its writes in the
   common case of a journal that is complete. A stricter rule, no close at all before the read-back, is Open
   question 1.

### 5. The list's events and schemas

| Event | Direction | Schema | Section |
|---|---|---|---|
| `record_list.made` | copy to host (`#journal`) | `record_made` (ADR 0083, at change units by ADR 0085) | 3.1: it is the offer |
| `record_list.verdict` | host to copy | `record_verdict` (new): `wallTime`, `counter`, `outcome` | 3.1 |
| `record_list.confirm` | host to copy | `record_stamp` (new): `wallTime`, `counter`, `command` (default 0) | 3.3 |
| `record_list.withdraw` | host to copy | `record_stamp` | 3.4 |
| `record_list.raise` | host to copy | `record_stamp` | 4.3 |
| `record_list.recall` | host to copy | ADR 0085's change, revised: `provisional` (since 2, default `false`) | 2.1, 3.5 |
| `record_list.restore` | host to copy | ADR 0085 section 4.4, unchanged | 2.1 |
| `record_list.closed` | copy to host (`#journal`) | `record_closed` (ADR 0083), after the verdict | 3.3 |
| `record_list.pending`, `record_list.confirmed` | copy to parent | `record_stamp` | 3.3 |
| `record_list.refused` | copy to parent | `record_refused` revision 2: `command` and `withdrawn` (since 2, defaults 0 and `false`) | 3.3, 3.4 |

`record_list.restore` and `record_list.recall` are the one way a copy is built from operations; neither sends
anything nor closes a change. `record_refusal` gains `peerRefused` = 9: the change was refused by the copy's
delivery layer, at once or by withdrawal. Every revision is a field with a default (ADR 0044). The list's saved
shape gains `offered` and `provisional`, and the pin in `sce-build/tests/fixtures/standard_saved_shapes.json` moves
with the commit that moves it (ADR 0045).

### 6. The core and the C ABI

1. **Hand over what the core holds.** `Peer::history` (ADR 0083 section 2.5, in causal order by ADR 0085 section
   3.1) marks each of the device's own changes that still waits to be signed as provisional, so that a host whose
   journal is incomplete (lost, or older than the core) can rebuild the journal from the core and recall those
   changes with `provisional: true`.
2. **Withdraw the suffix.** `Trust::sign_waiting` refuses a waiting send that can never be signed and every waiting
   send after it, in order (today a send that conflicts is dropped alone and the sends after it may be signed,
   `trust.rs:2492-2523`). `Outcome::dropped` becomes `Outcome::withdrawn`, the stamps of the device's own changes
   withdrawn, earliest first, each with why; `Outcome::rebuild` is no longer raised for a withdrawal.
3. **Report the own chain's last stamp.** `Peer::own_stamp()` gives the highest stamp of the device's own changes in
   the chain the core holds, and whether the core has read its own chain back (`awaits_own_chain` today); a round's
   outcome says when it rose.
4. **Answer a send in the same step.** `Peer::made` (ADR 0083 section 3.1) answers with `Kept` in the call that took
   the change: `Taken` (the host gives `accepted`), `Held` (`held`), `Waiting` (`waiting`), `Refused` (`refused`).
   The host journals the record of a taken change in the transaction of section 1.3, then acknowledges it
   (`Peer::recorded`), as ADR 0083 section 3.1 says.
5. **The ABI.** `sce_peer_own_stamp(ScePeer *, int64_t *wall_time, uint32_t *counter, int32_t *read_back)`, new;
   `sce_peer_history` writes `"provisional": true` on a waiting own change; `sce_peer_take_outcome` writes `withdrawn`
   in place of `dropped`. A lane of the version before does not know `withdrawn` and would rebuild where it must
   withdraw, so the version policy (`docs/SCE_SHARED_LIST_PEER_ABI.md` section 11) makes it a new number: one more
   than the version current when this lands. Folded into ADR 0085's core landing, as recommended (Open question 5),
   that is the one version 7 which ADR 0083 section 6.3 plans. The Kotlin binding gains each call. The recorded
   scenario (`tests/fixtures/two_devices_and_a_relay.json`) is recorded again with a refused change, a waiting change
   confirmed, a waiting change withdrawn with one after it, and a restore whose read-back raises the floor; every lane
   replays it.

### 7. Repair of a list a replay has already polluted

What was signed and published is not taken back; repair is new operations (Garcia-Molina and Salem 1987).

1. **Each device's copy.** The first start under this ADR builds the journal from the core: `Peer::history` gives
   what the core holds, own and others', and the copy is recalled from it (ADR 0083 section 6.5's conversion). A
   write the core refused is in no core, so it leaves the copy with the conversion; no operation is made.
2. **What spread.** The host compares what the core holds of its device against what its record of intent means
   (its command log judged alone, which is the reference the consumer's desktop replay already computes), and finds:
   notes that name a dot no change holds; registers holding more than one value of the device; values of the device
   that its record of intent wrote over and that are live; rows its record of intent retired that hold values.
3. **Compensation.** A note that names no held dot is inert: under ADR 0085's effect a write-over whose value is gone
   changes nothing, and under today's list its tombstone leaves at the next settle. The rest is repaired by ordinary
   changes the host makes now, from commands it marks as repairs in its journal: an edit, which writes over every
   value the copy holds in its register (ADR 0046 section 1) and so leaves one; a clear; a retire. They are journalled,
   offered and accepted as any change (section 3), carry new dots, and converge on every copy by the list's own rule.
4. **A fresh list, as the fallback.** When a device's chain is frozen (ADR 0034) or the repair is larger than the
   list, the members open a new list (a new genesis, ADR 0079's administrator's acts) whose first changes are the
   repaired state, and retire the old one. It loses the history and asks every member to join again, so it is the
   fallback and compensation the standard path.

### 8. What a host changes (ADR 0078)

- Its journal is the PO-log of section 1, committed before the verdict is given; its command log stays for audit.
- It gives `record_list.made` to `Peer::made`, commits, gives `record_list.verdict`, and gives the copy no command
  while an offer is outstanding.
- After a round: `Outcome::taken` becomes `record_list.confirm`, `Outcome::withdrawn` becomes a journal entry and one
  `record_list.withdraw` naming the earliest, a risen `own_stamp` becomes `record_list.raise`.
- Its start: restore, give the core the journal's changes, recall (provisional ones marked), raise, then sync.
- Its parent statechart is rebuilt from the parent's effect records or its saved state, not from commands.
- A device whose journal and sealed state are both gone joins as a new node.

## Landing order

Each step lands with the test that holds it, every test is written so that the structure it replaces turns it red,
and each new guard of the list or the core gets a mutation casefile under `sce-build/tests/mutations`.

1. **A reproduction in the world, before anything moves.** In `backends/rust/world/tests/`, beside
   `a_copy_built_again_from_its_own_commands_writes_nothing_twice` and on its harness: two devices and the relay that
   is their hub; device 2's commands stamped with a `now` ahead of device 1's (its clock runs ahead); device 2's
   changes delivered to device 1 between each of device 1's commands; then device 1's copy built again by the path
   hosts take today (its command log given to a fresh machine, the core restored), and a resync on every device. It
   measures the core's new operations of device 1 during the rebuild, the sends refused or dropped, and whether every
   copy's live values and adds are equal. Today: refusals above 0, new notes above 0, and the rebuilt copy differs
   from device 2's. That arm is kept for good as the negative control (ADR 0083 section 7.1's "must fail, checked
   once" made permanent): it holds the defect and says why commands are not replayed. The target arm, added by step
   2 and passing from then on, rebuilds from the journal and the history: 0 new operations, 0 refusals, every copy
   equal.
2. **The list.** Offer and verdict, the four outcomes, provisional changes with their undo records and the strike
   rule, withdrawal of the suffix, `record_list.raise`, `peerRefused`, the schema revisions, the saved-shape pin, and
   every generated tree of the list regenerated in the same landing. Folded into ADR 0085 landing 4 if that has not
   landed (Open question 5). Tests, on Rust, Kotlin, Python, Go, C++, C11 and the Interpreter: scenarios
   `record_list_verdict` (accepted; held; refused, after which the copy's values, adds and digest equal those before
   the close and the parent is told `peerRefused` once), `record_list_provisional` (waiting then confirmed; waiting
   then withdrawn, after which the copy equals one that never took it; a peer's write-over of a value the withdrawn
   change also took out, which stays out; two provisional changes, the first withdrawn, both withdrawn latest first),
   `record_list_raise` (a raise above the clock moves it, one below does not, the next stamp exceeds it) and a command
   given while an offer is outstanding refused by `error.execution`. In the world, the property test of the
   withdrawal: for random sets of changes, peers' changes interleaved and random withdrawals, the copy after the
   withdrawals equals a copy given every change but the withdrawn ones.
3. **The core and the ABI.** `Peer::own_stamp`, the provisional mark in `Peer::history`, the suffix in
   `Trust::sign_waiting`, `Outcome::withdrawn`, the ABI version and the Kotlin binding, the recorded scenario recorded
   again (section 6.5). Folded into ADR 0085 landing 5 if that has not landed. Tests: in `sce-rust-trust`, a waiting
   send that can never be signed withdraws itself and every one after it and signs none of them; in `sce-rust-peer`,
   a restored device whose read-back finds own operations beyond its journal reports the higher own stamp and
   withdraws the provisional changes below it; every lane replays the recorded scenario; the world's Kotlin device of
   `a_shared_list_is_held_through_the_core_on_every_device` is restarted through the ABI and publishes nothing.
4. **The documents.** ADR 0046, 0079, 0083 and 0085 each gain "Amended by: ADR 0093" with the sections this ADR
   names; `docs/SCE_SHARED_LIST_PEER_ABI.md` sections 4, 5, 6 and 9 and `docs/SCE_SHARED_LIST_TRUST.md` section 5
   say the verdict, the suffix and the own stamp; the list's document comment says the offer and the verdict. The
   test: the reproduction of step 1, both arms, and the recorded scenario, are cited from the ABI document.
5. **The hosts**, each in a landing of its own, with the migration of section 8 and the repair of section 7.

## Consequences

- No copy holds a change its layer refused; a change waiting for its layer is shown as provisional and withdrawn in
  place if it is dropped, and no drop rebuilds a machine.
- A restart publishes nothing and refuses nothing, whatever other devices wrote and whenever it arrived; a new build
  holds what the old one held.
- A device's stamps rise along its chain across restarts, restores and journal loss, so the trust layer's "goes back"
  refusal is reached only by a defect.
- A device that lost everything is a new node; a lost device is not continued under its old dots.
- The list gains an offer and four host inputs (verdict, confirm, withdraw, raise), three parent tellings, an undo
  record per provisional change and one build limit; `record_refusal` gains one variant and `record_refused` one
  revision.
- The ABI moves once with ADR 0085's core landing; the core's drop of a waiting send takes the sends after it.
- A host commits each change before its copy shows it, and keeps an effect record for its parent beside each command.

## Alternatives rejected

- **Record each change's stamp with its command, and replay commands with the recorded stamps.** It fixes the stamp
  and nothing else: the write-over set, the parent's judgement and its counters, and a new build's judgement still
  differ, and a refused change still stays in its source.
- **Record the other devices' arrivals between the commands, and replay the whole interleaving.** It meets
  Schneider's condition for one build and breaks with the next, keeps a second durable copy of every peer operation
  beside the core's that can disagree with it (an eviction by the core is an input too), and still takes in a change
  the layer refuses.
- **Rely on the core's idempotence** (ADR 0079 section 7, the status quo). It holds only for byte-identical sends,
  which the generator does not produce (ADR 0083's Context, and this one's), and turns a replay into new published
  operations or refusals the source keeps.
- **Take the change in at close and roll it back by rebuilding on a refusal** (today's `Outcome::rebuild`). A rebuild
  for each refused change, and before ADR 0083 lands a rebuild is a command replay, which is the defect itself; after
  it, a full recall for an event that touches one change.
- **Let the core restamp a refused write above its chain.** It prepares the operation a second time, at the layer,
  and the copy then holds a write the layer never signed beside one it did; the write-over set is still the first
  close's. An operation is made once.
- **A deterministic generator** (stamp by the command alone): rejected in ADR 0083 for what the stamp must order.

## Open questions for the owner

Each has a recommendation; the sections are written to it.

1. **Before the core has read back its own chain, may a copy close a change?** Recommended: yes, provisionally
   (section 4.4); a change whose stamp proves to go back is withdrawn, and offline writing after a start, which ADR
   0079's landing of 2026-11-02 exists for, is kept. The stricter alternative refuses or holds every close until the
   read-back: no change is ever withdrawn for its stamp, and a device cannot write until it has reached its relay
   after each start.
2. **A device set up again from its seed alone.** Recommended: it joins as a new node (section 4.1), so that no
   dot's uniqueness rests on the relay's word about how far a chain goes, which is ADR 0034's limit for a relay that
   withholds. The alternative keeps its node behind the wait for its own chain, as ADR 0083 section 3.4 allows.
3. **The parent's effect record.** Recommended: the host's, with no SCE shape (section 2.2); the list's tellings and
   verdicts carry what a parent needs. The alternative is a standard pattern in SCE for a parent statechart rebuilt
   from records (an input that applies a recorded judgement), which would also serve applications without a list.
4. **Withdrawal granularity.** Recommended: the suffix of provisional changes (section 3.4). Withdrawing only the
   changes that write over a value of the dropped one is finer, but misses a dependency through the parent's
   judgement, which the list cannot see.
5. **Landing.** Recommended: the list's part folded into ADR 0085 landing 4 and the core's into landing 5, one ABI
   version for all three ADRs; the alternative lands this ADR after them, with one more ABI version.
6. **A repair aid in SCE.** Recommended: none in the core; the comparison of section 7.2 needs the host's record of
   intent, which SCE does not hold. The world test of step 1 shows the comparison on its own two devices.
7. **`shared_provisional`.** Recommended default 8: each provisional change holds an undo record of up to
   `shared_parts` values, which a C11 build allocates statically.

## References

- Almeida, P. S., Baquero, C., Gonçalves, R., Preguiça, N., Fonte, V. "Scalable and Accurate Causality Tracking for
  Eventually Consistent Stores." DAIS 2014, LNCS 8460, pp. 67-81.
- Baquero, C., Almeida, P. S., Shoker, A. "Making Operation-based CRDTs Operation-based." DAIS 2014, LNCS 8460,
  pp. 126-140.
- Baquero, C., Almeida, P. S., Shoker, A. "Pure Operation-Based Replicated Data Types." arXiv:1710.04469, 2017.
- Elnozahy, E. N., Alvisi, L., Wang, Y.-M., Johnson, D. B. "A Survey of Rollback-Recovery Protocols in
  Message-Passing Systems." ACM Computing Surveys 34(3), 2002, pp. 375-408.
- Garcia-Molina, H., Salem, K. "Sagas." SIGMOD 1987, pp. 249-259.
- Gray, J., Reuter, A. *Transaction Processing: Concepts and Techniques.* Morgan Kaufmann, 1993.
- Kleppmann, M., Beresford, A. R. "A Conflict-Free Replicated JSON Datatype." IEEE Transactions on Parallel and
  Distributed Systems 28(10), 2017, pp. 2733-2746.
- Kulkarni, S. S., Demirbas, M., Madappa, D., Avva, B., Leone, M. "Logical Physical Clocks." OPODIS 2014, LNCS 8878.
- Lamport, L. "Time, Clocks, and the Ordering of Events in a Distributed System." CACM 21(7), 1978.
- Mohan, C., Haderle, D., Lindsay, B., Pirahesh, H., Schwarz, P. "ARIES: A Transaction Recovery Method Supporting
  Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging." ACM TODS 17(1), 1992.
- Petersen, K., Spreitzer, M. J., Terry, D. B., Theimer, M. M., Demers, A. J. "Flexible Update Propagation for
  Weakly Consistent Replication." SOSP 1997, pp. 288-301.
- Schneider, F. B. "Implementing Fault-Tolerant Services Using the State Machine Approach: A Tutorial." ACM
  Computing Surveys 22(4), 1990, pp. 299-319.
- Shapiro, M., Preguiça, N., Baquero, C., Zawirski, M. "Conflict-free Replicated Data Types." SSS 2011, LNCS 6976,
  pp. 386-400; in full as "A comprehensive study of Convergent and Commutative Replicated Data Types," INRIA
  Research Report RR-7506, 2011.
- Terry, D. B., Theimer, M. M., Petersen, K., Demers, A. J., Spreitzer, M. J., Hauser, C. H. "Managing Update
  Conflicts in Bayou, a Weakly Connected Replicated Storage System." SOSP 1995, pp. 172-183.
