// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What taking one transition of a test component does, in the order it does
//! it: the states it exits, the content it runs, the states it enters
//! (docs/SCE_TEST_PROFILE.md §4.4, §9.2 E8).
//!
//! A test component has no `<parallel>` and no `<history>` (§8.2), so its
//! configuration is one atomic state and the ancestors of it, and the exit
//! and entry sets of a transition taken from a given atomic state are fixed:
//! the export computes them once, where it writes the branch. It does not
//! compute them itself. The machine is the one Appendix D core every Rust
//! machine runs on (`sce_rust_runtime::helpers::microstep`, docs/adr/0064),
//! over the document's tables as the generator builds them
//! ([`crate::document_tables::DocumentTables::of`]), started in the
//! configuration of that atomic state, and what the core asks the machine to
//! do is recorded instead of done. So the order of exits, of content and of
//! entries, a default entry's initial content among them, is the engine's own
//! and is not written a second time here; checking it against the profile is
//! the translation validator's (ADR 0091 answer 12).

use std::mem::MaybeUninit;

use sce_rust_runtime::helpers::microstep::{
    exit_interpreter, microstep, Behaviour, Scratch, WalkStep,
};
use sce_rust_runtime::helpers::run_store::RunStoreMut;
use sce_rust_runtime::helpers::state_set::Word;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables as CoreTables, EventId, HistoryId, StateId,
    StateKind as CoreKind, StateRow as CoreState, Target, TransitionId,
    TransitionRow as CoreTransition,
};

use crate::document_tables::{DocumentTables, StateKind, TargetRef, TransitionOrigin};
use crate::model::SCXMLModel;

/// What the core asked the machine to do, one call at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Step {
    /// The state's onexit, then the cancel of its invocations.
    Exit(String),
    /// The content of a transition.
    Content(Content),
    /// The state's onentry, then, when `default`, the content of its initial
    /// transition.
    Enter { state: String, default: bool },
}

/// Which transition's content a [`Step::Content`] runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Content {
    /// The `index`-th transition written in `state`.
    Written { state: String, index: usize },
    /// The initial transition of the compound state `state`.
    Initial(String),
    /// The document's own initial transition, which holds none.
    Document,
}

/// A transition taken: the steps, and the state the machine is left in, its
/// one atomic state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Taken {
    pub(super) steps: Vec<Step>,
    pub(super) leaf: String,
}

/// A test component's tables, as the core reads them and as the generator
/// builds them.
pub(super) struct Machine<'m> {
    model: &'m SCXMLModel,
    rows: DocumentTables,
    core: &'static CoreTables,
}

impl<'m> Machine<'m> {
    /// The machine of `model`, or why its tables could not be built.
    pub(super) fn of(model: &'m SCXMLModel) -> Result<Self, String> {
        let rows = DocumentTables::of(model).map_err(|e| e.to_string())?;
        let core = core_tables(&rows)?;
        Ok(Self { model, rows, core })
    }

    /// The machine started: the document's initial transition, taken from
    /// no configuration.
    pub(super) fn start(&self) -> Taken {
        self.run(&[], TransitionId(self.rows.initial))
    }

    /// The `index`-th transition written in `source`, taken from the
    /// configuration whose atomic state is `leaf`.
    pub(super) fn take(&self, leaf: &str, source: &str, index: usize) -> Option<Taken> {
        let transition = self.rows.transition_id(source, index)?;
        Some(self.run(&self.configuration(leaf), TransitionId(transition)))
    }

    /// The states the machine exits when it halts in the configuration whose
    /// atomic state is `leaf`: the core's exitInterpreter.
    pub(super) fn halt(&self, leaf: &str) -> Vec<Step> {
        let mut recorder = Recorder::new(&self.rows);
        recorder.store().set_configuration(self.configuration(leaf));
        exit_interpreter(&mut recorder);
        recorder.steps
    }

    /// Whether `state` is a child of the `<scxml>` element and a `<final>`:
    /// entering it ends the component (P10).
    pub(super) fn is_top_level_final(&self, state: &str) -> bool {
        self.rows
            .states
            .iter()
            .any(|row| row.id == state && row.parent.is_none() && row.kind == StateKind::Final)
    }

    /// The states the machine is in while `leaf` is its atomic state.
    fn configuration(&self, leaf: &str) -> Vec<StateId> {
        crate::transition_function::configuration(self.model, leaf)
            .iter()
            .filter_map(|id| self.rows.state_id(id))
            .map(StateId)
            .collect()
    }

    fn run(&self, configuration: &[StateId], transition: TransitionId) -> Taken {
        let states = self.rows.states.len();
        let mut sets: Vec<Word> = vec![0; state_scratch(states)];
        let mut walk: Vec<MaybeUninit<WalkStep>> =
            vec![MaybeUninit::uninit(); self.rows.bounds.entry_walk.max(1)];
        let mut history_defaults: Vec<MaybeUninit<(StateId, HistoryId)>> =
            vec![MaybeUninit::uninit(); self.rows.bounds.history_defaults.max(1)];
        let mut recorder = Recorder::new(&self.rows);
        recorder
            .store()
            .set_configuration(configuration.iter().copied());
        let scratch = Scratch {
            sets: &mut sets,
            walk: &mut walk,
            history_defaults: &mut history_defaults,
        };
        let _ = microstep(self.core, &mut recorder, &[transition], scratch);
        // With no `<parallel>`, the configuration is a chain from a top-level
        // state down, and the deepest state of it has the largest number.
        let leaf = recorder
            .store()
            .as_ref()
            .configuration()
            .last()
            .map(|state| self.rows.states[usize::from(state.0)].id.clone())
            .unwrap_or_default();
        Taken {
            steps: recorder.steps,
            leaf,
        }
    }
}

/// The core's tables over `rows`. The core reads `'static` rows, as a
/// generated machine's are, so the rows are leaked: a test component's are
/// built once for each export, and the process that exports it ends.
fn core_tables(rows: &DocumentTables) -> Result<&'static CoreTables, String> {
    if !rows.histories.is_empty() || rows.states.iter().any(|r| r.kind == StateKind::Parallel) {
        return Err(
            "a test component has no <parallel> and no <history> (docs/SCE_TEST_PROFILE.md \
             §8.2)"
                .to_string(),
        );
    }
    let target = |target: &TargetRef| match target {
        TargetRef::State(state) => Target::state(StateId(*state)),
        TargetRef::History(history) => Target::history(HistoryId(*history)),
    };
    let mut pool: Vec<Target> = Vec::new();
    let mut transitions: Vec<CoreTransition> = Vec::with_capacity(rows.transitions.len());
    for row in &rows.transitions {
        let written: Vec<Target> = rows.targets[row.targets.range()]
            .iter()
            .map(target)
            .collect();
        let targets = match written.as_slice() {
            [] => Target::NONE,
            [only] => *only,
            _ => {
                let position = u16::try_from(pool.len()).map_err(|e| e.to_string())?;
                let count = u16::try_from(written.len()).map_err(|e| e.to_string())?;
                pool.push(Target::count(count));
                pool.extend_from_slice(&written);
                Target::list(position)
            }
        };
        transitions.push(CoreTransition::new(
            row.source.map(StateId),
            targets,
            row.internal,
            row.has_content,
        ));
    }
    let states: Vec<CoreState> = rows
        .states
        .iter()
        .map(|row| {
            CoreState::new(
                row.parent.map(StateId),
                match row.kind {
                    StateKind::Atomic => CoreKind::Atomic,
                    StateKind::Compound => CoreKind::Compound,
                    StateKind::Parallel => CoreKind::Parallel,
                    StateKind::Final => CoreKind::Final,
                },
                StateId(row.last_descendant),
                row.initial.map(TransitionId),
            )
        })
        .collect();
    let tables = CoreTables {
        states: Box::leak(states.into_boxed_slice()),
        histories: &[],
        transitions: Box::leak(transitions.into_boxed_slice()),
        targets: Box::leak(pool.into_boxed_slice()),
        initial: TransitionId(rows.initial),
        // No main loop is driven: the core is handed the one transition it
        // takes, and the machine never names a state before it starts.
        default_state: StateId(0),
        null_event: EventId(0),
        error_events: &[],
        history: None,
        parallel: None,
    };
    tables
        .check()
        .map_err(|fault| format!("the tables break a rule of the core: {fault:?}"))?;
    Ok(Box::leak(Box::new(tables)))
}

/// A machine that does nothing the core asks of it but write it down, in a
/// run store of its own.
struct Recorder<'r> {
    rows: &'r DocumentTables,
    configuration: Vec<Word>,
    records: Vec<Word>,
    recorded: Vec<bool>,
    steps: Vec<Step>,
}

impl<'r> Recorder<'r> {
    fn new(rows: &'r DocumentTables) -> Self {
        Self {
            rows,
            configuration: vec![0; state_words(rows.states.len())],
            records: Vec::new(),
            recorded: Vec::new(),
            steps: Vec::new(),
        }
    }

    fn id(&self, state: StateId) -> String {
        self.rows.states[usize::from(state.0)].id.clone()
    }
}

impl Behaviour for Recorder<'_> {
    fn store(&mut self) -> RunStoreMut<'_> {
        RunStoreMut::new(
            &mut self.configuration,
            &mut self.records,
            &mut self.recorded,
        )
    }

    // The core is handed the transition it takes and selects none.
    fn first_enabled_transition(
        &mut self,
        _state: StateId,
        _event: EventId,
    ) -> Option<TransitionId> {
        None
    }

    fn exit_state(&mut self, state: StateId) {
        let id = self.id(state);
        self.steps.push(Step::Exit(id));
    }

    fn execute_transition_content(&mut self, transition: TransitionId) {
        let row = &self.rows.transitions[usize::from(transition.0)];
        let state = |source: Option<u16>| {
            source
                .map(|state| self.rows.states[usize::from(state)].id.clone())
                .unwrap_or_default()
        };
        let content = match row.origin {
            TransitionOrigin::Written { index_in_source } => Content::Written {
                state: state(row.source),
                index: usize::from(index_in_source),
            },
            TransitionOrigin::Initial => Content::Initial(state(row.source)),
            TransitionOrigin::HistoryDefault(_) | TransitionOrigin::Document => Content::Document,
        };
        self.steps.push(Step::Content(content));
    }

    fn enter_state(&mut self, state: StateId, is_default_entry: bool) {
        let id = self.id(state);
        self.steps.push(Step::Enter {
            state: id,
            default: is_default_entry,
        });
    }

    // A test component has no `<history>` (docs/SCE_TEST_PROFILE.md §8.2),
    // and its tables hold none ([`core_tables`]).
    fn execute_history_default_content(&mut self, _history: HistoryId) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(body: &str) -> SCXMLModel {
        crate::parser::SCXMLParser::new()
            .parse_string(
                &format!(
                    "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" \
                     datamodel=\"null\" initial=\"outer\">{body}</scxml>"
                ),
                "macrostep",
            )
            .expect("parses")
    }

    /// A transition out of a nested state to a sibling of its parent exits
    /// the state and its parent, innermost first, runs its content, and
    /// enters the target's default entry, the initial content where a
    /// compound state is entered by default; an internal transition inside
    /// its compound source keeps the source.
    #[test]
    fn the_core_orders_the_exits_the_content_and_the_entries() {
        let model = model(
            "<state id=\"outer\" initial=\"a\">\
               <state id=\"a\"><transition target=\"other\"><log label=\"a\"/></transition></state>\
               <state id=\"b\"/>\
               <transition type=\"internal\" target=\"b\"/>\
             </state>\
             <state id=\"other\"><initial><transition target=\"c\"><log label=\"c\"/></transition></initial>\
               <state id=\"c\"/></state>",
        );
        let machine = Machine::of(&model).expect("tables");
        let start = machine.start();
        assert_eq!(start.leaf, "a");
        let taken = machine.take("a", "a", 0).expect("a transition");
        assert_eq!(
            taken.steps,
            [
                Step::Exit("a".into()),
                Step::Exit("outer".into()),
                Step::Content(Content::Written {
                    state: "a".into(),
                    index: 0
                }),
                Step::Enter {
                    state: "other".into(),
                    default: true
                },
                Step::Enter {
                    state: "c".into(),
                    default: false
                },
            ]
        );
        assert_eq!(taken.leaf, "c");
        let internal = machine.take("a", "outer", 0).expect("a transition");
        assert_eq!(
            internal.steps,
            [
                Step::Exit("a".into()),
                Step::Enter {
                    state: "b".into(),
                    default: false
                },
            ]
        );
        assert_eq!(internal.leaf, "b");
    }
}
