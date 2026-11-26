---- MODULE W3CTest570 ----
(***************************************************************************)
(* The W3C test 570 (resources/570/test570.scxml) as tables of AppendixD,  *)
(* written as sce-codegen model will write them (docs/adr/0115, D7): each  *)
(* region of the parallel state p0 reaches its final state and raises the  *)
(* done event of its compound state, and the second raises the done event  *)
(* of p0 after its own. The test passes in s1 on done.state.p0, once       *)
(* done.state.p0s1 has set Var1.                                           *)
(*                                                                         *)
(* A block of executable content is named by where it is; a condition by   *)
(* its expression. The delayed send of timeout is the only event from      *)
(* outside, and the test ends before it is due.                            *)
(***************************************************************************)
EXTENDS AppendixD

DocStates == {"p0", "p0s1", "p0s11", "p0s1final", "p0s2", "p0s21", "p0s2final",
              "s1", "pass", "fail"}

DocOrder == <<"p0", "p0s1", "p0s11", "p0s1final", "p0s2", "p0s21", "p0s2final",
              "s1", "pass", "fail">>

DocKind == [s \in DocStates |->
              CASE s = "p0" -> "parallel"
                [] s \in {"p0s1final", "p0s2final", "pass", "fail"} -> "final"
                [] OTHER -> "state"]

DocParent == [s \in DocStates |->
                CASE s \in {"p0s1", "p0s2"} -> "p0"
                  [] s \in {"p0s11", "p0s1final"} -> "p0s1"
                  [] s \in {"p0s21", "p0s2final"} -> "p0s2"
                  [] OTHER -> "scxml"]

DocHistoryType == [h \in {} |-> "shallow"]

DocTransitions == <<
    Initial("scxml", <<"p0">>, NoContent),
    Transition("p0", {<<"done", "state", "p0s1">>}, NoCond, <<>>, "transition 2"),
    Transition("p0", {<<"done", "state", "p0s2">>}, NoCond, <<"s1">>, NoContent),
    Transition("p0", {<<"timeout">>}, NoCond, <<"fail">>, NoContent),
    Initial("p0s1", <<"p0s11">>, NoContent),
    Transition("p0s11", {<<"e1">>}, NoCond, <<"p0s1final">>, NoContent),
    Initial("p0s2", <<"p0s21">>, NoContent),
    Transition("p0s21", {<<"e2">>}, NoCond, <<"p0s2final">>, NoContent),
    Transition("s1", {<<"done", "state", "p0">>}, "Var1 == 1", <<"pass">>, NoContent),
    Transition("s1", {<<"*">>}, NoCond, <<"fail">>, NoContent)
>>

DocOnEntry == [s \in DocStates |-> IF s = "p0" THEN "onentry p0" ELSE NoContent]

DocOnExit == [s \in DocStates |-> NoContent]

DocData0 == [Var1 |-> 0]

DocGuard(cond, data, event, conf) ==
    CASE cond = "Var1 == 1" -> data.Var1 = 1

DocExecute(block, data, event) ==
    CASE block = "onentry p0" -> Effect(data, <<<<"e1">>, <<"e2">>>>, <<>>, <<<<"timeout">>>>)
      [] block = "transition 2" -> Effect([data EXCEPT !.Var1 = 1], <<>>, <<>>, <<>>)

\* Appendix D with every part in, over the same tables: what each part taken
\* out must break, beside the invariant it names.
Promise == INSTANCE AppendixD WITH
    ExitOrder <- TRUE, EntryOrder <- TRUE, RemoveConflicts <- TRUE,
    EnterAncestors <- TRUE, FillParallel <- TRUE, RecordHistory <- TRUE,
    DeepHistory <- TRUE, DoneCompound <- TRUE, DoneParallel <- TRUE,
    RunToCompletion <- TRUE
Refines == Promise!Spec

\* The test's verdict: the session never ends in fail, and it ends in pass.
NeverFails == outcome # "fail"
Passes == <>(outcome = "pass")
====
