--------------------------- MODULE OpenInstances ---------------------------
(***************************************************************************)
(* The protocol of ADR 0113: a server that binds instances of a machine    *)
(* nobody listed, forgets them, and still runs a request at most once.     *)
(*                                                                         *)
(* An instance joins and is OFFERED an incarnation; its first request      *)
(* under the offer CONFIRMS it (a three-way handshake, so a join that      *)
(* arrives late changes nothing). It numbers its requests within the       *)
(* incarnation. The server spreads them over sessions, and a session keeps *)
(* a COMPLETION RECORD (incarnation, number, answer) written with the      *)
(* effect. Every request carries the instance's LOW-WATER mark, below      *)
(* which it has every answer; a record below the mark is dropped, and a    *)
(* request below it is not run. A membership expires at any moment; what   *)
(* names an incarnation the server does not hold is FENCED, and the        *)
(* instance learns that the outcome of what it waited on is unknown.       *)
(*                                                                         *)
(* The network loses, repeats, delays and reorders: with Dup a message     *)
(* once sent can arrive at any later moment, any number of times, or never.*)
(* Each constant that is a part of the protocol can be switched off, and   *)
(* TLC must then find a violation.                                         *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Authentication proves which instance speaks.                          *)
(*   What a party signed cannot be changed or made by another; what a      *)
(*     message carries that is not signed can be (the Sign parts, D4).     *)
(*   An incarnation number is drawn once and never given twice, across a   *)
(*     restart.                                                            *)
(*   A record, a membership and the mark of a commit are durable once      *)
(*     written.                                                            *)
(*   LiveSpec: a message arrives, and what can be done is done.            *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS
    \* instances
    \* @type: Int;
    NInst,
    \* sessions
    \* @type: Int;
    NSess,
    \* requests of an instance
    \* @type: Int;
    MaxK,
    \* incarnations drawn, per instance
    \* @type: Int;
    MaxInc,
    \* joins sent, per instance
    \* @type: Int;
    MaxJoin,
    \* crashes of an instance
    \* @type: Int;
    MaxCliCrash,
    \* crashes of the server
    \* @type: Int;
    MaxSrvCrash,
    \* expiries
    \* @type: Int;
    MaxExpire,
    \* the bound on members
    \* @type: Int;
    MaxMembers,
    \* messages repeat
    \* @type: Bool;
    Dup,
    \* the client sends again and acknowledges
    \* @type: Bool;
    Resend,
    \* a join only offers; a request confirms
    \* @type: Bool;
    Handshake,
    \* an offer is taken only as the answer to the join waited on
    \* @type: Bool;
    Match,
    \* an unknown incarnation is refused and told
    \* @type: Bool;
    Fence,
    \* completion records and the mark (else a window of one)
    \* @type: Bool;
    Records,
    \* a session judges incarnation and mark again as it runs
    \* @type: Bool;
    Recheck,
    \* records are dropped by the durable mark
    \* @type: Bool;
    GcDurable,
    \* a live member's records may be dropped
    \* @type: Bool;
    Evict,
    \* a join repeated is offered the same incarnation
    \* @type: Bool;
    SameOffer,
    \* a request's incarnation is signed
    \* @type: Bool;
    SignInc,
    \* a request's low-water mark is signed
    \* @type: Bool;
    SignMark,
    \* the incarnation an offer names is signed
    \* @type: Bool;
    SignOffer,
    \* the answer that fences is signed
    \* @type: Bool;
    SignFence,
    \* no member is taken past the bound
    \* @type: Bool;
    Limit,
    \* messages an intermediary changes or makes
    \* @type: Int;
    MaxForge

Inst == 1..NInst
Sess == 0..(NSess - 1)
Ks   == 1..MaxK
Target(k) == k % NSess

VARIABLES
    \* the server: the member, the incarnation offered, the join it answers, the last drawn
    \* @type: Int -> Int;
    member,
    \* @type: Int -> Int;
    offer,
    \* @type: Int -> Int;
    ofor,
    \* @type: Int -> Int;
    drawn,
    \* the low-water mark the server has seen, and the one its store holds
    \* @type: Int -> Int;
    low,
    \* @type: Int -> Int;
    lowD,
    \* each session's completion records and its inbox
    \* @type: Int -> Set({i: Int, inc: Int, seq: Int, k: Int});
    rec,
    \* @type: Int -> Set({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int});
    inbox,
    \* how many times each request took effect
    \* @type: Int -> (Int -> Int);
    effects,
    \* the instances: incarnation, joins sent, whether a join is waited on, the next number
    \* @type: Int -> Int;
    cinc,
    \* @type: Int -> Int;
    cjn,
    \* @type: Int -> Bool;
    cwait,
    \* @type: Int -> Int;
    cnext,
    \* the request each number names, its status and its answer
    \* @type: Int -> (Int -> Int);
    slot,
    \* @type: Int -> (Int -> Str);
    status,
    \* @type: Int -> (Int -> Int);
    got,
    \* the network, and the faults and forgeries so far
    \* @type: Set({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int});
    net,
    \* @type: Int;
    nCli,
    \* @type: Int;
    nSrv,
    \* @type: Int;
    nExp,
    \* @type: Int;
    nForge

srv  == <<member, offer, ofor, drawn, low, lowD, rec, inbox, effects>>
cli  == <<cinc, cjn, cwait, cnext, slot, status, got>>
\* @type: <<Int, Int, Int, Int>>;
cnt  == <<nCli, nSrv, nExp, nForge>>
vars == <<member, offer, ofor, drawn, low, lowD, rec, inbox, effects,
          cinc, cjn, cwait, cnext, slot, status, got, net, nCli, nSrv, nExp, nForge>>

\* @type: (Str, Int, Int, Int, Int, Int, Int) => {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int};
Msg(kind, i, inc, seq, k, mark, jn) ==
    [kind |-> kind, i |-> i, inc |-> inc, seq |-> seq, k |-> k, mark |-> mark, jn |-> jn]

Max(a, b) == IF a > b THEN a ELSE b
Members   == {i \in Inst : member[i] # 0}

Init ==
    /\ member = [i \in Inst |-> 0] /\ offer = [i \in Inst |-> 0] /\ ofor = [i \in Inst |-> 0] /\ drawn = [i \in Inst |-> 0]
    /\ low = [i \in Inst |-> 1] /\ lowD = [i \in Inst |-> 1]
    /\ rec = [s \in Sess |-> {}] /\ inbox = [s \in Sess |-> {}]
    /\ effects = [i \in Inst |-> [k \in Ks |-> 0]]
    /\ cinc = [i \in Inst |-> 0] /\ cjn = [i \in Inst |-> 0] /\ cwait = [i \in Inst |-> FALSE]
    /\ cnext = [i \in Inst |-> 1] /\ slot = [i \in Inst |-> [q \in Ks |-> 0]]
    /\ status = [i \in Inst |-> [k \in Ks |-> "new"]]
    /\ got = [i \in Inst |-> [k \in Ks |-> 0]]
    /\ net = {} /\ nCli = 0 /\ nSrv = 0 /\ nExp = 0 /\ nForge = 0

-----------------------------------------------------------------------------
\* The instances.

Waiting(i, q) == slot[i][q] # 0 /\ status[i][slot[i][q]] = "waiting"

\* The lowest number whose answer the instance has not.
Mark(i) ==
    IF \E q \in Ks : Waiting(i, q)
    THEN CHOOSE q \in Ks : Waiting(i, q) /\ \A p \in Ks : Waiting(i, p) => q <= p
    ELSE cnext[i]

CJoin(i) ==
    /\ cinc[i] = 0 /\ ~cwait[i] /\ cjn[i] < MaxJoin
    /\ \E k \in Ks : status[i][k] = "new"
    /\ cjn' = [cjn EXCEPT ![i] = @ + 1] /\ cwait' = [cwait EXCEPT ![i] = TRUE]
    /\ net' = net \cup {Msg("join", i, 0, 0, 0, 0, cjn[i] + 1)}
    /\ UNCHANGED <<srv, cinc, cnext, slot, status, got, cnt>>

\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
COffer(m) ==
    /\ m.kind = "offer" /\ cinc[m.i] = 0
    /\ Match => (cwait[m.i] /\ m.jn = cjn[m.i])
    /\ cinc' = [cinc EXCEPT ![m.i] = m.inc] /\ cwait' = [cwait EXCEPT ![m.i] = FALSE]
    /\ cnext' = [cnext EXCEPT ![m.i] = 1]
    /\ slot' = [slot EXCEPT ![m.i] = [q \in Ks |-> 0]]
    /\ UNCHANGED <<srv, cjn, status, got, net, cnt>>

CSend(i, k) ==
    /\ cinc[i] # 0 /\ status[i][k] = "new" /\ cnext[i] \in Ks
    /\ slot' = [slot EXCEPT ![i][cnext[i]] = k]
    /\ status' = [status EXCEPT ![i][k] = "waiting"]
    /\ cnext' = [cnext EXCEPT ![i] = @ + 1]
    /\ net' = net \cup {Msg("req", i, cinc[i], cnext[i], k, Mark(i), 0)}
    /\ UNCHANGED <<srv, cinc, cjn, cwait, got, cnt>>

CAgain(i, q) ==
    /\ Resend /\ cinc[i] # 0 /\ Waiting(i, q)
    /\ net' = net \cup {Msg("req", i, cinc[i], q, slot[i][q], Mark(i), 0)}
    /\ UNCHANGED <<srv, cli, cnt>>

CAck(i) ==
    /\ Resend /\ cinc[i] # 0
    /\ net' = net \cup {Msg("ack", i, cinc[i], 0, 0, Mark(i), 0)}
    /\ UNCHANGED <<srv, cli, cnt>>

\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
CReply(m) ==
    /\ m.kind = "reply" /\ cinc[m.i] = m.inc /\ Waiting(m.i, m.seq)
    /\ status' = [status EXCEPT ![m.i][slot[m.i][m.seq]] = "done"]
    /\ got' = [got EXCEPT ![m.i][slot[m.i][m.seq]] = m.k]
    /\ UNCHANGED <<srv, cinc, cjn, cwait, cnext, slot, net, cnt>>

Unknown(i) == [k \in Ks |-> IF status[i][k] = "waiting" THEN "unknown" ELSE status[i][k]]

\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
CFenced(m) ==
    /\ m.kind = "fenced" /\ m.inc # 0 /\ cinc[m.i] = m.inc
    /\ status' = [status EXCEPT ![m.i] = Unknown(m.i)]
    /\ cinc' = [cinc EXCEPT ![m.i] = 0]
    /\ UNCHANGED <<srv, cjn, cwait, cnext, slot, got, net, cnt>>

\* An instance crashes, with or without a checkpoint of its own: it forgets its incarnation and
\* never speaks under it again, every request it waits on has an unknown outcome, and a request it
\* never numbered stays new, to be numbered under the next incarnation; its identity and its
\* credential stay. A restore from its own checkpoint is this step (docs/adr/0113, D4), in the
\* run where every request the checkpoint holds was sent before the crash: each may have left once
\* the checkpoint was durable, and one the checkpoint does not hold never left.
CCrash(i) ==
    /\ nCli < MaxCliCrash /\ nCli' = nCli + 1
    /\ status' = [status EXCEPT ![i] = Unknown(i)]
    /\ cinc' = [cinc EXCEPT ![i] = 0] /\ cwait' = [cwait EXCEPT ![i] = FALSE]
    /\ UNCHANGED <<srv, cjn, cnext, slot, got, net, nSrv, nExp, nForge>>

-----------------------------------------------------------------------------
\* The server. What is taken from the network stays in it when messages repeat.

\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Set({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int});
Left(m) == IF Dup THEN net ELSE net \ {m}

\* The answer to a join. `ofor` is the join the offer stands for: a join repeated while its offer
\* stands is offered the same incarnation again.
\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
SJoin(m) ==
    /\ m.kind = "join"
    /\ LET again == SameOffer /\ Handshake /\ offer[m.i] # 0 /\ ofor[m.i] = m.jn
           inc   == IF again THEN offer[m.i] ELSE drawn[m.i] + 1
       IN  /\ again \/ drawn[m.i] < MaxInc
           /\ drawn' = IF again THEN drawn ELSE [drawn EXCEPT ![m.i] = @ + 1]
           /\ net' = Left(m) \cup {Msg("offer", m.i, inc, 0, 0, 0, m.jn)}
           /\ IF Handshake
              THEN /\ offer' = [offer EXCEPT ![m.i] = inc] /\ ofor' = [ofor EXCEPT ![m.i] = m.jn]
                   /\ UNCHANGED <<member, low, lowD>>
              ELSE /\ member' = [member EXCEPT ![m.i] = inc]
                   /\ low' = [low EXCEPT ![m.i] = 1] /\ lowD' = [lowD EXCEPT ![m.i] = 1]
                   /\ UNCHANGED <<offer, ofor>>
    /\ UNCHANGED <<rec, inbox, effects, cli, cnt>>

Room(i) == ~Limit \/ member[i] # 0 \/ Cardinality(Members) < MaxMembers

\* The edge: judge the incarnation, take the mark, hand the request to its session.
\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
SEdge(m) ==
    /\ m.kind \in {"req", "ack"}
    /\ LET i       == m.i
           current == member[i] = m.inc
           confirm == ~current /\ offer[i] = m.inc
           anew    == ~current /\ ~confirm /\ ~Fence /\ m.kind = "req"
           base    == IF current THEN low[i] ELSE 1
           mark    == Max(base, m.mark)
           admitted == m.kind = "req" /\ (Records => m.seq >= mark)
       IN  IF current \/ ((confirm \/ anew) /\ Room(i))
           THEN /\ member' = [member EXCEPT ![i] = m.inc]
                /\ offer' = IF confirm THEN [offer EXCEPT ![i] = 0] ELSE offer
                /\ low' = [low EXCEPT ![i] = mark]
                /\ lowD' = IF current THEN lowD ELSE [lowD EXCEPT ![i] = 1]
                /\ inbox' = IF admitted
                            THEN [inbox EXCEPT ![Target(m.k)] = @ \cup {m}] ELSE inbox
                /\ net' = Left(m)
           ELSE /\ ~confirm /\ ~anew
                /\ net' = Left(m) \cup {Msg("fenced", i, m.inc, 0, 0, 0, 0)}
                /\ UNCHANGED <<member, offer, low, lowD, inbox>>
    /\ UNCHANGED <<ofor, drawn, rec, effects, cli, cnt>>

SCommitMark(i) ==
    /\ lowD[i] # low[i]
    /\ lowD' = [lowD EXCEPT ![i] = low[i]]
    /\ UNCHANGED <<member, offer, ofor, drawn, low, rec, inbox, effects, cli, net, cnt>>

\* @type: ({i: Int, inc: Int, seq: Int, k: Int}, {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
Of(r, m) == r.i = m.i /\ r.inc = m.inc /\ r.seq = m.seq

\* A session runs a request: the effect and its record are one write, and that write is refused when
\* the store no longer holds the incarnation, which is the judgement `gone` makes (D5).
\* @type: (Int, {kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
SRun(s, m) ==
    /\ m \in inbox[s]
    /\ inbox' = [inbox EXCEPT ![s] = @ \ {m}]
    /\ LET gone  == Recheck /\ member[m.i] # m.inc
           below == Recheck /\ Records /\ m.seq < low[m.i]
           held  == {r \in rec[s] : Of(r, m)}
       IN  IF gone
           THEN /\ net' = net \cup {Msg("fenced", m.i, m.inc, 0, 0, 0, 0)}
                /\ UNCHANGED <<rec, effects>>
           ELSE IF below
           THEN UNCHANGED <<rec, effects, net>>
           ELSE IF held # {}
           THEN /\ LET r == CHOOSE x \in held : TRUE
                   IN net' = net \cup {Msg("reply", m.i, m.inc, m.seq, r.k, 0, 0)}
                /\ UNCHANGED <<rec, effects>>
           ELSE /\ effects' = [effects EXCEPT ![m.i][m.k] = @ + 1]
                /\ rec' = [rec EXCEPT ![s] =
                      (IF Records THEN @ ELSE {}) \cup {[i |-> m.i, inc |-> m.inc, seq |-> m.seq, k |-> m.k]}]
                /\ net' = net \cup {Msg("reply", m.i, m.inc, m.seq, m.k, 0, 0)}
    /\ UNCHANGED <<member, offer, ofor, drawn, low, lowD, cli, cnt>>

SDrop(s) ==
    /\ Records
    /\ LET mark(i) == IF GcDurable THEN lowD[i] ELSE low[i]
           old == {r \in rec[s] : member[r.i] # r.inc \/ r.seq < mark(r.i)}
       IN old # {} /\ rec' = [rec EXCEPT ![s] = @ \ old]
    /\ UNCHANGED <<member, offer, ofor, drawn, low, lowD, inbox, effects, cli, net, cnt>>

SEvict(s) ==
    /\ Evict /\ rec[s] # {}
    /\ rec' = [rec EXCEPT ![s] = {}]
    /\ UNCHANGED <<member, offer, ofor, drawn, low, lowD, inbox, effects, cli, net, cnt>>

SExpire(i) ==
    /\ nExp < MaxExpire /\ nExp' = nExp + 1
    /\ \/ member[i] # 0 /\ member' = [member EXCEPT ![i] = 0] /\ UNCHANGED offer
       \/ offer[i] # 0 /\ offer' = [offer EXCEPT ![i] = 0] /\ UNCHANGED member
    /\ UNCHANGED <<ofor, drawn, low, lowD, rec, inbox, effects, cli, net, nCli, nSrv, nForge>>

SCrash ==
    /\ nSrv < MaxSrvCrash /\ nSrv' = nSrv + 1
    /\ low' = lowD /\ inbox' = [s \in Sess |-> {}]
    /\ UNCHANGED <<member, offer, ofor, drawn, lowD, rec, effects, cli, net, nCli, nExp, nForge>>

-----------------------------------------------------------------------------
\* An intermediary that reads an exchange changes what of it is not signed, or makes an answer that
\* nobody signed. It holds no key, so with every Sign part in place it can do nothing but what the
\* network does.
Incs == 1..MaxInc

AForge ==
    /\ nForge < MaxForge /\ nForge' = nForge + 1
    /\ \E forged \in
           (IF SignInc THEN {} ELSE
              {[m EXCEPT !.inc = c] : m \in {x \in net : x.kind \in {"req", "ack"}}, c \in Incs})
           \cup (IF SignMark THEN {} ELSE
              {[m EXCEPT !.mark = k] : m \in {x \in net : x.kind \in {"req", "ack"}}, k \in 1..(MaxK + 1)})
           \cup (IF SignOffer THEN {} ELSE
              {[m EXCEPT !.inc = c] : m \in {x \in net : x.kind = "offer"}, c \in Incs})
           \cup (IF SignFence THEN {} ELSE
              {Msg("fenced", i, c, 0, 0, 0, 0) : i \in Inst, c \in Incs}) :
         net' = net \cup {forged}
    /\ UNCHANGED <<srv, cli, nCli, nSrv, nExp>>

-----------------------------------------------------------------------------
Next ==
    \/ \E i \in Inst : CJoin(i) \/ CAck(i) \/ CCrash(i) \/ SCommitMark(i) \/ SExpire(i)
    \/ \E i \in Inst, k \in Ks : CSend(i, k) \/ CAgain(i, k)
    \/ \E m \in net : COffer(m) \/ CReply(m) \/ CFenced(m) \/ SJoin(m) \/ SEdge(m)
    \/ \E s \in Sess : SDrop(s) \/ SEvict(s) \/ \E m \in inbox[s] : SRun(s, m)
    \/ SCrash
    \/ AForge

Spec == Init /\ [][Next]_vars

\* For the liveness configuration, where a message arrives once and is not repeated.
Fair ==
    /\ \A i \in Inst : WF_vars(CJoin(i))
    /\ \A i \in Inst, k \in Ks : WF_vars(CSend(i, k))
    /\ \A i \in Inst :
         /\ WF_vars(\E m \in net : m.i = i /\ COffer(m))
         /\ WF_vars(\E m \in net : m.i = i /\ CReply(m))
         /\ WF_vars(\E m \in net : m.i = i /\ CFenced(m))
         /\ WF_vars(\E m \in net : m.i = i /\ SJoin(m))
         /\ WF_vars(\E m \in net : m.i = i /\ SEdge(m))
    /\ \A s \in Sess : WF_vars(\E m \in inbox[s] : SRun(s, m))

LiveSpec == Spec /\ Fair

-----------------------------------------------------------------------------
\* A request takes effect at most once.
AtMostOnce == \A i \in Inst, k \in Ks : effects[i][k] <= 1

\* An instance that was answered was answered for what it asked, and it happened.
Answered == \A i \in Inst, k \in Ks :
    status[i][k] = "done" => (effects[i][k] >= 1 /\ got[i][k] = k)

\* An instance that neither crashed nor was expired never loses an outcome, whatever the network
\* repeats: a late or repeated join disturbs no member.
Undisturbed ==
    (nCli = 0 /\ nExp = 0) => \A i \in Inst, k \in Ks : status[i][k] # "unknown"

\* The server holds no more members than its bound.
Bounded == Cardinality(Members) <= MaxMembers

\* Every request an instance made ends, answered or known to be of unknown outcome.
Ends == \A i \in Inst, k \in Ks : <>(status[i][k] \in {"done", "unknown"})

-----------------------------------------------------------------------------
\* What the protocol implements (docs/adr/0115, D2): the service a client may rely on, its requests
\* the instances' own and its status and effect the ones above.
Requests == Inst \X Ks
Service == INSTANCE AtMostOnceService WITH
    Requests <- Requests,
    status <- [r \in Requests |-> status[r[1]][r[2]]],
    effect <- [r \in Requests |-> effects[r[1]][r[2]]],
    faulted <- nCli + nExp > 0

Refines == Service!Spec

-----------------------------------------------------------------------------
\* At most once for every length of execution (docs/adr/0115, D3; docs/adr/0113, D4 and D5): an
\* inductive invariant that implies AtMostOnce. Init satisfies it, every step from a state that
\* satisfies it keeps it, and it implies AtMostOnce; Apalache checks the three at the parameters of a
\* configuration (models.json), over every state the invariant allows and not only the reachable
\* ones, so at those parameters no execution of any length takes a request's effect twice.
\*
\* Why: a request has ONE NAME, an incarnation and a number, wherever it is, since an instance names a
\* request once, when it first sends it, and sends it again only under the name its slot keeps; and
\* once it took effect, it is KEPT from taking it again: its session holds its record, its
\* incarnation can never be a member again (each is drawn once, above every one before, and an offer
\* is never the member), or its number is below the mark the store holds of its member, which a crash
\* does not take back and below which alone a record is dropped.

ReqStatuses == {"new", "waiting", "done", "unknown"}
Marks == 1..(MaxK + 1)

\* The messages of each kind, with the fields that kind carries.
JoinMsgs   == {Msg("join", i, 0, 0, 0, 0, j) : i \in Inst, j \in 1..MaxJoin}
OfferMsgs  == {Msg("offer", i, c, 0, 0, 0, j) : i \in Inst, c \in Incs, j \in 1..MaxJoin}
ReqMsgs    == {Msg("req", i, c, q, k, w, 0) : i \in Inst, c \in Incs, q \in Ks, k \in Ks, w \in Marks}
AckMsgs    == {Msg("ack", i, c, 0, 0, w, 0) : i \in Inst, c \in Incs, w \in Marks}
ReplyMsgs  == {Msg("reply", i, c, q, k, 0, 0) : i \in Inst, c \in Incs, q \in Ks, k \in Ks}
FencedMsgs == {Msg("fenced", i, c, 0, 0, 0, 0) : i \in Inst, c \in Incs}
Messages   == JoinMsgs \cup OfferMsgs \cup ReqMsgs \cup AckMsgs \cup ReplyMsgs \cup FencedMsgs
Completions == [i : Inst, inc : Incs, seq : Ks, k : Ks]

\* Each variable a value of its type. Its conjuncts give every variable a value (x \in S), so
\* Apalache starts the step from the invariant itself.
TypeOK ==
    /\ member \in [Inst -> 0..MaxInc]
    /\ offer \in [Inst -> 0..MaxInc]
    /\ ofor \in [Inst -> 0..MaxJoin]
    /\ drawn \in [Inst -> 0..MaxInc]
    /\ low \in [Inst -> Marks]
    /\ lowD \in [Inst -> Marks]
    /\ rec \in [Sess -> SUBSET Completions]
    /\ inbox \in [Sess -> SUBSET ReqMsgs]
    /\ effects \in [Inst -> [Ks -> 0..2]]
    /\ cinc \in [Inst -> 0..MaxInc]
    /\ cjn \in [Inst -> 0..MaxJoin]
    /\ cwait \in [Inst -> BOOLEAN]
    /\ cnext \in [Inst -> Marks]
    /\ slot \in [Inst -> [Ks -> 0..MaxK]]
    /\ status \in [Inst -> [Ks -> ReqStatuses]]
    /\ got \in [Inst -> [Ks -> 0..MaxK]]
    /\ net \in SUBSET Messages
    /\ nCli \in 0..MaxCliCrash
    /\ nSrv \in 0..MaxSrvCrash
    /\ nExp \in 0..MaxExpire
    /\ nForge \in 0..MaxForge

\* @type: ({kind: Str, i: Int, inc: Int, seq: Int, k: Int, mark: Int, jn: Int}) => Bool;
IsReq(m) == m.kind = "req"

\* An incarnation that can never be a member again: neither the member nor the one offered. A
\* member is confirmed only from the offer, and an offer is drawn above every incarnation before it.
Dead(i, c) == member[i] # c /\ offer[i] # c

\* What keeps the request k of instance i, named by incarnation c and number q, from taking effect
\* again: the record its session holds, its dead incarnation, or a number below the durable mark.
Kept(i, c, q, k) ==
    \/ \E r \in rec[Target(k)] : r.i = i /\ r.inc = c /\ r.seq = q
    \/ Dead(i, c)
    \/ member[i] = c /\ q < lowD[i]

\* Two of one request of one instance, on the network, in an inbox or as a record, carry one name.
\* @type: ({i: Int, k: Int, inc: Int, seq: Int, a}, {i: Int, k: Int, inc: Int, seq: Int, b}) => Bool;
OneName(x, y) == (x.i = y.i /\ x.k = y.k) => (x.inc = y.inc /\ x.seq = y.seq)

AtMostOnceInductive ==
    /\ TypeOK
    /\ AtMostOnce
    \* Incarnations.
    /\ \A i \in Inst :
           /\ member[i] <= drawn[i] /\ offer[i] <= drawn[i] /\ cinc[i] <= drawn[i]
           /\ offer[i] # 0 => offer[i] # member[i]
           /\ lowD[i] <= low[i]
    /\ \A m \in net : m.inc <= drawn[m.i]
    /\ \A s \in Sess : \A m \in inbox[s] : m.inc <= drawn[m.i] /\ Target(m.k) = s
    /\ \A s \in Sess : \A r \in rec[s] : r.inc <= drawn[r.i] /\ r.inc # offer[r.i]
    \* One name.
    /\ \A x, y \in net : (IsReq(x) /\ IsReq(y)) => OneName(x, y)
    /\ \A s, t \in Sess :
           /\ \A x \in inbox[s], y \in inbox[t] : OneName(x, y)
           /\ \A x \in rec[s], y \in rec[t] : OneName(x, y)
           /\ \A x \in rec[s], y \in inbox[t] : OneName(x, y)
    /\ \A s \in Sess : \A y \in net :
           IsReq(y) => /\ \A x \in inbox[s] : OneName(x, y)
                       /\ \A x \in rec[s] : OneName(x, y)
    \* The instances: a request named once, when first sent, and again only under its slot's name.
    /\ \A i \in Inst :
           /\ cinc[i] = 0 => \A k \in Ks : status[i][k] # "waiting"
           /\ \A q \in Ks : q >= cnext[i] => slot[i][q] = 0
           /\ \A p, q \in Ks : (slot[i][p] # 0 /\ slot[i][p] = slot[i][q]) => p = q
           /\ \A k \in Ks :
                  status[i][k] = "new" =>
                      /\ effects[i][k] = 0
                      /\ \A q \in Ks : slot[i][q] # k
                      /\ \A m \in net : IsReq(m) => ~(m.i = i /\ m.k = k)
                      /\ \A s \in Sess : \A m \in inbox[s] : ~(m.i = i /\ m.k = k)
                      /\ \A s \in Sess : \A r \in rec[s] : ~(r.i = i /\ r.k = k)
           /\ \A q \in Ks :
                  Waiting(i, q) =>
                      LET k == slot[i][q] IN
                      /\ \A m \in net :
                             (IsReq(m) /\ m.i = i /\ m.k = k) => (m.inc = cinc[i] /\ m.seq = q)
                      /\ \A s \in Sess : \A m \in inbox[s] :
                             (m.i = i /\ m.k = k) => (m.inc = cinc[i] /\ m.seq = q)
                      /\ \A s \in Sess : \A r \in rec[s] :
                             (r.i = i /\ r.k = k) => (r.inc = cinc[i] /\ r.seq = q)
                      /\ effects[i][k] >= 1 => Kept(i, cinc[i], q, k)
    \* Kept.
    /\ \A m \in net : (IsReq(m) /\ effects[m.i][m.k] >= 1) => Kept(m.i, m.inc, m.seq, m.k)
    /\ \A s \in Sess : \A m \in inbox[s] : effects[m.i][m.k] >= 1 => Kept(m.i, m.inc, m.seq, m.k)

=============================================================================
