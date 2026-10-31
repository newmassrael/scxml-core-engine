------------------------- MODULE BroadcastLogWrapped -------------------------
(***************************************************************************)
(* The protocol of ADR 0062 over a stamp of StampBits bits, which wraps:   *)
(* the narrow row of "Stamp width" (BroadcastLog.tla is the same protocol  *)
(* over a stamp that does not).                                            *)
(*                                                                         *)
(* Ledger section: §synth-5-S, the broadcast-log kind of the RFC.          *)
(*                                                                         *)
(* What differs from BroadcastLog.tla is arithmetic, and it is all here.   *)
(* M = 2^StampBits. The stamps and the head hold the count they stand for  *)
(* modulo M: Busy(c) = (2c + 1) mod M, Done(c) = (2c + 2) mod M, head =    *)
(* the number completed mod M. A reader's cursor `cur` is a count modulo   *)
(* M and it never learns the true count: it takes its distance from the    *)
(* head modulo M, which is the true distance while that is below M. The    *)
(* slot of a count is its residue modulo S = R + 1, so a cursor, known     *)
(* only modulo M, names the publisher's slot only when S divides M.        *)
(*                                                                         *)
(* The true counts are history variables and no step reads them: `n` (the  *)
(* publisher's completed count) and `ab` (the count the reader's cursor    *)
(* stands for, which is the reader's own full count of its place and starts *)
(* at the true one). They are what the specification of ADR 0062 section 2 *)
(* is stated over, so an answer is checked against them.                   *)
(*                                                                         *)
(* The wrap bound is an assumption on the environment, and EnforceBound    *)
(* is the switch for it. While it is on, a reader starts a read only when  *)
(* the newest publish begun is at most K = M / 2 entries past its cursor,  *)
(* and the publisher does not begin a publish that would put it more than  *)
(* K past the cursor of a reader that is inside a read. That is the        *)
(* hypothesis of the theorem in backends/rust/forge-runtime/src/           *)
(* broadcast_log_core.rs, at every instant. It covers the two loads that   *)
(* open a reader at the oldest entry as it covers the loads of a read: a   *)
(* reader between them (open2, dhead) holds the log to K publishes past    *)
(* the first load (`ohl`), because a stamp space that wrapped between them *)
(* leaves a distance of 0 for what is M entries, and an Empty for a head   *)
(* that is not there (BroadcastLogWrapped_deferred_without_bound.cfg).     *)
(* With it off the model is a log                                          *)
(* whose reader may be stopped for as long as it likes, and TLC finds the  *)
(* answer the bound exists to prevent (BroadcastLogWrapped_without_bound   *)
(* .cfg: a copy of an entry a lap on, handed as the entry the reader       *)
(* wanted). With S not dividing M the model finds the reader of a cursor   *)
(* taken modulo M in the wrong slot (BroadcastLogWrapped_non_divisor.cfg). *)
(*                                                                         *)
(* A reader opens at the head, or at the oldest entry (OpenMode). The      *)
(* second is two shared reads: the stamp of the last slot, to know whether *)
(* it was ever written, and then the head. A last slot still holding       *)
(* Unwritten means the log has published at most R entries, so the oldest  *)
(* is the first; it is how a count modulo M tells a young log from one     *)
(* that wrapped. In OpenMode "deferred" the same two loads are the start   *)
(* of the reader's first read (RDStart, RDHead): it binds its cursor to    *)
(* the oldest retained entry of the head it loads and goes on from there,  *)
(* so a reader that waited is told of nothing, and one that was overtaken  *)
(* inside that first read is told as any reader is. DeferredStart and      *)
(* FirstLagIsOvertaken are what is checked of it.                          *)
(*                                                                         *)
(* Each shared access is one step, so the model is sequentially consistent: *)
(* it proves the arithmetic of the wrap, and the memory ordering is the    *)
(* business of loom (tests/loom_broadcast_log_narrow.rs). No liveness is   *)
(* claimed (ADR 0002).                                                     *)
(***************************************************************************)
EXTENDS Integers, TLC

CONSTANTS Readers,        \* the reader handles
          R,              \* the retention the specification promises
          StampBits,      \* the width of the stamp in bits
          MaxPub,         \* the most entries the producer publishes
          EnforceBound,   \* whether the wrap bound is assumed
          OpenMode        \* "head", "oldest" or "deferred": where a reader opens

M == 2 ^ StampBits
K == M \div 2
S == R + 1

ASSUME R \in 1..4 /\ StampBits \in 2..4 /\ MaxPub \in Nat
ASSUME EnforceBound \in BOOLEAN /\ OpenMode \in {"head", "oldest", "deferred"}
ASSUME S <= K

Idx(c) == c % S
Busy(c) == (2 * c + 1) % M
Done(c) == (2 * c + 2) % M
Unwritten == 1
Garbage == MaxPub + 1
Words == {0, 1}

VARIABLES head, stamp, words, n, ppc,
          opened, cur, ab, lost, rpc, hl, s1, cp, res, tch, ohl, oab,
          fresh, fl, ob          \* what a deferred reader's first read adds (history)

pvars == <<head, stamp, words, n, ppc>>
dvars == <<fresh, fl, ob>>
rvars == <<opened, cur, ab, lost, rpc, hl, s1, cp, res, tch, ohl, oab, dvars>>
vars  == <<pvars, rvars>>

None == [k |-> "none"]

Init ==
    /\ head = 0 /\ n = 0 /\ ppc = "idle"
    /\ stamp = [i \in 0..(S - 1) |-> Unwritten]
    /\ words = [i \in 0..(S - 1) |-> [w \in Words |-> Garbage]]
    /\ opened = [r \in Readers |-> FALSE]
    /\ cur  = [r \in Readers |-> 0]
    /\ ab   = [r \in Readers |-> 0]
    /\ lost = [r \in Readers |-> 0]
    /\ rpc  = [r \in Readers |-> "idle"]
    /\ hl   = [r \in Readers |-> 0]
    /\ s1   = [r \in Readers |-> 0]
    /\ cp   = [r \in Readers |-> [w \in Words |-> Garbage]]
    /\ res  = [r \in Readers |-> None]
    /\ tch  = [r \in Readers |-> FALSE]
    /\ ohl  = [r \in Readers |-> 0]
    /\ oab  = [r \in Readers |-> 0]
    /\ fresh = [r \in Readers |-> OpenMode = "deferred"]
    /\ fl    = [r \in Readers |-> FALSE]
    /\ ob    = [r \in Readers |-> 0]

(* The number of publishes begun: the busy stamp of the newest is stored. *)
Begun == IF ppc \in {"idle", "busy"} THEN n ELSE n + 1

(* The producer. `n` counts the entries completed, and so the head is n mod M. *)

PStart ==
    /\ ppc = "idle" /\ n < MaxPub
    /\ ppc' = "busy"
    /\ UNCHANGED <<head, stamp, words, n, rvars>>

(* Begins the publish of entry n. Under the bound it does not put the newest begun *)
(* more than K past the cursor of a reader that is inside a read. *)
PBusy ==
    /\ ppc = "busy"
    /\ (~EnforceBound \/
        (/\ \A r \in Readers : (opened[r] /\ rpc[r] # "idle") => (n + 1 - ab[r] <= K)
         /\ \A r \in Readers :
               (~opened[r] /\ rpc[r] \in {"open2", "dhead"}) => (n + 1 - ohl[r] <= K)))
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
    /\ head' = (n + 1) % M
    /\ n' = n + 1
    /\ ppc' = "idle"
    /\ UNCHANGED <<stamp, words, rvars>>

(* Opening a reader. At the head, the cursor is the head's residue. At the oldest, *)
(* the last slot says whether the log has published at least R entries: unwritten *)
(* means it has not, and the oldest is the first, count 0; written means the oldest *)
(* is R behind the head loaded next. ohl is the true head the position was fixed *)
(* at and oab the true count it stands for, for the check below. *)

ROpen(r) ==
    /\ OpenMode = "head" /\ ~opened[r] /\ rpc[r] = "idle"
    /\ opened' = [opened EXCEPT ![r] = TRUE]
    /\ cur' = [cur EXCEPT ![r] = head]
    /\ ab' = [ab EXCEPT ![r] = n]
    /\ ohl' = [ohl EXCEPT ![r] = n]
    /\ oab' = [oab EXCEPT ![r] = n]
    /\ UNCHANGED <<pvars, lost, rpc, hl, s1, cp, res, tch, dvars>>

ROpenStamp(r) ==
    /\ OpenMode = "oldest" /\ ~opened[r] /\ rpc[r] = "idle"
    /\ tch' = [tch EXCEPT ![r] = stamp[S - 1] # Unwritten]
    /\ ohl' = [ohl EXCEPT ![r] = n]
    /\ rpc' = [rpc EXCEPT ![r] = "open2"]
    /\ UNCHANGED <<pvars, opened, cur, ab, lost, hl, s1, cp, res, oab, dvars>>

ROpenHead(r) ==
    /\ rpc[r] = "open2"
    /\ IF tch[r]
          THEN /\ cur' = [cur EXCEPT ![r] = (head + M - R) % M]
               /\ ab' = [ab EXCEPT ![r] = n - R]
               /\ ohl' = [ohl EXCEPT ![r] = n]
               /\ oab' = [oab EXCEPT ![r] = n - R]
          ELSE /\ cur' = [cur EXCEPT ![r] = 0]
               /\ ab' = [ab EXCEPT ![r] = 0]
               /\ oab' = [oab EXCEPT ![r] = 0]
               /\ UNCHANGED ohl
    /\ opened' = [opened EXCEPT ![r] = TRUE]
    /\ rpc' = [rpc EXCEPT ![r] = "idle"]
    /\ UNCHANGED <<pvars, lost, hl, s1, cp, res, tch, dvars>>

(* A read. Every answer is set by a load of head, which is `hl`. *)

Behind(r) == (head + M - cur[r]) % M

Skipped(r) == IF Behind(r) < S THEN Behind(r) + M - R ELSE Behind(r) - R

RStart(r) ==
    /\ opened[r] /\ rpc[r] = "idle"
    /\ (~EnforceBound \/ Begun - ab[r] <= K)
    /\ rpc' = [rpc EXCEPT ![r] = "head1"]
    /\ res' = [res EXCEPT ![r] = None]
    /\ UNCHANGED <<pvars, opened, cur, ab, lost, hl, s1, cp, tch, ohl, oab, dvars>>

RHead(r) ==
    /\ rpc[r] = "head1"
    /\ hl' = [hl EXCEPT ![r] = n]
    /\ \/ /\ Behind(r) = 0
          /\ res' = [res EXCEPT ![r] = [k |-> "Empty"]]
          /\ rpc' = [rpc EXCEPT ![r] = "idle"]
          /\ fresh' = [fresh EXCEPT ![r] = FALSE]
          /\ UNCHANGED <<cur, ab, lost, fl>>
       \/ /\ Behind(r) >= S
          /\ res' = [res EXCEPT ![r] =
                [k |-> "Lagged", sk |-> Behind(r) - R, from |-> ab[r]]]
          /\ cur' = [cur EXCEPT ![r] = (cur[r] + Behind(r) - R) % M]
          /\ ab' = [ab EXCEPT ![r] = ab[r] + Behind(r) - R]
          /\ lost' = [lost EXCEPT ![r] = @ + (Behind(r) - R)]
          /\ rpc' = [rpc EXCEPT ![r] = "idle"]
          /\ fl' = [fl EXCEPT ![r] = fresh[r]]
          /\ fresh' = [fresh EXCEPT ![r] = FALSE]
       \/ /\ Behind(r) > 0 /\ Behind(r) < S
          /\ rpc' = [rpc EXCEPT ![r] = "stamp1"]
          /\ UNCHANGED <<cur, ab, lost, res, fresh, fl>>
    /\ UNCHANGED <<pvars, opened, s1, cp, tch, ohl, oab, ob>>

(* The first read of a deferred reader. It begins with the two loads of opening at the oldest   *)
(* (the stamp of the last slot, then the head), which here are part of the read: the cursor the *)
(* second load gives is the oldest retained entry of the head it loaded, and the read goes on   *)
(* from there as any read does, answering at that same load if it can: Empty, or Lagged when a  *)
(* publish completed between the two loads and overtook the cursor. `ob` is the true count at  *)
(* the first load. *)

RDStart(r) ==
    /\ OpenMode = "deferred" /\ ~opened[r] /\ rpc[r] = "idle"
    /\ tch' = [tch EXCEPT ![r] = stamp[S - 1] # Unwritten]
    /\ ohl' = [ohl EXCEPT ![r] = n]
    /\ ob' = [ob EXCEPT ![r] = n]
    /\ rpc' = [rpc EXCEPT ![r] = "dhead"]
    /\ res' = [res EXCEPT ![r] = None]
    /\ UNCHANGED <<pvars, opened, cur, ab, lost, hl, s1, cp, oab, fresh, fl>>

RDHead(r) ==
    /\ rpc[r] = "dhead"
    /\ LET c0 == IF tch[r] THEN (head + M - R) % M ELSE 0
           a0 == IF tch[r] THEN n - R ELSE 0
           b0 == (head + M - c0) % M
       IN /\ hl' = [hl EXCEPT ![r] = n]
          /\ opened' = [opened EXCEPT ![r] = TRUE]
          /\ ohl' = [ohl EXCEPT ![r] = IF tch[r] THEN n ELSE ohl[r]]
          /\ oab' = [oab EXCEPT ![r] = a0]
          /\ \/ /\ b0 = 0
                /\ res' = [res EXCEPT ![r] = [k |-> "Empty"]]
                /\ cur' = [cur EXCEPT ![r] = c0]
                /\ ab' = [ab EXCEPT ![r] = a0]
                /\ rpc' = [rpc EXCEPT ![r] = "idle"]
                /\ fresh' = [fresh EXCEPT ![r] = FALSE]
                /\ UNCHANGED <<lost, fl>>
             \/ /\ b0 >= S
                /\ res' = [res EXCEPT ![r] =
                      [k |-> "Lagged", sk |-> b0 - R, from |-> a0]]
                /\ cur' = [cur EXCEPT ![r] = (c0 + b0 - R) % M]
                /\ ab' = [ab EXCEPT ![r] = a0 + b0 - R]
                /\ lost' = [lost EXCEPT ![r] = @ + (b0 - R)]
                /\ rpc' = [rpc EXCEPT ![r] = "idle"]
                /\ fl' = [fl EXCEPT ![r] = fresh[r]]
                /\ fresh' = [fresh EXCEPT ![r] = FALSE]
             \/ /\ b0 > 0 /\ b0 < S
                /\ cur' = [cur EXCEPT ![r] = c0]
                /\ ab' = [ab EXCEPT ![r] = a0]
                /\ rpc' = [rpc EXCEPT ![r] = "stamp1"]
                /\ UNCHANGED <<lost, res, fresh, fl>>
    /\ UNCHANGED <<pvars, s1, cp, tch, ob>>

RStamp1(r) ==
    /\ rpc[r] = "stamp1"
    /\ s1' = [s1 EXCEPT ![r] = stamp[Idx(cur[r])]]
    /\ rpc' = [rpc EXCEPT ![r] =
          IF stamp[Idx(cur[r])] = Done(cur[r]) THEN "c0" ELSE "reload"]
    /\ UNCHANGED <<pvars, opened, cur, ab, lost, hl, cp, res, tch, ohl, oab, dvars>>

RCopy(r, w, from, to) ==
    /\ rpc[r] = from
    /\ cp' = [cp EXCEPT ![r][w] = words[Idx(cur[r])][w]]
    /\ rpc' = [rpc EXCEPT ![r] = to]
    /\ UNCHANGED <<pvars, opened, cur, ab, lost, hl, s1, res, tch, ohl, oab, dvars>>

RValidate(r) ==
    /\ rpc[r] = "v2"
    /\ \/ /\ stamp[Idx(cur[r])] = s1[r]
          /\ res' = [res EXCEPT ![r] = [k |-> "Item", seq |-> ab[r], v |-> cp[r]]]
          /\ cur' = [cur EXCEPT ![r] = (cur[r] + 1) % M]
          /\ ab' = [ab EXCEPT ![r] = ab[r] + 1]
          /\ rpc' = [rpc EXCEPT ![r] = "idle"]
          /\ fresh' = [fresh EXCEPT ![r] = FALSE]
          /\ UNCHANGED fl
       \/ /\ stamp[Idx(cur[r])] # s1[r]
          /\ rpc' = [rpc EXCEPT ![r] = "reload"]
          /\ UNCHANGED <<cur, ab, res, fresh, fl>>
    /\ UNCHANGED <<pvars, opened, lost, hl, s1, cp, tch, ohl, oab, ob>>

(* A copy that was overwritten while it was read: the head is reloaded and the *)
(* distance, if it looks nearer than the window, is a head that wrapped. *)
RReload(r) ==
    /\ rpc[r] = "reload"
    /\ hl' = [hl EXCEPT ![r] = n]
    /\ res' = [res EXCEPT ![r] =
          [k |-> "Lagged", sk |-> Skipped(r), from |-> ab[r]]]
    /\ cur' = [cur EXCEPT ![r] = (cur[r] + Skipped(r)) % M]
    /\ ab' = [ab EXCEPT ![r] = ab[r] + Skipped(r)]
    /\ lost' = [lost EXCEPT ![r] = @ + Skipped(r)]
    /\ rpc' = [rpc EXCEPT ![r] = "idle"]
    /\ fl' = [fl EXCEPT ![r] = fresh[r]]
    /\ fresh' = [fresh EXCEPT ![r] = FALSE]
    /\ UNCHANGED <<pvars, opened, s1, cp, tch, ohl, oab, ob>>

Next ==
    \/ PStart \/ PBusy \/ PWord(0, "w0", "w1") \/ PWord(1, "w1", "done") \/ PDone \/ PHead
    \/ \E r \in Readers :
          \/ ROpen(r) \/ ROpenStamp(r) \/ ROpenHead(r)
          \/ RDStart(r) \/ RDHead(r)
          \/ RStart(r) \/ RHead(r) \/ RStamp1(r)
          \/ RCopy(r, 0, "c0", "c1") \/ RCopy(r, 1, "c1", "v2")
          \/ RValidate(r) \/ RReload(r)

Spec == Init /\ [][Next]_vars

(* What the specification requires of an answer, given the head it was decided at, *)
(* in the true counts: the reader's own count `ab` is the one its cursor stands for. *)

AnswerRefines ==
    \A r \in Readers :
        /\ res[r].k = "Item" =>
              /\ res[r].v = [w \in Words |-> res[r].seq]
              /\ res[r].seq < hl[r]
              /\ hl[r] - res[r].seq <= R
        /\ res[r].k = "Empty" => ab[r] = hl[r]
        /\ res[r].k = "Lagged" =>
              /\ res[r].sk >= 1
              /\ ab[r] = hl[r] - R
              /\ res[r].sk = hl[r] - R - res[r].from

CursorNotPastHead == \A r \in Readers : opened[r] => ab[r] <= n

LostIsNonNegative == \A r \in Readers : lost[r] >= 0

(* The cursor is the count modulo M, and names the slot the count's entry is in. *)
(* It is so exactly when S divides M. *)
SlotIndexContinuous ==
    \A r \in Readers : opened[r] => (cur[r] % S = ab[r] % S)

(* A reader opened where the specification puts it: at the head, or at the oldest *)
(* retained entry, max(0, h - R). A deferred reader's position is fixed by its first *)
(* read, at the oldest retained entry of the head that read loaded. *)
OpenRefines ==
    \A r \in Readers :
        opened[r] =>
            IF OpenMode = "head"
               THEN oab[r] = ohl[r]
               ELSE oab[r] = IF ohl[r] > R THEN ohl[r] - R ELSE 0

(* A deferred reader never goes below its start, and what it was told it lost lies at or *)
(* after it: every entry it was told it missed advanced its count. *)
DeferredStart ==
    \A r \in Readers :
        opened[r] => (ab[r] >= oab[r] /\ lost[r] <= ab[r] - oab[r])

(* A deferred reader's first read answers Lagged only if a publish had completed since the *)
(* first load of that read: the cursor it was given is at most R behind the head, which the *)
(* R + 1 slots keep readable until another publish begins. *)
FirstLagIsOvertaken ==
    \A r \in Readers : fl[r] => hl[r] > ob[r]

=============================================================================
