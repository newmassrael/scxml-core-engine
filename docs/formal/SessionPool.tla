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
(* reader reads the entries in the order of their positions, holding at    *)
(* most a window of them read and not yet taken in (FOLLOW_WINDOW), and    *)
(* hands each to the session of its key with the table held, as a post is  *)
(* made (Pool::post_followed): an entry waits in the inbox and counts      *)
(* toward max_inbox as an envelope does (Input::waits_as_an_envelope). It  *)
(* hands the entries of a key in the order of their positions: an entry    *)
(* that meets a bound, or that was lost when its slot left the table with  *)
(* it, holds every later entry of its key, and a later entry follows an    *)
(* earlier one that is out only into the slot the table holds for the key  *)
(* now. The reader reads what became of an entry it handed at the start of *)
(* a call, so an entry its slot lost is out in that slot until it reads    *)
(* it, and then keeps its mark: the position through which every entry it  *)
(* read was taken in. A session takes in an entry above the position it    *)
(* took in of the entry's key, and drops one at or below it as a repeat.   *)
(*                                                                         *)
(* An envelope may carry a position of the journal (docs/adr/0116, D5): it *)
(* waits in its inbox until the reader's mark has passed the position, and *)
(* so does every envelope behind it, while every other input, an entry     *)
(* among them, is taken past them (Inbox::take). An entry that meets the   *)
(* bound of an inbox in which such an envelope waits is put in past the    *)
(* bound, counted by none (Pool::post_past_waiting), since the mark that   *)
(* envelope waits for may wait for it; the window bounds how many are put  *)
(* in so. A stop drains what the sessions can take, and answers what waits *)
(* in the inbox of a session it never loaded.                              *)
(*                                                                         *)
(* A tell (Pool::tell_holders; Pool::end_members tells EndMember the same  *)
(* way) puts an input the pool makes in the inbox of each session the      *)
(* table holds, with the table held, unless the pool stops, and no bound   *)
(* counts it.                                                              *)
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
(*   PastWaiting     an entry that meets the bound is put in past it only  *)
(*                   where an envelope waits for the reader                *)
(*   ReadWindow      the reader reads an entry only while it holds fewer   *)
(*                   than the window read and not yet taken in             *)
(*   SkipWaiting     a runner passes an envelope that waits for the reader *)
(*                   and every envelope behind it                          *)
(*   DrainTakeable   the stop waits only for what the sessions can take    *)
(*   AnswerUnloaded  the stop answers what waits in the inbox of a session *)
(*                   it never loaded                                       *)
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
(* thread; of the reader, its page, the store its mark is kept in, which   *)
(* is EventJournal's (Follow with CursorAtomic), an entry of a key another *)
(* process holds, which is that process's to hand, and an entry that names *)
(* no session; a position above the journal's horizon, which is refused    *)
(* where it is received, so an envelope carries one at or below the last   *)
(* entry; and a durable checkpoint apart from the save of an eviction or a *)
(* stop, so the receipt of an entry is settled when its session applies    *)
(* it.                                                                     *)
(*                                                                         *)
(* Taken as given: a mutex is mutual exclusion; what a thread does with a  *)
(* lock held is one step; an inbox's queue and its door move together     *)
(* (pool/slot.rs); the stop flag is read and written sequentially          *)
(* consistently (an atomic of Ordering::SeqCst, and under the table only   *)
(* its holders decide by it), and so is the reader's mark                  *)
(* (Pool::read_mark), which only rises, so a thread that read it below its *)
(* value now decides as it would have earlier.                             *)
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
    \* the envelopes that carry a position of the journal, and the position each carries
    \* @type: Set(Str);
    Carriers,
    \* @type: Int;
    After,
    \* the most entries the reader holds read and not yet taken in (FOLLOW_WINDOW)
    \* @type: Int;
    Window,
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
    TellHoldsTable,
    \* @type: Bool;
    PastWaiting,
    \* @type: Bool;
    ReadWindow,
    \* @type: Bool;
    SkipWaiting,
    \* @type: Bool;
    DrainTakeable,
    \* @type: Bool;
    AnswerUnloaded

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

\* The smallest of a set of positions.
\* @type: (Set(Int)) => Int;
Least(S) == CHOOSE p \in S : \A q \in S : p <= q

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
    \* the position of the last entry the reader read
    \* @type: Int;
    read,
    \* the reader's mark as it was last kept: every entry at or below it was taken in
    \* @type: Int;
    readMark,
    \* whether each entry was put in past the bound of its inbox, which counts it then by none
    \* @type: Str -> Bool;
    past,
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
          status, target, stopping, stage, handed, handedTo, follower, mark, read, readMark, past,
          told, tellTo, lin, applied, took>>

\* What the follower and the tells move.
followVars == <<handed, handedTo, follower, mark, read, readMark, past, took>>
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

\* Whether the input x is counted by the bound of the inbox it is in: an envelope is, and an entry
\* unless it was put in past the bound. A tell never is.
\* @type: (Str) => Bool;
Counted(x) == x \in Envelopes \/ (x \in Entries /\ ~past[x])

\* Whether the input x counts toward max_inbox (Input::waits_as_an_envelope): what is counted, an
\* entry only with BoundFollowed.
\* @type: (Str) => Bool;
WaitsAsAnEnvelope(x) == Counted(x) /\ (BoundFollowed \/ x \notin Entries)

\* How many inputs of the inbox q count toward max_inbox (the bounded count of its Door).
\* @type: (Seq(Str)) => Int;
Waiting(q) == Cardinality({i \in DOMAIN q : WaitsAsAnEnvelope(q[i])})

\* Whether the input x is an envelope that waits for the reader's mark to pass its position
\* (Input::waits_on_reader). One whose position the mark had passed when it was posted carries
\* none, which is the same, since the mark only rises.
\* @type: (Str) => Bool;
WaitsOnReader(x) == x \in Carriers /\ After > readMark

\* Whether a runner may take the i-th input of q: with SkipWaiting, one that is no envelope, or an
\* envelope no envelope at or before it waits for the reader (Inbox::takeable).
\* @type: (Seq(Str), Int) => Bool;
MayTake(q, i) ==
    \/ ~SkipWaiting
    \/ q[i] \notin Envelopes
    \/ \A j \in 1..i : q[j] \in Envelopes => ~WaitsOnReader(q[j])

\* Where the oldest input a runner may take is in q, 0 for none.
\* @type: (Seq(Str)) => Int;
TakeableAt(q) ==
    LET at == {i \in DOMAIN q : MayTake(q, i)} IN
    IF at = {} THEN 0 ELSE Least(at)

\* q without its i-th input.
\* @type: (Seq(Str), Int) => Seq(Str);
Without(q, i) == SubSeq(q, 1, i - 1) \o SubSeq(q, i + 1, Len(q))

\* The entries the reader read and has not read taken in, and the position through which every
\* entry it read was taken in, which it keeps as its mark (Reader::mark).
Open == {e \in Entries : PositionOf(e) <= read /\ handed[e] # "in"}
ReadThrough == IF Open = {} THEN read ELSE Least({PositionOf(e) : e \in Open}) - 1

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
    /\ read = 0
    /\ readMark = 0
    /\ past = [e \in Entries |-> FALSE]
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

\* The host runs the key k a post named: the runner reads its slot from the table. A session in
\* memory is run for an input it may take; one out of memory, for any.
\* @type: (Str, Str) => Bool;
Get(r, k) ==
    /\ rpc[r] = "idle"
    /\ table[k] # 0
    /\ inbox[table[k]] # <<>>
    /\ ~loaded[table[k]] \/ TakeableAt(inbox[table[k]]) # 0
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

\* The runner takes the oldest input of the inbox it may take (Pool::serve, Inbox::take).
\* @type: (Str) => Bool;
Take(r) ==
    LET s == rslot[r]
        i == TakeableAt(inbox[s])
    IN
    /\ rpc[r] = "holds"
    /\ loaded[s]
    /\ i # 0
    /\ rinput' = [rinput EXCEPT ![r] = inbox[s][i]]
    /\ inbox' = [inbox EXCEPT ![s] = Without(@, i)]
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
                   stopping, stage, lin, handed, handedTo, follower, read, readMark, past,
                   tellVars>>

\* The inbox holds nothing more the runner may take, and the machine waits (Served::Waiting).
\* @type: (Str) => Bool;
Wait(r) ==
    LET s == rslot[r] IN
    /\ rpc[r] = "holds"
    /\ loaded[s]
    /\ TakeableAt(inbox[s]) = 0
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

\* Every inbox the table holds is drained of what its session may take; with DrainTakeable, an
\* envelope that waits for the reader, which a stopping pool no longer runs, is not waited for.
Drained ==
    /\ stage = "draining"
    /\ \A s \in HeldSlots : IF DrainTakeable THEN TakeableAt(inbox[s]) = 0 ELSE inbox[s] = <<>>
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

\* With AnswerUnloaded, a session never loaded whose inbox still holds something, envelopes that
\* wait for the reader, leaves the table, and what its inbox holds is answered.
\* @type: (Int) => Bool;
Answer(s) ==
    /\ AnswerUnloaded
    /\ stage = "saving"
    /\ s <= made
    /\ InTable(s)
    /\ ~loaded[s]
    /\ holders[s] = {}
    /\ inbox[s] # <<>>
    /\ Abandon(s)
    /\ UNCHANGED <<made, slotKey, loaded, holders, rpc, rslot, rinput, evicting, target, stopping,
                   stage, lin, applied>>
    /\ UNCHANGED <<followVars, tellVars>>

\* No session the table holds is in memory, and with AnswerUnloaded none holds anything: the stop
\* has ended.
Stopped ==
    /\ stage = "saving"
    /\ \A s \in HeldSlots : ~loaded[s] /\ (AnswerUnloaded => inbox[s] = <<>>)
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
                   evicting, status, target, stopping, stage, handed, handedTo, mark, read,
                   readMark, past, tellVars, lin, applied, took>>

\* The entry e, read, is handed to the session of its key, with the table held
\* (Pool::post_followed, which admits it as slot::admission admits an envelope): the stop, then the
\* inbox of the session the table holds for the key below max_inbox, or a new session below
\* max_live. With FollowInOrder it is handed only once every earlier entry of its key was taken in
\* or is out, and with FollowSameSlot an earlier entry that is out lets it follow only into the
\* slot the table holds for the key now. At the inbox's bound it is put in past the bound, counted
\* by none, where an envelope of the inbox waits for the reader (Pool::post_past_waiting), which
\* without PastWaiting it never is. An entry not handed holds every later entry of its key.
\* @type: (Str) => Bool;
Hand(e) ==
    LET k == KeyOf(e)
        s == table[k]
    IN
    /\ handed[e] = "no"
    /\ PositionOf(e) <= read
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
              /\ past' = [past EXCEPT ![e] = FALSE]
       ELSE IF ~(BoundInbox /\ BoundFollowed /\ Waiting(inbox[s]) >= MaxInbox)
         THEN /\ TakeInto(e, s)
              /\ handedTo' = [handedTo EXCEPT ![e] = s]
              /\ past' = [past EXCEPT ![e] = FALSE]
       ELSE /\ PastWaiting
            /\ \E j \in DOMAIN inbox[s] : WaitsOnReader(inbox[s][j])
            /\ TakeInto(e, s)
            /\ handedTo' = [handedTo EXCEPT ![e] = s]
            /\ past' = [past EXCEPT ![e] = TRUE]
    /\ handed' = [handed EXCEPT ![e] = "out"]
    /\ follower' = "idle"
    /\ UNCHANGED <<closed, loaded, holders, rpc, rslot, rinput, evicting, stopping, stage, mark,
                   read, readMark, tellVars, applied, took>>

\* At the start of a call the follower reads the receipt of an entry it handed out: one taken in
\* is passed, and one lost is to be handed out again.
\* @type: (Str) => Bool;
Notice(e) ==
    /\ ~stopping
    /\ handed[e] = "out"
    /\ status[e] \in {"applied", "lost"}
    /\ handed' = [handed EXCEPT ![e] = IF status[e] = "applied" THEN "in" ELSE "no"]
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, stage, handedTo, follower, mark, read,
                   readMark, past, tellVars, lin, applied, took>>

\* The reader reads the next entry of the journal, with ReadWindow only while it holds fewer than
\* Window read and not yet taken in (FOLLOW_WINDOW).
Read ==
    /\ ~stopping
    /\ read < Len(Journal)
    /\ ReadWindow => Cardinality(Open) < Window
    /\ read' = read + 1
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, stage, handed, handedTo, follower, mark,
                   readMark, past, tellVars, lin, applied, took>>

\* At the end of a call the reader keeps its mark once it rose (JournalStore::keep_read), and the
\* envelopes that waited for it may be taken (Pool::passed).
KeepMark ==
    /\ ~stopping
    /\ ReadThrough > readMark
    /\ readMark' = ReadThrough
    /\ UNCHANGED <<table, made, slotKey, inbox, closed, loaded, holders, rpc, rslot, rinput,
                   evicting, status, target, stopping, stage, handed, handedTo, follower, mark,
                   read, past, tellVars, lin, applied, took>>

\* A step of the follower's thread.
FollowerNext == Read \/ KeepMark \/ \E e \in Entries : Hand(e) \/ Notice(e)

-----------------------------------------------------------------------------
\* A tell (Pool::tell_holders): an input the pool makes, put in the inbox of each session the table
\* holds, which no bound counts.

\* The inboxes, with x put last in the inbox of each slot of ss.
\* @type: (Str, Set(Int)) => (Int -> Seq(Str));
WithTell(x, ss) == [s \in Slots |-> IF s \in ss THEN Append(inbox[s], x) ELSE inbox[s]]

\* With TellHoldsTable, x is put in the inbox of every slot the table holds, with the table held.
\* Without it, the tell looks the slots up and lets the table go, and writes later (TellWrite).
\* Neither is taken once the pool stops, which the tell reads with the table held.
\* @type: (Str) => Bool;
Tell(x) ==
    /\ told[x] = "none"
    /\ ~stopping
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
    \/ \E s \in Slots : Answer(s)
    \/ FollowerReadsStop
    \/ FollowerNext
    \/ \E x \in Tells : Tell(x) \/ TellWrite(x)

Spec == Init /\ [][Next]_vars

\* A step of the stop's thread once it is decided.
StopNext == Drained \/ Stopped \/ \E s \in Slots : Save(s) \/ Answer(s)

\* The host runs each key a post named, an eviction under way ends, the host calls the reader on
\* its schedule, and a stop decided runs on.
LiveSpec ==
    /\ Spec
    /\ \A r \in Runners : WF_vars(RunnerNext(r))
    /\ WF_vars(Out)
    /\ WF_vars(FollowerNext)
    /\ WF_vars(StopNext)

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

\* An envelope that carries a position is applied only once every entry at or below the position
\* was taken in by the session of its key.
AppliedAfterRead ==
    \A k \in Keys : \A i \in DOMAIN applied[k] :
        applied[k][i] \in Carriers =>
            \A d \in Entries : PositionOf(d) <= After => InSeq(d, took[KeyOf(d)])

\* No more entries are in inboxes past their bound than the reader's window.
PastBounded == Cardinality({e \in Entries : past[e] /\ status[e] = "taken"}) <= Window

\* A stop decided ends.
StopEnds == stage = "draining" ~> stage = "stopped"
====
