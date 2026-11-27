---- MODULE W3CTest570 ----
\* The document resources/570/test570.scxml as the tables of AppendixD
\* (docs/adr/0115, D7), written by `sce-codegen model`: regenerate it, never
\* edit it. Row r of DocTransitions is the transition the generated code
\* numbers r - 1 (sce-build/src/document_tables.rs): the written
\* transitions by state in document order, then the initial transition of
\* each compound state, the default transition of each <history>, and the
\* document's own. A block of executable content is named by where it is
\* written, and a condition by its expression.
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
      [] OTHER -> Root]

DocHistoryType == [h \in {} |-> "shallow"]

DocTransitions == <<
    Transition("p0", {<<"done", "state", "p0s1">>}, NoCond, <<>>, "states.p0.transitions[0]"),
    Transition("p0", {<<"done", "state", "p0s2">>}, NoCond, <<"s1">>, NoContent),
    Transition("p0", {<<"timeout">>}, NoCond, <<"fail">>, NoContent),
    Transition("p0s11", {<<"e1">>}, NoCond, <<"p0s1final">>, NoContent),
    Transition("p0s21", {<<"e2">>}, NoCond, <<"p0s2final">>, NoContent),
    Transition("s1", {<<"done", "state", "p0">>}, "Var1 == 1", <<"pass">>, NoContent),
    Transition("s1", {<<"*">>}, NoCond, <<"fail">>, NoContent),
    Initial("p0s1", <<"p0s11">>, NoContent),
    Initial("p0s2", <<"p0s21">>, NoContent),
    Initial(Root, <<"p0">>, NoContent)
>>

DocOnEntry == [s \in DocStates |->
    CASE s = "p0" -> "onentry p0"
      [] OTHER -> NoContent]

DocOnExit == [s \in DocStates |-> NoContent]

DocData0 == [Var1 |-> 0]

DocGuard(cond, data, event, conf) ==
    CASE cond = "Var1 == 1" -> data.Var1 = 1

DocExecute(block, data, event) ==
    CASE block = "states.p0.transitions[0]" -> Effect([data EXCEPT !.Var1 = 1], <<>>, <<>>, <<>>)
      [] block = "onentry p0" -> Effect(data, <<<<"e1">>, <<"e2">>>>, <<>>, <<<<"timeout">>>>)

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
