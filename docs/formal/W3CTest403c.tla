---- MODULE W3CTest403c ----
(***************************************************************************)
(* W3C test 403c (resources/403/test403c.scxml) as tables of AppendixD,    *)
(* written as sce-codegen model will write them (docs/adr/0115, D7): of    *)
(* the transitions the regions of p0 select, one that leaves p0 is         *)
(* preempted by an earlier one in document order, and one that a region    *)
(* inherits from s0 is preempted by a later one in a descendant. The       *)
(* targetless transition of p0s4 is taken with both, and counts in Var1.   *)
(*                                                                         *)
(* A block of executable content is named by where it is; a condition by   *)
(* its expression. The delayed send of timeout is the only event from      *)
(* outside, and the test ends before it is due.                            *)
(***************************************************************************)
EXTENDS AppendixD

DocStates == {"s0", "p0", "p0s1", "p0s2", "p0s3", "p0s4", "s1", "pass", "fail"}

DocOrder == <<"s0", "p0", "p0s1", "p0s2", "p0s3", "p0s4", "s1", "pass", "fail">>

DocKind == [s \in DocStates |->
              CASE s = "p0" -> "parallel"
                [] s \in {"pass", "fail"} -> "final"
                [] OTHER -> "state"]

DocParent == [s \in DocStates |->
                CASE s = "p0" -> "s0"
                  [] s \in {"p0s1", "p0s2", "p0s3", "p0s4"} -> "p0"
                  [] OTHER -> "scxml"]

DocHistoryType == [h \in {} |-> "shallow"]

DocTransitions == <<
    Initial("scxml", <<"s0">>, NoContent),
    Initial("s0", <<"p0">>, NoContent),
    Transition("s0", {<<"event2">>}, NoCond, <<"fail">>, NoContent),
    Transition("s0", {<<"timeout">>}, NoCond, <<"fail">>, NoContent),
    Transition("p0s1", {<<"event1">>}, NoCond, <<>>, NoContent),
    Transition("p0s1", {<<"event2">>}, NoCond, <<>>, NoContent),
    Transition("p0s2", {<<"event1">>}, NoCond, <<"p0s1">>, "transition 7"),
    Transition("p0s3", {<<"event1">>}, NoCond, <<"fail">>, NoContent),
    Transition("p0s3", {<<"event2">>}, NoCond, <<"s1">>, NoContent),
    Transition("p0s4", {<<"*">>}, NoCond, <<>>, "transition 10"),
    Transition("s1", {}, "Var1 == 2", <<"pass">>, NoContent),
    Transition("s1", {}, NoCond, <<"fail">>, NoContent)
>>

DocOnEntry == [s \in DocStates |-> IF s = "s0" THEN "onentry s0" ELSE NoContent]

DocOnExit == [s \in DocStates |-> NoContent]

DocData0 == [Var1 |-> 0]

DocGuard(cond, data, event, conf) ==
    CASE cond = "Var1 == 2" -> data.Var1 = 2

DocExecute(block, data, event) ==
    CASE block = "onentry s0" -> Effect(data, <<<<"event1">>>>, <<>>, <<<<"timeout">>>>)
      [] block = "transition 7" -> Effect(data, <<<<"event2">>>>, <<>>, <<>>)
      [] block = "transition 10" -> Effect([data EXCEPT !.Var1 = @ + 1], <<>>, <<>>, <<>>)

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
