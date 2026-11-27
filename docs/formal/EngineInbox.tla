---- MODULE EngineInbox ----
(***************************************************************************)
(* ADR 0084's door, the mechanism (docs/adr/0147, D2), refining            *)
(* InboxService. Each session has an inbox, a queue many threads post to   *)
(* and its owner drains; the tree has one owner and one waker. The owner   *)
(* drains at a macrostep boundary and at the one chokepoint its external   *)
(* enqueue goes through, before an event of its own (ADR 0084, D2 as       *)
(* amended at step 9), one drain at a time; an input that names a session  *)
(* is posted to that session's inbox when it is posted (step 10); what an  *)
(* inbox holds when its session ends is applied then, and a runtime that   *)
(* never closes applies a later post at the end of its loop.               *)
(*                                                                         *)
(* The parts, each a constant that is TRUE in the protocol, and each taken *)
(* out by a run that must find what it breaks:                             *)
(*   DrainBeforeOwn  the drain before an event of the owner's own          *)
(*   AddressAtPost   the address resolved when the input is posted         *)
(*   OneDrain        a drain inside a drain applies nothing                *)
(*   TakeOut         a drain takes out of the inbox what it applies        *)
(*   ApplyAtEnd      what the inbox holds at the end is applied, not left  *)
(*   Wake            a post wakes the owner                                *)
(*                                                                         *)
(* lin is a history variable: the order posts and own events linearized,   *)
(* per session, which no runtime keeps and every property is stated by.   *)
(*                                                                         *)
(* Taken as given: the inbox is a linearizable FIFO of many producers and  *)
(* one consumer (each runtime's history is held to that, D3; the atomics   *)
(* of the bounded ones by GenMC, D5); the owner runs one macrostep at a    *)
(* time, which the language or the runtime confines (ADR 0084, D1).        *)
(***************************************************************************)
EXTENDS Sequences, Naturals, FiniteSets, Apalache

CONSTANTS
    \* the tree as the promise names it: its two sessions
    \* @type: Str;
    Root,
    \* @type: Str;
    Child,
    \* the inputs posted to each, and the events each puts on its own queue
    \* @type: Set(Str);
    RootInputs,
    \* @type: Set(Str);
    ChildInputs,
    \* @type: Set(Str);
    RootOwn,
    \* @type: Set(Str);
    ChildOwn,
    \* whether the runtime refuses a post after the end
    \* @type: Bool;
    Closes,
    \* the parts
    \* @type: Bool;
    DrainBeforeOwn,
    \* @type: Bool;
    AddressAtPost,
    \* @type: Bool;
    OneDrain,
    \* @type: Bool;
    TakeOut,
    \* @type: Bool;
    ApplyAtEnd,
    \* @type: Bool;
    Wake

VARIABLES
    \* each session's inbox, the inputs posted to it in order
    \* @type: Str -> Seq(Str);
    inbox,
    \* each session's external queue
    \* @type: Str -> Seq(Str);
    extq,
    \* each input: none, taken or refused
    \* @type: Str -> Str;
    status,
    \* the events of the owner not yet put on a queue
    \* @type: Set(Str);
    ownUnsent,
    \* whether each session has ended
    \* @type: Str -> Bool;
    ended,
    \* whether the owner is woken
    \* @type: Bool;
    awake,
    \* the history variable: the order each session linearized
    \* @type: Str -> Seq(Str);
    lin

vars == <<inbox, extq, status, ownUnsent, ended, awake, lin>>

\* The tree as the promise names it.
Tree == INSTANCE InboxService WITH arrivals <- [s \in {Root, Child} |-> <<>>]
Sessions == Tree!Sessions
Inputs == Tree!Inputs
OwnEvents == Tree!OwnEvents

\* What the inbox holds of the inputs posted for session t: an input names its
\* session, so an inbox holds the inputs alone.
ItemsTo(es, t) == SelectSeq(es, LAMBDA x : Tree!Target(x) = t)

\* The order a drain applies what an inbox holds: in order, or, when a drain
\* inside the drain applies, the rest before the head.
DrainOrder(es) == IF OneDrain \/ Len(es) < 2 THEN es ELSE Tail(es) \o <<Head(es)>>

\* The queues once the entries es are applied, each to the session it names.
ApplyOf(q, es) == [t \in Sessions |-> q[t] \o ItemsTo(DrainOrder(es), t)]

Init ==
    /\ inbox = [s \in Sessions |-> <<>>]
    /\ extq = [s \in Sessions |-> <<>>]
    /\ status = [i \in Inputs |-> "none"]
    /\ ownUnsent = OwnEvents
    /\ ended = [s \in Sessions |-> FALSE]
    /\ awake = TRUE
    /\ lin = [s \in Sessions |-> <<>>]

\* A thread posts the input i through the tree's handle.
Post(i) ==
    LET s == Tree!Target(i)
        at == IF AddressAtPost THEN s ELSE Root
    IN
    /\ status[i] = "none"
    /\ IF ended[s] /\ Closes
         THEN /\ status' = [status EXCEPT ![i] = "refused"]
              /\ UNCHANGED <<inbox, awake, lin>>
         ELSE /\ status' = [status EXCEPT ![i] = "taken"]
              /\ inbox' = [inbox EXCEPT ![at] = Append(@, i)]
              /\ awake' = (awake \/ Wake)
              /\ lin' = [lin EXCEPT ![s] = Append(@, i)]
    /\ UNCHANGED <<extq, ownUnsent, ended>>

\* The running owner puts the event e of session s on s's own queue, through
\* s's chokepoint.
Own(e) ==
    LET s == Tree!OwnerOf(e) IN
    /\ e \in ownUnsent
    /\ ~ended[s]
    /\ awake
    /\ IF DrainBeforeOwn
         THEN /\ extq' = [ApplyOf(extq, inbox[s]) EXCEPT ![s] = Append(@, e)]
              /\ inbox' = IF TakeOut THEN [inbox EXCEPT ![s] = <<>>] ELSE inbox
         ELSE /\ extq' = [extq EXCEPT ![s] = Append(@, e)]
              /\ UNCHANGED inbox
    /\ ownUnsent' = ownUnsent \ {e}
    /\ lin' = [lin EXCEPT ![s] = Append(@, e)]
    /\ UNCHANGED <<status, ended, awake>>

\* A macrostep boundary of the tree: the owner applies the root's inbox and
\* then the child's, of the sessions still running, and waits to be woken.
Boundary ==
    LET q1 == IF ended[Root] THEN extq ELSE ApplyOf(extq, inbox[Root])
        i1 == IF ended[Root] \/ ~TakeOut THEN inbox ELSE [inbox EXCEPT ![Root] = <<>>]
        q2 == IF ended[Child] THEN q1 ELSE ApplyOf(q1, i1[Child])
        i2 == IF ended[Child] \/ ~TakeOut THEN i1 ELSE [i1 EXCEPT ![Child] = <<>>]
    IN
    /\ awake
    /\ extq' = q2
    /\ inbox' = i2
    /\ awake' = FALSE
    /\ UNCHANGED <<status, ownUnsent, ended, lin>>

\* The session s's run ends: what its inbox holds is applied, as a direct call
\* made then would be, or, without the part, left.
End(s) ==
    /\ ~ended[s]
    /\ ended' = [ended EXCEPT ![s] = TRUE]
    /\ extq' = IF ApplyAtEnd THEN ApplyOf(extq, inbox[s]) ELSE extq
    /\ inbox' = [inbox EXCEPT ![s] = <<>>]
    /\ UNCHANGED <<status, ownUnsent, awake, lin>>

\* A runtime that never closes applies a post taken after the end where the
\* owner next reaches the end of its loop.
Late(s) ==
    /\ ended[s]
    /\ ~Closes
    /\ inbox[s] # <<>>
    /\ extq' = ApplyOf(extq, inbox[s])
    /\ inbox' = [inbox EXCEPT ![s] = <<>>]
    /\ UNCHANGED <<status, ownUnsent, ended, awake, lin>>

Next ==
    \/ \E i \in Inputs : Post(i)
    \/ \E e \in OwnEvents : Own(e)
    \/ Boundary
    \/ \E s \in Sessions : End(s) \/ Late(s)

Spec == Init /\ [][Next]_vars

\* The owner is not starved: woken, it reaches a boundary, and the end of its
\* loop applies what came late.
LiveSpec == Spec /\ WF_vars(Boundary) /\ \A s \in Sessions : WF_vars(Late(s))

InSeq(x, q) == \E k \in DOMAIN q : q[k] = x

\* What each queue took is the order its posts and own events linearized in.
InOrder ==
    \A t \in Sessions :
        /\ Len(extq[t]) <= Len(lin[t])
        /\ extq[t] = SubSeq(lin[t], 1, Len(extq[t]))

\* No input enters a queue twice.
AtMostOnce ==
    \A t \in Sessions : \A j, k \in DOMAIN extq[t] : j # k => extq[t][j] # extq[t][k]

\* An input taken is in a queue or in an inbox, never nowhere.
NoLoss ==
    \A i \in Inputs :
        status[i] = "taken" =>
            \/ \E t \in Sessions : InSeq(i, extq[t])
            \/ \E s \in Sessions : InSeq(i, ItemsTo(inbox[s], Tree!Target(i)))

\* Every input taken reaches the queue of the session it names.
Delivered == \A i \in Inputs : status[i] = "taken" ~> InSeq(i, extq[Tree!Target(i)])

\* What has arrived, as the promise counts it: what the queue took, then what
\* the session's inbox holds, then what the root's holds for it.
Arrived(t) ==
    extq[t] \o ItemsTo(inbox[t], t) \o (IF t = Root THEN <<>> ELSE ItemsTo(inbox[Root], t))

Promise == INSTANCE InboxService WITH arrivals <- [t \in Sessions |-> Arrived(t)]
Refines == Promise!Spec
-----------------------------------------------------------------------------
\* InOrder, AtMostOnce and NoLoss for every length of execution (docs/adr/0115, D3; docs/adr/0147,
\* D2): an inductive invariant that implies the three, checked by Apalache at the parameters of a
\* configuration (models.json), over every state it allows and not only the reachable ones.
\*
\* Why: what a session linearized is what its queue took followed by what its inbox holds, and an
\* inbox holds only inputs that name its session, so a drain moves the inbox's inputs to the end of
\* that session's queue and nowhere else (InOrder); every input and own event is linearized once,
\* an input when it is taken and an own event when it leaves ownUnsent (AtMostOnce); and an input
\* taken is linearized, so it is in the queue or the inbox of its session (NoLoss).

\* The most a queue can hold: every input and every own event, once. Apalache generates each
\* sequence up to it (Gen), and the invariant implies no sequence is longer, so the states it starts
\* from are every state the invariant allows.
Capacity ==
    Cardinality(RootInputs) + Cardinality(ChildInputs) + Cardinality(RootOwn) + Cardinality(ChildOwn)

\* Each variable a value of its type, for Apalache to start the step from, conjoined with the
\* invariant (the types of models.json): a sequence is given one only by Gen. It narrows nothing
\* the invariant does not.
Typed ==
    /\ inbox = [s \in Sessions |-> Gen(Capacity)]
    /\ extq = [s \in Sessions |-> Gen(Capacity)]
    /\ status \in [Inputs -> {"none", "taken", "refused"}]
    /\ ownUnsent \in SUBSET OwnEvents
    /\ ended \in [Sessions -> BOOLEAN]
    /\ awake \in BOOLEAN
    /\ lin = [s \in Sessions |-> Gen(Capacity)]

QueueInductive ==
    /\ status \in [Inputs -> {"none", "taken", "refused"}]
    /\ ownUnsent \subseteq OwnEvents
    /\ Inputs \cap OwnEvents = {}
    \* What a session linearized is its queue, then its inbox.
    /\ \A t \in Sessions : lin[t] = extq[t] \o inbox[t]
    /\ \A s \in Sessions : \A k \in DOMAIN inbox[s] : Tree!Target(inbox[s][k]) = s
    \* Each is linearized once, and only once it is taken or sent.
    /\ \A t \in Sessions : \A k \in DOMAIN lin[t] :
           /\ lin[t][k] \in Inputs \cup OwnEvents
           /\ lin[t][k] \in Inputs => status[lin[t][k]] = "taken"
           /\ lin[t][k] \in OwnEvents => lin[t][k] \notin ownUnsent
    /\ \A t \in Sessions : \A j, k \in DOMAIN lin[t] : j # k => lin[t][j] # lin[t][k]
    /\ \A i \in Inputs : status[i] = "taken" => InSeq(i, lin[Tree!Target(i)])
====
