# SCE test profile — operational semantics

- Status: specification of ADR 0091, order of work step 1 (§1 to §10), the document forms of step 2 (§11), and the
  structure of the native runtime of step 3 (§12).
- Decision: `docs/adr/0091-a-test-is-a-document-of-the-test-profile-and-its-ttcn-3-export-runs-and-agrees-with-it.md`.
  Where this document and the ADR disagree, the ADR is amended or this document is wrong; neither is read as
  overriding the other.
- Sources, pinned by ADR 0091 answer 2:
  - ETSI ES 201 873-1 V4.17.1 (2025), the core language, cited as **-1 §x**;
  - ETSI ES 201 873-4 V4.6.1 (2017), the operational semantics, cited as **-4 §x**;
  - W3C SCXML 1.0 (2015), cited in the tree's token form: `§scxml-` followed by the section number.

This document defines what a profile document means. It is the basis of the native test runtime (step 3), of the
exporter and its translation validator (step 4), and of the conformance lane (step 5). The names of elements and
attributes are those of ADR 0091 answer 5. §11 fixes the XML they are written in; the schemas that check it are in
`schemas/sce-forge-ext.xsd`.

## 1. How the semantics is stated

-4 defines TTCN-3 behaviour as an evaluation procedure over a **module state** (-4 §8.3): the states of module
control and of every test component (entity states), the states of all ports, and the test case verdict. Each
statement is a flow graph segment whose nodes change that state (-4 §9). The procedure repeats three phases: update
(time progress and input from the SUT), selection of one entity that is not blocked, and execution of one node of it
(-4 §8.6.1).

The profile is stated over the same module state. A profile document does not have flow graphs; it has a statechart.
So the profile replaces the control stack of an entity by a statechart configuration, and states, for each rule
below, which -4 segment the step it takes is. A rule is written as:

- **Form**: what the document writes;
- **Step**: how the module state changes;
- **Adopts**: the -1 and -4 clauses whose meaning the step has;
- **Export**: the TTCN-3 the exporter writes for it (§9).

Everything -4 leaves outside its scope stays outside this document's scope too (-4 §6): encoding and decoding,
grouping, import and visibility. Data matching is defined by -1 (§6 here), because -4 takes it as given
(`MATCH-ITEM`, -4 §8.4.5).

## 2. The module state of a profile run

### 2.1 Entities

| Entity (-4 §8.3.2) | Profile source |
|---|---|
| module control (`CONTROL`) | the `test-suite` document |
| the MTC | the `test-component` named by the running `test-case` |
| a PTC | a `test-component` invoked from another component's state (§4.9) |

Each entity state holds:

| Field | -4 name | Profile meaning |
|---|---|---|
| status | `STATUS` | `ACTIVE`, `SNAPSHOT` or `BLOCKED` (-4 §8.3.2.0) |
| control | `CONTROL-STACK` | the statechart configuration: since a `test-component` has no `<parallel>` (§8.2), one active atomic state and its ancestors |
| data | `DATA-STATE` | the `sce-static` variables the component declares (ADR 0091 answer 9) |
| timers | `TIMER-STATE` | one binding per `<sce:test-timer>`: `STATUS` (`IDLE`, `RUNNING`, `TIMEOUT`), `DEF-DURATION`, `ACT-DURATION`, `TIME-LEFT`, `SNAP-STATUS` (-4 §8.3.2.4) |
| ports | `PORT-REF` | one port state per port the component type declares (§2.2) |
| local verdict | `E-VERDICT` | initially `none` (§5) |
| defaults | `DEFAULT-LIST` | not stored: it is the list of the active atomic state's proper ancestors, innermost first (§4.5) |
| pending invocations | — | the `<invoke>` elements of states entered in the current macrostep, not yet started (§4.9) |

### 2.2 Ports

Each port state (-4 §8.3.3) holds a FIFO queue of received messages, its snapshot `SNAP-VALUE` (the head of the
queue when the snapshot was taken, or none), and its connections. A message that a component sends on a connected
port is appended to the queue of the port it is connected to (-4 §8.3.3.1, §9.44). A message from the SUT is
appended to the queue of the component port mapped to the system port it arrived on (-4 §8.6.1.2 b). Queues of
different ports are never merged (ADR 0091 §5).

A port of the profile is connected to at most one other port, or mapped to one system port. -1 allows one-to-many
connections; the profile does not (§8.1), because the sender of a message is then always known from the port it
arrived on, and no receive needs a `from` clause.

### 2.3 Global state

| Field | -4 name | Profile meaning |
|---|---|---|
| all entities | `ALL-ENTITY-STATES` | the MTC and every PTC created and not yet removed |
| test case verdict | `TC-VERDICT` | §5.3 |
| stopped components | `DONE` | PTCs whose statechart reached a top-level final state, or that were killed |
| killed components | `KILLED` | PTCs removed by leaving their invoking state (§4.9) |
| time | — | the logical clock `NOW`, an integer count of microseconds since the test case started (§3.3) |

-4 leaves time progress outside the semantics ("no assumptions about time progress", -4 §8.6.1.2). The profile fixes
it (§3.3), because the conformance lane needs a run to be a function of its input trace (ADR 0091 answer 3).

## 3. The scheduler

### 3.1 Why the profile fixes one

-4's selection phase picks any entity that is not blocked (-4 §8.6.1.3). With PTCs, the order in which components take
snapshots relative to one another's sends is therefore free, and a test case with PTCs has many runs on one input
trace. Two of them may differ in which alt branch a component takes, so the observable trace (ADR 0091 §1: messages
per port, timeouts, final verdict) is not a function of the inputs.

C3 compares the native run with the TTCN-3 run and treats any difference as `error`. That comparison has a meaning
only if both sides take the same run. The profile therefore fixes one scheduler. Every run under it is a run that
-4's selection phase may produce, so the profile stays a subset of TTCN-3 behaviour. It is not every run TTCN-3
admits. The equivalence theorem (§9.3) is stated for this scheduler, and the conformance lane runs Titan under it
(§10).

### 3.2 Rounds

The scheduler runs in **rounds**. A component is **waiting** when it has completed a macrostep and its active atomic
state has no enabled eventless transition, so that it can proceed only by a snapshot.

1. **Quiescence.** A round starts when every component is waiting or has terminated, no port message is in transit
   between components, and no create, connect, map or done is being carried out.
2. **Update.** If no branch of any waiting component can fire on a snapshot taken now (§3.3), time advances. Then
   every input of the trace and every timer expiry due at the new `NOW` is delivered: inputs appended to their port
   queues in trace order, timers set to `TIMEOUT`.
3. **Snapshot.** Every waiting component takes its snapshot at once (-4 §9.3.1).
4. **Selection.** The waiting components evaluate their branches, each over its own snapshot (§4.3), one at a time in
   the order they were created: the MTC first, then each PTC in the order of its creation.
5. **Execution.** A component whose selection found a branch takes it and runs its macrostep to completion (§4.4)
   before the next component selects. Messages it sends are appended to the receiving port queues. They are not in
   any snapshot of this round, because every snapshot of the round was taken in step 3. A component that was removed
   or that terminated earlier in the round does not select (P9, P10).
6. The round ends when quiescence holds again.

The order of steps 4 and 5 is fixed because it can change the result. A component reads only its own snapshot and
its own variables, so in most rounds it does not; but a component can remove another that has not yet selected in the
round: by leaving the state that invoked it (P9), or, for the MTC, by terminating (P10). Whether the removed component
takes its step then decides whether its verdict changes and whether a send to its ports is an error (P6). Running the
removed component's step first, or deferring every removal to the end of the round, does not remove the dependence:
two components can each remove a component the other sends to, and then every -4 order of the two ends in a send to
a removed port. With the order fixed, the run is determined by the input trace and the document.

**Lemma 3.2 (the schedule is a TTCN-3 schedule).** Every round is a sequence of steps that -4's evaluation procedure
may take: the update phase delivers input and advances timers (-4 §8.6.1.2); the selection phase may select the
entities in the order the round uses, and may select one entity repeatedly until its macrostep is done
(-4 §8.6.1.3); taking a snapshot and evaluating branches are separate nodes of `<alt-stmt>` (-4 §9.3.0), so other
entities may take their snapshots and run between one entity's snapshot and its evaluation.

### 3.3 Time

- `NOW` is an integer count of microseconds. Every duration in a profile document is a whole number of microseconds
  (§4.7), so two instants are equal exactly when their integers are.
- Time advances only in step 2 of a round, and only when no branch can fire without it. It advances to the least of
  the next timer expiry of any component, the next input instant of the trace, and the guard instant of the test
  case (§4.11).
- -4 lets timers be updated in any update phase (-4 §8.6.1.2 a). The profile updates them only at the start of a
  round; any other update is a run the profile does not take.
- Outside the conformance lane, a runtime may take `NOW` from a real clock. The round structure is unchanged. What is
  lost is only the guarantee that two runs on the same inputs take the same branches when inputs and timeouts are
  close together.

## 4. Behaviour rules

### 4.1 P1 — The alt state

- **Form**: an atomic state of a `test-component` whose outgoing transitions include receive, timeout or done
  branches (§4.2).
- **Step**: when the component is waiting in this state, it takes part in the round's snapshot (§3.2 step 3), then
  evaluates its branches (§4.3). If no branch is selected, its status stays `SNAPSHOT`, nothing is consumed, and it
  waits for the next round.
- **Adopts**: `<alt-stmt>` and `<take-snapshot>`, -4 §9.3.0 and §9.3.1; the snapshot mechanism, -1 §20.1.
- **Export**: a label for the state, followed by an `alt` statement (§9.2).

### 4.2 P2 — Branches

A transition of an alt state is one branch. Its kind is given by one attribute:

| Branch | Form | Matches on the snapshot when | Adopts |
|---|---|---|---|
| receive | `<transition sce:receive="p" sce:test-template="t">` | `p.SNAP-VALUE` is present and matches `t` (§6) | -4 §9.37; -1 §22.2.2 |
| receive any | `<transition sce:receive="p">` | `p.SNAP-VALUE` is present | -4 §9.37 with no matching specification |
| timeout | `<transition sce:timeout="T">` | `T.SNAP-STATUS == TIMEOUT` | -4 §9.55; -1 §23.6 |
| done | `<transition event="done.invoke.<id>">` | the PTC the state's `<invoke id="<id>">` created is in the snapshot's `DONE` | -4 §9.16; -1 §21.3.7 |

A branch may have a `cond`. It is the branch's Boolean guard, evaluated before the branch's matching, over the
component's variables (-4 §9.3.2). A `cond` may not read the received message (ADR 0091 §3).

### 4.3 P3 — Branch evaluation order

- **Step**: the branches are tried on the snapshot in this order, and the first whose guard holds and which matches is
  taken:
  1. the transitions of the active atomic state, in document order;
  2. then the transitions of its parent, in document order;
  3. and so on to the outermost state.
- **Adopts**: the alternatives of an alt are tried in their textual order (-4 §9.3.0); after them, the defaults in
  `DEFAULT-LIST` from its first element on (-4 §9.3.5), and an activated default is put first in that list (-4 §9.2).
  §4.5 shows that the ancestors, innermost first, are exactly the `DEFAULT-LIST` of a component that activates a
  default on entering a state and deactivates it on leaving it.
- This order is also `selectTransitions` of §scxml-D for a configuration with one atomic state: the atomic state's
  transitions in document order, then each ancestor's (§scxml-3.13). The profile changes which transitions are
  enabled (§7), not the order in which they are tried.

### 4.4 P4 — Taking a branch

- **Step**: taking a branch is one macrostep of §scxml-D, unchanged:
  1. a receive branch removes the matched message from its queue (-4 §9.37.1 `remove-from-port`), and a timeout
     branch sets the timer to `IDLE` (-4 §9.55). A done branch removes nothing: a component stays in `DONE`
     (-4 §9.16), so a done branch must exit the invoking state, which removes the PTC (P9, §8.2);
  2. the exit set is exited, the transition's executable content runs, and the entry set is entered (§scxml-3.13);
  3. eventless transitions are taken until none is enabled (§scxml-D `mainEventLoop`);
  4. the pending invocations of states still active are started (§4.9).
  
  The component is then waiting again, or it has reached a top-level final state and terminates (§4.10).
- In the executable content of a receive branch, `_event.data` is the received message. Reading it there is the
  value redirect of a receive operation (-4 §9.37.3); reading it in a `cond` is refused. A receive any branch stores
  no message (-1 §22.2.2, restriction d), so its content reads no `_event`.
- **Adopts**: the statement block of a selected alternative (-4 §9.3.2); a receive with assignment (-4 §9.37.3).

### 4.5 P5 — Defaults are ancestor transitions

- **Form**: a transition of a compound state is a default for every state nested in it.
- **Step**: as P3. The ancestor's branches are tried after the atomic state's, innermost ancestor first.
- **Adopts**: `activate` puts a default first in `DEFAULT-LIST` (-4 §9.2), `deactivate` removes it (-4 §9.13), and the
  defaults are evoked from the first on (-4 §9.3.5). Read entering a compound state as `activate` of its transitions,
  and leaving it as `deactivate`. States are entered outermost first and exited innermost first (§scxml-3.13), so at
  every moment `DEFAULT-LIST` lists the active ancestors innermost first.
- **What this leaves out**: §8.1, "defaults activated anywhere".

### 4.6 P6 — Send

- **Form**: `<send type="<test-port processor URI>" target="<port>" sce:test-template="t"/>`. The template must be
  specific: it holds no matching symbol (§6.2).
- **Step**: the value of `t` is appended to the queue of the port `<port>` is connected to, or handed to the system
  adapter if `<port>` is mapped (ADR 0091 answer 10). In an engine, the send leaves through ADR 0084's processor door
  (ADR 0091 answer 6).
- **Adopts**: `<send-op>`, -4 §9.44; -1 §22.2.1.
- A send has no delay. A delay is a timer (§4.7).

### 4.7 P7 — Timers

- **Form**: `<sce:test-timer id="T" duration="500ms"/>` declares a timer, with an optional default duration.
  `<sce:start-timer timer="T" duration="…"/>` starts it, with the default duration if none is given.
  `<sce:stop-timer timer="T"/>` stops it.
- **Step**:
  - start: `STATUS := RUNNING`, `ACT-DURATION` and `TIME-LEFT` set to the duration. Starting a running timer restarts
    it (-1 §23.2).
  - stop: `STATUS := IDLE` (-4 §9.52). Stopping a timer that is not running has no effect.
  - expiry: in a round's update step, when `NOW` reaches the start instant plus the duration, `STATUS := TIMEOUT`
    (-4 §8.6.1.2 a).
  - timeout branch: P2.
- **Durations** are written in SCXML's delay syntax (§scxml-6.2) and must be whole microseconds. They are exported as
  `float` seconds whose decimal value is exact (§9.2).
- **Adopts**: -4 §8.3.2.4, §9.48, §9.52, §9.54, §9.55; -1 §23.
- A test timer is one-shot. The periodic `timer` kind (`SCE_FORGE.md` §4.10) is not a test timer.

### 4.8 P8 — Log

- **Form**: SCXML `<log>`.
- **Step**: no change to the module state. The text is recorded through the test log (ADR 0091 answer 8).
- **Adopts**: -4 §9.31.

### 4.9 P9 — Parallel test components

- **Form**: `<invoke type="<test-component URI>" src="<test-component>" id="<id>"/>` in a state of a component.
- **Step**:
  - **create and start.** When the macrostep that entered the invoking state ends with the state still active, the
    runtime creates the PTC (-4 §9.12), applies every `connect` and `map` of the test configuration that names it
    (-4 §9.10, §9.32), and starts its statechart (-4 §9.46). This is when §scxml-6.4 starts an invocation.
  - **done.** When the PTC terminates (§4.10), it is added to `DONE` (-4 §9.16). A `done.invoke.<id>` branch of the
    invoking component matches on a snapshot that contains it.
  - **kill.** Leaving the invoking state removes the PTC, whether it is still running or already done
    (-4 §9.29a.2). Removing a component adds its local verdict to `TC-VERDICT` (-4 §9.29a.2, §9.29a.1), so the
    verdict of a removed PTC still counts.
- `create` and `start` are not separable (§8.1).
- The invoke channel (`#_parent`, `#_<invokeid>`) carries the lifecycle and nothing else. Messages between components
  go through ports (ADR 0091 answer 10).

### 4.10 P10 — Termination

- **Step**: when a component enters a top-level `<final>`, its behaviour ends and it terminates:
  - a PTC is added to `DONE` and its local verdict is kept for §5.3;
  - when the MTC terminates, every PTC still running is removed and the test case verdict is computed (-4 §9.29a.1).
- `<donedata>` carries no verdict (ADR 0091 §3).

### 4.11 P11 — The test suite and the guard

- **Form**: a `test-suite` lists its test cases in order, each with an optional guard duration and the feature
  requirement that selects it (ADR 0091 §2, answer 4).
- **Step**: test cases run one at a time, in the listed order, if their requirement holds for the suite's features.
  Each starts with `NOW = 0`, `TC-VERDICT = none` and fresh entities.
- **Guard**: if the guard instant is reached before the MTC terminates, every component is removed and the test case
  verdict is `error` (-4 §9.17.2). The document cannot cause this verdict any other way (§5.2).
- **Adopts**: -4 §9.17; -1 §26.1, §26.2.

## 5. Test verdicts

### 5.1 P12 — `setverdict`

- **Form**: `<sce:setverdict value="pass|inconclusive|fail|none" reason="…"/>`.
- **Step**: the local verdict becomes the greater of the old one and the given value, in the order
  `none < pass < inconclusive < fail` (-4 §9.45; -1 §24.1, §24.2). The reason does not take part in the semantics
  (-4 §9.45) and is recorded in the test log.

### 5.2 P13 — `error`

`error` is greater than every other verdict, and only the runtime sets it (-1 §24.1). The runtime sets it for:

- a dynamic error (-4 §8.6.2 d, §9.17.3);
- the guard of §4.11;
- a test case that can never end: no branch of any component fires on a round's snapshot, and there is no instant to
  advance `NOW` to (§3.3), so every later round would be the same. -4 gives such a run no verdict, because it never
  ends; the profile ends it, and with `error`, because the document did not decide its outcome;
- a codec or adapter failure;
- a disagreement under ADR 0091 C3, which the lane records after the run.

### 5.3 P14 — The test case verdict

`TC-VERDICT` is the greatest of the local verdicts of the MTC and every PTC of the test case, removed or not
(-4 §9.29a.1). The runtime computes it; this is the UTP arbiter (ADR 0091 answer 5).

## 6. Templates and matching

-4 takes matching as given (`MATCH-ITEM`, -4 §8.4.5). Its meaning is -1 §15 and Annex B.1. A `test-template` is a
template over one `event-schema` type, or over one field type of it.

### 6.1 Matching mechanisms in the profile

| Mechanism | -1 clause | Applies to |
|---|---|---|
| specific value | B.1.1 | every type |
| value list | B.1.2.1 | every type |
| complemented value list | B.1.2.2 | every type |
| any value `?` | B.1.2.3 | every type |
| any value or none `*` | B.1.2.4 | optional fields |
| value range | B.1.2.5 | integer types |
| omit | B.1.2.8 | optional fields |
| any element `?` inside a list | B.1.3.1 | elements of `list<T>[B]` |
| length restriction | B.1.4.1 | `string[N]`, `bytes[N]`, `list<T>[B]` |
| `ifpresent` | B.1.4.2 | optional fields |
| parameterised template, value parameters | -1 §15.1, §5.4.1.1 | every template |

Not in the profile: superset and subset (B.1.2.6, B.1.2.7), which match set types; matching decoded content
(B.1.2.9); patterns (B.1.5); any elements or none and permutation (B.1.3.2, B.1.3.3), which change how many elements
of a list a template matches; template parameters (-1 §5.4.1.2). §8.1 records each.

### 6.2 Specific templates

A template used in a send must be specific: it holds no matching mechanism other than specific values, and every
parameter it takes is bound to a value. This is -1's rule for sending (-1 §22.2.1).

### 6.3 Types

The types of messages are those of `event-schema` and `enum` documents. The exporter writes them into the types
module (ADR 0091 C2) as:

| SCE type | TTCN-3 type | Note |
|---|---|---|
| `intN`, `uintN` | `integer (min .. max)` of the width | |
| `bool` | `boolean` | |
| `enum:X` | `enumerated` with the declared names, in declared order | |
| `string[N]` | `universal charstring` | SCE bounds a string by UTF-8 bytes; -1 length restrictions count characters. The bound is held by the codec and stated in the types module as a comment, not as a length restriction |
| `optional<T>` | an `optional` field of type T | |
| `bytes[N]` | `octetstring` | the bound is stated as for `string[N]`; for octets the two counts agree |
| `record:X` | the `record` type of the event schema `X` | |
| `list<T>[B]` | `record length (0 .. B) of T` | |
| `union:X` | `union` with one alternative per case of `X`, named by its variant; a case that holds no record is an empty `record {}` | the tag is the alternative chosen, as ADR 0092 §1 makes it |
| `float32`, `float64` | none | not in the profile (§8.1) |
| an `event-schema` | `record` with one field per declared field, in declared order | |

## 7. The change to §scxml-D

The profile changes one step of `mainEventLoop` (§scxml-D) for a `test-component` and leaves the others as they are.

| §scxml-D step | SCXML | Profile |
|---|---|---|
| taking an external event | dequeue the head of the one external queue | wait for the next round, take the snapshot (§3.2) |
| selecting transitions for it | `selectTransitions(externalEvent)` | the same procedure and order, with a transition enabled when its branch matches the snapshot (§4.2, §4.3) |
| an event that enables no transition | removed from the queue and discarded | nothing is removed; the snapshot is discarded and the component waits for the next round |
| microstep, exit and entry order, executable content, eventless transitions, internal completion of the macrostep | §scxml-D | unchanged |
| starting invocations | at the end of the macrostep | unchanged (§4.9) |

In an engine, what reaches the component from outside still enters through its one inbox (ADR 0084). At the
macrostep boundary, the runtime distributes the inbox: a port message to its port queue by its `EventOrigin`
(ADR 0090, ADR 0091 answer 6), a timer expiry to its timer binding, a PTC's termination to `DONE`. Run-to-completion is
unchanged: a snapshot is taken only between macrosteps.

## 8. What the subset leaves out

ADR 0091 §2.1 asks, for each omission, whether it is out by design or deferred. A deferred item returns only through
an amendment of the ADR that gives its semantics and its export.

### 8.1 Omissions

| Construct | -1 / -4 | Decision | Reason |
|---|---|---|---|
| procedure-based ports (`call`, `getcall`, `reply`, `getreply`, `raise`, `catch`) | -1 §22.3; -4 §9.6, §9.25, §9.26, §9.35, §9.39, §9.7 | deferred | the round scheduler extends to them, but they need a blocking call inside a macrostep, which the statechart has no form for yet |
| defaults activated anywhere | -1 §20.5; -4 §9.2, §9.13 | out by design | a default that outlives the state that activated it has no statechart form; P5 covers the nested defaults |
| `interleave` | -1 §20.4; -4 §7.5 | out by design | -4 itself replaces it by nested alts (-4 §7.5); a document writes that replacement as states |
| concurrency inside a component | — | out by design | TTCN-3 has none (-4 §8.3.2: one control stack per entity) |
| `create` apart from `start`, or several `start`s | -1 §21.3.1, §21.3.2 | out by design | one invocation runs one statechart from creation to end |
| one-to-many connections | -1 §21.1 | out by design | §2.2: the sender is known from the port |
| `from` and `sender` clauses | -4 §9.37.1 | out by design | follows from one-to-many being out |
| `any port`, `any timer`, `all component` | -4 §9.37.1, §9.55, §9.16 | deferred | each is a loop over a fixed set and can be written as branches |
| `trigger` | -4 §7.6 | out by design | -4 replaces it by a receive in a loop (-4 §7.6); a document writes a receive-any branch that stays in the state |
| `check` | -4 §9.8 | deferred | a branch that matches without removing; needs its own rule for P4 step 1 |
| `else` branch | -4 §9.3.4 | out by design | an alt with `else` never waits for a snapshot; a document writes an eventless transition |
| `repeat` | -1 §20.3 | out by design | a branch that targets its own state, without exiting it, is a `type="internal"` transition |
| `getverdict` | -4 §9.27 | deferred | needs the verdict readable as a `sce-static` value |
| `read`, `running` (timers and components), `alive`, `killed` | -4 §9.36, §9.42, §9.41 | deferred | need the timer and component state readable as values |
| `now` | -1 §23 | deferred | needs `NOW` readable as a value |
| `stop` of the test case or of a component by statement | -4 §9.50, §9.49, §9.53a | out by design | a component stops by entering a top-level `<final>` |
| patterns, superset, subset, any elements or none, permutation | -1 B.1.2.6, B.1.2.7, B.1.3.2, B.1.3.3, B.1.5 | deferred | §6.1 |
| template parameters | -1 §5.4.1.2 | deferred | a template that takes a template needs a matcher as an argument, and an argument is a component variable (§11.5) |
| `float` | -1 §6.1.0 | deferred | -1's `float` is a real with the special values `infinity`, `-infinity` and `not_a_number`; IEEE 754's binary32 and binary64 add `-0.0` and many NaNs, and matching them needs a rule of its own |

### 8.2 SCXML constructs a `test-component` refuses

ADR 0091 §3 lists the refusals of the profile. The semantics above also needs these, which ADR 0091 §3 lists from
the amendment that adopted this document:

| Construct | Why |
|---|---|
| `<parallel>`, `<history>`, `<script>`, `<donedata>` as a verdict | ADR 0091 §3 |
| `<raise>`, and `<send>` to `#_internal` | an internal event has no TTCN-3 form; the profile has states and eventless transitions for it |
| `<send>` whose target is not a port of the component | messages leave a component only through its ports (P6) |
| `<send>` with `delay` or `delayexpr`, and `<cancel>` | a delay is a timer (P6, P7) |
| `<invoke>` whose type is not the test-component type | a component invokes only PTCs (P9) |
| a transition with an `event` other than `done.invoke.<id>` of the state's own invocation, and a transition with more than one of `event`, `sce:receive`, `sce:timeout` | every branch is one of P2's kinds |
| a done branch whose target keeps the invoking state active | the PTC stays in `DONE` (-4 §9.16), so such a branch would match again on every snapshot (P4) |

## 9. The export and the equivalence theorem

### 9.1 The translation

The exporter is a function `T` from a profile suite (its `test-suite`, `test-case`, `test-configuration`,
`test-component`, `test-port-type`, `test-template`, `event-schema` and `enum` documents) to TTCN-3 modules: a types
module (ADR 0091 C2) and a behaviour module. `T` is compositional: each construct is translated by one rule of §9.2,
and each rule names the construct of the semantics above that it preserves.

### 9.2 Translation rules

| # | Profile | TTCN-3 | Rule of the semantics |
|---|---|---|---|
| E1 | `event-schema`, `enum` | types module, per §6.3 | §6.3 |
| E2 | `test-template` | a template, with each mechanism of §6.1 written as -1 writes it | §6 |
| E3 | `test-port-type` | a message port type with its `in`, `out`, `inout` lists, stated explicitly | §2.2 |
| E4 | `test-component` (its declarations) | a component type with its ports, timers and variables | §2.1 |
| E5 | `test-component` (its statechart) | a function that `runs on` that component type (E6 to E11) | P1–P10 |
| E6 | an atomic state | a label, then: the checks of its eventless transitions in document order, each a `goto` if taken; then the start of its pending invocations; then an `alt` | P1, P4, P9 |
| E7 | the branches of an atomic state and of its ancestors | the alternatives of that `alt`, in the order of P3: the state's own, then each ancestor's. The ancestors' branches are written as alternatives of each alt rather than as activated altsteps: an altstep cannot `goto` out of itself (-1 §19.8, restriction a), and P3 shows the two orders are the same | P3, P5 |
| E8 | a branch | `[guard] p.receive(t) -> value v { … }`, `[guard] T.timeout { … }`, or `[guard] c.done { … }`, whose block is the statically computed exit actions, transition content and entry actions of that branch from that atomic state, then `goto` its target's atomic state. With no `<parallel>` and no `<history>`, the exit and entry sets of a transition from a given atomic state are fixed, so they are computed by the exporter | P2, P4 |
| E9 | `<send>` to a port | `p.send(t)` with the specific template | P6 |
| E10 | `<sce:start-timer>`, `<sce:stop-timer>` | `T.start(d)`, `T.stop`, with `d` as an exact decimal of seconds | P7 |
| E11 | `<invoke>` of a PTC; leaving its state | `c := CT.create`, the configuration's `connect` and `map` for it, `c.start(f())`; on leaving, `c.kill` | P9 |
| E12 | a top-level `<final>` | `return` from the function, which ends the component | P10 |
| E13 | `<sce:setverdict>`, `<log>` | `setverdict(v, reason)`, `log(…)` | P12, P8 |
| E14 | `test-case` | a test case that runs on the MTC's component type, with the configuration's system component type, and calls the MTC's function | §2.1 |
| E15 | `test-suite` | the `control` part: `execute` of each selected test case in order, with its guard as the second argument; feature requirements as conditions on module parameters (ADR 0091 §2) | P11 |

### 9.3 The theorem

**Theorem (equivalence of the export).** For every profile suite `S`, every test case of it, and every input trace
with logical timestamps: the run of `S` under §3 and the run of `T(S)` under §3 have the same observable trace. The
observable trace is the sequence, per port, of messages sent and received with their values; the sequence of
timeouts per timer; and the test case verdict.

The proof is by simulation, one round at a time. Each rule of §9.2 maps one step of §4 to the TTCN-3 segment that
rule's **Adopts** line names, so a round of `S` and a round of `T(S)` change corresponding states in corresponding
ways. The obligations, one per rule, are:

1. P1, E6, E7: the alternatives of the exported `alt`, in textual order, are the branches of P3 in their order.
2. P2, E8: each branch kind matches on the same snapshot field as its TTCN-3 operation.
3. P4, E8: the exported block is the exit set, content and entry set §scxml-3.13 computes for that transition from
   that atomic state.
4. P5, E7: §4.5.
5. P6–P13, E9–E13: the TTCN-3 operation is the one the rule adopts.
6. §6, E1, E2: the types module and the templates denote the same sets of values.
7. §3: both runs use the round scheduler, which for TTCN-3 is a scheduler -4's selection phase may follow
   (Lemma 3.2).

The theorem is stated here and checked, not assumed (ADR 0091 §1). It is held by translation validation of every
export together with C3 (ADR 0091 answer 12).

### 9.4 The correspondence witness

Beside each exported module the exporter writes a witness: one entry per construct of the suite, giving the
construct (document and node), the rule of §9.2 that translated it, and the TTCN-3 construct it became (module and
the path of the definition or statement). The validator of ADR 0091 answer 12 is separate from the exporter. It checks
that:

- every construct of the suite has exactly one entry, and every exported definition and statement is named by one;
- each entry's rule applies to the construct's kind and produced the construct's TTCN-3 form;
- for E7 and E8, the order of alternatives and the computed exit and entry sets are those P3 and §scxml-3.13 give.

Each check is local to one entry or one state, so it is decidable, and the validator does not compare the runs of
two programs.

## 10. What the conformance lane requires of the TTCN-3 tool

The lane runs `T(S)` on Eclipse Titan (ADR 0091 answer 1). Titan by itself does not follow §3: its components are
processes that take snapshots whenever they wait, and its time is the wall clock. The change ADR 0091 answer 3
contributes to Titan therefore has two parts, both carried by Titan's Main Controller:

- **rounds**: a component that waits in an `alt` does not take its snapshot until the Main Controller grants a round,
  and the Main Controller grants one only at quiescence (§3.2 step 1), which it decides from every component's report
  of waiting, with its counts of port messages sent and received, in the current epoch. A grant has two phases:
  every waiting component takes its snapshot, and then the components evaluate their branches one at a time in the
  order of §3.2 step 4, each running its macrostep to its next wait before the next is granted;
- **the logical clock**: when no branch of any component fired in the previous round's grant, the next grant first
  advances `NOW` (§3.3).

The two share the one condition and the one protocol: a grant either starts a round at the present instant or
advances time and then starts one. When it can do neither, because no branch fired and no instant is pending, the Main
Controller ends the test case with `error` (P13).

Titan's clock must compare instants as integers of microseconds (§3.3). Titan holds time as a `double` of seconds, and
the sum of a start instant and a duration in `double` arithmetic can differ in its last bit from the instant a trace
gives for the same moment, which would order a timeout and an input differently on the two sides.

## 11. Document forms

This section fixes the XML of the six kinds (ADR 0091 §2, answer 5). The names are those of answer 5: every element
and attribute is in SCE's one extension namespace (`http://sce.dev/ext`), and none reuses a name the grammar already
has. A name follows -1's term for the concept where -1 has one. The rules of §4 to §6 give each form its meaning, and
this section adds only what the XML must say. Every refusal named here is a `DiagnosticCode` whose name begins with
`test-profile/`, unless an existing code already states it.

### 11.1 Two URIs

| URI | Names | Rule |
|---|---|---|
| `http://sce.dev/ext/test-port` | the Event I/O Processor whose addresses are ports (ADR 0091 answer 6) | `type` of every `<send>` in a `test-component` (P6) |
| `http://sce.dev/ext/test-component` | the invocation of a PTC | `type` of every `<invoke>` in a `test-component` (P9) |

### 11.2 Data model

A `test-component` declares `datamodel="sce-static"`: it is the only kind of the profile that holds expressions
(answer 9). The other five kinds hold declarations and literals. A document of one of them that names a data model
names `sce-static`. Any other value is refused, in all six kinds.

### 11.3 `test-port-type`

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="job_port">
  <sce:import kind="event-schema" src="job_request.scxml" as="JobRequest"/>
  <sce:import kind="event-schema" src="job_completed.scxml" as="JobCompleted"/>
  <sce:message type="JobRequest" direction="out"/>
  <sce:message type="JobCompleted" direction="in"/>
</scxml>
```

- `<sce:message>` names an imported `event-schema` and its direction: `in`, `out` or `inout`, seen from the component
  that owns a port of this type (-1 §6.2.9).
- The `in` list of a port type is the set of messages with direction `in` or `inout`, and the `out` list is the set
  with `out` or `inout`. §11.6 checks ports against each other by these lists.
- Refused: a port type with no message; one type listed twice; two types with the same event name, because a message
  delivered to a port is told apart from the others by its event name.
- Export: `type port job_port message { out JobRequest; in JobCompleted }`.

### 11.4 `test-template`

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="completed_within" sce:type="record:JobCompleted">
  <sce:import kind="event-schema" src="job_completed.scxml" as="JobCompleted"/>
  <datamodel>
    <data id="limit" sce:type="uint32" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:value-range field="elapsed_ms">
      <sce:lower value="0"/>
      <sce:upper param="limit"/>
    </sce:value-range>
    <sce:any-value-or-none field="note"/>
  </sce:record-match>
</scxml>
```

- `sce:type` on the root is the template's type, written in the type grammar of `event-schema` fields. A template
  used in a send or a receive has the type `record:<alias>` of a message.
- The root holds exactly one matcher, of the template's type.
- **Formal parameters** are `<data sce:direction="in">` declarations with no `expr`. They are value parameters
  (-1 §5.4.1.1). A template parameter (-1 §5.4.1.2) is not in the profile (§8.1).

**Matchers.** Each row is one element. A matcher is written where a value of some type is matched: the root, a field
of a `record-match`, an element of a `list-match`, the case of a `union-match`, or an item of a `value-list` or
`complement`.

| Element | -1 | Content | Type of the position |
|---|---|---|---|
| `<sce:specific value="…"/>` or `<sce:specific param="p"/>` | B.1.1 | a literal (below) or a formal parameter of the position's type | integer, `bool`, `enum:`, `string[N]`, `bytes[N]` |
| `<sce:record-match>` | B.1.1 | one matcher per field of the record, each with `field="<name>"`, every field exactly once | `record:` |
| `<sce:list-match>` | B.1.1, B.1.3.1 | one matcher per element, in order; it matches a list of exactly that many elements | `list<T>[B]` |
| `<sce:union-match variant="v">` | B.1.1 | the matcher of case `v`'s record, or nothing when case `v` holds none | `union:` |
| `<sce:value-list>` | B.1.2.1 | two or more matchers; the value matches if it matches one of them | every type |
| `<sce:complement>` | B.1.2.2 | one or more matchers; the value matches if it matches none of them | every type |
| `<sce:any-value/>` | B.1.2.3 | — | every type; in a `list-match` it is one element of any value |
| `<sce:any-value-or-none/>` | B.1.2.4 | — | an `optional<T>` field |
| `<sce:value-range>` | B.1.2.5 | `<sce:lower>` and `<sce:upper>`, each optional, each with `value` or `param`; a missing bound is unbounded; bounds are inclusive | integer |
| `<sce:omit/>` | B.1.2.8 | — | an `optional<T>` field |
| `<sce:template-ref template="A" args="p q"/>` | -1 §15.1 | the imported template `A`, its formal parameters bound in order to this template's own parameters `p q` | the type of `A` |

**Attributes every matcher may carry:**

- `field="<name>"`: on a matcher that is a child of `record-match`, required there and refused anywhere else.
- `length-min` and `length-max` (B.1.4.1): a length restriction, on a position of type `string[N]` (counted in
  characters, as -1 counts the length of a `universal charstring`), `bytes[N]` (octets) or `list<T>[B]` (elements).
  Either may be absent; both are whole numbers and `length-min ≤ length-max`.
- `ifpresent="true"` (B.1.4.2): on a matcher in an `optional<T>` field; the field matches if it is absent or if it
  matches.

**Literals of `<sce:specific value>`:** an integer in decimal, within the range of its type; `true` or `false`; the
name of a variant of the enum; the attribute's text for `string[N]`, within the bound; an even number of hex digits
for `bytes[N]`, within the bound.

**A specific template** (§6.2) is one whose matchers are only `specific`, `record-match`, `list-match`, `union-match`,
`omit`, and `template-ref` to a specific template, with no `length-min`, `length-max` or `ifpresent`. A send names a
specific template; a receive names any template.

Refused: a matcher whose position's type does not admit it (the table above); a `record-match` that misses a field or
names one twice or names one the record does not have; a `union-match` whose variant the union does not declare; a
`value-list` with fewer than two items; a literal outside its type; a `param` or `args` entry that is not a formal
parameter or whose type differs from the position's; a `length-min` greater than `length-max`; `any-value-or-none`
inside a `list-match` (B.1.3.2, §8.1); a template whose references form a cycle; a template whose type contains
`float32` or `float64` (§8.1); a template whose type, or a formal parameter's, holds at any depth a record whose
schema declares no field (`validation/attribute-rule-violated`): such a schema names an event that carries no data,
and no backend generates a record type for it (`docs/SCE_ACCEPTED_SUBSET.md`, "Fieldless schema").

### 11.5 `test-component`

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="job_tester" datamodel="sce-static" initial="requesting">
  <sce:import kind="test-port-type" src="job_port.scxml" as="JobPort"/>
  <sce:import kind="test-template" src="job_request.scxml" as="Request"/>
  <sce:import kind="test-template" src="completed_within.scxml" as="CompletedWithin"/>
  <sce:test-port id="sut" type="JobPort"/>
  <sce:test-timer id="reply" duration="2s"/>
  <datamodel>
    <data id="limit" sce:type="uint32" sce:module-parameter="true"/>
  </datamodel>
  <state id="requesting">
    <onentry>
      <send type="http://sce.dev/ext/test-port" target="sut" sce:test-template="Request"/>
      <sce:start-timer timer="reply"/>
    </onentry>
    <transition sce:receive="sut" sce:test-template="CompletedWithin" sce:template-args="limit" target="passed">
      <sce:stop-timer timer="reply"/>
      <sce:setverdict value="pass"/>
    </transition>
    <transition sce:timeout="reply" target="failed">
      <sce:setverdict value="fail" reason="'no reply in time'"/>
    </transition>
  </state>
  <final id="passed"/>
  <final id="failed"/>
</scxml>
```

- **Declarations**, children of `<scxml>`:
  - `<sce:test-port id="p" type="<alias>">`, a port of an imported `test-port-type` (-1 §6.2.10);
  - `<sce:test-timer id="T" duration="…">`, a timer with an optional default duration (P7).
- **Variables** are `sce-static` `<data>`. A `<data sce:module-parameter="true">` has no `expr`. Its value is the
  module parameter of the same `id` that the suite declares (§11.8), it is exported as a reference to that
  `modulepar`, and assigning to it is refused.
- **Executable content** added by the profile:
  - `<sce:setverdict value="pass|inconclusive|fail|none" reason="<expr>"/>` (P12), where `reason` is optional and is
    an `sce-static` expression of type `string`;
  - `<sce:start-timer timer="T" duration="…"/>` and `<sce:stop-timer timer="T"/>` (P7). A start with no `duration`
    needs a default on the timer.

  Each is executable content like `<assign>`: it runs in its place among the other elements of its block, in
  document order (§scxml-4.1). So a `reason` read in a receive branch reads the received message as the branch's
  `<assign>` does. In a statechart that is not a `test-component`, each is an element its executable content does not
  take (`validation/unexpected-child-element`).
- **Template arguments.** `sce:template-args` on a `<send>` or a receive branch lists component variables, separated
  by spaces, bound in order to the template's formal parameters. Like SCXML's `namelist`, it names locations, not
  expressions. It is an attribute so that the children of `<transition>` stay executable content (ADR 0057 §3).
- **A send** has exactly the attributes `type`, `target` (a port of the component), `sce:test-template`, and
  `sce:template-args` when the template takes parameters. The template is specific and its type is in the port's `out`
  list. The event name of the sent message is that of the template's record type.
- **A receive branch** names a port whose `in` list contains the template's type. A receive any branch names a port
  and no template.
- **The received message.** The message a receive branch takes is an event of its template's record, under that
  record's event name, as a send sends one. Its content reads it as `_event.data`, typed by the record, which the
  component imports as an `event-schema` like any statechart that reads a payload: a value redirect stores into a
  variable whose type the module sees (-1 §8.2.3, §22.2.2). A branch that reads the message of a record the component
  does not import is refused (`test-profile/message-read`), and a read of a field the record does not have is refused
  as any payload read of one is.
- **A PTC** is `<invoke type="http://sce.dev/ext/test-component" src="<test-component document>" id="<id>"/>`, with
  a literal `id` and no `idlocation`, `autoforward`, `<param>`, `<content>` or `<finalize>`. The id is the PTC's name in
  the configuration (§11.6).
- Refused, besides §8.2: a duration that is not whole microseconds; `sce:template-args` whose length differs from the
  template's parameter count, or whose variable types differ from the parameters'; a port or timer declared twice;
  a reference to a port, timer or template the component does not declare or import.

### 11.6 `test-configuration`

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-configuration" name="tester_and_helper">
  <sce:import kind="test-component" src="job_tester.scxml" as="Tester"/>
  <sce:import kind="test-component" src="helper.scxml" as="Helper"/>
  <sce:import kind="test-port-type" src="job_port.scxml" as="JobPort"/>
  <sce:mtc type="Tester"/>
  <sce:ptc id="helper" type="Helper"/>
  <sce:system-port id="sut" type="JobPort"/>
  <sce:map component="mtc" port="sut" system-port="sut"/>
  <sce:connect>
    <sce:endpoint component="mtc" port="control"/>
    <sce:endpoint component="helper" port="control"/>
  </sce:connect>
</scxml>
```

- `<sce:mtc>` names the MTC's component type, once. `<sce:ptc>` names each PTC and its type. The name `mtc` is the
  MTC's and no PTC takes it.
- `<sce:system-port>` declares a port of the test system interface. Together they are the `system` component type of
  the exported test case (-1 §6.2.10).
- `<sce:connect>` joins two component ports (-1 §21.1.1 b): each one's `out` list is contained in the other's `in`
  list, and at least one `out` list is not empty.
- `<sce:map>` joins a component port to a system port (-1 §21.1.1 c): the component port's `out` list is contained in
  the system port's `out` list, the system port's `in` list in the component port's `in` list, and at least one of
  those two is not empty.
- **Ports are one-to-one** (§2.2): every component port and every system port takes part in exactly one `connect` or
  `map`.
- **The PTCs are created by invocations.** Every `<sce:ptc>` is the `id` of exactly one `<invoke>` in the components
  of the configuration, whose `src` is a document of the PTC's type; every such `<invoke>` names a declared PTC. The
  invoking relation is a tree rooted at the MTC, so no component creates itself.

### 11.7 `test-case`

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-case" name="completes_in_time" sce:configuration="TesterAndHelper">
  <sce:import kind="test-configuration" src="tester_and_helper.scxml" as="TesterAndHelper"/>
</scxml>
```

- A test case names its configuration. Its body is the MTC's statechart, and it `runs on` the MTC type with the
  configuration's system ports as its `system` (-1 §26.1).
- Its name is the test case's name in the suite, in the test log and in the export.

### 11.8 `test-suite`

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="job_suite">
  <sce:import kind="feature-model" src="job_features.scxml" as="Features"/>
  <sce:import kind="test-case" src="completes_in_time.scxml" as="CompletesInTime"/>
  <sce:module-parameter id="limit" type="uint32" value="500"/>
  <sce:execute test-case="CompletesInTime" guard="10s" requires="job-timeout"/>
</scxml>
```

- `<sce:execute>` lists one test case, in run order (P11). `guard` is its time limit (answer 4). `requires` is its
  selection, a feature condition over the imported `feature-model` in ADR 0057's grammar; an `<sce:execute>` is an
  `sce:` element, so the attribute is `requires` (ADR 0057 §3). The selection is the suite's, as TTCN-3 writes it in
  the control part (`if (…) { execute(…) }`, -1 §26.1, §26.2); a test case document carries none.
- A test case is listed once.
- `<sce:module-parameter id type value>` declares a module parameter (-1 §8.2.1) and its default. Its type is a
  scalar, `string[N]` or `bytes[N]` type, and `value` is a literal of §11.4. Every `<data sce:module-parameter="true">`
  of a component reached from the suite has a declaration here with its `id` and type.
- Feature conditions appear only on `<sce:execute>`. The other kinds of the profile take none: a `test-component`
  that imports a `feature-model` is refused (`test-profile/component-construct`), and a `requires` with no model to
  read it against is refused by ADR 0057's own rules (§11.10).
- At most one `feature-model` import. A `requires` names only its features
  (`validation/feature-undeclared`, the code ADR 0057 gives a condition that names a feature its model does not
  declare; ADR 0091 §3).

### 11.9 Generation

The six kinds are parsed and checked on every backend from the step that lands them. Until the native runtime of the
order of work step 3 lands in a backend for a kind, generating a document of that kind on it is refused by name
(`generate/unsupported-feature`). The backends land one kind at a time, in the order of §12.6, and
`codegen_matrix::TEST_TEMPLATE_BACKENDS`, `codegen_matrix::TEST_COMPONENT_BACKENDS` and
`codegen_matrix::TEST_SUITE_BACKENDS` list those that generate a template, a component, and the tables of a test
with the port types they read.

C++ generates `test-template` (§12.6, slice 3). The unit is a header `<name>.h` in the namespace
`SCE::Generated::<Name>`, which declares:

- `Value`, the template's type as the unit of an `event-schema` spells a field of that type (a record is the
  schema's payload struct);
- `bool match(const Value &value, <the formal parameters, in order>)`, which holds each mechanism of §6.1 as
  ES 201 873-1 Annex B.1 defines it. A matcher of a value's type on an `optional<T>` field requires the field
  present; `omit`, `any-value-or-none`, `ifpresent`, a value list or complement and a parameter or a reference of
  the optional type itself take an absent one. A length restriction on a `string[N]` counts characters, through
  the profile's runtime (`sce/include/test/Length.h`), and on `bytes[N]` and `list<T>[B]` octets and elements;
- for a specific template (§6.2) alone, `Value value(<the formal parameters>)`, the value it sends, which matches
  it.

A template whose type reaches a union at any depth is refused on C++ as a field of a union is (ADR 0092, section
7).

Rust generates `test-template` (§12.6, slice 6). The unit is a file `<name>.rs`, named by the document's file as
every forge unit is, which declares:

- `Value`, the template's type as the unit of an `event-schema` spells a field of that type (a record is the
  schema's payload struct, a union the enum its unit declares);
- `fn matches(value: &Value, <the formal parameters, in order>) -> bool`, which holds each mechanism of §6.1 by the
  rules C++ holds it by. A number, a `bool`, a value of an enum and an optional one are passed by value, every other
  parameter is lent. A length restriction on a `string[N]` counts characters (`chars().count()`), and on `bytes[N]`
  and `list<T>[B]` octets and elements. A `union-match` holds where the union is in the case its `variant` names,
  and the case's record matches the case's matcher;
- for a specific template (§6.2) alone, `fn value(<the formal parameters>) -> Value`, the value it sends, which
  matches it.

Every enum, record and union the unit names is named through the sibling module of the document that declares it
(`super::<document>`), so an enum a record of another document holds is named by its one declaration. The walk over
the matchers, which decides each matcher's tests, the optional rules, the bounds a range leaves out and the binding
of a reference's parameters, is one for every backend (`forge::generator::test_template_walk`); a backend spells it.

Kotlin generates `test-template` (§12.6, slice 6). The unit is a file `<Name>.kt` in the package
`com.sce.generated.<document>`, named by the document's file as every forge unit is, which declares:

- `Value`, a type alias of the template's type as the unit of an `event-schema` spells a field of that type (a
  record is the schema's payload data class, a union the sealed interface its unit declares);
- `fun matches(value: Value, <the formal parameters, in order>): Boolean`, which holds each mechanism of §6.1 by the
  rules C++ holds it by. A value that may be absent is read through a `let` that binds it, and a byte string is
  compared by its octets (`contentEquals`). A length restriction on a `string[N]` counts characters, through the
  profile's runtime (`com.sce.runtime.test.textLength`), and on `bytes[N]` and `list<T>[B]` octets and elements. A
  `union-match` holds where the union is an instance of the class of the case its `variant` names (`is`), and the
  case's record matches the case's matcher;
- for a specific template (§6.2) alone, `fun value(<the formal parameters>): Value`, the value it sends, which
  matches it; a byte string a parameter gives is copied into it.

Every enum, record, union and referenced template the unit names is named by the package of the document that
declares it, so the unit imports no package. Rust and Kotlin join the tests the walk decides the same way
(`test_template_walk::JoinSpelling`): a negation carried down to the tests, and a test the walk decides for every
value left out of its join.

Go generates `test-template` (§12.6, slice 6). The unit is a file `<name>.go`, the package `<name>` of the
directory `<name>` below the module `--go-module-prefix` names, named by the document's file as every forge unit
is, which declares:

- `Value`, a type alias of the template's type as the unit of an `event-schema` spells a field of that type (a
  record is the schema's payload struct, a value that may be absent the forge runtime's `optional.Of[T]`);
- `func Matches(value Value, <the formal parameters, in order>) bool`, which holds each mechanism of §6.1 by the
  rules C++ holds it by. A value that may be absent is tested by `IsPresent` and read by `Proven` behind that
  test, a value that may be absent and a parameter of its type are compared by `Equal`, and a byte string by its
  octets (`bytes.Equal`). A length restriction on a `string[N]` counts characters, through the profile's runtime
  (`testprofile.TextLength`), and on `bytes[N]` and `list<T>[B]` octets and elements (`len`);
- for a specific template (§6.2) alone, `func Specific(<the formal parameters>) Value`, the value it sends, which
  matches it; a byte string a parameter gives is copied into it, and a value that may be absent is built with its
  element type named (`optional.Some[T]`, `optional.None[T]`). It is not `Value`, since Go keeps the type and the
  package's functions in one namespace.

Every enum, record and referenced template the unit names is named through the package of the document that
declares it, under the alias `sce_<document>` every Go unit imports a sibling package by, so an enum a record of
another document holds is named by its one declaration; the unit imports exactly the packages its text names, as
Go requires. Go joins the tests as Rust and Kotlin do. Go lowers no union, so a template whose type reaches a union
at any depth is refused on Go as on C++ (ADR 0092, section 7).

Python generates `test-template` (§12.6, slice 6). The unit is a module `<name>.py`, a sibling of the modules of
the documents it names, named by the document's file as every forge unit is, which declares:

- `Value`, an alias of the template's type as the unit of an `event-schema` spells a field of that type (a record is
  the schema's payload dataclass, a value that may be absent `Optional[T]`);
- `def matches(value: Value, <the formal parameters, in order>) -> bool`, which holds each mechanism of §6.1 by the
  rules C++ holds it by. A value that may be absent is the value itself or `None`, so it is tested by `is None`
  and read as it is behind that test; a text, a byte string, a value that may be absent and a parameter of its type
  are compared by `==`, which compares Python's values by their contents. A length restriction on a `string[N]`
  counts characters, through the profile's runtime (`sce_runtime.testprofile.text_length`, a text's `len`, since a
  Python text is a sequence of code points), and on `bytes[N]` and `list<T>[B]` octets and elements (`len`);
- for a specific template (§6.2) alone, `def value(<the formal parameters>) -> Value`, the value it sends, which
  matches it: a record built by its fields' keywords, a field it omits `None`. A byte string a parameter gives is
  held as given, since a Python `bytes` is immutable.

Every enum, record and referenced template the unit names is named through the module of the document that declares
it, under the alias `sce_<document>` every Python unit imports a sibling module by (`from . import <document> as
sce_<document>`), so an enum a record of another document holds is named by its one declaration; the unit imports
exactly the modules its text names, as the Go unit does, so it reaches no module a run need not have generated.
Python joins the tests by the join Rust, Kotlin and Go share, in Python's words (`and`, `or`, `not`, `True`,
`False`, `test_template_walk::Connectives`), whose precedence is C's. Python lowers no union, so a template whose
type reaches a union at any depth is refused on Python as on C++ and Go (ADR 0092, section 7).

C11 generates `test-template` (§12.6, slice 6). The unit is a header `<name>.h`, named by the document's file as every
forge unit is, and, since C has no namespace, each declaration carries the document's name:

- `<name>_value_t`, a typedef of the template's type as the unit of an `event-schema` holds a field of that type:
  one entry where the type is one (a number, `bool`, a value of an enum, a text in the buffer of its bound
  `sce_static_string_<N>_t`, an optional one in `sce_optional_<elem>_t`), and a composition laid out statically
  where it is not (a record is the schema's `<Name>Payload_t`), so a template's value is the field's own type;
- `bool <name>_matches(const <name>_value_t *value, <the formal parameters, in order>)`, which holds each mechanism
  of §6.1 by the rules C++ holds it by. A text is compared by its bytes (`strcmp`), a byte string inside a list by
  its length and its octets (`memcmp`), and a value that may be absent is tested by `present` and read by `value`
  behind that test; an optional value a parameter gives, the one composed value a specific value stands for, is
  compared by its two parts, since C compares no two structs. A length restriction on a `string[N]` counts characters, through the profile's runtime
  (`sce_test_text_length`, `sce/test/text_length.h`), and on `bytes[N]` and `list<T>[B]` octets and elements
  (`len`);
- for a specific template (§6.2) alone, `<name>_value_t <name>_value(<the formal parameters>)`, the value it sends,
  which matches it: the initializer of its variable, each record by its fields' designators and a text through the
  fill function of its buffer, so no type is named inside it.

Every record, enum and referenced template the unit names is reached through the header of the document that
declares it, which the unit includes, and a referenced template's functions are named by its document. C11 joins
the tests as Rust, Kotlin, Go and Python do. C11 lowers no union, so a template whose type reaches a union at any
depth is refused on C11 as on C++ (ADR 0092, section 7). It refuses by name too a template that compares, counts or
builds a byte string of one entry, which the C11 unit of an event schema holds as a pointer with no length, and a
reference, inside a list, to a template of an optional type, whose value the list holds in its composition's own
optional type and not in the one the referenced template takes.

C++ generates `test-component` (§12.6, slice 4). The unit is the statechart machine the C++ generator writes for any
`sce-static` document, `<file>_sm.h` and its `<file>_sm.inl`, in the namespace `SCE::Generated::<name>`, lowered
through the doors of §12.3 before it is rendered (`test_component::snapshot::lower`, written once for every
backend):

- every branch is a transition on the snapshot event `sce.test.snapshot`, in its document place, whose guard is
  its `cond` and then its match; taking it runs, before its content, the statement of P4 step 1. A receive
  branch's match asks the runtime for the snapshot's head under its template's record's event and calls the
  template's `match` on it, so the machine includes the unit of every template it uses;
- `<sce:setverdict>`, `<sce:start-timer>` and `<sce:stop-timer>` are calls on the component's
  `SCE::Test::ComponentContext`, a duration in whole microseconds, a timer's own where the statement writes none;
- a `<log>` is recorded in the test log through the same context, with `NOW` and the component, its label and its
  value as the engine's log shows them (P8), and never in the engine's log;
- a send names its template's record's event and hands the test-port processor the specific template's value,
  as a value (`HostSendRequest::value`) and never an encoding (ADR 0091, C2); a PTC's `<invoke>` is run by the
  test-component invoker.

Beside the machine the header declares `<name>Component`, which implements `SCE::Test::ComponentMachine`: `start`
binds the context, registers the test-port processor and the test-component invoker on it and starts the machine;
`deliverSnapshot` delivers the snapshot event; `terminated` is the machine's having entered a top-level `<final>`.
A component that uses a template whose type reaches a union is refused on C++ as the template is.

Rust generates `test-component` (§12.6, slice 6). The unit is the statechart machine the Rust generator writes for any
`sce-static` document, `<file>_sm.rs`, lowered through the same doors by the same lowering and spelled for the Rust
runtime (`sce_rust_runtime::test`):

- a branch's guard, the statement taking it, the statements of the profile and a `<log>` are calls on the component's
  `ComponentContext`, which the machine's policy holds and reads through one accessor. A receive branch's match is
  `receives` over its template's `matches`, the template's unit brought into scope as `super::<file>`, and a variable
  bound to a formal parameter is handed over as the unit takes it, by value or lent. A variable of an enum crosses into
  the variant of the same declared name of the enum unit's type, as a value handed to an algorithm does, on C++ and on
  Rust alike; an optional value of an enum has no crossing yet and is refused by name;
- a receive branch whose content reads the message copies it, field by field, into the payload struct the machine's
  channel holds its record's event in, bound as the content's `_event.data`. The machine holds a text, a byte string
  and an enum in types of its own, which the record's unit does not, so a message of a record with such a field that
  the content reads is refused by name;
- a send hands the test-port processor the specific template's value as `HostSendRequest::value`.

Beside the machine the file declares `<Name>Component`, which implements `sce_rust_runtime::test::ComponentMachine`:
`start` builds the machine around a policy the context is bound to, registers the test-port processor and the
test-component invoker on clones of the context and starts the machine; `deliver_snapshot` delivers the snapshot event;
`terminated` is the machine's having entered a top-level `<final>`. Rust refuses by name a component that declares
`<sce:action>`, whose machine is a policy over a host the component is given nothing of; and every component under
`--no-std`, since the profile's runtime is `std` only. The machine's session is named by the component, `mtc` or the
id of the `<invoke>` that creates it, which the parse holds to an XML Name.

Kotlin generates `test-component` (§12.6, slice 6). The unit is the statechart machine the Kotlin generator writes for
any `sce-static` document, `<file>Sm.kt`, lowered through the same doors by the same lowering and spelled for the
Kotlin runtime (`com.sce.runtime.test`):

- a branch's guard, the statement taking it, the statements of the profile and a `<log>` are calls on the component's
  `ComponentContext`, which the machine is constructed with (`sceTestContext`). A receive branch's match is
  `receives` over its template's `matches`; every declaration of a template's unit is named by the unit's package
  (`com.sce.generated.<document>`), as one unit names another's, so the machine imports no template's package. A
  variable bound to a formal parameter is handed over as the machine holds it, since the unit takes every parameter
  by value; a variable of an enum crosses into the variant of the same declared name of the enum unit's class, and
  an optional value of an enum is refused by name, as on C++ and Rust;
- a receive branch whose content reads the message copies it, field by field, into the payload data class the
  machine's channel holds its record's event in, which the content reads as `_event.data`. The machine and the
  record's unit hold a number, a `bool`, a text and a byte string in the same Kotlin types, which are copied as they
  are, and an enum in classes of their own, so an enum field takes the variant of the machine's class of the same
  declared name, an optional one where it is present. Kotlin so generates the message of a record with a text, a
  byte string or an enum field that Rust refuses;
- a send hands the test-port processor the specific template's value as `HostSendRequest.value`.

Beside the machine the file declares `<Name>Component`, which implements `com.sce.runtime.test.ComponentMachine`:
`start` constructs the machine with the context, under the session the component's name gives, registers the
test-port processor and the test-component invoker on it and starts it (`initialize`); `deliverSnapshot` sends the
snapshot event and runs the macrostep it starts (`tick`); `terminated` is the machine's having entered a top-level
`<final>`. Kotlin refuses by name a component that declares `<sce:action>`, whose machine is constructed with a host
the component is given nothing of. A component's machine always has a saved shape, which a session id it is given
needs: what would leave it none, a delayed send or an invoke a restore cannot start, the profile refuses.

Go generates `test-component` (§12.6, slice 6). The unit is the statechart machine the Go generator writes for any
`sce-static` document, `<file>_sm.go`, the package `<file>`, lowered through the same doors by the same lowering
and spelled for the Go runtime (the package `testprofile` of `sce-go-runtime`, which the machine imports as
`scetestprofile`, under the generator's own prefix, so no local an author names hides it):

- a branch's guard, the statement taking it, the statements of the profile and a `<log>` are calls on the component's
  `*ComponentContext`, which the machine's policy holds (`sceTestContext`). A receive branch's match is `Receives`
  over a function literal that calls its template's `Matches`, the literal naming the template's `Value`, from
  which Go infers the runtime's type parameter. Every declaration of a template's unit is named through its package
  under the alias `sce_<document>` every Go unit imports a sibling package by, and the machine imports the package of
  each template its text names, under the module path the run is given (`--go-module-prefix`), so a component that
  uses a template is refused without one. A variable bound to a formal parameter is handed over as the machine holds
  it; a variable of an enum crosses into the variant of the same declared name of the enum unit's type, and an
  optional value of an enum is refused by name, as on the other backends;
- a receive branch whose content reads the message copies it, field by field, into the payload field of the policy
  the machine's channel holds its record's event in, which the content reads as `_event.data`. The machine and the
  record's unit hold a number, a `bool`, a text, a byte string and an optional value of one in the same Go types,
  which are copied as they are, and an enum in types of their own, so an enum field takes the variant of the
  machine's type that its declared name names (`DeclaredName`, read as a saved value of that type is,
  `<Type>FromSaved`), an optional one where it is present. Go so generates the message of a record with a text, a
  byte string or an enum field that Rust refuses, as Kotlin does;
- a `<log>` records its label, a colon and its value as the engine's `<log>` formats it (`%v`);
- a send hands the test-port processor the specific template's value (`Specific`) as `HostSendRequest.Value`.

Beside the policy the file declares `<Name>Component`, which implements `testprofile.ComponentMachine`: `Start`
builds the policy, gives it the session the component's name gives (`SetSessionID`), sets the context on it, builds
the engine on it, registers the test-port processor and the test-component invoker on the engine and starts it
(`Initialize`); `DeliverSnapshot` processes the snapshot event (`ProcessEvent`); `Terminated` is the machine's having
entered a top-level `<final>`. Every `sce-static` policy declares `SetSessionID` and draws an id when none is given,
whether or not a saved state can hold its machine, so the component's session id needs no saved shape. Go refuses by
name a component that declares `<sce:action>`, whose policy is constructed with a host the component is given
nothing of, and one that uses a template whose type reaches a union, which Go does not lower (ADR 0092, section 7).

Python generates `test-component` (§12.6, slice 6). The unit is the statechart machine the Python generator writes for
any `sce-static` document, `<file>_sm.py`, lowered through the same doors by the same lowering and spelled for the
Python runtime (`sce_runtime.testprofile`, which the machine imports as `_sce_testprofile`, under a leading
underscore as its other imports of the runtime are, so it is never the alias of a template's module):

- a branch's guard, the statement taking it, the statements of the profile and a `<log>` are calls on the component's
  `ComponentContext`, which the machine's policy holds (`_sce_test_context`). A branch's guard is its `cond` and then
  its match, joined by Python's `and`. A receive branch's match is `receives` over a `lambda` that calls its
  template's `matches`, given the template's `Value`, which `receives` holds the snapshot's head to by `isinstance`.
  Every declaration of a template's unit is named through its module under the alias `sce_<document>` every Python
  unit imports a sibling module by, and the machine imports the module of each template it uses as a sibling
  (`from . import <document> as sce_<document>`), as it imports an algorithm. A variable bound to a formal parameter
  is handed over as the machine holds it; a variable of an enum crosses into the variant of the same declared name of
  the enum unit's type, and an optional value of an enum is refused by name, as on the other backends;
- a receive branch whose content reads the message copies it, field by field, into the payload attribute of the
  policy the machine's channel holds its record's event in, which the content reads as `_event.data`. The machine and
  the record's unit hold a number, a `bool`, a text, a byte string and an optional value of one in the same Python
  types, which are copied as they are, and an enum in `IntEnum`s of their own, so an enum field takes the variant of
  the machine's type that its declared name names (`declared_name`, read as a saved value of that type is,
  `from_saved`), an optional one where it is present. Python so generates the message of a record with a text, a
  byte string or an enum field that Rust refuses, as Kotlin and Go do. The statement raises, and reads nothing, where
  the message it removes is not of the template's type, which the guard has already held it to;
- a `<log>` records its label, a colon and its value as the engine's `<log>` shows it (`_sce_log_name`, then `str`);
- a send hands the test-port processor the specific template's value (`value`) as `HostSendRequest.value`.

Beside the policy the module declares `<Name>Component`, a `ComponentMachine` of the runtime: `start` builds the
machine through the module's `create_engine`, under the session the component's name gives (`session_id`), sets the
context on its policy, registers the test-port processor and the test-component invoker on the engine and starts it
(`initialize`); `deliver_snapshot` sends the snapshot event (`send_event`), which runs the macrostep it starts;
`terminated` is the engine's having reached a top-level `<final>` (`reached_final`). The class says
`__test__ = False`, so pytest collects none, whatever the document's name makes of its own. The engine takes a session
id it is given, or draws one, whether or not a saved state can hold its machine, so the component's session id needs
no saved shape. Python refuses by name a component that declares `<sce:action>`, whose policy is constructed with a
host the component is given nothing of, and one that uses a template whose type reaches a union, which Python does
not lower (ADR 0092, section 7).

A module parameter (§11.5) is a variable of the machine declared with its type and no value of the document's: the
static lowering, written once for every backend, lowers no initial value for it. Its value is the run's. Before
`start` starts the machine it hands the machine the run's module parameters through the context
(`ComponentContext::readModuleParameters`), and the machine reads each of its module parameters from them as a saved
state's variable of that type is read back (`sce/include/saved/SavedValues.h`). A value the run does not give, or
one not of the type or past its bound, ends the run with `error` (P13), and the machine is not started, since no
value is made up in its place. The runtime holds each value as the one JSON value model a saved state is made of
(`SCE::Test::ModuleParameters`), so it knows the type of none, and the component's reader and the suite's writer
(below) are each chosen by the type its own document declares, which §11.8 holds to be one type.

Rust reads a module parameter the same way: the field is declared at its type's default, or for an enum, whose
machine type has none, at the variant its document declares first, neither of which anything reads. An enum's
saved form (`SavedValue`), which the read goes through, is emitted for a machine with an enum module parameter
whether or not the machine has a saved shape. Then
`start`, after it binds the context and before it builds the machine, calls
`ComponentContext::read_module_parameters` with the policy's `accept_module_parameters`, which reads each module
parameter from the run's values as a restored variable of its type is read (`SavedValue::from_saved`, a byte string
held to its bound by `saved_state::bounded_bytes`). A refused read ends the run with `error` and leaves the machine
unbuilt.

Kotlin reads a module parameter the same way: the field is declared at its type's zero, or for an enum at the
variant its document declares first (`forge::generator::kotlin_module_parameter_starts`), neither of which anything
reads; an enum's class declares its saved form (`fromSaved`) whether or not the machine saves. Then `start`, after
it constructs the machine with the context and before it registers a door on it or starts it, calls
`ComponentContext.readModuleParameters` with the machine's `acceptModuleParameters`, which reads each module
parameter from the run's values by the reader a restore reads a variable of its type by (`SavedValues`, a text and a
byte string held to their bounds). A refused read ends the run with `error` and leaves the machine unstarted.

Go reads a module parameter the same way: the policy's field is declared at its type's zero, or for an enum at the
variant its document declares first, a constant that carries the name of the machine's enum type, neither of which
anything reads; both starts are one walk with Kotlin's (`forge::generator::go_module_parameter_starts`). An enum's
type declares its saved form (`<Type>FromSaved`) with the type, whether or not the machine saves. Then `Start`, after
it sets the context on the policy and before it builds the engine, calls `ComponentContext.ReadModuleParameters` with
the policy's `sceAcceptModuleParameters`, which reads every module parameter from the run's values by the reader a
restore reads a variable of its type by (`sce.SavedRead<Type>`, a text and a byte string held to their bounds,
`<Type>FromSaved` for an enum) before it writes any. A refused read ends the run with `error` and leaves the machine
unbuilt.

Python reads a module parameter the same way: the policy's attribute is declared at its type's zero, or for an enum at
the variant its document declares first, a member of the machine's `IntEnum`, neither of which anything reads; the
starts are the one walk Kotlin's and Go's are (`forge::generator::python_module_parameter_starts`). An enum's class
declares its saved form (`from_saved`) with the type, whether or not the machine saves. Then `start`, after it builds
the engine through `create_engine` and sets the context on the policy, and before it registers a door on the engine or
starts it, calls `ComponentContext.read_module_parameters` with the policy's `_sce_accept_module_parameters`, which
reads every module parameter from the run's values by the reader a restore reads a variable of its type by
(`sce_runtime.saved_values.saved_read_<type>`, a text and a byte string held to their bounds, `from_saved` for an
enum; the one spelling `_macros/python_saved_value.jinja2` gives the saved functions too) before it writes any. A
refused read ends the run with `error` and leaves the machine unstarted, and the component one that has not started.

A backend that generates a component and not the suite's tables refuses a component that declares a module parameter
by name (`test_component::snapshot::lower`), since its value is the suite's; C++, Rust, Kotlin, Go and Python
generate both.

C++ generates `test-configuration`, `test-case` and `test-suite`, and writes no file for `test-port-type` (§12.6,
slice 5; `codegen_matrix::TEST_SUITE_BACKENDS`): the units of the components and configurations that read a port
type hold what they need of it by name. Each table is a header `<name>.h` in the namespace `SCE::Generated::<Name>`,
and names only the units of its own imports:

- a configuration declares `configuration()`, the `SCE::Test::Configuration` the runtime reads, its component types
  named by their machines; `machine(type)`, which makes the `<machine>Component` of each type and none for another
  type, which the run ends with `error` where it is created; and `moduleParameters()`, the ids its components read;
- a test case declares `NAME` and `testCase()`, the `SCE::Test::TestCase` of its name, its configuration's tables
  and the module parameters they read;
- a suite declares `suite()`, the `SCE::Test::Suite` of its module parameters at their defaults, each written by
  the saved writer of its declared type (an enum's as the name its document declares for the variant), and of its
  `<sce:execute>` entries in run order, each with its test case, its guard in microseconds and its selection: the
  condition lowered to a function of the world's features, `all` a conjunction and `any` a disjunction. A guard
  past the microseconds the runtime's clock counts to is refused by name.

The runner `SCE::Test::runSuite` (`sce/include/test/Suite.h`) runs the test cases a world selects (§12.4).

Rust generates `test-configuration`, `test-case` and `test-suite`, and writes no file for `test-port-type` (§12.6,
slice 6). Each table is a file `<name>.rs`, named by the document's file as every forge unit is, over
`sce_rust_runtime::test`, and names each unit it reads through its sibling module: a test case's configuration and a
suite's test cases as `super::<file>`, a component's machine as `super::<name>_sm`, so no unit has an import line:

- a configuration declares `configuration()`, the `Configuration` the runtime reads, every field written and an empty
  table as empty; `machine(component_type)`, which makes the `<Name>Component` of each type and none for another; and
  `module_parameters()`, the ids its components read;
- a test case declares `NAME` and `test_case()`, the `TestCase` of its name, its configuration's tables, its
  configuration's `machine` as the factory of machines, and the module parameters they read;
- a suite declares `suite()`, the `Suite` of its module parameters at their defaults, each the saved value of its
  declared type (an integer by its own type's `SavedValue::to_saved`, which writes a 64-bit one as its text; a
  `bool`, a text and an enum's variant name as the JSON value they are; a byte string by
  `saved_state::bytes_to_saved`), and of its `<sce:execute>` entries in run order, each with its test case, its guard
  in microseconds and its selection as a closure over the world's features. The outermost join of a selection is the
  closure's body and stands bare, every join inside it in parentheses.

The runner `sce_rust_runtime::test::run_suite` runs the test cases a world selects (§12.4).

Kotlin generates `test-configuration`, `test-case` and `test-suite`, and writes no file for `test-port-type` (§12.6,
slice 6). Each table is a file `<Name>.kt` in the package `com.sce.generated.<document>`, named by the document's file
as every forge unit is, over `com.sce.runtime.test`, and names each unit it reads by its package, so no unit has an
import line: a test case's configuration and a suite's test cases by the package of their documents, and a
component's class by the package its statechart is generated into when no `--kotlin-package-prefix` is given
(`com.sce.generated.<name>`, `generator::kotlin_machine_package`), since a configuration cannot know the prefix
another run chose. A component a configuration runs is generated with no prefix:

- a configuration declares `configuration()`, the `Configuration` the runtime reads, every field written and an empty
  table as empty; `machine(componentType)`, which makes the `<Name>Component` of each type and null for another; and
  `moduleParameters()`, the ids its components read;
- a test case declares `NAME` and `testCase()`, the `TestCase` of its name, its configuration's tables, a factory of
  machines that calls its configuration's `machine`, and the module parameters they read;
- a suite declares `suite()`, the `Suite` of its module parameters at their defaults, each the saved value of its
  declared type (a number, a `bool`, a text and a byte string by `SavedValues.of` of a literal of the type, which
  writes a 64-bit integer as its text and a byte string as its Latin-1 text; an enum's variant by the name its
  document declares), and of its `<sce:execute>` entries in run order, each with its test case, its guard in
  microseconds and its selection as a lambda over the world's features, whose body stands bare as Rust's closure's
  does.

The runner `com.sce.runtime.test.runSuite` runs the test cases a world selects (§12.4).

Go generates `test-configuration`, `test-case` and `test-suite`, and writes no file for `test-port-type` (§12.6,
slice 6). Each table is a file `<file>.go` in the package `<file>`, named by the document's file as every forge unit
is, over the profile's runtime, which it imports by its package's own name (`testprofile`). A Go unit names another
package only by an import path, which has no bare form, so each table imports what it reads as a sibling package
below the module path the run is given (`--go-module-prefix`), under the alias `sce_<document>` every Go unit imports
a sibling by, and is refused by name without one: a test case its configuration's unit, a suite its test cases', and
a configuration each component's machine. A component's machine is in the package its statechart declares,
`<module path>/<snake of the machine's name>`, the directory a forge unit of that name is placed in
(`forge::generator::forge_unit_dir`); so a configuration and the components it runs are generated under one module
path, each component's machine placed in that directory, as every Go importer and the units it imports are; a
package two rows name is imported once:

- a configuration declares `Configuration()`, the `testprofile.Configuration` the runtime reads, every field written
  and an empty table as empty; `Machine(componentType)`, a `testprofile.MachineFactory` that makes the
  `<Name>Component` of each type and nil for another; and `ModuleParameters()`, the ids its components read;
- a test case declares `Name` and `TestCase()`, the `testprofile.TestCase` of its name, its configuration's tables,
  its configuration's `Machine` as the factory of machines, and the module parameters they read;
- a suite declares `Suite()`, the `testprofile.Suite` of its module parameters at their defaults, each the saved
  value of its declared type written by the writer of that type (`sce.SavedWrite<Type>` of an untyped constant, which
  writes a 64-bit integer as its text and a byte string as its Latin-1 text; an enum's variant by the name its
  document declares, written as a text is), which imports the SCE runtime as `sce` where a module parameter is
  declared, and of its `<sce:execute>` entries in run order, each with its test case, its guard
  (`testprofile.GuardOf` of its microseconds, or `testprofile.NoGuard`) and its selection as a function literal over
  the world's features, whose returned expression stands bare as Rust's closure's body does.

The runner `testprofile.RunSuite` runs the test cases a world selects (§12.4).

Python generates `test-configuration`, `test-case` and `test-suite`, and writes no file for `test-port-type` (§12.6,
slice 6). Each table is a module `<file>.py`, named by the document's file as every forge unit is, over the profile's
runtime, which it imports as `_sce_testprofile`, the name a component's machine imports it by. Each table imports
what it reads as a sibling module, under the alias `sce_<document>` every Python unit imports a sibling by (`from .
import <document> as sce_<document>`): a test case its configuration's unit and a suite its test cases'. A
configuration imports each component's machine from the module the component's statechart is generated as,
`<machine>_sm` (the file `<stem>_sm.py` the Python generator writes for any statechart), under `sce_<machine>_sm`; so a
configuration, the components it runs, its test cases and the suite are one package, as the machines and the units
they import are. A module two rows name is imported once. The runtime modules a table reads are imported under names
with a leading underscore, which no sibling's alias and no declaration of a table can be:

- a configuration declares `configuration()`, the `Configuration` the runtime reads, every field written and an empty
  table as empty; `machine(component_type)`, a `MachineFactory` that makes the `<Name>Component` of each type and
  `None` for another; and `module_parameters()`, the ids its components read;
- a test case declares `NAME` and `test_case()`, the `TestCase` of its name, its configuration's tables, its
  configuration's `machine` as the factory of machines, and the module parameters they read. `test_case` begins as a
  test of pytest's does, and a test case's document may be named `test_<...>`, whose module pytest collects where it
  lies among tests, so the function says `__test__ = False`;
- a suite declares `suite()`, the `Suite` of its module parameters at their defaults, each the saved value of its
  declared type written by the writer of that type (`sce_runtime.saved_values.saved_write_<type>`, imported as
  `_sce_saved` where a module parameter is declared, which writes a 64-bit integer as its text and a byte string as
  its Latin-1 text; an enum's variant by the name its document declares, written as a text is), and of its
  `<sce:execute>` entries in run order, each with its test case, its guard in microseconds or `None`, and its
  selection as a `lambda` over the world's features in Python's `and` and `or`, whose body stands bare as Rust's
  closure's does, every join inside it in parentheses.

Which rows each table holds, in which order, and what is refused is decided once for every backend
(`forge::generator::test_tables`); a backend spells them. The runner `sce_runtime.testprofile.run_suite` runs the
test cases a world selects (§12.4). Every kind stays refused on the other backends until their slices land.

### 11.10 The rules under feature derivation

ADR 0057 §5 asks of every rule how it behaves under the derivation `π_c`, which removes each element whose condition
is false in the configuration `c`. ADR 0091 §3 asks that every world of a suite's features be a valid suite.

- **Where `π_c` acts.** Only `<sce:execute>` takes a condition (§11.8). A `test-component` imports no `feature-model`,
  and a template, a port type, a configuration and a test case have no element that takes one. So `π_c` is the
  identity on every document of the profile except a suite, and on a suite it removes `<sce:execute>` entries and
  nothing else. This is the TTCN-3 reading: a control part selects test cases (-1 §26.1), and a test case runs its
  components as written.
- **Rules of the kinds `π_c` leaves as written.** Every rule of a template, a port type, a component, a configuration
  and a test case, together with the rules of the statechart that a component is held to, holds in `π_c(D)` exactly
  when it holds in `D`, because the two documents are the same.
- **Rules of the suite.** Each holds in a suite with fewer `<sce:execute>` entries when it holds in the suite with all
  of them:
  - a test case is listed no more often;
  - the imports are not gated, so a listed case is still imported;
  - a `guard` and an `<sce:module-parameter>` are not removed;
  - the components reached from fewer test cases read no module parameter that the components reached from all of
    them did not read (`test-profile/module-parameter`).
- **The suite's selection.** A malformed condition, a feature the model does not declare and a second `feature-model`
  are the product line's own rules (`Family`), judged once on the suite as written.

So every world of a suite's features is a valid suite exactly when the suite as written is, and no rule of the profile
needs a variant judged on its own. The registry (`DiagnosticCode::derivation_class`) lists each `test-profile/` code as
`ProductOnly`, the class ADR 0057 gives every rule when it lands and calls sound for any rule. The codes do not move
to `PreservedByRemoval`, because that class claims more than the argument above proves. It claims that a rule holds in
every sub-document, and some do not: removing an `<invoke>` breaks "every PTC is created by exactly one `<invoke>`"
(`test-profile/ptc-invocation`). Those rules hold in every world only because `π_c` never removes the element.

## 12. The native runtime

ADR 0091 order of work step 3 builds a runtime that runs a suite as §2 to §5 define, first in one backend and then in
the others. This section fixes its structure. It changes no rule of §2 to §11.

### 12.1 Which backend first

C++. The corpus that ADR 0091's Context measured, 432 documents that pair `cpp:` conditions with C++ message types,
runs in C++, and its migration (order of work step 7) needs a C++ runtime to migrate onto. The other backends follow
one at a time. Each lifts the refusal of §11.9 for itself when its runtime lands.

### 12.2 What is generated and what is library

| Part | Where | Why there |
|---|---|---|
| Round scheduler, `NOW`, port queues and snapshots, timer bindings, verdicts, entity table, guard | a runtime library per backend (C++: `sce/include/test/`, Rust: `sce_rust_runtime::test`, Kotlin: `com.sce.runtime.test`, Go: the package `testprofile` of `sce-go-runtime`, Python: `sce_runtime.testprofile`, C11: the headers `sce/test/` of `backends/c/runtime`) | it is the same for every suite, so it is written once and tested on its own (Zero Duplication) |
| A component's machine | generated from the `test-component`, by the backend's statechart generator | it is a statechart (§7) |
| A template's matcher and a specific template's value | generated from the `test-template`, over the record types the statechart generator already writes for an `event-schema` | matching is on values, not on encoded bytes, so no codec takes part (C2) |
| Which port is connected or mapped to which, and the MTC and PTC of each component | generated tables from the `test-configuration` and `test-case` | they are data |
| The test cases a world of the features runs, with guards and module parameters | a generated table from the `test-suite` | it is data |
| The SUT's side of a mapped port | a system adapter: an interface the runtime calls, shaped after TRI's `triSend` and `triEnqueueMsg` (ADR 0091 answer 1) | the codec and the transport are the adapter's |

### 12.3 A component is an unchanged statechart machine

The runtime reaches a component's machine only through doors the engine already has. The engine's macrostep, its
exit and entry order, its eventless transitions and its executable content are used as they are, which is what §7
requires.

| Profile | Engine door |
|---|---|
| a round's snapshot (§3.2 step 3) | one external event of a reserved name, delivered to each waiting component. No document can write the name, because `test-profile/branch` refuses every `event` on a transition but `done.invoke.<id>`. |
| a branch (P2) | a transition on that event whose guard is the branch's `cond` and its match: receive, the port's `SNAP-VALUE` is present and matches the template; receive any, it is present; timeout, the timer's `SNAP-STATUS` is `TIMEOUT`; done, the PTC is in the snapshot's `DONE`. |
| the order of P3 | the engine's `selectTransitions` (§scxml-D), unchanged. When no branch matches, no transition is enabled and the engine discards the event: nothing is removed, as §7 requires. |
| taking a branch, P4 step 1 | one statement the generator puts before the branch's executable content. A receive branch removes the head of its port's queue and holds it as the transition's `_event.data`, in the payload member of its record's event (`Transition::payload_event`). A timeout branch sets its timer to `IDLE`. Every branch's statement also tells the runtime that the component took a branch in this round. |
| `<sce:setverdict>`, `<sce:start-timer>`, `<sce:stop-timer>` | actions of their block (§11.5) that call the component's runtime context |
| a `<log>` (P8) | the same action of its block, which calls the runtime context instead of the engine's log, so the test log records it with `NOW` and the component |
| a send (P6) | a host Event I/O Processor for the test-port URI, registered by the runtime (ADR 0084). The request names the port and carries the template's value, as the value the generated code holds and never an encoding (C++ and Rust: `HostSendRequest::value`, Kotlin and Python: `HostSendRequest.value`, Go: `HostSendRequest.Value`, C11: `sce_host_send_request_t.value`, a view of the value's bytes beside the name of its C type). The runtime appends the value to the queue of the connected port, or hands it to the system adapter when the port is mapped. |
| a PTC (P9) | a host invoke processor for the test-component URI, registered by the runtime. The engine asks for the start when the macrostep that entered the state ends (§scxml-6.4), which is when P9 creates, connects, maps and starts the PTC. It asks for the cancel when the state is exited, which is P9's kill. The runtime never reports a completion to the engine: `done` is read from the snapshot (P2). So a PTC that has terminated is still running for the engine, and the engine delivers the cancel for it too, which P9's kill requires. |

A module parameter (§11.5) takes no door of the engine. It is a variable of the machine, and the component writes
the run's value into it before the machine starts, as a host writes a restored variable, so the engine starts on it
as on any initial value.

A component has no other source of external events: `<raise>` and `<cancel>` are refused (§8.2), a send has no delay
(P6) and goes to a port, and every `<invoke>` is a PTC. So when the engine returns from delivering the snapshot event,
the component's macrostep has completed and it is waiting (§3.2) or terminated (P10).

### 12.4 The scheduler

All the components of a test case run on one thread, in the order they were created, which is the order §3.2 step 4
fixes. On one thread, quiescence (§3.2 step 1) holds whenever no delivery is in progress, because a send appends to
its queue before it returns.

A round:

1. Deliver every input of the system adapter and every timer expiry due at `NOW` (§3.2 step 2).
2. Take every waiting component's snapshot, then deliver the snapshot event to each, in that order, skipping a
   component removed or terminated earlier in the round. Once the MTC has terminated, or the run has met a dynamic
   error, nothing more is delivered.
3. If some component took a branch, start the next round at the same `NOW`.
4. Otherwise advance `NOW` to the least of the next timer expiry, the next input instant the adapter announces, and
   the guard instant (§3.3), and start the next round.

`NOW` is a signed 64-bit count of microseconds, owned by the runtime. The engine's own clock and scheduler take no
part, because a component starts no delayed send.

When no component took a branch and there is no instant to advance to, the test case can never end, and the runtime
ends it with `error` (P13). The lane's clock does the same (§10), so the two runs agree (C3).

A suite is run by a library function, with no `main` of its own (C++: `SCE::Test::runSuite`, Rust:
`sce_rust_runtime::test::run_suite`, Kotlin: `com.sce.runtime.test.runSuite`, Go: `testprofile.RunSuite`, Python:
`sce_runtime.testprofile.run_suite`, C11: `sce_test_run_suite`, which hands each result to the caller's function as
it ends rather than returning a list, in one run the caller places): given the suite's
tables, a world of its features and a factory of system adapters, it runs the test cases the world selects, one at
a time on the calling thread, in the suite's order (P11), each a fresh run from `NOW = 0` against a fresh adapter,
and returns each one's result. The caller may give a module parameter another value than the suite's default; a
value for a parameter the suite does not declare is the caller's mistake and is refused before any test case runs.
A test case whose components read a module parameter the run has no value for, or for which no adapter is given,
ends with `error` (P13) before any of its components is created.

### 12.5 Verdicts and the result

The runtime keeps `E-VERDICT` for each component and computes `TC-VERDICT` over the MTC and every PTC, removed or not
(P14), with `error` above the four verdicts a document sets (P13). A test case's result is its verdict and its log:
each send, receive, timeout, verdict and log line, with its `NOW` and its component. The log's schema is fixed with
the equivalence law (order of work step 6).

### 12.6 The order of the slices

1. A test component's statements are actions of their blocks (§11.5).
2. The C++ runtime library of §12.2, with unit tests over hand-written components, before any generation.
3. C++ generation of templates: the matcher of each template and the value of each specific template. The
   refusal of §11.9 is lifted for C++ for `test-template`.
4. C++ generation of test components through the doors of §12.3. The refusal of §11.9 is lifted for C++ for
   `test-component`, but for a component that declares a module parameter, whose value slice 5 gives.
5. C++ generation of the configuration, case and suite tables, a runner, and the examples run end to end with the
   verdicts they expect. The refusal is lifted for the remaining kinds, and for a component's module parameter.
6. The other backends, one at a time. Rust generates templates first, a template over a union among them; the
   refusal of §11.9 is lifted for Rust for `test-template`. Rust generates test components next, through the doors of
   §12.3, and the refusal is lifted for Rust for `test-component`, but for a component that declares a module
   parameter, whose value Rust's suite tables give. Rust then generates the configuration, case and suite tables
   over the rows C++ spells (`forge::generator::test_tables`) and runs the examples end to end through
   `sce_rust_runtime::test::run_suite`; the refusal is lifted for Rust for the remaining kinds, and for a
   component's module parameter. Kotlin's runtime library of §12.2 (`com.sce.runtime.test`) is the Rust one ported
   type for type and held to its cases, with `textLength` for a template's length restriction, since a Kotlin text
   counts UTF-16 units; and the Kotlin host send request carries a test-port send's value (`HostSendRequest.value`).
   Kotlin generates templates, a template over a union among them, and the refusal of §11.9 is lifted for Kotlin
   for `test-template`. Kotlin generates test components next, through the doors of §12.3, and runs them through
   `com.sce.runtime.test.TestCaseRun`; the refusal is lifted for Kotlin for `test-component`, but for a component
   that declares `<sce:action>`. Kotlin then generates the configuration, case and suite tables over the same rows
   and runs the examples end to end through `com.sce.runtime.test.runSuite`; the refusal is lifted for Kotlin for
   the remaining kinds, and for a component's module parameter. Go's runtime library of §12.2 is the package
   `testprofile` of `sce-go-runtime`, the Rust one ported type for type and held to its cases, under a name that
   is not Go's `test`, which the toolchain gives its own files and packages; it has `TextLength` for a template's
   length restriction, since a Go text is held as bytes, and the Go host send request carries a test-port send's
   value (`HostSendRequest.Value`). Go generates templates, and the refusal of §11.9 is lifted for Go for
   `test-template`; Go lowers no union, so a template whose type reaches one stays refused there, as on C++. Go
   generates test components next, through the doors of §12.3, and runs them through `testprofile.TestCaseRun`; the
   refusal is lifted for Go for `test-component`, but for a component that declares `<sce:action>` and for one
   whose template reaches a union. Go then generates the configuration, case and suite tables over the same rows,
   each importing what it reads below the run's module path, and runs the examples end to end through
   `testprofile.RunSuite`; the refusal is lifted for Go for the remaining kinds, and for a component's module
   parameter. Python's runtime library of §12.2 is the package `sce_runtime.testprofile`, the Rust one ported type
   for type and held to its cases, under a name that is neither the standard library's `test` nor one pytest
   collects (a file `test_*.py`, a class `Test*`); its classes whose names begin with `Test` say `__test__ = False`
   for the same reason. It has `text_length` for a template's length restriction, a text's `len`, since a Python
   text is a sequence of code points, and the Python host send request carries a test-port send's value
   (`HostSendRequest.value`). Python generates templates, and the refusal of §11.9 is lifted for Python for
   `test-template`; Python lowers no union, so a template whose type reaches one stays refused there, as on C++ and
   Go. Python generates test components next, through the doors of §12.3, and runs them through
   `sce_runtime.testprofile.TestCaseRun`; the refusal is lifted for Python for `test-component`, but for a component
   that declares `<sce:action>` and for one whose template reaches a union. Python then generates the configuration,
   case and suite tables over the same rows, each importing what it reads as a sibling module, and runs the examples
   end to end through `sce_runtime.testprofile.run_suite`; the refusal is lifted for Python for the remaining kinds,
   and for a component's module parameter. C11's runtime library of §12.2 is the headers `sce/test/` of
   `backends/c/runtime`, the Rust one ported function for function and held to its cases, allocating nothing: a run
   is one struct the caller places, each of its tables bounded at compile time and overridable with `-D` (the
   components present at once, `SCE_TEST_MAX_COMPONENTS` 8; a component's ports and timers, `SCE_TEST_MAX_PORTS` and
   `SCE_TEST_MAX_TIMERS` 8 each; the messages waiting in all its queues, one pool of
   `SCE_TEST_MAX_QUEUED_MESSAGES` 16, each value of at most `SCE_TEST_MAX_VALUE_SIZE` 256 bytes; the lines of its log,
   `SCE_TEST_MAX_LOG_RECORDS` 64 of `SCE_TEST_MAX_LOG_DETAIL` 160 bytes; a suite's module parameters,
   `SCE_TEST_MAX_MODULE_PARAMETERS` 8). A run that would pass one of them ends with `error` (P13), the bound named in
   its log; a log line that does not fit is counted in the result's `log_lost`, never cut, and ends the run with
   `error` too. A machine and a system adapter are structs whose first member points to their table of functions,
   and a component's context is a handle the machine keeps: a call through the handle of a removed component ends
   the run with `error`, and so does a kill of a component whose machine is running. It has `sce_test_text_length`
   for a template's length restriction, since a C text is held as UTF-8 bytes, and the C11 host send request
   carries a test-port send's value (`sce_host_send_request_t.value`). C11 generates templates, and the refusal of
   §11.9 is lifted for C11 for `test-template`; C11 lowers no union, so a template whose type reaches one stays
   refused there, as on C++, Go and Python, and so does one over a byte string of one entry, which its event schema
   unit holds as a pointer with no length.
