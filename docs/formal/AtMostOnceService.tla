-------------------------- MODULE AtMostOnceService --------------------------
(***************************************************************************)
(* What a client of a server whose clients nobody listed may rely on       *)
(* (docs/adr/0115, D1; the promise of docs/adr/0113). No message, crash,   *)
(* lease or number is in it.                                               *)
(*                                                                         *)
(* A request is asked. It takes effect at most once, at any moment after,  *)
(* also after its client has given it up, since a client that gives up     *)
(* does not stop what it asked for. An answer is given only once, to what  *)
(* was asked, and only after the effect. A request that will have no       *)
(* answer ends as one whose outcome is unknown, and that is all it says:   *)
(* it may have taken effect or not, and it happens only after a fault:     *)
(* a client that was stopped or forgotten. Nothing the network does     *)
(* ends a request unanswered.                                           *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Nothing: it is what a user may rely on, and names no mechanism.       *)
(***************************************************************************)
EXTENDS Naturals

CONSTANTS Requests

VARIABLES status,    \* status[r]: new, waiting, done or unknown
          effect,    \* effect[r]: how many times r took effect
          faulted    \* whether a client was stopped or forgotten: nothing less ends a request unanswered

vars == <<status, effect, faulted>>

Statuses == {"new", "waiting", "done", "unknown"}

Init == /\ status = [r \in Requests |-> "new"]
        /\ effect = [r \in Requests |-> 0]
        /\ faulted = FALSE

\* A step may ask, effect, answer or give up several requests: an implementation may end a
\* client's requests together.
Asked(r)   == status[r] = "new" /\ status'[r] = "waiting" /\ effect'[r] = effect[r]
Effect(r)  == status[r] \in {"waiting", "unknown"} /\ effect[r] = 0 /\ effect'[r] = 1
              /\ status'[r] = status[r]
Answer(r)  == status[r] = "waiting" /\ effect[r] = 1 /\ status'[r] = "done" /\ effect'[r] = effect[r]
GiveUp(r)  == faulted' /\ status[r] = "waiting" /\ status'[r] = "unknown" /\ effect'[r] = effect[r]
Same(r)    == status'[r] = status[r] /\ effect'[r] = effect[r]

Next == /\ faulted => faulted'
        /\ \A r \in Requests : Asked(r) \/ Effect(r) \/ Answer(r) \/ GiveUp(r) \/ Same(r)

Spec == Init /\ [][Next]_vars

=============================================================================
