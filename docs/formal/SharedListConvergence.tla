---------------------- MODULE SharedListConvergence ----------------------
(***************************************************************************)
(* The safety properties ADR 0034 states for a shared list, for the rule   *)
(* the hosts apply to operations: each operation of a device names the     *)
(* number it has in the device's chain and the entry of the membership log *)
(* it was made under; the log can take a device off at a number (`cut`),   *)
(* and a device can sign two operations at one number.                     *)
(*                                                                         *)
(* A host holds an operation when the entry it names is one the host has,  *)
(* its number is not past a cutoff of the entries the host has, and its    *)
(* device is not frozen at or before it. Entries reach a host in order,    *)
(* operations in any order and again. What is asked: that no host holds    *)
(* what its own log puts past a cutoff (HeldValid), and that two hosts     *)
(* that have the whole log and have been given every operation hold the    *)
(* same (Convergence).                                                     *)
(*                                                                         *)
(* `Evict` is the rule the code applies: when an entry arrives, a host      *)
(* drops what it held that the entry puts past a cutoff, and when it sees  *)
(* two operations at one number it drops everything of the device from     *)
(* that number. With `Evict = FALSE` the model is what the code was before *)
(* the property was checked against it, and TLC finds the divergence       *)
(* (SharedListConvergence_without_eviction.cfg).                           *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS Devices,      \* the devices that write
          Hosts,        \* every host, the devices' own included
          MaxSeq,       \* the most operations a device makes
          MaxRemovals,  \* the most entries that take a device off (the configurations that
                        \* study a fork alone set it to 0, the ones that study removal alone
                        \* forbid the fork, so that each is small enough to check whole)
          Evict,        \* whether a host drops what a later entry or a conflict puts out
          Equivocate    \* whether a device may sign two operations at one number

ASSUME Devices \subseteq Hosts

Seqs     == 1..MaxSeq
Versions == {1, 2}

VARIABLES made,     \* made[d]: how many operations device d has made
          named,    \* named[d][s]: the length of the log d held when it made operation s
          twin,     \* twin[d][s]: whether a second version of operation s exists
          log,      \* the entries: removals, each of a device at a number
          known,    \* known[h]: how many entries host h has been given
          held,     \* held[h][d]: the <<number, version>> of d's operations host h holds
          given,    \* given[h]: the <<device, number, version>> host h has been given
          frozen    \* frozen[h][d]: from where host h takes nothing of d (MaxSeq + 1: nowhere)

vars == <<made, named, twin, log, known, held, given, frozen>>

Remove(d, c) == [dev |-> d, cut |-> c]

\* Whether an operation of d numbered s, which named entry mh, holds for a host that has `k`
\* entries and is frozen for d from `fr`.
Holds(k, fr, d, s) ==
    /\ s < fr
    /\ \A i \in (named[d][s] + 1)..k : log[i].dev = d => s <= log[i].cut

Init ==
    /\ made   = [d \in Devices |-> 0]
    /\ named  = [d \in Devices |-> [s \in Seqs |-> 0]]
    /\ twin   = [d \in Devices |-> [s \in Seqs |-> FALSE]]
    /\ log    = <<>>
    /\ known  = [h \in Hosts |-> 0]
    /\ held   = [h \in Hosts |-> [d \in Devices |-> {}]]
    /\ given  = [h \in Hosts |-> {}]
    /\ frozen = [h \in Hosts |-> [d \in Devices |-> MaxSeq + 1]]

\* A device makes its next operation, if its own log does not say it was taken off.
Make(d) ==
    /\ made[d] < MaxSeq
    /\ \A i \in 1..known[d] : log[i].dev # d
    /\ made'  = [made EXCEPT ![d] = @ + 1]
    /\ named' = [named EXCEPT ![d][made[d] + 1] = known[d]]
    /\ UNCHANGED <<twin, log, known, held, given, frozen>>

\* A device signs a second operation at a number it has used.
Twin(d, s) ==
    /\ Equivocate
    /\ s <= made[d]
    /\ ~twin[d][s]
    /\ twin' = [twin EXCEPT ![d][s] = TRUE]
    /\ UNCHANGED <<made, named, log, known, held, given, frozen>>

\* The administrator takes a device off at a number it has seen of it, once.
AdminRemove(d, c) ==
    /\ Len(log) < MaxRemovals
    /\ c \in 0..made[d]
    /\ \A i \in 1..Len(log) : log[i].dev # d
    /\ log' = Append(log, Remove(d, c))
    /\ UNCHANGED <<made, named, twin, known, held, given, frozen>>

\* A host is given the next entry. With `Evict` it drops what the entry puts past a cutoff.
GiveEntry(h) ==
    /\ known[h] < Len(log)
    /\ known' = [known EXCEPT ![h] = @ + 1]
    /\ held'  = IF Evict
                THEN [held EXCEPT ![h] = [d \in Devices |->
                        {x \in held[h][d] : Holds(known[h] + 1, frozen[h][d], d, x[1])}]]
                ELSE held
    /\ UNCHANGED <<made, named, twin, log, given, frozen>>

\* A host is given an operation whose entry it has.
Deliver(h, d, s, v) ==
    /\ s <= made[d]
    /\ v = 1 \/ twin[d][s]
    /\ named[d][s] <= known[h]
    /\ given' = [given EXCEPT ![h] = @ \cup {<<d, s, v>>}]
    /\ IF ~Holds(known[h], frozen[h][d], d, s)
       THEN UNCHANGED <<held, frozen>>
       ELSE IF \E x \in held[h][d] : x[1] = s /\ x[2] # v
            THEN \* two operations at one number: the device is frozen from it
                 /\ frozen' = [frozen EXCEPT ![h][d] = s]
                 /\ held'   = IF Evict
                              THEN [held EXCEPT ![h][d] = {x \in @ : x[1] < s}]
                              ELSE held
            ELSE /\ held' = [held EXCEPT ![h][d] = @ \cup {<<s, v>>}]
                 /\ UNCHANGED frozen
    /\ UNCHANGED <<made, named, twin, log, known>>

Next ==
    \/ \E d \in Devices : Make(d)
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

\* No host holds what its own log puts past the last accepted of a device, or past a fork of it.
HeldValid ==
    \A h \in Hosts : \A d \in Devices : \A x \in held[h][d] :
        Holds(known[h], frozen[h][d], d, x[1])

\* Two hosts that have the whole log and have been given every operation hold the same.
Convergence ==
    \A h1, h2 \in Hosts :
        (Saturated(h1) /\ Saturated(h2)) => held[h1] = held[h2]

=============================================================================
