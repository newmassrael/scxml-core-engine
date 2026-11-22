---------------------------- MODULE JournalService ----------------------------
(***************************************************************************)
(* What a reader of a journal may rely on (docs/adr/0115, D1; the promise  *)
(* of docs/adr/0114). No transaction, position, lease or clock is in it.   *)
(*                                                                         *)
(* The journal is a sequence that only grows: an entry, once there, stays  *)
(* where it is, so what a reader asked as of any point does not change. No *)
(* entry is in it twice, and a session's entries are in the order the      *)
(* session made them. A reader takes the entries in order, each once, and  *)
(* passes none.                                                            *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Nothing: it is what a user may rely on, and names no mechanism.       *)
(***************************************************************************)
EXTENDS Naturals, Sequences

CONSTANTS Streams, MaxEntry

VARIABLES journal,   \* the entries, as <<stream, number>>
          taken,     \* how many of them the reader has taken
          did        \* did[e]: how many times entry e took effect in the reader

vars == <<journal, taken, did>>

Entry == Streams \X (1..MaxEntry)

Grows(old, new) == Len(old) <= Len(new) /\ SubSeq(new, 1, Len(old)) = old

Distinct(s) == \A i, j \in 1..Len(s) : s[i] = s[j] => i = j

InOrder(s) == \A i, j \in 1..Len(s) : (i < j /\ s[i][1] = s[j][1]) => s[i][2] < s[j][2]

Init == journal = <<>> /\ taken = 0 /\ did = [e \in Entry |-> 0]

\* Entries become visible, some at once: a transaction that committed early may hold back several.
Extend == /\ Grows(journal, journal') /\ journal' # journal
          /\ Distinct(journal') /\ InOrder(journal')
          /\ taken' = taken /\ did' = did

Take == /\ taken < Len(journal) /\ taken' = taken + 1 /\ journal' = journal
        /\ did' = [did EXCEPT ![journal[taken + 1]] = @ + 1]

Spec == Init /\ [][Extend \/ Take]_vars

=============================================================================
