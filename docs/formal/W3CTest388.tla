---- MODULE W3CTest388 ----
\* The document resources/388/test388.scxml as the tables of AppendixD
\* (docs/adr/0115, D7), written by `sce-codegen model`: regenerate it, never
\* edit it. Row r of DocTransitions is the transition the generated code
\* numbers r - 1 (sce-build/src/document_tables.rs): the written
\* transitions by state in document order, then the initial transition of
\* each compound state, the default transition of each <history>, and the
\* document's own. A block of executable content is named by where it is
\* written, and a condition by its expression.
EXTENDS AppendixD

DocStates == {"s0", "s0HistShallow", "s0HistDeep", "s01", "s011", "s012",
              "s02", "s021", "s022", "s1", "s2", "pass", "fail"}

DocOrder == <<"s0", "s0HistShallow", "s0HistDeep", "s01", "s011", "s012",
              "s02", "s021", "s022", "s1", "s2", "pass", "fail">>

DocKind == [s \in DocStates |->
    CASE s \in {"s0HistShallow", "s0HistDeep"} -> "history"
      [] s \in {"pass", "fail"} -> "final"
      [] OTHER -> "state"]

DocParent == [s \in DocStates |->
    CASE s \in {"s0HistShallow", "s0HistDeep", "s01", "s02"} -> "s0"
      [] s \in {"s011", "s012"} -> "s01"
      [] s \in {"s021", "s022"} -> "s02"
      [] OTHER -> Root]

DocHistoryType == [h \in {"s0HistDeep", "s0HistShallow"} |->
    CASE h = "s0HistDeep" -> "deep"
      [] OTHER -> "shallow"]

DocTransitions == <<
    Transition("s0", {<<"entering", "s012">>}, "Var1 == 1", <<"s1">>, "states.s0.transitions[0]"),
    Transition("s0", {<<"entering", "s012">>}, "Var1 == 2", <<"s2">>, NoContent),
    Transition("s0", {<<"entering">>}, "Var1 == 2", <<"fail">>, NoContent),
    Transition("s0", {<<"entering", "s011">>}, "Var1 == 3", <<"pass">>, NoContent),
    Transition("s0", {<<"entering">>}, "Var1 == 3", <<"fail">>, NoContent),
    Transition("s0", {<<"timeout">>}, NoCond, <<"fail">>, NoContent),
    Transition("s1", {}, NoCond, <<"s0HistDeep">>, NoContent),
    Transition("s2", {}, NoCond, <<"s0HistShallow">>, NoContent),
    Initial("s0", <<"s01">>, NoContent),
    Initial("s01", <<"s011">>, NoContent),
    Initial("s02", <<"s021">>, NoContent),
    HistoryDefault("s0HistDeep", <<"s022">>, NoContent),
    HistoryDefault("s0HistShallow", <<"s02">>, NoContent),
    Initial(Root, <<"s012">>, NoContent)
>>

DocOnEntry == [s \in DocStates |->
    CASE s = "s0" -> "onentry s0"
      [] s = "s011" -> "onentry s011"
      [] s = "s012" -> "onentry s012"
      [] s = "s021" -> "onentry s021"
      [] s = "s022" -> "onentry s022"
      [] OTHER -> NoContent]

DocOnExit == [s \in DocStates |-> NoContent]

DocData0 == [Var1 |-> 0]

DocGuard(cond, data, event, conf) ==
    CASE cond = "Var1 == 1" -> data.Var1 = 1
      [] cond = "Var1 == 2" -> data.Var1 = 2
      [] cond = "Var1 == 3" -> data.Var1 = 3

DocExecute(block, data, event) ==
    CASE block = "states.s0.transitions[0]" -> Effect(data, <<>>, <<>>, <<<<"timeout">>>>)
      [] block = "onentry s0" -> Effect([data EXCEPT !.Var1 = data.Var1 + 1], <<>>, <<>>, <<>>)
      [] block = "onentry s011" -> Effect(data, <<<<"entering", "s011">>>>, <<>>, <<>>)
      [] block = "onentry s012" -> Effect(data, <<<<"entering", "s012">>>>, <<>>, <<>>)
      [] block = "onentry s021" -> Effect(data, <<<<"entering", "s021">>>>, <<>>, <<>>)
      [] block = "onentry s022" -> Effect(data, <<<<"entering", "s022">>>>, <<>>, <<>>)

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
