---- MODULE AppendixD ----
(***************************************************************************)
(* The algorithm of the SCXML recommendation's Appendix D (docs/adr/0115,  *)
(* D7), once, over the tables of one document. Each operator is the        *)
(* procedure or function of the same name, cited by its section, and runs  *)
(* as the recommendation's text runs it: interpret, mainEventLoop with its *)
(* macrostep of microsteps, selectEventlessTransitions, selectTransitions, *)
(* removeConflictingTransitions, exitStates, computeExitSet,               *)
(* executeTransitionContent, enterStates, computeEntrySet,                 *)
(* addDescendantStatesToEnter, addAncestorStatesToEnter, isInFinalState,   *)
(* getTransitionDomain, findLCCA, getEffectiveTargetStates,                *)
(* getProperAncestors, the internal and external queues, history recorded  *)
(* and restored, the done events of a compound state, of a parallel state  *)
(* and of the top level, and exitInterpreter.                              *)
(*                                                                         *)
(* A document brings tables, as sce-codegen model writes them: its states  *)
(* with their kind, parent and document order, its history kinds, and its  *)
(* transitions by row (source, event descriptors, condition, targets,      *)
(* type, content), the initial transitions and the default transition of   *)
(* each history being rows too. A module of one document EXTENDS this one  *)
(* and gives each constant by substitution.                                *)
(*                                                                         *)
(* The parts, each a constant that is TRUE in the recommendation, and each *)
(* taken out by a run of a document's module that must find what it        *)
(* breaks:                                                                 *)
(*   ExitOrder        states are exited in exitOrder, else document order  *)
(*   EntryOrder       states are entered in entryOrder, else reversed      *)
(*   RemoveConflicts  removeConflictingTransitions filters the selection   *)
(*   EnterAncestors   computeEntrySet enters the ancestors of the targets  *)
(*                    within the transition's domain                       *)
(*   FillParallel     entering a parallel state enters each child          *)
(*   RecordHistory    exitStates records the history of an exited state    *)
(*   DeepHistory      a deep history records atomic descendants, else the  *)
(*                    children, as a shallow one does                      *)
(*   DoneCompound     a final child raises done.state of its parent        *)
(*   DoneParallel     the last final region raises done.state of the       *)
(*                    parallel state                                       *)
(*   RunToCompletion  an external event is taken only when the macrostep   *)
(*                    is done                                              *)
(*                                                                         *)
(* One step is a microstep, the taking of an event that enables nothing,   *)
(* the start or the exit of the interpreter, or a step of the environment. *)
(* told is a history variable: what the last step exited, which rows it    *)
(* took and what it entered, in order, as a trace of a run tells it.       *)
(*                                                                         *)
(* Taken as given: a condition is a function of the datamodel, the current *)
(* event and the configuration, and a block of executable content is a     *)
(* function of the datamodel and the current event, both named by the      *)
(* document (Guard, Execute) over a finite domain it names, so a block     *)
(* that fails raises what the document says it raises; the datamodel is    *)
(* bound early; a delayed send is due at any moment after it is made,      *)
(* since time is D15's, and no send is cancelled; an event from outside is *)
(* any of Outside, at most MaxOutside of them. Not modelled, and refused   *)
(* by name where a document has one: invoke, finalize and autoforward, the *)
(* cancel event of a parent session, late binding, a send to another       *)
(* session, and In() inside executable content.                            *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS
    States,             \* the <state>, <parallel>, <final> and <history> ids
    Root,               \* the <scxml> element, not one of States
    Kind,               \* [States -> {"state", "parallel", "final", "history"}]
    Parent,             \* [States -> States \cup {Root}]
    DocumentOrder,      \* every state once, in document order
    HistoryType,        \* [the history states -> {"shallow", "deep"}]
    Transitions,        \* the rows, in document order
    OnEntry, OnExit,    \* [States -> the block of each, or NoContent]
    Data0,              \* the datamodel as the document initializes it
    Guard(_, _, _, _),  \* Guard(cond, datamodel, event, configuration)
    Execute(_, _, _),   \* Execute(block, datamodel, event): an Effect
    Outside,            \* the events that may come from outside
    MaxOutside,         \* the most of them that come
    ExitOrder, EntryOrder, RemoveConflicts, EnterAncestors, FillParallel,
    RecordHistory, DeepHistory, DoneCompound, DoneParallel, RunToCompletion

VARIABLES
    configuration,      \* the active states
    historyValue,       \* [the history states -> the states recorded, or {}]
    datamodel,
    internalQueue,
    externalQueue,
    delayed,            \* the delayed sends not yet due, in the order made
    currentEvent,       \* datamodel["_event"]: the event being processed
    phase,              \* "start", "loop" (running), "exit", "end"
    told,               \* what the last step did, in order
    arrived,            \* how many events have come from outside
    outcome             \* the top-level final state the session ended in

vars == <<configuration, historyValue, datamodel, internalQueue, externalQueue,
          delayed, currentEvent, phase, told, arrived, outcome>>

\* What getProperAncestors(s, null) is given, and the outcome before the end.
\* It is no XML name, so it is no document's state.
None == "<none>"
NoCond == ""
NoContent == ""
\* An event's name is the sequence of its tokens; the empty one is no event,
\* and selects the eventless transitions.
NoEvent == <<>>

\* The rows of a document's transition table, as the generator writes them.
Transition(source, events, cond, target, content) ==
    [source |-> source, kind |-> "transition", events |-> events, cond |-> cond,
     target |-> target, type |-> "external", content |-> content]
InternalTransition(source, events, cond, target, content) ==
    [Transition(source, events, cond, target, content) EXCEPT !.type = "internal"]
\* The <initial> transition of a compound state, or of the document when the
\* source is Root (§scxml-D-interpret converts each initial attribute to one).
Initial(source, target, content) ==
    [Transition(source, {}, NoCond, target, content) EXCEPT !.kind = "initial"]
HistoryDefault(history, target, content) ==
    [Transition(history, {}, NoCond, target, content) EXCEPT !.kind = "default"]

\* What a block of executable content does: the datamodel after it, and the
\* events it raises, sends to the external queue, and sends with a delay.
Effect(data, raised, sent, later) ==
    [data |-> data, raise |-> raised, send |-> sent, delay |-> later]

Histories == {s \in States : Kind[s] = "history"}
Rows == 1..Len(Transitions)

ASSUME
    /\ Root \notin States
    /\ None \notin States \cup {Root}
    /\ DOMAIN Kind = States
    /\ \A s \in States : Kind[s] \in {"state", "parallel", "final", "history"}
    /\ DOMAIN Parent = States
    /\ \A s \in States : Parent[s] \in States \cup {Root}
    /\ Len(DocumentOrder) = Cardinality(States)
    /\ \A s \in States : \E i \in 1..Len(DocumentOrder) : DocumentOrder[i] = s
    /\ DOMAIN HistoryType = Histories
    /\ \A r \in Rows : Transitions[r].source \in States \cup {Root}

Range(sq) == {sq[i] : i \in DOMAIN sq}
Reverse(sq) == [i \in 1..Len(sq) |-> sq[Len(sq) - i + 1]]

\* §scxml-D-getChildStates: the <state>, <final> and <parallel> children.
ChildrenOf == [s \in States \cup {Root} |->
                 {c \in States : Parent[c] = s /\ Kind[c] # "history"}]

IsHistory(s) == Kind[s] = "history"
IsFinal(s) == Kind[s] = "final"
IsParallel(s) == Kind[s] = "parallel"
IsCompound(s) == Kind[s] = "state" /\ ChildrenOf[s] # {}
IsAtomic(s) == IsFinal(s) \/ (Kind[s] = "state" /\ ChildrenOf[s] = {})
IsCompoundOrScxml(s) == s = Root \/ IsCompound(s)

RECURSIVE Ancestry(_)
Ancestry(s) == IF s = Root THEN {} ELSE {Parent[s]} \cup Ancestry(Parent[s])
AncestorsOf == [s \in States \cup {Root} |-> Ancestry(s)]

\* §scxml-D-isDescendant
IsDescendant(s1, s2) == s2 \in AncestorsOf[s1]

RECURSIVE Climb(_, _)
Climb(s, stop) ==
    IF s = Root \/ Parent[s] = stop THEN <<>> ELSE <<Parent[s]>> \o Climb(Parent[s], stop)

\* §scxml-D-getProperAncestors: in ancestry order, up to and including the
\* scxml element when stop is None, else up to stop and without it; nothing
\* when stop is s, its parent, or a descendant of it.
ProperAncestors(s, stop) ==
    IF stop = None \/ IsDescendant(s, stop) THEN Climb(s, stop) ELSE <<>>

\* A set of states in document order, which is entryOrder; exitOrder is its
\* reverse (§scxml-D-enterStates, §scxml-D-exitStates).
InDocumentOrder(S) == SelectSeq(DocumentOrder, LAMBDA x : x \in S)
EntrySequence(S) == IF EntryOrder THEN InDocumentOrder(S) ELSE Reverse(InDocumentOrder(S))
ExitSequence(S) == IF ExitOrder THEN Reverse(InDocumentOrder(S)) ELSE InDocumentOrder(S)

RowSeq == [r \in Rows |-> r]
\* s.transition.sort(documentOrder)
TransitionsOf(s) ==
    SelectSeq(RowSeq, LAMBDA r : Transitions[r].source = s /\ Transitions[r].kind = "transition")
InitialOf == [s \in {x \in States : IsCompound(x)} \cup {Root} |->
                CHOOSE r \in Rows : Transitions[r].kind = "initial" /\ Transitions[r].source = s]
DefaultOf == [h \in Histories |->
                CHOOSE r \in Rows : Transitions[r].kind = "default" /\ Transitions[r].source = h]

\* The global variables of the algorithm as one value, which each procedure
\* below takes and gives (§scxml-D-GlobalVariables); running is phase = "loop".
Machine == [conf |-> configuration, hist |-> historyValue, data |-> datamodel,
            intq |-> internalQueue, extq |-> externalQueue, delayed |-> delayed,
            event |-> currentEvent, phase |-> phase]
NoTold == [exited |-> <<>>, fired |-> <<>>, entered |-> <<>>]

\* executeContent of one block: the queues take what it raises and sends, in
\* the order it makes them.
RunBlock(b, m) ==
    IF b = NoContent THEN m
    ELSE LET r == Execute(b, m.data, m.event)
         IN [m EXCEPT !.data = r.data, !.intq = @ \o r.raise,
                      !.extq = @ \o r.send, !.delayed = @ \o r.delay]

RECURSIVE RunBlocks(_, _)
RunBlocks(bs, m) == IF bs = <<>> THEN m ELSE RunBlocks(Tail(bs), RunBlock(Head(bs), m))

\* §scxml-3.12.1: a descriptor matches a name whose tokens it is, or a prefix
\* of them; "*" matches every name. A descriptor that ends in ".*" is written
\* without it.
NameMatch(descriptors, name) ==
    \E d \in descriptors : d = <<"*">> \/ (Len(d) <= Len(name) /\ SubSeq(name, 1, Len(d)) = d)

ConditionMatch(t, m) ==
    Transitions[t].cond = NoCond \/ Guard(Transitions[t].cond, m.data, m.event, m.conf)

EnabledBy(t, e, m) ==
    IF e = NoEvent
    THEN Transitions[t].events = {} /\ ConditionMatch(t, m)
    ELSE Transitions[t].events # {} /\ NameMatch(Transitions[t].events, e) /\ ConditionMatch(t, m)

\* The first transition, in document order, of the first state of ss that has
\* one enabled by e, or 0.
RECURSIVE FirstEnabled(_, _, _)
FirstEnabled(ss, e, m) ==
    IF ss = <<>> THEN 0
    ELSE LET here == SelectSeq(TransitionsOf(Head(ss)), LAMBDA t : EnabledBy(t, e, m))
         IN IF here # <<>> THEN Head(here) ELSE FirstEnabled(Tail(ss), e, m)

\* OrderedSet.add of each of xs, in order.
RECURSIVE OrderedUnion(_, _)
OrderedUnion(acc, xs) ==
    IF xs = <<>> THEN acc
    ELSE OrderedUnion(IF Head(xs) \in Range(acc) THEN acc ELSE Append(acc, Head(xs)), Tail(xs))

\* §scxml-D-getEffectiveTargetStates
RECURSIVE EffectiveTargets(_, _)
EffectiveTargets(targets, m) ==
    UNION {IF IsHistory(s)
           THEN IF m.hist[s] # {} THEN m.hist[s]
                ELSE EffectiveTargets(Transitions[DefaultOf[s]].target, m)
           ELSE {s} : s \in Range(targets)}

\* §scxml-D-findLCCA
FindLCCA(source, targets) ==
    Head(SelectSeq(ProperAncestors(source, None),
                   LAMBDA a : IsCompoundOrScxml(a) /\ \A s \in targets : IsDescendant(s, a)))

\* §scxml-D-getTransitionDomain. The document's <initial> transition has the
\* scxml element as its only ancestor, which is so its domain.
TransitionDomain(t, m) ==
    LET row == Transitions[t]
        tstates == EffectiveTargets(row.target, m)
    IN IF tstates = {} THEN None
       ELSE IF row.source = Root THEN Root
       ELSE IF /\ row.type = "internal"
               /\ IsCompound(row.source)
               /\ \A s \in tstates : IsDescendant(s, row.source)
            THEN row.source
       ELSE FindLCCA(row.source, tstates)

\* §scxml-D-computeExitSet
ComputeExitSet(ts, m) ==
    UNION {{s \in m.conf : IsDescendant(s, TransitionDomain(ts[i], m))} :
           i \in {j \in 1..Len(ts) : Transitions[ts[j]].target # <<>>}}

Conflict(t1, t2, m) == ComputeExitSet(<<t1>>, m) \cap ComputeExitSet(<<t2>>, m) # {}

\* t1 is preempted by a conflicting t2 whose source it does not descend from;
\* else it preempts every conflicting t2, which all are then its ancestors'.
RECURSIVE Filtered(_, _, _)
Filtered(kept, rest, m) ==
    IF rest = <<>> THEN kept
    ELSE LET t1 == Head(rest)
             against == SelectSeq(kept, LAMBDA t2 : Conflict(t1, t2, m))
             preempted == \E i \in 1..Len(against) :
                              ~IsDescendant(Transitions[t1].source, Transitions[against[i]].source)
             filtered == IF preempted THEN kept
                         ELSE Append(SelectSeq(kept, LAMBDA t2 : ~Conflict(t1, t2, m)), t1)
         IN Filtered(filtered, Tail(rest), m)

\* §scxml-D-removeConflictingTransitions
RemoveConflictingTransitions(ts, m) == IF RemoveConflicts THEN Filtered(<<>>, ts, m) ELSE ts

\* For each atomic state in document order, the first enabled transition of it
\* and then of its ancestors in ancestry order.
Select(e, m) ==
    LET atomic == InDocumentOrder({s \in m.conf : IsAtomic(s)})
        picked == [i \in 1..Len(atomic) |->
                     FirstEnabled(<<atomic[i]>> \o ProperAncestors(atomic[i], None), e, m)]
    IN RemoveConflictingTransitions(OrderedUnion(<<>>, SelectSeq(picked, LAMBDA t : t # 0)), m)

\* §scxml-D-selectEventlessTransitions
SelectEventlessTransitions(m) == Select(NoEvent, m)

\* §scxml-D-selectTransitions
SelectTransitions(e, m) == Select(e, m)

\* The history value of h when its parent is exited from configuration conf.
Recorded(h, conf) ==
    IF HistoryType[h] = "deep" /\ DeepHistory
    THEN {s \in conf : IsAtomic(s) /\ IsDescendant(s, Parent[h])}
    ELSE {s \in conf : Parent[s] = Parent[h]}

\* §scxml-D-exitStates: the history of each exited state is recorded from the
\* configuration before any is exited; then each runs its onexit and leaves.
ExitStates(ts, m) ==
    LET toExit == ComputeExitSet(ts, m)
        order == ExitSequence(toExit)
        hist == [h \in Histories |->
                   IF RecordHistory /\ Parent[h] \in toExit THEN Recorded(h, m.conf) ELSE m.hist[h]]
        ran == RunBlocks([i \in 1..Len(order) |-> OnExit[order[i]]], [m EXCEPT !.hist = hist])
    IN [m |-> [ran EXCEPT !.conf = @ \ toExit], exited |-> order]

\* The entry of a microstep is accumulated in a record: statesToEnter (enter),
\* statesForDefaultEntry (defaultEntry) and defaultHistoryContent
\* (historyContent, by the parent of the history).
RECURSIVE AddEach(_, _, _), AddAncestorsOf(_, _, _, _), AddDescendantsOf(_, _, _),
          FillChildren(_, _, _), AddDescendants(_, _, _)

AddEach(ancestors, acc, m) ==
    IF ancestors = <<>> THEN acc
    ELSE LET anc == Head(ancestors)
             added == [acc EXCEPT !.enter = @ \cup {anc}]
             filled == IF anc # Root /\ IsParallel(anc) /\ FillParallel
                       THEN FillChildren(InDocumentOrder(ChildrenOf[anc]), added, m)
                       ELSE added
         IN AddEach(Tail(ancestors), filled, m)

\* §scxml-D-addAncestorStatesToEnter
AddAncestors(s, ancestor, acc, m) == AddEach(ProperAncestors(s, ancestor), acc, m)

AddAncestorsOf(ss, ancestor, acc, m) ==
    IF ss = <<>> THEN acc
    ELSE AddAncestorsOf(Tail(ss), ancestor, AddAncestors(Head(ss), ancestor, acc, m), m)

AddDescendantsOf(ss, acc, m) ==
    IF ss = <<>> THEN acc ELSE AddDescendantsOf(Tail(ss), AddDescendants(Head(ss), acc, m), m)

\* The children of a parallel state, in document order, each that has no
\* descendant to enter yet.
FillChildren(cs, acc, m) ==
    IF cs = <<>> THEN acc
    ELSE LET c == Head(cs)
             acc1 == IF \E s \in acc.enter : IsDescendant(s, c) THEN acc ELSE AddDescendants(c, acc, m)
         IN FillChildren(Tail(cs), acc1, m)

\* §scxml-D-addDescendantStatesToEnter. A history value is taken in document
\* order; what is entered does not depend on the order it is added in.
AddDescendants(s, acc, m) ==
    IF IsHistory(s)
    THEN IF m.hist[s] # {}
         THEN LET vs == InDocumentOrder(m.hist[s])
              IN AddAncestorsOf(vs, Parent[s], AddDescendantsOf(vs, acc, m), m)
         ELSE LET d == Transitions[DefaultOf[s]]
                  noted == [acc EXCEPT !.historyContent[Parent[s]] = d.content]
              IN AddAncestorsOf(d.target, Parent[s], AddDescendantsOf(d.target, noted, m), m)
    ELSE LET added == [acc EXCEPT !.enter = @ \cup {s}]
         IN IF IsCompound(s)
            THEN LET i == Transitions[InitialOf[s]]
                     dflt == [added EXCEPT !.defaultEntry = @ \cup {s}]
                 IN AddAncestorsOf(i.target, s, AddDescendantsOf(i.target, dflt, m), m)
            ELSE IF IsParallel(s) /\ FillParallel
            THEN FillChildren(InDocumentOrder(ChildrenOf[s]), added, m)
            ELSE added

RECURSIVE EntryFor(_, _, _)
EntryFor(ts, acc, m) ==
    IF ts = <<>> THEN acc
    ELSE LET row == Transitions[Head(ts)]
             down == AddDescendantsOf(row.target, acc, m)
             up == IF EnterAncestors
                   THEN AddAncestorsOf(InDocumentOrder(EffectiveTargets(row.target, m)),
                                       TransitionDomain(Head(ts), m), down, m)
                   ELSE down
         IN EntryFor(Tail(ts), up, m)

\* §scxml-D-computeEntrySet
ComputeEntrySet(ts, m) ==
    EntryFor(ts, [enter |-> {}, defaultEntry |-> {},
                  historyContent |-> [s \in States |-> NoContent]], m)

\* §scxml-D-isInFinalState
RECURSIVE InFinalState(_, _)
InFinalState(s, conf) ==
    IF IsCompound(s) THEN \E c \in ChildrenOf[s] : IsFinal(c) /\ c \in conf
    ELSE IF IsParallel(s) THEN \A c \in ChildrenOf[s] : InFinalState(c, conf)
    ELSE FALSE

DoneEvent(s) == <<"done", "state", s>>

\* The end of enterStates for a final state s: the top level stops running;
\* else done.state of its parent, and of the parallel state above it once
\* every region of that state is in a final state.
EnteredFinal(s, m) ==
    IF Parent[s] = Root THEN [m EXCEPT !.phase = "exit"]
    ELSE LET p == Parent[s]
             gp == Parent[p]
             m1 == IF DoneCompound THEN [m EXCEPT !.intq = Append(@, DoneEvent(p))] ELSE m
         IN IF /\ gp # Root
               /\ DoneParallel
               /\ IsParallel(gp)
               /\ \A c \in ChildrenOf[gp] : InFinalState(c, m1.conf)
            THEN [m1 EXCEPT !.intq = Append(@, DoneEvent(gp))]
            ELSE m1

RECURSIVE EnterEach(_, _, _)
EnterEach(order, entry, m) ==
    IF order = <<>> THEN m
    ELSE LET s == Head(order)
             m1 == RunBlock(OnEntry[s], [m EXCEPT !.conf = @ \cup {s}])
             m2 == IF s \in entry.defaultEntry
                   THEN RunBlock(Transitions[InitialOf[s]].content, m1)
                   ELSE m1
             m3 == RunBlock(entry.historyContent[s], m2)
             m4 == IF IsFinal(s) THEN EnteredFinal(s, m3) ELSE m3
         IN EnterEach(Tail(order), entry, m4)

\* §scxml-D-enterStates
EnterStates(ts, m) ==
    LET entry == ComputeEntrySet(ts, m)
        order == EntrySequence(entry.enter)
    IN [m |-> EnterEach(order, entry, m), entered |-> order]

\* §scxml-D-microstepProcedure: exit, §scxml-D-executeTransitionContent, enter.
Microstep(ts, m) ==
    LET exited == ExitStates(ts, m)
        ran == RunBlocks([i \in 1..Len(ts) |-> Transitions[ts[i]].content], exited.m)
        entered == EnterStates(ts, ran)
    IN [m |-> entered.m,
        told |-> [exited |-> exited.exited, fired |-> ts, entered |-> entered.entered]]

\* The event m.event, dequeued: the microstep of what it enables, or none.
TakeEvent(m) ==
    LET ts == SelectTransitions(m.event, m)
    IN IF ts = <<>> THEN [m |-> m, told |-> NoTold] ELSE Microstep(ts, m)

MacrostepDone(m) == SelectEventlessTransitions(m) = <<>> /\ m.intq = <<>>

Becomes(r) ==
    /\ configuration' = r.m.conf
    /\ historyValue' = r.m.hist
    /\ datamodel' = r.m.data
    /\ internalQueue' = r.m.intq
    /\ externalQueue' = r.m.extq
    /\ delayed' = r.m.delayed
    /\ currentEvent' = r.m.event
    /\ phase' = r.m.phase
    /\ told' = r.told

\* §scxml-D-interpret, before enterStates.
Init ==
    /\ configuration = {}
    /\ historyValue = [h \in Histories |-> {}]
    /\ datamodel = Data0
    /\ internalQueue = <<>>
    /\ externalQueue = <<>>
    /\ delayed = <<>>
    /\ currentEvent = NoEvent
    /\ phase = "start"
    /\ told = NoTold
    /\ arrived = 0
    /\ outcome = None

\* §scxml-D-interpret: running, then enterStates on the document's initial
\* transition.
Start ==
    /\ phase = "start"
    /\ LET entered == EnterStates(<<InitialOf[Root]>>, [Machine EXCEPT !.phase = "loop"])
       IN Becomes([m |-> entered.m,
                   told |-> [NoTold EXCEPT !.fired = <<InitialOf[Root]>>,
                                           !.entered = entered.entered]])
    /\ UNCHANGED <<arrived, outcome>>

\* §scxml-D-mainEventLoop, the macrostep: an eventless microstep first.
Eventless ==
    /\ phase = "loop"
    /\ LET ts == SelectEventlessTransitions(Machine)
       IN /\ ts # <<>>
          /\ Becomes(Microstep(ts, Machine))
    /\ UNCHANGED <<arrived, outcome>>

\* §scxml-D-mainEventLoop, the macrostep: else an internal event.
Internal ==
    /\ phase = "loop"
    /\ SelectEventlessTransitions(Machine) = <<>>
    /\ internalQueue # <<>>
    /\ Becomes(TakeEvent([Machine EXCEPT !.intq = Tail(@), !.event = Head(internalQueue)]))
    /\ UNCHANGED <<arrived, outcome>>

\* §scxml-D-mainEventLoop: an external event once the macrostep is done.
External ==
    /\ phase = "loop"
    /\ externalQueue # <<>>
    /\ RunToCompletion => MacrostepDone(Machine)
    /\ Becomes(TakeEvent([Machine EXCEPT !.extq = Tail(@), !.event = Head(externalQueue)]))
    /\ UNCHANGED <<arrived, outcome>>

\* §scxml-D-exitInterpreter: every state exited in exit order; the top-level
\* final state, then the only one active, is the session's done event.
Stop ==
    /\ phase = "exit"
    /\ LET order == ExitSequence(configuration)
           ran == RunBlocks([i \in 1..Len(order) |-> OnExit[order[i]]], Machine)
           top == {s \in configuration : IsFinal(s) /\ Parent[s] = Root}
       IN /\ Becomes([m |-> [ran EXCEPT !.conf = {}, !.phase = "end"],
                      told |-> [NoTold EXCEPT !.exited = order]])
          /\ outcome' = IF top = {} THEN None ELSE CHOOSE s \in top : TRUE
    /\ UNCHANGED arrived

\* The environment: the i-th delayed send comes due, onto the external queue.
Due(i) ==
    /\ phase = "loop"
    /\ externalQueue' = Append(externalQueue, delayed[i])
    /\ delayed' = [j \in 1..Len(delayed) - 1 |-> IF j < i THEN delayed[j] ELSE delayed[j + 1]]
    /\ told' = NoTold
    /\ UNCHANGED <<configuration, historyValue, datamodel, internalQueue, currentEvent,
                   phase, arrived, outcome>>

\* The environment: the event e comes from outside.
Arrive(e) ==
    /\ phase = "loop"
    /\ arrived < MaxOutside
    /\ externalQueue' = Append(externalQueue, e)
    /\ arrived' = arrived + 1
    /\ told' = NoTold
    /\ UNCHANGED <<configuration, historyValue, datamodel, internalQueue, delayed,
                   currentEvent, phase, outcome>>

\* What the interpreter does, as against its environment.
Step == Start \/ Eventless \/ Internal \/ External \/ Stop

Next ==
    \/ Step
    \/ \E i \in 1..Len(delayed) : Due(i)
    \/ \E e \in Outside : Arrive(e)

Spec == Init /\ [][Next]_vars

\* The interpreter is not starved; the environment is owed nothing.
LiveSpec == Spec /\ WF_vars(Step)

\* A legal configuration: the scxml element and each active compound state
\* have exactly one active child, an active parallel state has every child
\* active, the parent of an active state is active, and no pseudo state is.
OneChildOfScxml == configuration # {} => Cardinality(ChildrenOf[Root] \cap configuration) = 1
OneChildOfCompound ==
    \A s \in configuration : IsCompound(s) => Cardinality(ChildrenOf[s] \cap configuration) = 1
EveryChildOfParallel == \A s \in configuration : IsParallel(s) => ChildrenOf[s] \subseteq configuration
AncestorsClosed == \A s \in configuration : Parent[s] = Root \/ Parent[s] \in configuration
NoPseudoState == \A s \in configuration : ~IsHistory(s)

LegalConfiguration ==
    /\ OneChildOfScxml
    /\ OneChildOfCompound
    /\ EveryChildOfParallel
    /\ AncestorsClosed
    /\ NoPseudoState

\* exitOrder and entryOrder as a trace shows them: no state is exited after
\* one of its ancestors, and none is entered before one of its ancestors.
ChildrenExitFirst ==
    \A i, j \in 1..Len(told.exited) : i < j => ~IsDescendant(told.exited[j], told.exited[i])
ParentsEnterFirst ==
    \A i, j \in 1..Len(told.entered) : i < j => ~IsDescendant(told.entered[i], told.entered[j])
====
