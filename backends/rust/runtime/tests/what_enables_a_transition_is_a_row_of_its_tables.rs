// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0127: what enables a transition is a row of its document's tables,
// and one selection reads it for every machine.
//
// A generated machine's tables carry, for each state, the range of its written
// transitions and, for each transition, the events that enable it as numbers
// (`SelectionRows`). `DocumentTables::first_enabled` walks the range and asks
// the machine `Behaviour::guard_holds` only for a transition the event
// enables. A machine whose tables carry no rows answers the selection itself
// (`Behaviour::first_enabled_transition`), as a generated machine did before
// the rows.
//
// The cases here write random documents twice: once as tables with no rows and
// a machine that answers the selection the way the generated
// `first_enabled_transition` did (each of the state's transitions in the order
// written, the event compared first and the guard only then, an eventless
// transition only for the null event), and once as tables with the rows. For
// every state and every event the two answer the same transition and evaluate
// the same guards in the same order, and Appendix D's selectTransitions over a
// configuration answers the same set. What `DocumentTables::check` refuses of
// the rows is asked of hand-made faults.
//
// docs/adr/0135: a document with no `<history>`, no `<parallel>` and no
// transition of more than one target is served by the row walk, which selects
// from candidate rows: for each atomic state and event, the transitions the
// selection tries, the state's own as written then each ancestor's. Each random
// document is written a third time as such tables, and from every atomic state
// and for every event the row walk selects what the machine's own selection
// selects and evaluates the same guards in the same order. What the check
// refuses of the candidate rows is asked of hand-made faults too.

use std::collections::BTreeMap;

use sce_rust_runtime::helpers::microstep::{select_transitions, Behaviour, Buffers};
use sce_rust_runtime::helpers::run_store::RunStoreMut;
use sce_rust_runtime::helpers::state_set::Word;
use sce_rust_runtime::helpers::tables::TablesFault;
use sce_rust_runtime::{
    state_words, Candidate, CandidateCell, CandidateRows, DocumentTables, EntryRow, EntryStep,
    EventId, Named, SelectionRows, StateId, StateKind, StateRow, Target, TransitionId,
    TransitionRow, TriggerEvent, TriggerRow, Workspace, GENERAL_WALK, ROW_WALK,
};

/// Lent for the life of the test process: the tables hand out `&'static`
/// slices, as a generated machine's are.
fn leak<T>(items: Vec<T>) -> &'static [T] {
    Box::leak(items.into_boxed_slice())
}

/// A small xorshift generator: the documents are a function of the seed alone,
/// so a failure names the seed that reproduces it.
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

    fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

/// What a written transition answers, as a document writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Answers {
    /// An eventless transition: only the selection for the null event.
    Eventless,
    /// The named events its descriptor matches, by number, which may be none.
    Events(Vec<u16>),
}

/// One written transition: its source, what it answers, and whether its guard
/// holds (`None`: it has no guard).
#[derive(Debug, Clone)]
struct Written {
    source: StateId,
    answers: Answers,
    guard: Option<bool>,
}

/// A random document: a forest of compound, atomic and final states numbered in
/// document order, its written transitions grouped by source in state order and
/// within a source in the order written, and `events` named events, the null
/// event numbered after them.
#[derive(Debug)]
struct Document {
    parent: Vec<Option<StateId>>,
    kind: Vec<StateKind>,
    written: Vec<Written>,
    events: u16,
}

impl Document {
    fn null_event(&self) -> EventId {
        EventId(self.events)
    }

    /// Whether `event` enables written transition `index`, as the generated
    /// `first_enabled_transition` compared it: an eventless transition only
    /// for the null event, a named one only for an event its descriptor
    /// matches, never the null event.
    fn enables(&self, index: usize, event: EventId) -> bool {
        match &self.written[index].answers {
            Answers::Eventless => event == self.null_event(),
            Answers::Events(events) => events.contains(&event.0),
        }
    }
}

fn random_document(rng: &mut Rng) -> Document {
    let n = 1 + rng.below(10);
    // Drawn as a pre-order walk: each state's parent is one of the states on
    // the path from the root to the state before it, which numbers the tree in
    // document order.
    let mut parent: Vec<Option<StateId>> = Vec::with_capacity(n);
    let mut path: Vec<StateId> = Vec::new();
    for index in 0..n {
        let state = StateId(index as u16);
        if index > 0 && !rng.chance(4) {
            path.truncate(1 + rng.below(path.len()));
        } else {
            path.clear();
        }
        parent.push(path.last().copied());
        path.push(state);
    }
    let kind: Vec<StateKind> = (0..n)
        .map(|index| {
            let has_children = parent.contains(&Some(StateId(index as u16)));
            if has_children {
                StateKind::Compound
            } else if rng.chance(4) {
                StateKind::Final
            } else {
                StateKind::Atomic
            }
        })
        .collect();
    let events = 1 + rng.below(6) as u16;
    let mut written = Vec::new();
    for index in 0..n {
        for _ in 0..rng.below(4) {
            let answers = if rng.chance(4) {
                Answers::Eventless
            } else {
                // Ascending, as a list of the rows is.
                Answers::Events((0..events).filter(|_| rng.chance(3)).collect())
            };
            let guard = match rng.below(3) {
                0 => None,
                1 => Some(true),
                _ => Some(false),
            };
            written.push(Written {
                source: StateId(index as u16),
                answers,
                guard,
            });
        }
    }
    Document {
        parent,
        kind,
        written,
        events,
    }
}

/// The tables of `doc`, in the layout the generator writes: the written
/// transitions first, then one initial transition for each compound state,
/// each to its first child, and the document's initial transition to state 0
/// last. With `rows`, the selection rows the generator writes beside them:
/// each state's first written transition and one past the last, and each
/// written transition's list of events, ascending, each distinct list once.
fn tables_of(doc: &Document, rows: bool) -> &'static DocumentTables<'static> {
    let n = doc.parent.len();
    let mut last: Vec<StateId> = (0..n).map(|state| StateId(state as u16)).collect();
    for state in (0..n).rev() {
        if let Some(parent) = doc.parent[state] {
            last[parent.index()] = last[parent.index()].max(last[state]);
        }
    }
    let mut transitions: Vec<TransitionRow> = doc
        .written
        .iter()
        .map(|written| {
            TransitionRow::new(
                Some(written.source),
                Target::state(written.source),
                false,
                false,
            )
        })
        .collect();
    let mut initial = vec![None; n];
    for (state, kind) in doc.kind.iter().enumerate() {
        if *kind == StateKind::Compound {
            initial[state] = Some(TransitionId(transitions.len() as u16));
            let child = StateId(state as u16 + 1);
            transitions.push(TransitionRow::new(
                Some(StateId(state as u16)),
                Target::state(child),
                false,
                false,
            ));
        }
    }
    let document_initial = TransitionId(transitions.len() as u16);
    transitions.push(TransitionRow::new(
        None,
        Target::state(StateId(0)),
        false,
        false,
    ));
    let states: Vec<StateRow> = (0..n)
        .map(|state| {
            StateRow::new(
                doc.parent[state],
                doc.kind[state],
                last[state],
                initial[state],
            )
        })
        .collect();

    let selection = rows.then(|| {
        let mut first_written = Vec::with_capacity(n + 1);
        let mut number = 0usize;
        for state in 0..n {
            while number < doc.written.len() && doc.written[number].source.index() < state {
                number += 1;
            }
            first_written.push(TransitionId(number as u16));
        }
        first_written.push(TransitionId(doc.written.len() as u16));
        let mut pool: Vec<TriggerEvent> = Vec::new();
        let mut positions: BTreeMap<Vec<u16>, u16> = BTreeMap::new();
        let mut triggers: Vec<TriggerRow> = doc
            .written
            .iter()
            .map(|written| {
                let list = match &written.answers {
                    Answers::Eventless => vec![doc.events],
                    Answers::Events(events) => events.clone(),
                };
                if list.is_empty() {
                    return TriggerRow::NONE;
                }
                let position = *positions.entry(list.clone()).or_insert_with(|| {
                    let position = pool.len() as u16;
                    pool.push(TriggerEvent::count(list.len() as u16));
                    pool.extend(list.iter().map(|&event| TriggerEvent::new(EventId(event))));
                    position
                });
                TriggerRow::at(position)
            })
            .collect();
        triggers.resize(transitions.len(), TriggerRow::NONE);
        SelectionRows {
            first_written: leak(first_written),
            triggers: leak(triggers),
            trigger_events: leak(pool),
        }
    });

    let tables = DocumentTables {
        states: leak(states),
        histories: &[],
        transitions: leak(transitions),
        targets: &[],
        initial: document_initial,
        default_state: StateId(0),
        null_event: doc.null_event(),
        error_events: &[],
        walk: &GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
        candidates: None,
        selection,
    };
    assert_eq!(
        tables.check(),
        Ok(()),
        "the test wrote tables that break the rules `helpers::tables` states"
    );
    Box::leak(Box::new(tables))
}

/// The tables of `doc` for the row walk (docs/adr/0135): the rows of
/// [`tables_of`], each transition's entry path, and the candidate rows in
/// place of the selection rows. Every written transition targets its own
/// source and is external, so its domain is the source's parent and it enters
/// the source again and, down each compound state's initial transition to its
/// first child, an atomic state, each compound state of the path by default;
/// the document's initial transition enters state 0 the same way. The
/// candidate cell of an atomic state and an event lists the written
/// transitions the event enables, the state's own as written then each
/// ancestor's, outward: computed here from the document, not from the rows.
fn class_tables_of(doc: &Document) -> &'static DocumentTables<'static> {
    let general = tables_of(doc, false);
    let mut paths: Vec<EntryStep> = Vec::new();
    let mut rows: Vec<EntryRow> = Vec::new();
    for index in 0..general.transitions.len() {
        let transition = TransitionId(index as u16);
        let initial = general
            .states
            .iter()
            .any(|row| row.initial() == Some(transition));
        let target = general
            .targets(transition)
            .first()
            .map(|target| match target.named() {
                Named::State(state) => state,
                Named::History(_) => unreachable!("the documents have no <history>"),
            });
        let Some(target) = target.filter(|_| !initial) else {
            rows.push(EntryRow::NONE);
            continue;
        };
        let mut path = vec![target];
        let mut last = target;
        while doc.kind[last.index()] == StateKind::Compound {
            last = StateId(last.0 + 1);
            path.push(last);
        }
        rows.push(EntryRow::at(paths.len() as u16));
        paths.push(EntryStep::count(path.len() as u16));
        paths.extend(
            path.iter().map(|&state| {
                EntryStep::new(state, doc.kind[state.index()] == StateKind::Compound)
            }),
        );
    }

    let mut first_cell: Vec<u16> = Vec::with_capacity(doc.parent.len() + 1);
    let mut cells: Vec<CandidateCell> = Vec::new();
    let mut pool: Vec<Candidate> = Vec::new();
    for (state, kind) in doc.kind.iter().enumerate() {
        first_cell.push(cells.len() as u16);
        if *kind == StateKind::Compound {
            continue;
        }
        let mut chain = vec![StateId(state as u16)];
        while let Some(parent) = doc.parent[chain[chain.len() - 1].index()] {
            chain.push(parent);
        }
        for event in 0..=doc.events {
            let event = EventId(event);
            let list: Vec<usize> = chain
                .iter()
                .flat_map(|&source| {
                    (0..doc.written.len()).filter(move |&index| doc.written[index].source == source)
                })
                .filter(|&index| doc.enables(index, event))
                .collect();
            if list.is_empty() {
                continue;
            }
            cells.push(CandidateCell::new(event, pool.len() as u16));
            pool.push(Candidate::count(list.len() as u16));
            pool.extend(
                list.iter()
                    .map(|&index| Candidate::new(TransitionId(index as u16))),
            );
        }
    }
    first_cell.push(cells.len() as u16);

    let tables = DocumentTables {
        walk: &ROW_WALK,
        entry_rows: leak(rows),
        entry_paths: leak(paths),
        selection: None,
        candidates: Some(CandidateRows {
            first_cell: leak(first_cell),
            cells: leak(cells),
            candidates: leak(pool),
        }),
        ..*general
    };
    assert_eq!(
        tables.check(),
        Ok(()),
        "the test wrote candidate rows that break the rules `helpers::tables` states"
    );
    Box::leak(Box::new(tables))
}

/// The machine the selection drives: its run store, and the guards it was
/// asked to evaluate, in order. `by_hand` says whether it answers the
/// selection itself, as a machine whose tables carry no rows does.
struct Machine<'d> {
    doc: &'d Document,
    by_hand: bool,
    configuration: Vec<Word>,
    /// No `<history>`, so nothing is recorded.
    records: Vec<Word>,
    recorded: Vec<bool>,
    guards: Vec<TransitionId>,
}

impl<'d> Machine<'d> {
    fn new(doc: &'d Document, by_hand: bool) -> Self {
        Machine {
            doc,
            by_hand,
            configuration: vec![0; state_words(doc.parent.len())],
            records: Vec::new(),
            recorded: Vec::new(),
            guards: Vec::new(),
        }
    }

    /// Evaluate the guard of written transition `transition`, and note it.
    fn evaluate(&mut self, transition: TransitionId) -> bool {
        match self.doc.written[transition.index()].guard {
            None => true,
            Some(holds) => {
                self.guards.push(transition);
                holds
            }
        }
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

    // The selection the generated `first_enabled_transition` made: the
    // state's transitions in the order written, the event compared first and
    // the guard only for a transition the event enables.
    fn first_enabled_transition(&mut self, state: StateId, event: EventId) -> Option<TransitionId> {
        assert!(
            self.by_hand,
            "tables with selection rows never ask the machine to select"
        );
        let doc = self.doc;
        for (index, written) in doc.written.iter().enumerate() {
            if written.source != state || !doc.enables(index, event) {
                continue;
            }
            let transition = TransitionId(index as u16);
            if self.evaluate(transition) {
                return Some(transition);
            }
        }
        None
    }

    fn guard_holds(&mut self, transition: TransitionId) -> bool {
        assert!(
            !self.by_hand,
            "tables with no selection rows never ask whether a guard holds"
        );
        self.evaluate(transition)
    }

    fn exit_state(&mut self, _state: StateId) {}

    fn execute_transition_content(&mut self, _transition: TransitionId) {}

    fn enter_state(&mut self, _state: StateId, _is_default_entry: bool) {}

    fn execute_history_default_content(&mut self, _history: sce_rust_runtime::HistoryId) {}
}

#[test]
fn the_rows_select_what_the_machines_own_selection_selects_for_any_document() {
    let (mut found, mut guarded, mut eventless, mut refused_by_a_guard) = (0, 0, 0, 0);
    let mut class_selections = 0usize;
    for seed in 1..=3000u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let doc = random_document(&mut rng);
        let by_hand = tables_of(&doc, false);
        let rows = tables_of(&doc, true);
        let class = class_tables_of(&doc);
        assert!(by_hand.selection.is_none() && rows.selection.is_some());
        assert!(class.selection.is_none() && class.candidates.is_some());

        // Every state, every event and the null event: the same transition, and
        // the same guards evaluated in the same order.
        for state in 0..doc.parent.len() {
            let state = StateId(state as u16);
            for event in 0..=doc.events {
                let event = EventId(event);
                let mut reference = Machine::new(&doc, true);
                let mut machine = Machine::new(&doc, false);
                let expected = by_hand.first_enabled(state, event, &mut reference);
                let answered = rows.first_enabled(state, event, &mut machine);
                assert_eq!(
                    answered, expected,
                    "seed {seed}: state {state:?}, event {event:?}, {doc:?}"
                );
                assert_eq!(
                    machine.guards, reference.guards,
                    "seed {seed}: the guards evaluated for state {state:?}, event {event:?}"
                );
                found += usize::from(expected.is_some());
                guarded += machine.guards.len();
                if expected.is_some() && event == doc.null_event() {
                    eventless += 1;
                }
                if expected.is_none() && !machine.guards.is_empty() {
                    refused_by_a_guard += 1;
                }
            }
        }

        // Appendix D's selectTransitions over one path down the tree, from
        // each atomic state: the same set.
        for (leaf, kind) in doc.kind.iter().enumerate() {
            if *kind == StateKind::Compound {
                continue;
            }
            let mut active = vec![StateId(leaf as u16)];
            while let Some(parent) = doc.parent[active[active.len() - 1].index()] {
                active.push(parent);
            }
            for event in 0..=doc.events {
                let event = EventId(event);
                let mut answers: Vec<(Vec<TransitionId>, Vec<TransitionId>)> = Vec::new();
                for (tables, answers_itself) in [(by_hand, true), (rows, false), (class, false)] {
                    let mut machine = Machine::new(&doc, answers_itself);
                    machine.store().set_configuration(active.iter().copied());
                    let mut workspace = Workspace::<0, 4, 0, 0>::new();
                    let (mut enabled, scratch) = workspace.split();
                    select_transitions(tables, &mut machine, event, &mut enabled, scratch);
                    answers.push((enabled.to_vec(), machine.guards));
                }
                assert_eq!(
                    answers[1].0, answers[0].0,
                    "seed {seed}: selectTransitions in {active:?} for {event:?}"
                );
                // docs/adr/0135: the row walk's selection, from the candidate
                // rows, selects the same transition and evaluates the same
                // guards in the same order.
                assert_eq!(
                    answers[2], answers[0],
                    "seed {seed}: the row walk's selection in {active:?} for {event:?}"
                );
                class_selections += usize::from(!answers[2].0.is_empty());
            }
        }
    }
    // The cases asked real questions: transitions found, guards run, eventless
    // transitions taken, and selections a failing guard left empty.
    assert!(found >= 2000, "{found} selections found a transition");
    assert!(guarded >= 2000, "{guarded} guards were evaluated");
    assert!(
        eventless >= 300,
        "{eventless} eventless selections found one"
    );
    assert!(
        refused_by_a_guard >= 300,
        "{refused_by_a_guard} selections ran a guard and found none"
    );
    assert!(
        class_selections >= 2000,
        "{class_selections} selections of the row walk found a transition"
    );
}

/// A document of two states, `a` (0) and `b` (1), whose written transitions
/// are `a`'s on event 0 (row 0), `a`'s eventless one (row 1) and `b`'s on
/// events 0 and 1 (row 2); the document's initial transition is row 3. Three
/// named events, the null event 3.
fn two_states(selection: SelectionRows<'static>) -> DocumentTables<'static> {
    DocumentTables {
        states: TWO_STATES,
        histories: &[],
        transitions: TWO_TRANSITIONS,
        targets: &[],
        initial: TransitionId(3),
        default_state: StateId(0),
        null_event: EventId(3),
        error_events: &[],
        walk: &GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
        candidates: None,
        selection: Some(selection),
    }
}

const TWO_STATES: &[StateRow] = &[
    StateRow::new(None, StateKind::Atomic, StateId(0), None),
    StateRow::new(None, StateKind::Atomic, StateId(1), None),
];
const TWO_TRANSITIONS: &[TransitionRow] = &[
    TransitionRow::new(Some(StateId(0)), Target::state(StateId(1)), false, false),
    TransitionRow::new(Some(StateId(0)), Target::NONE, false, false),
    TransitionRow::new(Some(StateId(1)), Target::state(StateId(0)), false, false),
    TransitionRow::new(None, Target::state(StateId(0)), false, false),
];
const FIRST_WRITTEN: &[TransitionId] = &[TransitionId(0), TransitionId(2), TransitionId(3)];
const TRIGGERS: &[TriggerRow] = &[
    TriggerRow::at(0),
    TriggerRow::at(2),
    TriggerRow::at(4),
    TriggerRow::NONE,
];
const TRIGGER_EVENTS: &[TriggerEvent] = &[
    TriggerEvent::count(1),
    TriggerEvent::new(EventId(0)),
    TriggerEvent::count(1),
    TriggerEvent::new(EventId(3)),
    TriggerEvent::count(2),
    TriggerEvent::new(EventId(0)),
    TriggerEvent::new(EventId(1)),
];

#[test]
fn rows_that_keep_the_rules_are_read_back_as_written() {
    let rows = SelectionRows {
        first_written: FIRST_WRITTEN,
        triggers: TRIGGERS,
        trigger_events: TRIGGER_EVENTS,
    };
    let tables = two_states(rows);
    assert_eq!(tables.check(), Ok(()));
    let events = |transition: u16| -> Vec<EventId> {
        rows.events(TransitionId(transition))
            .iter()
            .map(|entry| entry.event())
            .collect()
    };
    assert_eq!(events(0), [EventId(0)]);
    assert_eq!(events(1), [EventId(3)]);
    assert_eq!(events(2), [EventId(0), EventId(1)]);
    assert!(
        events(3).is_empty(),
        "the document's initial transition answers none"
    );
    assert!(rows.enables(TransitionId(2), EventId(1)));
    assert!(
        !rows.enables(TransitionId(0), EventId(3)),
        "a named transition is not eventless"
    );
    assert!(
        !rows.enables(TransitionId(1), EventId(0)),
        "an eventless one answers no name"
    );
}

#[test]
fn rows_that_break_a_rule_are_refused() {
    let with = |first_written: &'static [TransitionId],
                triggers: &'static [TriggerRow],
                trigger_events: &'static [TriggerEvent]| {
        two_states(SelectionRows {
            first_written,
            triggers,
            trigger_events,
        })
        .check()
    };
    let faults: [(&str, Result<(), TablesFault>); 9] = [
        (
            "no entry past the last state",
            with(&FIRST_WRITTEN[..2], TRIGGERS, TRIGGER_EVENTS),
        ),
        (
            "a row short",
            with(FIRST_WRITTEN, &TRIGGERS[..3], TRIGGER_EVENTS),
        ),
        (
            "a range that does not start at the first row",
            with(
                leak(vec![TransitionId(1), TransitionId(2), TransitionId(3)]),
                TRIGGERS,
                TRIGGER_EVENTS,
            ),
        ),
        (
            "a range that holds another state's transition",
            with(
                leak(vec![TransitionId(0), TransitionId(1), TransitionId(3)]),
                TRIGGERS,
                TRIGGER_EVENTS,
            ),
        ),
        (
            "a list for a transition no selection returns",
            with(
                FIRST_WRITTEN,
                leak(vec![
                    TriggerRow::at(0),
                    TriggerRow::at(2),
                    TriggerRow::at(4),
                    TriggerRow::at(0),
                ]),
                TRIGGER_EVENTS,
            ),
        ),
        (
            "a list past the pool",
            with(
                FIRST_WRITTEN,
                leak(vec![
                    TriggerRow::at(0),
                    TriggerRow::at(2),
                    TriggerRow::at(6),
                    TriggerRow::NONE,
                ]),
                TRIGGER_EVENTS,
            ),
        ),
        (
            "a list out of order",
            with(
                FIRST_WRITTEN,
                TRIGGERS,
                leak(vec![
                    TriggerEvent::count(1),
                    TriggerEvent::new(EventId(0)),
                    TriggerEvent::count(1),
                    TriggerEvent::new(EventId(3)),
                    TriggerEvent::count(2),
                    TriggerEvent::new(EventId(1)),
                    TriggerEvent::new(EventId(0)),
                ]),
            ),
        ),
        (
            "an event past the null event",
            with(
                FIRST_WRITTEN,
                TRIGGERS,
                leak(vec![
                    TriggerEvent::count(1),
                    TriggerEvent::new(EventId(4)),
                    TriggerEvent::count(1),
                    TriggerEvent::new(EventId(3)),
                    TriggerEvent::count(2),
                    TriggerEvent::new(EventId(0)),
                    TriggerEvent::new(EventId(1)),
                ]),
            ),
        ),
        (
            "the null event beside a named one",
            with(
                FIRST_WRITTEN,
                TRIGGERS,
                leak(vec![
                    TriggerEvent::count(1),
                    TriggerEvent::new(EventId(0)),
                    TriggerEvent::count(1),
                    TriggerEvent::new(EventId(3)),
                    TriggerEvent::count(2),
                    TriggerEvent::new(EventId(0)),
                    TriggerEvent::new(EventId(3)),
                ]),
            ),
        ),
    ];
    let expected = [
        Err(TablesFault::Selection),
        Err(TablesFault::Selection),
        Err(TablesFault::Selection),
        Err(TablesFault::Selection),
        Err(TablesFault::Triggers(TransitionId(3))),
        Err(TablesFault::Triggers(TransitionId(2))),
        Err(TablesFault::Triggers(TransitionId(2))),
        Err(TablesFault::Triggers(TransitionId(0))),
        Err(TablesFault::Triggers(TransitionId(2))),
    ];
    for ((why, answer), expected) in faults.into_iter().zip(expected) {
        assert_eq!(answer, expected, "{why}");
    }
    let empty = with(
        FIRST_WRITTEN,
        leak(vec![
            TriggerRow::at(0),
            TriggerRow::at(2),
            TriggerRow::at(7),
            TriggerRow::NONE,
        ]),
        leak(vec![
            TriggerEvent::count(1),
            TriggerEvent::new(EventId(0)),
            TriggerEvent::count(1),
            TriggerEvent::new(EventId(3)),
            TriggerEvent::count(2),
            TriggerEvent::new(EventId(0)),
            TriggerEvent::new(EventId(1)),
            TriggerEvent::count(0),
        ]),
    );
    assert_eq!(
        empty,
        Err(TablesFault::Triggers(TransitionId(2))),
        "an empty list, which is a transition no selection returns"
    );
}

/// `two_states` for the row walk (docs/adr/0135): the transitions enter one
/// path each (`a`'s `go` enters `b`, `b`'s transition and the document's
/// initial one enter `a`, the eventless one enters nothing), and the
/// candidate rows in place of the selection rows.
fn two_states_on_rows(candidates: Option<CandidateRows<'static>>) -> DocumentTables<'static> {
    DocumentTables {
        walk: &ROW_WALK,
        entry_rows: TWO_ENTRY_ROWS,
        entry_paths: TWO_ENTRY_PATHS,
        selection: None,
        candidates,
        ..two_states(SelectionRows {
            first_written: FIRST_WRITTEN,
            triggers: TRIGGERS,
            trigger_events: TRIGGER_EVENTS,
        })
    }
}

const TWO_ENTRY_ROWS: &[EntryRow] = &[
    EntryRow::at(0),
    EntryRow::NONE,
    EntryRow::at(2),
    EntryRow::at(2),
];
const TWO_ENTRY_PATHS: &[EntryStep] = &[
    EntryStep::count(1),
    EntryStep::new(StateId(1), false),
    EntryStep::count(1),
    EntryStep::new(StateId(0), false),
];
/// `a` answers event 0 with row 0 and the null event with row 1; `b` answers
/// events 0 and 1 with row 2, one list for both.
const FIRST_CELL: &[u16] = &[0, 2, 4];
const CELLS: &[CandidateCell] = &[
    CandidateCell::new(EventId(0), 0),
    CandidateCell::new(EventId(3), 2),
    CandidateCell::new(EventId(0), 4),
    CandidateCell::new(EventId(1), 4),
];
const CANDIDATES: &[Candidate] = &[
    Candidate::count(1),
    Candidate::new(TransitionId(0)),
    Candidate::count(1),
    Candidate::new(TransitionId(1)),
    Candidate::count(1),
    Candidate::new(TransitionId(2)),
];

const CANDIDATE_ROWS: CandidateRows<'static> = CandidateRows {
    first_cell: FIRST_CELL,
    cells: CELLS,
    candidates: CANDIDATES,
};

#[test]
fn candidate_rows_that_keep_the_rules_are_read_back_as_written() {
    let tables = two_states_on_rows(Some(CANDIDATE_ROWS));
    assert_eq!(tables.check(), Ok(()));
    let tried = |state: u16, event: u16| -> Vec<TransitionId> {
        tables
            .candidates_of(StateId(state), EventId(event))
            .iter()
            .map(|candidate| candidate.transition())
            .collect()
    };
    assert_eq!(tried(0, 0), [TransitionId(0)]);
    assert_eq!(tried(0, 3), [TransitionId(1)]);
    assert!(tried(0, 1).is_empty(), "`a` answers no event 1");
    assert_eq!(tried(1, 0), [TransitionId(2)]);
    assert_eq!(tried(1, 1), [TransitionId(2)]);
    assert!(tried(1, 3).is_empty(), "`b` has no eventless transition");
}

#[test]
fn candidate_rows_that_break_a_rule_are_refused() {
    let with = |first_cell: &'static [u16],
                cells: &'static [CandidateCell],
                candidates: &'static [Candidate]| {
        two_states_on_rows(Some(CandidateRows {
            first_cell,
            cells,
            candidates,
        }))
        .check()
    };
    let general_with_rows = DocumentTables {
        walk: &GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        ..two_states_on_rows(Some(CANDIDATE_ROWS))
    };
    let beside_selection_rows = DocumentTables {
        selection: Some(SelectionRows {
            first_written: FIRST_WRITTEN,
            triggers: TRIGGERS,
            trigger_events: TRIGGER_EVENTS,
        }),
        ..two_states_on_rows(Some(CANDIDATE_ROWS))
    };
    // What each case is, what `check` answers, and what it should answer.
    type Case = (
        &'static str,
        Result<(), TablesFault>,
        Result<(), TablesFault>,
    );
    let faults: [Case; 13] = [
        (
            "the general walk's tables with candidate rows",
            general_with_rows.check(),
            Err(TablesFault::Candidates),
        ),
        (
            "the row walk's tables without them",
            two_states_on_rows(None).check(),
            Err(TablesFault::Candidates),
        ),
        (
            "candidate rows beside selection rows",
            beside_selection_rows.check(),
            Err(TablesFault::Candidates),
        ),
        (
            "no entry past the last state",
            with(&FIRST_CELL[..2], CELLS, CANDIDATES),
            Err(TablesFault::Candidates),
        ),
        (
            "a first entry past the first cell",
            with(&[1, 2, 4], CELLS, CANDIDATES),
            Err(TablesFault::Candidates),
        ),
        (
            "a last entry short of the cells",
            with(&[0, 2, 3], CELLS, CANDIDATES),
            Err(TablesFault::Candidates),
        ),
        (
            "a range that falls",
            with(&[0, 5, 4], CELLS, CANDIDATES),
            Err(TablesFault::Candidates),
        ),
        (
            "cells out of the order of their events",
            with(
                FIRST_CELL,
                leak(vec![CELLS[1], CELLS[0], CELLS[2], CELLS[3]]),
                CANDIDATES,
            ),
            Err(TablesFault::CandidateCell(StateId(0))),
        ),
        (
            "an event past the null event",
            with(
                FIRST_CELL,
                leak(vec![
                    CELLS[0],
                    CandidateCell::new(EventId(4), 2),
                    CELLS[2],
                    CELLS[3],
                ]),
                CANDIDATES,
            ),
            Err(TablesFault::CandidateCell(StateId(0))),
        ),
        (
            "a list past the pool",
            with(
                FIRST_CELL,
                leak(vec![
                    CELLS[0],
                    CELLS[1],
                    CELLS[2],
                    CandidateCell::new(EventId(1), 9),
                ]),
                CANDIDATES,
            ),
            Err(TablesFault::CandidateCell(StateId(1))),
        ),
        (
            "another state's transition",
            with(
                FIRST_CELL,
                CELLS,
                leak(vec![
                    Candidate::count(1),
                    Candidate::new(TransitionId(2)),
                    Candidate::count(1),
                    Candidate::new(TransitionId(1)),
                    Candidate::count(1),
                    Candidate::new(TransitionId(2)),
                ]),
            ),
            Err(TablesFault::CandidateCell(StateId(0))),
        ),
        (
            "a state's transitions out of the order they are written",
            with(
                FIRST_CELL,
                leak(vec![
                    CandidateCell::new(EventId(0), 0),
                    CandidateCell::new(EventId(3), 3),
                    CandidateCell::new(EventId(0), 5),
                    CandidateCell::new(EventId(1), 5),
                ]),
                leak(vec![
                    Candidate::count(2),
                    Candidate::new(TransitionId(1)),
                    Candidate::new(TransitionId(0)),
                    Candidate::count(1),
                    Candidate::new(TransitionId(1)),
                    Candidate::count(1),
                    Candidate::new(TransitionId(2)),
                ]),
            ),
            Err(TablesFault::CandidateCell(StateId(0))),
        ),
        (
            "the document's initial transition, which no selection returns",
            with(
                FIRST_CELL,
                CELLS,
                leak(vec![
                    Candidate::count(1),
                    Candidate::new(TransitionId(3)),
                    Candidate::count(1),
                    Candidate::new(TransitionId(1)),
                    Candidate::count(1),
                    Candidate::new(TransitionId(2)),
                ]),
            ),
            Err(TablesFault::CandidateCell(StateId(0))),
        ),
    ];
    for (why, answer, expected) in faults {
        assert_eq!(answer, expected, "{why}");
    }
    let empty = with(
        FIRST_CELL,
        leak(vec![
            CELLS[0],
            CELLS[1],
            CELLS[2],
            CandidateCell::new(EventId(1), 6),
        ]),
        leak(vec![
            Candidate::count(1),
            Candidate::new(TransitionId(0)),
            Candidate::count(1),
            Candidate::new(TransitionId(1)),
            Candidate::count(1),
            Candidate::new(TransitionId(2)),
            Candidate::count(0),
        ]),
    );
    assert_eq!(
        empty,
        Err(TablesFault::CandidateCell(StateId(1))),
        "an empty list, which is an event that enables nothing and has no cell"
    );

    // A compound state is never a configuration's atomic state, so it has no
    // cell: `p` (0) holds `c` (1), whose transition answers event 0.
    let nested = Document {
        parent: vec![None, Some(StateId(0))],
        kind: vec![StateKind::Compound, StateKind::Atomic],
        written: vec![Written {
            source: StateId(1),
            answers: Answers::Events(vec![0]),
            guard: None,
        }],
        events: 1,
    };
    let tables = class_tables_of(&nested);
    let rows = tables
        .candidates
        .expect("the row walk's tables carry candidate rows");
    assert_eq!(rows.first_cell, [0, 0, 1]);
    let compound_with_a_cell = DocumentTables {
        candidates: Some(CandidateRows {
            first_cell: &[0, 1, 1],
            ..rows
        }),
        ..*tables
    };
    assert_eq!(
        compound_with_a_cell.check(),
        Err(TablesFault::CandidateCell(StateId(0)))
    );
}
