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
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS NProc, NSess, MaxInput, MaxOpen, MaxCrash,
          Fence,        \* a write needs the lease's fence
          Atomic,       \* the entry is written with the state
          Horizon,      \* a reader reads up to the horizon
          CursorAtomic, \* a cursor is written with what its follower did
          ByPosition    \* an earlier moment is a position, not a clock's time

Procs == 1..NProc
Sess  == 1..NSess
MaxPos == NSess * MaxInput * 2

VARIABLES fence, hold, mem,         \* the lease of a session; a process's copy of it
          st, log, open, nextpos,   \* the store: state, visible entries, transactions under way
          clock,                    \* each process's clock, which may lag
          pend,                     \* entries whose state was written and that are not yet (not Atomic)
          cursor, applied, staged,  \* the follower
          asked, crashes

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
    /\ t \in open /\ open' = open \ {t}
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
Append(e) ==
    /\ e \in pend /\ pend' = pend \ {e} /\ log' = log \cup {e}
    /\ UNCHANGED <<fence, hold, mem, st, open, nextpos, clock, cursor, applied, staged, asked, crashes>>

\* A process stops: what it had under way is gone.
Crash(p) ==
    /\ crashes < MaxCrash /\ crashes' = crashes + 1
    /\ hold' = [hold EXCEPT ![p] = [s \in Sess |-> 0]]
    /\ open' = {t \in open : t.p # p}
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
    \/ \E e \in pend : Append(e)
    \/ Follow \/ KeepCursor
    \/ \E r \in 1..MaxPos : Ask(r)

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* An input makes one entry.
AppendOnce == \A e, f \in log : (e.s = f.s /\ e.n = f.n) => e = f

\* A state that was written has its entries: none is missing.
NothingLost == \A s \in Sess : \A n \in 1..st[s] :
    \E e \in log \cup pend : e.s = s /\ e.n = n

\* A follower did each entry once, and passed none.
FollowedOnce == \A k \in DOMAIN applied : applied[k] <= 1
NonePassed == \A e \in log : e.pos <= cursor => applied[<<e.s, e.n>>] >= 1

\* An answer as of a moment does not change after it was given.
Stable == \A a \in asked : AsOf(a.r) = a.held

=============================================================================
