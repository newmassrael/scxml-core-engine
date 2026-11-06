# SCE test profile — operational semantics

- Status: specification of ADR 0091, order of work step 1 (§1 to §10), and the document forms of step 2 (§11).
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
4. **Selection.** Each waiting component evaluates its branches over its own snapshot (§4.3). Components whose
   selection finds a branch take it.
5. **Execution.** Each component that took a branch runs its macrostep to completion (§4.4). Messages it sends are
   appended to the receiving port queues. They are not in any snapshot of this round, because every snapshot of the
   round was taken in step 3.
6. The round ends when quiescence holds again.

Within a round, the order in which components run steps 4 and 5 does not change the result. Each component reads only
its own snapshot and its own variables, and nothing it writes is read by another component in the same round. So the
run is determined by the input trace and the document.

**Lemma 3.2 (the schedule is a TTCN-3 schedule).** Every round is a sequence of steps that -4's evaluation procedure
may take: the update phase delivers input and advances timers (-4 §8.6.1.2); the selection phase may select the
entities in the order the round uses (-4 §8.6.1.3); taking a snapshot and evaluating branches are separate nodes of
`<alt-stmt>` (-4 §9.3.0), so other entities may run between one entity's snapshot and its evaluation.

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
  of waiting, with its counts of port messages sent and received, in the current epoch;
- **the logical clock**: when no branch of any component fired in the previous round's grant, the next grant first
  advances `NOW` (§3.3).

The two share the one condition and the one protocol: a grant either starts a round at the present instant or
advances time and then starts one.

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
`float32` or `float64` (§8.1).

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
- Feature conditions appear only on `<sce:execute>`. The other kinds of the profile take none.
- At most one `feature-model` import. A `requires` names only its features
  (`validation/feature-undeclared`, the code ADR 0057 gives a condition that names a feature its model does not
  declare; ADR 0091 §3).

### 11.9 Generation

The six kinds are parsed and checked on every backend from the step that lands them. Until the native runtime of the
order of work step 3 lands in a backend, generating a document of the profile on it is refused by name
(`generate/unsupported-feature`).
