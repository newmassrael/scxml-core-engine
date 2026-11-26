---- MODULE InboxService ----
(***************************************************************************)
(* What a host may rely on of ADR 0084's door, with no mechanism           *)
(* (docs/adr/0147, D1): a tree of sessions, inputs posted from any thread, *)
(* and each session's events of its own.                                   *)
(*                                                                         *)
(* arrivals[s] is what has arrived for session s, in the order the posts   *)
(* to s and s's own events linearized: the order s's external queue takes  *)
(* them in. An input taken is appended once and never removed, so it is    *)
(* neither lost nor applied twice; a post is refused only once the session *)
(* it names has ended and its runtime closes its inbox (Closes). Nothing   *)
(* here says when an arrival is consumed: the queue is the machine's, read *)
(* at a macrostep boundary, so a posted input never reaches a running      *)
(* macrostep (ADR 0084, D1).                                               *)
(*                                                                         *)
(* Taken as given: the inputs are distinct values, and a post linearizes   *)
(* at one instant, which is what the runtime's queue must show            *)
(* (docs/adr/0147, D3 and D5 judge that).                                  *)
(***************************************************************************)
EXTENDS Sequences

CONSTANTS
    Root, Child,              \* the tree: a root session and a child it drives
    RootInputs, ChildInputs,  \* the inputs posted to each, each posted once
    RootOwn, ChildOwn,        \* the events each session puts on its own queue
    Closes                    \* whether the runtime refuses a post after the end

VARIABLES arrivals, status, ownUnsent, ended

vars == <<arrivals, status, ownUnsent, ended>>

Sessions == {Root, Child}
Inputs == RootInputs \cup ChildInputs
OwnEvents == RootOwn \cup ChildOwn
Target(i) == IF i \in RootInputs THEN Root ELSE Child
OwnerOf(e) == IF e \in RootOwn THEN Root ELSE Child

Init ==
    /\ arrivals = [s \in Sessions |-> <<>>]
    /\ status = [i \in Inputs |-> "none"]
    /\ ownUnsent = OwnEvents
    /\ ended = [s \in Sessions |-> FALSE]

Post(i) ==
    LET s == Target(i) IN
    /\ status[i] = "none"
    /\ IF ended[s] /\ Closes
         THEN /\ status' = [status EXCEPT ![i] = "refused"]
              /\ UNCHANGED arrivals
         ELSE /\ status' = [status EXCEPT ![i] = "taken"]
              /\ arrivals' = [arrivals EXCEPT ![s] = Append(@, i)]
    /\ UNCHANGED <<ownUnsent, ended>>

Own(e) ==
    LET s == OwnerOf(e) IN
    /\ e \in ownUnsent
    /\ ~ended[s]
    /\ arrivals' = [arrivals EXCEPT ![s] = Append(@, e)]
    /\ ownUnsent' = ownUnsent \ {e}
    /\ UNCHANGED <<status, ended>>

End(s) ==
    /\ ~ended[s]
    /\ ended' = [ended EXCEPT ![s] = TRUE]
    /\ UNCHANGED <<arrivals, status, ownUnsent>>

Next ==
    \/ \E i \in Inputs : Post(i)
    \/ \E e \in OwnEvents : Own(e)
    \/ \E s \in Sessions : End(s)

Spec == Init /\ [][Next]_vars
====
