---- MODULE InboxService ----
(***************************************************************************)
(* What a host may rely on of ADR 0084's door, with no mechanism           *)
(* (docs/adr/0147, D1): a tree of sessions, inputs posted from any thread, *)
(* and each session's events of its own.                                   *)
(*                                                                         *)
(* A post is either taken or refused, and which is decided when it is      *)
(* posted. arrivals[s] is what has arrived for session s, in the order the *)
(* posts to s and s's own events linearized: the order s's external queue  *)
(* takes them in. An input taken is appended once and never removed, so it *)
(* is neither lost nor applied twice. entered[s] is how much of            *)
(* arrivals[s] the queue has taken; the rest is what s holds, the inputs   *)
(* taken and not yet in the queue. An own event enters behind every input  *)
(* taken before it, and what a session holds when its run ends enters      *)
(* then. Nothing here says when an arrival is consumed: the queue is the   *)
(* machine's, read at a macrostep boundary, so a posted input never        *)
(* reaches a running macrostep (ADR 0084, D1).                             *)
(*                                                                         *)
(* A post goes through one of two doors. Through the tree's directory, by  *)
(* the id of the session it names: the directory holds the sessions of the *)
(* tree, and a session leaves it when its run ends. Or through the handle  *)
(* of the session's own inbox (ByHandle), which names a session of the     *)
(* tree and looks nothing up.                                              *)
(*                                                                         *)
(* A refused post changes nothing but its own answer, which says why:      *)
(*   unknown  the post went through the directory, which does not hold the *)
(*            session: the tree never held it, or its run has ended and it *)
(*            has left. A post through a handle is never unknown           *)
(*   ended    the session's run has ended, where the runtime closes its    *)
(*            inbox (Closes)                                               *)
(*   full     the session holds Capacity inputs taken and not yet in its   *)
(*            queue                                                        *)
(* Where the runtime never closes, a post to a session whose run has ended *)
(* that is not refused is taken, and what it takes enters at the end of    *)
(* the owner's loop. A post the promise counts full may still be taken: a  *)
(* drain takes inputs out of an inbox before they enter the queue, and     *)
(* frees room that what a session holds here does not show. That an inbox  *)
(* never holds more than Capacity is the mechanism's (Bounded in           *)
(* EngineInbox), and no poster is told anything false past it.             *)
(*                                                                         *)
(* Taken as given: the inputs are distinct values, and a post linearizes   *)
(* at one instant, which is what the runtime's queue must show             *)
(* (docs/adr/0147, D3 and D5 judge that).                                  *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS
    Root, Child,              \* the tree: a root session and a child it drives
    RootInputs, ChildInputs,  \* the inputs posted to each, each posted once
    Unknown,                  \* a session the tree does not hold
    UnknownInputs,            \* the inputs posted naming it, each posted once
    ByHandle,                 \* the inputs posted through their session's own handle
    RootOwn, ChildOwn,        \* the events each session puts on its own queue
    Capacity,                 \* the inputs a session holds before a post is full
    Closes                    \* whether the runtime refuses a post after the end

VARIABLES arrivals, entered, status, ownUnsent, ended

vars == <<arrivals, entered, status, ownUnsent, ended>>

Sessions == {Root, Child}
\* The inputs that name a session the tree holds, through either door.
TreeInputs == RootInputs \cup ChildInputs
Inputs == TreeInputs \cup UnknownInputs
OwnEvents == RootOwn \cup ChildOwn
Target(i) ==
    IF i \in RootInputs THEN Root ELSE IF i \in ChildInputs THEN Child ELSE Unknown
OwnerOf(e) == IF e \in RootOwn THEN Root ELSE Child

\* A handle is a session's own, so a post through one names a session of the tree.
ASSUME ByHandle \subseteq TreeInputs

\* What a refused post answers.
Refusals == {"unknown", "ended", "full"}

\* The most that can arrive for a session: every input and every own event.
MostArrivals == Cardinality(Inputs) + Cardinality(OwnEvents)

\* What session s holds: the inputs taken and not yet in its queue.
Held(s) == Len(arrivals[s]) - entered[s]

Init ==
    /\ arrivals = [s \in Sessions |-> <<>>]
    /\ entered = [s \in Sessions |-> 0]
    /\ status = [i \in Inputs |-> "none"]
    /\ ownUnsent = OwnEvents
    /\ ended = [s \in Sessions |-> FALSE]

\* The post of i is refused for the reason why, and changes nothing else.
Refuse(i, why) ==
    /\ status' = [status EXCEPT ![i] = why]
    /\ UNCHANGED <<arrivals, entered>>

\* The post of i reaches session s, which may hold its capacity.
Reach(i, s) ==
    \/ /\ Held(s) >= Capacity
       /\ Refuse(i, "full")
    \/ /\ status' = [status EXCEPT ![i] = "taken"]
       /\ arrivals' = [arrivals EXCEPT ![s] = Append(@, i)]
       /\ UNCHANGED entered

Post(i) ==
    LET s == Target(i) IN
    /\ status[i] = "none"
    /\ IF s \notin Sessions
         THEN Refuse(i, "unknown")
         ELSE \/ /\ ended[s]
                 /\ i \notin ByHandle
                 /\ Refuse(i, "unknown")
              \/ IF ended[s] /\ Closes
                   THEN Refuse(i, "ended")
                   ELSE Reach(i, s)
    /\ UNCHANGED <<ownUnsent, ended>>

\* An own event enters the queue at once, behind everything its session held.
Own(e) ==
    LET s == OwnerOf(e) IN
    /\ e \in ownUnsent
    /\ ~ended[s]
    /\ arrivals' = [arrivals EXCEPT ![s] = Append(@, e)]
    /\ entered' = [entered EXCEPT ![s] = Len(arrivals[s]) + 1]
    /\ ownUnsent' = ownUnsent \ {e}
    /\ UNCHANGED <<status, ended>>

\* The queues take what their sessions hold, each in order, some or all of it.
Enter ==
    /\ entered' \in [Sessions -> 0..MostArrivals]
    /\ \A s \in Sessions : entered[s] <= entered'[s] /\ entered'[s] <= Len(arrivals[s])
    /\ UNCHANGED <<arrivals, status, ownUnsent, ended>>

\* The session s's run ends, and what it holds enters its queue then.
End(s) ==
    /\ ~ended[s]
    /\ ended' = [ended EXCEPT ![s] = TRUE]
    /\ entered' = [entered EXCEPT ![s] = Len(arrivals[s])]
    /\ UNCHANGED <<arrivals, status, ownUnsent>>

Next ==
    \/ \E i \in Inputs : Post(i)
    \/ \E e \in OwnEvents : Own(e)
    \/ Enter
    \/ \E s \in Sessions : End(s)

Spec == Init /\ [][Next]_vars
====
