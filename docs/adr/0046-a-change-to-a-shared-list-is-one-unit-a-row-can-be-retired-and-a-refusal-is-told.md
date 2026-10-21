# ADR 0046 — A change to a shared list is one unit, a row can be retired, and a refusal is told

- Status: Accepted (2026-10-21), under the owner's standing rule: the most long-term correct way,
  cost ignored
- Date: 2026-10-21
- Amends: ADR 0042 (the shared record list is a standard part); its section 4 rule that the
  protocol is changed in both copies at once is amended in section 7 below
- Scope: `stdlib/shared/record_list.scxml` and its schemas under `stdlib/shared/` (`record_edit`,
  `record_write` and `record_supersede` at revision 2; `record_clear`, `record_retire`,
  `record_take`, `record_refused` and the enum `record_refusal`, new);
  `sce-build/tests/fixtures/static_datamodel/record_list_parent.scxml` and the list's scenarios;
  the scenario member `router` and the seven drivers that read it
  (`backends/rust/scenarios/src/vocabulary.rs` holds the vocabulary);
  `backends/rust/world/tests/a_world_run_is_what_every_backend_replays.rs`;
  `sce-build/tests/fixtures/standard_saved_shapes.json`; docs/SCE_ACCEPTED_SUBSET.md §2.15
- Related: ADR 0034 and docs/SCE_SHARED_LIST_TRUST.md (the trust layer the list is secured by),
  ADR 0044 (the revisions of a schema), ADR 0045 (a standard statechart that saves is a promise
  about its shape), ADR 0005 (bounding and declaring)

## Context

An application built on `sce:std/shared/record_list` found four gaps, and one bound the list
never stated:

1. **Nothing can be emptied or forgotten.** `record_list.edit` always adds a value. A person can
   write over a value only by writing another, so a register is never empty, and a row the
   application deletes keeps one live value in each of its fields forever. Settling forgets only
   tombstones, so `values` fills (512 by default) after a few dozen records and stays full. The
   list had no command that writes over a value without writing one, and a supersede a parent
   sent its list was taken in there and never passed on.
2. **An offline copy loses an overwrite** (a defect). In `record_list.edit` a person's copy queued
   the supersede of each value it saw for itself, then sent the same supersede to `#hub`, and
   queued its own write only after the loop. With no Mesh router (a copy on one device, or one
   whose host has not attached one), the send to `#hub` is `error.execution`, which ends the
   block: the old value was taken out and the new one never written, so the register was left
   empty. The hub's `record_list.ack` had the same shape: it sent the settled floor to `#people`
   before it queued its own, so a hub that could not reach its people never settled itself. No
   scenario could show either, because every scenario driver registers a router.
3. **A refusal was never told.** A change the list refused (its member may only read, the room is
   full, the text is too long) was counted in `denied`, `full` or `refusals`, which a parent
   statechart cannot read. The parent was told nothing, and reported success for a change that
   was not made.
4. **A record is not written as one.** An application spreads a record over the fields of one row
   (ADR 0042 section 1) and writes them one `record_list.edit` at a time. Each is stamped, sent and
   taken in alone, so another member's parent is told the row field by field, and a refusal
   halfway (the fourth field's text is too long) leaves a row with three fields that no person
   ever saved.

And the list does not say how long a text a person may write, or how an application lets a person
write a longer one.

## Decision

### 1. A change is a unit: staged, judged whole, stamped once, taken whole

A **change** is the commands a copy is given from the first one after the last change ended, up to
and including the first command that does not say `more`. There are three commands:

| Command | Schema | What it does to the registers it names |
|---|---|---|
| `record_list.edit` | `record_edit` (revision 2: `more`, since 2, default `false`) | writes over every value the copy holds in the register, and writes the new value |
| `record_list.clear` | `record_clear`: `id`, `field`, `now`, `more` | writes over every value the copy holds in the register, and writes none |
| `record_list.retire` | `record_retire`: `id`, `now`, `more` | writes over every value the copy holds in any register of the row, and writes none |

A command with `more` is **staged** (`staged`, at most `shared_staged` commands) and nothing else
happens: nothing is stamped, sent or told. The command that ends the change (no `more`, which an
`edit` from a revision-1 sender never carries) **closes** it, and the change is judged as one:

1. the copy's member may write (`record_list.role`);
2. no register is written or cleared by two commands of the change: the order of the commands
   inside a change means nothing (below), so two answers for one register are refused rather than
   one chosen; a retire beside an edit of its own row is not two answers, it is the row written
   anew;
3. the room (`record_list.room`): refused when the values the copy would hold after the change pass
   `values` (a change that writes no more values than it writes over is never refused for it), or
   the copy holds as many tombstones (`history`) or digest nodes (`digest`) as it was let — the
   rules a single edit was judged by, which they still are;
4. it fits: at most `shared_parts` operations (below), and a clock that can stamp it.

A change that passes is stamped once, with the closing command's `now` (`hlc_send`): every write
of the change carries that one dot, and every write-over is of the change stamped so. It is then
**operations**: a write-over (a supersede) of each value the copy holds in a register the change
names, and a write of each edited register. The copy takes every operation in within one
macrostep, every write-over before every write, so its parent is told each register the change
moved once, as it stands after the whole change (ADR 0042, "A register is told once"). A change
that names nothing the copy holds and writes nothing is no change: nothing is stamped, sent or told.

A change that fails any test is refused whole: the clock is where it was, nothing is sent, nothing
is taken in, and every staged command of it is dropped (section 5 tells the parent).

**One dot for several writes is safe under the trust layer.** A write is known by its register and
its dot (an `orset_entry` compares the element and the dot, `mv_value_*` the row, the field and the
dot), so the writes of one change are distinct adds. ADR 0034's host seals a write under the nonce
of its stamp, and that stamp is the write's row, field, wall time, counter and node
(`message::write_stamp`), so no two writes of a change share a nonce; a stamp that "goes back
along a chain" is one strictly earlier than one before it (`chain.rs`), which an equal stamp is
not; and "two writes sealed under one stamp" compares the whole write stamp, register included.

### 2. Emptying a register and retiring a row: observed-remove, and an edit not seen wins

A clear and a retire are write-overs, nothing else. A copy writes over what it holds and nothing it
has not seen: exactly the rule a write over a field has kept since the list began.

**The rule for an edit made at once with a retire, or with a clear, is that the edit wins.** A
write the retiring copy had not seen when it retired the row is not named by any write-over of the
retire, so it stays live on every copy whatever order the two reach it in; the row then holds the
fields such writes gave it and no others, and every parent is told those registers. A write the
retiring copy had seen is written over, and stays written over if it arrives again (its tombstone
names it), however late. This is the observed-remove rule of the set the registers are; a parent
that reads a row with fields an application requires missing decides what it means (an
application may retire such a row again, which is then a change made after seeing the late write).

**What is reclaimed, and when.** On every copy that takes a retire or a clear in, the values and
the adds it wrote over go at once (`values` and `adds` hold only what is live). Its tombstones
stay until a settle passes the dots of the values they name (`record_list.settle`, `floor`), and
then go everywhere, as every tombstone does. The digest's hash under each minute is again what it
was without them. Its nodes are kept: the digest is a trie over the minutes changes were made in,
and a minute whose changes were all written over and settled holds a node of hash 0, which the walk
reads as it reads an absent node. The digest's own growth with the minutes a list is written in is
not a property of rows and is not decided here.

**Why not the other rules.** A retire that wins over every write to its row (a row marker kept,
and every write of the row with an earlier stamp refused) is a last-writer-wins rule by stamp: it
throws away a write a person made without seeing the retire, decides by clocks a member claims,
and needs a marker for every row ever retired, which would itself have to be forgotten by a
settle; a row number never used again needs that marker forever. The edit-wins rule needs no state
the list does not keep already, converges in every order, and gives storage back.

### 3. A change of more than one operation travels as one unit, and is taken in when whole

A change of **one** operation is sent as revision 1 sends it: a `record_list.write` or a
`record_list.supersede` with no new field, which every copy built before this decision reads and
takes in alone. A change of **n > 1** operations is n messages, each a **part**:

- `record_write` revision 2 adds `parts` (since 2, default 1): how many operations the change of
  the write has. Its change is known by the write's own dot, which is the change's stamp.
- `record_supersede` revision 2 adds `parts` (since 2, default 1), `atWallTime` and `atCounter`
  (since 2, default 0): the stamp of the change it is part of, whose node is the `by` it already
  carries.

A copy that is sent a part **holds** it (`pending`, at most `shared_parts` parts of changes not
yet whole) and takes nothing of it in until it holds every part of that change: then it takes them
all in within one macrostep, and its parent is told the change once. The hub passes each part on
to `#people` as it comes (it relays, and holds nothing back from the people) and takes the change
in when it is whole, as every copy does. A part already taken in (one a copy is sent back, or
sent twice) is not held again; one already held is not held twice.

A unit is what a link delivers whole or in part. What repairs a part that never arrives:

- **the walk.** `record_list.resync` takes in every part the copy holds, each alone, before it
  shows its digest, and the walk then shares what the copy lacks one operation at a time, as
  revision 1 shares everything. A change that reached a copy in parts on a link that dropped some
  is therefore taken in whole if it arrives whole, and operation by operation if it has to be
  repaired: a repair restores what a copy holds, not the order it arrived in;
- **a part of a change already partly taken in.** When a copy is sent a part it has already taken
  in, every part it holds of the same change is taken in at once: the change reached it in pieces
  already, and holding the rest back would hold it forever;
- **a settle.** Parts of a change stamped at or before a floor a copy accepts are dropped (every
  copy held the same up to the floor, so what the copy holds of it is held already); a write-over
  part of such a change that arrives later is let go, as a write at or before the floor is.

What this leaves: a change made only of write-overs (a clear or a retire) one operation of which
reached a copy alone by a peer's walk before its other parts arrived waits until that copy's own
next resync.

Every bound of a unit is a build's: a part past `pending` is `error.execution` and is lost on
that copy (and, on the hub, not passed on), which the walk repairs. A deployment sizes
`shared_parts` for the changes its people make at once.

**The operations of one change are the parts of one change on the wire, in a fixed order:** every
write-over (in the order of the copy's `adds`), then every write (in the order of the commands).

**The trust layer.** When ADR 0042 step 3 binds ADR 0034's host to this part, `parts`,
`atWallTime` and `atCounter` are fields a member signs (`message::operation_bytes` reads the
fields of a write and of a supersede by name): a relay that changed them could hold a change back
from being whole, or make one part look like a change of its own.

### 4. What a copy does to itself never waits on what it sends

A send to a peer can fail where it is made (no router: `error.execution`, which ends its block).
So, in every transition of the list, everything the copy does to itself is done or queued before
anything is sent to a peer, and a send to a peer is the last act of its block. The list is built
so that this holds by construction:

- an operation is taken in by `record_list.take` and sent on by `record_list.pass`, two internal
  events with no data, each reading the next operation of its own queue (`taking`, `passing`);
  a change queues every `take` before any `pass`, so the copy holds the change before it is sent;
- `record_list.write` and `record_list.supersede` from a peer judge what they were sent (the hub's
  roster, the text that has to fit) and hand it to `record_list.arrive`, which holds a part or
  takes a single operation in, and on the hub passes it on, last;
- the hub's `record_list.ack` queues its own `record_list.floor` before it tells the people.

A send a copy cannot make is counted in `refusals` and is not retried: a copy whose link was never
attached takes its own changes in, and its first `record_list.resync` on a link shares them. A
copy whose host runs it alone on one device (no router, no hub) is therefore a whole list: it
writes, clears, retires and compacts.

**A scenario can now run a machine with no router.** A scenario's top-level member `router`, when
`false`, makes its driver register neither a Mesh router nor a factory for the machine's children:
every send to a peer is then refused as `error.execution` on every engine, and the scenario states
no `sent` and no `invoke`, which the vocabulary refuses beside it. Absent or `true` is the recording
router every scenario had before. `record_list_offline` is the first: a person's copy and then a
hub, with no router, holding every change it makes.

### 5. A change the list refuses is told to its parent, with a typed reason

A list a statechart started (`announce`, ADR 0042) sends that statechart `record_list.refused`
(`record_refused`) once for each change it refuses: `id`, `field` and `kind` (0 an edit, 1 a clear,
2 a retire) of the command at fault, and `reason`, a value of the enum `record_refusal`:

| Variant | Value | What refused the change |
|---|---|---|
| `notWriter` | 1 | the copy's member may only read (`record_list.role`); counted in `denied` as before |
| `noRoomForValue` | 2 | the values the copy would hold after the change pass the room's `values`; counted in `full` |
| `noRoomForHistory` | 3 | the copy holds as many tombstones as the room's `history` (a settle frees them); counted in `full` |
| `noRoomForDigest` | 4 | the copy holds as many digest nodes as the room's `digest`; counted in `full` |
| `textTooLong` | 5 | an edit's text is past what a person writes (`shared_typed_text`, section 6) |
| `changeTooLarge` | 6 | the change stages more commands than `shared_staged`, or is more operations than `shared_parts` |
| `registerNamedTwice` | 7 | two commands of the change write or clear one register |
| `clockExhausted` | 8 | the copy's clock cannot stamp another change in its millisecond (`hlc_send` overflows) |

The command at fault is the one whose text or staging failed, the second of two that name one
register, or, for what is judged of the whole change, the command that closed it. A change of
several commands is refused once. A refusal is told after the command that ended the change, and
nothing else of the change is told. A list a host started tells no one, as it tells no register.

The enum is a standard document, `sce:std/shared/record_refusal.scxml`, which a parent imports
under the alias the schema writes (`Refusal`), as every holder of a record with an enum field
does (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").

### 6. The bounds, stated

- **`shared_typed_text`, 32 bytes by default**, is the most UTF-8 bytes of text a person writes in
  one `record_list.edit` (`record_edit`'s `text` and the list's `typed`); a longer one is refused
  whole (`textTooLong`). A build raises it with `sce-codegen --limit shared_typed_text=N`, and
  every copy of one list is built with the same N. A write carries and a copy keeps the text in
  `merge_text` bytes (96 by default: the write, the value, the register and value told to a parent,
  `record_take`), so a build that raises the first keeps `merge_text` at least N
  (`--limit merge_text=M`, M ≥ N). A host that keeps content from the hub (ADR 0034,
  `sce-rust-trust`'s `secrecy::seal`) puts the sealed text in its place, which for a text of up to
  C bytes (N rounded up to a multiple of 32) is 1 + ⌈4·(30 + C)/3⌉ characters (84 for 32, 127 for
  64, 169 for 96), so such a deployment keeps `merge_text` at least that.
- **`shared_staged`, 32 by default**: the commands one change stages.
- **`shared_parts`, 64 by default**: the operations one change may have, and the parts of changes
  not yet whole a copy holds (`taking`, `passing` and `pending` are each this long).
- **`shared_moved`, 64 by default** (16 before): the registers one change moves before they are
  told. A change of n operations moves at most n registers, so a build keeps `shared_moved` at
  least `shared_parts`, and no telling runs out of room.

### 7. The protocol, ADR 0042's two copies, and the saved shape

The list's protocol is revision 2 of `record_write` and `record_supersede`, plus the commands. A
revision-1 copy (one built before this decision, and the fixture `todo_peer`, which ADR 0042 step 2
has not yet rebuilt on this part) reads every message of revision 2 (ADR 0044: it walks past the
fields it does not know) and takes each part in alone, and its own messages are revision 2's
single operations. ADR 0042 section 4 forbade a protocol change in one copy alone until step 2;
that rule is amended: the part moves on, and `todo_peer` stays the revision-1 copy it is. The world
test that replays `todo_peer`'s afternoon on the part
(`a_world_run_is_what_every_backend_replays.rs`) maps what changed and holds the rest to one run:
where a to-do copy's edit wrote over values it had seen, the part sends the same messages as one
unit (`parts`, and on each write-over the write's stamp as `atWallTime` and `atCounter`).

**The list's saved shape changes, deliberately, as a new revision of the part.** The shape names
every variable (docs/adr/0045, "The shape of a tree"), and this decision adds the variables of a
change (`staged`, `taking`, `passing`, `pending` and what a change is judged by), drops `fresh` and
`blocked`, and widens `moved`. ADR 0044 cannot make that a revision of one shape: it grows record
schemas, and a saved list holds no record of a schema that grew. It is made now because no saved
state that holds the list can exist yet: ADR 0045 has not landed a format-2 save of a running
child (its steps 3 to 7), so every parent that invokes the list restarts it, and the list is a
pre-release part of ADR 0042, whose step 1 is not verified. A host that saved a list it ran at the
top level with an earlier build is refused at the restore, as any shape change is, and starts its
list again; its peers give the list back by resync. The pin in
`sce-build/tests/fixtures/standard_saved_shapes.json` moves with each commit that moves the shape,
and is from here a promise again.

## Alternatives considered

- **A change of one register stays operation by operation on the wire.** It would have kept every
  message `todo_peer` sends identical, and the world replay untouched. Rejected: another member
  would still be told a register empty and then full whenever a person writes over a value, which is
  the gap this decision closes for rows; a unit has to mean the same for one register as for many.
- **A message that carries the whole change.** An event carries no list (ADR 0042, section 2), and a
  message with a fixed number of field slots bounds a record by the schema of a message, which no
  application could raise without a new protocol. Parts and a count need no such bound.
- **The hub passes a part on only when its change is whole.** Rejected: the hub would hold a
  person's change back from every other person while one part is late, and the people hold parts
  in any case (a part can be lost between the hub and one of them).
- **A host-given room for the size of a change** (a `record_list.room` field). Rejected: a build
  bound that a change past it refuses (`changeTooLarge`), with `shared_moved` at least
  `shared_parts`, makes the telling unable to run out of room, and needs no host to know it.
- **Telling a refusal as a counter the parent reads** (a published variable). A parent cannot read
  its child's variables, and a count says neither which change nor why.
- **A refusal per command of a change.** Rejected: the change is refused whole, so it is told once.
- **Reaching the no-router path only through a child given no router.** The drivers differ in
  whether they give a child a router, so the same scenario would run with one on some lanes and
  without one on others. A top-level member says it once for every driver.

## Consequences

- An application writes a record as one change: every member's parent is told it whole, a
  refusal leaves nothing of it, and the parent is told the refusal and why.
- A register can be emptied and a row retired, and what they held is given back at once and,
  for its tombstones, at the next settle.
- A copy with no router keeps every change it makes; its first link shares them.
- Every operation of a multi-operation change carries one more field (two for a write-over) on the
  wire; a copy holds the parts of changes not yet whole, up to `shared_parts`.
- The person's and the hub's scenarios state the parts of each change they send; four scenarios
  are added (`record_list_offline`, `record_list_change`, `record_list_retire`, and steps of
  `record_list_parent` for the refusals), and every driver replays them.
- The list's saved shape is pinned anew.
