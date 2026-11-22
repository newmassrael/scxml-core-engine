-------------------------- MODULE PrincipalService --------------------------
(***************************************************************************)
(* What a document may rely on of the person an event is told to be of     *)
(* (docs/adr/0115, D1; the promise of docs/adr/0116). No message, key,     *)
(* challenge, lease or store is in it.                                     *)
(*                                                                         *)
(* A person proves who they are FOR an incarnation. A run is kept under an *)
(* incarnation: for a person who proved so for that incarnation, and no    *)
(* other than the one it was kept for before; for nobody; or for nobody    *)
(* with a person who was only quoted, which says nothing of that person.   *)
(* After an incarnation has ended nothing is kept under it. What was made  *)
(* for a person is read by that person.                                    *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Nothing: it is what a user may rely on, and names no mechanism.       *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS Incarnations, Persons, Nobody

VARIABLES proved,   \* the pairs <<person, incarnation>> proved
          ended,    \* the incarnations that ended
          kept,     \* what was kept, as <<incarnation, person or Nobody>>
          read      \* what was read, as <<reader, person it was made for>>

vars == <<proved, ended, kept, read>>

Init == proved = {} /\ ended = {} /\ kept = {} /\ read = {}

Prove == /\ \E p \in Persons, c \in Incarnations : proved' = proved \cup {<<p, c>>}
         /\ UNCHANGED <<ended, kept, read>>

End == /\ \E c \in Incarnations : ended' = ended \cup {c}
       /\ UNCHANGED <<proved, kept, read>>

\* An end may be told together with the beginning of the next incarnation.
Keep == /\ \E c \in Incarnations \ ended, p \in Persons \cup {Nobody} :
             /\ p # Nobody => (<<p, c>> \in proved /\ \A k \in kept : k[1] = c => k[2] \in {p, Nobody})
             /\ kept' = kept \cup {<<c, p>>}
        /\ UNCHANGED <<proved, ended, read>>

Read == /\ \E k \in kept, r \in Persons \cup {Nobody} :
             /\ k[2] \in {r, Nobody}
             /\ read' = read \cup {<<r, k[2]>>}
        /\ UNCHANGED <<proved, ended, kept>>

Spec == Init /\ [][Prove \/ End \/ Keep \/ Read]_vars

=============================================================================
