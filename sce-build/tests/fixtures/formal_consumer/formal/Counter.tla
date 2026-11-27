---- MODULE Counter ----
(* A model shaped as a consumer's: a manifest and its modules in a directory  *)
(* of their own, judged by sce-codegen formal models from outside this        *)
(* repository's docs/formal (docs/adr/0146, D6). A path the judge assumes of  *)
(* this repository fails here before it fails a consumer.                    *)
(*                                                                            *)
(* Taken as given: nothing; the counter is the whole system.                 *)
EXTENDS Naturals

CONSTANTS
    \* @type: Int;
    Limit,      \* the largest value the counter may hold
    \* @type: Bool;
    Bounded     \* the part: the guard that stops the counter at its limit

VARIABLE
    \* @type: Int;
    n

\* @type: <<Int>>;
vars == <<n>>

Init == n = 0

Next == /\ (Bounded => n < Limit)
        /\ n' = (n + 1) % (Limit + 2)

Spec == Init /\ [][Next]_vars

WithinLimit == n <= Limit

\* WithinLimit for every length of execution (docs/adr/0115, D3): the counter
\* starts in its range, and a step from the range stays in it. It gives the
\* variable a value (n \in S), so Apalache starts the step from it.
Inductive == n \in 0..Limit
====
