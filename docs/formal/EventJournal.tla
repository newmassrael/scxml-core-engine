------------------------------ MODULE EventJournal ------------------------------
(***************************************************************************)
(* The journal of ADR 0114: an append-only record of what keyed sessions    *)
(* did, written with the session's state, read by followers with a cursor   *)
(* and as of an earlier moment.                                             *)
(*                                                                         *)
(* A session is run by the process that holds its lease; another process    *)
(* may take it over at any time. A run takes the next input and commits it: *)
(* the session's state and the entry the input made are one write, judged   *)
(* by the lease's fence. A store numbers entries as transactions begin and  *)
(* makes them visible as they commit, so a lower number may appear after a  *)
(* higher one; a reader reads up to the HORIZON, the number below which     *)
(* nothing can still appear. A follower keeps its cursor in the write of    *)
(* what it did. Each part can be switched off, and TLC must then find a     *)
(* violation.                                                               *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   A write is atomic and durable, and its fence is judged inside it.     *)
(*   A position is given once, above every one before, inside the write.   *)
(*   Release: a process that stops ends the transactions it had under way. *)
(*   A clock may lag and does not run back for one process.                *)
(*   LiveSpec: a transaction under way ends, and a follower goes on.       *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets, Sequences, Apalache

CONSTANTS
    \* processes, sessions, inputs of a session, transactions under way, crashes
    \* @type: Int;
    NProc,
    \* @type: Int;
    NSess,
    \* @type: Int;
    MaxInput,
    \* @type: Int;
    MaxOpen,
    \* @type: Int;
    MaxCrash,
    \* a write needs the lease's fence
    \* @type: Bool;
    Fence,
    \* the entry is written with the state
    \* @type: Bool;
    Atomic,
    \* a reader reads up to the horizon
    \* @type: Bool;
    Horizon,
    \* a cursor is written with what its follower did
    \* @type: Bool;
    CursorAtomic,
    \* an earlier moment is a position, not a clock's time
    \* @type: Bool;
    ByPosition,
    \* a process that stops ends the transactions it had under way
    \* @type: Bool;
    Release

Procs == 1..NProc
Nobody == 0             \* whose a transaction is that a stopped process left under way
Sess  == 1..NSess
MaxPos == NSess * MaxInput * 2

VARIABLES
    \* the lease of a session; a process's copy of it
    \* @type: Int -> Int;
    fence,
    \* @type: Int -> (Int -> Int);
    hold,
    \* @type: Int -> (Int -> Int);
    mem,
    \* the store: state, visible entries, transactions under way
    \* @type: Int -> Int;
    st,
    \* @type: Set({pos: Int, s: Int, n: Int, at: Int});
    log,
    \* @type: Set({p: Int, s: Int, n: Int, pos: Int, f: Int, at: Int});
    open,
    \* @type: Int;
    nextpos,
    \* each process's clock, which may lag
    \* @type: Int -> Int;
    clock,
    \* entries whose state was written and that are not yet (not Atomic)
    \* @type: Set({pos: Int, s: Int, n: Int, at: Int});
    pend,
    \* the follower
    \* @type: Int;
    cursor,
    \* @type: <<Int, Int>> -> Int;
    applied,
    \* @type: Int;
    staged,
    \* the answers given as of a moment, and the crashes so far
    \* @type: Set({r: Int, held: Set({pos: Int, s: Int, n: Int, at: Int})});
    asked,
    \* @type: Int;
    crashes

vars == <<fence, hold, mem, st, log, open, nextpos, clock, pend, cursor, applied, staged, asked, crashes>>

Init ==
    /\ fence = [s \in Sess |-> 0] /\ hold = [p \in Procs |-> [s \in Sess |-> 0]]
    /\ mem = [p \in Procs |-> [s \in Sess |-> 0]]
    /\ st = [s \in Sess |-> 0] /\ log = {} /\ open = {} /\ nextpos = 1
    /\ clock = [p \in Procs |-> 1] /\ pend = {}
    /\ cursor = 0 /\ applied = [e \in (Sess \X (1..MaxInput)) |-> 0] /\ staged = 0
    /\ asked = {} /\ crashes = 0

Take(p, s) ==
    /\ fence[s] < 2
    /\ fence' = [fence EXCEPT ![s] = @ + 1]
    /\ hold' = [hold EXCEPT ![p][s] = fence[s] + 1]
    /\ mem' = [mem EXCEPT ![p][s] = st[s]]
    /\ UNCHANGED <<st, log, open, nextpos, clock, pend, cursor, applied, staged, asked, crashes>>

Tick(p) ==
    /\ clock[p] < 2 /\ clock' = [clock EXCEPT ![p] = @ + 1]
    /\ UNCHANGED <<fence, hold, mem, st, log, open, nextpos, pend, cursor, applied, staged, asked, crashes>>

\* A transaction begins: the store numbers the entry it will write.
Begin(p, s) ==
    /\ hold[p][s] # 0 /\ mem[p][s] < MaxInput /\ Cardinality(open) < MaxOpen
    /\ nextpos <= MaxPos
    /\ ~ \E t \in open : t.p = p /\ t.s = s
    /\ open' = open \cup {[p |-> p, s |-> s, n |-> mem[p][s] + 1, pos |-> nextpos,
                           f |-> hold[p][s], at |-> clock[p]]}
    /\ nextpos' = nextpos + 1
    /\ UNCHANGED <<fence, hold, mem, st, log, clock, pend, cursor, applied, staged, asked, crashes>>

Entry(t) == [pos |-> t.pos, s |-> t.s, n |-> t.n, at |-> t.at]

\* It commits or, its fence being another's now, writes nothing.
Commit(t) ==
    /\ t \in open /\ t.p # Nobody /\ open' = open \ {t}
    /\ IF Fence /\ t.f # fence[t.s]
       THEN /\ hold' = [hold EXCEPT ![t.p][t.s] = 0]
            /\ UNCHANGED <<st, log, mem, pend>>
       ELSE /\ st' = [st EXCEPT ![t.s] = t.n]
            /\ mem' = [mem EXCEPT ![t.p][t.s] = t.n]
            /\ IF Atomic THEN log' = log \cup {Entry(t)} /\ UNCHANGED pend
                         ELSE pend' = pend \cup {Entry(t)} /\ UNCHANGED log
            /\ UNCHANGED hold
    /\ UNCHANGED <<fence, nextpos, clock, cursor, applied, staged, asked, crashes>>

\* Not Atomic: the entry is appended by a write of its own.
AppendPending(e) ==
    /\ e \in pend /\ pend' = pend \ {e} /\ log' = log \cup {e}
    /\ UNCHANGED <<fence, hold, mem, st, open, nextpos, clock, cursor, applied, staged, asked, crashes>>

\* A process stops: what it had under way is gone.
Crash(p) ==
    /\ crashes < MaxCrash /\ crashes' = crashes + 1
    /\ hold' = [hold EXCEPT ![p] = [s \in Sess |-> 0]]
    /\ open' = {t \in open : t.p # p}
               \cup (IF Release THEN {} ELSE {[t EXCEPT !.p = Nobody] : t \in {u \in open : u.p = p}})
    /\ pend' = {} /\ staged' = 0
    /\ UNCHANGED <<fence, mem, st, log, nextpos, clock, cursor, applied, asked>>

-----------------------------------------------------------------------------
\* The position below which nothing can still appear.
Limit == IF Horizon /\ open # {}
         THEN (CHOOSE q \in {t.pos : t \in open} : \A r \in {t.pos : t \in open} : q <= r) - 1
         ELSE nextpos - 1

Readable == {e \in log : e.pos > cursor /\ e.pos <= Limit}

First == CHOOSE e \in Readable : \A o \in Readable : e.pos <= o.pos

Follow ==
    /\ Readable # {} /\ staged = 0
    /\ LET e == First IN
       /\ applied' = [applied EXCEPT ![<<e.s, e.n>>] = @ + 1]
       /\ IF CursorAtomic THEN cursor' = e.pos /\ UNCHANGED staged
                          ELSE staged' = e.pos /\ UNCHANGED cursor
    /\ UNCHANGED <<fence, hold, mem, st, log, open, nextpos, clock, pend, asked, crashes>>

KeepCursor ==
    /\ staged # 0 /\ cursor' = staged /\ staged' = 0
    /\ UNCHANGED <<fence, hold, mem, st, log, open, nextpos, clock, pend, applied, asked, crashes>>

\* What the journal held as of r: a position at or below the horizon, or a clock's moment.
AsOf(r) == IF ByPosition THEN {e \in log : e.pos <= r} ELSE {e \in log : e.at <= r}

Ask(r) ==
    \* A clock's moment is asked for once it is past on the asker's clock.
    /\ IF ByPosition THEN r <= Limit ELSE r \in 1..1 /\ \E p \in Procs : clock[p] > r
    /\ ~ \E a \in asked : a.r = r
    /\ asked' = asked \cup {[r |-> r, held |-> AsOf(r)]}
    /\ UNCHANGED <<fence, hold, mem, st, log, open, nextpos, clock, pend, cursor, applied, staged, crashes>>

Next ==
    \/ \E p \in Procs, s \in Sess : Take(p, s) \/ Begin(p, s)
    \/ \E p \in Procs : Tick(p) \/ Crash(p)
    \/ \E t \in open : Commit(t)
    \/ \E e \in pend : AppendPending(e)
    \/ Follow \/ KeepCursor
    \/ \E r \in 1..MaxPos : Ask(r)

Spec == Init /\ [][Next]_vars

\* A transaction under way ends, and a follower goes on: what a follower's progress stands on.
Fair ==
    /\ WF_vars(\E t \in open : Commit(t))
    /\ WF_vars(Follow) /\ WF_vars(KeepCursor)

LiveSpec == Spec /\ Fair

-----------------------------------------------------------------------------
\* An input makes one entry.
AppendOnce == \A e, f \in log : (e.s = f.s /\ e.n = f.n) => e = f

\* A state that was written has its entries: none is missing.
NothingLost == \A s \in Sess : \A n \in 1..st[s] :
    \E e \in log \cup pend : e.s = s /\ e.n = n

\* A follower did each entry once, and passed none.
FollowedOnce == \A k \in DOMAIN applied : applied[k] <= 1
NonePassed == \A e \in log : e.pos <= cursor => applied[<<e.s, e.n>>] >= 1

\* Every entry the journal holds is followed in the end.
Followed == \A k \in Sess \X (1..MaxInput) : (\E e \in log : <<e.s, e.n>> = k) ~> (applied[k] >= 1)

\* A follower has taken in all there is. From every state it can come to this (`AG EF`): nothing
\* that happens freezes the horizon for good.
CaughtUp == open = {} /\ Readable = {} /\ staged = 0

\* An answer as of a moment does not change after it was given.
Stable == \A a \in asked : AsOf(a.r) = a.held

-----------------------------------------------------------------------------
\* What the protocol implements (docs/adr/0115, D2): the journal a reader may rely on is the entries at
\* or below the horizon, by their position, and what the reader has taken is the ones at or below its
\* cursor.
Visible == {e \in log : e.pos <= Limit}

\* The i-th entry by position is the one with i - 1 below it.
ByPos(es) == [i \in 1..Cardinality(es) |->
    LET e == CHOOSE x \in es : Cardinality({y \in es : y.pos < x.pos}) = i - 1
    IN <<e.s, e.n>>]

Service == INSTANCE JournalService WITH
    Streams <- Sess,
    MaxEntry <- MaxInput,
    journal <- ByPos(Visible),
    taken <- Cardinality({e \in log : e.pos <= cursor}),
    did <- applied

Refines == Service!Spec

-----------------------------------------------------------------------------
\* A follower misses nothing for every length of execution (docs/adr/0115, D3; docs/adr/0114, D3 and
\* D4): an inductive invariant that implies NonePassed and FollowedOnce, checked by Apalache at the
\* parameters of a configuration (models.json), over every state it allows and not only the
\* reachable ones.
\*
\* Why: a position is given once, above every one before, so the entries and the transactions under
\* way have distinct positions; a write is judged by the lease's fence, and the one process that
\* holds the fence has the state it continues, so each input makes one entry; the follower's cursor
\* never passes a transaction under way (the horizon), so an entry that appears later is above it;
\* and the follower keeps its cursor with what it did, so it has done each entry at or below its
\* cursor once and none above. An answer as of a position is below the horizon too, so it is the
\* journal as of that position for ever (Stable), the one value its type gives it below.

EntryRecords == [pos : 1..MaxPos, s : Sess, n : 1..MaxInput, at : 1..2]
Transactions == [p : Procs, s : Sess, n : 1..MaxInput, pos : 1..MaxPos, f : 1..2, at : 1..2]

\* Each variable a value of its type. Its conjuncts give every variable a value (x \in S), so
\* Apalache starts the step from the invariant itself.
JournalTypeOK ==
    /\ fence \in [Sess -> 0..2]
    /\ hold \in [Procs -> [Sess -> 0..2]]
    /\ mem \in [Procs -> [Sess -> 0..MaxInput]]
    /\ st \in [Sess -> 0..MaxInput]
    /\ log \in SUBSET EntryRecords
    /\ open \in SUBSET Transactions
    /\ nextpos \in 1..(MaxPos + 1)
    /\ clock \in [Procs -> 1..2]
    /\ pend \in SUBSET EntryRecords
    /\ cursor \in 0..MaxPos
    /\ applied \in [Sess \X (1..MaxInput) -> 0..2]
    /\ staged \in 0..MaxPos
    /\ asked \in SUBSET {[r |-> q, held |-> AsOf(q)] : q \in 1..MaxPos}
    /\ crashes \in 0..MaxCrash

FollowerInductive ==
    /\ JournalTypeOK
    /\ FollowedOnce
    /\ pend = {} /\ staged = 0
    \* Positions: each given once, below the next.
    /\ \A t \in open : t.pos < nextpos
    /\ \A e \in log : e.pos < nextpos
    /\ \A t, u \in open : t.pos = u.pos => t = u
    /\ \A e, d \in log : e.pos = d.pos => e = d
    /\ \A t \in open : \A e \in log : t.pos # e.pos
    \* The lease: one holder of the fence, which has the state its write continues.
    /\ \A s \in Sess :
           /\ \A p \in Procs : hold[p][s] <= fence[s]
           /\ \A p, q \in Procs : (hold[p][s] # 0 /\ hold[p][s] = hold[q][s]) => p = q
           /\ \A p \in Procs : (hold[p][s] # 0 /\ hold[p][s] = fence[s]) => mem[p][s] = st[s]
    /\ \A t \in open : t.f <= fence[t.s]
    /\ \A t \in open : t.f = fence[t.s] => (hold[t.p][t.s] = t.f /\ t.n = st[t.s] + 1)
    /\ \A t, u \in open : (t.p = u.p /\ t.s = u.s) => t = u
    \* One entry per input.
    /\ \A e \in log : e.n <= st[e.s]
    /\ AppendOnce
    \* The follower: below the horizon, each entry at or below its cursor done once and none above.
    /\ cursor < nextpos
    /\ \A t \in open : cursor < t.pos
    /\ \A e \in log : applied[<<e.s, e.n>>] = IF e.pos <= cursor THEN 1 ELSE 0
    /\ \A k \in DOMAIN applied : (\A e \in log : <<e.s, e.n>> # k) => applied[k] = 0
    \* An answer as of a position: below the horizon.
    /\ \A a \in asked : a.r < nextpos /\ \A t \in open : a.r < t.pos

=============================================================================
