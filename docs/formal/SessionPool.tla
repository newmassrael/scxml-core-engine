---- MODULE SessionPool ----
(***************************************************************************)
(* Who holds which lock of a pool of sessions, and when (docs/adr/0115,    *)
(* D13; docs/adr/0107, D3; docs/adr/0109, D1 and D10). What each lock      *)
(* guards is decided by a core proved by Verus (backends/rust/mesh/src/    *)
(* pool/slot.rs); this module is what is left of the pool: the table of    *)
(* sessions by key, each session's slot with its inbox and its live lock,  *)
(* the threads that post, run, evict and stop, and the order they take     *)
(* the locks in.                                                           *)
(*                                                                         *)
(* An envelope is posted for a key (Pool::try_post): with the table held   *)
(* it is refused when the pool stops, put in the inbox of the slot the     *)
(* table holds for its key below max_inbox, or given a new slot below      *)
(* max_live. The host runs the key a post named (Pool::run): a runner      *)
(* reads the slot from the table, takes the slot's live lock, and with it  *)
(* held loads the session, applies what the inbox holds one input at a     *)
(* time, or takes the slot out of the table when its inbox holds nothing.  *)
(* A load the store refuses (another process holds the key) takes the slot *)
(* out and answers what its inbox held (Pool::abandon). The evictor takes  *)
(* the live lock of a session that is idle and holds nothing, saves it,    *)
(* and takes it out of the table only if its inbox still holds nothing     *)
(* (Pool::evict). A stop (Pool::stop) refuses every admit from then on,    *)
(* waits until the inboxes are drained, and saves and takes out every      *)
(* session in memory.                                                      *)
(*                                                                         *)
(* The pool follows a journal (docs/adr/0114, D4; Pool::follow). Its       *)
(* follower hands each entry to the session of its key with the table      *)
(* held, as a post is made (Pool::post_followed): an entry waits in the    *)
(* inbox and counts toward max_inbox as an envelope does                   *)
(* (Input::waits_as_an_envelope). It hands the entries of a key in the     *)
(* order of their positions: an entry that meets a bound, or that was lost *)
(* when its slot left the table with it, holds every later entry of its    *)
(* key, and a later entry follows an earlier one that is out only into the *)
(* slot the table holds for the key now. The follower reads what became of *)
(* an entry it handed at the start of a call, so an entry its slot lost is *)
(* out in that slot until it reads it. A session takes in an entry above   *)
(* the position it took in of the entry's key, and drops one at or below   *)
(* it as a repeat.                                                         *)
(*                                                                         *)
(* A tell (Pool::tell_holders; Pool::end_members tells EndMember the same  *)
(* way) puts an input the pool makes in the inbox of each session the      *)
(* table holds, with the table held, and no bound counts it.               *)
(*                                                                         *)
(* The parts, each a constant that is TRUE in the protocol, and each taken *)
(* out by a run that must find what it breaks:                             *)
(*   PostHoldsTable  a post holds the table from the lookup of its slot to *)
(*                   the write into its inbox, as a removal does           *)
(*   RemoveIfEmpty   an eviction takes a slot out only if its inbox still  *)
(*                   holds nothing once the session is saved               *)
(*   LiveLock        one thread at a time holds a slot's live lock         *)
(*   BoundInbox      an inbox at max_inbox refuses an envelope             *)
(*   BoundLive       a table at max_live refuses a new session             *)
(*   StopHoldsTable  the stop is set with the table held and read with it  *)
(*                   held by every admit                                   *)
(*   BoundFollowed   a followed entry counts toward max_inbox as an        *)
(*                   envelope does                                         *)
(*   FollowInOrder   the follower hands an entry out only once every       *)
(*                   earlier entry of its key was taken in or is out       *)
(*   FollowSameSlot  an earlier entry that is out lets a later one follow  *)
(*                   only into the slot it went to, judged with the table  *)
(*                   held                                                  *)
(*   TellHoldsTable  a tell holds the table from the lookup of the slots   *)
(*                   to the write into their inboxes                       *)
(*                                                                         *)
(* lin, applied and took are history variables: the order each key's       *)
(* envelopes were taken in, the order they were applied in, and the        *)
(* followed entries each key's session took in, in that order. No pool     *)
(* keeps them.                                                             *)
(*                                                                         *)
(* Left out, each for its reason: the lease's lock, whose every access is  *)
(* one step of the proved KeyLease (the lease across processes is          *)
(* CheckpointWrite's); the wake of a session that comes due (Pool::due),   *)
(* an admit through another door under the same table and the same stop;   *)
(* the other inputs the pool makes (a link's readiness, a tick, a note),   *)
(* each put in an inbox under the table as a tell is; the eviction a post  *)
(* or a hand-out makes room by, which is the evictor's on the posting      *)
(* thread; of the follower, its page, its window and the mark it keeps in  *)
(* the store, which are EventJournal's (Follow with CursorAtomic), an      *)
(* entry of a key another process holds, which is that process's to hand,  *)
(* and an entry that names no session; a durable checkpoint apart from the *)
(* save of an eviction or a stop, so the receipt of an entry is settled    *)
(* when its session applies it; and the stop as a tell reads it, apart     *)
(* from the table: a tell is put whatever the stop, as one that read it    *)
(* just before it was set is.                                              *)
(*                                                                         *)
(* Taken as given: a mutex is mutual exclusion; what a thread does with a  *)
(* lock held is one step; an inbox's queue and its door move together     *)
(* (pool/slot.rs); the stop flag is read and written sequentially          *)
(* consistently (an atomic of Ordering::SeqCst, and under the table only   *)
(* its holders decide by it).                                              *)
(***************************************************************************)
EXTENDS Sequences, Naturals, FiniteSets

CONSTANTS
    \* the keys of two sessions, and the envelopes posted naming each
    \* @type: Str;
    First,
    \* @type: Str;
    Second,
    \* @type: Set(Str);
    FirstEnvelopes,
    \* @type: Set(Str);
    SecondEnvelopes,
    \* the threads the host runs sessions on
    \* @type: Set(Str);
    Runners,
    \* the most slots the model makes: a session that left the table is given a new slot when its
    \* key is posted to again
    \* @type: Int;
    MaxSlots,
    \* sessions.max_live and sessions.max_inbox
    \* @type: Int;
    MaxLive,
    \* @type: Int;
    MaxInbox,
    \* the entries of the followed journal in the order of their positions, and those of them that
    \* name the key First; the others name Second
    \* @type: Seq(Str);
    Journal,
    \* @type: Set(Str);
    FirstEntries,
    \* the tells, each one call that tells the sessions the table holds
    \* @type: Set(Str);
    Tells,
    \* the parts
    \* @type: Bool;
    PostHoldsTable,
    \* @type: Bool;
    RemoveIfEmpty,
    \* @type: Bool;
    LiveLock,
    \* @type: Bool;
    BoundInbox,
    \* @type: Bool;
    BoundLive,
    \* @type: Bool;
    StopHoldsTable,
    \* @type: Bool;
    BoundFollowed,
    \* @type: Bool;
    FollowInOrder,
    \* @type: Bool;
    FollowSameSlot,
    \* @type: Bool;
    TellHoldsTable

Keys == {First, Second}
Envelopes == FirstEnvelopes \cup SecondEnvelopes
Entries == {Journal[i] : i \in DOMAIN Journal}
\* The inputs whose fate status follows: the envelopes, and the entries of the journal.
Inputs == Envelopes \cup Entries
Slots == 1..MaxSlots

\* The key an envelope or an entry names.
\* @type: (Str) => Str;
KeyOf(e) == IF e \in FirstEnvelopes \cup FirstEntries THEN First ELSE Second

\* The position of the entry e in the journal.
\* @type: (Str) => Int;
PositionOf(e) == CHOOSE i \in DOMAIN Journal : Journal[i] = e

\* The entries of the key of e that come before it in the journal.
\* @type: (Str) => Set(Str);
EarlierOf(e) == {d \in Entries : KeyOf(d) = KeyOf(e) /\ PositionOf(d) < PositionOf(e)}

\* Whether the input x counts toward max_inbox (Input::waits_as_an_envelope): an envelope does,
\* and an entry with BoundFollowed. A tell never does.
\* @type: (Str) => Bool;
WaitsAsAnEnvelope(x) == x \in Envelopes \/ (BoundFollowed /\ x \in Entries)

\* How many inputs of the inbox q count toward max_inbox (the envelopes of its Door).
\* @type: (Seq(Str)) => Int;
Waiting(q) == Cardinality({i \in DOMAIN q : WaitsAsAnEnvelope(q[i])})

\* The journals the configurations give Journal: none, and two entries.
\* @type: () => Seq(Str);
NoEntries == <<>>
\* @type: () => Seq(Str);
TwoEntries == <<"j1", "j2">>

VARIABLES
    \* the slot the table holds for each key, 0 for none
    \* @type: Str -> Int;
    table,
    \* how many slots were made: a slot is one of 1..made
    \* @type: Int;
    made,
    \* the key of each slot made
    \* @type: Int -> Str;
    slotKey,
    \* what each slot's inbox holds, in the order it took it
    \* @type: Int -> Seq(Str);
    inbox,
    \* whether each slot's inbox is closed, which it is once the slot left the table
    \* @type: Int -> Bool;
    closed,
    \* whether each slot's session is in memory
    \* @type: Int -> Bool;
    loaded,
    \* the runners that hold each slot's live lock
    \* @type: Int -> Set(Str);
    holders,
    \* where each runner stands: idle; got, a slot read from the table; holds, its live lock; or
    \* applying, an input taken from its inbox
    \* @type: Str -> Str;
    rpc,
    \* @type: Str -> Int;
    rslot,
    \* @type: Str -> Str;
    rinput,
    \* the slot whose live lock the evictor holds while it saves the session, 0 for none
    \* @type: Int;
    evicting,
    \* what became of each envelope and entry: none; checked, the stop read apart from the table;
    \* looked, its slot looked up and the table let go; taken; applied; answered, an envelope its
    \* slot left the table with; lost, an entry its slot left the table with; or refused for a
    \* reason (stopping, full, live)
    \* @type: Str -> Str;
    status,
    \* the slot a post that let the table go writes into
    \* @type: Str -> Int;
    target,
    \* whether a stop was decided, and where it stands: running, draining, saving or stopped
    \* @type: Bool;
    stopping,
    \* @type: Str;
    stage,
    \* where each entry stands for the follower: no, to be handed out; out, handed out and not yet
    \* read taken in or lost; or in, read taken in
    \* @type: Str -> Str;
    handed,
    \* the slot each entry out was put in
    \* @type: Str -> Int;
    handedTo,
    \* whether the follower read the stop apart from the table for its next hand-out (checked) or
    \* not (idle)
    \* @type: Str;
    follower,
    \* the position of the last entry each key's session took in, which its checkpoint keeps
    \* @type: Str -> Int;
    mark,
    \* where each tell stands: none; looked, its slots looked up and the table let go; or told
    \* @type: Str -> Str;
    told,
    \* the slots a tell that let the table go writes into
    \* @type: Str -> Set(Int);
    tellTo,
    \* the history variables
    \* @type: Str -> Seq(Str);
    lin,
    \* @type: Str -> Seq(Str);
    applied,
    \* @type: Str -> Seq(Str);
    took

vars == <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput, evicting,
          status, target, stopping, stage, handed, handedTo, follower, mark, told, tellTo, lin,
          applied, took>>

\* What the follower and the tells move.
followVars == <<handed, handedTo, follower, mark, took>>
tellVars == <<told, tellTo>>

\* @type: (Str, Seq(Str)) => Bool;
InSeq(x, q) == \E k \in DOMAIN q : q[k] = x

\* Whether the table holds slot s for its key.
\* @type: (Int) => Bool;
InTable(s) == table[slotKey[s]] = s

\* The table with slot s taken out, if it is still the slot the table holds for its key.
\* @type: (Int) => (Str -> Int);
Remove(s) == IF InTable(s) THEN [table EXCEPT ![slotKey[s]] = 0] ELSE table

\* The keys the table holds a session for.
Held == {k \in Keys : table[k] # 0}

\* The slots the table holds. Quantified over directly, so that no slot is read through a key
\* the table holds none for: TLC takes each disjunct of an action as a branch of its own, and
\* would read inbox[0] in one.
HeldSlots == {table[k] : k \in Held}

Init ==
    /\ table = [k \in Keys |-> 0]
    /\ made = 0
    /\ slotKey = [s \in Slots |-> First]
    /\ inbox = [s \in Slots |-> <<>>]
    /\ closed = [s \in Slots |-> FALSE]
    /\ loaded = [s \in Slots |-> FALSE]
    /\ holders = [s \in Slots |-> {}]
    /\ rpc = [r \in Runners |-> "idle"]
    /\ rslot = [r \in Runners |-> 0]
    /\ rinput = [r \in Runners |-> ""]
    /\ evicting = 0
    /\ status = [e \in Inputs |-> "none"]
    /\ target = [e \in Envelopes |-> 0]
    /\ stopping = FALSE
    /\ stage = "running"
    /\ handed = [e \in Entries |-> "no"]
    /\ handedTo = [e \in Entries |-> 0]
    /\ follower = "idle"
    /\ mark = [k \in Keys |-> 0]
    /\ told = [x \in Tells |-> "none"]
    /\ tellTo = [x \in Tells |-> {}]
    /\ lin = [k \in Keys |-> <<>>]
    /\ applied = [k \in Keys |-> <<>>]
    /\ took = [k \in Keys |-> <<>>]

-----------------------------------------------------------------------------
\* A post of the envelope e, each step with the table held.

\* @type: (Str, Str) => Bool;
Refuse(e, why) ==
    /\ status' = [status EXCEPT ![e] = why]
    /\ UNCHANGED <<table, made, slotKey, inbox, target, lin>>

\* e enters the inbox of slot s.
\* @type: (Str, Int) => Bool;
TakeInto(e, s) ==
    /\ inbox' = [inbox EXCEPT ![s] = Append(@, e)]
    /\ status' = [status EXCEPT ![e] = "taken"]
    /\ lin' = IF e \in Envelopes THEN [lin EXCEPT ![slotKey[s]] = Append(@, e)] ELSE lin
    /\ UNCHANGED <<table, made, slotKey, target>>

\* e is given a new slot, which the table holds for its key.
\* @type: (Str) => Bool;
NewSlot(e) ==
    LET k == KeyOf(e)
        n == made + 1
    IN
    /\ made < MaxSlots
    /\ made' = n
    /\ table' = [table EXCEPT ![k] = n]
    /\ slotKey' = [slotKey EXCEPT ![n] = k]
    /\ inbox' = [inbox EXCEPT ![n] = <<e>>]
    /\ status' = [status EXCEPT ![e] = "taken"]
    /\ lin' = IF e \in Envelopes THEN [lin EXCEPT ![k] = Append(@, e)] ELSE lin
    /\ UNCHANGED target

\* Without StopHoldsTable, the poster reads the stop before it takes the table.
\* @type: (Str) => Bool;
ReadStop(e) ==
    /\ ~StopHoldsTable
    /\ status[e] = "none"
    /\ status' = [status EXCEPT ![e] = IF stopping THEN "stopping" ELSE "checked"]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, target, stopping, stage, followVars, tellVars, lin, applied>>

\* The admit, with the table held (slot::admission): the stop, then the inbox of the session the table
\* holds for the key, or a new session. Without PostHoldsTable the post lets the table go once it
\* has looked its slot up, and writes into it later (Write).
\* @type: (Str) => Bool;
Admit(e) ==
    LET s == table[KeyOf(e)] IN
    /\ status[e] = (IF StopHoldsTable THEN "none" ELSE "checked")
    /\ IF StopHoldsTable /\ stopping
         THEN Refuse(e, "stopping")
       ELSE IF s = 0
         THEN (IF BoundLive /\ Cardinality(Held) >= MaxLive
                 THEN Refuse(e, "live")
                 ELSE NewSlot(e))
       ELSE IF BoundInbox /\ Waiting(inbox[s]) >= MaxInbox
         THEN Refuse(e, "full")
       ELSE IF PostHoldsTable
         THEN TakeInto(e, s)
       ELSE /\ status' = [status EXCEPT ![e] = "looked"]
            /\ target' = [target EXCEPT ![e] = s]
            /\ UNCHANGED <<table, made, slotKey, inbox, lin>>
    /\ UNCHANGED <<closed, loaded, holders, rpc, rslot, rinput, evicting, stopping, stage, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* Without PostHoldsTable: the write into the inbox the post looked up, whatever became of its slot.
\* @type: (Str) => Bool;
Write(e) ==
    /\ status[e] = "looked"
    /\ TakeInto(e, target[e])
    /\ UNCHANGED <<closed, loaded, holders, rpc, rslot, rinput, evicting, stopping, stage, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

-----------------------------------------------------------------------------
\* A runner, as Pool::run runs a key.

\* The runner lets the slot's live lock go.
\* @type: (Str) => Bool;
Release(r) ==
    /\ holders' = [holders EXCEPT ![rslot[r]] = @ \ {r}]
    /\ rpc' = [rpc EXCEPT ![r] = "idle"]
    /\ rslot' = [rslot EXCEPT ![r] = 0]

\* Slot s leaves the table whatever its inbox holds: each envelope it held is answered, and each
\* entry it held is lost, for the follower to hand out again.
\* @type: (Int) => Bool;
Abandon(s) ==
    /\ closed' = [closed EXCEPT ![s] = TRUE]
    /\ table' = Remove(s)
    /\ status' = [e \in Inputs |->
                    IF InSeq(e, inbox[s])
                      THEN (IF e \in Envelopes THEN "answered" ELSE "lost")
                      ELSE status[e]]
    /\ inbox' = [inbox EXCEPT ![s] = <<>>]

\* The host runs the key k a post named: the runner reads its slot from the table.
\* @type: (Str, Str) => Bool;
Get(r, k) ==
    /\ rpc[r] = "idle"
    /\ table[k] # 0
    /\ inbox[table[k]] # <<>>
    /\ rpc' = [rpc EXCEPT ![r] = "got"]
    /\ rslot' = [rslot EXCEPT ![r] = table[k]]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rinput, evicting, status,
                   target, stopping, stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* It takes the slot's live lock, once no other thread holds it.
\* @type: (Str) => Bool;
Lock(r) ==
    LET s == rslot[r] IN
    /\ rpc[r] = "got"
    /\ LiveLock => (holders[s] = {} /\ evicting # s)
    /\ holders' = [holders EXCEPT ![s] = @ \cup {r}]
    /\ rpc' = [rpc EXCEPT ![r] = "holds"]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, rslot, rinput, evicting, status,
                   target, stopping, stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* A session out of memory whose inbox holds nothing, or that left the table meanwhile, is not
\* loaded: it leaves the table (Pool::retire_if_idle).
\* @type: (Str) => Bool;
Retire(r) ==
    LET s == rslot[r] IN
    /\ rpc[r] = "holds"
    /\ ~loaded[s]
    /\ closed[s] \/ inbox[s] = <<>>
    /\ closed' = [closed EXCEPT ![s] = TRUE]
    /\ table' = Remove(s)
    /\ Release(r)
    /\ UNCHANGED <<made, slotKey, inbox, loaded, rinput, evicting, status, target, stopping, stage,
                   lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* The session is loaded: the store granted the key's lease (Pool::load).
\* @type: (Str) => Bool;
Load(r) ==
    LET s == rslot[r] IN
    /\ rpc[r] = "holds"
    /\ ~loaded[s]
    /\ ~closed[s]
    /\ inbox[s] # <<>>
    /\ loaded' = [loaded EXCEPT ![s] = TRUE]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, holders, rpc, rslot, rinput, evicting,
                   status, target, stopping, stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* The store refused the key's lease: another process serves it, and what the inbox held is
\* answered (Pool::abandon).
\* @type: (Str) => Bool;
Refused(r) ==
    LET s == rslot[r] IN
    /\ rpc[r] = "holds"
    /\ ~loaded[s]
    /\ ~closed[s]
    /\ inbox[s] # <<>>
    /\ Abandon(s)
    /\ Release(r)
    /\ UNCHANGED <<made, slotKey, loaded, rinput, evicting, target, stopping, stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* The runner takes the oldest input of the inbox (Pool::serve).
\* @type: (Str) => Bool;
Take(r) ==
    LET s == rslot[r] IN
    /\ rpc[r] = "holds"
    /\ loaded[s]
    /\ inbox[s] # <<>>
    /\ rinput' = [rinput EXCEPT ![r] = Head(inbox[s])]
    /\ inbox' = [inbox EXCEPT ![s] = Tail(@)]
    /\ rpc' = [rpc EXCEPT ![r] = "applying"]
    /\ UNCHANGED <<table, made, slotKey, closed, loaded, holders, rslot, evicting, status, target,
                   stopping, stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* The session takes in the entry e when it is above the position it took in of the entry's key,
\* and drops it as a repeat when it is at or below it (docs/adr/0114, D4).
\* @type: (Str) => Bool;
TakeIn(e) ==
    LET k == KeyOf(e) IN
    IF PositionOf(e) > mark[k]
      THEN /\ mark' = [mark EXCEPT ![k] = PositionOf(e)]
           /\ took' = [took EXCEPT ![k] = Append(@, e)]
      ELSE UNCHANGED <<mark, took>>

\* It applies the input it took to the session's machine: an envelope, an entry, or a tell, which
\* nothing here follows.
\* @type: (Str) => Bool;
Apply(r) ==
    LET e == rinput[r] IN
    /\ rpc[r] = "applying"
    /\ IF e \in Envelopes
         THEN /\ applied' = [applied EXCEPT ![KeyOf(e)] = Append(@, e)]
              /\ status' = [status EXCEPT ![e] = "applied"]
              /\ UNCHANGED <<mark, took>>
       ELSE IF e \in Entries
         THEN /\ TakeIn(e)
              /\ status' = [status EXCEPT ![e] = "applied"]
              /\ UNCHANGED applied
       ELSE UNCHANGED <<applied, status, mark, took>>
    /\ rpc' = [rpc EXCEPT ![r] = "holds"]
    /\ rinput' = [rinput EXCEPT ![r] = ""]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rslot, evicting, target,
                   stopping, stage, lin, handed, handedTo, follower, tellVars>>

\* The inbox holds nothing more, and the machine waits (Served::Waiting).
\* @type: (Str) => Bool;
Wait(r) ==
    LET s == rslot[r] IN
    /\ rpc[r] = "holds"
    /\ loaded[s]
    /\ inbox[s] = <<>>
    /\ Release(r)
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, rinput, evicting, status, target,
                   stopping, stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* @type: (Str) => Bool;
RunnerNext(r) ==
    \/ \E k \in Keys : Get(r, k)
    \/ Lock(r)
    \/ Retire(r)
    \/ Load(r)
    \/ Refused(r)
    \/ Take(r)
    \/ Apply(r)
    \/ Wait(r)

-----------------------------------------------------------------------------
\* The evictor (Pool::evict_idle; Pool::make_room evicts the same way).

\* It takes the live lock of a session in memory that nobody runs and whose inbox holds nothing,
\* and saves it as a clean checkpoint.
\* @type: (Int) => Bool;
Idle(s) ==
    /\ evicting = 0
    /\ s <= made
    /\ InTable(s)
    /\ loaded[s]
    /\ holders[s] = {}
    /\ inbox[s] = <<>>
    /\ evicting' = s
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput, status,
                   target, stopping, stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* Saved, the session leaves the table and memory, unless an input reached its inbox meanwhile,
\* when it stays and its owner applies the input.
Out ==
    LET s == evicting IN
    /\ s # 0
    /\ IF ~RemoveIfEmpty \/ inbox[s] = <<>>
         THEN /\ closed' = [closed EXCEPT ![s] = TRUE]
              /\ table' = Remove(s)
              /\ loaded' = [loaded EXCEPT ![s] = FALSE]
         ELSE UNCHANGED <<closed, table, loaded>>
    /\ evicting' = 0
    /\ UNCHANGED <<made, slotKey, inbox, holders, rpc, rslot, rinput, status, target, stopping,
                   stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

-----------------------------------------------------------------------------
\* The stop (Pool::stop).

\* It is decided, with the table held: every admit after it is refused.
Stop ==
    /\ stage = "running"
    /\ stopping' = TRUE
    /\ stage' = "draining"
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* Every inbox the table holds is drained.
Drained ==
    /\ stage = "draining"
    /\ \A s \in HeldSlots : inbox[s] = <<>>
    /\ stage' = "saving"
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* A session in memory that no thread runs is saved, and leaves the table and memory; what its
\* inbox still holds is answered.
\* @type: (Int) => Bool;
Save(s) ==
    /\ stage = "saving"
    /\ s <= made
    /\ InTable(s)
    /\ loaded[s]
    /\ holders[s] = {}
    /\ evicting # s
    /\ loaded' = [loaded EXCEPT ![s] = FALSE]
    /\ Abandon(s)
    /\ UNCHANGED <<made, slotKey, holders, rpc, rslot, rinput, evicting, target, stopping, stage,
                   lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* No session the table holds is in memory: the stop has ended.
Stopped ==
    /\ stage = "saving"
    /\ \A s \in HeldSlots : ~loaded[s]
    /\ stage' = "stopped"
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

-----------------------------------------------------------------------------
\* The follower (Pool::follow), one thread that hands out the entries of the journal.

\* Without StopHoldsTable, the follower reads the stop before it takes the table, and hands an
\* entry out on what it read.
FollowerReadsStop ==
    /\ ~StopHoldsTable
    /\ follower = "idle"
    /\ ~stopping
    /\ follower' = "checked"
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, stage, handed, handedTo, mark, tellVars,
                   lin, applied, took>>

\* The entry e is handed to the session of its key, with the table held (Pool::post_followed, which
\* admits it as slot::admission admits an envelope): the stop, then the inbox of the session the
\* table holds for the key below max_inbox, or a new session below max_live. With FollowInOrder it
\* is handed only once every earlier entry of its key was taken in or is out, and with
\* FollowSameSlot an earlier entry that is out lets it follow only into the slot the table holds
\* for the key now. An entry that meets a bound is not handed, and holds every later entry of its
\* key.
\* @type: (Str) => Bool;
Hand(e) ==
    LET k == KeyOf(e)
        s == table[k]
    IN
    /\ handed[e] = "no"
    /\ follower = (IF StopHoldsTable THEN "idle" ELSE "checked")
    /\ FollowInOrder =>
           \A d \in EarlierOf(e) :
               \/ handed[d] = "in"
               \/ handed[d] = "out" /\ (FollowSameSlot => handedTo[d] = s)
    /\ ~(StopHoldsTable /\ stopping)
    /\ IF s = 0
         THEN /\ ~(BoundLive /\ Cardinality(Held) >= MaxLive)
              /\ NewSlot(e)
              /\ handedTo' = [handedTo EXCEPT ![e] = made + 1]
         ELSE /\ ~(BoundInbox /\ WaitsAsAnEnvelope(e) /\ Waiting(inbox[s]) >= MaxInbox)
              /\ TakeInto(e, s)
              /\ handedTo' = [handedTo EXCEPT ![e] = s]
    /\ handed' = [handed EXCEPT ![e] = "out"]
    /\ follower' = "idle"
    /\ UNCHANGED <<closed, loaded, holders, rpc, rslot, rinput, evicting, stopping, stage, mark,
                   tellVars, applied, took>>

\* At the start of a call the follower reads the receipt of an entry it handed out: one taken in
\* is passed, and one lost is to be handed out again.
\* @type: (Str) => Bool;
Notice(e) ==
    /\ handed[e] = "out"
    /\ status[e] \in {"applied", "lost"}
    /\ handed' = [handed EXCEPT ![e] = IF status[e] = "applied" THEN "in" ELSE "no"]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, stage, handedTo, follower, mark, tellVars,
                   lin, applied, took>>

-----------------------------------------------------------------------------
\* A tell (Pool::tell_holders): an input the pool makes, put in the inbox of each session the table
\* holds, which no bound counts.

\* The inboxes, with x put last in the inbox of each slot of ss.
\* @type: (Str, Set(Int)) => (Int -> Seq(Str));
WithTell(x, ss) == [s \in Slots |-> IF s \in ss THEN Append(inbox[s], x) ELSE inbox[s]]

\* With TellHoldsTable, x is put in the inbox of every slot the table holds, with the table held.
\* Without it, the tell looks the slots up and lets the table go, and writes later (TellWrite).
\* Either is taken whatever the stop, which the pool reads before it takes the table.
\* @type: (Str) => Bool;
Tell(x) ==
    /\ told[x] = "none"
    /\ IF TellHoldsTable
         THEN /\ inbox' = WithTell(x, HeldSlots)
              /\ told' = [told EXCEPT ![x] = "told"]
              /\ UNCHANGED tellTo
         ELSE /\ tellTo' = [tellTo EXCEPT ![x] = HeldSlots]
              /\ told' = [told EXCEPT ![x] = "looked"]
              /\ UNCHANGED inbox
    /\ UNCHANGED <<table, made, slotKey, closed, loaded, holders, rpc, rslot, rinput, evicting,
                   status, target, stopping, stage, followVars, lin, applied>>

\* Without TellHoldsTable: the write into the inboxes the tell looked up, whatever became of their
\* slots.
\* @type: (Str) => Bool;
TellWrite(x) ==
    /\ told[x] = "looked"
    /\ inbox' = WithTell(x, tellTo[x])
    /\ told' = [told EXCEPT ![x] = "told"]
    /\ UNCHANGED <<table, made, slotKey, closed, loaded, holders, rpc, rslot, rinput, evicting,
                   status, target, stopping, stage, followVars, tellTo, lin, applied>>

-----------------------------------------------------------------------------

Next ==
    \/ \E e \in Envelopes : ReadStop(e) \/ Admit(e) \/ Write(e)
    \/ \E r \in Runners : RunnerNext(r)
    \/ \E s \in Slots : Idle(s) \/ Save(s)
    \/ Out
    \/ Stop
    \/ Drained
    \/ Stopped
    \/ FollowerReadsStop
    \/ \E e \in Entries : Hand(e) \/ Notice(e)
    \/ \E x \in Tells : Tell(x) \/ TellWrite(x)

Spec == Init /\ [][Next]_vars

\* The host runs each key a post named, and an eviction under way ends.
LiveSpec == Spec /\ (\A r \in Runners : WF_vars(RunnerNext(r))) /\ WF_vars(Out)

\* Where x is in q.
\* @type: (Seq(Str), Str) => Int;
Pos(q, x) == CHOOSE i \in DOMAIN q : q[i] = x

\* An envelope taken is in an open inbox or being applied, never nowhere.
NoLoss ==
    \A e \in Envelopes :
        status[e] = "taken" =>
            \/ \E s \in 1..made : ~closed[s] /\ InSeq(e, inbox[s])
            \/ \E r \in Runners : rpc[r] = "applying" /\ rinput[r] = e

\* Each key's envelopes are applied in the order they were taken, each once.
InOrder ==
    \A k \in Keys : \A i, j \in DOMAIN applied[k] :
        i < j =>
            /\ InSeq(applied[k][i], lin[k])
            /\ InSeq(applied[k][j], lin[k])
            /\ Pos(lin[k], applied[k][i]) < Pos(lin[k], applied[k][j])

\* No inbox holds more than max_inbox envelopes and entries, which a tell is not counted among,
\* and the table no more sessions than max_live.
Bounded ==
    /\ \A s \in Slots : Cardinality({i \in DOMAIN inbox[s] : inbox[s][i] \in Inputs}) <= MaxInbox
    /\ Cardinality(Held) <= MaxLive

\* Once the stop has ended, every envelope the pool took was applied or answered.
StoppedClean == stage = "stopped" => \A e \in Envelopes : status[e] # "taken"

\* Every envelope taken is applied or answered.
Settled == \A e \in Envelopes : status[e] = "taken" ~> status[e] \in {"applied", "answered"}

\* The entries each key's session took in are the first entries of the key in the order of their
\* positions, each once: the i-th it took in comes after exactly the ones it took in before it.
FollowedInOrder ==
    \A k \in Keys : \A i \in DOMAIN took[k] :
        EarlierOf(took[k][i]) = {took[k][j] : j \in 1..(i - 1)}

\* No tell is in the inbox of a slot that left the table.
ToldInTable == \A s \in Slots : closed[s] => \A i \in DOMAIN inbox[s] : inbox[s][i] \notin Tells

\* Once the stop has ended, no entry waits in an inbox: none was admitted after the stop.
StoppedFollowed == stage = "stopped" => \A e \in Entries : status[e] # "taken"

\* Every entry handed out is taken in, or lost with its slot.
FollowedSettled == \A e \in Entries : status[e] = "taken" ~> status[e] \in {"applied", "lost"}
====
