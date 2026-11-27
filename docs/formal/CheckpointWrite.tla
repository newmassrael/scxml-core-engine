---------------------------- MODULE CheckpointWrite ----------------------------
(***************************************************************************)
(* The composition of docs/adr/0115, D12: OpenInstances (docs/adr/0113)    *)
(* and EventJournal (docs/adr/0114) over the one write they meet in. A     *)
(* session runs a request in a transaction of the process that holds its   *)
(* key's lease, and the write that ends the transaction makes durable      *)
(* together the session's state, the journal's entry, the member's         *)
(* completion record and the answer its outbox sends. The store refuses    *)
(* that write whole where the lease's fence is another's (docs/adr/0114,   *)
(* D2) or where it no longer holds the incarnation the run was made under  *)
(* (docs/adr/0113, D5: Write::under).                                      *)
(*                                                                         *)
(* Each protocol is instantiated, not copied: every step of OpenInstances  *)
(* but the run that takes an effect, every step of EventJournal but its    *)
(* transaction, and the write in place of both, so that a property of      *)
(* either is checked where the other moves too. A run that writes nothing  *)
(* (a repeat answered from its record, a request fenced or below the mark) *)
(* is OpenInstances' own step: it reads the store and writes nothing.      *)
(*                                                                         *)
(* Time is an input where a lease is judged (D12). now is the time, and    *)
(* each process's clock lags it. A process takes a key once its clock has  *)
(* passed the deadline of the lease it ends, and is given a lease of Term  *)
(* from its clock; it runs the key while its own clock is before its own   *)
(* deadline. A holder whose clock lags, or that pauses, may write after    *)
(* its deadline, and the fence refuses that write. So no property of the   *)
(* two protocols is one of a clock, and the one property of a deadline,    *)
(* TakenWhenDue, is one of time: it stands on the clocks lagging, and on   *)
(* nothing else. A member's lease, and the retention of the records it     *)
(* ends, are judged at any moment (SExpire, SDrop), of which every clock's *)
(* judgement is one; the journal keeps every entry (docs/adr/0114 has no   *)
(* retention).                                                             *)
(*                                                                         *)
(* The parts, each a constant TRUE in the protocol, and each taken out by  *)
(* a run that must find what it breaks:                                    *)
(*   LeaseFence      the write is judged by the lease's fence              *)
(*   JournalInWrite  the entry is in the write                             *)
(*   RecordInWrite   the completion record is in the write                 *)
(*   OutboxInWrite   the answer leaves from what the write kept            *)
(*   ClockLags       a clock never runs ahead of time                      *)
(*   Release         a process that stops ends its transactions            *)
(*                                                                         *)
(* ran is a history variable: the request each write ran, which the        *)
(* mapping to the promise needs and no process keeps (docs/adr/0115, D2).  *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   What OpenInstances and EventJournal take as given.                    *)
(*   A run is decided from the records the store held when its             *)
(*     transaction began, and its write is atomic and durable, its fence   *)
(*     and its incarnation judged inside it.                               *)
(*   A clock lags time and never runs back for one process (ClockLags).    *)
(*   LiveSpec: a message arrives, and what can be done is done.            *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS
    \* the members: instances, sessions, requests of an instance, incarnations drawn, joins sent,
    \* expiries
    \* @type: Int;
    NInst,
    \* @type: Int;
    NSess,
    \* @type: Int;
    MaxK,
    \* @type: Int;
    MaxInc,
    \* @type: Int;
    MaxJoin,
    \* @type: Int;
    MaxExpire,
    \* messages repeat; the client sends again and acknowledges
    \* @type: Bool;
    Dup,
    \* @type: Bool;
    Resend,
    \* the journal: processes, inputs of a session, transactions under way, stops
    \* @type: Int;
    NProc,
    \* @type: Int;
    MaxInput,
    \* @type: Int;
    MaxOpen,
    \* @type: Int;
    MaxCrash,
    \* how long a lease lasts, on the clock of the process that takes it
    \* @type: Int;
    Term,
    \* the write is judged by the lease's fence
    \* @type: Bool;
    LeaseFence,
    \* the entry is in the write
    \* @type: Bool;
    JournalInWrite,
    \* the completion record is in the write
    \* @type: Bool;
    RecordInWrite,
    \* the answer leaves from what the write kept
    \* @type: Bool;
    OutboxInWrite,
    \* a clock never runs ahead of time
    \* @type: Bool;
    ClockLags,
    \* a process that stops ends the transactions it had under way
    \* @type: Bool;
    Release

VARIABLES
    \* OpenInstances' (its module says what each is)
    \* @type: Int -> Int;
    member,
    \* @type: Int -> Int;
    offer,
    \* @type: Int -> Int;
    ofor,
    \* @type: Int -> Int;
    drawn,
    \* @type: Int -> Int;
    low,
    \* @type: Int -> Int;
    lowD,
    \* the store's completion records, per session
    \* @type: Int -> Set({i: Int, inc: Int, seq: Int, k: Int});
    rec,
    \* @type: Int -> Set({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int});
    inbox,
    \* @type: Int -> (Int -> Int);
    effects,
    \* @type: Int -> Int;
    cinc,
    \* @type: Int -> Int;
    cjn,
    \* @type: Int -> Bool;
    cwait,
    \* @type: Int -> Int;
    cnext,
    \* @type: Int -> (Int -> Int);
    slot,
    \* @type: Int -> (Int -> Str);
    status,
    \* @type: Int -> (Int -> Int);
    got,
    \* @type: Set({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int});
    net,
    \* @type: Int;
    nCli,
    \* @type: Int;
    nSrv,
    \* @type: Int;
    nExp,
    \* @type: Int;
    nForge,
    \* EventJournal's (its module says what each is)
    \* @type: Int -> Int;
    fence,
    \* @type: Int -> (Int -> Int);
    hold,
    \* @type: Int -> (Int -> Int);
    mem,
    \* @type: Int -> Int;
    st,
    \* @type: Set({pos: Int, s: Int, n: Int, at: Int});
    log,
    \* @type: Set({p: Int, s: Int, n: Int, pos: Int, f: Int, at: Int});
    open,
    \* @type: Int;
    nextpos,
    \* @type: Int -> Int;
    clock,
    \* @type: Set({pos: Int, s: Int, n: Int, at: Int});
    pend,
    \* @type: Int;
    cursor,
    \* @type: <<Int, Int>> -> Int;
    applied,
    \* @type: Int;
    staged,
    \* @type: Set({r: Int, held: Set({pos: Int, s: Int, n: Int, at: Int})});
    asked,
    \* @type: Int;
    crashes,
    \* the time
    \* @type: Int;
    now,
    \* due[j][f]: the deadline of the lease key j was taken under with fence f
    \* @type: Int -> (Int -> Int);
    due,
    \* runs[p][j]: the request the transaction of process p on key j runs
    \* @type: Int -> (Int -> {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int});
    runs,
    \* the history variable: ran[<<j, n>>], the request the n-th write of key j ran
    \* @type: <<Int, Int>> -> <<Int, Int>>;
    ran,
    \* without RecordInWrite: the records a process has still to write, each of its session
    \* @type: Set({p: Int, s: Int, r: {i: Int, inc: Int, seq: Int, k: Int}});
    prec

\* OpenInstances with every part in, but for its sizes. A crash of an instance and the server's
\* loss of what it holds in memory are its own steps, judged by its runs, and touch no write; a stop
\* here is a process of the journal's. No intermediary changes a message: the parts that sign are
\* OpenInstances' to show.
OI == INSTANCE OpenInstances WITH
    NInst <- NInst, NSess <- NSess, MaxK <- MaxK, MaxInc <- MaxInc, MaxJoin <- MaxJoin,
    MaxCliCrash <- 0, MaxSrvCrash <- 0, MaxExpire <- MaxExpire, MaxMembers <- NInst,
    Dup <- Dup, Resend <- Resend, Handshake <- TRUE, Match <- TRUE, Fence <- TRUE,
    Records <- TRUE, Recheck <- TRUE, GcDurable <- TRUE, Evict <- FALSE, SameOffer <- TRUE,
    SignInc <- TRUE, SignMark <- TRUE, SignOffer <- TRUE, SignFence <- TRUE, Limit <- TRUE,
    MaxForge <- 0

\* EventJournal over the same sessions, its fence, the place of its entry and its release the
\* parts of the write. Its earlier moment is a position, which EventJournal's runs show.
EJ == INSTANCE EventJournal WITH
    NProc <- NProc, NSess <- NSess, MaxInput <- MaxInput, MaxOpen <- MaxOpen,
    MaxCrash <- MaxCrash, Fence <- LeaseFence, Atomic <- JournalInWrite, Horizon <- TRUE,
    CursorAtomic <- TRUE, ByPosition <- TRUE, Release <- Release

\* A session of OpenInstances is numbered from 0, and its key in EventJournal from 1.
KeyOf(s) == s + 1
SessOf(j) == j - 1

\* The moments EventJournal's clocks count, and the leases it lets a key be taken under.
Moments == 1..2
Leases == 1..2

\* The variables of each protocol, as its module lists them, and the write's own. They are named here
\* and not through the instances, so that a step that leaves them unchanged gives each its value.
memberVars == <<member, offer, ofor, drawn, low, lowD, rec, inbox, effects, cinc, cjn, cwait, cnext,
                slot, status, got, net, nCli, nSrv, nExp, nForge>>
clientVars == <<cinc, cjn, cwait, cnext, slot, status, got>>
countVars == <<nCli, nSrv, nExp, nForge>>
journalVars == <<fence, hold, mem, st, log, open, nextpos, clock, pend, cursor, applied, staged, asked, crashes>>
own == <<now, due, runs, ran, prec>>
vars == <<memberVars, journalVars, own>>

\* What a transaction runs where it runs nothing, and what a write that was not made ran.
\* @type: {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int};
NoRun == OI!Msg("none", 0, 0, 0, 0, 0, 0)
Unwritten == <<0, 0>>
Writes == EJ!Sess \X (1..MaxInput)

Init ==
    /\ OI!Init
    /\ EJ!Init
    /\ now = 1
    /\ due = [j \in EJ!Sess |-> [f \in Leases |-> 0]]
    /\ runs = [p \in EJ!Procs |-> [j \in EJ!Sess |-> NoRun]]
    /\ ran = [w \in Writes |-> Unwritten]
    /\ prec = {}

\* A step of one protocol leaves the other's variables, and the write's, as they are.
\* @type: (Bool) => Bool;
OfMembers(A) == A /\ UNCHANGED <<journalVars, own>>
\* @type: (Bool) => Bool;
OfJournal(A) == A /\ UNCHANGED <<memberVars, own>>

\* Every step of OpenInstances but the run that takes an effect, which only the write takes.
Members == OfMembers(OI!Next /\ effects' = effects)

\* The steps of EventJournal that are no part of a transaction: a pending entry appended, and the
\* follower. A question as of a moment reads the journal and writes nothing either protocol reads;
\* EventJournal's runs judge it.
Journal ==
    \/ \E e \in pend : OfJournal(EJ!AppendPending(e))
    \/ OfJournal(EJ!Follow)
    \/ OfJournal(EJ!KeepCursor)

-----------------------------------------------------------------------------
\* Time and the leases.

Time ==
    /\ now + 1 \in Moments
    /\ now' = now + 1
    /\ UNCHANGED <<memberVars, journalVars, due, runs, ran, prec>>

\* A process's clock goes on, and stays behind the time.
Clock(p) ==
    /\ EJ!Tick(p)
    /\ ClockLags => clock'[p] <= now
    /\ UNCHANGED <<memberVars, now, due, runs, ran, prec>>

\* The deadline of the lease key j is held under, as the store records it.
Due(j) == IF fence[j] = 0 THEN 0 ELSE due[j][fence[j]]

\* Process p holds key j by its own clock: a lease it took, before the deadline it was given.
Holds(p, j) == hold[p][j] # 0 /\ clock[p] < due[j][hold[p][j]]

\* p takes key j once its clock has passed the deadline of the lease that holds it, and is given
\* one of Term from its clock. A process takes no key it is writing under: it ends that write first.
Take(p, j) ==
    /\ clock[p] >= Due(j)
    /\ ~ \E t \in open : t.p = p /\ t.s = j
    /\ EJ!Take(p, j)
    /\ due' = [due EXCEPT ![j][fence[j] + 1] = clock[p] + Term]
    /\ UNCHANGED <<memberVars, now, runs, ran, prec>>

\* A process stops, as in EventJournal, and what it had still to write is gone with it.
Crash(p) ==
    /\ EJ!Crash(p)
    /\ runs' = [runs EXCEPT ![p] = [j \in EJ!Sess |-> NoRun]]
    /\ prec' = {x \in prec : x.p # p}
    /\ UNCHANGED <<memberVars, now, due, ran>>

-----------------------------------------------------------------------------
\* The write.

\* The completion record of the run of m, as OpenInstances' run writes it.
\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => {i: Int, inc: Int, seq: Int, k: Int};
Completion(m) == [i |-> m.i, inc |-> m.inc, seq |-> m.seq, k |-> m.k]

\* The answer the run of m sends.
\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int};
Reply(m) == OI!Msg("reply", m.i, m.inc, m.seq, m.k, 0, 0)

\* The store holds the incarnation m was sent under.
\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
Under(m) == member[m.i] = m.inc

\* Run in session s, m takes an effect: the branch of OpenInstances' run that writes, its
\* incarnation held, at or above the mark, and no record of it.
\* @type: (Int, {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
Fresh(s, m) == Under(m) /\ m.seq >= low[m.i] /\ ~ \E r \in rec[s] : OI!Of(r, m)

\* p begins the transaction that runs m: EventJournal's, under the lease p holds by its clock.
\* m stays in its inbox until a write takes it out, so a write refused leaves it to the next run.
\* @type: (Int, Int, {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
Begin(p, s, m) ==
    /\ Holds(p, KeyOf(s))
    /\ Fresh(s, m)
    /\ EJ!Begin(p, KeyOf(s))
    /\ runs' = [runs EXCEPT ![p][KeyOf(s)] = m]
    /\ net' = IF OutboxInWrite THEN net ELSE net \cup {Reply(m)}
    /\ UNCHANGED <<member, offer, ofor, drawn, low, lowD, rec, inbox, effects, clientVars, countVars,
                   now, due, ran, prec>>

\* The lease's fence refuses the write of t: the key is another's now.
\* @type: ({p: Int, s: Int, n: Int, pos: Int, f: Int, at: Int}) => Bool;
Fenced(t) == LeaseFence /\ t.f # fence[t.s]

\* The one write that ends t. It is refused whole by the lease's fence (EventJournal's judgement)
\* or by the incarnation (Write::under); else it makes the session's state and the entry durable
\* (EventJournal's commit) and, in the same write, the effect, the record and the answer.
\* @type: ({p: Int, s: Int, n: Int, pos: Int, f: Int, at: Int}) => Bool;
Write(t) ==
    LET m == runs[t.p][t.s]
        s == SessOf(t.s)
    IN
    /\ t.p # EJ!Nobody
    /\ runs' = [runs EXCEPT ![t.p][t.s] = NoRun]
    /\ IF Fenced(t)
       THEN /\ EJ!Commit(t)
            /\ UNCHANGED <<memberVars, now, due, ran, prec>>
       ELSE IF ~Under(m)
       THEN /\ open' = open \ {t}
            /\ UNCHANGED <<memberVars, fence, hold, mem, st, log, nextpos, clock, pend, cursor,
                           applied, staged, asked, crashes, now, due, ran, prec>>
       ELSE /\ EJ!Commit(t)
            /\ effects' = [effects EXCEPT ![m.i][m.k] = @ + 1]
            /\ inbox' = [inbox EXCEPT ![s] = @ \ {m}]
            /\ ran' = [ran EXCEPT ![<<t.s, t.n>>] = <<m.i, m.k>>]
            /\ IF RecordInWrite
               THEN /\ rec' = [rec EXCEPT ![s] = @ \cup {Completion(m)}]
                    /\ UNCHANGED prec
               ELSE /\ prec' = prec \cup {[p |-> t.p, s |-> s, r |-> Completion(m)]}
                    /\ UNCHANGED rec
            /\ net' = IF OutboxInWrite THEN net \cup {Reply(m)} ELSE net
            /\ UNCHANGED <<member, offer, ofor, drawn, low, lowD, clientVars, countVars, now, due>>

\* Without RecordInWrite: the record kept by a write of its own, after the one that took the effect.
\* @type: ({p: Int, s: Int, r: {i: Int, inc: Int, seq: Int, k: Int}}) => Bool;
KeepRecord(x) ==
    /\ rec' = [rec EXCEPT ![x.s] = @ \cup {x.r}]
    /\ prec' = prec \ {x}
    /\ UNCHANGED <<member, offer, ofor, drawn, low, lowD, inbox, effects, clientVars, net, countVars,
                   journalVars, now, due, runs, ran>>

-----------------------------------------------------------------------------
Next ==
    \/ Members
    \/ Journal
    \/ \E p \in EJ!Procs : Clock(p) \/ Crash(p)
    \/ \E p \in EJ!Procs, j \in EJ!Sess : Take(p, j)
    \/ \E p \in EJ!Procs, s \in OI!Sess : \E m \in inbox[s] : Begin(p, s, m)
    \/ \E t \in open : Write(t)
    \/ \E x \in prec : KeepRecord(x)
    \/ Time

Spec == Init /\ [][Next]_vars

\* What both protocols' progress stands on, of the steps either may take: a message arrives, a key
\* whose lease has passed is taken, a run is begun and its write made, and a follower goes on. No
\* clock and no time is asked to go on.
Fair ==
    /\ \A i \in OI!Inst :
         /\ WF_vars(OfMembers(OI!CJoin(i)))
         /\ WF_vars(OfMembers(\E m \in net : m.i = i /\ OI!COffer(m)))
         /\ WF_vars(OfMembers(\E m \in net : m.i = i /\ OI!CReply(m)))
         /\ WF_vars(OfMembers(\E m \in net : m.i = i /\ OI!CFenced(m)))
         /\ WF_vars(OfMembers(\E m \in net : m.i = i /\ OI!SJoin(m)))
         /\ WF_vars(OfMembers(\E m \in net : m.i = i /\ OI!SEdge(m)))
    /\ \A i \in OI!Inst, k \in OI!Ks : WF_vars(OfMembers(OI!CSend(i, k)))
    /\ \A s \in OI!Sess :
         WF_vars(OfMembers(\E m \in inbox[s] : OI!SRun(s, m) /\ effects' = effects))
    /\ \A p \in EJ!Procs, j \in EJ!Sess : WF_vars(Take(p, j))
    /\ \A p \in EJ!Procs, s \in OI!Sess : WF_vars(\E m \in inbox[s] : Begin(p, s, m))
    /\ WF_vars(\E t \in open : Write(t))
    /\ WF_vars(OfJournal(EJ!Follow))
    /\ WF_vars(OfJournal(EJ!KeepCursor))

LiveSpec == Spec /\ Fair

-----------------------------------------------------------------------------
\* Each protocol's properties, where the other moves too.
AtMostOnce == OI!AtMostOnce
Answered == OI!Answered
AppendOnce == EJ!AppendOnce
NothingLost == EJ!NothingLost
Ends == OI!Ends
Followed == EJ!Followed
CaughtUp == EJ!CaughtUp

\* A key passes to another owner only once the lease its holder holds has ended in time, by the
\* deadline the holder was given last, at its grant or at its renewal; or once the holder holds it no
\* more, having given it up or stopped. A process that takes the key again under its own name holds
\* it still, and passes it to nobody. A property of a deadline, and so of time: no effect, answer or
\* entry depends on it.
TakenWhenDue == [][\A j \in EJ!Sess : fence'[j] # fence[j] =>
    \A p \in EJ!Procs :
        (fence[j] # 0 /\ hold[p][j] = fence[j] /\ hold'[p][j] # fence'[j]) => now >= due[j][fence[j]]]_vars

-----------------------------------------------------------------------------
\* What the composition implements (docs/adr/0115, D2): the two promises over one server, and what
\* ties them. The promise's requests and its client are OpenInstances', its journal and reader
\* EventJournal's, and the entry of an effect the write that ran it.

\* The entry the effect of r made: the write that ran r.
\* @type: (<<Int, Int>>) => <<Int, Int>>;
EntryOf(r) == IF \E w \in Writes : ran[w] = r THEN CHOOSE w \in Writes : ran[w] = r ELSE Unwritten

Promise == INSTANCE CheckpointService WITH
    Requests <- OI!Requests,
    Streams <- EJ!Sess,
    MaxEntry <- MaxInput,
    status <- [r \in OI!Requests |-> status[r[1]][r[2]]],
    effect <- [r \in OI!Requests |-> effects[r[1]][r[2]]],
    faulted <- nCli + nExp > 0,
    journal <- EJ!ByPos(EJ!Visible),
    taken <- Cardinality({e \in log : e.pos <= cursor}),
    did <- applied,
    entry <- [r \in OI!Requests |-> EntryOf(r)]

Refines == Promise!Spec

=============================================================================
