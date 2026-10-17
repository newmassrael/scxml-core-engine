---------------------- MODULE SharedListConflicts ----------------------
(***************************************************************************)
(* What a host holds of a device whose operations conflict, and that it    *)
(* holds the same of it whatever order the operations and the entries of   *)
(* the membership log came in (ADR 0034, `docs/SCE_SHARED_LIST_TRUST.md`). *)
(*                                                                         *)
(* A device can sign two operations at one number (`twin`), and can sign   *)
(* an operation whose time goes back along its chain (`back`: operation s  *)
(* has an earlier time than operation s - 1). Each is a conflict between   *)
(* two operations, at the lower number of the two. An independent review   *)
(* of the code found that the rule it had (a host refuses at the door an   *)
(* operation that conflicts with what it holds, and from then on takes     *)
(* nothing of the device from there) gives two hosts that were given the   *)
(* same operations in two orders different lists, for a conflict of the    *)
(* second kind, which the model of the first kind alone (`SharedList-      *)
(* Convergence`) did not have.                                             *)
(*                                                                         *)
(* `Arrival = TRUE` is that rule. `Arrival = FALSE` is the rule the code   *)
(* has now: a host holds every operation that holds under its log, the    *)
(* proof of a conflict included, and what its machine is told is what is   *)
(* below the lowest position where any two it holds conflict, a function   *)
(* of the set. What is asked is that two hosts that have the whole log     *)
(* and have been given every operation are told the same (Convergence),    *)
(* and that, under the new rule, a host holds exactly the operations it    *)
(* was given that hold under its log (HoldsWhatHolds), which is what makes *)
(* the set, and so the view, independent of order.                         *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS Devices,      \* the devices that write
          Hosts,        \* every host, the devices' own included
          MaxSeq,       \* the most operations a device makes
          MaxRemovals,  \* the most entries that take a device off
          Arrival       \* whether a host refuses at the door what conflicts with what it holds

ASSUME Devices \subseteq Hosts

Seqs     == 1..MaxSeq
Versions == {1, 2}
Nowhere  == MaxSeq + 1

VARIABLES made,     \* made[d]: how many operations device d has made
          named,    \* named[d][s]: the length of the log d held when it made operation s
          twin,     \* twin[d][s]: whether a second version of operation s exists
          back,     \* back[d][s]: whether operation s has an earlier time than operation s - 1
          log,      \* the entries: removals, each of a device at a number
          known,    \* known[h]: how many entries host h has been given
          store,    \* store[h][d]: the <<number, version>> of d's operations host h holds
          given,    \* given[h]: the <<device, number, version>> host h has been given
          frozen    \* frozen[h][d]: from where host h takes nothing of d (the old rule only)

vars == <<made, named, twin, back, log, known, store, given, frozen>>

Remove(d, c) == [dev |-> d, cut |-> c]

\* Whether an operation of d numbered s, which named entry `named[d][s]`, holds for a host that has `k`
\* entries and refuses d from `fr`.
Holds(k, fr, d, s) ==
    /\ s < fr
    /\ \A i \in (named[d][s] + 1)..k : log[i].dev = d => s <= log[i].cut

Min2(a, b) == IF a < b THEN a ELSE b

\* The lowest position at which two operations of d in S conflict, Nowhere when none do.
Positions(d, S) ==
    {s \in Seqs : <<s, 1>> \in S /\ <<s, 2>> \in S}
    \cup {s - 1 : s \in {t \in Seqs : t >= 2 /\ back[d][t] /\ (\E v \in Versions : <<t, v>> \in S)
                         /\ (\E w \in Versions : <<t - 1, w>> \in S)}}

MinConflict(d, S) ==
    IF Positions(d, S) = {} THEN Nowhere
    ELSE CHOOSE p \in Positions(d, S) : \A q \in Positions(d, S) : p <= q

\* What a host's machine is told of d: all of what is held under the old rule (it holds only what did
\* not conflict), what is below the lowest conflict under the new.
View(d, S) == IF Arrival THEN S ELSE {x \in S : x[1] < MinConflict(d, S)}

Init ==
    /\ made   = [d \in Devices |-> 0]
    /\ named  = [d \in Devices |-> [s \in Seqs |-> 0]]
    /\ twin   = [d \in Devices |-> [s \in Seqs |-> FALSE]]
    /\ back   = [d \in Devices |-> [s \in Seqs |-> FALSE]]
    /\ log    = <<>>
    /\ known  = [h \in Hosts |-> 0]
    /\ store  = [h \in Hosts |-> [d \in Devices |-> {}]]
    /\ given  = [h \in Hosts |-> {}]
    /\ frozen = [h \in Hosts |-> [d \in Devices |-> Nowhere]]

\* A device makes its next operation, if its own log does not say it was taken off, with an earlier
\* time than the one before it or not.
Make(d, b) ==
    /\ made[d] < MaxSeq
    /\ \A i \in 1..known[d] : log[i].dev # d
    /\ b => made[d] >= 1
    /\ made'  = [made EXCEPT ![d] = @ + 1]
    /\ named' = [named EXCEPT ![d][made[d] + 1] = known[d]]
    /\ back'  = [back EXCEPT ![d][made[d] + 1] = b]
    /\ UNCHANGED <<twin, log, known, store, given, frozen>>

\* A device signs a second operation at a number it has used.
Twin(d, s) ==
    /\ s <= made[d]
    /\ ~twin[d][s]
    /\ twin' = [twin EXCEPT ![d][s] = TRUE]
    /\ UNCHANGED <<made, named, back, log, known, store, given, frozen>>

\* The administrator takes a device off at a number it has seen of it, once.
AdminRemove(d, c) ==
    /\ Len(log) < MaxRemovals
    /\ c \in 0..made[d]
    /\ \A i \in 1..Len(log) : log[i].dev # d
    /\ log' = Append(log, Remove(d, c))
    /\ UNCHANGED <<made, named, twin, back, known, store, given, frozen>>

\* A host is given the next entry: it drops what the entry puts past a cutoff.
GiveEntry(h) ==
    /\ known[h] < Len(log)
    /\ known' = [known EXCEPT ![h] = @ + 1]
    /\ store' = [store EXCEPT ![h] = [d \in Devices |->
                    {x \in store[h][d] : Holds(known[h] + 1, frozen[h][d], d, x[1])}]]
    /\ UNCHANGED <<made, named, twin, back, log, given, frozen>>

\* A host is given an operation whose entry it has.
Deliver(h, d, s, v) ==
    LET x == <<s, v>>
        old == store[h][d]
    IN /\ s <= made[d]
       /\ v = 1 \/ twin[d][s]
       /\ named[d][s] <= known[h]
       /\ given' = [given EXCEPT ![h] = @ \cup {<<d, s, v>>}]
       /\ IF ~Holds(known[h], frozen[h][d], d, s)
          THEN UNCHANGED <<store, frozen>>
          ELSE IF Arrival /\ MinConflict(d, old \cup {x}) < Nowhere
               THEN \* refused at the door: the device is frozen from the position, and what was held there goes
                    /\ frozen' = [frozen EXCEPT ![h][d] = Min2(@, MinConflict(d, old \cup {x}))]
                    /\ store'  = [store EXCEPT ![h][d] = {y \in old : y[1] < MinConflict(d, old \cup {x})}]
               ELSE /\ store' = [store EXCEPT ![h][d] = old \cup {x}]
                    /\ UNCHANGED frozen
       /\ UNCHANGED <<made, named, twin, back, log, known>>

Next ==
    \/ \E d \in Devices, b \in BOOLEAN : Make(d, b)
    \/ \E d \in Devices, s \in Seqs : Twin(d, s)
    \/ \E d \in Devices, c \in 0..MaxSeq : AdminRemove(d, c)
    \/ \E h \in Hosts : GiveEntry(h)
    \/ \E h \in Hosts, d \in Devices, s \in Seqs, v \in Versions : Deliver(h, d, s, v)

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------

AllOps ==
    UNION { UNION { {<<d, s, v>> : v \in {w \in Versions : w = 1 \/ twin[d][s]}}
                    : s \in 1..made[d] }
            : d \in Devices }

\* The host has the whole log and has been given every operation there is.
Saturated(h) == known[h] = Len(log) /\ given[h] = AllOps

\* Under the set rule, a host holds exactly what it was given that holds under its log.
HoldsWhatHolds ==
    ~Arrival =>
        \A h \in Hosts : \A d \in Devices :
            store[h][d] = {x \in {y \in Seqs \X Versions : <<d, y[1], y[2]>> \in given[h]} :
                                Holds(known[h], Nowhere, d, x[1])}

\* Two hosts that have the whole log and have been given every operation are told the same.
Convergence ==
    \A h1, h2 \in Hosts :
        (Saturated(h1) /\ Saturated(h2)) =>
            \A d \in Devices : View(d, store[h1][d]) = View(d, store[h2][d])

=============================================================================
