---- MODULE DurableReceipt ----
(***************************************************************************)
(* The router's durable receipt between a sending machine and a receiving  *)
(* one, both checkpointed, the mechanism (SCE_MESH.md section 10.11;       *)
(* backends/rust/mesh/src/router.rs: Router::send, seal, durable, restore, *)
(* receive, acknowledge and tick), refining ReceiptService.                *)
(*                                                                         *)
(* The sender numbers each envelope it writes (sNext) and holds it until a *)
(* checkpoint that holds it is durable: a seal takes what was written      *)
(* since the last one, and the host's report that the checkpoint is        *)
(* durable releases it. A released envelope is sent, and kept and sent     *)
(* again until an acknowledgement covers its number. The receiver admits a *)
(* number once, keeping the highest below which it admitted every number   *)
(* (rHeld) and those above a gap (rAbove); a repeat is dropped by its      *)
(* number and answered with the acknowledgement. Each checkpoint of the    *)
(* receiver holds its machine's state and the mark it had when sealed;     *)
(* when it is durable the mark is what an acknowledgement reports          *)
(* (rDurable). A machine that stops restarts from its last durable         *)
(* checkpoint: the sender's numbers, what it held unacknowledged and what  *)
(* the checkpoint released; the receiver's state and marks. An envelope is *)
(* named by its number and the incarnation of the sender that wrote it,    *)
(* so a number written again after a restart is another envelope.          *)
(*                                                                         *)
(* The parts, each a constant that is TRUE in the protocol, and each taken *)
(* out by a run that must find what it breaks:                             *)
(*   HoldUntilDurable    an envelope leaves only once a durable checkpoint *)
(*                       holds it                                          *)
(*   KeepUntilAck        a released envelope is kept and sent again until  *)
(*                       it is acknowledged                                *)
(*   UnackedInCheckpoint a checkpoint holds what is unacknowledged, and a  *)
(*                       restore sends it again                            *)
(*   RestoreCounter      a restore takes back the sender's next number     *)
(*   WindowBound         a send is refused while the window is held        *)
(*   ByNumber            the receiver drops a repeat by its number         *)
(*   AnswerRepeat        a repeat is answered with the acknowledgement     *)
(*   AckDurable          an acknowledgement reports the durable mark, not  *)
(*                       what was admitted since                           *)
(*   MarkAtSeal          a durable checkpoint's mark is the one it was     *)
(*                       sealed with                                       *)
(*   RestoreMarks        a restore takes back the receiver's marks         *)
(*                                                                         *)
(* hist and acked are history variables: what the sender's durable         *)
(* checkpoints released, in order, and what an acknowledgement let the     *)
(* sender go of, which no router keeps and the promise is stated by.       *)
(*                                                                         *)
(* Taken as given: a checkpoint is written in one atomic, durable write,   *)
(* holding a machine's state and its router's together, and the host       *)
(* reports durable only a checkpoint so written; a restore reads the last  *)
(* durable one; a machine that stops loses everything else. The link names *)
(* the sender (SCE_MESH.md section 10.9, invariant 10). The network loses, *)
(* repeats, delays and reorders envelopes and acknowledgements and alters  *)
(* none. One checkpoint of each machine is written at a time, a bound of   *)
(* the model: the router releases several in the order they were sealed.   *)
(* The receiver releases what it admits as it arrives (ordering: none);    *)
(* ordering is OrderingBuffer's. LiveSpec: the hosts checkpoint and report *)
(* it, a held envelope is sent again (retry_ms, a readiness edge), and the *)
(* network delivers what is sent again and again.                          *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS
    \* the numbers the sender writes in one incarnation
    \* @type: Int;
    MaxSeq,
    \* the envelopes the sender holds unacknowledged at most (receipt: window)
    \* @type: Int;
    Window,
    \* how many times each machine stops
    \* @type: Int;
    SenderCrashes,
    \* @type: Int;
    ReceiverCrashes,
    \* the parts
    \* @type: Bool;
    HoldUntilDurable,
    \* @type: Bool;
    KeepUntilAck,
    \* @type: Bool;
    UnackedInCheckpoint,
    \* @type: Bool;
    RestoreCounter,
    \* @type: Bool;
    WindowBound,
    \* @type: Bool;
    ByNumber,
    \* @type: Bool;
    AnswerRepeat,
    \* @type: Bool;
    AckDurable,
    \* @type: Bool;
    MarkAtSeal,
    \* @type: Bool;
    RestoreMarks

ASSUME Window >= 1

VARIABLES
    \* the sender: the next number, what was written since the last seal, the checkpoint
    \* sealed and not yet durable, what it released and holds unacknowledged, its last durable
    \* checkpoint, and its incarnation
    \* @type: Int;
    sNext,
    \* @type: Seq(<<Int, Int>>);
    sOpen,
    \* @type: {on: Bool, batch: Seq(<<Int, Int>>), next: Int, unacked: Set(<<Int, Int>>)};
    sSealed,
    \* @type: Set(<<Int, Int>>);
    sUnacked,
    \* @type: {next: Int, unacked: Set(<<Int, Int>>), unreleased: Seq(<<Int, Int>>)};
    sStore,
    \* @type: Int;
    sEra,
    \* the receiver: what its machine took in, its marks, the checkpoint sealed and not yet
    \* durable, its last durable checkpoint (how much of what it took in, and its marks then),
    \* and how many times it stopped
    \* @type: Seq(<<Int, Int>>);
    rTaken,
    \* @type: Int;
    rHeld,
    \* @type: Set(Int);
    rAbove,
    \* @type: Int;
    rDurable,
    \* @type: {on: Bool, count: Int, held: Int, above: Set(Int)};
    rSealed,
    \* @type: {count: Int, held: Int, above: Set(Int)};
    rStore,
    \* @type: Int;
    rCrashes,
    \* the network: envelopes and acknowledgements in flight
    \* @type: Set(<<Int, Int>>);
    wire,
    \* @type: Set(Int);
    acks,
    \* the history variables
    \* @type: Seq(<<Int, Int>>);
    hist,
    \* @type: Set(<<Int, Int>>);
    acked

senderVars == <<sNext, sOpen, sSealed, sUnacked, sStore, sEra>>
receiverVars == <<rTaken, rHeld, rAbove, rDurable, rSealed, rStore, rCrashes>>
vars == <<senderVars, receiverVars, wire, acks, hist, acked>>

\* An envelope: its number and the incarnation of the sender that wrote it.
Envelopes == (1..MaxSeq) \X (0..SenderCrashes)

\* @type: (Seq(<<Int, Int>>)) => Set(<<Int, Int>>);
Elems(s) == {s[k] : k \in DOMAIN s}

NoSenderSeal == [on |-> FALSE, batch |-> <<>>, next |-> 0, unacked |-> {}]
NoReceiverSeal == [on |-> FALSE, count |-> 0, held |-> 0, above |-> {}]

\* What the sender holds against its window: written and not yet released, and released and
\* not yet acknowledged.
SenderHolds == Len(sOpen) + Len(sSealed.batch) + Cardinality(sUnacked)

\* What the receiving machine keeps: the prefix of what it took in its last durable checkpoint
\* holds.
Kept == SubSeq(rTaken, 1, rStore.count)

\* Whether the receiver admitted the number n already.
\* @type: (Int) => Bool;
Holds(n) == n <= rHeld \/ n \in rAbove

\* The receiver's marks once the number n is admitted: rHeld moves past every number now
\* admitted without a gap, and rAbove keeps the rest.
\* @type: (Int) => <<Int, Set(Int)>>;
Admitted(n) ==
    LET all == rAbove \cup {n}
        h == CHOOSE k \in rHeld..MaxSeq :
                 /\ \A j \in (rHeld + 1)..k : j \in all
                 /\ (k + 1) \notin all
    IN <<h, {j \in all : j > h}>>

\* What an acknowledgement reports once the durable mark is d.
\* @type: (Int) => Int;
Reported(d) == IF AckDurable THEN d ELSE rHeld

Init ==
    /\ sNext = 1
    /\ sOpen = <<>>
    /\ sSealed = NoSenderSeal
    /\ sUnacked = {}
    /\ sStore = [next |-> 1, unacked |-> {}, unreleased |-> <<>>]
    /\ sEra = 0
    /\ rTaken = <<>>
    /\ rHeld = 0
    /\ rAbove = {}
    /\ rDurable = 0
    /\ rSealed = NoReceiverSeal
    /\ rStore = [count |-> 0, held |-> 0, above |-> {}]
    /\ rCrashes = 0
    /\ wire = {}
    /\ acks = {}
    /\ hist = <<>>
    /\ acked = {}

-----------------------------------------------------------------------------
\* The sender's machine sends an envelope (Router::send): it takes the next number and is held
\* for the next checkpoint, or, without the part, leaves at once. A send while the window is
\* held is refused (backpressureDrop), and nothing is written.
Write ==
    /\ sNext <= MaxSeq
    /\ ~WindowBound \/ SenderHolds < Window
    /\ LET m == <<sNext, sEra>> IN
       /\ sOpen' = Append(sOpen, m)
       /\ sNext' = sNext + 1
       /\ wire' = IF HoldUntilDurable THEN wire ELSE wire \cup {m}
    /\ UNCHANGED <<sSealed, sUnacked, sStore, sEra, receiverVars, acks, hist, acked>>

\* The sender's host seals a checkpoint (Router::seal): what was written since the last seal
\* belongs to it, with the next number and what is unacknowledged.
SenderSeal ==
    /\ ~sSealed.on
    /\ sSealed' = [on |-> TRUE, batch |-> sOpen, next |-> sNext,
                   unacked |-> IF UnackedInCheckpoint THEN sUnacked ELSE {}]
    /\ sOpen' = <<>>
    /\ UNCHANGED <<sNext, sUnacked, sStore, sEra, receiverVars, wire, acks, hist, acked>>

\* The sender's host reports the checkpoint durable (Router::durable): it is the one a restore
\* reads, and what it holds is released and sent, and kept until acknowledged.
SenderDurable ==
    /\ sSealed.on
    /\ sStore' = [next |-> sSealed.next, unacked |-> sSealed.unacked,
                  unreleased |-> sSealed.batch]
    /\ hist' = hist \o sSealed.batch
    /\ sUnacked' = IF KeepUntilAck THEN sUnacked \cup Elems(sSealed.batch) ELSE sUnacked
    /\ wire' = wire \cup Elems(sSealed.batch)
    /\ sSealed' = NoSenderSeal
    /\ UNCHANGED <<sNext, sOpen, sEra, receiverVars, acks, acked>>

\* The sender sends again an envelope it holds unacknowledged (Router::tick after retry_ms,
\* Router::peer_ready at a readiness edge).
Resend(m) ==
    /\ m \in sUnacked
    /\ m \notin wire
    /\ wire' = wire \cup {m}
    /\ UNCHANGED <<senderVars, receiverVars, acks, hist, acked>>

\* An acknowledgement reaches the sender (Router::receive, durableReceipt): it lets go of every
\* envelope numbered at most its mark. A copy may stay in flight.
SenderAcked(k) ==
    /\ k \in acks
    /\ \/ acks' = acks \ {k}
       \/ acks' = acks
    /\ acked' = acked \cup {m \in sUnacked : m[1] <= k}
    /\ sUnacked' = {m \in sUnacked : m[1] > k}
    /\ UNCHANGED <<sNext, sOpen, sSealed, sStore, sEra, receiverVars, wire, hist>>

\* The sender's machine stops and is restored from its last durable checkpoint
\* (Router::restore): its next number, what it held unacknowledged and what the checkpoint
\* released, which it sends again. What was written since is gone, and was never sent.
SenderCrash ==
    /\ sEra < SenderCrashes
    /\ sEra' = sEra + 1
    /\ sNext' = IF RestoreCounter THEN sStore.next ELSE 1
    /\ sOpen' = <<>>
    /\ sSealed' = NoSenderSeal
    /\ sUnacked' = IF KeepUntilAck THEN sStore.unacked \cup Elems(sStore.unreleased) ELSE {}
    /\ wire' = wire \cup Elems(sStore.unreleased)
    /\ UNCHANGED <<sStore, receiverVars, acks, hist, acked>>

-----------------------------------------------------------------------------
\* An envelope reaches the receiver (Router::receive), which may leave a copy in flight. A
\* number admitted already is a repeat: dropped, and answered with the acknowledgement once a
\* checkpoint is durable. Any other is admitted and its machine takes it in.
Receive(m) ==
    /\ m \in wire
    /\ \/ wire' = wire \ {m}
       \/ wire' = wire
    /\ IF ByNumber /\ Holds(m[1])
         THEN /\ acks' = IF AnswerRepeat /\ rDurable > 0 THEN acks \cup {Reported(rDurable)}
                         ELSE acks
              /\ UNCHANGED <<rTaken, rHeld, rAbove>>
         ELSE /\ rTaken' = Append(rTaken, m)
              /\ rHeld' = Admitted(m[1])[1]
              /\ rAbove' = Admitted(m[1])[2]
              /\ UNCHANGED acks
    /\ UNCHANGED <<senderVars, rDurable, rSealed, rStore, rCrashes, hist, acked>>

\* The receiver's host seals a checkpoint (Router::seal): its machine's state and the mark it
\* has now.
ReceiverSeal ==
    /\ ~rSealed.on
    /\ rSealed' = [on |-> TRUE, count |-> Len(rTaken), held |-> rHeld, above |-> rAbove]
    /\ UNCHANGED <<senderVars, rTaken, rHeld, rAbove, rDurable, rStore, rCrashes, wire, acks,
                   hist, acked>>

\* The receiver's host reports the checkpoint durable (Router::durable): it is the one a restore
\* reads, and when its mark moved the sender is acknowledged.
ReceiverDurable ==
    LET mark == IF MarkAtSeal THEN rSealed.held ELSE rHeld IN
    /\ rSealed.on
    /\ rStore' = [count |-> rSealed.count, held |-> rSealed.held, above |-> rSealed.above]
    /\ IF mark > rDurable
         THEN /\ rDurable' = mark
              /\ acks' = acks \cup {Reported(mark)}
         ELSE UNCHANGED <<rDurable, acks>>
    /\ rSealed' = NoReceiverSeal
    /\ UNCHANGED <<senderVars, rTaken, rHeld, rAbove, rCrashes, wire, hist, acked>>

\* The receiver's machine stops and is restored from its last durable checkpoint
\* (Router::restore): its state and its marks.
ReceiverCrash ==
    /\ rCrashes < ReceiverCrashes
    /\ rCrashes' = rCrashes + 1
    /\ rTaken' = Kept
    /\ rHeld' = IF RestoreMarks THEN rStore.held ELSE 0
    /\ rAbove' = IF RestoreMarks THEN rStore.above ELSE {}
    /\ rDurable' = IF RestoreMarks THEN rStore.held ELSE 0
    /\ rSealed' = NoReceiverSeal
    /\ UNCHANGED <<senderVars, rStore, wire, acks, hist, acked>>

-----------------------------------------------------------------------------
\* The network loses an envelope or an acknowledgement in flight.
LoseEnvelope(m) ==
    /\ m \in wire
    /\ wire' = wire \ {m}
    /\ UNCHANGED <<senderVars, receiverVars, acks, hist, acked>>

LoseAck(k) ==
    /\ k \in acks
    /\ acks' = acks \ {k}
    /\ UNCHANGED <<senderVars, receiverVars, wire, hist, acked>>

Next ==
    \/ Write \/ SenderSeal \/ SenderDurable \/ SenderCrash
    \/ ReceiverSeal \/ ReceiverDurable \/ ReceiverCrash
    \/ \E m \in Envelopes : Resend(m) \/ Receive(m) \/ LoseEnvelope(m)
    \/ \E k \in 1..MaxSeq : SenderAcked(k) \/ LoseAck(k)

Spec == Init /\ [][Next]_vars

\* The hosts checkpoint and report it; a held envelope is sent again; the network delivers an
\* envelope or an acknowledgement that is in flight again and again.
Fair ==
    /\ WF_vars(SenderSeal) /\ WF_vars(SenderDurable)
    /\ WF_vars(ReceiverSeal) /\ WF_vars(ReceiverDurable)
    /\ \A m \in Envelopes : WF_vars(Resend(m)) /\ SF_vars(Receive(m))
    /\ \A k \in 1..MaxSeq : SF_vars(SenderAcked(k))

LiveSpec == Spec /\ Fair

-----------------------------------------------------------------------------
\* The receiving machine took in only what the sender's durable checkpoints released.
NoPhantom == Elems(rTaken) \subseteq Elems(hist)

\* It took in each envelope once.
Once == \A i, j \in DOMAIN rTaken : i # j => rTaken[i] # rTaken[j]

\* The sender let go only of what the receiving machine keeps.
ReceiptKept == acked \subseteq Elems(Kept)

\* An envelope released is kept by the receiving machine, or held by the sender.
NoLoss == \A m \in Elems(hist) : m \in Elems(Kept) \/ m \in sUnacked

\* The sender holds no more than its window.
Bounded == SenderHolds <= Window

\* Every envelope released comes to be kept by the receiving machine.
Delivered == \A m \in Envelopes : m \in Elems(hist) ~> m \in Elems(Kept)

\* And the sender comes to be acknowledged for it.
Settled == \A m \in Envelopes : m \in Elems(hist) ~> m \in acked

\* The sender is acknowledged for everything released. From every state it can come to this
\* (AG EF): nothing that happens leaves it holding for good what the receiver keeps.
Idle == Elems(hist) \subseteq acked

-----------------------------------------------------------------------------
\* What the protocol implements (docs/adr/0115, D2): what the sender committed is what its
\* durable checkpoints released, the receiving machine keeps what its last durable checkpoint
\* holds, and the sender is released from what an acknowledgement let it go of.
Promise == INSTANCE ReceiptService
    WITH history <- hist, taken <- rTaken, kept <- Kept, receipted <- acked

Refines == Promise!Spec
====
