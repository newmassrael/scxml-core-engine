------------------------------ MODULE BroadcastLog ------------------------------
(***************************************************************************)
(* The protocol of ADR 0062: a bounded log of retention R with one         *)
(* producer and any number of readers, kept in a ring of S slots, each     *)
(* with a stamp and two payload words.                                     *)
(*                                                                         *)
(* Ledger section: §synth-5-S, the broadcast-log kind of the RFC.          *)
(*                                                                         *)
(* The producer writes a slot as: stamp := busy(n); the words; stamp :=    *)
(* done(n); then head := n + 1. A reader holding cursor r loads head       *)
(* (empty, or outside the window, or else), loads the stamp of slot        *)
(* r mod S, copies the words, loads the stamp again, and answers Item, or  *)
(* reloads head and answers Lagged. Each shared access is one step, so     *)
(* the model is sequentially consistent: it proves the protocol, and the   *)
(* memory ordering is the business of loom and GenMC (ADR 0062 section 6). *)
(*                                                                         *)
(* The entry published at sequence n has the value n in both words, so a   *)
(* copy that mixes two entries shows as two words that differ.             *)
(*                                                                         *)
(* What is checked is the refinement of the abstract log (section 2 of the *)
(* ADR), by the step the answer is linearized at: every answer is a load   *)
(* of head, and the answer must be what the specification gives when the   *)
(* head held the value that load read (`hl`). Item must carry the entry    *)
(* of the cursor and have the cursor in the window; Empty must have the    *)
(* cursor at head; Lagged must have the cursor outside the window, skip at *)
(* least one entry, and put the cursor at the oldest retained.             *)
(*                                                                         *)
(* `Extra` is the slot beyond the retention (S = R + Extra) and            *)
(* `Revalidate` the second stamp load. With either off the model is a      *)
(* design the ADR rejects, and TLC finds the violation                     *)
(* (BroadcastLog_without_extra_slot.cfg, _without_revalidation.cfg).       *)
(*                                                                         *)
(* A reader of `DeferredReaders` is opened by `reader_deferred()`: it has  *)
(* no cursor until its first read, which binds the cursor to the oldest    *)
(* retained entry of the head that read loads, in the same step, and then  *)
(* goes on as any read does. The other readers are opened at 0, the        *)
(* oldest entry of a log with nothing published. Two things are checked of *)
(* a deferred reader, and they are the reasons it needs no new mechanism:  *)
(* `DeferredStart` (its start is that oldest retained entry, and what it   *)
(* is told it lost lies after the start), and `FirstLagIsOvertaken` (its   *)
(* first read answers Lagged only when the head had moved on since the     *)
(* bind, which takes the slot beyond the retention: with S = R, the        *)
(* bound entry's slot is the one the next publish begins to write, and the *)
(* reader is told of a loss nothing had caused,                            *)
(* BroadcastLog_deferred_without_extra_slot.cfg).                          *)
(***************************************************************************)
EXTENDS Integers, TLC

CONSTANTS Readers,         \* the reader handles
          DeferredReaders, \* those opened by reader_deferred(): bound by their first read
          R,               \* the retention the specification promises
          Extra,           \* 1: S = R + 1 slots (the design); 0: S = R
          MaxPub,          \* the most entries the producer publishes
          Revalidate       \* whether a reader loads the stamp again after the copy

ASSUME DeferredReaders \subseteq Readers
ASSUME R \in 1..4 /\ Extra \in {0, 1} /\ MaxPub \in Nat /\ Revalidate \in BOOLEAN

S == R + Extra
Idx(n) == n % S
Busy(n) == 2 * n + 1
Done(n) == 2 * n + 2
Garbage == MaxPub + 1
Words == {0, 1}
Max0(x) == IF x > 0 THEN x ELSE 0

VARIABLES head, stamp, words, n, ppc,        \* shared state and the producer
          cur, lost, rpc, hl, s1, cp, res,   \* each reader's
          bound, origin, ob, fresh, fl       \* and what a deferred reader's start adds

pvars == <<head, stamp, words, n, ppc>>
bvars == <<bound, origin, ob, fresh, fl>>
rvars == <<cur, lost, rpc, hl, s1, cp, res, bvars>>
vars  == <<pvars, rvars>>

None == [k |-> "none"]

Init ==
    /\ head = 0 /\ n = 0 /\ ppc = "idle"
    /\ stamp = [i \in 0..(S - 1) |-> 0]
    /\ words = [i \in 0..(S - 1) |-> [w \in Words |-> Garbage]]
    /\ cur  = [r \in Readers |-> 0]
    /\ lost = [r \in Readers |-> 0]
    /\ rpc  = [r \in Readers |-> "idle"]
    /\ hl   = [r \in Readers |-> 0]
    /\ s1   = [r \in Readers |-> 0]
    /\ cp   = [r \in Readers |-> [w \in Words |-> Garbage]]
    /\ res  = [r \in Readers |-> None]
    /\ bound  = [r \in Readers |-> r \notin DeferredReaders]
    /\ origin = [r \in Readers |-> 0]
    /\ ob     = [r \in Readers |-> 0]
    /\ fresh  = [r \in Readers |-> r \in DeferredReaders]
    /\ fl     = [r \in Readers |-> FALSE]

(* The producer. *)

PStart ==
    /\ ppc = "idle" /\ n < MaxPub
    /\ ppc' = "busy"
    /\ UNCHANGED <<head, stamp, words, n, rvars>>

PBusy ==
    /\ ppc = "busy"
    /\ stamp' = [stamp EXCEPT ![Idx(n)] = Busy(n)]
    /\ ppc' = "w0"
    /\ UNCHANGED <<head, words, n, rvars>>

PWord(w, from, to) ==
    /\ ppc = from
    /\ words' = [words EXCEPT ![Idx(n)][w] = n]
    /\ ppc' = to
    /\ UNCHANGED <<head, stamp, n, rvars>>

PDone ==
    /\ ppc = "done"
    /\ stamp' = [stamp EXCEPT ![Idx(n)] = Done(n)]
    /\ ppc' = "head"
    /\ UNCHANGED <<head, words, n, rvars>>

PHead ==
    /\ ppc = "head"
    /\ head' = n + 1
    /\ n' = n + 1
    /\ ppc' = "idle"
    /\ UNCHANGED <<stamp, words, rvars>>

(* A reader. Every answer is set by a load of head, which is `hl`. *)

RStart(r) ==
    /\ rpc[r] = "idle"
    /\ rpc' = [rpc EXCEPT ![r] = "head1"]
    /\ res' = [res EXCEPT ![r] = None]
    /\ UNCHANGED <<pvars, cur, lost, hl, s1, cp, bvars>>

(* The binding of a deferred reader's first read: the cursor is the oldest retained entry of  *)
(* the head this read loads, and the reader records where it started and which head it saw.   *)
Bind(r) ==
    /\ ~bound[r]
    /\ bound'  = [bound  EXCEPT ![r] = TRUE]
    /\ origin' = [origin EXCEPT ![r] = Max0(head - R)]
    /\ ob'     = [ob     EXCEPT ![r] = head]
    /\ cur'    = [cur    EXCEPT ![r] = Max0(head - R)]

RHead(r) ==
    /\ rpc[r] = "head1"
    /\ hl' = [hl EXCEPT ![r] = head]
    /\ \/ /\ bound[r] /\ cur[r] = head
          /\ res' = [res EXCEPT ![r] = [k |-> "Empty"]]
          /\ rpc' = [rpc EXCEPT ![r] = "idle"]
          /\ fresh' = [fresh EXCEPT ![r] = FALSE]
          /\ UNCHANGED <<cur, lost, bound, origin, ob, fl>>
       \/ /\ bound[r] /\ cur[r] # head /\ head - cur[r] >= S
          /\ res' = [res EXCEPT ![r] = [k |-> "Lagged", sk |-> head - R - cur[r]]]
          /\ cur' = [cur EXCEPT ![r] = head - R]
          /\ lost' = [lost EXCEPT ![r] = @ + (head - R - cur[r])]
          /\ rpc' = [rpc EXCEPT ![r] = "idle"]
          /\ fl' = [fl EXCEPT ![r] = fresh[r]]
          /\ fresh' = [fresh EXCEPT ![r] = FALSE]
          /\ UNCHANGED <<bound, origin, ob>>
       \/ /\ bound[r] /\ cur[r] # head /\ head - cur[r] < S
          /\ rpc' = [rpc EXCEPT ![r] = "stamp1"]
          /\ UNCHANGED <<cur, lost, res, bvars>>
       \/ /\ Bind(r) /\ Max0(head - R) = head
          /\ res' = [res EXCEPT ![r] = [k |-> "Empty"]]
          /\ rpc' = [rpc EXCEPT ![r] = "idle"]
          /\ fresh' = [fresh EXCEPT ![r] = FALSE]
          /\ UNCHANGED <<lost, fl>>
       \/ /\ Bind(r) /\ Max0(head - R) # head
          /\ rpc' = [rpc EXCEPT ![r] = "stamp1"]
          /\ UNCHANGED <<lost, res, fresh, fl>>
    /\ UNCHANGED <<pvars, s1, cp>>

RStamp1(r) ==
    /\ rpc[r] = "stamp1"
    /\ s1' = [s1 EXCEPT ![r] = stamp[Idx(cur[r])]]
    /\ rpc' = [rpc EXCEPT ![r] = IF stamp[Idx(cur[r])] = Done(cur[r]) THEN "c0" ELSE "reload"]
    /\ UNCHANGED <<pvars, cur, lost, hl, cp, res, bvars>>

RCopy(r, w, from, to) ==
    /\ rpc[r] = from
    /\ cp' = [cp EXCEPT ![r][w] = words[Idx(cur[r])][w]]
    /\ rpc' = [rpc EXCEPT ![r] = to]
    /\ UNCHANGED <<pvars, cur, lost, hl, s1, res, bvars>>

RValidate(r) ==
    /\ rpc[r] = "v2"
    /\ \/ /\ (~Revalidate \/ stamp[Idx(cur[r])] = s1[r])
          /\ res' = [res EXCEPT ![r] = [k |-> "Item", seq |-> cur[r], v |-> cp[r]]]
          /\ cur' = [cur EXCEPT ![r] = @ + 1]
          /\ rpc' = [rpc EXCEPT ![r] = "idle"]
          /\ fresh' = [fresh EXCEPT ![r] = FALSE]
          /\ UNCHANGED <<bound, origin, ob, fl>>
       \/ /\ Revalidate /\ stamp[Idx(cur[r])] # s1[r]
          /\ rpc' = [rpc EXCEPT ![r] = "reload"]
          /\ UNCHANGED <<cur, res, bvars>>
    /\ UNCHANGED <<pvars, lost, hl, s1, cp>>

RReload(r) ==
    /\ rpc[r] = "reload"
    /\ hl' = [hl EXCEPT ![r] = head]
    /\ res' = [res EXCEPT ![r] = [k |-> "Lagged", sk |-> head - R - cur[r]]]
    /\ cur' = [cur EXCEPT ![r] = head - R]
    /\ lost' = [lost EXCEPT ![r] = @ + (head - R - cur[r])]
    /\ rpc' = [rpc EXCEPT ![r] = "idle"]
    /\ fl' = [fl EXCEPT ![r] = fresh[r]]
    /\ fresh' = [fresh EXCEPT ![r] = FALSE]
    /\ UNCHANGED <<pvars, s1, cp, bound, origin, ob>>

Next ==
    \/ PStart \/ PBusy \/ PWord(0, "w0", "w1") \/ PWord(1, "w1", "done") \/ PDone \/ PHead
    \/ \E r \in Readers :
          \/ RStart(r) \/ RHead(r) \/ RStamp1(r)
          \/ RCopy(r, 0, "c0", "c1") \/ RCopy(r, 1, "c1", "v2")
          \/ RValidate(r) \/ RReload(r)

Spec == Init /\ [][Next]_vars

(* What the specification requires of an answer, given the head it was decided at. *)

AnswerRefines ==
    \A r \in Readers :
        /\ res[r].k = "Item" =>
              /\ res[r].v = [w \in Words |-> res[r].seq]
              /\ res[r].seq < hl[r]
              /\ hl[r] - res[r].seq <= R
        /\ res[r].k = "Empty" => cur[r] = hl[r]
        /\ res[r].k = "Lagged" =>
              /\ res[r].sk >= 1
              /\ cur[r] = hl[r] - R

CursorNotPastHead == \A r \in Readers : cur[r] <= head

LostIsNonNegative == \A r \in Readers : lost[r] >= 0

(* A deferred reader: its start is the oldest retained entry of the head its first read    *)
(* loaded, its cursor never goes below the start, and what it was told it lost lies at or  *)
(* after the start (every entry it was told it missed advanced its cursor).                *)
DeferredStart ==
    \A r \in Readers :
        bound[r] =>
            /\ (r \in DeferredReaders => origin[r] = Max0(ob[r] - R))
            /\ cur[r] >= origin[r]
            /\ lost[r] <= cur[r] - origin[r]

(* A deferred reader's first read answers Lagged only if the head had moved since the bind. *)
FirstLagIsOvertaken ==
    \A r \in DeferredReaders : fl[r] => hl[r] > ob[r]

=============================================================================
