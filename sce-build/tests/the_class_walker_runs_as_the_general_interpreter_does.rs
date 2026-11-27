// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document of the static class runs on the class walker as it runs on the
//! general interpreter (ADR 0135, decision 4).
//!
//! The class is ADR 0125's: no `<history>`, no `<parallel>`, no transition of
//! more than one target. Such a document's tables carry the row walk, which
//! selects from candidate rows and reads its exit and entry sets from entry
//! rows; any other document's carry the general walk, which selects over the
//! configuration from the selection rows and computes the sets. Both are one
//! copy in the runtime, reached through the tables.
//!
//! For every generatable document of the corpus in the class, the two
//! consumer-shaped documents of `tests/integration/static_class` among them,
//! the tables are built twice from one model, as the generator builds them:
//! the row walk's and the general walk's. Seeded random sequences of the
//! document's events drive both through the runtime's procedures, by a loop
//! that keeps Appendix D's order (eventless transitions to completion, then
//! the internal queue, one external event at a time), with a machine whose
//! guards answer from the seed, whose entry into a `<final>` child of a
//! compound state raises `done.state.<id>` where the machine numbers it, and
//! which keeps a journal of every call the walkers make of it. Every step's
//! configuration and `Entered`, and the journal (the guards asked and what
//! each answered, every exit, every transition's content, every entry), must
//! be the same.

mod common;

use std::collections::VecDeque;
use std::mem::MaybeUninit;

use sce_build::document_tables::{DocumentTables, StateKind};
use sce_rust_runtime::helpers::microstep::{
    enter_initial_configuration, exit_interpreter, microstep, select_transitions, Behaviour,
    Entered, Scratch, SliceVec, WalkStep,
};
use sce_rust_runtime::helpers::run_store::RunStoreMut;
use sce_rust_runtime::helpers::state_set::Word;
use sce_rust_runtime::helpers::tables as rt;
use sce_rust_runtime::{state_scratch, state_words};

use common::runtime_tables;

/// The seeds each document is run with.
const SEEDS: u64 = 6;

/// The external events of one run.
const EVENTS: usize = 48;

/// The microsteps one macrostep may take: a document whose eventless
/// transitions or raised events cycle is stopped here, at the same step on
/// both walks.
const MACROSTEP_BUDGET: usize = 64;

/// A small xorshift generator: a run is a function of its seed alone, so a
/// failure names the seed that reproduces it.
struct Rng(u64);

impl Rng {
    fn draw(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.draw() % n as u64) as usize
    }
}

/// One call a walker made of the machine, in the order it made it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    /// A guard asked, and what it answered.
    Guard(rt::TransitionId, bool),
    /// A state's `<onexit>`.
    Exit(rt::StateId),
    /// A transition's content.
    Content(rt::TransitionId),
    /// A state's `<onentry>`, and whether it was entered by default.
    Enter(rt::StateId, bool),
}

/// What the document's behaviour does that the walkers can see, the same for
/// both runs of one seed.
struct Hooks {
    /// For each transition, whether it is guarded: an unguarded transition's
    /// guard holds, as the generated `guard_holds` answers it.
    guarded: Vec<bool>,
    /// For each state, the event its entry raises: `done.state.<parent>` for
    /// a `<final>` child of a compound state, where the machine numbers it.
    raises: Vec<Option<u16>>,
    /// What every guarded transition's guard answers is drawn from.
    seed: u64,
}

impl Hooks {
    /// What `transition`'s guard answers when it is the `asked`-th guard of
    /// the run: a function of the seed, the transition and the count, so two
    /// runs that ask the same guards in the same order are answered alike.
    fn answer(&self, transition: rt::TransitionId, asked: u64) -> bool {
        let mut rng = Rng((self.seed ^ (u64::from(transition.0) << 32) ^ asked) | 1);
        rng.draw() & 1 == 0
    }
}

/// The machine both walks drive: its run store, the internal queue its entry
/// hooks raise into, and the journal of every call.
struct Machine<'h> {
    hooks: &'h Hooks,
    configuration: Vec<Word>,
    /// No `<history>` in the class, so nothing is recorded.
    records: Vec<Word>,
    recorded: Vec<bool>,
    internal: VecDeque<u16>,
    journal: Vec<Call>,
    asked: u64,
}

impl<'h> Machine<'h> {
    fn new(hooks: &'h Hooks, states: usize) -> Self {
        Self {
            hooks,
            configuration: vec![0; state_words(states)],
            records: Vec::new(),
            recorded: Vec::new(),
            internal: VecDeque::new(),
            journal: Vec::new(),
            asked: 0,
        }
    }

    /// The active states, in document order.
    fn active(&mut self, states: usize) -> Vec<u16> {
        let store = self.store();
        (0..states as u16)
            .filter(|&state| store.as_ref().is_active(rt::StateId(state)))
            .collect()
    }
}

impl Behaviour for Machine<'_> {
    fn store(&mut self) -> RunStoreMut<'_> {
        RunStoreMut::new(
            &mut self.configuration,
            &mut self.records,
            &mut self.recorded,
        )
    }

    fn first_enabled_transition(
        &mut self,
        _state: rt::StateId,
        _event: rt::EventId,
    ) -> Option<rt::TransitionId> {
        panic!("tables that carry selection or candidate rows never ask the machine to select")
    }

    fn guard_holds(&mut self, transition: rt::TransitionId) -> bool {
        let holds =
            !self.hooks.guarded[transition.index()] || self.hooks.answer(transition, self.asked);
        self.asked += 1;
        self.journal.push(Call::Guard(transition, holds));
        holds
    }

    fn exit_state(&mut self, state: rt::StateId) {
        self.journal.push(Call::Exit(state));
    }

    fn execute_transition_content(&mut self, transition: rt::TransitionId) {
        self.journal.push(Call::Content(transition));
    }

    fn enter_state(&mut self, state: rt::StateId, is_default_entry: bool) {
        self.journal.push(Call::Enter(state, is_default_entry));
        if let Some(event) = self.hooks.raises[state.index()] {
            self.internal.push_back(event);
        }
    }

    fn execute_history_default_content(&mut self, _history: rt::HistoryId) {
        panic!("a document of the class has no <history>")
    }
}

/// The buffers a call into the procedures borrows, of the document's sizes.
struct Buffers {
    sets: Vec<Word>,
    walk: Vec<MaybeUninit<WalkStep>>,
    defaults: Vec<MaybeUninit<(rt::StateId, rt::HistoryId)>>,
    enabled: Vec<MaybeUninit<rt::TransitionId>>,
}

impl Buffers {
    fn of(doc: &DocumentTables) -> Self {
        Self {
            sets: vec![0; state_scratch(doc.states.len())],
            walk: vec![MaybeUninit::uninit(); doc.bounds.entry_walk.max(1)],
            defaults: vec![MaybeUninit::uninit(); doc.bounds.history_defaults.max(1)],
            enabled: vec![MaybeUninit::uninit(); doc.bounds.enabled.max(1)],
        }
    }

    fn scratch(&mut self) -> Scratch<'_> {
        Scratch {
            sets: &mut self.sets,
            walk: &mut self.walk,
            history_defaults: &mut self.defaults,
        }
    }
}

/// What one run observed: the configuration and `Entered` after every step,
/// the configuration once the interpreter is exited, the journal, and whether
/// the run ended in a top-level `<final>`.
#[derive(Debug, PartialEq)]
struct Run {
    steps: Vec<(Entered, Vec<u16>)>,
    exited: Vec<u16>,
    journal: Vec<Call>,
    ended: bool,
}

/// One run of `events` over `tables`.
struct Runner<'r> {
    tables: &'r rt::DocumentTables<'static>,
    states: usize,
    machine: Machine<'r>,
    buffers: Buffers,
    steps: Vec<(Entered, Vec<u16>)>,
}

impl Runner<'_> {
    /// Record a step; whether it ended the run.
    fn record(&mut self, entered: Entered) -> bool {
        let active = self.machine.active(self.states);
        self.steps.push((entered, active));
        entered.ended_in.is_some()
    }

    /// The microstep `event` selects, if it selects one: `None` when it
    /// selects nothing, else whether the step ended the run.
    fn take(&mut self, event: u16) -> Option<bool> {
        let enabled: Vec<rt::TransitionId> = {
            let Buffers {
                sets,
                walk,
                defaults,
                enabled,
            } = &mut self.buffers;
            let mut slots = SliceVec::new(enabled);
            select_transitions(
                self.tables,
                &mut self.machine,
                rt::EventId(event),
                &mut slots,
                Scratch {
                    sets,
                    walk,
                    history_defaults: defaults,
                },
            );
            slots.to_vec()
        };
        if enabled.is_empty() {
            return None;
        }
        let entered = microstep(
            self.tables,
            &mut self.machine,
            &enabled,
            self.buffers.scratch(),
        );
        Some(self.record(entered))
    }

    /// Appendix D's inner loop: eventless transitions to completion, then one
    /// internal event, and again, until neither selects anything; whether the
    /// run ended.
    fn complete(&mut self, null_event: u16) -> bool {
        for _ in 0..MACROSTEP_BUDGET {
            // The eventless selection first, and an internal event only when
            // it selects nothing; an internal event that selects nothing is a
            // turn of the loop all the same.
            let step = self.take(null_event).or_else(|| {
                self.machine
                    .internal
                    .pop_front()
                    .map(|event| self.take(event).unwrap_or(false))
            });
            match step {
                Some(true) => return true,
                Some(false) => {}
                None => return false,
            }
        }
        false
    }
}

/// `events` run over `tables`, from the initial configuration to the
/// interpreter's exit: by Appendix D's loop, or, `reactive`, by the reactive
/// turn's (docs/adr/0148), which takes each external event's microstep and no
/// other.
fn run(
    tables: &rt::DocumentTables<'static>,
    doc: &DocumentTables,
    hooks: &Hooks,
    events: &[u16],
    reactive: bool,
) -> Run {
    let states = doc.states.len();
    let mut runner = Runner {
        tables,
        states,
        machine: Machine::new(hooks, states),
        buffers: Buffers::of(doc),
        steps: Vec::new(),
    };
    let null_event = tables.null_event.0;
    let entered =
        enter_initial_configuration(tables, &mut runner.machine, runner.buffers.scratch());
    let mut ended = runner.record(entered) || (!reactive && runner.complete(null_event));
    for &event in events {
        if ended {
            break;
        }
        ended = runner.take(event).unwrap_or(false) || (!reactive && runner.complete(null_event));
    }
    exit_interpreter(&mut runner.machine);
    let exited = runner.machine.active(states);
    Run {
        steps: runner.steps,
        exited,
        journal: runner.machine.journal,
        ended,
    }
}

/// For each state of `doc`, the event its entry raises: `done.state.<id>` of
/// its parent, for a `<final>` child of a compound state whose machine
/// numbers that event among `names`.
fn done_state_events(doc: &DocumentTables, names: &[String]) -> Vec<Option<u16>> {
    doc.states
        .iter()
        .map(|row| {
            let parent = row.parent?;
            if row.kind != StateKind::Final
                || doc.states[usize::from(parent)].kind != StateKind::Compound
            {
                return None;
            }
            let done = format!("done.state.{}", doc.states[usize::from(parent)].id);
            names
                .iter()
                .position(|name| *name == done)
                .map(|event| event as u16)
        })
        .collect()
}

#[test]
fn every_document_of_the_class_runs_on_the_class_walker_as_on_the_general_interpreter() {
    let corpus = common::repository::files_git_tracks(&["*.scxml"]);
    let (mut documents, mut consumer_shaped) = (0usize, 0usize);
    let (mut microsteps, mut guards, mut raised, mut ended) = (0usize, 0usize, 0usize, 0usize);
    let (mut reactive_documents, mut reactive_runs) = (0usize, 0usize);
    for path in &corpus {
        let Some(model) = runtime_tables::generatable(path) else {
            continue;
        };
        let name = path.display().to_string();
        let doc = DocumentTables::of(&model)
            .unwrap_or_else(|refusal| panic!("{name}: no tables: {refusal}"));
        let Some(paths) = doc.entry_paths() else {
            continue;
        };
        let names = runtime_tables::event_names(&model);
        let selection = runtime_tables::selection(&doc, &model, &names);
        let null_event = u16::try_from(names.len()).expect("a machine numbers its events in a u16");
        let general = runtime_tables::general(&doc, Some(&selection), null_event);
        let class = runtime_tables::row_walk(&doc, &paths, &selection, null_event);
        assert_eq!(general.check(), Ok(()), "{name}: the general walk's tables");
        assert_eq!(class.check(), Ok(()), "{name}: the class walker's tables");
        // docs/adr/0148: the tables' answer is the document's, read here from
        // the model: no eventless transition, and every <final> a child of the
        // <scxml> element (the row walk is the class already).
        let reactive = class.reactive_class();
        let by_model = model.states.values().all(|state| {
            state.transitions.iter().all(|t| !t.event.trim().is_empty())
                && (!state.is_final || state.parent.is_none())
        });
        assert_eq!(reactive, by_model, "{name}: the tables' reactive class");
        reactive_documents += usize::from(reactive);
        let raises = done_state_events(&doc, &names);
        raised += raises.iter().flatten().count();
        for seed in 1..=SEEDS {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let hooks = Hooks {
                guarded: (0..doc.transitions.len())
                    .map(|_| rng.below(3) == 0)
                    .collect(),
                raises: raises.clone(),
                seed: rng.draw(),
            };
            let events: Vec<u16> = if names.is_empty() {
                Vec::new()
            } else {
                (0..EVENTS).map(|_| rng.below(names.len()) as u16).collect()
            };
            let on_general = run(&general, &doc, &hooks, &events, false);
            let on_class = run(&class, &doc, &hooks, &events, false);
            assert_eq!(
                on_class, on_general,
                "{name}, seed {seed}: the class walker and the general interpreter \
                 disagree over the events {events:?}"
            );
            // docs/adr/0148: a document of the reactive class runs on the
            // reactive turn as Appendix D's loop runs it.
            if reactive {
                let on_reactive = run(&class, &doc, &hooks, &events, true);
                assert_eq!(
                    on_reactive, on_general,
                    "{name}, seed {seed}: the reactive turn and the general loop \
                     disagree over the events {events:?}"
                );
                reactive_runs += 1;
            }
            microsteps += on_general.steps.len();
            guards += on_general
                .journal
                .iter()
                .filter(|call| matches!(call, Call::Guard(..)))
                .count();
            ended += usize::from(on_general.ended);
        }
        documents += 1;
        if name.contains("tests/integration/static_class/") {
            consumer_shaped += 1;
        }
    }
    // The two consumer-shaped documents were run, and the class was run over
    // real documents and not over none: many documents, many steps, guards
    // asked, done.state events raised and runs that ended.
    assert_eq!(
        consumer_shaped, 2,
        "the documents of tests/integration/static_class"
    );
    assert!(
        documents >= 50,
        "{documents} documents of the class were run"
    );
    assert!(microsteps >= 5000, "{microsteps} steps were taken");
    assert!(guards >= 1000, "{guards} guards were asked");
    assert!(raised >= 1, "{raised} states raise done.state on entry");
    assert!(ended >= 10, "{ended} runs ended in a top-level <final>");
    assert!(corpus.len() >= 1000, "walked {} documents", corpus.len());
    // The reactive turn was held to the general loop over real documents.
    assert!(
        reactive_documents >= 10 && reactive_runs >= 60,
        "{reactive_documents} documents of the reactive class, {reactive_runs} runs"
    );
}
