// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096: a machine whose script session cannot be established does
// not start, and its host is told why.
//
// The policy is written by hand so that what these cases read is the engine:
// its `initialize_data_model` is the step a generated policy establishes its
// script session in, and here it fails on request at the step the case names.
// A start refused that way enters nothing, runs no executable content, takes
// no event and leaves the machine not running; each door of the start says so
// in its own type:
//
//   * `initialize` returns the policy's `StartFailure`, here the
//     `ScriptSessionFailure` naming the step;
//   * `initialize_as_root` returns `RootStartRefusal::ScriptSession`;
//   * `enter_at` returns `ConfigurationRejection::ScriptSessionUnavailable`
//     and touches nothing;
//   * `invoke_processing::start_child`, the start a generated parent gives its
//     child, returns the failure for the parent to raise.
//
// The machine: `root` (compound) holds `x1`, whose `<onentry>` raises `go`, and
// `x2`; `x1 --go--> x2`. A start that ran would enter `x1`, run its entry
// action and take `go`, so a refused one is told apart by every counter.

// The session's failure carries the engine's words, an owned string: the
// `std` profile's, as every script engine is.
#![cfg(feature = "std")]

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::helpers::invoke_processing::start_child;
use sce_rust_runtime::{
    state_scratch, state_words, ConfigurationRejection, DocumentTables, Engine, EventId,
    EventWithMetadata, HistoryId, NoHistory, RootStartRefusal, RunStore, ScriptSessionFailure,
    ScriptSessionStage, StateId, StateKind, StatePolicy, StateRow, Target, TransitionId,
    TransitionRow, Workspace,
};

// Declared in document order, so a variant's discriminant is its `StateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    Root,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ev {
    None,
    Go,
}

/// What the engine asked of the policy, read from the outside.
struct SessionPolicy {
    /// The step establishing the session fails at, or `None` for a session
    /// that is established.
    fails_at: Option<ScriptSessionStage>,
    /// How many times the start asked for the datamodel.
    data_model_inits: u32,
    /// How many states had their `<onentry>` run.
    entries: u32,
    /// How many transitions' content ran.
    transitions: u32,
}

impl SessionPolicy {
    fn failing_at(stage: ScriptSessionStage) -> Self {
        Self {
            fails_at: Some(stage),
            data_model_inits: 0,
            entries: 0,
            transitions: 0,
        }
    }

    fn established() -> Self {
        Self {
            fails_at: None,
            data_model_inits: 0,
            entries: 0,
            transitions: 0,
        }
    }
}

/// `x1`'s one transition, the first row of the transitions table.
const X1_ON_GO: TransitionId = TransitionId(0);

impl StatePolicy for SessionPolicy {
    type State = St;
    type Event = Ev;
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type InternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ExternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ScheduledSendId = sce_rust_runtime::SceString;
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // Three states and no history; one atomic state active; three steps for
    // each of the two levels and three of headroom for the walks.
    type Workspace = Workspace<{ state_scratch(3) }, 1, 9, 0>;
    type RunStore = RunStore<{ state_words(3) }, 0, 0>;
    type Configuration = BoundedVec<St, 2>;
    // A machine with a script engine: its start is refused when its session
    // cannot be established.
    type StartFailure = ScriptSessionFailure;

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
            TransitionRow::new(Some(StateId(1)), Target::state(StateId(2)), false, false),
            // root's initial: x1
            TransitionRow::new(Some(StateId(0)), Target::state(StateId(1)), false, false),
            // the document's initial: root
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(2),
        default_state: StateId(St::Root as u16),
        null_event: EventId(0),
        error_events: &[],
        history: None,
        parallel: None,
    };
    const STATES: &'static [St] = &[St::Root, St::X1, St::X2];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [Ev] = &[Ev::None, Ev::Go];

    const HAS_PARALLEL_STATES: bool = false;
    const NEEDS_SCRIPT_ENGINE: bool = true;
    const NEEDS_DATA_MODEL_INIT: bool = true;

    fn state_id(state: Self::State) -> StateId {
        StateId(state as u16)
    }

    fn history_id(history: Self::History) -> HistoryId {
        match history {}
    }

    fn event_id(event: Self::Event) -> EventId {
        EventId(event as u16)
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        match event {
            Ev::None => "",
            Ev::Go => "go",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "go" => Some(Ev::Go),
            _ => None,
        }
    }

    fn get_state_name(state: Self::State) -> &'static str {
        match state {
            St::Root => "root",
            St::X1 => "x1",
            St::X2 => "x2",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "root" => Some(St::Root),
            "x1" => Some(St::X1),
            "x2" => Some(St::X2),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        Ev::None
    }

    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
        self.entries += 1;
        if let St::X1 = state {
            engine.raise(EventWithMetadata::new(Ev::Go));
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
            (St::X1, Ev::Go) => Some(X1_ON_GO),
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        _transition: TransitionId,
        _engine: &mut Engine<Self>,
    ) {
        self.transitions += 1;
    }

    // Where a generated policy establishes its script session: create it, load
    // the shared library, bind the system variables. Here the step the case
    // names fails, and the policy reports it as a generated one does.
    fn initialize_data_model(
        &mut self,
        _engine: &mut Engine<Self>,
    ) -> Result<(), ScriptSessionFailure> {
        self.data_model_inits += 1;
        match self.fails_at {
            Some(stage) => Err(ScriptSessionFailure::new(stage, "the engine said no")),
            None => Ok(()),
        }
    }
}

/// The engine is as `Engine::new` left it: nothing entered, nothing run,
/// nothing taken, not running.
fn assert_nothing_started(engine: &Engine<SessionPolicy>) {
    assert!(
        !engine.is_running(),
        "a refused start leaves the machine not running"
    );
    assert!(
        engine.get_active_states().is_empty(),
        "a refused start enters no state, and {:?} is active",
        engine.get_active_states()
    );
    assert_eq!(
        engine.policy().entries,
        0,
        "a refused start runs no <onentry>"
    );
    assert_eq!(
        engine.policy().transitions,
        0,
        "a refused start takes no event"
    );
}

#[test]
fn the_hand_written_tables_keep_the_rules_the_procedures_assume() {
    assert_eq!(SessionPolicy::TABLES.check(), Ok(()));
}

#[test]
fn a_start_whose_session_is_established_runs() {
    // The control: the same machine, its session established, enters x1, runs
    // its <onentry> and takes `go`. Without it the cases below could pass on a
    // machine that cannot move at all.
    let mut engine = Engine::new(SessionPolicy::established());
    engine
        .initialize()
        .expect("a session that is established starts the machine");
    assert!(engine.is_running());
    assert_eq!(engine.get_current_state(), St::X2);
    assert!(engine.policy().entries > 0);
    assert_eq!(engine.policy().transitions, 1);
}

#[test]
fn a_start_whose_session_cannot_be_created_is_refused_with_the_step() {
    let mut engine = Engine::new(SessionPolicy::failing_at(ScriptSessionStage::Create));
    let refused = engine
        .initialize()
        .expect_err("a session that cannot be created refuses the start");
    assert_eq!(refused.stage, ScriptSessionStage::Create);
    assert_eq!(refused.message, "the engine said no");
    assert_eq!(engine.policy().data_model_inits, 1);
    assert_nothing_started(&engine);
}

#[test]
fn each_step_that_fails_is_the_step_the_host_is_told() {
    for stage in [
        ScriptSessionStage::Create,
        ScriptSessionStage::Library,
        ScriptSessionStage::SystemVariables,
    ] {
        let mut engine = Engine::new(SessionPolicy::failing_at(stage));
        let refused = engine
            .initialize()
            .expect_err("a session that cannot be established refuses the start");
        assert_eq!(refused.stage, stage);
        assert_nothing_started(&engine);
    }
}

#[test]
fn a_refused_start_is_still_refused_when_the_host_drives_on() {
    // A host that drives a machine it was refused has nothing to drive: the
    // event it posts is refused as one posted to a machine that is not
    // running, and runs no transition.
    let mut engine = Engine::new(SessionPolicy::failing_at(ScriptSessionStage::Library));
    assert!(engine.initialize().is_err());
    engine.process_event(Ev::Go);
    assert_eq!(engine.unseen_external_events(), 1);
    assert_nothing_started(&engine);
}

#[test]
fn the_checked_root_start_refuses_for_the_session() {
    let mut engine = Engine::new(SessionPolicy::failing_at(
        ScriptSessionStage::SystemVariables,
    ));
    let refused = engine
        .initialize_as_root()
        .expect_err("a session that cannot be established refuses the root start");
    match &refused {
        RootStartRefusal::ScriptSession(failure) => {
            assert_eq!(failure.stage, ScriptSessionStage::SystemVariables);
        }
        other => panic!("the root start was refused for {other:?}, not for its session"),
    }
    assert_eq!(
        refused.reason(),
        "the machine's script session could not be established"
    );
    assert_eq!(
        refused.to_string(),
        "the script session could not be established at step 'system-variables': the engine said no"
    );
    assert_nothing_started(&engine);
}

#[test]
fn the_checked_root_start_runs_a_machine_whose_session_is_established() {
    let mut engine = Engine::new(SessionPolicy::established());
    assert_eq!(engine.initialize_as_root(), Ok(()));
    assert_eq!(engine.get_current_state(), St::X2);
}

#[test]
fn a_resumed_start_whose_session_cannot_be_established_is_refused_and_touches_nothing() {
    let mut engine = Engine::new(SessionPolicy::failing_at(ScriptSessionStage::Create));
    let before = engine.get_current_state();
    let refused = engine
        .enter_at(&[St::Root, St::X2], St::X2)
        .expect_err("a session that cannot be established refuses the resumed start");
    assert_eq!(
        refused,
        ConfigurationRejection::ScriptSessionUnavailable(ScriptSessionFailure::new(
            ScriptSessionStage::Create,
            "the engine said no"
        ))
    );
    assert_eq!(engine.get_current_state(), before);
    assert_nothing_started(&engine);
}

#[test]
fn a_resumed_start_whose_session_is_established_enters_the_configuration() {
    // The control for the case above: the same chain, the same door.
    let mut engine = Engine::new(SessionPolicy::established());
    engine
        .enter_at(&[St::Root, St::X2], St::X2)
        .expect("a chain from the root down is a configuration");
    assert!(engine.is_running());
    assert_eq!(engine.get_current_state(), St::X2);
    assert_eq!(engine.policy().data_model_inits, 1);
}

#[test]
fn a_configuration_of_another_document_is_refused_before_the_session_is_asked_for() {
    // The configuration is judged before anything is established, so a stale
    // record costs no session.
    let mut engine = Engine::new(SessionPolicy::failing_at(ScriptSessionStage::Create));
    assert_eq!(
        engine.enter_at(&[St::X2], St::X2),
        Err(ConfigurationRejection::AncestorMissing {
            state: St::X2,
            parent: St::Root
        })
    );
    assert_eq!(engine.policy().data_model_inits, 0);
}

#[test]
fn a_child_whose_session_fails_is_handed_back_as_the_failure() {
    let mut child = Engine::new(SessionPolicy::failing_at(ScriptSessionStage::Library));
    let refused = start_child(&mut child)
        .expect_err("a child whose session cannot be established is not started");
    assert_eq!(refused.stage, ScriptSessionStage::Library);
    assert_nothing_started(&child);

    let mut started = Engine::new(SessionPolicy::established());
    assert_eq!(start_child(&mut started), Ok(()));
    assert_eq!(started.get_current_state(), St::X2);
}
