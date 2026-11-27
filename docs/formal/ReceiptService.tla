---- MODULE ReceiptService ----
(***************************************************************************)
(* What two machines may rely on of a binding with durable receipt         *)
(* (SCE_MESH.md section 10.11), with no mechanism: no checkpoint, number,  *)
(* acknowledgement, window or resend is in it.                             *)
(*                                                                         *)
(* The sending machine commits what it sends (history): a send it has not  *)
(* committed when it stops is one its restart does not know it made, and   *)
(* is never received. The receiving machine takes in (taken) only what the *)
(* sender committed, each envelope once, in whatever order it comes. What  *)
(* it took in is made durable up to a point (kept), and a restart takes it *)
(* back to what it keeps. The sender is released from an envelope          *)
(* (receipted) only once the receiver keeps it, so nothing the sender lets *)
(* go is lost with the receiver. What it keeps only grows.                 *)
(*                                                                         *)
(* That every envelope committed comes to be kept is progress, the         *)
(* mechanism's under its fairness (Delivered in DurableReceipt).           *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Nothing: it is what a user may rely on, and names no mechanism.       *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS
    \* the envelopes the sender may send
    \* @type: Set(<<Int, Int>>);
    Envelopes

VARIABLES
    \* what the sender committed, in the order it committed it
    \* @type: Seq(<<Int, Int>>);
    history,
    \* what the receiving machine took in, in the order it took it
    \* @type: Seq(<<Int, Int>>);
    taken,
    \* how much of it the receiving machine keeps durably: a prefix of taken
    \* @type: Seq(<<Int, Int>>);
    kept,
    \* the envelopes the sender is released from
    \* @type: Set(<<Int, Int>>);
    receipted

vars == <<history, taken, kept, receipted>>

\* @type: (Seq(<<Int, Int>>)) => Set(<<Int, Int>>);
Elems(s) == {s[k] : k \in DOMAIN s}

\* @type: (Seq(<<Int, Int>>)) => Bool;
Distinct(s) == \A i, j \in DOMAIN s : i # j => s[i] # s[j]

\* @type: (Seq(<<Int, Int>>), Seq(<<Int, Int>>)) => Bool;
Prefix(a, b) == Len(a) <= Len(b) /\ SubSeq(b, 1, Len(a)) = a

Init ==
    /\ history = <<>>
    /\ taken = <<>>
    /\ kept = <<>>
    /\ receipted = {}

\* The sender commits envelopes it sent, several at once.
Commit ==
    /\ Prefix(history, history')
    /\ history' # history
    /\ Distinct(history')
    /\ Elems(history') \subseteq Envelopes
    /\ UNCHANGED <<taken, kept, receipted>>

\* The receiving machine takes in an envelope the sender committed, which it has not taken.
Take ==
    /\ Len(taken') = Len(taken) + 1
    /\ Prefix(taken, taken')
    /\ taken'[Len(taken')] \in Elems(history) \ Elems(taken)
    /\ UNCHANGED <<history, kept, receipted>>

\* What the receiving machine took in is made durable, up to a point.
Keep ==
    /\ Prefix(kept, kept')
    /\ Prefix(kept', taken)
    /\ kept' # kept
    /\ UNCHANGED <<history, taken, receipted>>

\* The receiving machine restarts from what it keeps.
Restart ==
    /\ taken' = kept
    /\ UNCHANGED <<history, kept, receipted>>

\* The sender is released from envelopes the receiving machine keeps.
Receipt ==
    /\ receipted \subseteq receipted'
    /\ receipted' \subseteq receipted \cup Elems(kept)
    /\ receipted' # receipted
    /\ UNCHANGED <<history, taken, kept>>

Next == Commit \/ Take \/ Keep \/ Restart \/ Receipt

Spec == Init /\ [][Next]_vars
====
