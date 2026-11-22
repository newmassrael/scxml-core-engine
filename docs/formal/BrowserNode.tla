---------------------------- MODULE BrowserNode ----------------------------
(***************************************************************************)
(* The client of ADR 0117: several pages of one browser, which share one   *)
(* store and one key and are one instance of ADR 0113.                     *)
(*                                                                         *)
(* The page that holds the lock is the node. The lock may be taken while   *)
(* the page that held it has not noticed, so two may believe they are the  *)
(* node. What must be one is therefore in the store, and changed in one    *)
(* transaction of it: the nonce a join is made with, the incarnation, the  *)
(* number of a request, the request itself before its page is told that it *)
(* was accepted, and an answer. A page stops at any moment and loses what  *)
(* it had in memory. The store may be cleared. Who is at the browser may   *)
(* change. The server offers an incarnation to a join, the same for the    *)
(* same nonce, and fences one it does not hold. An offer is taken only as  *)
(* the answer to the join the store waits on: a page that did not notice   *)
(* may hold one of an incarnation that ended.                              *)
(*                                                                         *)
(* Each constant that is a part of the protocol can be switched off, and   *)
(* TLC must then find a violation.                                         *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   A transaction of the store is one step, seen whole by every page.     *)
(*   The handshake, the records and the mark of the server are             *)
(*     OpenInstances'; here a join, an offer and its confirmation are      *)
(*     steps of their own, and the server does not crash.                  *)
(*   A page that finds the store cleared or an exchange fenced says so to  *)
(*     the document.                                                       *)
(*   LiveSpec: a message arrives, a page starts again, and what can be     *)
(*     done is done.                                                       *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS Pages, MaxK,
          MaxInc,                 \* incarnations offered
          MaxStop, MaxSteal,      \* pages that stop, and locks taken while their holder goes on
          MaxClear, MaxSwitch, MaxExpire,
          JoinInStore,            \* the nonce of a join is the store's
          MatchInStore,           \* an offer is taken only as the answer to the join the store waits on
          NumberInStore,          \* the number of a request is given by the store
          StoreFirst,             \* a request is stored before its page is told it was accepted
          Fence,                  \* an incarnation the server does not hold is refused and told
          ClearTold,              \* what waited when the store was cleared is told unknown
          DeleteOld               \* what was kept for a person is deleted when the person changes

Ks == 1..MaxK
Incs == 1..MaxInc

VARIABLES sinc, sjn, snext, out, view,              \* the store
          alive, lead, lock, minc, mjn, mofr, mfor, mnext, mout,   \* the pages
          status, got, effect, at, shown,           \* the document, what happened, and who is there
          member, offer, ofor, drawn, recs, net, jns,        \* the server and the network
          nStop, nSteal, nClear, nSwitch, nExp

store == <<sinc, sjn, snext, out, view>>
pages == <<alive, lead, lock, minc, mjn, mofr, mfor, mnext, mout>>
doc   == <<status, got, effect, at, shown>>
srv   == <<member, offer, ofor, drawn, recs, jns>>
cnt   == <<nStop, nSteal, nClear, nSwitch, nExp>>
vars  == <<store, pages, doc, srv, net, cnt>>

Init ==
    /\ sinc = 0 /\ sjn = 0 /\ snext = 1 /\ out = {} /\ view = {}
    /\ alive = Pages /\ lead = {} /\ lock = 0
    /\ minc = [p \in Pages |-> 0] /\ mjn = [p \in Pages |-> 0] /\ mofr = [p \in Pages |-> 0]
    /\ mfor = [p \in Pages |-> 0]
    /\ mnext = [p \in Pages |-> 1] /\ mout = [p \in Pages |-> {}]
    /\ status = [k \in Ks |-> "new"] /\ got = [k \in Ks |-> 0] /\ effect = [k \in Ks |-> 0]
    /\ at = 1 /\ shown = {}
    /\ member = 0 /\ offer = 0 /\ ofor = 0 /\ drawn = 0 /\ recs = {} /\ net = {} /\ jns = 0
    /\ nStop = 0 /\ nSteal = 0 /\ nClear = 0 /\ nSwitch = 0 /\ nExp = 0

Unknown == [k \in Ks |-> IF status[k] = "waiting" THEN "unknown" ELSE status[k]]

-----------------------------------------------------------------------------
\* The pages.

\* A page takes the lock: one that is free, or one whose holder goes on and has not noticed.
PLead(p) ==
    /\ p \in alive /\ p \notin lead
    /\ \/ lock = 0 /\ UNCHANGED nSteal
       \/ lock # 0 /\ nSteal < MaxSteal /\ nSteal' = nSteal + 1
    /\ lock' = p /\ lead' = lead \cup {p}
    /\ minc' = [minc EXCEPT ![p] = sinc] /\ mnext' = [mnext EXCEPT ![p] = snext]
    /\ mjn' = [mjn EXCEPT ![p] = 0] /\ mofr' = [mofr EXCEPT ![p] = 0]
    /\ UNCHANGED <<store, alive, mout, mfor, doc, srv, net, nStop, nClear, nSwitch, nExp>>

PStop(p) ==
    /\ p \in alive /\ nStop < MaxStop /\ nStop' = nStop + 1
    /\ alive' = alive \ {p} /\ lead' = lead \ {p} /\ lock' = IF lock = p THEN 0 ELSE lock
    /\ minc' = [minc EXCEPT ![p] = 0] /\ mjn' = [mjn EXCEPT ![p] = 0] /\ mofr' = [mofr EXCEPT ![p] = 0]
    /\ mout' = [mout EXCEPT ![p] = {}] /\ mnext' = [mnext EXCEPT ![p] = 1]
    /\ UNCHANGED <<store, mfor, doc, srv, net, nSteal, nClear, nSwitch, nExp>>

PStart(p) ==
    /\ p \notin alive /\ alive' = alive \cup {p}
    /\ UNCHANGED <<store, lead, lock, minc, mjn, mofr, mfor, mnext, mout, doc, srv, net, cnt>>

\* A page that believes it is the node reads the incarnation the store holds.
PRead(p) ==
    /\ p \in lead /\ minc[p] # sinc
    /\ minc' = [minc EXCEPT ![p] = sinc] /\ mnext' = [mnext EXCEPT ![p] = snext]
    /\ UNCHANGED <<store, alive, lead, lock, mjn, mofr, mfor, mout, doc, srv, net, cnt>>

\* The nonce of a join: drawn once, by whichever page writes it first.
PNonce(p) ==
    /\ p \in lead /\ sinc = 0 /\ jns < MaxInc
    /\ IF JoinInStore THEN sjn = 0 /\ sjn' = jns + 1 /\ UNCHANGED mjn
                      ELSE mjn[p] = 0 /\ mjn' = [mjn EXCEPT ![p] = jns + 1] /\ UNCHANGED sjn
    /\ jns' = jns + 1
    /\ UNCHANGED <<sinc, snext, out, view, alive, lead, lock, minc, mofr, mfor, mnext, mout, doc,
                   member, offer, ofor, drawn, recs, net, cnt>>

Nonce(p) == IF JoinInStore THEN sjn ELSE mjn[p]

\* The join and its offer: the same incarnation for the same nonce while the offer stands.
PJoin(p) ==
    /\ p \in lead /\ sinc = 0 /\ Nonce(p) # 0 /\ mofr[p] = 0
    /\ LET again == offer # 0 /\ ofor = Nonce(p)
           inc   == IF again THEN offer ELSE drawn + 1
       IN  /\ again \/ drawn < MaxInc
           /\ drawn' = IF again THEN drawn ELSE drawn + 1
           /\ offer' = inc /\ ofor' = Nonce(p)
           /\ mofr' = [mofr EXCEPT ![p] = inc] /\ mfor' = [mfor EXCEPT ![p] = Nonce(p)]
    /\ UNCHANGED <<store, alive, lead, lock, minc, mjn, mnext, mout, doc, member, recs, jns, net, cnt>>

\* The offer is written, if the store holds no incarnation and waits on the join it answers.
PTake(p) ==
    /\ p \in lead /\ mofr[p] # 0 /\ mofr' = [mofr EXCEPT ![p] = 0] /\ mjn' = [mjn EXCEPT ![p] = 0]
    /\ IF sinc = 0 /\ (MatchInStore => Nonce(p) = mfor[p]) THEN sinc' = mofr[p] /\ sjn' = 0 /\ snext' = 1 ELSE UNCHANGED <<sinc, sjn, snext>>
    /\ UNCHANGED <<out, view, alive, lead, lock, minc, mfor, mnext, mout, doc, srv, net, cnt>>

\* A request is accepted: its number is given, and it is stored.
PAccept(p, k) ==
    /\ p \in lead /\ status[k] = "new" /\ sinc # 0 /\ minc[p] = sinc
    /\ status' = [status EXCEPT ![k] = "waiting"]
    /\ LET n == IF NumberInStore THEN snext ELSE mnext[p]
           e == [n |-> n, k |-> k, c |-> sinc]
       IN  /\ snext' = IF NumberInStore THEN snext + 1 ELSE snext
           /\ mnext' = IF NumberInStore THEN mnext ELSE [mnext EXCEPT ![p] = @ + 1]
           /\ IF StoreFirst THEN out' = out \cup {e} /\ UNCHANGED mout
                            ELSE mout' = [mout EXCEPT ![p] = @ \cup {e}] /\ UNCHANGED out
    /\ UNCHANGED <<sinc, sjn, view, alive, lead, lock, minc, mjn, mofr, mfor, got, effect, at, shown, srv, net, cnt>>

\* Without StoreFirst a request is stored afterwards.
PStore(p) ==
    /\ p \in alive /\ mout[p] # {}
    /\ out' = out \cup {e \in mout[p] : e.c = sinc} /\ mout' = [mout EXCEPT ![p] = {}]
    /\ UNCHANGED <<sinc, sjn, snext, view, alive, lead, lock, minc, mjn, mofr, mfor, mnext, doc, srv, net, cnt>>

PSend(p, e) ==
    /\ p \in lead /\ e \in out /\ minc[p] = e.c
    /\ net' = net \cup {[kind |-> "req", c |-> e.c, n |-> e.n, k |-> e.k]}
    /\ UNCHANGED <<store, pages, doc, srv, cnt>>

\* An answer, in one transaction with what the page shows.
PReply(p, m) ==
    /\ p \in lead /\ m.kind = "reply" /\ m.c = sinc
    /\ \E e \in out :
         /\ e.n = m.n /\ e.c = m.c
         /\ out' = out \ {e}
         /\ status' = [status EXCEPT ![e.k] = IF @ = "waiting" THEN "done" ELSE @]
         /\ got' = [got EXCEPT ![e.k] = m.k]
         /\ view' = view \cup {at}
    /\ UNCHANGED <<sinc, sjn, snext, pages, effect, at, shown, srv, net, cnt>>

PFenced(p, m) ==
    /\ p \in lead /\ m.kind = "fenced" /\ m.c = sinc
    /\ sinc' = 0 /\ sjn' = 0 /\ out' = {} /\ status' = Unknown
    /\ UNCHANGED <<snext, view, pages, got, effect, at, shown, srv, net, cnt>>

PShow(p) ==
    /\ p \in alive /\ shown' = shown \cup {<<at, w>> : w \in view}
    /\ UNCHANGED <<store, pages, status, got, effect, at, srv, net, cnt>>

-----------------------------------------------------------------------------
\* The browser clears the store, and its pages find it cleared.
Clear ==
    /\ nClear < MaxClear /\ nClear' = nClear + 1
    /\ sinc' = 0 /\ sjn' = 0 /\ snext' = 1 /\ out' = {} /\ view' = {}
    /\ status' = IF ClearTold THEN Unknown ELSE status
    /\ UNCHANGED <<pages, got, effect, at, shown, srv, net, nStop, nSteal, nSwitch, nExp>>

\* Who is at the browser changes: the incarnation is given up.
Switch ==
    /\ nSwitch < MaxSwitch /\ nSwitch' = nSwitch + 1
    /\ at' = at + 1
    /\ sinc' = 0 /\ sjn' = 0 /\ out' = {} /\ status' = Unknown
    /\ view' = IF DeleteOld THEN {} ELSE view
    /\ UNCHANGED <<snext, pages, got, effect, shown, srv, net, nStop, nSteal, nClear, nExp>>

-----------------------------------------------------------------------------
\* The server. A message taken stays in the network, which repeats it.
SEdge(m) ==
    /\ m.kind = "req"
    /\ LET held    == member = m.c
           confirm == ~held /\ offer = m.c
           old     == {r \in recs : r.c = m.c /\ r.n = m.n}
       IN  IF held \/ confirm \/ ~Fence
           THEN /\ member' = IF held THEN member ELSE m.c
                /\ offer' = IF confirm THEN 0 ELSE offer
                /\ IF old # {}
                   THEN /\ LET r == CHOOSE x \in old : TRUE
                           IN net' = net \cup {[kind |-> "reply", c |-> m.c, n |-> m.n, k |-> r.k]}
                        /\ UNCHANGED <<recs, effect>>
                   ELSE /\ recs' = {r \in recs : held \/ r.c = m.c} \cup {[c |-> m.c, n |-> m.n, k |-> m.k]}
                        /\ effect' = [effect EXCEPT ![m.k] = @ + 1]
                        /\ net' = net \cup {[kind |-> "reply", c |-> m.c, n |-> m.n, k |-> m.k]}
           ELSE /\ net' = net \cup {[kind |-> "fenced", c |-> m.c, n |-> 0, k |-> 0]}
                /\ UNCHANGED <<member, offer, recs, effect>>
    /\ UNCHANGED <<store, pages, status, got, at, shown, ofor, drawn, jns, cnt>>

SExpire ==
    /\ nExp < MaxExpire /\ nExp' = nExp + 1 /\ member # 0
    /\ member' = 0 /\ recs' = {}
    /\ UNCHANGED <<store, pages, doc, offer, ofor, drawn, jns, net, nStop, nSteal, nClear, nSwitch>>

-----------------------------------------------------------------------------
Next ==
    \/ \E p \in Pages : PLead(p) \/ PStop(p) \/ PStart(p) \/ PRead(p) \/ PNonce(p) \/ PJoin(p) \/ PTake(p)
                         \/ PStore(p) \/ PShow(p)
    \/ \E p \in Pages, k \in Ks : PAccept(p, k)
    \/ \E p \in Pages : \E e \in out : PSend(p, e)
    \/ \E p \in Pages, m \in net : PReply(p, m) \/ PFenced(p, m)
    \/ \E m \in net : SEdge(m)
    \/ Clear \/ Switch \/ SExpire

Spec == Init /\ [][Next]_vars

Fair ==
    /\ \A p \in Pages : /\ WF_vars(PLead(p) /\ lock = 0) /\ WF_vars(PStart(p)) /\ WF_vars(PRead(p)) /\ WF_vars(PNonce(p))
                         /\ WF_vars(PJoin(p)) /\ WF_vars(PTake(p)) /\ WF_vars(PStore(p))
                         /\ \A k \in Ks : WF_vars(PAccept(p, k))
                         /\ WF_vars(\E e \in out : PSend(p, e))
                         /\ WF_vars(\E m \in net : PReply(p, m)) /\ WF_vars(\E m \in net : PFenced(p, m))
    /\ WF_vars(\E m \in net : SEdge(m))

LiveSpec == Spec /\ Fair

-----------------------------------------------------------------------------
AtMostOnce == \A k \in Ks : effect[k] <= 1

Answered == \A k \in Ks : status[k] = "done" => (effect[k] >= 1 /\ got[k] = k)

\* A page that stops and a lock that moves lose no request the document was told is accepted.
Undisturbed == (nClear + nSwitch + nExp = 0) => \A k \in Ks : status[k] # "unknown"

Kept == \A k \in Ks : status[k] = "waiting" => \E e \in out : e.k = k

RightReader == \A s \in shown : s[1] = s[2]

Ends == \A k \in Ks : (status[k] = "waiting") ~> (status[k] \in {"done", "unknown"})

-----------------------------------------------------------------------------
Service == INSTANCE AtMostOnceService WITH
    Requests <- Ks, status <- status, effect <- effect, faulted <- nClear + nSwitch + nExp > 0

Refines == Service!Spec

=============================================================================
