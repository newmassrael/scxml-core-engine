# ADR 0083 — A host records what a change made, and a rebuild never makes it again

- Status: Accepted (2026-11-03). Written at the owner's order of that day, under the rule of 2026-10-26 and
  2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored. The owner approved
  the direction (prepare and effect apart, outcomes recorded, an outbox in the core) before the draft, and accepted
  the draft the same day with the answers recorded in "Owner's answers".
- Date: 2026-11-03
- Amends:
  - ADR 0079 section 7 ("A replay of the application's own log does not write twice") and the status of its step 3,
    whose guarantee rests on a machine sending the same bytes again, which it does not (Context);
  - ADR 0046 sections 1, 4 and 5: a change's outcome is reported to the copy's host, the copy's report comes before
    any send to a peer, and a refusal is told to a host that started the list as well as to a parent.
- Scope:
  - `stdlib/shared/record_list.scxml` and its schemas (`record_edit`, `record_clear`, `record_retire`, `record_join`
    revised; `record_made` and `record_closed` new), the list's scenarios and the seven drivers that replay them;
  - `sce-rust-trust` (`trust.rs`: `seal_sent`, `keep_sent`, `make_operation`, `await_own_chain`), `sce-rust-relay`
    (`client.rs`: `keep`, `Kept`; `relay.rs`: `submit`; `protocol.rs`), `sce-rust-peer` (`Peer`), and
    `sce-rust-peer-capi` (`include/sce_peer.h`, ABI version 6 today);
  - `docs/SCE_SHARED_LIST_PEER_ABI.md` (sections 4, 5, 6, 9), `docs/SCE_SHARED_LIST_RELAY.md` (sections 3 and 6),
    `docs/SCE_SHARED_LIST_TRUST.md` (section 5);
  - the committed generated trees of the list (Rust, Go and Kotlin `static_datamodel`, the Go round-trip tree) and
    every lane that builds it at test time (Python, C++, C11, the Interpreter).
- Related:
  - ADR 0034 (a shared list is held by its members, and the relay only relays; a device that signs two histories is
    frozen on every host that holds both);
  - ADR 0042 (the shared record list is a standard part), ADR 0044 (the revisions of a schema), ADR 0045 (a
    standard statechart that saves is a promise about its shape);
  - ADR 0078 (a consumer follows SCE one landing at a time, and each API change says how);
  - ADR 0079 (one peer core, one C ABI, no I/O in the core), ADR 0080 (a device's relay token).

## Context

A host rebuilds its machine (its application's statechart and the standard shared list,
`stdlib/shared/record_list.scxml`, invoked under it) by replaying its own command log at every process start. The
machine then sends its list's writes again (`record_list.write`, `record_list.supersede`), and the host gives each
to its core (`Peer::outbound`, `backends/rust/peer/src/lib.rs:1229`). ADR 0079 section 7 and `Kept::Held`
(`backends/rust/relay/src/client.rs:2026-2034`, "What a machine built again from its host's own log sends") rely on
those sends being byte-identical, so that the core recognises each as one it signed and signs nothing new
(`Trust::seal_sent`, `backends/rust/trust/src/trust.rs:2074`; the write is looked up by its register, stamp and
content at `trust.rs:2498`, a note by the write it names at `trust.rs:2025`).

They are not byte-identical, and the list does not promise that they are:

- **The clock moves on what the copy has seen.** A change is stamped once, at close, by
  `HlcSend(clock, closingNow, node)` (`record_list.scxml:626`), and the clock is moved by every peer write and
  write-over the copy takes in (`HlcReceive`, `record_list.scxml:728` and `:787`). The document says so on purpose
  (`record_list.scxml:44-45`: "a change made after reading someone else's sorts after it"). `hlc_send` keeps the wall
  time at the greater of `now` and the clock's and advances the counter when they are equal
  (`stdlib/merge/hlc_send.scxml`); `hlc_receive` takes the greatest of three and sets the counter by which of them
  were equal (`stdlib/merge/hlc_receive.scxml:31-38`). So the same command, given the same `now`, gets another
  counter, and sometimes another wall time, whenever the peer operations taken in before it differ.
- **A write-over names what the copy holds.** Closing a change writes over every value the copy holds in each
  register the change names (`record_list.scxml:630-651`, the loop over `adds`), so the same command makes another
  set of write-overs whenever the copy holds other values.
- **At start the replay races with the core.** The core's restore and resync rounds deliver other devices' operations
  to the same machine while the host replays its commands, in an order no one fixes.

Measured on a real device on 2026-11-03, against a relay at `d48f008fe9`: one restart added 182 new operations of
the device. 151 had the register (`id`, `field`) and the wall time of an operation the relay held and differed only
in their counter. The core's `send_kept` log records (`client.rs:2056` and `note_kept`) showed 182 taken and 65
held. The relay's record of the device was then read whole (765 operations of node 1): its numbers run 1 to 765 with
no gap, no number holds two operations, and no two operations share their event, register, wall time and counter.
The "30 operations with equal stamps" the first report cited are the write and the write-over of one change, which
share the change's stamp by design (ADR 0046 section 1). So the one path observed is the replay making another
counter; no conflict happened. The merged state is order-independent, as a CRDT's must be; the emitted operations are
not, and every one of the 182 is a value the other devices now hold beside the first (a register that reads as
changed twice at once) or a write-over that names another set of values.

The world test that holds ADR 0079 section 7
(`a_copy_built_again_from_its_own_commands_writes_nothing_twice`,
`backends/rust/world/tests/a_shared_list_runs_through_a_relay_with_no_hub_machine.rs:362-411`) passes because it
replays alice's four commands into a fresh machine with nothing of bob's taken in between: the one interleaving in
which the generator happens to reproduce its output. It holds the property for that schedule and not for the
property's domain.

A second defect has the same root. It is latent: it is read from the code, and the measurement did not show it. A
process killed after a round gave the relay an operation and before the host saved the core's sealed state comes back
with a core that does not hold that operation. When the core was restored
(`Peer::restore_sealed`, `lib.rs:1448`) it waits for its own chain (`lib.rs:1484`, `trust.rs:601` and the check at
`trust.rs:1934`) and reads the operation back; but a core that was started without a sealed state (a device killed
before its first save, set up again from its seed) begins with `awaiting_own_chain: false` (`trust.rs:249`) and
numbers its next operation where the lost one stood (`trust.rs:1944`). The first report of the measurement
attributed such other bytes to a random nonce, and none is drawn there. In the tree the content nonce is derived from
the list, the epoch and the write's stamp (`backends/rust/trust/src/secrecy.rs:13-15`, `nonce_of` at `:72`) and the
signature is Ed25519's, which is deterministic (`backends/rust/trust/src/identity.rs:3-6`); the randomness drawn is
the sealed state's data key, one for each save (`backends/rust/trust/src/backup.rs:398-401`), which never reaches the
relay. The bytes at one number would differ because what is signed differs: the replayed write's stamp, the epoch
its content is sealed under, and the entry of the log its link names (`Link { seq, prev, log, log_hash }`,
`backends/rust/trust/src/message.rs:254-262`).
Every host that held both would freeze the device (`backends/rust/trust/src/chain.rs:1-20`), and the relay would
keep the second as a new record and answer it `Accepted { new: true }` (`backends/rust/relay/src/relay.rs:921-942`).

Both defects are one mistake: the host's durable original is the input of a computation that reads local state (a
command) instead of the computation's output (what the command made), and the core's durable original of a signed
operation is a snapshot taken after the operation was published instead of a record made before.

## The textbook structure

**Operation-based CRDTs separate prepare from effect.** Shapiro, Preguiça, Baquero and Zawirski define an
operation-based object by an update that runs in two phases ("Conflict-free Replicated Data Types", SSS 2011, LNCS
6976, pp. 386-400; in full in "A comprehensive study of Convergent and Commutative Replicated Data Types", INRIA
Research Report RR-7506, January 2011, section 2.2.2). The *prepare* phase (the generator) runs at the source only, may
read the source's state, has no side effect, and returns the arguments of the *effect* phase (the effector). The
effect phase runs at every replica, the source included, is deterministic, and is applied once for each update under
the delivery the object assumes. In the report's observed-remove set (Specification 15), the add's prepare draws a
unique tag and the remove's prepare collects the tags the source has observed for the element; the effects add the
tag and remove exactly the collected tags. The record list is that set (`record_list.scxml:25-41`): a write is an add
whose tag is its dot, a write-over removes the dots the source held. Baquero, Almeida and Shoker make the point
sharper ("Making Operation-based CRDTs Operation-based", DAIS 2014, LNCS 8460): what travels and what is kept is the
effector, and the generator is never run twice for one update.

**Event sourcing replays events, never commands.** An event-sourced application records the events its command
handlers produced and rebuilds its state by applying them; a replay does not run the command handlers again, because
a handler reads the current state and the clock and would decide otherwise the second time (Fowler, "Event
Sourcing", 2005, the sections on external queries and updates during replay; Young, "CQRS Documents", 2010). The
command is kept for audit; the event is the original.

**A message is recorded before it is sent.** The write-ahead rule of recovery is that the log record of a change is
on stable storage before the change is made visible (Gray and Reuter, *Transaction Processing: Concepts and
Techniques*, 1993, chapter 9; Mohan, Haderle, Lindsay, Pirahesh and Schwarz, "ARIES", ACM TODS 17(1), 1992). Its
messaging form is the transactional outbox: the message is written in the same local transaction as the state change
that caused it, a relaying step sends what the outbox holds, and the receiver is idempotent on the message's identity
(Richardson, *Microservices Patterns*, 2018, chapter 3; Helland, "Life beyond Distributed Transactions", CIDR 2007).

**A clock is an input of prepare.** A hybrid logical clock (Kulkarni, Demirbas, Madappa, Avva and Leone, "Logical
Physical Clocks", OPODIS 2014, LNCS 8878) gives each event a timestamp that respects happened-before (Lamport,
"Time, Clocks, and the Ordering of Events in a Distributed System", CACM 21(7), 1978) and stays near physical time.
It is read when an update is prepared; what the effect needs is the timestamp it produced, not the clock.

Measured against this, the record list already has the two phases inside the machine. `record_list.close`
(`record_list.scxml:556-705`) is the prepare: it reads `adds`, `staged` and `clock` and builds the change's
operations in `taking`. `record_list.take` (`:708-836`) is the effect: one path for a copy's own operations and a
peer's ("This is where every operation is merged, a copy's own and a peer's alike", `:705-707`), idempotent by dot
(an add a tombstone names is not taken in, `:739-749`; a held value is not held twice, `:757`). What is missing is
outside the effect path: what leaves the machine is not marked as a change's effector, the host records the
generator's input, and the core publishes a signed operation before anything durable holds it.

## Decision

**The host's durable original of a change is what the change made: its effectors, recorded with the command in one
transaction. A rebuild applies recorded effectors, the device's own from the host's journal and the others' from
the core's store, through the list's one effect path, and never runs the generator again; nothing is sent while it
does. The core records a signed operation before any exchange gives it to the relay: the host appends the exact
bytes to its journal, and the core withholds them from the relay until the host says they are durable.**

### 1. The standard list reports a change's effector to its host

1. **The effector is what close builds.** A change's effector is its operations in `taking` as close leaves them
   (`record_list.scxml:628-669`): its write-overs, each naming the dot it removes (the causal context of what the
   change writes over), then its writes, each at the change's stamp, which is its dot (ADR 0046 section 1). It is
   the fields of `record_part` (`stdlib/shared/record_part.scxml`) for each operation, with the change's stamp
   (`atWallTime`, `atCounter`) and its size (`parts`) on every part, a one-part change included. Nothing is added to
   what close computes; the decision is that it is reported.
2. **A copy reports it to its host's journal.** `record_join` gains `journal` (revision 2, `sce:since="2"`,
   default `true`): a list a host starts keeps a journal unless it is told not to, and only a test copy that is
   thrown away when its run ends (a scenario, a world test of the Mesh walk) is joined with `journal: false`. A copy
   joined with `journal: true` reports each change it closes on a route of its own, `#journal`, which a host binds
   as it binds `#hub`:
   - `record_list.made` (`record_made`, new): the fields of `record_part`, every one present, and `command` and
     `ordinal` (the part's place in the change, from 1), once for each operation, in the fixed order of ADR 0046
     section 3;
   - `record_list.closed` (`record_closed`, new): `command`, `parts` (0 for a change that was refused or named
     nothing), `reason` (a `record_refusal`; `none` for a change made or a change that named nothing), and the
     change's stamp when it was made. It is sent for every change, refused or not, so a host's record of a command
     always has an outcome, and a list a host started is told its refusals, which today it is not
     (`record_list.scxml:688-703` tells only a parent).
   `command` is the host's identifier of the command that closed the change: `record_edit` (revision 3),
   `record_clear` and `record_retire` (revision 2) gain `command` (`uint64`, default 0), which the copy keeps from the
   closing command and echoes. It is opaque to the list; a host uses its journal's sequence number.
3. **The report comes before any send to a peer.** Close queues every `take`, then every report, then every `pass`
   (`record_list.scxml:678-683` today queues takes then passes). Each report is a send of its own internal event, as
   each pass is (`:844`), so a report that fails where it is made fails alone (ADR 0046 section 4). This is the
   outbox's order inside the machine: the copy holds the change, the host is told it, and only then is anything
   offered to a peer.
4. **A copy whose host carries its changes passes none of its own.** A copy joined with `journal: true` does not pass
   its own changes to `#hub` or `#people`: its host gives the reported effector to its core, which is the delivery
   layer (ADR 0079 section 1). Sending the same effector twice by two routes would make two originals of one fact. A
   hub copy still passes on what it relays (`:1118-1123`), and the digest walk (`record_list.resync`,
   `:1425`; `record_list.share`, `:1531`) is unchanged for a copy that has peers by Mesh routes. A host with no
   core that keeps a copy (a Mesh deployment, a copy on one device) records the report as every host does and
   carries the parts to its peers itself. A copy joined with `journal: false`, a throwaway test copy, behaves
   exactly as every copy does today.
5. **Applying an effector is one path, idempotent by dot.** A host gives a rebuilt copy a recorded effector, its
   own or another device's, as `record_list.recall` (schema `record_part`, every field present). Recall is taken as
   a peer's part is taken by `record_list.arrive` (`:1052-1124`): a part of a change is held until its change is
   whole and then the change is taken in through `record_list.take` at once and told to the parent once. It differs
   from a peer's arrival in two things only, both because what is recalled was already admitted: it is not judged
   against the roster or the text bound again (the core admitted it, or the copy made it), and a hub does not pass it
   on. The source's own effector at close goes through `take` as today. There is no second effect path. The rule
   holds for every copy that is rebuilt, a hub's included: a hub copy (`Peer::for_a_hub`, `lib.rs:681`) is rebuilt
   by recall from its core's history as a person's copy is, and passes nothing on while it is.
6. **The effect path raises the clock to every stamp it takes.** A peer's write and write-over move the clock by
   `HlcReceive` (`:728`, `:787`), and the copy's own write moves it up to the write's dot (`:730-734`). The copy's
   own write-over does not move it (`:783`, `op.by !== node`), which was harmless while the source's clock was always
   already at its stamp, and is not once the effector is applied to a rebuilt copy: a change of write-overs only (a
   clear, a retire) would leave the clock behind its own stamp. The own write-over with a change stamp moves the
   clock up to that stamp, as the own write does.

### 2. The durable original is the command with its outcome, and a rebuild applies what was recorded

1. **One transaction per change.** When a command ends a change (the commands with `more` before it are part of
   it, ADR 0046 section 1), the host runs the machine until it is quiet, collects the reports it was given on
   `#journal`, gives the parts to its core (section 3), and commits in one transaction of its own storage: the
   commands of the change, the effector (every `record_list.made`), the outcome (`record_list.closed`), and the
   signed operations the core made of them (section 3). Before the commit, nothing of the change is shown to the
   person as saved; a crash before it loses the command, as a crash before any acknowledgement does.
2. **A rebuild applies, in causal order, and runs no generator.** At start, the host does not give the machine its
   commands. It:
   1. restores the core (section 3.4);
   2. gives the core the parts of its journal whose operations the journal does not hold yet (an effector made and
      committed whose signing waited, `Kept::Waiting`); each is held or kept again, and none is a new prepare;
   3. asks the core for the list's history: `Peer::history`, every operation the core holds that a round would tell
      the machine (below each chain's frozen position, under the log, sealed content opened with the epoch keys the
      core holds), the device's own included, and the sends that still wait to be signed, preceded by the roster the
      log gives, in the order of each operation's change stamp, then the author's node, then the part's order;
   4. starts a fresh machine, joins it (`journal: true`), gives it the room and the roles, and gives it every
      operation of the history as `record_list.recall`.
   A hybrid logical clock respects happened-before, so ordering by change stamp is a linear extension of the causal
   order: a write-over is recalled after the write it names whenever the source saw that write. The list does not
   depend on it (a tombstone recalled first keeps the add out, `record_list.scxml:739-749`); the parent is told the
   registers once per change in an order that a person could have seen.
3. **Nothing is sent while rebuilding.** Recall passes nothing to a peer (section 1.5) and closes no change, so no
   report is made; the host gives the core nothing it did not hold or keep. The machine's parent is told every
   register the history moves, which is how its own view is rebuilt.
4. **The first round of a process is a sync.** ADR 0079's step 3 left "handing a rebuilt machine the other devices'
   operations from the core alone" undone, so a rebuilt machine was told the list by a resync round from the relay,
   which needs a network at start and races with the replay (Context). `Peer::history` is that hand-over: what the
   core's cursor and its roster told passed (`CourierState`, `docs/SCE_SHARED_LIST_RELAY.md` section 6) is exactly
   what the history gives, so the next round is a sync from the core's cursor, offline starts show what others
   wrote, and a resync is needed only when the core itself lost state (a lapsed list, ADR 0080 section 3) or the
   outcome asks for a rebuild (`Outcome::rebuild`, `lib.rs:207`), which is now answered by the same history and not
   by a resync.
5. **The core and ABI surface.** `Peer::history(&self) -> History`, where `History` is the roster events and the
   operations, each as the event name and JSON payload a round's `events` already carries (`Outcome::events`,
   `lib.rs:161`), with the part's fields complete; in the ABI `sce_peer_history(ScePeer *, ScePeerBuf *events)`, a
   JSON array in the form `sce_peer_take_outcome` writes its events in. It reads no clock and makes no request.
6. **The journal is bounded at the list's settle floor.** A floor the copies accept (`record_list.floor`, ADR 0042)
   is a point of causal stability: every copy holds the same up to it, and none will add to it (Baquero, Almeida
   and Shoker, DAIS 2014, use the same property to compact an operation-based CRDT's log). At a floor the host takes
   a snapshot made of effectors, the live writes stamped at or before the floor (what every copy still holds of
   that time; what was written over there is forgotten by the settle on every copy), records it with the floor in
   one transaction, saves the core's checkpoint, and drops the effectors and records before the floor. A rebuild
   recalls the snapshot, gives the floor, then recalls the history after it; `Peer::history` is bounded the same
   way. This needs no saved state of a running child (ADR 0045) and does not wait for one.

### 3. An outbox in the core

1. **The core assigns, signs and seals; the host records; then the relay is given.** `Peer::made(&mut self, data)`
   takes one `record_list.made` part (a write when `kind` is 0, a write-over when 1) and maps it onto the signed
   event of the list (one place, in Rust, instead of a mapping in every host language), and answers as `outbound`
   answers (`Kept`). A part that is signed now (`Kept::Taken`) produces a **record**: the operation's number, its
   event and its wire text, exactly the bytes a relay would be given. `Peer::take_records()` gives every record not
   yet acknowledged, in order of number; `Peer::recorded(through)` acknowledges every record of the device's chain up
   to `through`. A round gives the relay no operation of the device's own chain past the highest acknowledged number:
   the step that gives the relay what it lacks (`docs/SCE_SHARED_LIST_RELAY.md` section 6, step 2) stops there. A
   send that waited and is signed inside a round (`Trust::sign_waiting`, `trust.rs:2426`) is a record as well: the
   round makes it, offers it (`Peer::take_records`), and gives it to the relay only after `recorded`; a round driven
   by the library (`sce_peer_run`) calls a function the host gives (`sce_peer_set_journal`, beside the log sink of
   ADR 0079's last landing) and goes on when it returns 0. The host appends records to its journal, in the
   transaction of section 2.1 when the part was signed at once, and in a transaction of its own when a round signed
   it.
2. **Where the durable record lives: the host's journal, with the sealed state as its checkpoint.** Two places
   could hold it. The core's sealed state (`Peer::save_sealed`, ADR 0079 step 7) holds the device's whole state and
   its secrets; saving it before each round that carries a new operation is a snapshot per write, of a size that
   grows with the list, in a second store whose commit is not the journal's, so a crash between the two commits
   leaves the effector in one and the operation in the other. The host's journal is append-only, already holds the
   effector, and is committed once per change. The write-ahead rule asks for an append before the effect is made
   visible and a checkpoint from time to time, not a full image before each effect (Gray and Reuter, chapter 9;
   ARIES). So the record lives in the journal, in the same transaction as the effector it signs, and the sealed state
   is the checkpoint: the host saves it when it likes (after a round, on going to the background), and a record
   past the checkpoint is replayed into the core at restore. A record holds no secret: it is what the relay holds,
   its content sealed when the list keeps content from the hub (ADR 0034), so the journal needs no wrapping by the
   platform's key store; the secrets stay in the sealed state.
3. **Recorded bytes, not a deterministic re-derivation.** The content is already sealed under a nonce derived from
   the stamp (`secrecy.rs:72`), deterministic authenticated encryption in the sense of Rogaway and Shrimpton
   ("A Provable-Security Treatment of the Key-Wrap Problem", EUROCRYPT 2006; the SIV construction of RFC 5297), and
   Ed25519 signatures are deterministic (RFC 8032). Re-deriving the same operation after a crash is therefore
   possible only if every input is the same: the effector (which section 2 now fixes), and the number, the
   operation before it, the entry of the log and the epoch the core had reached when it signed, which are not
   functions of the effector and would have to be recorded anyway. A record of the bytes is that and no more, can be
   checked against the device's key on restore, and survives a change of the log or the epoch between the crash
   and the restart, where a re-derivation would sign other bytes at the same number. Keeping the derived content
   nonce stays right for its own reason: one write is sealed one way, and a host that sends a recorded write again
   sends the same bytes.
4. **Restore.** `Peer::restore_sealed` (the checkpoint), then `Peer::restore_records(records)` with every record of
   the journal past the checkpoint's highest own number: each is verified (signature, link to the one before, the
   log entry it names) and taken in as the device's own, so the core holds its chain from the device's own storage
   before it signs anything. The wait for the device's own chain (ADR 0079 step 3) stays, as defence in depth for a
   journal that is itself older than what was published (an application's data restored from a backup), and it
   becomes the core's rule for every start that sets an identity: `Peer::set_identity` waits as `restore_sealed`
   does (`lib.rs:1484`), so that a device set up again from its seed with no state cannot number an operation where
   a lost one stands. A device whose relay holds nothing of it ends the wait at its first round's heads, as today.
5. **What `Kept::Held` becomes.** The receiver's idempotence in the outbox pattern: a part given again (a rebuild's
   step 2.2, a host that crashed between its commit and its `recorded`) is the operation the core holds, as ADR 0079
   section 7 says. It is no longer what makes a replay safe; recording outcomes is.

### 4. The relay's defence in depth

1. **A second operation of one author at one number with other bytes is refused as an operation and kept as
   proof.** Today the relay takes it as new (`relay.rs:928-932`), and the submitter learns nothing until a later
   round shows its chain frozen. The relay's verdict becomes a refusal with a code of its own, `conflict`, naming the
   number and giving the operation the relay holds there, while the relay still keeps the second record: the freeze
   of ADR 0034 is a function of the set of operations every host holds (`chain.rs:7-20`), the relay may not be the
   one place that hides an equivocation from the other members, and a member's courier that gives the relay a
   conflicting operation it holds of another device (the proof a courier carries, `docs/SCE_SHARED_LIST_RELAY.md`
   section 6) must have it kept. The rule is the same whoever submits it, the author's own session included: the
   relay never chooses a history. What changes is that no relay answers `accepted` to an operation that freezes its
   author. `Chain::insert` already says whether an operation moved the frozen position (`Inserted { before, after }`,
   `chain.rs:112-120`); `Trust::take_operation` gains a field of `Taken` (`trust.rs:1575`) that carries it, and the
   relay answers by it.
2. **The author's core acts on it at once.** A core given `conflict` for its own operation takes the held operation
   in (its own, signed by its key), finds its chain frozen as every host will, signs nothing more, ends the round,
   and its outcome says so (`Outcome::conflicted`, the number), so that the host tells its person that an
   administrator has to take the device off and grant a new one. After section 3, an honest device reaches this only
   through a defect or a key used elsewhere; both are what the freeze is for.
3. **Two notes of one author naming one write are not a conflict.** A note names the write it removes, and the core
   signs one note per write of each author already (`trust.rs:2025`). A second one from one author is possible only
   from a device that lost its first (the defect this ADR removes), its effect is idempotent (the tombstone is held
   once, `record_list.scxml:794-802`), and it is a link of its author's chain: refusing it would leave a gap at its
   number that no later operation of the device could follow, and freezing for it would take off a device that
   signed two true statements. It is admitted and harmless; a core built on this ADR does not make it.
4. **The protocol's version.** The new answer is a response a version-1 client does not read, so the relay's protocol
   is version 2 (`protocol.rs:19`); a relay answers a version-1 request as it does today for as long as a deployment
   keeps version-1 cores.

### 5. The clock stays where it is, and recording outcomes reconciles it with replay

The hybrid logical clock is right for prepare and is kept: a change is stamped after everything its copy has seen
(happened-before is respected) and near physical time (so a settle's instant and a person's reading of "when" mean
something), and the observed-remove rule needs exactly that. Its non-determinism under reordering (Context) is a
property of the generator, and only a generator run twice can expose it. Once the effector is recorded, the stamp
is data: the rebuild takes it from the record. What the clock must still guarantee after a rebuild is that the next
change is stamped after every stamp the copy holds, its own included (no dot is used twice, and a change made after
reading a write sorts after it). Both are monotone: the clock is raised to the greatest stamp taken, by maximum for
the copy's own (section 1.6) and by `HlcReceive` for a peer's, and the greatest of a set does not depend on the
order it is read in. The clock's counter after a rebuild may differ from the one before; no record depends on it.

### 6. Migration

1. **The list.** `record_edit` revision 3, `record_clear` and `record_retire` revision 2 (`command`), `record_join`
   revision 2 (`journal`), `record_made` and `record_closed` new, `record_list.recall` new, the own write-over's clock
   (section 1.6). Every revision is a field with a default (ADR 0044), so a copy built before this ADR reads every
   message of it. `journal` defaults to `true` (section 1.2), so every existing scenario and world test whose copy
   is thrown away and passes its own changes on Mesh routes joins with `journal: false`, said in the commit that
   moves it, and every other copy keeps a journal. The list's saved shape gains
   `journal` and the command it echoes; the pin in `sce-build/tests/fixtures/standard_saved_shapes.json` moves with
   the commit that moves the shape (ADR 0045). Every generated backend of the list moves with it: the committed Rust,
   Go and Kotlin `static_datamodel` trees (`record_list_sm`, `record_list_parent_sm` and its synthesised child) are
   regenerated by `scripts/regen_static_datamodel_{rust,go,kotlin}.sh` and the Go round-trip tree by
   `round_trip/generate.sh`; Python, C++, C11 and the Interpreter build the list at test time; Swift gains it when the
   list is among its trees (it is not today). The C11 internal queue holds the reports too: a change of n operations
   queues n reports and one close beside its takes and passes, so ADR 0046 section 6's bound becomes about five
   events for each operation, and `SCE_MAX_EVENTS` at least five times `shared_parts`.
2. **The trust layer.** The signed events stay `record_list.write` and `record_list.supersede`, whose fields already
   include `parts`, `atWallTime` and `atCounter`; a write-over of a one-part change now carries its stamp, which is
   signed, so the bytes of a one-part write-over differ from the same one made before this ADR. That is a new domain
   version of the signed bytes (`sce-trust/event/3`), as ADR 0079 section 2 made one for the names; recorded fixtures
   signed under version 2 are recorded again.
3. **The core and the ABI.** `Peer::made`, `take_records`, `recorded`, `restore_records`, `history`, the wait on
   `set_identity`, the round that withholds what is not recorded, and `Outcome::conflicted`. The ABI is version 7: a
   host of version 6 never acknowledges a record, and its core would give the relay nothing, which the version policy
   (`docs/SCE_SHARED_LIST_PEER_ABI.md` section 11) makes a new number. New functions: `sce_peer_made`,
   `sce_peer_take_records`, `sce_peer_recorded`, `sce_peer_restore_records`, `sce_peer_history`,
   `sce_peer_set_journal`; the Kotlin binding (`backends/kotlin/peer`) gains each. The recorded scenario
   (`tests/fixtures/two_devices_and_a_relay.json`) is recorded again with a restart, a history and a record
   acknowledged, and every lane replays it (C, C++, Python, Go, Kotlin, Swift).
4. **ADR 0079 section 7** is amended to: "A rebuild applies recorded effectors and signs nothing new; a part given
   again is the operation the core holds (`Kept::Held`)". Its step 3 status keeps its history and gains a line that
   the guarantee it stated held for one schedule only (Context). **ADR 0046** sections 1, 4 and 5 are amended as this
   ADR's section 1 says.
5. **What a host changes**, said in each landing as ADR 0078 requires:
   - its journal holds, per change, the commands, the `record_list.made` parts, the `record_list.closed` outcome and
     the core's records, in one transaction; the commands stay for audit and are not replayed;
   - it joins the list with `journal: true`, binds `#journal`, gives each part to `Peer::made` and not to
     `Peer::outbound`, and acknowledges records after its commit;
   - its start is: restore the checkpoint and the records, give the core the parts not yet signed, read the history,
     recall it into a fresh machine, then sync;
   - a host whose journal holds only commands, from before this ADR, converts once: its core already holds every
     operation the device published or keeps waiting, so the first start under ABI 7 rebuilds from
     `Peer::history`, and from then on the host records outcomes. A command that never became an operation (refused,
     or lost in the crash before its commit) has nothing to convert.

### 7. Verification

Every test below states the property, runs on every lane that runs the list or the core, and is written so that the
defect of the Context turns it red; each new guard also gets a mutation casefile under `sce-build/tests/mutations`.

1. **The cross product, in the world.** For each kind of change (an edit, an edit over a held value, a clear, a
   retire, a change of several parts, a refused change) and each point at which another device's operations reach the
   rebuilt machine (none, all before, interleaved after each recalled change, all after), and with the remote stamps
   ahead of, equal to and behind the device's wall time: the history recalled into a fresh machine leaves `values`,
   `adds`, `tombstones` and `digest` equal to the machine's before the restart, the clock at or past every stamp it
   holds, and the core's chain and the relay's records unchanged. The same matrix replayed with commands instead of
   effectors must fail, which is checked once before the test is trusted.
2. **A device restarted N times publishes nothing.** A device makes a hundred changes among two others' writes, then
   is restarted twenty times, each start racing a round that delivers the others' new writes; after each start the
   relay's record count and the device's chain length are what they were, and every device's registers read with one
   value where one was written (no register changed twice at once by a replay).
3. **An offline start shows what others wrote.** With no transport at all, a restarted device's machine holds every
   value its core held, its parent is told each register once, and the first round after the network returns is a
   sync that tells it only what is new.
4. **A kill after a push conflicts with no one.** The path is latent (read from the code, not observed: Context), so
   the test holds the paths the code names. The process is killed at each step boundary (after `made`, after
   the commit, after `recorded`, after the relay's `accepted`, before the checkpoint), for a restored device and for
   a device that never saved; after the restart and a round, `Trust::equivocating` is empty on every host and on the
   relay, and the relay holds each operation once. A hub copy is restarted the same way (test 2) and passes nothing
   on while it is rebuilt.
5. **The relay's answer.** A second operation at one number with other bytes is answered `conflict` with the held
   one, is kept, and freezes the author on every host at the same number, from the author's own session as from a
   member's courier; a second note of one author naming one
   write is admitted and changes no register.
6. **The list on every lane.** New scenarios `record_list_journal` (the reports' order and fields for each kind of
   change, `closed` for a refused change of a list a host started, and no pass of its own for a copy with
   `journal: true`) and `record_list_recall` (a history of own and others' changes recalled, nothing sent, each
   register told once, the clock past every stamp) replay on Rust, Kotlin, Python, Go, C++, C11 and the
   Interpreter, and every existing `record_list_*` scenario replays unchanged with `journal: false`. A scenario
   joined without `journal` holds the default: its copy reports on `#journal` and passes nothing of its own.
7. **The ABI.** The recorded scenario of section 6.3, replayed from every lane with a toolchain; the Kotlin device of
   `a_shared_list_is_held_through_the_core_on_every_device` restarted through the ABI as in test 2.

## Landing order

The owner's order of 2026-11-03. Each landing carries the tests of section 7 that hold it, and says how a consumer
follows it (ADR 0078).

1. **The relay.** Protocol version 2 and its `conflict` answer (section 4), with the field of `Taken` that says an
   operation froze its author; a relay of version 2 answers a version-1 request as today, so it deploys before any
   core moves.
2. **The standard list.** `journal` (default `true`), the `#journal` reports, `record_list.recall`, the own
   write-over's clock (section 1), the schema revisions, the saved-shape pin, the scenarios on seven lanes with the
   throwaway copies joined `journal: false`, and every generated backend of the list regenerated in the same landing
   (the Rust, Go and Kotlin `static_datamodel` trees and the Go round-trip tree; Python, C++, C11 and the
   Interpreter at test time).
3. **The core.** The outbox and the history (sections 2 and 3), the journal's bound at the settle floor (section
   2.6), the wait on `set_identity`, the trust layer's domain version 3, ABI version 7 with the Kotlin binding, the
   recorded scenario recorded again, and the world tests of section 7.
4. **The hosts.** Each host in the tree moves in a landing of its own (section 6.5); a host that pins SCE takes
   landings 2 and 3 together, since a copy that keeps a journal by default needs a core that takes its reports.

## Consequences

- A restart publishes nothing; a register reads as changed twice at once only when two people changed it at once.
- A device's chain no longer depends on when its host saved: a crash anywhere after the commit conflicts with no one.
- A rebuilt machine is told the list by its own core, offline, and its process's first round is a sync.
- A host's storage holds a journal of outcomes beside the checkpoint, and every change is one transaction of it.
- The list gains one route (`#journal`), two reports, one recall input and five schema revisions; every copy keeps a
  journal by default, and only a throwaway test copy is the list it is today.
- Every copy that is rebuilt, a hub's included, is rebuilt by recall, and the journal is bounded at the settle floor.
- The ABI is version 7, the relay's protocol version 2, the signed bytes' domain version 3; recorded fixtures are
  recorded again.

## Alternatives rejected

- **Make the generator deterministic** (stamp a change by its command's `now` alone and the source's own previous
  stamp, and write over only what the command names). Rejected: the stamp would no longer order a change after what
  its copy has seen, which the observed-remove rule and a settle's instant need (section 5), and a write-over that
  does not name what its source observed is not the OR-set's remove. It also moves the fault, not the root: any
  generator that reads state can be re-run into another answer.
- **Fix the start order** (the host replays all its commands before the core delivers anything). Rejected: it makes
  the outcome depend on a schedule every host must keep, which is the property this ADR removes; a host that is
  killed mid-replay, or receives an operation during it from a parent's event, breaks it again. The world test that
  holds today's rule shows exactly this kind of schedule-bound guarantee (Context).
- **Recognise a replayed write by its register and wall time** (widen `Kept::Held` to ignore the counter).
  Rejected: two real changes of one register in one millisecond would be merged into one, losing a write, and a
  write-over set that differs cannot be matched to anything.
- **The core's sealed state as the only durable record** (save before every round that carries a new operation).
  Rejected in section 3.2: a full image per write, in a store whose commit is not the journal's.
- **Re-derive the operation deterministically instead of recording it.** Rejected in section 3.3: the log entry,
  the epoch and the number are inputs that are not the effector's.
- **The relay refuses a conflicting operation and does not keep it.** It would let an honest device that lost its
  operation recover without being frozen. Rejected: after section 3 an honest device does not reach it, a member's
  courier that carries another device's conflict must have it kept, and a relay that drops one of two operations
  decides which history is the device's, which ADR 0034 does not let a relay decide. The narrow case of the author's
  own session is refused and kept as well (Owner's answers, 2).
- **Bound the journal by a snapshot of the machine's saved state** (ADR 0045's save of a running child). Rejected for
  now: it waits on another ADR, and a snapshot made of the live effectors at the settle floor needs nothing the list
  does not already agree on (section 2.6).
- **Report the effector on `#hub` and let the host tell it from relayed traffic and walk shares by context.**
  Rejected: it makes the outcome of a command a matter of what else was sent in the same macrostep, which the next
  change to the list could break silently; a route and a closing report make it a property of the document.

## Owner's answers

The draft asked five questions. The owner accepted the ADR on 2026-11-03 and answered them as follows; the sections
named are written to the answers.

1. **What the relay holds of the measured device.** The relay's record of node 1 was read whole: 765 operations,
   numbered 1 to 765 with no gap, no number holding two operations, and no two operations with equal event,
   register, wall time and counter. The "30 equal stamps" of the first report were a write and a write-over of one
   change, which share the change's stamp by design. No conflict happened. The one observed path is the replay
   making another counter; the kill-after-push path is latent, read from the code and not observed. The Context
   says so, and test 7.4 holds the paths the code names.
2. **A conflicting operation from the author's own session** is refused with `conflict` and still kept as proof,
   as from any other session: ADR 0034 stays consistent, and the relay never chooses a history (section 4.1).
3. **Hub copies are in scope.** Every copy that is rebuilt follows the rule, a hub's included (section 1.5).
4. **`journal: true` is the default** for a list a host starts; only a throwaway test copy may turn it off
   (sections 1.2, 1.4 and 6.1).
5. **Compaction bounds the journal at the list's settle floor**: a floor is a point of causal stability, the host
   keeps a snapshot at the floor and drops the records before it, and this does not wait for ADR 0045
   (section 2.6).

No question is left open.
