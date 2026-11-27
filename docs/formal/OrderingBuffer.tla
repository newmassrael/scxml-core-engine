---- MODULE OrderingBuffer ----
(***************************************************************************)
(* The receive half's ordering of one sender's stream, the mechanism       *)
(* (SCE_MESH.md section 10.6.4; backends/rust/mesh/src/inbound.rs, its     *)
(* rules sce:std/mesh/ordering_hold, ordering_drain, ordering_prune and    *)
(* ordering_gap_end, and the C++ OrderingBuffer), refining OrderService.   *)
(* The receiver expects a number (next), anchored by the first envelope    *)
(* that reaches it. One below it is dropped; one above it is held with the *)
(* time it has waited since it first arrived; the one it expects is        *)
(* released with every held envelope that follows on from it. The host's   *)
(* tick ends a gap whose first held envelope has waited GapTimeout: the    *)
(* numbers before that envelope are told lost (ORDERING_GAP) and what is   *)
(* held from it on is released. A durable stream (SCE_MESH.md section      *)
(* 10.11) is anchored at the number its sender sends first, and its gaps   *)
(* never time out: its sender holds the missing envelope and sends it      *)
(* again.                                                                  *)
(*                                                                         *)
(* The parts, each a constant that is TRUE in the protocol, and each taken *)
(* out by a run that must find what it breaks:                             *)
(*   DropBehind      an envelope below the number expected is dropped      *)
(*   HoldAhead       an envelope above it is held, not released            *)
(*   Drain           a release takes every held envelope that follows on   *)
(*   OldestFirst     a gap ends at the lowest held envelope, and only once *)
(*                   that one has waited                                   *)
(*   TimeoutGap      the tick ends a gap that has waited                   *)
(*   FirstArrival    a copy of a held envelope keeps its first arrival's   *)
(*                   time, so a gap's wait is not started again            *)
(*   WaitWhenDurable a durable stream's gap never times out                *)
(*   AnchorDurable   a durable stream is anchored at the number its sender *)
(*                   sends first, not at the first envelope that arrives   *)
(*                                                                         *)
(* seen and late are history variables: the numbers that reached the       *)
(* receiver, and those that reached it after they were told lost, which    *)
(* the code keeps neither of and a property is stated by.                  *)
(*                                                                         *)
(* Taken as given: the sender numbers what it sends 1, 2, 3, ..., each     *)
(* once, and the link the stream arrives on names the sender (SCE_MESH.md  *)
(* section 10.9, invariant 10), so the stream is one sender's; the network *)
(* loses, repeats, delays and reorders envelopes and alters none; the held *)
(* envelopes never reach the bound of the rule's list (a hold past it is   *)
(* an error to the host, which no envelope is released by); an envelope    *)
(* the receiver does not order (a reply, a field notification) passes the  *)
(* buffer and is not of this stream. Time passes in steps of Elapse, and a *)
(* gap's wait is counted up to GapTimeout and no further. LiveSpec: time   *)
(* passes, the host ticks, the network delivers what is sent again and     *)
(* again, and a durable sender sends again what was not released.          *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS
    \* the most numbers the sender sends
    \* @type: Int;
    MaxNumber,
    \* how long a held envelope waits before the gap before it ends, in steps of Elapse
    \* @type: Int;
    GapTimeout,
    \* whether the stream is durable
    \* @type: Bool;
    Durable,
    \* the parts
    \* @type: Bool;
    DropBehind,
    \* @type: Bool;
    HoldAhead,
    \* @type: Bool;
    Drain,
    \* @type: Bool;
    OldestFirst,
    \* @type: Bool;
    TimeoutGap,
    \* @type: Bool;
    FirstArrival,
    \* @type: Bool;
    WaitWhenDurable,
    \* @type: Bool;
    AnchorDurable

ASSUME GapTimeout >= 1

VARIABLES
    \* how many the sender has sent: the numbers 1..sent
    \* @type: Int;
    sent,
    \* the envelopes in flight, by number
    \* @type: Set(Int);
    wire,
    \* the number the receiver expects, 0 until the stream is anchored
    \* @type: Int;
    next,
    \* each held envelope, by number, with the time it has waited since it first arrived
    \* @type: Int -> Int;
    pending,
    \* what the receiver released to its machine, in order
    \* @type: Seq(Int);
    released,
    \* the numbers it told its machine are lost
    \* @type: Set(Int);
    told,
    \* the history variables: what reached the receiver, and what reached it after it was told lost
    \* @type: Set(Int);
    seen,
    \* @type: Set(Int);
    late

vars == <<sent, wire, next, pending, released, told, seen, late>>

Numbers == 1..MaxNumber

\* @type: (Seq(Int)) => Set(Int);
Elems(s) == {s[k] : k \in DOMAIN s}

\* The numbers from a up to b - 1, in order: what a release takes of the held envelopes.
\* @type: (Int, Int) => Seq(Int);
Run(a, b) == [i \in 1..(b - a) |-> a + i - 1]

\* The number expected once start - 1 is released: start, moved past every held envelope that
\* follows on from it without a gap (ordering_drain).
\* @type: (Int) => Int;
Drained(start) ==
    CHOOSE k \in start..(MaxNumber + 1) :
        /\ k \notin DOMAIN pending
        /\ \A j \in start..(k - 1) : j \in DOMAIN pending

\* What stays held once the number expected is d (ordering_prune).
\* @type: (Int) => (Int -> Int);
Prune(d) == [k \in {j \in DOMAIN pending : j >= d} |-> pending[k]]

\* The held envelopes with n among them, just arrived (ordering_hold).
\* @type: (Int) => (Int -> Int);
Hold(n) == [k \in DOMAIN pending \cup {n} |-> IF k = n THEN 0 ELSE pending[k]]

Init ==
    /\ sent = 0
    /\ wire = {}
    /\ next = 0
    /\ pending = [k \in {} |-> 0]
    /\ released = <<>>
    /\ told = {}
    /\ seen = {}
    /\ late = {}

\* The sender sends its next number.
Send ==
    /\ sent < MaxNumber
    /\ sent' = sent + 1
    /\ wire' = wire \cup {sent + 1}
    /\ UNCHANGED <<next, pending, released, told, seen, late>>

\* The network loses an envelope in flight.
Lose(n) ==
    /\ n \in wire
    /\ wire' = wire \ {n}
    /\ UNCHANGED <<sent, next, pending, released, told, seen, late>>

\* A durable sender sends again an envelope its receiver has not released.
Resend(n) ==
    /\ Durable
    /\ n \in 1..sent
    /\ n \notin wire
    /\ n \notin Elems(released)
    /\ wire' = wire \cup {n}
    /\ UNCHANGED <<sent, next, pending, released, told, seen, late>>

\* The envelope n reaches the receiver, which may leave a copy in flight. The stream is anchored
\* at the first that arrives, or, durable, at the number its sender sends first.
Admit(n) ==
    LET at == IF next # 0 THEN next ELSE IF Durable /\ AnchorDurable THEN 1 ELSE n
        d == IF Drain THEN Drained(n + 1) ELSE n + 1
    IN
    /\ n \in wire
    /\ \/ wire' = wire \ {n}
       \/ wire' = wire
    /\ seen' = seen \cup {n}
    /\ late' = IF n \in told THEN late \cup {n} ELSE late
    /\ IF n = at
         THEN /\ released' = Append(released, n) \o Run(n + 1, d)
              /\ next' = d
              /\ pending' = Prune(d)
       ELSE IF n < at
         THEN IF DropBehind
                THEN UNCHANGED <<released, next, pending>>
                ELSE /\ released' = Append(released, n)
                     /\ UNCHANGED <<next, pending>>
       ELSE IF HoldAhead
         THEN /\ next' = at
              /\ pending' = IF n \notin DOMAIN pending THEN Hold(n)
                            ELSE IF FirstArrival THEN pending
                            ELSE [pending EXCEPT ![n] = 0]
              /\ UNCHANGED released
         ELSE /\ released' = Append(released, n)
              /\ next' = at
              /\ UNCHANGED pending
    /\ UNCHANGED <<sent, told>>

\* Time passes for every held envelope, counted up to GapTimeout.
Elapse ==
    /\ \E k \in DOMAIN pending : pending[k] < GapTimeout
    /\ pending' = [k \in DOMAIN pending |->
                      IF pending[k] < GapTimeout THEN pending[k] + 1 ELSE pending[k]]
    /\ UNCHANGED <<sent, wire, next, released, told, seen, late>>

\* The held envelopes a gap may end at: those that have waited, and, with the part, only the
\* lowest held (ordering_gap_end).
Waited == {k \in DOMAIN pending : pending[k] >= GapTimeout /\ k # next}
Resumable ==
    IF OldestFirst
      THEN {k \in Waited : \A j \in DOMAIN pending : k <= j}
      ELSE Waited

\* The host's tick ends a gap: the numbers before the envelope it ends at are told lost, and
\* what is held from that envelope on is released.
Tick ==
    /\ TimeoutGap
    /\ ~(Durable /\ WaitWhenDurable)
    /\ next # 0
    /\ \E r \in Resumable :
           LET d == IF Drain THEN Drained(r) ELSE r + 1 IN
           /\ told' = told \cup (next..(r - 1))
           /\ released' = released \o Run(r, d)
           /\ next' = d
           /\ pending' = Prune(d)
    /\ UNCHANGED <<sent, wire, seen, late>>

Next ==
    \/ Send
    \/ \E n \in Numbers : Lose(n) \/ Resend(n) \/ Admit(n)
    \/ Elapse
    \/ Tick

Spec == Init /\ [][Next]_vars

\* Time passes and the host ticks; the network delivers an envelope that is in flight again and
\* again; a durable sender sends again what was not released.
Fair ==
    /\ WF_vars(Elapse)
    /\ WF_vars(Tick)
    /\ \A n \in Numbers : SF_vars(Admit(n)) /\ WF_vars(Resend(n))

LiveSpec == Spec /\ Fair

-----------------------------------------------------------------------------
\* The machine is given what it is given in increasing number.
InOrder == \A i, j \in DOMAIN released : i < j => released[i] < released[j]

\* A number told lost had not reached the receiver when it was told: one that reached it after
\* is late, and nothing else told is seen.
LostUnseen == told \cap seen \subseteq late

\* A durable stream tells nothing lost.
NoneTold == Durable => told = {}

\* A durable stream gives its machine first the number its sender sent first.
FromFirst == Durable /\ released # <<>> => released[1] = 1

\* No envelope is held for ever: each is released or given up in the end.
Released == \A n \in Numbers : n \in DOMAIN pending ~> n \notin DOMAIN pending

\* Nothing is held. From every state it can come to this (AG EF): nothing that happens leaves an
\* envelope held for good.
Idle == DOMAIN pending = {}

-----------------------------------------------------------------------------
\* What the protocol implements (docs/adr/0115, D2): what arrived is what reached the receiver,
\* and the machine is given what was released and told what was told.
Promise == INSTANCE OrderService WITH arrived <- seen, delivered <- released

Refines == Promise!Spec
====
