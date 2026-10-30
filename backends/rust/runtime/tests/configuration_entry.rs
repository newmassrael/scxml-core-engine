// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-3.3 / §scxml-3.4 / §scxml-3.8: `Engine::enter_at` — entering a
// configuration a machine was already in, without re-running `<onentry>`.
//
// Two hand-written policies, because the two shapes of a configuration are
// different facts:
//
//   - `ParallelPolicy` is a `<parallel>` document, the shape the consumer that
//     asked for this door actually runs. Its configuration holds every region,
//     and it is the case where `current_state` is NOT recoverable from that
//     set.
//   - `LinearPolicy` has no parallel regions, so its configuration is one
//     chain from the `<scxml>` element's child down to the leaf, and the leaf
//     is the current state.
//
// Neither keeps its own configuration: the engine keeps it, as a set of state
// numbers, and `Engine::get_active_states` reads it back in document order
// (docs/adr/0064). `enter_at` has to close the round trip for both.
//
// The onentry witness is what separates a resume from a replay. Both policies
// raise an event from an `<onentry>` that has a transition waiting for it, so a
// re-entry would MOVE the machine. Asserting the machine stays put after
// `enter_at` + `step()` is the observable form of "entry actions did not run";
// the `entries` counter is the same claim read from the inside.

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, ConfigurationRejection, DocumentTables, Engine, EventId,
    EventWithMetadata, HistoryId, NoHistory, RunStore, Span, StateId, StateKind, StatePolicy,
    StateRow, Target, TransitionId, TransitionRow, Workspace, PARALLEL,
};

// ══════════════════════════════════════════════════════════════════
// A <parallel> document
//
//   P (parallel, root)
//     RA (compound)   A1 (atomic, onentry raise Advance) | A2 (atomic)
//     RB (compound)   B1 (atomic)
//
//   A1 --Advance--> A2
// ══════════════════════════════════════════════════════════════════

// Declared in document order, so a variant's discriminant is its `StateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PState {
    P,
    Ra,
    A1,
    A2,
    Rb,
    B1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PEvent {
    None,
    Advance,
}

struct ParallelPolicy {
    /// How many states this policy has run entry actions for. The inside view
    /// of the onentry contract.
    entries: u32,
}

impl ParallelPolicy {
    fn new() -> Self {
        Self { entries: 0 }
    }
}

/// `a1`'s one transition, the first row of the transitions table.
const A1_ON_ADVANCE: TransitionId = TransitionId(0);

impl StatePolicy for ParallelPolicy {
    type State = PState;
    type Event = PEvent;
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // Six states and no history, so the sets of states are bit vectors of six.
    // P, RA, one of A1/A2, RB, B1 are active at once, two of them atomic, so
    // two transitions enabled at most; three steps for each of the three
    // levels and three of headroom for the walks.
    type Workspace = Workspace<{ state_scratch(6) }, 2, 12, 0>;
    type RunStore = RunStore<{ state_words(6) }, 0, 0>;
    type Configuration = BoundedVec<PState, 5>;

    // The transitions: `a1 --advance--> a2`, then the initial transitions of
    // the compound states `ra` and `rb`, then the document's.
    const TABLES: &'static DocumentTables = &DocumentTables {
        states: &[
            // p
            StateRow::new(None, StateKind::Parallel, StateId(5), None),
            // ra
            StateRow::new(
                Some(StateId(0)),
                StateKind::Compound,
                StateId(3),
                Some(TransitionId(1)),
            ),
            // a1
            StateRow::new(Some(StateId(1)), StateKind::Atomic, StateId(2), None),
            // a2
            StateRow::new(Some(StateId(1)), StateKind::Atomic, StateId(3), None),
            // rb
            StateRow::new(
                Some(StateId(0)),
                StateKind::Compound,
                StateId(5),
                Some(TransitionId(2)),
            ),
            // b1
            StateRow::new(Some(StateId(4)), StateKind::Atomic, StateId(5), None),
        ],
        histories: &[],
        transitions: &[
            // a1: advance -> a2
            TransitionRow::new(Some(StateId(2)), Span { start: 0, len: 1 }, false, false),
            // ra's initial: a1
            TransitionRow::new(Some(StateId(1)), Span { start: 1, len: 1 }, false, false),
            // rb's initial: b1
            TransitionRow::new(Some(StateId(4)), Span { start: 2, len: 1 }, false, false),
            // the document's initial: p
            TransitionRow::new(None, Span { start: 3, len: 1 }, false, false),
        ],
        targets: &[
            Target::state(StateId(3)),
            Target::state(StateId(2)),
            Target::state(StateId(5)),
            Target::state(StateId(0)),
        ],
        initial: TransitionId(3),
        history: None,
        parallel: Some(&PARALLEL),
    };
    const STATES: &'static [PState] = &[
        PState::P,
        PState::Ra,
        PState::A1,
        PState::A2,
        PState::Rb,
        PState::B1,
    ];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [PEvent] = &[PEvent::None, PEvent::Advance];

    const HAS_PARALLEL_STATES: bool = true;
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
        PState::P
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        match event {
            PEvent::None => "",
            PEvent::Advance => "advance",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "advance" => Some(PEvent::Advance),
            _ => None,
        }
    }

    fn get_state_name(state: Self::State) -> &'static str {
        match state {
            PState::P => "p",
            PState::Ra => "ra",
            PState::A1 => "a1",
            PState::A2 => "a2",
            PState::Rb => "rb",
            PState::B1 => "b1",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "p" => Some(PState::P),
            "ra" => Some(PState::Ra),
            "a1" => Some(PState::A1),
            "a2" => Some(PState::A2),
            "rb" => Some(PState::Rb),
            "b1" => Some(PState::B1),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        PEvent::None
    }

    // Mirrors the generated shape: one state's onentry, and nothing below it —
    // the engine's Appendix D entry set enters every region and initial child,
    // one state at a time, and keeps the configuration itself.
    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
        self.entries += 1;
        // The onentry that separates a resume from a replay.
        if state == PState::A1 {
            engine.raise(EventWithMetadata::new(PEvent::Advance));
        }
    }

    fn execute_exit_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {}

    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<TransitionId> {
        match (state, event) {
            (PState::A1, PEvent::Advance) => Some(A1_ON_ADVANCE),
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        _transition: TransitionId,
        _engine: &mut Engine<Self>,
    ) {
    }
}

// ══════════════════════════════════════════════════════════════════
// A document with no parallel regions
//
//   Root (compound)   X1 (atomic, onentry raise Go) | X2 (atomic)
//   X1 --Go--> X2
// ══════════════════════════════════════════════════════════════════

// Declared in document order, so a variant's discriminant is its `StateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LState {
    Root,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LEvent {
    None,
    Go,
}

struct LinearPolicy {
    entries: u32,
}

impl LinearPolicy {
    fn new() -> Self {
        Self { entries: 0 }
    }
}

/// `x1`'s one transition, the first row of the transitions table.
const X1_ON_GO: TransitionId = TransitionId(0);

impl StatePolicy for LinearPolicy {
    type State = LState;
    type Event = LEvent;
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // Three states and no history, so the sets of states are bit vectors of
    // three. Root with one of X1/X2 below it: one atomic state active; three
    // steps for each of the two levels and three of headroom for the walks.
    type Workspace = Workspace<{ state_scratch(3) }, 1, 9, 0>;
    type RunStore = RunStore<{ state_words(3) }, 0, 0>;
    type Configuration = BoundedVec<LState, 2>;

    // The transitions: `x1 --go--> x2`, then `root`'s initial transition, then
    // the document's.
    const TABLES: &'static DocumentTables = &DocumentTables {
        states: &[
            // root
            StateRow::new(None, StateKind::Compound, StateId(2), Some(TransitionId(1))),
            // x1
            StateRow::new(Some(StateId(0)), StateKind::Atomic, StateId(1), None),
            // x2
            StateRow::new(Some(StateId(0)), StateKind::Atomic, StateId(2), None),
        ],
        histories: &[],
        transitions: &[
            // x1: go -> x2
            TransitionRow::new(Some(StateId(1)), Span { start: 0, len: 1 }, false, false),
            // root's initial: x1
            TransitionRow::new(Some(StateId(0)), Span { start: 1, len: 1 }, false, false),
            // the document's initial: root
            TransitionRow::new(None, Span { start: 2, len: 1 }, false, false),
        ],
        targets: &[
            Target::state(StateId(2)),
            Target::state(StateId(1)),
            Target::state(StateId(0)),
        ],
        initial: TransitionId(2),
        history: None,
        parallel: None,
    };
    const STATES: &'static [LState] = &[LState::Root, LState::X1, LState::X2];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [LEvent] = &[LEvent::None, LEvent::Go];

    const HAS_PARALLEL_STATES: bool = false;
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
        LState::Root
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        match event {
            LEvent::None => "",
            LEvent::Go => "go",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "go" => Some(LEvent::Go),
            _ => None,
        }
    }

    fn get_state_name(state: Self::State) -> &'static str {
        match state {
            LState::Root => "root",
            LState::X1 => "x1",
            LState::X2 => "x2",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "root" => Some(LState::Root),
            "x1" => Some(LState::X1),
            "x2" => Some(LState::X2),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        LEvent::None
    }

    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
        self.entries += 1;
        if let LState::X1 = state {
            engine.raise(EventWithMetadata::new(LEvent::Go));
        }
    }

    fn execute_exit_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {}

    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<TransitionId> {
        match (state, event) {
            (LState::X1, LEvent::Go) => Some(X1_ON_GO),
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        _transition: TransitionId,
        _engine: &mut Engine<Self>,
    ) {
    }
}

#[test]
fn the_hand_written_tables_keep_the_rules_the_procedures_assume() {
    assert_eq!(ParallelPolicy::TABLES.check(), Ok(()));
    assert_eq!(LinearPolicy::TABLES.check(), Ok(()));
}

// ══════════════════════════════════════════════════════════════════
// Acceptance 1: the round trip closes, through both code paths
// ══════════════════════════════════════════════════════════════════

#[test]
fn parallel_configuration_round_trips_through_a_new_engine() {
    let mut ran = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    ran.initialize();

    // What a host would persist: both readers.
    let saved_configuration = ran.get_active_states();
    let saved_current = ran.get_current_state();

    // The onentry fired and the macrostep consumed it, so the run really did
    // move — the configuration below is one this document reached, not its
    // initial one.
    assert_eq!(saved_current, PState::A2);
    assert!(saved_configuration.contains(&PState::A2));

    let mut restored = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    restored
        .enter_at(&saved_configuration, saved_current)
        .expect("the configuration a run published is a configuration it can be put back into");

    assert_eq!(restored.get_active_states(), saved_configuration);
    assert_eq!(restored.get_current_state(), saved_current);
    assert!(restored.is_running());
}

#[test]
fn linear_configuration_round_trips_without_a_parallel_region() {
    let mut ran = Engine::<LinearPolicy>::new(LinearPolicy::new());
    ran.initialize();

    let saved_configuration = ran.get_active_states();
    let saved_current = ran.get_current_state();
    assert_eq!(saved_current, LState::X2);
    // No parallel regions: the chain from the `<scxml>` element's child down to
    // the leaf. The configuration is a set, reported in document order.
    assert_eq!(saved_configuration, [LState::Root, LState::X2]);

    let mut restored = Engine::<LinearPolicy>::new(LinearPolicy::new());
    restored
        .enter_at(&saved_configuration, saved_current)
        .expect("a chain from the root down is a configuration");

    assert_eq!(restored.get_active_states(), saved_configuration);
    assert_eq!(restored.get_current_state(), saved_current);
}

// ══════════════════════════════════════════════════════════════════
// Acceptance 2: no `<onentry>` — the difference between resume and replay
// ══════════════════════════════════════════════════════════════════

#[test]
fn entering_a_configuration_runs_no_entry_actions() {
    // The configuration this document is in right after entry, before the
    // `<onentry>`-raised event has been taken: A1 and B1 active.
    let configuration = [PState::P, PState::Ra, PState::A1, PState::Rb, PState::B1];

    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    engine
        .enter_at(&configuration, PState::A1)
        .expect("this is the configuration the document enters");

    // Inside view: not one entry action ran.
    assert_eq!(
        engine.policy().entries,
        0,
        "enter_at must run no entry actions"
    );

    // Outside view, and the one that matters to a host: A1's `<onentry>` raises
    // Advance, and A1 has a transition on Advance. Had the entry actions run,
    // this step would have taken the machine to A2. It is still at A1, so
    // nothing was queued — the resume did not replay.
    engine.step();
    assert_eq!(
        engine.get_current_state(),
        PState::A1,
        "a replayed <onentry> would have raised Advance and moved this to A2"
    );
    assert_eq!(engine.get_active_states(), configuration);
}

#[test]
fn initialize_by_contrast_does_run_them() {
    // The control: the same document through the other door moves, because the
    // entry actions are exactly what `initialize` is for. Without this the test
    // above could pass on a machine that simply cannot move.
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    engine.initialize();
    assert!(engine.policy().entries > 0);
    assert_eq!(engine.get_current_state(), PState::A2);
}

// ══════════════════════════════════════════════════════════════════
// Acceptance 3: what is refused
// ══════════════════════════════════════════════════════════════════

#[test]
fn an_empty_chain_is_refused() {
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let empty: [PState; 0] = [];
    assert_eq!(
        engine.enter_at(&empty, PState::A1),
        Err(ConfigurationRejection::Empty)
    );
}

#[test]
fn two_siblings_of_one_region_are_refused() {
    // §scxml-3.3: RA is compound, so exactly one of A1/A2 can be active.
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = [
        PState::P,
        PState::Ra,
        PState::A1,
        PState::A2,
        PState::Rb,
        PState::B1,
    ];
    assert_eq!(
        engine.enter_at(&chain, PState::A1),
        Err(ConfigurationRejection::CompoundChildCount {
            parent: PState::Ra,
            found: 2
        })
    );
}

#[test]
fn a_parallel_with_a_region_missing_is_refused() {
    // §scxml-3.4: RB is a region of P, so a configuration holding P holds it.
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = [PState::P, PState::Ra, PState::A1];
    assert_eq!(
        engine.enter_at(&chain, PState::A1),
        Err(ConfigurationRejection::ParallelRegionMissing {
            parallel: PState::P,
            region: PState::Rb
        })
    );
}

#[test]
fn a_chain_that_skips_an_ancestor_is_refused() {
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = [PState::A1];
    assert_eq!(
        engine.enter_at(&chain, PState::A1),
        Err(ConfigurationRejection::AncestorMissing {
            state: PState::A1,
            parent: PState::Ra
        })
    );
}

#[test]
fn a_repeated_state_is_refused() {
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = [
        PState::P,
        PState::Ra,
        PState::A1,
        PState::A1,
        PState::Rb,
        PState::B1,
    ];
    assert_eq!(
        engine.enter_at(&chain, PState::A1),
        Err(ConfigurationRejection::Duplicate { state: PState::A1 })
    );
}

#[test]
fn a_current_state_outside_the_configuration_is_refused() {
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = [PState::P, PState::Ra, PState::A1, PState::Rb, PState::B1];
    assert_eq!(
        engine.enter_at(&chain, PState::A2),
        Err(ConfigurationRejection::CurrentNotActive {
            current: PState::A2
        })
    );
}

#[test]
fn a_non_atomic_current_state_is_refused() {
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = [PState::P, PState::Ra, PState::A1, PState::Rb, PState::B1];
    assert_eq!(
        engine.enter_at(&chain, PState::Ra),
        Err(ConfigurationRejection::CurrentNotAtomic {
            current: PState::Ra
        })
    );
}

#[test]
fn a_refused_entry_leaves_the_engine_untouched() {
    // The property that makes a rejection safe to handle: the host can try a
    // stale record, be told no, and still hold an engine it can initialize.
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let before = engine.get_current_state();
    let chain = [PState::A1];
    assert!(engine.enter_at(&chain, PState::A1).is_err());
    assert_eq!(engine.get_current_state(), before);
    assert!(!engine.is_running());
    assert_eq!(engine.policy().entries, 0);

    engine.initialize();
    assert_eq!(engine.get_current_state(), PState::A2);
}
