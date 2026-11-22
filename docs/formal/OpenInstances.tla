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
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS NInst, NSess, MaxK,         \* instances, sessions, requests of an instance
          MaxInc, MaxJoin,            \* incarnations drawn and joins sent, per instance
          MaxCliCrash, MaxSrvCrash, MaxExpire,
          MaxMembers,                 \* the bound on members
          Dup,                        \* messages repeat
          Resend,                     \* the client sends again and acknowledges
          Handshake,                  \* a join only offers; a request confirms
          Match,                      \* an offer is taken only as the answer to the join waited on
          Fence,                      \* an unknown incarnation is refused and told
          Records,                    \* completion records and the mark (else a window of one)
          Recheck,                    \* a session judges incarnation and mark again as it runs
          GcDurable,                  \* records are dropped by the durable mark
          Evict,                      \* a live member's records may be dropped
          SameOffer                   \* a join repeated is offered the same incarnation

Inst == 1..NInst
Sess == 0..(NSess - 1)
Ks   == 1..MaxK
Target(k) == k % NSess

VARIABLES member, offer, ofor, drawn, low, lowD, rec, inbox, effects,   \* the server
          cinc, cjn, cwait, cnext, slot, status, got,             \* the instances
          net, nCli, nSrv, nExp

srv  == <<member, offer, ofor, drawn, low, lowD, rec, inbox, effects>>
cli  == <<cinc, cjn, cwait, cnext, slot, status, got>>
cnt  == <<nCli, nSrv, nExp>>
vars == <<member, offer, ofor, drawn, low, lowD, rec, inbox, effects,
          cinc, cjn, cwait, cnext, slot, status, got, net, nCli, nSrv, nExp>>

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
    /\ net = {} /\ nCli = 0 /\ nSrv = 0 /\ nExp = 0

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

CReply(m) ==
    /\ m.kind = "reply" /\ cinc[m.i] = m.inc /\ Waiting(m.i, m.seq)
    /\ status' = [status EXCEPT ![m.i][slot[m.i][m.seq]] = "done"]
    /\ got' = [got EXCEPT ![m.i][slot[m.i][m.seq]] = m.k]
    /\ UNCHANGED <<srv, cinc, cjn, cwait, cnext, slot, net, cnt>>

Unknown(i) == [k \in Ks |-> IF status[i][k] = "waiting" THEN "unknown" ELSE status[i][k]]

CFenced(m) ==
    /\ m.kind = "fenced" /\ m.inc # 0 /\ cinc[m.i] = m.inc
    /\ status' = [status EXCEPT ![m.i] = Unknown(m.i)]
    /\ cinc' = [cinc EXCEPT ![m.i] = 0]
    /\ UNCHANGED <<srv, cjn, cwait, cnext, slot, got, net, cnt>>

\* An instance that keeps nothing of its own forgets its incarnation, its numbers and what it
\* waits on; its identity and its credential stay.
CCrash(i) ==
    /\ nCli < MaxCliCrash /\ nCli' = nCli + 1
    /\ status' = [status EXCEPT ![i] = Unknown(i)]
    /\ cinc' = [cinc EXCEPT ![i] = 0] /\ cwait' = [cwait EXCEPT ![i] = FALSE]
    /\ UNCHANGED <<srv, cjn, cnext, slot, got, net, nSrv, nExp>>

-----------------------------------------------------------------------------
\* The server. What is taken from the network stays in it when messages repeat.

Left(m) == IF Dup THEN net ELSE net \ {m}

\* The answer to a join. `ofor` is the join the offer stands for: a join repeated while its offer
\* stands is offered the same incarnation again.
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

Room(i) == member[i] # 0 \/ Cardinality(Members) < MaxMembers

\* The edge: judge the incarnation, take the mark, hand the request to its session.
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

Of(r, m) == r.i = m.i /\ r.inc = m.inc /\ r.seq = m.seq

\* A session runs a request: the effect and its record are one write.
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
    /\ UNCHANGED <<ofor, drawn, low, lowD, rec, inbox, effects, cli, net, nCli, nSrv>>

SCrash ==
    /\ nSrv < MaxSrvCrash /\ nSrv' = nSrv + 1
    /\ low' = lowD /\ inbox' = [s \in Sess |-> {}]
    /\ UNCHANGED <<member, offer, ofor, drawn, lowD, rec, effects, cli, net, nCli, nExp>>

-----------------------------------------------------------------------------
Next ==
    \/ \E i \in Inst : CJoin(i) \/ CAck(i) \/ CCrash(i) \/ SCommitMark(i) \/ SExpire(i)
    \/ \E i \in Inst, k \in Ks : CSend(i, k) \/ CAgain(i, k)
    \/ \E m \in net : COffer(m) \/ CReply(m) \/ CFenced(m) \/ SJoin(m) \/ SEdge(m)
    \/ \E s \in Sess : SDrop(s) \/ SEvict(s) \/ \E m \in inbox[s] : SRun(s, m)
    \/ SCrash

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

=============================================================================
