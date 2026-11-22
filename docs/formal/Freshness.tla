----------------------------- MODULE Freshness -----------------------------
(***************************************************************************)
(* The freshness step of ADR 0117, D4: a request signed while someone was  *)
(* inside a page is of no use long after they are out of it.               *)
(*                                                                         *)
(* The server makes a value of a key of its own and the present step of    *)
(* time, gives it with each answer, and takes a request that signs the     *)
(* value of this step or the one before. A thief inside the page signs     *)
(* what they like with the page's key, with every value the page was       *)
(* given, keeps it, and sends it at any later moment. They cannot make a   *)
(* value of a step that has not come.                                      *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   A value of a step is known to nobody before the server gives it.      *)
(*   The key does not leave the page, and a signature cannot be made       *)
(*     without it.                                                         *)
(*   LiveSpec: what can be done is done.                                   *)
(***************************************************************************)
EXTENDS Naturals

CONSTANTS MaxStep,
          Step,        \* a request signs the step, and an older one is refused
          Tell         \* a refusal carries the present value

VARIABLES now, inside, left, given, taken, has, late, sent

vars == <<now, inside, left, given, taken, has, late, sent>>

Init == /\ now = 1 /\ inside = FALSE /\ left = 0      \* the last step a thief was inside
        /\ given = {}                                 \* the steps whose value the page was given
        /\ taken = {}                                 \* the steps a thief signed requests for
        /\ has = 0                                    \* the step the page would sign
        /\ late = 0                                   \* how long after it left a thief was served
        /\ sent = FALSE                               \* the page's own request was taken

Tick == now < MaxStep /\ now' = now + 1 /\ UNCHANGED <<inside, left, given, taken, has, late, sent>>

Enter == ~inside /\ inside' = TRUE /\ left' = now /\ UNCHANGED <<now, given, taken, has, late, sent>>

Stay == inside /\ left # now /\ left' = now /\ UNCHANGED <<now, inside, given, taken, has, late, sent>>

Leave == inside /\ left = now /\ inside' = FALSE /\ UNCHANGED <<now, left, given, taken, has, late, sent>>

Good(s) == ~Step \/ s \in {now, now - 1}

\* The page's exchange: taken with a good value, or refused and given the present one.
Exchange ==
    /\ ~sent
    /\ IF Good(has) THEN sent' = TRUE /\ UNCHANGED <<has, given>>
       ELSE /\ Tell /\ has' = now /\ given' = given \cup {now} /\ UNCHANGED sent
    /\ UNCHANGED <<now, inside, left, taken, late>>

\* Inside, a thief asks for the present value as the page would, and signs for it.
Take ==
    /\ inside /\ left = now /\ taken' = taken \cup given \cup {now} /\ given' = given \cup {now}
    /\ UNCHANGED <<now, inside, left, has, late, sent>>

Use ==
    /\ \E s \in taken : Good(s)
    /\ late' = IF now - left > late THEN now - left ELSE late
    /\ UNCHANGED <<now, inside, left, given, taken, has, sent>>

Next == Tick \/ Enter \/ Stay \/ Leave \/ Exchange \/ Take \/ Use

Spec == Init /\ [][Next]_vars

LiveSpec == Spec /\ WF_vars(Exchange) /\ WF_vars(Tick)

\* What a thief signed serves them in the step after the last one they were inside, and no later.
Bounded == late <= 1

\* The page's own request is taken.
Taken == <>sent

=============================================================================
