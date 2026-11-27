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
(*                                                                         *)
(* lin and applied are history variables: the order each key's envelopes  *)
(* were taken in, and the order they were applied in, which no pool keeps. *)
(*                                                                         *)
(* Left out, each for its reason: the lease's lock, whose every access is  *)
(* one step of the proved KeyLease (the lease across processes is          *)
(* CheckpointWrite's); the wake of a session that comes due (Pool::due),   *)
(* an admit through another door under the same table and the same stop;   *)
(* the inputs the pool makes for each session the table holds (a link's    *)
(* readiness, a tick, a note), which no bound counts and no order of       *)
(* envelopes depends on; and the eviction a post makes room by, which is   *)
(* the evictor's on the posting thread.                                    *)
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
    StopHoldsTable

Keys == {First, Second}
Envelopes == FirstEnvelopes \cup SecondEnvelopes
Slots == 1..MaxSlots

\* The key an envelope names.
\* @type: (Str) => Str;
KeyOf(e) == IF e \in FirstEnvelopes THEN First ELSE Second

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
    \* what became of each envelope: none; checked, the stop read apart from the table; looked,
    \* its slot looked up and the table let go; taken; applied; answered; or refused for a reason
    \* (stopping, full, live)
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
    \* the history variables
    \* @type: Str -> Seq(Str);
    lin,
    \* @type: Str -> Seq(Str);
    applied

vars == <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput, evicting,
          status, target, stopping, stage, lin, applied>>

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
    /\ status = [e \in Envelopes |-> "none"]
    /\ target = [e \in Envelopes |-> 0]
    /\ stopping = FALSE
    /\ stage = "running"
    /\ lin = [k \in Keys |-> <<>>]
    /\ applied = [k \in Keys |-> <<>>]

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
    /\ lin' = [lin EXCEPT ![slotKey[s]] = Append(@, e)]
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
    /\ lin' = [lin EXCEPT ![k] = Append(@, e)]
    /\ UNCHANGED target

\* Without StopHoldsTable, the poster reads the stop before it takes the table.
\* @type: (Str) => Bool;
ReadStop(e) ==
    /\ ~StopHoldsTable
    /\ status[e] = "none"
    /\ status' = [status EXCEPT ![e] = IF stopping THEN "stopping" ELSE "checked"]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, target, stopping, stage, lin, applied>>

\* The admit, with the table held (slot::admit): the stop, then the inbox of the session the table
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
       ELSE IF BoundInbox /\ Len(inbox[s]) >= MaxInbox
         THEN Refuse(e, "full")
       ELSE IF PostHoldsTable
         THEN TakeInto(e, s)
       ELSE /\ status' = [status EXCEPT ![e] = "looked"]
            /\ target' = [target EXCEPT ![e] = s]
            /\ UNCHANGED <<table, made, slotKey, inbox, lin>>
    /\ UNCHANGED <<closed, loaded, holders, rpc, rslot, rinput, evicting, stopping, stage, applied>>

\* Without PostHoldsTable: the write into the inbox the post looked up, whatever became of its slot.
\* @type: (Str) => Bool;
Write(e) ==
    /\ status[e] = "looked"
    /\ TakeInto(e, target[e])
    /\ UNCHANGED <<closed, loaded, holders, rpc, rslot, rinput, evicting, stopping, stage, applied>>

-----------------------------------------------------------------------------
\* A runner, as Pool::run runs a key.

\* The runner lets the slot's live lock go.
\* @type: (Str) => Bool;
Release(r) ==
    /\ holders' = [holders EXCEPT ![rslot[r]] = @ \ {r}]
    /\ rpc' = [rpc EXCEPT ![r] = "idle"]
    /\ rslot' = [rslot EXCEPT ![r] = 0]

\* Slot s leaves the table whatever its inbox holds, and each envelope it held is answered.
\* @type: (Int) => Bool;
Abandon(s) ==
    /\ closed' = [closed EXCEPT ![s] = TRUE]
    /\ table' = Remove(s)
    /\ status' = [e \in Envelopes |-> IF InSeq(e, inbox[s]) THEN "answered" ELSE status[e]]
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

\* It applies the input it took to the session's machine.
\* @type: (Str) => Bool;
Apply(r) ==
    LET e == rinput[r] IN
    /\ rpc[r] = "applying"
    /\ applied' = [applied EXCEPT ![KeyOf(e)] = Append(@, e)]
    /\ status' = [status EXCEPT ![e] = "applied"]
    /\ rpc' = [rpc EXCEPT ![r] = "holds"]
    /\ rinput' = [rinput EXCEPT ![r] = ""]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rslot, evicting, target,
                   stopping, stage, lin>>

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

-----------------------------------------------------------------------------
\* The stop (Pool::stop).

\* It is decided, with the table held: every admit after it is refused.
Stop ==
    /\ stage = "running"
    /\ stopping' = TRUE
    /\ stage' = "draining"
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, lin, applied>>

\* Every inbox the table holds is drained.
Drained ==
    /\ stage = "draining"
    /\ \A k \in Keys : table[k] = 0 \/ inbox[table[k]] = <<>>
    /\ stage' = "saving"
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, lin, applied>>

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

\* No session the table holds is in memory: the stop has ended.
Stopped ==
    /\ stage = "saving"
    /\ \A k \in Keys : table[k] = 0 \/ ~loaded[table[k]]
    /\ stage' = "stopped"
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, lin, applied>>

-----------------------------------------------------------------------------

Next ==
    \/ \E e \in Envelopes : ReadStop(e) \/ Admit(e) \/ Write(e)
    \/ \E r \in Runners : RunnerNext(r)
    \/ \E s \in Slots : Idle(s) \/ Save(s)
    \/ Out
    \/ Stop
    \/ Drained
    \/ Stopped

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

\* No inbox holds more than max_inbox, and the table no more sessions than max_live.
Bounded ==
    /\ \A s \in Slots : Len(inbox[s]) <= MaxInbox
    /\ Cardinality(Held) <= MaxLive

\* Once the stop has ended, every envelope the pool took was applied or answered.
StoppedClean == stage = "stopped" => \A e \in Envelopes : status[e] # "taken"

\* Every envelope taken is applied or answered.
Settled == \A e \in Envelopes : status[e] = "taken" ~> status[e] \in {"applied", "answered"}
====
