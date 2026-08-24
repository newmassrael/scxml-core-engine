// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//   2. process_next_external_event: pop Foo → S0→S1 (hierarchical exit/entry at root)
//   3. run_main_event_loop continues: pop Bar → S1→Pass
//   4. Pass is a top-level final state → is_in_final_state() == true

use sce_rust_runtime::{Engine, EventWithMetadata, StatePolicy};

// `Fail` mirrors the W3C SCXML enum shape (test144 has both pass and fail
// terminal states) but the hand-crafted happy-path port never constructs
// it; matches!() arms still need the variant for exhaustiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum Test144State {
    Fail,
    Pass,
    S0,
    S1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Test144Event {
    /// W3C SCXML 6.2: default/scheduler-poll event (no semantic meaning).
    None,
    Bar,
    Foo,
}

struct Test144Policy {
    last_transition_is_internal: bool,
    last_transition_is_targetless: bool,
    last_transition_source_state: Test144State,
}

impl Test144Policy {
    fn new() -> Self {
        Self {
            last_transition_is_internal: false,
            last_transition_is_targetless: false,
            last_transition_source_state: Test144State::S0,
        }
    }
}

impl StatePolicy for Test144Policy {
    type State = Test144State;
    type Event = Test144Event;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;

    // C++ `static constexpr bool HAS_PARALLEL_STATES = false;`
    const HAS_PARALLEL_STATES: bool = false;
    // C++ `static constexpr bool NEEDS_SCRIPT_ENGINE = false;`
    const NEEDS_SCRIPT_ENGINE: bool = false;

    fn initial_state() -> Self::State {
        // C++ `return State::S0;`
        Test144State::S0
    }

    fn is_final_state(state: Self::State) -> bool {
        // C++ switch: Pass, Fail → true
        matches!(state, Test144State::Pass | Test144State::Fail)
    }

    fn get_parent(_state: Self::State) -> Option<Self::State> {
        // C++ `return std::nullopt;` for all (flat hierarchy)
        None
    }

    fn is_compound_state(_state: Self::State) -> bool {
        // C++ `return false;` for all
        false
    }

    fn is_descendant_of(_desc: Self::State, _anc: Self::State) -> bool {
        false
    }

    fn get_document_order(state: Self::State) -> u32 {
        match state {
            Test144State::Fail => 0,
            Test144State::Pass => 1,
            Test144State::S0 => 2,
            Test144State::S1 => 3,
        }
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
            Test144State::Fail => "fail",
            Test144State::Pass => "pass",
            Test144State::S0 => "s0",
            Test144State::S1 => "s1",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "fail" => Some(Test144State::Fail),
            "pass" => Some(Test144State::Pass),
            "s0" => Some(Test144State::S0),
            "s1" => Some(Test144State::S1),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        Test144Event::None
    }

    // Required field accessors
    fn last_transition_is_internal(&self) -> bool {
        self.last_transition_is_internal
    }
    fn set_last_transition_is_internal(&mut self, v: bool) {
        self.last_transition_is_internal = v;
    }
    fn last_transition_is_targetless(&self) -> bool {
        self.last_transition_is_targetless
    }
    fn set_last_transition_is_targetless(&mut self, v: bool) {
        self.last_transition_is_targetless = v;
    }
    fn last_transition_source_state(&self) -> Self::State {
        self.last_transition_source_state
    }
    fn set_last_transition_source_state(&mut self, s: Self::State) {
        self.last_transition_source_state = s;
    }

    // ──────────────────────────────────────────────
    // Entry actions (1:1 port of C++ executeEntryActions)
    // ──────────────────────────────────────────────

    // `_path_child` (§scxml-D) is unused: this hand-written mirror of test144
    // has no compound state whose default child could be at stake.
    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        _path_child: Option<Self::State>,
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

    fn execute_exit_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _pre: &[Self::State],
    ) {
        // C++ executeExitActions: empty body
    }

    // ──────────────────────────────────────────────
    // Transition logic (1:1 port of C++ processTransition + tryTransitionInState)
    // ──────────────────────────────────────────────

    fn process_transition(
        &mut self,
        current_state: &mut Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> bool {
        // Record transition source before any state change
        self.last_transition_source_state = *current_state;

        match (*current_state, event) {
            // S0 on Foo → S1
            (Test144State::S0, Test144Event::Foo) => {
                self.last_transition_is_internal = false;
                self.last_transition_is_targetless = false;
                *current_state = Test144State::S1;
                true
            }
            // S1 on Bar → Pass
            (Test144State::S1, Test144Event::Bar) => {
                self.last_transition_is_internal = false;
                self.last_transition_is_targetless = false;
                *current_state = Test144State::Pass;
                true
            }
            // Wildcard '*' transitions to Fail are omitted: the happy path
            // (Foo → Bar → Pass) never exercises them, and W3C SCXML 5.9.3
            // descriptor matching has its own coverage (`helpers::event_matching`).
            // The full generated output emits `matchesEventDescriptor(name, "*")`
            // arms here.
            _ => false,
        }
    }

    fn execute_transition_actions(&mut self, _engine: &mut Engine<Self>) {
        // C++ executeTransitionActions: empty body
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

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
        engine.get_current_state(),
        Test144State::Pass,
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
