# ADR 0089 — A run tells each transition it takes by the row of its transition table

- Status: Accepted (2026-11-05). The owner decided on 2026-11-05 that transition coverage is measured and not
  predicted, by a read-only transition observer on every runtime, matching the one the Python engine already has,
  and that it is a public runtime surface recorded in an ADR of its own. Landed with this ADR on six runtimes; the
  Python engine's observer gains the row's index.
- Date: 2026-11-05
- Amends:
  - ADR 0087, section 9.4 item 1 and Landing order item 7: "every transition of the harness taken" is measured by
    this observer, as the rows of the harness's emitted transition table seen across a run;
  - docs/SCE_TRANSITION_TABLE.md: a section on reading a run against the table;
  - docs/SCE_ACCEPTED_SUBSET.md section 2.20: one paragraph pointing at that section.
- Scope:
  - the Rust runtime (`backends/rust/runtime`: the `transition-observer` feature, `TransitionTaken`,
    `Engine::set_transition_observer`, `DocumentTables::position_in_source`);
  - the Go, Kotlin and Swift runtimes (`TransitionTaken` and a setter on the engine);
  - the C++ engine (`StaticExecutionEngine::TransitionTaken`, `setTransitionObserver`);
  - the C11 machine template (`c/state_machine.h.jinja2`, `c/microstep.jinja2`) and `sce/types.h`;
  - the Python engine's `"transition"` record (`backends/python/runtime/sce_runtime/engine.py`), which gains
    `index`;
  - a test on each of the seven lanes, and a structural test of the C11 output in `sce-build`.
- Out of scope: the Interpreter (`sce_static.js`) and the C++ Interpreter engine. ADR 0087 section 7.1 refuses the
  reference harness there by name, so there is no harness run on them whose coverage this would measure.
- Related: ADR 0053 (the Rust runtime's features only add), ADR 0064 (Appendix D runs once per program over a
  document's tables), ADR 0084 (what reaches an engine enters through one door, on the owner's thread), ADR 0087
  (the harness and its two layers of testing).

## Context

ADR 0087 section 9.4 tests the system around a model exhaustively: every sequence of outcome classes up to the
repair bound is run against the exhaustive provider, on every lane that runs the reference harness, and coverage is
required of it, "every transition of the harness taken (100% transition coverage)". The requirement needs a
measurement on each lane, and only one lane could make it. The Python engine has `Engine.set_observer`
(`backends/python/runtime/sce_runtime/engine.py`), which tells a callback `"transition"` for each transition a
microstep takes, with its source, its event, its targets and whether it is internal. No other runtime tells a host
which transition it took:

- the Rust, Go, Swift and C++ engines and the C11 machine have no such callback;
- the Kotlin engine has `transitions`, a `SharedFlow<TransitionRecord>`, which a host collects later, which is
  emitted only for a transition an event took and only when it has a target, and whose record names no
  transition, only a source, an event and the state reached.

A document that declares `sce:transition-table="emit"` (docs/SCE_ACCEPTED_SUBSET.md section 2.20) carries its
transitions as constant data in every backend, one row per written transition, each named by `from` (the state it
is written in) and `index` (its position among that state's own transitions in document order). The table is the
denominator coverage needs; what is missing is the numerator, the set of rows a run took.

## Decision

### 1. One callback, told each transition taken, by its row

Each runtime gains a read-only transition observer: a callback invoked once for each transition the engine takes.

1. **When.** After the selection and before the microstep's first exit, each transition of the optimal enabled set
   once and in selection order. This is exactly where the Python engine tells its `"transition"`
   (`Engine._take_microstep`, the one place a selection becomes a microstep), so the eventless chain and the
   event-driven step are told alike and neither can be missed. An event that selects nothing tells nothing.
2. **What.** The pair the emitted transition table names its rows by: the source state, as the machine's own state
   type (the type the table's `from` holds), and the index, as the table's `index` type (an unsigned 32-bit
   integer in Rust, Go, C and C++, `Int` in Kotlin and Swift, an `int` in Python;
   docs/SCE_TRANSITION_TABLE.md, "The type of `index`"). A run's set of pairs is therefore a set of rows, compared
   with the table without a conversion, and the share of the table it holds is the run's transition coverage.
3. **Where the pair comes from.** The selection already carries it on six lanes: the Go, Kotlin, Swift, C++ and
   Python selection record (`EnabledTransition`) and the C11 one (`<name>_transition_info_t`) hold the source and
   the transition's index among the source's transitions, written by the generator from the model's
   `transition_index`, the position the table's `index` is computed from (`sce-build/src/transition_records.rs`).
   The Rust engine selects `TransitionId`s, a number over the whole document; its tables number the written
   transitions first, grouped by source in state order and within a source in document order
   (`sce-build/src/document_tables.rs`), so the index is the transition's number less the number of the first row
   with the same source (`DocumentTables::position_in_source`). It is computed only when an observer is set, and
   no table of first rows is added to any machine.
4. **Read-only.** The observer is given values and asked nothing. A machine with an observer set takes the same
   transitions, reaches the same configurations and makes the same sends as one without; no lane lets the
   observer's answer reach the run.

### 2. The surface on each lane

| Lane | Setter | Told | Unset |
|---|---|---|---|
| Rust | `Engine::set_transition_observer(f)`, `clear_transition_observer()`; feature `transition-observer` | `TransitionTaken<P::State> { source, index: u32 }` | `clear_transition_observer()` |
| Go | `(*Engine).SetTransitionObserver(func(TransitionTaken[S]))` | `TransitionTaken[S]{Source, Index uint32}` | `nil` |
| Kotlin | `StateMachineEngine.transitionObserver: ((TransitionTaken<S>) -> Unit)?` | `TransitionTaken<S>(source, index: Int)` | `null` |
| Swift | `Engine.setTransitionObserver(_:)` | `TransitionTaken<State>(source:, index: Int)` | `nil` |
| C++ | `StaticExecutionEngine::setTransitionObserver(std::function<void(const TransitionTaken &)>)` | `TransitionTaken{source, std::uint32_t index}` | an empty function |
| C11 | `<prefix><name>_observe_transition(const <name>_t *, <name>_state_t, uint32_t)`, defined by the host, under `<PREFIX><NAME>_TRANSITION_OBSERVER` | the call's arguments | the switch at 0 (the default) |
| Python | `Engine.set_observer(f)`, kind `"transition"` | `fields["source"]` (the state's name, which is `str` of the table's `from_state`), `fields["index"]` | `None` |

Every lane's observer is set before the run starts to be told the transitions the first macrostep takes; on every
lane but C11 that is between constructing the engine and initializing it.

### 3. Zero cost when unused

The observer is a test and diagnostic surface, and a bare-metal image must not pay for it.

- **C11: a compile-time switch and a link-time hook.** `<PREFIX><NAME>_TRANSITION_OBSERVER` defaults to the build's
  `SCE_TRANSITION_OBSERVER` (`sce/types.h`), which defaults to 0. At 0 the preprocessor removes the hook's
  declaration and its one call in `microstep`, so the translation unit a compiler sees is the one it saw before
  this ADR: no field, no branch, no byte. At 1 the machine calls a function the host defines, directly. A pointer
  in the machine's struct was rejected because `_init` zeroes the struct and runs the first macrostep in the same
  call (the rule `_init_with_host_processors` already states: "a value this call MEMSETS cannot be set before
  it"), so a pointer could not be installed before the transitions that macrostep takes; passing it to `_init`
  would add an argument to every `_init` variant. A build that turns the switch on and defines no hook does not
  link, so a coverage run cannot silently observe nothing. The per-machine switch lets a build observe one machine
  of several without defining hooks for the rest.
- **Rust: a cargo feature.** `transition-observer` adds the field, the setter and the call; without it the engine
  has neither, and its code is what it was (the feature only adds, ADR 0053). A const generic was rejected because
  it would be a type parameter of `Engine` that every generated machine and every host names; a bare function
  pointer checked for null was rejected because it would be a field and a branch in every engine of every image,
  the cost ADR 0053 keeps behind features (`macrostep-diagnostics` is the precedent). With the feature on, the
  observer is any `FnMut + Send + 'static` closure in the `std` profile and a `fn` pointer in the bare-metal one,
  which has no allocator to box a closure in; the `std` setter accepts a `fn` pointer too, so a bare-metal crate
  that names the setter still compiles in a build that unifies `std` in. With the feature on and no observer set,
  each microstep checks one `Option`.
- **Go, Kotlin, Swift, C++, Python.** A nullable function held by the engine and checked once per microstep, as
  each engine already holds its completion callback. These are hosted runtimes that already allocate a selection
  per microstep (a slice, a list, an array, a `std::vector`), against which one pointer comparison is not
  measurable; none of them runs on the bare-metal profile, which is C11's and Rust's.

### 4. Ordering and threading

The observer runs on the thread that drives the engine (the owner thread of ADR 0084), synchronously, inside the
macrostep and inside the host call that drove it (`initialize`, `step`, `tick`, an event delivery, a clock
advance). It must not call back into the engine: the engine is between selection and exit, and no lane defines
what a reentrant call does. Two lanes enforce it by type: the Rust closure is `'static`, so it cannot hold the
`&mut Engine` a call needs, and the C11 hook is handed a `const` machine, which no machine function that changes
it takes. On the other lanes it is the contract the Python engine already states. A failure in the observer is not
swallowed (a panic in Rust and Go, an exception in Kotlin, C++ and Python): a trace that silently stopped would
read as "nothing more happened", which is the one thing it exists to tell apart. An observer sees its own engine's
transitions; an invoked child session is another engine, with an observer of its own.

### 5. Python parity

The Python engine's `"transition"` record gains `index` (`EnabledTransition.transition_index`, which the generated
Python machine already sets), beside `source`, `event`, `targets` and `internal`. Its `source` is the state's name,
and the Python table's `from_state` is the machine's state enum, whose `str` is that name, so a Python run's pairs
compare with its table as `(str(row.from_state), row.index)`. Its other kinds (`event`, `discarded`, `exit`,
`enter`) are unchanged, and no other lane gains them: coverage needs the transitions, and a lane that needs the
rest asks for it in a decision of its own.

### 6. Documents of a feature model

A document whose elements are gated (ADR 0057) has one table per variant, and a row's `index` is its position among
the transitions the variant keeps (docs/SCE_TRANSITION_TABLE.md, "A document written against a feature model").
Where a backend generates one output for every build (Rust, C11, C++), the index the generated selection carries is
the model's `transition_index`. This ADR claims the pair equals the row for a document without a feature model,
which every reference harness is; holding it for a gated document is left to the harness of such a document, and
is not claimed here.

## Tests

Each lane runs `integration_resources/a_computed_event_name_is_matched_like_any_other`, the one fixture whose
document declares its transition table and whose trees every lane already builds, with an observer set before the
run. A run that reaches `pass` takes each of its six transitions once, in document order: the observer is told
exactly `(s1, 0)` to `(s6, 0)` in that order, every pair told is a row of the table, and every row of the table is
told. A second test sets an observer, clears it and runs: it is told nothing, and the run still reaches `pass`.

| Lane | Test |
|---|---|
| Rust | `backends/rust/tests/tests/a_computed_event_name_is_matched_like_any_other.rs`; `backends/rust/runtime/tests/hello_world.rs` (a hand-written table whose state holds two transitions, so the second is told as index 1, not its number 2, and `position_in_source` without the feature) |
| Go | `backends/go/tests/integration/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other_test.go` |
| Kotlin | `backends/kotlin/tests/src/test/kotlin/com/sce/integration/AComputedEventNameIsMatchedLikeAnyOtherTest.kt` |
| Swift | `backends/swift/tests/Tests/IntegrationTests/AComputedEventNameIsMatchedLikeAnyOtherTests.swift` |
| C++ | `tests/integration/AComputedEventNameIsMatchedLikeAnyOtherAotTest.cpp` |
| C11 | `backends/c/tests/integration/test_transition_observer.c` (target `c11_integration_transition_observer`, built with `SCE_TRANSITION_OBSERVER=1`); the target of the same machine without the switch defines no hook, and links only while the default is off |
| Python | `backends/python/tests/integration/a_computed_event_name_is_matched_like_any_other/test_a_computed_event_name_is_matched_like_any_other_aot.py`; `backends/python/tests/integration/discarded_event_is_observable/test_engine_observer_aot.py` (index 2 and 1 for the third and second transitions of one state) |

`sce-build/tests/a_c11_machine_keeps_no_transition_observer_unless_its_build_asks.rs` generates a C11 machine and
holds every line of its header and source that names the hook to the region the machine's switch keeps, so a build
that does not ask for the observer is left with nothing of it.

## Consequences

- Every lane that runs the reference harness can measure its transition coverage as the rows of its emitted table
  the exhaustive run took, and ADR 0087's step 7 requires 100% of them on each lane by measurement.
- The Rust runtime has a fifth feature, off by default; a crate that measures coverage names it, as
  `backends/rust/tests` does.
- A C11 build that observes a machine defines one function per observed machine.
- The Python `"transition"` record has one more field; a consumer that compares the whole record (the observer's
  own test did) sees it.
- The Kotlin `transitions` flow is unchanged and remains a log of moves; coverage does not read it.

## Alternatives rejected

- **Predicting the paths per lane.** Computing, from the document, which transitions a sequence of outcome classes
  must take, and asserting that each lane reached the predicted configurations. It is a second implementation of
  the selection algorithm in the test, and it measures the document rather than the engine: a lane whose selection
  differed from the prediction on a transition no configuration distinguishes (two transitions of one state to one
  target) would pass. Coverage is a fact about the run, and only the engine that ran can state it.
- **Parsing logs.** Each engine logs some of its transitions at a debug level, in its own words, and a log line
  names a transition by its source and event, not by its index; two transitions of one state on one event are one
  line. A log is also absent where cost matters (a bare-metal image installs no logger, and link-time optimisation
  drops the calls), and a test that reads one depends on wording that no contract holds.
- **The Kotlin `transitions` flow, extended to the other lanes.** It is asynchronous, so a test cannot tell "not
  taken" from "not yet collected", it omits eventless and targetless transitions, and its record names no row.
- **A pointer in the C11 machine's struct, checked for null** (section 3).
- **A const generic or a function pointer in the Rust engine** (section 3).
- **One observer for every kind the Python engine reports, on every lane.** Coverage needs one kind; the others are
  a larger surface with no consumer that needs it on every lane yet.
