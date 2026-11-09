# SCE test profile — operational semantics

- Status: specification of ADR 0091, order of work step 1 (§1 to §10), the document forms of step 2 (§11), the
  structure of the native runtime of step 3 (§12), and the structure of the exporter of step 4, with its types
  module, its manifest, its name contract and the behaviour of each test component (§9.5), and its correspondence
  witness (§9.4), and the test log that step 6's equivalence law compares (§12.5).
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
| `intN`, `uintN` | the type `intN` or `uintN` of the types module, `type integer uint8 (0 .. 255)` and so on | its range is the width's; it is named because TTCN-3 writes no range on the element of a `record of` in place (§9.5) |
| `bool` | `boolean` | |
| `enum:X` | `enumerated` with the declared names, in declared order, each with its declared number | an enum of an open set has no form (§9.5) |
| `string[N]` | `universal charstring` | SCE bounds a string by UTF-8 bytes; -1 length restrictions count characters. The bound is held by the codec and stated in the types module as a comment, not as a length restriction |
| `optional<T>` | an `optional` field of type T | |
| `bytes[N]` | `octetstring` | the bound is stated as for `string[N]`; for octets the two counts agree |
| `record:X` | the `record` type of the event schema `X` | |
| `list<T>[M..B]` | `record length (M .. B) of T` | M is 0 where the type writes no lower count; a list of `optional<T>` has no form, since the elements of a `record of` are never omitted |
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

Beside the two modules the exporter writes one witness, `S.witness.json` (`schemas/sce-ttcn3-witness.v1.schema.json`,
`pre-release`, a row of `SCE_WIRE_CONTRACTS.md`): one entry per construct of the suite and rule of §9.2 that
translated it, giving the construct (its document, its node and the row of the node), the rule, and every place of
the two modules that rule wrote for it. A construct two rules translate has an entry for each: a `test-component`
one for E4, its component type, and one for E5, its functions; a transition one for E8, its alternatives and the
`goto` that ends each of its blocks, and, where it is eventless and guarded, one for E6, its check; an atomic state
one for E6, its label, and one for E7, its `alt`. The validator of ADR 0091 answer 12 is separate from the exporter,
and reads the witness, the suite's documents and the two modules. It checks that:

- every construct of the suite has exactly one entry, and every exported definition and statement is named by one;
- each entry's rule applies to the construct's kind and produced the construct's TTCN-3 form;
- for E7 and E8, the order of alternatives and the computed exit and entry sets are those P3 and §scxml-3.13 give.

Each check is local to one entry or one state, so it is decidable, and the validator does not compare the runs of
two programs.

**Constructs.** A document is named by its path relative to the directory of the suite's document, `.` and `..`
folded and `/` its separator, or a document of the standard library by its `sce:std/` name, so the witness is the
same bytes wherever the suite is read from. A node is an element by its kind and id (`state 'requesting'`,
`final 'failed'`, `invoke 'helper'`, `sce:test-port 'sut'`, `sce:execute 'CompletesInTime'`), or a child by its kind
and its position from 1 under one (`state 'requesting' / transition 1`, `state 'requesting' / onentry 1 / send 1`); a
document a rule translates whole is its root, `scxml`. The eight integer types of the types module translate no
construct of a document: §6.3 gives them to every types module, and the witness names them under the suite's root
and E1. The two codec functions of a type are part of that type's E1.

**Places.** A place is a module and a path from its top, each segment of it an object of one member, its kind:

- a definition: `type` and `external-function` in the types module; `port-type`, `template`, `modulepar`,
  `component-type` (a test component's and a configuration's system component type alike), `function`, `testcase`,
  and `control`, which its module's name names, in the behaviour module;
- a member of a component type: `port`, `timer`, `var`;
- in a function: `local`, a local variable; `label`, the label of an atomic state, which opens the statements after
  it up to the next label; and `statement`, a position from 0 among the statements the segment before it opens: the
  function's own, which run before its first label, a label's, an alternative's block, or the `execute`s of the
  control part;
- in a statement: `then` and `else`, a position from 0 in an `if`'s two blocks, and `alternative`, a position from 0
  in textual order among an `alt`'s alternatives, which opens its block.

So in the job suite the receive branch of `requesting` is the place
`[{"function": "f_job_tester_in_tester_and_helper"}, {"label": "requesting"}, {"statement": 1}, {"alternative": 0}]`
of the module `job_suite`, and the `goto` that ends its block is that path followed by `{"statement": 5}`. Each
definition, member of a component type and statement of the model the modules are written from (**Provenance** in
§9.5) is exactly one place, and each place is in exactly one entry. The entries stand in the order of the construct's
document, its row, its node and the rule; an entry's places in the order of the modules, the types module first.

**Claims.** An `alt` and each of its alternatives also state what the third check computes again. The `alt` (E7)
claims its atomic state and the transition each alternative is, in textual order. An alternative (E8) claims its
atomic state, its transition, and the states its block exits and enters, each in the order the block does so: the
exit and entry sets of that transition taken from that atomic state. The exits of the halt that follows the entry of
a top-level `<final>` are E12's, and are not among them. The witness states the claim and the validator does not
trust it: it computes the order from P3 and the two sets from §scxml-3.13 over the documents, and compares.

**The validator.** It is the crate `sce-ttcn3-validator/` and its command
`sce-ttcn3-validate <suite.scxml> <export directory>`, which prints each violation on a line of its own, its code
first, and exits 0 when there is none, 1 when there is one or more, and 2 when an input is not read. It shares no
code with the exporter: it depends on no crate of SCE, and reads each of its inputs with a reader of its own, the
suite's documents with `roxmltree` and named as **Constructs** names them, the witness into a model of its own, and
the two modules with a tokenizer and a recursive descent over exactly the TTCN-3 §9.5 writes, each definition and
statement a place as **Places** names it. So a defect of the exporter's reading is not shared by the reading that
checks it. A construct of a module outside that TTCN-3 is a `parse` violation, never a place left out. A document
the validator does not read as written is not read, and is named: a document of the standard library; one that
holds an `<xi:include>` or an `<sce:use>`, which the export expands before it reads the document; and a
`test-component` that holds a `<parallel>`, a `<history>`, a `<script>`, a `<donedata>`, a `<raise>` or a
`<cancel>`, which the profile's check refuses (§8.2) and the third check could not compute the sets of. It holds the
three checks:

1. **Places and entries** (`sce-ttcn3-validator/src/coverage.rs`):
   - every place of the two modules is named by exactly one location (`place-unnamed`, `place-named-twice`, and
     `place-twice` for a module that holds one place twice), and every location names a place
     (`location-dangling`);
   - every construct has exactly one entry of each rule that translates it (`construct-missing`,
     `entry-duplicate`), and every entry names a construct of the documents (`construct-dangling`).
2. **Rule and form** (`form.rs`): each entry's rule translates a construct of its construct's kind (`rule-kind`),
   and every place the entry names has the TTCN-3 form that rule writes for the construct, read from the documents
   (`form`): E1 a type of the document's kind, name, fields, variants or alternatives (a union's in its tag enum's
   order) and its codec functions; E2 a template of its name, type and formal parameters; E3 a port type of its
   messages and their directions; E4 a component type and its ports, timers and variables by name and type; E5 a
   function that runs on its component; E6 a label for the atomic state and an `if` for a guarded eventless
   transition; E7 an `alt`; E8 an alternative that receives on the transition's port with its template, or waits
   for its timer or its PTC, with a guard exactly where the transition has a `cond`, or a `goto`; E9 a send on the
   port with the template; E10 a start or a stop of the timer, a duration the same number of microseconds as the
   document's; E11 the reference, the creation and the kill of the PTC; E12 a `return`; E13 a `setverdict` of the
   value (`inconc` for `inconclusive`) and a `log` of the label; E14 a test case on the MTC's component type in
   the configuration's system, mapping the MTC's ports and calling its function, and the system component type and
   its ports; E15 the control part, the module parameters and an `execute` of the test case with its guard, under a
   selection exactly where the suite writes one. The structure is compared, not the text, and an expression is not
   compared: the export writes it from a typed tree the validator does not build. A statement, a label and a local
   stand in a function of their component, and a member in its component's type.
3. **Order and sets** (`order.rs`, `scxml313.rs`): for each function, the statements before its first label are
   owed the start of the machine and those of each label the atomic state's checks, the start of its invocations
   and its `alt` (§9.5 **Behaviour**). The validator computes each branch's exit and entry sets itself
   (§scxml-3.13, for a statechart of one active atomic state and its ancestors): the domain is the source of an
   internal transition whose targets are all inside its compound source, and otherwise the least common compound
   ancestor of the source and the targets; it exits the active states below the domain, deepest first, and enters
   the states from the domain down to each target and the default entry below a compound one, outermost first; a
   targetless transition exits and enters nothing. A block is owed each exited state's onexit and the kill of its
   PTCs, the transition's content, each entered state's onentry and a default entry's initial content, then the
   `goto` of the atomic state the machine is left in, or the exit of a top-level `<final>` and its `return`. It
   holds the constructs that name each block's places to that order (`block-order`, a `goto` to another state among
   them); the constructs that name the alternatives of each `alt`, in textual order, and the `alt`'s claim, to the
   branches of the state and then of each ancestor, innermost first (`alt-order`, and an alternative that claims
   another state or transition); each alternative's claim of the states it exits and enters to the sets
   (`exit-set`, `entry-set`); and an `alt` or an alternative that claims nothing is `claim-missing`. With the
   second check, which holds each place's text to the construct that names it, this holds the order of the text.

Which rule translates a construct of each kind, and when one must, is one table
(`sce-ttcn3-validator/src/rules.rs`). A rule writes a construct always, or when the documents fix it without the exit and entry set of any transition: a test
case the suite executes and its configuration, an atomic state that waits in an `alt`, a branch of such a state or
of an ancestor, an eventless transition checked with its guard, and the content of a transition written as one of
these. Whether an entry those sets decide is owed (the content of an `<onentry>`, `<onexit>` or `<initial>`, the E12
of a top-level `<final>`, the `goto` of an eventless transition) is a question of the sets, which the third check
computes; the first asks of such an entry only that it name a construct of the documents. Every export the tests of
`sce-build` make of the fixtures is held to the validator (`a_ttcn3_export_is_validated_apart_from_the_exporter`).

### 9.5 The exporter

The export is the translation `T` of §9.1, written by `sce-build/src/ttcn3_export` and run by
`sce-codegen export-ttcn3`. It is not a backend: it is in no `Language`, it holds no W3C obligation, and it reads
the documents of the profile alone (ADR 0091 §8).

- **Input.** One `test-suite` document and every document it reaches through the imports of its test cases, their
  configurations, the components those name, and the port types, templates, event schemas, enums and unions below
  them; and the feature model the suite imports, which E15 alone reads, for the names of its features. A document
  of any other kind a component imports is not read: no rule of §9.2 translates one. Every document read is first
  held to the rules of its kind that no backend decides, the ones `check` holds it to before a language is asked,
  so the export translates no document `check` refuses.
- **Output.** For a suite whose unit name is `S`, its file's stem in lower snake case as every unit of a document
  is named: `S_Types.ttcn`, the types module (E1; ADR 0091 C2), and `S_Types.manifest.json`, its structural
  manifest; `S.ttcn`, the behaviour module; and `S.witness.json`, the witness of §9.4. The behaviour module imports the types module whole
  (`import from S_Types all;`) and holds, in this order: the port types (E3); the templates (E2); the module
  parameters, the suite's in the order it declares them and then one for each feature of its feature model in the
  order of the features' names (E15); the component types, those of the test components (E4) and then the system
  component types of the configurations (E14), each group in the order of their names; the functions (E5 to E13);
  the test cases (E14), in the order the suite executes them; and the control part (E15). A library caller reads
  each definition as its text and as the model it is written from (`Ttcn3Export`). Nothing is written before the
  whole export is computed, so a refused suite leaves no file, and no witness is written of an export not computed
  whole. Each module carries the header of every generated source (`source-hash`); the manifest and the witness
  carry none. The witness is a serialization of the model each definition is written from (**Provenance**, below).
- **Text.** The TTCN-3 is written by the templates under `tools/codegen/templates/ttcn3/`, through the door every
  template passes, read as TTCN-3: a value written into a comment is encoded for it, and a text value is written as
  a character string, a `"` twice and each character outside printable ASCII concatenated as its quadruple
  `char(g, p, r, c)` (-1 §6.1.1), so a module is ASCII whatever its values hold. The output is a function of the
  documents and the generator (`docs/SCE_CODEGEN_DETERMINISM.md`).

**The name contract.** Every name the export writes is a name of the documents, written as it is written: a
document by its unit name; a field, a variant, a union's alternative and a formal parameter by its own. A name the
export cannot write so is refused (`test-profile/export-name`) and never renamed, so that a consumer's own types
module, the test log and the witness need no mapping between two names of one thing. A name is refused when:

- it is not a TTCN-3 identifier, an ASCII letter followed by ASCII letters, digits and underscores (-1 Annex A);
- it is a word the export reserves: the keywords of TTCN-3 and of its extension packages (-1 tables A.3 and A.5,
  both forbidden as identifiers), those of its object-oriented package, and the names of its predefined functions
  (-1 table 15, reserved by A.1.5.0) (`ttcn3_export::names::RESERVED`). The list is wider than one tool
  needs on purpose: a refused name is renamed in its document once, where a reserved name written is a module that
  does not compile;
- it is one identifier with another name of a scope that sees both (-1 §5.2.2, figure 2 of -1 §5.2.0). The
  definitions of the two modules are one scope, since the behaviour module imports the types module whole: the
  types, the codec functions, the port types, the templates, the module parameters, the component types and the
  functions. Every scope below it sees its names: a template's formal parameters; a component type's ports, timers
  and variables; and a function's labels and locals, which see the names of the component type it runs on too. Two
  scopes of one level, two templates or two component types or two functions, may each hold a name the other holds.
  An enumerated value may repeat among enumerated types and is no other name. So a module parameter and a template's
  formal parameter of one name are refused, and so are a module parameter and a component's variable, and a port and
  a state of the component. A test case is named by the unit name of its document, and a configuration's system
  component type by the unit name of the configuration's, both definitions of the module; a system port is a name
  of its system component type's scope. The names the export makes are in the same scopes: the eight integer types,
  the modules `S` and `S_Types`, the codec functions and their parameters `msg` and `octets`, each component's
  functions, the module parameter `PICS_<NAME>` of each feature (E15), and the locals of a function, `v_<ptc>` for
  each PTC it creates and `v_<record>` for each record a branch of it receives into a variable. Where a made name
  and a written one collide, the written one is refused.

**Types (E1).** The types module holds, in this order:

- the integer types of the type grammar, each under its own name, from `type integer uint8 (0 .. 255)` to
  `type integer int64 (-9223372036854775808 .. 9223372036854775807)`;
- each enum, event schema and union the suite reaches, in the order of their names: an enum as
  `type enumerated E { v(n), … }`, its variants in declared order with their declared numbers; an event schema as
  `type record R { … }`, its fields in declared order, an `optional<T>` field `optional`, a text's and a byte
  string's bound in bytes stated as a comment (§6.3), a list `record length (m .. n) of T`; a union as
  `type union U { … }`, one alternative per case in its tag enum's order, named by its variant, the empty case
  `record {}`;
- for each type a port of the suite carries, in the order of the types' names, its codec:
  `external function enc_R(in R msg) return octetstring;` and
  `external function dec_R(inout octetstring octets, out R msg) return integer;`. The export declares them and does
  not implement them (ADR 0091 C2, answer 1). `dec_R` reports as -1's `decvalue` does: 0 when it decoded a message
  and took its octets from the front of `octets`, 1 when it could not, 2 when the octets end before the message
  does.

An enum of an open set (`sce:strict-variants="false"`), whose values an `enumerated` cannot hold, a float, which the
profile leaves out (§8.1), and a list of values that may be absent are refused by name
(`generate/unsupported-feature`).

**Templates (E2).** Each template is `template T name(parameters) := body;`, in the order of the templates' names.
Its type and each value parameter's are named as the types module names them, and a parameter of `optional<T>` is
`template (omit) T p` (-1 §15.8), a value or `omit`. A template or a parameter of a list type is refused by name:
TTCN-3 declares a template and a parameter by a type's name, and a list of the documents has none. Each mechanism
of §6.1 is written as -1 Annex B.1 writes it:

| Matcher | TTCN-3 |
|---|---|
| `specific` | the value: an integer's digits, `true` or `false`, the variant's name, a text as a character string, a byte string `'…'O` in upper-case hex; a formal parameter by its name |
| `record-match` | `{ f := m, … }` in the record's field order; at the root of a template, a field to a line |
| `list-match` | `{ m, … }` |
| `union-match` | `{ v := m }`, and `{ v := {} }` for a case that holds no record |
| `value-list` | `(m, …)` |
| `complement` | `complement(m, …)` |
| `any-value` | `?` |
| `any-value-or-none` | `*` |
| `value-range` | `(lower .. upper)`, a bound not written `-infinity` or `infinity` |
| `omit` | `omit` |
| `template-ref` | the template's name, with its arguments `(p, …)` where it binds any |
| `length-min`, `length-max` | `length(m .. n)` after the matcher, 0 for a lower count not written and `infinity` for an upper |
| `ifpresent` | `ifpresent`, after the length |

**Port types (E3).** Each port type is `type port P message { d T; … }`, each message in document order with its
direction, `in`, `out` or `inout`, so its `in` and `out` lists are stated explicitly, as Eclipse Titan requires
(ADR 0091 answer 1).

**Module parameters.** Each `<sce:module-parameter>` of the suite is `modulepar T name := value;` (-1 §8.2.1), in
the order the suite declares them, its value a literal of §11.4 as a template writes one, the bound of a text or a
byte string stated beside it as a field's is. A component's `<data sce:module-parameter="true">` declares no
variable: every expression that reads it reads the `modulepar` of the same name. The witness names the suite's
declaration and E15, the rule of the suite.

**Component types (E4).** Each test component the suite reaches is `type component C { … }` (-1 §6.2.10.1), named by
its unit name: each port `port P p;` of its port type, in document order; each timer `timer T := d;` with its
default duration, or `timer T;` with none; each variable `var V v := e;` with its initial value. A variable the
export writes is a number, a bool, a variant of an enum, a text or a byte string; a record, a list, a union or a
value that may be absent is refused by name (`generate/unsupported-feature`).

**Behaviour (E5 to E13).** Each component's statechart is a function `function f() runs on C { … }`. Its body is
the entry of the initial configuration and a `goto` to the initial atomic state; then, for each atomic state that is
no top-level `<final>`, in document order:

- `label s;` (E6);
- the checks of the eventless transitions of `s` and of each ancestor, in the order of P3, each
  `if (guard) { block }` or, without a guard, the block itself, which ends the checks;
- the start of each invocation of `s` and its ancestors that is pending, outermost first (E11, below);
- `alt { … }` with one alternative per branch of `s` and of each ancestor, in the order of P3 (E7): `[guard]
  p.receive(t) -> value v { block }`, `[guard] p.receive { block }`, `[guard] T.timeout { block }` or
  `[guard] c.done { block }` (E8). The value redirect is written where the block reads the message, into a local of
  the record's type.

A block is what taking the transition from `s` does, in the engine's own order: each exited state's onexit and the
kill of the PTCs it invoked, the transition's content, each entered state's onentry and, for a state entered by
default, its initial transition's content; then `goto` the atomic state the machine is left in, or, where that is
a top-level `<final>`, its exit and `return` (E12). The exit and entry sets are not computed by the exporter: it
asks the Appendix D core every Rust machine runs on (`sce_rust_runtime::helpers::microstep`, ADR 0064) to take the
transition from the configuration of `s`, over the tables the generator builds, and writes what the core asks the
machine to do (`ttcn3_export::macrostep`). The validator of §9.4 computes them again on its own.

Executable content is written statement for statement: `p.send(t)` (E9); `T.start(d)` with `d` the exact decimal of
the duration's seconds, read from its whole number of microseconds (`0.5`, `0.000001`, never a rounded binary
float), `T.start` for a timer's default, and `T.stop` (E10); `setverdict(v, reason)` with `inconc` for
`inconclusive`, and `log(…)` with the label and the value (E13); `<assign>` as `v := e`, and `<if>`, `<elseif>` and
`<else>` as `if` and `else`. Any other element is refused by name.

**Expressions.** An expression is not read again from its text: it is the typed tree the data model's judge reads
(`forge::expr::resolve`, over `forge::type_ctx::StaticScope`, the scope every backend's lowering reads), and the
export prints that tree (`ttcn3_export::expression`). It writes a variable or a module parameter by its name, a field
of the received message as a field of the redirect's local, a variant by its name, an integer's decimal digits,
`true` and `false`, a text as a character string, a byte string as `'…'O`, `===` and `!==` as `==` and `!=`, the
four relational operators of numbers and enums, and `&&`, `||` and `!` as `and`, `or` and `not` (-1 §7.1), each
operand that is an operation in parentheses. What has no TTCN-3 form that computes the same value is refused by name
(`generate/unsupported-feature`), never approximated: integer arithmetic, bitwise and shift operators, which
`sce-static` computes at the width of their operands and fails on an overflow while a TTCN-3 `integer` has no
width; a conditional expression; a call; an index; a value that may be absent; a float; an assignment of a text or
a byte string that is no literal, whose bound `sce-static` holds at run time and TTCN-3 does not; and an assignment
of an integer from a type wider than the place's.

**PTCs (E11).** A PTC is held in a local of the creating function, `var C v_p := null;`. Its invocation is pending
exactly while its invoking state is active and the reference is `null`, so the start at a label is

```ttcn3
if (v_p == null) {
  v_p := C.create("p") alive;
  connect(mtc:q, v_p:q);
  map(v_p:r, system:s);
  v_p.start(f_c());
}
```

and leaving the invoking state is `if (v_p != null) { v_p.kill; v_p := null; }`. The PTC's name in the
configuration is its name in the test system (-1 §21.3.1), which the test log names it by. It is created `alive`:
a PTC whose behaviour ended stays in `DONE` with its ports and its verdict until its invoking state is left (P9,
P10), which is what an alive component is after its behaviour stops and before it is killed (-1 §21.3.2 to
§21.3.4, §21.3.7; -4 §9.12, §9.16, §9.29a); a non-alive one would be destroyed when its function returns.

The creator applies, as it creates the PTC, every `connect` and `map` of the configuration that names it (P9; -1
§21.1.1; -4 §9.10, §9.32), connects first, then maps, each in document order, and then starts it (-1 §21.3.2; -4
§9.46). Which connects and maps those are depends on the configuration, so a component that invokes a PTC has one
function for each configuration that holds it, `f_<component>_in_<configuration>`, and every other component one
function, `f_<component>`, which every configuration starts. The alternative, one function per component with the
configuration's operations done elsewhere, has no place to do them: TTCN-3 applies a connect only once both
components exist (-1 §21.1.1), a PTC is created inside the creator's behaviour where P9 creates it, and a creator's
function written once would have to branch on a configuration it is not told. With one function for each
configuration the exported behaviour of a component in a configuration is exactly §4.9 for that configuration. The
other end of a connect is named as the creator can name it when it creates the PTC: `mtc`, `self`, the PTC itself,
or another PTC the same component creates, connected then only if that one is created already
(`if (v_o != null) { connect(…); }`), so that the connect is made at whichever creation comes second. A connect
whose other end is a PTC the creator holds no reference to is refused by name: -1 reaches a component outside the
scope of its creation only through a reference passed to it (-1 §21.3.1), which no document writes.

**Test cases (E14).** Each configuration a test case of the suite runs in is a system component type,
`type component C { port P p; … }`, named by the configuration's unit name, with a port of each `<sce:system-port>`
of its port type, in document order (-1 §6.2.10); a configuration that maps no port to the test system is an empty
one. It is written once, however many test cases run in it. Each test case the suite executes is

```ttcn3
testcase t() runs on M system C {
  map(mtc:p, system:s);
  f_m();
}
```

named by its unit name: it runs on the component type of its configuration's MTC, its `system` is the
configuration's system component type (-1 §26.1), its body applies each `<sce:map>` of the configuration whose
component is `mtc`, in document order, and then calls the MTC's function, `f_<mtc>_in_<configuration>` where the
MTC invokes a PTC and `f_<mtc>` otherwise. A map or a connect that names a PTC is not applied here: its creator
applies it as it creates the PTC (E11).

**The control part (E15).** Each feature of the suite's feature model is `modulepar boolean PICS_<NAME>;` with no
default (-1 §8.2.1), `<NAME>` the feature's name upper-cased with each hyphen an underscore, the spelling ADR 0057
gives a feature on every backend that writes it in upper case (`SCE_FEATURE_<NAME>` in C); a feature's name has no
upper-case letter and no underscore, so two features never share one. Which features a run enables is its caller's
to say, as the world is the input of a native runner (§12). The control part executes each test case in the order
of the suite's `<sce:execute>`s (P11; -1 §26.2):

```ttcn3
control {
  execute(t1(), 10.0);
  if (PICS_PTCS and (PICS_GUARD or PICS_REMOVAL)) {
    execute(t2());
  }
}
```

The guard is the second argument, the exact decimal of its seconds as a timer's duration is written (E10), and is
left out where the suite gives none. A test case the suite selects stands inside an `if` whose condition is the
selection: each feature its module parameter, `all` a conjunction `and`, `any` a disjunction `or`, and a join
inside another in parentheses, as every backend writes a suite's selection. The selection is the one the suite's
check has read against the feature model, and a feature the model does not declare is refused by name.

**Provenance.** Every definition of the two modules, every member of a component type and every statement, the
statements of the control part among them, is computed as a model that names the rule of §9.2 that wrote it, the
document and the node it was written from (`ttcn3_export::behaviour::Source`), so the witness of §9.4 is a
serialization of that model (`ttcn3_export::witness`), never read again from the modules' text. A type of the types
module names its document, and its codec functions the same; a port type and a template their documents; a
feature's module parameter names the feature in the feature model; a system component type and its ports the
configuration and its `<sce:system-port>`s; a test case its document; an `execute` its `<sce:execute>`. Each
alternative of an `alt` also holds the states its block exits and enters, as the core computed them, which the
witness states as the alternative's claim.

**The manifest.** `S_Types.manifest.json` is the model the types module is written from, serialized
(`schemas/sce-ttcn3-types-manifest.v1.schema.json`, `pre-release`, a row of `SCE_WIRE_CONTRACTS.md`): every
definition by its name, its kind and its structure, and the codec functions. A consumer that replaces the types
module with one written apart from SCE, with a codec of its own, holds that module to it: the same names and kinds,
the same fields in the same order with the same optionality, the same bounds, lengths, variants, numbers and
alternatives, the same functions. The validator of ADR 0091 answer 12 is a crate of its own that does not depend on
the exporter, and reads the manifest as such a consumer does.

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
  <sce:test-port id="ctl" type="JobPort"/>
  <sce:test-timer id="deadline" duration="2s"/>
  <datamodel>
    <data id="elapsed_limit" sce:type="uint32" sce:module-parameter="true"/>
  </datamodel>
  <state id="requesting">
    <onentry>
      <send type="http://sce.dev/ext/test-port" target="sut" sce:test-template="Request"/>
      <sce:start-timer timer="deadline"/>
    </onentry>
    <transition sce:receive="sut" sce:test-template="CompletedWithin" sce:template-args="elapsed_limit" target="passed">
      <sce:stop-timer timer="deadline"/>
      <sce:setverdict value="pass"/>
    </transition>
    <transition sce:timeout="deadline" target="failed">
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
    <sce:endpoint component="mtc" port="ctl"/>
    <sce:endpoint component="helper" port="ctl"/>
  </sce:connect>
</scxml>
```

The tester's `ctl` port is of `job_port`, which sends `JobRequest` and receives `JobCompleted`. The helper's
`ctl` port is of the port type that turns each direction, so that what one port sends the other receives:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="job_control_port">
  <sce:import kind="event-schema" src="job_request.scxml" as="JobRequest"/>
  <sce:import kind="event-schema" src="job_completed.scxml" as="JobCompleted"/>
  <sce:message type="JobRequest" direction="in"/>
  <sce:message type="JobCompleted" direction="out"/>
</scxml>
```

Two ports of one type connect only where the type receives every message it sends; two ports of `job_port` would
not, since `JobRequest` is on its `out` list and not on its `in` list.

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
  <sce:module-parameter id="elapsed_limit" type="uint32" value="500"/>
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
- `const char *<name>_value_type(void)`, the name a message's value of that type is known by
  (`sce_host_value_t.type`): the type `<name>_value_t` stands for, the same for every template of the type;
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
depth is refused on C11 as on C++ (ADR 0092, section 7). The C11 unit of an event schema holds a byte string of one
entry in the buffer of its bound beside its length (`sce_static_bytes_<N>_t`), as a composition holds one inside a
list, so a byte string is compared by its length and its octets, the other side read through the runtime's view of
bytes, counted by its length and built by the initializer of its type. C11 refuses by name a reference, inside a
list, to a template of an optional type, whose value the list holds in its composition's own optional type and not
in the one the referenced template takes.

Swift generates `test-template` (§12.6, slice 6). The unit is a file `<Name>.swift`, named by the document's file as
every forge unit is. Every unit of a run is a file of one module, so the unit declares a caseless enum `<Name>`, as an
algorithm's unit does, whose members are:

- `Value`, a type alias of the template's type as the unit of an `event-schema` spells a field of that type (a record
  is the schema's payload struct, a value that may be absent `T?`);
- `static func matches(_ value: Value, <the formal parameters, in order>) -> Bool`, which holds each mechanism of §6.1
  by the rules C++ holds it by. A value that may be absent is read through its `map`, whose closure binds the present
  value under a name of its own and states its result, and the answer for an absent value is given by `??`; a text,
  a byte string, a list and a value that may be absent are compared by `==`, which compares a Swift value by its
  contents. A length restriction on a `string[N]` counts characters, through the profile's runtime
  (`SceTestProfile.textLength`, a text's Unicode code points, since a Swift `String` counts grapheme clusters), and on
  `bytes[N]` and `list<T>[B]` octets and elements (`count`);
- for a specific template (§6.2) alone, `static func value(<the formal parameters>) -> Value`, the value it sends,
  which matches it: a record built by the initializer of every field, a field it omits `nil`. A byte string a
  parameter gives is held as given, since a Swift array is a value.

Every enum, record and referenced template the unit names is named bare, as its own unit declares it in the module,
and a referenced template's functions through its enum (`<Name>.matches`, `<Name>.value`); the unit imports the
profile's runtime (`import SceTestProfile`) exactly where it counts a text. Swift joins the tests as Rust, Kotlin,
Go, Python and C11 do. Swift lowers no union, so a template whose type reaches a union at any depth is refused on
Swift as on C++ (ADR 0092, section 7). It refuses by name too a template whose type, or a formal parameter's, holds
at any depth a record or an enum whose own Swift unit is not written, by what that unit is refused for (a record
with a byte string that may be absent, an enum of an open set of variants), so no unit names a type nothing
declares. Every backend generates `test-template`.

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

C11 generates `test-component` (§12.6, slice 6). The unit is the statechart machine the C11 generator writes for any
`sce-static` document, `<file>_sm.h` and `<file>_sm.c`, lowered through the same doors by the same lowering and
spelled for the C11 runtime (`sce/test/test_case_run.h`, which the machine's header includes). Nothing of it is
allocated:

- a branch's guard, the statement taking it, the statements of the profile and a `<log>` are calls on the component's
  `sce_test_component_context_t`, a handle the machine keeps by value (`test_context`) and passes to every call by
  pointer. A branch's guard is its `cond` and then its match; a `cond` that can fail is the head of its `if` in C, the
  statements that evaluate it and `if (<verdict>)`, and the match is then the `if` that verdict opens, so the snapshot
  is asked only of a `cond` that held. A receive branch's match is `sce_test_receives`, given the name and the size of
  the template's type (`<unit>_value_type()`, `sizeof(<unit>_value_t)`) and a function of the machine's own,
  `<machine>_test_matches_<n>`, which the runtime calls with the snapshot's head and the machine as its closure and
  which calls the template's `<unit>_matches` with the template's arguments as the machine holds them, since C has no
  closure to bind them in. The machine includes the header of each template it uses. A variable bound to a formal
  parameter is handed over as the unit takes it: a number, a `bool` and an optional value as the machine holds them, a
  text as the buffer the machine holds it in, of the bound the parameter declares too; the machine holds a value of an
  enum in the type the enum's unit declares, so it crosses as it is, and an optional value of an enum is refused by
  name, as on the other backends;
- the statement taking a receive branch removes the head of its port into a message of its own
  (`sce_test_context_receive`), which stands in the block of the branch's content; a removal the run refused ended the
  run with `error` (P13), and the content does not run. A receive branch whose content reads the message takes it as
  its template's record by the record's name and copies it, field by field, into the member of the machine's payload
  channel its record's event is held in, which the content reads as `_event.data`. The machine and the record's unit
  hold a number, a `bool`, a value of an enum and an optional value of each, a text's among them, in the same C types,
  which are assigned; the unit holds a text in the buffer of its bound and the channel as a pointer, so a text is
  copied into the buffer the machine keeps for that field, whose size the build holds the bound to (`SCE_MAX_DATA_LEN`).
  The unit holds a byte string of one entry in the buffer of its bound beside its length and the channel in an array
  of the same capacity beside its `_len`, so its bytes and its length are copied;
- a `<log>` records its label, a colon and its value as the engine's `<log>` writes it, into a buffer one byte longer
  than a line of the test log holds, so a line the log cannot hold reaches the run long enough to be counted in
  `log_lost` and is never cut on the way;
- a send lends the test-port processor the specific template's value (`<unit>_value`), held in a local of the block
  its request stands in, as `sce_host_send_request_t.value`, under the name of the record's type, which the run copies
  before the processor returns.

A message's value is known by the name of its C type (`sce_host_value_t.type`), which the run compares by its text, so
the unit of a C11 template declares `<unit>_value_type()`, the name of the type its `<unit>_value_t` stands for: one
name, the record's `<Name>Payload_t`, for every template of the record, whatever each template is called. Beside the
machine the header declares `<machine>_component_t`, whose first member is the `sce_test_component_machine_t` the run
drives, the machine beside it, and `<machine>_component_make`, which hands a run the component in storage the factory
of machines owns, or nothing while a run still holds it; the run hands it back through `release`, which tears the
machine down. `start` gives the construction of the machine the test-port processor and the test-component invoker it
registered, sets the context on the machine the construction has cleared, and only then enters the initial
configuration (`<machine>_init_enter`, the entry half of `_init`), so the first `<onentry>` already reaches the run;
`deliver_snapshot` raises the snapshot event and runs the macrostep it starts (`_step`); `terminated` is the machine's
having entered a top-level `<final>`. A C11 root draws its own session id, `<name>_session_<n>`, and takes none from
its host (docs/SCE_ACCEPTED_SUBSET.md, "What C11 does otherwise"), so the component does not name its machine's
session after the component, as the other backends' components do. C11 refuses by name a component that declares
`<sce:action>`, whose machine is initialized with a table of acts the component is given nothing of, and one that
uses a template whose type reaches a union, which C11 does not lower (ADR 0092, section 7).

A C11 component reads a module parameter as the other backends' do. The machine's construction starts the member at
its type's zero, or for an enum at the variant its document declares first, a compound literal of the buffer's own
type for a text or a byte string of a bound, none of which anything reads. `start` sets the context on the machine
the construction has cleared and hands the run's module parameters to `<machine>_component_accept_module_parameters`
through `sce_test_context_read_module_parameters`, which reads every value into a local of the member's type by the
reader a restored variable of that type is read by (`sce/saved_values.h`, an enum's `<type>_saved_read` emitted with
the type whether or not the machine saves) and only then writes them all; a value not given, not of the type or past
its bound ends the run with `error`, and the initial configuration is not entered. A text or a byte string the
machine holds in no buffer of a bound has no place a value is read into, and is refused by name.

Swift generates `test-component` (§12.6, slice 6). The unit is the statechart machine the Swift generator writes for
any `sce-static` document, `<file>_sm.swift`, lowered through the same doors by the same lowering and spelled for the
Swift runtime (`SceTestProfile`, which the machine imports and names every declaration of by its module, so no
declaration of the machine's own module can hide one):

- a branch's guard, the statement taking it, the statements of the profile and a `<log>` are calls on the component's
  `ComponentContext`, read through one accessor of the policy (`sceTestContext`), which stops the program where the
  component bound none. A branch's guard is its `cond` and then its match, joined by `&&`. A receive branch's match is
  `SceTestProfile.receives`, given the template's `Value` and a closure written inside the call's parentheses that
  calls its template's `matches`, so the guard stands as one expression in its `if`. Every unit is a file of the
  machine's module, so a template's `Value`, `matches` and `value` are named through the enum its unit declares and
  the machine imports nothing for them. A variable bound to a formal parameter is handed over as the machine holds
  it; a variable of an enum crosses into the variant of the same declared name of the enum unit's type, and an
  optional value of an enum is refused by name, as on the other backends. A verdict is the runtime's case, written as
  an implicit member (`.pass`);
- the statement taking a branch stands in a block of its own before the branch's content, so what it declares stands
  beside nothing of the content's. A receive branch whose content reads the message takes it as its template's
  `Value` and copies it, field by field, into the payload property the machine's channel holds its record's event in
  (`pending<Event>Payload`), which the content reads as `_event.data`; the record's unit holds it in `<Stem>Payload`,
  named by the schema's file, and the machine in a struct of its own. The two hold a number, a `Bool`, a text, a byte
  string and an optional value of one in the same Swift types, each a value, which are copied as they are, and an
  enum in enums of their own, so an enum field takes the variant of the machine's enum that its declared name names
  (`init?(declaredName:)`), an optional one where it is present. Swift so generates the message of a record with a
  text, a byte string or an enum field that Rust refuses, as Kotlin, Go and Python do. Both enums are written from
  one enum document, and the guard has held the message to the template's type, so a name one enum declares and the
  other does not, or a message of another type, stops the program and is not read;
- a `<log>` records its label, a colon and its value as the engine's `<log>` shows it (`String(describing:)`), an
  absent optional value as `null` and a present one as its value, as the other backends record them;
- a send hands the test-port processor the specific template's value (`value`) as `HostSendRequest.value`, a
  `HostSendValue`.

Beside the policy the file declares `<Name>Component`, a `ComponentMachine` of the runtime: `start` builds the
policy, gives it the session id the component's name gives (`setSessionID`), binds the context on it, builds the
engine on it, registers the test-port processor and the test-component invoker on the engine and starts it
(`initialize`); `deliverSnapshot` processes the snapshot event (`processEvent`), which runs the macrostep it starts;
`terminated` is the engine's having entered a top-level `<final>` (`isInFinalState`). Every Swift policy draws a
session id and declares `setSessionID`, whether or not a saved state can hold its machine, so the component's
session id needs no saved shape. Swift refuses by name a component that declares `<sce:action>`, whose policy is
initialized with a host the component is given nothing of, and one that uses a template whose type reaches a union,
which Swift does not lower (ADR 0092, section 7).

A Swift component reads a module parameter as the other backends' do. The policy's property is declared at its type's
zero, a text and a byte string at the empty value whatever their bound, since the policy holds each in a type of any
length, or for an enum at the variant its document declares first, named through the machine's enum type, none of
which anything reads; the starts are the one walk the other backends' are
(`forge::generator::swift_module_parameter_starts`). The machine's enum type declares its saved form
(`SavedEnumeration`) with the type, whether or not the machine saves. Then `start`, after it binds the context on the
policy and before it builds the engine, calls `ComponentContext.readModuleParameters` with the policy's
`sceAcceptModuleParameters`, which reads every module parameter from the run's values by the reader a restore reads a
variable of its type by (`SavedValues`, a text and a byte string held to their bounds, `SavedValues.enumeration` for an
enum; the one spelling `_macros/swift_saved_value.jinja2` gives the saved state's restore too) before it writes any. A
refused read ends the run with `error` and leaves the engine unbuilt, so the component delivers nothing.

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
by name (`test_component::snapshot::lower`), since its value is the suite's; every backend generates both.

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

C11 generates `test-configuration`, `test-case` and `test-suite`, and writes no file for `test-port-type` (§12.6,
slice 6). Each table is a header `<file>.h`, named by the document's file as every forge unit is, over the profile's
runtime (`sce/test/test_case_run.h`, `sce/test/suite.h`); it is header-only (`static inline`), C has no namespace,
so each declaration carries the document's name, and a table includes the header of each unit it reads: a test
case its configuration's, a suite its test cases', and a configuration each component's machine, `<machine>_sm.h`.
Nothing is allocated, and the storage a run needs is the caller's:

- a configuration declares `<name>_configuration()`, the `sce_test_configuration_t` the runtime reads, constant
  storage each of whose tables is absent (`NULL`, `0`) where the configuration has none; `<name>_machines_t`, the
  storage of its components' machines, an array of `<machine>_component_t` for each component type, as long as the
  configuration has components of that type, which is the most a run of it holds at once; `<name>_machines()`, the
  `sce_test_machine_factory_t` over that storage, which makes a machine of a type from the first of its components
  no run holds and none of a type the configuration does not name; and `<name>_module_parameters()` and
  `<name>_module_parameter_count()`, the ids its components read;
- a test case declares `<NAME>_NAME`, its name, `<name>_machines_t`, its configuration's storage, and
  `<name>_test_case(machines)`, the `sce_test_case_t` of its name, its configuration's tables, the factory of
  machines over the storage it is handed, and the module parameters they read;
- a suite declares `<name>_storage_t`, which the caller places and which outlives every run of the suite: the
  `sce_test_suite_t`, its `<sce:execute>` entries and the storage of the machines of each of its test cases; and
  `<name>_suite(storage)`, which clears the storage, so every component starts free, and answers the suite in it:
  each module parameter at its default, the compact JSON text of the saved value of its declared type (a number
  narrower than 64 bits and a `bool` as the value they are, a 64-bit integer as its text, a text as itself, a byte
  string as its Latin-1 text, an enum's variant by the name its document declares), within
  `SCE_TEST_MAX_MODULE_PARAMETERS`, which the header asserts at compile time; and each entry in run order, with its
  test case, its guard in microseconds and its selection as a function of its own, `<name>_selects_<n>`, over the
  world's features, whose returned expression stands bare as Rust's closure's body does.

The runner `sce_test_run_suite` runs the test cases a world selects (§12.4), in one run the caller places.

Swift generates `test-configuration`, `test-case` and `test-suite`, and writes no file for `test-port-type` (§12.6,
slice 6). Each table is a file `<Name>.swift`, named by the document's file as every forge unit is, over the profile's
runtime (`SceTestProfile`). Every unit of a run is a file of one module, so each table declares its members in a
caseless enum named by its document, as a template's unit does, and names what it reads bare, as the module declares
it: a configuration each component's class, `<Machine>Component`, which its machine's file declares, a test case its
configuration's enum and a suite its test cases'. Each type and function of the runtime is named by its module
(`SceTestProfile.Configuration`), which the table imports, so no enum of the module (a document named `test_case`
declares `TestCase`) hides one of the runtime's:

- a configuration declares `configuration()`, the `Configuration` the runtime reads, every field written and an empty
  table as empty; `machine(_:)`, which makes the `<Machine>Component` of each type and nil for another; and
  `moduleParameters()`, the ids its components read;
- a test case declares `name` and `testCase()`, the `TestCase` of its name, its configuration's tables, a factory of
  machines that calls its configuration's `machine`, and the module parameters they read;
- a suite declares `suite()`, the `Suite` of its module parameters at their defaults, each the saved value of its
  declared type (a number, a `bool`, a text and a byte string by `SavedValues.of` of a literal of the type, a number's
  and a byte string's coerced to it so the writer of that type is the one called, which writes a 64-bit integer as its
  text and a byte string as its Latin-1 text; an enum's variant by the name its document declares, written as a text
  is), which imports the SCE runtime (`SceRuntime`) where a module parameter is declared, and of its `<sce:execute>`
  entries in run order, each with its test case, its guard in microseconds or `nil`, and its selection as a closure
  over the world's features, whose body stands bare as Rust's closure's does.

The runner `SceTestProfile.runSuite` runs the test cases a world selects (§12.4).

Which rows each table holds, in which order, and what is refused is decided once for every backend
(`forge::generator::test_tables`); a backend spells them. The runner `sce_runtime.testprofile.run_suite` runs the
test cases a world selects (§12.4). Every backend generates every kind of the profile.

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
| Round scheduler, `NOW`, port queues and snapshots, timer bindings, verdicts, entity table, guard | a runtime library per backend (C++: `sce/include/test/`, Rust: `sce_rust_runtime::test`, Kotlin: `com.sce.runtime.test`, Go: the package `testprofile` of `sce-go-runtime`, Python: `sce_runtime.testprofile`, C11: the headers `sce/test/` of `backends/c/runtime`, Swift: the library `SceTestProfile` of `backends/swift/runtime`) | it is the same for every suite, so it is written once and tested on its own (Zero Duplication) |
| A component's machine | generated from the `test-component`, by the backend's statechart generator | it is a statechart (§7) |
| A template's matcher and a specific template's value | generated from the `test-template`, over the record types the statechart generator already writes for an `event-schema` | matching is on values, not on encoded bytes, so no codec takes part (C2) |
| Which port is connected or mapped to which, and the MTC and PTC of each component | generated tables from the `test-configuration` and `test-case` | they are data |
| The test cases a world of the features runs, with guards and module parameters | a generated table from the `test-suite` | it is data |
| The SUT's side of a mapped port | a system adapter: an interface the runtime calls, shaped after TRI's `triSend` and `triEnqueueMsg` (ADR 0091 answer 1) | the codec and the transport are the adapter's |

Every generated unit states the runtime it names (`Runtime:` in its header), the tier of its document. A component's
machine and the tables of a configuration, a test case and a suite are on the SCE runtime. A template over a record,
the type of a message, is on it too, since its unit writes the message's type name and canonical form in the test log
with the profile's runtime (§12.5); so the `test-template` kind's bound is the SCE runtime, and a template over any
other type names no runtime. A `test-port-type` writes no file.

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
| a send (P6) | a host Event I/O Processor for the test-port URI, registered by the runtime (ADR 0084). The request names the port and carries the template's value, as the value the generated code holds and never an encoding (C++ and Rust: `HostSendRequest::value`, Kotlin and Python: `HostSendRequest.value`, Go: `HostSendRequest.Value`, C11: `sce_host_send_request_t.value`, a view of the value's bytes beside the name of its C type, Swift: `HostSendRequest.value`, a `HostSendValue` that holds any value Swift compares and shares across threads). The runtime appends the value to the queue of the connected port, or hands it to the system adapter when the port is mapped. |
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
it ends rather than returning a list, in one run the caller places, Swift: `runSuite` of `SceTestProfile`): given
the suite's tables, a world of its features and a factory of system adapters, it runs the test cases the world
selects, one at a time on the calling thread, in the suite's order (P11), each a fresh run from `NOW = 0` against a fresh adapter,
and returns each one's result. The caller may give a module parameter another value than the suite's default; a
value for a parameter the suite does not declare is the caller's mistake and is refused before any test case runs.
A test case whose components read a module parameter the run has no value for, or for which no adapter is given,
ends with `error` (P13) before any of its components is created.

### 12.5 Verdicts, the result and the test log

The runtime keeps `E-VERDICT` for each component and computes `TC-VERDICT` over the MTC and every PTC, removed or not
(P14), with `error` above the four verdicts a document sets (P13). A test case's result is its verdict, the lines of
its log, and its test log. The lines are for a reader: each send, receive, timeout, verdict and log line, the creation
and removal of each component, and the cause of an `error` in the runtime's own words, each with its `NOW` and its
component. The test log is what ADR 0091 C3 compares between the native run and the TTCN-3 run of a suite.

**The test log** (ADR 0091 C3 and answer 8) is the wire surface `schemas/sce-test-log.v1.schema.json`, `pre-release`,
registered in `SCE_WIRE_CONTRACTS.md`. Each run of a test case writes one:

```json
{"records":[…],"schema":"sce-test-log.v1","suite":"runtime_suite","test_case":"answers_the_job","verdict":"pass"}
```

`verdict` is the test case verdict, spelled as TTCN-3 spells it: `none`, `pass`, `inconc`, `fail` or `error`. Each
record has `at`, `NOW` when it was written, and `kind`, an event of TCI's test logging interface (ES 201 873-6) in
kebab case, with the members of its kind. A component is named `mtc` or by the PTC's name in its configuration. On the
TTCN-3 side the records are written from the tool's log events (answer 8); the native runtime writes the same kinds at
the points of the semantics this table names:

| `kind` | Members | Written | Rule |
|---|---|---|---|
| `test-case-started` | | as the run starts, at `NOW = 0`, before the MTC is created | P11 |
| `component-created` | `component`, `type` | when the run creates a component, before its machine starts: the MTC as the test case starts, a PTC when the invocation that creates it starts | P9 |
| `message-sent` | `component`, `port`, `type`, `value` | when a component sends, before the message is appended to the connected port or handed to the system adapter | P6 |
| `message-received` | `component`, `port`, `type`, `value` | when a receive branch removes the head of its port, before the branch's content runs: the message was received and matched | P4 step 1 |
| `timer-started` | `component`, `timer`, `duration_us` | at `<sce:start-timer>`, a restart of a running timer included | P7 |
| `timer-stopped` | `component`, `timer` | at `<sce:stop-timer>` of a running timer. The stop of one that is not running changes nothing and writes nothing | P7 |
| `timer-timed-out` | `component`, `timer` | when a timeout branch takes its timer, before the branch's content runs. An expiry no branch takes writes nothing | P4 step 1 |
| `verdict-set` | `component`, `verdict`, `reason` | at `<sce:setverdict>`, with the document's reason where it is not empty; and at each dynamic error, with `error` and no reason (below) | P12, P13 |
| `log` | `component`, `text` | at a `<log>` | P8 |
| `component-done` | `component`, `verdict` | when a component, the MTC included, enters a top-level `<final>`, with its local verdict then. A PTC removed before it terminates writes none | P10 |
| `test-case-ended` | `verdict` | when the test case verdict is computed, at the `NOW` the run ended at | P14 |

The records stand in the order the round scheduler produces them (§12.4): those of one macrostep in the order its
statements run, and the components of a round in their creation order. A test case that cannot start (§12.4) writes
`test-case-started` and `test-case-ended` with `error` alone, since no component was created. The `error` of a dynamic
error carries no reason, because its cause is the runtime's own prose, which the other side would not write alike; the
lines of the result keep it.

**The message's type.** A value in TCI always carries its type, so a message's `type` is the name of its record as the
types module of the export names it (rule E1, §9.5): the unit name of its `event-schema` document, the stem of the
document's file in lower snake case. Two records of one shape are then two types in the log, as they are on the TTCN-3
side.

**The value form.** A message's `value` is in the canonical form of the profile's types (§6.3), built from texts, truth
values, arrays and objects alone:

| Type | Form |
|---|---|
| `intN`, `uintN` | a text of the decimal digits, since an `int64` or a `uint64` is past what a reader that holds numbers as doubles reads exactly |
| `bool` | `true` or `false` |
| `enum:X` | the name the document declares for the variant. A number no variant of an open set declares, which the export refuses (§9.5), is the text of its digits |
| `string[N]` | a text |
| `bytes[N]` | the octets in upper-case hex, two digits each |
| `record:X` | an object of its fields by their declared ids; an absent field is omitted, as TTCN-3's `omit` is |
| `list<T>[B]` | an array of its elements in order |
| `union:X` | an object of one member, named by its case's variant, whose value is the case's record, or `{}` for a case that holds none |

The form is the type's, not the language's: a byte string and a list of `uint8` are one Rust type and take two forms.
So the generated unit of every template over a record writes its record's type name and its value's form by the types
its document declares (Rust: `TYPE` and `canonical`, with the writers of `sce_rust_runtime::test::canonical`; C++:
`TYPE` and `canonical`, with the writers of `SCE::Test::Canonical` in `sce/include/test/Canonical.h`), and a send
carries the value with both (Rust: `sce_rust_runtime::test::message_value`, a `MessageValue` that is a `TestValue`;
C++: `SCE::Test::messageValue`, a `SCE::Test::MessageValue`, which the request's `std::any` value holds). The walk over
a record's types that decides each form is shared by the backends' generators
(`forge::generator::test_template_walk::canonical_value`), and each spells it in its language. Such a unit names the profile's runtime, so a template over a record is on the SCE runtime and its unit's
header says `Runtime: sce_runtime` on every backend (§12.2). A system adapter gives the type name and the writer of
each value it hands the run.

**The text.** A log is written in the JSON Canonicalization Scheme (RFC 8785): no whitespace, the members of each object
sorted by their names as UTF-16 code units, each text escaped as ECMAScript's `JSON.stringify` escapes it, which is the
one form every engine writes (`tests/json_text/string_escape.json`). An instant, a duration and every integer of a
value are texts, so the log holds no number, and two logs agree exactly when their bytes do. Rust:
`TestCaseResult::canonical_log(suite, test_case)` and `CaseResult::canonical_log(suite)`, over the scheme's writer
`sce_rust_runtime::json::write_canonical`. C++: `TestCaseResult::canonicalLog(suite, testCase)` and
`CaseResult::canonicalLog(suite)`, the records of `sce/include/test/TestLog.h` written at the same points of the
semantics, over the scheme's writer `SCE::Saved::writeCanonical` (`sce/include/saved/SavedJson.h`). Kotlin: the same
two functions, the records of `com.sce.runtime.test.TestLogRecord` written at the same points, over the scheme's
writer `com.sce.runtime.Json.writeJcs`; the unit of a template over a record declares `TYPE` and `canonical`, with the
writers of `com.sce.runtime.test.Canonical`, and a send carries the value with both
(`com.sce.runtime.test.messageValue`, a `TestValue` on `HostSendRequest.value`). Go: `TestCaseResult.CanonicalLog`
and `CaseResult.CanonicalLog`, the records of `testprofile.TestLogRecord` written at the same points, over the
scheme's writer `sce.WriteJCS`, which sorts the members by their UTF-16 code units and not by Go's order of strings;
the unit of a template over a record declares `TYPE` and `Canonical`, with the writers of the package
`testprofile/canonical`, and a send carries the value with both (`testprofile.MessageValue`, a `testprofile.TestValue`
on `HostSendRequest.Value`). Python: the same two functions, `canonical_log`, the records of
`sce_runtime.testprofile.testlog` written at the same points, over the scheme's writer
`sce_runtime.saved_json.write_jcs`, which sorts a name by its UTF-16 encoding and escapes a lone surrogate as
`JSON.stringify` does; the unit of a template over a record declares `TYPE` and `canonical`, with the writers of
`sce_runtime.testprofile.canonical`, and a send carries the value with both (`sce_runtime.testprofile.message_value`,
a `TestValue` on `HostSendRequest.value`, which the message holds as its `test_value`). C11:
`sce_test_case_result_canonical_log(result, suite, test_case, buf, cap, &len)`, which writes into the caller's buffer
or, given none, measures, called in the runner's report of each result; the records of `sce/test/test_log.h` written
at the same points into bounded storage of the run, a record past it ending the run with `error` and refusing the log,
over the scheme's writer `sce_jcs_t` (`sce/saved_json.h`), which sorts an object's members in the buffer as it closes;
the unit of a template over a record declares `<name>_canonical` and `<name>_record_type`, with the writers of
`sce/test/canonical.h`, and a send lends the value with its record type (`sce_test_message_value_t`, on
`sce_host_send_request_t::value`). The Rust runtime wrote the test log first and the C++, Kotlin, Go, Python and C11
runtimes write the same one: the run of `answers_the_job` of the generated `runtime_suite` is held on all six to one
file,
`sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json`, so their agreement is one set of
bytes. Each other backend's runtime writes the same records, in the same text.

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
   refused there, as on C++, Go and Python. C11 generates test components next, through the doors of §12.3, each in
   storage its factory owns, and runs them through `sce_test_case_run_t`; the refusal is lifted for C11 for
   `test-component`, but for a component that declares `<sce:action>` and for one whose template reaches a union.
   C11 then
   generates the configuration, case and suite tables over the same rows, each including what it reads by its
   header and making every machine from storage the caller places, and runs the examples end to end through
   `sce_test_run_suite`; the refusal is lifted for C11 for the remaining kinds, and for a component's module
   parameter. Swift's runtime library of §12.2 is the library `SceTestProfile` of the runtime package, over
   `SceRuntime`, the Rust one ported type for type and held to its cases. Its port, timer, instant and duration are
   `TestPort`, `TestTimer`, `TestInstant` and `TestDuration`, since Swift and Foundation declare a `Duration`, a
   `Timer` and a `Port` of their own that a module of the same names would hide or make ambiguous; a misuse the Rust
   runtime panics on (a re-entrant call, a context that outlived its component) stops the program, and a
   `TestCaseRun` is not copyable and its `run` consumes it, so a second run of it does not compile, as in Rust. It
   has `textLength` for a template's length restriction, a text's Unicode code points, and the Swift host send
   request carries a test-port send's value (`HostSendRequest.value`, a `HostSendValue`), which a saved state does
   not carry: a save that finds one waiting refuses by name. Swift generates templates, and the refusal of §11.9 is
   lifted for Swift for `test-template`; Swift lowers no union, so a template whose type reaches one stays refused
   there, as on C++, Go, Python and C11, and so does one that holds a record or an enum whose own Swift unit is not
   written. Swift generates test components next, through the doors of §12.3, and runs them through
   `SceTestProfile.TestCaseRun`; the refusal is lifted for Swift for `test-component`, but for a component that
   declares `<sce:action>` and for one whose template reaches a union. Swift then generates the configuration, case
   and suite tables over the same rows, each the members of an enum named by its document in the one module every
   unit of a run is a file of, and runs the examples end to end through `SceTestProfile.runSuite`; the refusal is
   lifted for Swift for the remaining kinds, and for a component's module parameter. Every backend now generates
   every kind of the profile.
