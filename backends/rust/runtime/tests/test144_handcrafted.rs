// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Hand-translation of W3C SCXML test144, pinning the codegen-facing trait shape.
//
// Source of truth: build/tests/w3c_static_generated/test144_sm.h + test144_sm.inl
// (C++ output of `sce-codegen generate resources/144/test144.txml -o . -l cpp`).
//
// This test proves that `StatePolicy` + `Engine<P>` can represent real
// generated state machine output one-to-one. If this file doesn't compile or
// fails, the trait shape has regressed for generated code and must be revised.
//
// Test144 topology (pure static, no script engine, no hierarchy):
//     S0 (initial)
//       onentry: raise(Foo); raise(Bar)
//       on Foo  → S1
//       on *    → Fail   (wildcard; omitted in this hand port, unused in happy path)
//     S1
//       on Bar  → Pass  (final)
//       on *    → Fail
//     Pass (final)
//     Fail (final)
//
// Execution trace:
//   1. initialize() enters S0, runs onentry, internal queue = [Foo, Bar]
//   2. the macrostep takes Foo off the internal queue → microstep S0→S1
//   3. and then Bar → microstep S1→Pass
//   4. Pass is a top-level final state → is_in_final_state() == true

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EventId, EventWithMetadata, HistoryId,
    NoHistory, RunStore, Span, StateId, StateKind, StatePolicy, StateRow, Target, TransitionId,
    TransitionRow, Workspace,
};

// Declared in document order, so a variant's discriminant is its `StateId`:
// the generated enum's order (docs/adr/0064).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Test144State {
    S0,
    S1,
    Pass,
    Fail,
}

// Declared in the order of `EVENTS`, so a variant's discriminant is its
// `EventId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Test144Event {
    /// W3C SCXML 6.2: default/scheduler-poll event (no semantic meaning).
    None,
    Bar,
    Foo,
}

struct Test144Policy;

impl Test144Policy {
    fn new() -> Self {
        Self
    }
}

/// The written transitions, by `TransitionId`: grouped by source in state
/// order, as the tables number them.
const S0_ON_FOO: TransitionId = TransitionId(0);
const S1_ON_BAR: TransitionId = TransitionId(1);

impl StatePolicy for Test144Policy {
    type State = Test144State;
    type Event = Test144Event;
    // C++ `using History = NoHistory;` — test144 declares no <history>.
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // Four flat states and no history: the sets of states are bit vectors of
    // four states, one transition is enabled, and the walks owe three steps
    // for the one level and three of headroom.
    type Workspace = Workspace<{ state_scratch(4) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(4) }, 0, 0>;
    type Configuration = BoundedVec<Test144State, 1>;

    // C++ `getParent` answers `std::nullopt` for all: four children of the
    // `<scxml>` element, two of them `<final>`. The document's initial
    // transition, last, is `<scxml initial="s0">`.
    const TABLES: &'static DocumentTables = &DocumentTables {
        states: &[
            StateRow::new(None, StateKind::Atomic, StateId(0), None),
            StateRow::new(None, StateKind::Atomic, StateId(1), None),
            StateRow::new(None, StateKind::Final, StateId(2), None),
            StateRow::new(None, StateKind::Final, StateId(3), None),
        ],
        histories: &[],
        transitions: &[
            // s0: foo -> s1
            TransitionRow::new(Some(StateId(0)), Span { start: 0, len: 1 }, false, false),
            // s1: bar -> pass
            TransitionRow::new(Some(StateId(1)), Span { start: 1, len: 1 }, false, false),
            // <scxml initial="s0">
            TransitionRow::new(None, Span { start: 2, len: 1 }, false, false),
        ],
        targets: &[
            Target::state(StateId(1)),
            Target::state(StateId(2)),
            Target::state(StateId(0)),
        ],
        initial: TransitionId(2),
        history: None,
        parallel: None,
    };
    const STATES: &'static [Test144State] = &[
        Test144State::S0,
        Test144State::S1,
        Test144State::Pass,
        Test144State::Fail,
    ];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [Test144Event] =
        &[Test144Event::None, Test144Event::Bar, Test144Event::Foo];

    // C++ `static constexpr bool HAS_PARALLEL_STATES = false;`
    const HAS_PARALLEL_STATES: bool = false;
    // C++ `static constexpr bool NEEDS_SCRIPT_ENGINE = false;`
    const NEEDS_SCRIPT_ENGINE: bool = false;

    fn state_id(state: Self::State) -> StateId {
        StateId(state as u16)
    }

    fn history_id(history: Self::History) -> HistoryId {
        match history {}
    }

    fn event_id(event: Self::Event) -> EventId {
        EventId(event as u16)
    }

    fn initial_state() -> Self::State {
        // C++ `return State::S0;`
        Test144State::S0
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        // C++ `getEventName(Event)` 1:1
        match event {
            Test144Event::None => "",
            Test144Event::Bar => "bar",
            Test144Event::Foo => "foo",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        // C++ `getEventFromName(string)` 1:1
        match name {
            "bar" => Some(Test144Event::Bar),
            "foo" => Some(Test144Event::Foo),
            _ => None,
        }
    }

    fn get_state_name(state: Self::State) -> &'static str {
        // C++ `getStateName(State)` 1:1
        match state {
            Test144State::S0 => "s0",
            Test144State::S1 => "s1",
            Test144State::Pass => "pass",
            Test144State::Fail => "fail",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "s0" => Some(Test144State::S0),
            "s1" => Some(Test144State::S1),
            "pass" => Some(Test144State::Pass),
            "fail" => Some(Test144State::Fail),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        Test144Event::None
    }

    // ──────────────────────────────────────────────
    // Entry actions (1:1 port of C++ executeEntryActions)
    // ──────────────────────────────────────────────

    // `_is_default_entry` is unused: this hand-written mirror of test144 has
    // no compound state whose initial content could run.
    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
        match state {
            Test144State::S0 => {
                // C++ onentry: raise(Foo); raise(Bar)
                engine.raise(EventWithMetadata::new(Test144Event::Foo));
                engine.raise(EventWithMetadata::new(Test144Event::Bar));
            }
            Test144State::S1 | Test144State::Pass | Test144State::Fail => {}
        }
    }

    fn execute_exit_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {
        // C++ executeExitActions: empty body
    }

    // ──────────────────────────────────────────────
    // Transition logic (1:1 port of C++ firstEnabledTransition)
    // ──────────────────────────────────────────────

    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<TransitionId> {
        match (state, event) {
            // S0 on Foo → S1
            (Test144State::S0, Test144Event::Foo) => Some(S0_ON_FOO),
            // S1 on Bar → Pass
            (Test144State::S1, Test144Event::Bar) => Some(S1_ON_BAR),
            // Wildcard '*' transitions to Fail are omitted: the happy path
            // (Foo → Bar → Pass) never exercises them, and W3C SCXML 5.9.3
            // descriptor matching has its own coverage (`helpers::event_matching`).
            // The full generated output emits `matches_event_descriptor(name, "*")`
            // arms here.
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        _transition: TransitionId,
        _engine: &mut Engine<Self>,
    ) {
        // C++ executeTransitionActions: empty body
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[test]
fn test144_tables_keep_the_rules_the_procedures_assume() {
    assert_eq!(Test144Policy::TABLES.check(), Ok(()));
}

#[test]
fn test144_initial_state_is_s0() {
    let engine = Engine::<Test144Policy>::new(Test144Policy::new());
    assert_eq!(engine.get_current_state(), Test144State::S0);
}

#[test]
fn test144_initialize_drains_internal_queue_and_reaches_pass() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut engine = Engine::<Test144Policy>::new(Test144Policy::new());
    engine.initialize();

    // After initialize():
    //   - enter S0 → raise Foo, Bar into internal queue
    //   - macrostep loop drains internal queue:
    //     - pop Foo → S0 → S1 transition (hierarchical exit + entry at root level)
    //     - pop Bar → S1 → Pass transition
    //   - Pass is a final state; engine stops processing
    assert_eq!(
        engine.terminal_state(),
        Some(Test144State::Pass),
        "W3C test144: happy path Foo→Bar should land at Pass"
    );
    assert!(engine.is_in_final_state());
}

#[test]
fn test144_trait_shape_compiles_against_runtime() {
    // This test's existence proves: a generated-style StatePolicy impl compiles
    // against Engine<P> without Arc<RefCell<_>>, unsafe in generated code, or
    // any other workaround. If this file ever fails to build, generated
    // machines face the same compile errors — revise the trait first.
    let policy = Test144Policy::new();
    let engine = Engine::<Test144Policy>::new(policy);
    drop(engine); // ensure no borrow-checker issue around move
}
