---- MODULE OrderService ----
(***************************************************************************)
(* What a machine may rely on of a binding that orders what one sender     *)
(* sends it (SCE_MESH.md section 10.6), with no mechanism: no buffer,      *)
(* clock, timeout or tick is in it.                                        *)
(*                                                                         *)
(* The sender numbers what it sends 1, 2, 3, ... (sent). What reaches the  *)
(* receiver (arrived) may come late, twice, out of order or not at all.    *)
(* The machine is given envelopes (delivered) in increasing number, so     *)
(* each at most once. A number may instead be told lost (told), and never  *)
(* both. From the first number the machine is given up to the last it was  *)
(* given or told, every number is one or the other: nothing is passed over *)
(* silently. A number is told lost only when it had not arrived by then,   *)
(* so an envelope that reached the receiver is never given up for one that *)
(* came after it. A number below the first the machine is given is         *)
(* neither: the receiver joined the stream there, at the first envelope    *)
(* that reached it.                                                        *)
(*                                                                         *)
(* Where the stream is durable (Durable, SCE_MESH.md section 10.11) the    *)
(* sender keeps what it sent until it is received, so nothing is told      *)
(* lost and the first number the machine is given is the first the sender  *)
(* sent.                                                                   *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Nothing: it is what a user may rely on, and names no mechanism.       *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS
    \* the most numbers the sender sends
    \* @type: Int;
    MaxNumber,
    \* whether the stream is durable
    \* @type: Bool;
    Durable

VARIABLES
    \* how many the sender has sent: the numbers 1..sent
    \* @type: Int;
    sent,
    \* the numbers that reached the receiver, once or more
    \* @type: Set(Int);
    arrived,
    \* the numbers the machine was given, in the order it was given them
    \* @type: Seq(Int);
    delivered,
    \* the numbers the machine was told are lost
    \* @type: Set(Int);
    told

vars == <<sent, arrived, delivered, told>>

\* @type: (Seq(Int)) => Set(Int);
Elems(s) == {s[k] : k \in DOMAIN s}

\* @type: (Set(Int)) => Int;
Max(s) == CHOOSE n \in s : \A m \in s : m <= n

\* @type: (Seq(Int), Seq(Int)) => Bool;
Prefix(a, b) == Len(a) <= Len(b) /\ SubSeq(b, 1, Len(a)) = a

\* What the machine was given and told, d and t, as the stream allows it: in increasing number,
\* nothing both, every number from the first given to the last accounted for, and, on a durable
\* stream, nothing told and the first number the first given.
\* @type: (Seq(Int), Set(Int)) => Bool;
Ordered(d, t) ==
    LET accounted == Elems(d) \cup t IN
    /\ \A i, j \in DOMAIN d : i < j => d[i] < d[j]
    /\ Elems(d) \cap t = {}
    /\ accounted \subseteq 1..sent
    /\ accounted # {} =>
           /\ d # <<>>
           /\ \A n \in accounted : d[1] <= n
           /\ \A n \in d[1]..Max(accounted) : n \in accounted
    /\ Durable =>
           /\ t = {}
           /\ d # <<>> => d[1] = 1

Init ==
    /\ sent = 0
    /\ arrived = {}
    /\ delivered = <<>>
    /\ told = {}

\* The sender sends the next number.
Send ==
    /\ sent < MaxNumber
    /\ sent' = sent + 1
    /\ UNCHANGED <<arrived, delivered, told>>

\* Envelopes arrive, and the machine is given some and told some lost, several at once: what
\* arrived ahead of a number is given when that number is.
Step ==
    /\ arrived \subseteq arrived'
    /\ arrived' \subseteq 1..sent
    /\ Prefix(delivered, delivered')
    /\ told \subseteq told'
    /\ Ordered(delivered', told')
    /\ \A n \in Elems(delivered') \ Elems(delivered) : n \in arrived'
    /\ \A n \in told' \ told : n \notin arrived'
    /\ UNCHANGED sent

Next == Send \/ Step

Spec == Init /\ [][Next]_vars
====
