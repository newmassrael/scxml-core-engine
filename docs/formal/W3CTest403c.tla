---- MODULE W3CTest403c ----
\* The document resources/403/test403c.scxml as the tables of AppendixD
\* (docs/adr/0115, D7), written by `sce-codegen model`: regenerate it, never
\* edit it. Row r of DocTransitions is the transition the generated code
\* numbers r - 1 (sce-build/src/document_tables.rs): the written
\* transitions by state in document order, then the initial transition of
\* each compound state, the default transition of each <history>, and the
\* document's own. A block of executable content is named by where it is
\* written, and a condition by its expression.
EXTENDS AppendixD

DocStates == {"s0", "p0", "p0s1", "p0s2", "p0s3", "p0s4", "s1", "pass", "fail"}

DocOrder == <<"s0", "p0", "p0s1", "p0s2", "p0s3", "p0s4", "s1", "pass",
              "fail">>

DocKind == [s \in DocStates |->
    CASE s = "p0" -> "parallel"
      [] s \in {"pass", "fail"} -> "final"
      [] OTHER -> "state"]

DocParent == [s \in DocStates |->
    CASE s = "p0" -> "s0"
      [] s \in {"p0s1", "p0s2", "p0s3", "p0s4"} -> "p0"
      [] OTHER -> Root]

DocHistoryType == [h \in {} |-> "shallow"]

DocTransitions == <<
    Transition("s0", {<<"event2">>}, NoCond, <<"fail">>, NoContent),
    Transition("s0", {<<"timeout">>}, NoCond, <<"fail">>, NoContent),
    Transition("p0s1", {<<"event1">>}, NoCond, <<>>, NoContent),
    Transition("p0s1", {<<"event2">>}, NoCond, <<>>, NoContent),
    Transition("p0s2", {<<"event1">>}, NoCond, <<"p0s1">>, "states.p0s2.transitions[0]"),
    Transition("p0s3", {<<"event1">>}, NoCond, <<"fail">>, NoContent),
    Transition("p0s3", {<<"event2">>}, NoCond, <<"s1">>, NoContent),
    Transition("p0s4", {<<"*">>}, NoCond, <<>>, "states.p0s4.transitions[0]"),
    Transition("s1", {}, "Var1 == 2", <<"pass">>, NoContent),
    Transition("s1", {}, NoCond, <<"fail">>, NoContent),
    Initial("s0", <<"p0">>, NoContent),
    Initial(Root, <<"s0">>, NoContent)
>>

DocOnEntry == [s \in DocStates |->
    CASE s = "s0" -> "onentry s0"
      [] OTHER -> NoContent]

DocOnExit == [s \in DocStates |-> NoContent]

DocData0 == [Var1 |-> 0]

DocGuard(cond, data, event, conf) ==
    CASE cond = "Var1 == 2" -> data.Var1 = 2

DocExecute(block, data, event) ==
    CASE block = "states.p0s2.transitions[0]" -> Effect(data, <<<<"event2">>>>, <<>>, <<>>)
      [] block = "states.p0s4.transitions[0]" -> Effect([data EXCEPT !.Var1 = data.Var1 + 1], <<>>, <<>>, <<>>)
      [] block = "onentry s0" -> Effect(data, <<<<"event1">>>>, <<>>, <<<<"timeout">>>>)

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
