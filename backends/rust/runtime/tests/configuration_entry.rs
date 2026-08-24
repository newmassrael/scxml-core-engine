// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-3.3 / §scxml-3.4 / §scxml-3.8: `Engine::enter_at` — entering a
// configuration a machine was already in, without re-running `<onentry>`.
//
// Two hand-written policies, because the two halves of the door are different
// facts:
//
//   - `ParallelPolicy` keeps its own active set (`HAS_ACTIVE_STATES`), which is
//     the shape the consumer that asked for this door actually runs: a
//     `<parallel>` document. Its configuration is a set, and it is the case
//     where `current_state` is NOT recoverable from that set.
//   - `LinearPolicy` has no parallel regions, so its configuration is the
//     parent walk from the leaf and `Engine::get_active_states` derives it
//     rather than reading the policy. `enter_at` has to close the round trip
//     there too, through a different code path.
//
// The onentry witness is what separates a resume from a replay. Both policies
// raise an event from an `<onentry>` that has a transition waiting for it, so a
// re-entry would MOVE the machine. Asserting the machine stays put after
// `enter_at` + `step()` is the observable form of "entry actions did not run";
// the `entries` counter is the same claim read from the inside.

use sce_rust_runtime::helpers::hierarchy::{push_chain, state_chain_from_slice, StateChain};
use sce_rust_runtime::{ConfigurationRejection, Engine, EventWithMetadata, StatePolicy};

// ══════════════════════════════════════════════════════════════════
// A <parallel> document
//
//   P (parallel, root)
//     RA (compound)   A1 (atomic, onentry raise Advance) | A2 (atomic)
//     RB (compound)   B1 (atomic)
//
//   A1 --Advance--> A2
// ══════════════════════════════════════════════════════════════════

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
    last_internal: bool,
    last_targetless: bool,
    last_source: PState,
    active_states: StateChain<PState>,
    /// How many states this policy has run entry actions for. The inside view
    /// of the onentry contract.
    entries: u32,
}

impl ParallelPolicy {
    fn new() -> Self {
        Self {
            last_internal: false,
            last_targetless: false,
            last_source: PState::P,
            active_states: sce_rust_runtime::helpers::hierarchy::new_chain(),
            entries: 0,
        }
    }
}

impl StatePolicy for ParallelPolicy {
    type State = PState;
    type Event = PEvent;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;

    const HAS_PARALLEL_STATES: bool = true;
    const HAS_ACTIVE_STATES: bool = true;
    const NEEDS_SCRIPT_ENGINE: bool = false;

    fn initial_state() -> Self::State {
        PState::P
    }

    fn is_final_state(_state: Self::State) -> bool {
        false
    }

    fn get_parent(state: Self::State) -> Option<Self::State> {
        match state {
            PState::P => None,
            PState::Ra | PState::Rb => Some(PState::P),
            PState::A1 | PState::A2 => Some(PState::Ra),
            PState::B1 => Some(PState::Rb),
        }
    }

    fn is_compound_state(state: Self::State) -> bool {
        matches!(state, PState::P | PState::Ra | PState::Rb)
    }

    fn is_parallel_state(state: Self::State) -> bool {
        matches!(state, PState::P)
    }

    fn get_parallel_regions(state: Self::State) -> &'static [Self::State] {
        match state {
            PState::P => &[PState::Ra, PState::Rb],
            _ => &[],
        }
    }

    fn is_descendant_of(desc: Self::State, anc: Self::State) -> bool {
        let mut cur = Self::get_parent(desc);
        while let Some(s) = cur {
            if s == anc {
                return true;
            }
            cur = Self::get_parent(s);
        }
        false
    }

    fn get_document_order(state: Self::State) -> u32 {
        match state {
            PState::P => 0,
            PState::Ra => 1,
            PState::A1 => 2,
            PState::A2 => 3,
            PState::Rb => 4,
            PState::B1 => 5,
        }
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

    fn get_initial_or_history_child(&self, state: Self::State) -> Self::State {
        match state {
            PState::P => PState::Ra,
            PState::Ra => PState::A1,
            PState::Rb => PState::B1,
            other => other,
        }
    }

    fn last_transition_is_internal(&self) -> bool {
        self.last_internal
    }
    fn set_last_transition_is_internal(&mut self, v: bool) {
        self.last_internal = v;
    }
    fn last_transition_is_targetless(&self) -> bool {
        self.last_targetless
    }
    fn set_last_transition_is_targetless(&mut self, v: bool) {
        self.last_targetless = v;
    }
    fn last_transition_source_state(&self) -> Self::State {
        self.last_source
    }
    fn set_last_transition_source_state(&mut self, s: Self::State) {
        self.last_source = s;
    }

    // Mirrors the generated parallel shape: the entered state joins the active
    // set, and a parallel/compound state recurses into its regions or initial
    // child (which is why `Engine::resolve_current_state_to_leaf` does not
    // enter anything for a parallel machine).
    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        _path_child: Option<Self::State>,
    ) {
        self.entries += 1;
        if !self.active_states.contains(&state) {
            push_chain(&mut self.active_states, state);
        }
        match state {
            PState::P => {
                self.execute_entry_actions(PState::Ra, engine, None);
                self.execute_entry_actions(PState::Rb, engine, None);
            }
            PState::Ra => self.execute_entry_actions(PState::A1, engine, None),
            PState::Rb => self.execute_entry_actions(PState::B1, engine, None),
            // The onentry that separates a resume from a replay.
            PState::A1 => engine.raise(EventWithMetadata::new(PEvent::Advance)),
            PState::A2 | PState::B1 => {}
        }
    }

    fn execute_exit_actions(
        &mut self,
        state: Self::State,
        _engine: &mut Engine<Self>,
        _pre: &[Self::State],
    ) {
        self.active_states.retain(|&s| s != state);
    }

    fn process_transition(
        &mut self,
        current_state: &mut Self::State,
        event: Self::Event,
        engine: &mut Engine<Self>,
    ) -> bool {
        self.last_source = *current_state;
        match (*current_state, event) {
            (PState::A1, PEvent::Advance) => {
                self.last_internal = false;
                self.last_targetless = false;
                // A parallel machine owns its own microstep. `Engine` says so
                // where it decides not to call `handle_hierarchical_transition`
                // for `HAS_PARALLEL_STATES`: doing so would double-run the
                // exit/entry blocks the policy already ran. So the region-local
                // exit and entry happen here, as the generated policy does them.
                self.execute_exit_actions(PState::A1, engine, &[]);
                self.execute_entry_actions(PState::A2, engine, None);
                *current_state = PState::A2;
                true
            }
            _ => false,
        }
    }

    fn execute_transition_actions(&mut self, _engine: &mut Engine<Self>) {}

    fn get_active_states(&self) -> StateChain<Self::State> {
        self.active_states.clone()
    }

    fn set_active_states(&mut self, states: StateChain<Self::State>) {
        self.active_states = states;
    }
}

// ══════════════════════════════════════════════════════════════════
// A document with no parallel regions
//
//   Root (compound)   X1 (atomic, onentry raise Go) | X2 (atomic)
//   X1 --Go--> X2
// ══════════════════════════════════════════════════════════════════

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
    last_internal: bool,
    last_targetless: bool,
    last_source: LState,
    entries: u32,
}

impl LinearPolicy {
    fn new() -> Self {
        Self {
            last_internal: false,
            last_targetless: false,
            last_source: LState::Root,
            entries: 0,
        }
    }
}

impl StatePolicy for LinearPolicy {
    type State = LState;
    type Event = LEvent;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;

    const HAS_PARALLEL_STATES: bool = false;
    const NEEDS_SCRIPT_ENGINE: bool = false;

    fn initial_state() -> Self::State {
        LState::Root
    }

    fn is_final_state(_state: Self::State) -> bool {
        false
    }

    fn get_parent(state: Self::State) -> Option<Self::State> {
        match state {
            LState::Root => None,
            LState::X1 | LState::X2 => Some(LState::Root),
        }
    }

    fn is_compound_state(state: Self::State) -> bool {
        matches!(state, LState::Root)
    }

    fn is_descendant_of(desc: Self::State, anc: Self::State) -> bool {
        Self::get_parent(desc) == Some(anc)
    }

    fn get_document_order(state: Self::State) -> u32 {
        match state {
            LState::Root => 0,
            LState::X1 => 1,
            LState::X2 => 2,
        }
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

    fn get_initial_or_history_child(&self, state: Self::State) -> Self::State {
        match state {
            LState::Root => LState::X1,
            other => other,
        }
    }

    fn last_transition_is_internal(&self) -> bool {
        self.last_internal
    }
    fn set_last_transition_is_internal(&mut self, v: bool) {
        self.last_internal = v;
    }
    fn last_transition_is_targetless(&self) -> bool {
        self.last_targetless
    }
    fn set_last_transition_is_targetless(&mut self, v: bool) {
        self.last_targetless = v;
    }
    fn last_transition_source_state(&self) -> Self::State {
        self.last_source
    }
    fn set_last_transition_source_state(&mut self, s: Self::State) {
        self.last_source = s;
    }

    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        _path_child: Option<Self::State>,
    ) {
        self.entries += 1;
        if let LState::X1 = state {
            engine.raise(EventWithMetadata::new(LEvent::Go));
        }
    }

    fn execute_exit_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _pre: &[Self::State],
    ) {
    }

    fn process_transition(
        &mut self,
        current_state: &mut Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> bool {
        self.last_source = *current_state;
        match (*current_state, event) {
            (LState::X1, LEvent::Go) => {
                self.last_internal = false;
                self.last_targetless = false;
                *current_state = LState::X2;
                true
            }
            _ => false,
        }
    }

    fn execute_transition_actions(&mut self, _engine: &mut Engine<Self>) {}
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
fn linear_configuration_round_trips_without_a_policy_active_set() {
    let mut ran = Engine::<LinearPolicy>::new(LinearPolicy::new());
    ran.initialize();

    let saved_configuration = ran.get_active_states();
    let saved_current = ran.get_current_state();
    assert_eq!(saved_current, LState::X2);
    // No parallel regions: the chain is the parent walk, leaf first.
    assert_eq!(
        saved_configuration,
        state_chain_from_slice([LState::X2, LState::Root])
    );

    let mut restored = Engine::<LinearPolicy>::new(LinearPolicy::new());
    restored
        .enter_at(&saved_configuration, saved_current)
        .expect("a parent-walk configuration is a configuration");

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
    let configuration =
        state_chain_from_slice([PState::P, PState::Ra, PState::A1, PState::Rb, PState::B1]);

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
    let empty: StateChain<PState> = sce_rust_runtime::helpers::hierarchy::new_chain();
    assert_eq!(
        engine.enter_at(&empty, PState::A1),
        Err(ConfigurationRejection::Empty)
    );
}

#[test]
fn two_siblings_of_one_region_are_refused() {
    // §scxml-3.3: RA is compound, so exactly one of A1/A2 can be active.
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = state_chain_from_slice([
        PState::P,
        PState::Ra,
        PState::A1,
        PState::A2,
        PState::Rb,
        PState::B1,
    ]);
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
    let chain = state_chain_from_slice([PState::P, PState::Ra, PState::A1]);
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
    let chain = state_chain_from_slice([PState::A1]);
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
    let chain = state_chain_from_slice([
        PState::P,
        PState::Ra,
        PState::A1,
        PState::A1,
        PState::Rb,
        PState::B1,
    ]);
    assert_eq!(
        engine.enter_at(&chain, PState::A1),
        Err(ConfigurationRejection::Duplicate { state: PState::A1 })
    );
}

#[test]
fn a_current_state_outside_the_configuration_is_refused() {
    let mut engine = Engine::<ParallelPolicy>::new(ParallelPolicy::new());
    let chain = state_chain_from_slice([PState::P, PState::Ra, PState::A1, PState::Rb, PState::B1]);
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
    let chain = state_chain_from_slice([PState::P, PState::Ra, PState::A1, PState::Rb, PState::B1]);
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
    let chain = state_chain_from_slice([PState::A1]);
    assert!(engine.enter_at(&chain, PState::A1).is_err());
    assert_eq!(engine.get_current_state(), before);
    assert!(!engine.is_running());
    assert_eq!(engine.policy().entries, 0);

    engine.initialize();
    assert_eq!(engine.get_current_state(), PState::A2);
}
