// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D — what a microstep selects, exits and enters, asked of
// `sce_rust_runtime::helpers::microstep` over a document written out by hand.
//
// The procedures are compiled once for every program (docs/adr/0064): a
// document reaches them as its tables (`DocumentTables`), its states,
// histories, transitions and events as numbers, its configuration and history
// records as a run store, and the running machine as one `Behaviour`. So each
// document here is written as lists, built into tables the way the generator
// builds them — states numbered in document order, a subtree the range of ids
// up to its last descendant, the initial and history default transitions rows
// of the transitions table — and run by a machine that keeps its run store in
// slices of its own and records what it was asked to do.
//
// Each case states the answer the appendix computes, worked from the
// pseudo-code, and asks the transcription for it. The entry-set and completion
// cases are the ones the C++ engines' shared transcription is held to
// (`tests/states/EntrySetAlgorithmsTest.cpp`,
// `tests/states/CompletionAlgorithmsTest.cpp`), over the same documents, so
// the two transcriptions answer one set of questions. Three answers are the
// reason the entry procedures exist and are pinned by name:
//
//   * a target SET enters every target, and a `<parallel>` region no target
//     descends into still gets its default;
//   * a compound state entered only as an ANCESTOR of a deeper target is not in
//     statesForDefaultEntry, so its initial transition content does not run;
//   * a history that recorded several regions restores all of them.
//
// <scxml initial="s">
//   <state id="s" initial="p">
//     <parallel id="p">
//       <state id="r1" initial="a1"> a1 a2 </state>
//       <state id="r2" initial="b1"> b1 b2
//         <history id="hb" type="shallow"> -> b2 </history> </state>
//       <state id="r3"> c1 c2 </state>                    (no initial: c1)
//     </parallel>
//     <state id="q" initial="q1"> q1 <final id="qf"/> </state>
//     <history id="hs" type="deep"> -> "a2 b2" </history>
//     <state id="m" initial="m1a m2b">
//       <parallel id="mp">
//         <state id="m1"> m1a m1b </state>
//         <state id="m2"> m2a m2b </state>
//       </parallel>
//     </state>
//   </state>
//   <state id="t"/>
// </scxml>

use std::mem::MaybeUninit;
use std::sync::OnceLock;

use sce_rust_runtime::helpers::microstep::{
    compute_entry_set, compute_exit_set, compute_states_to_exit, effective_target_states_on,
    is_in_final_state, microstep, select_transitions, transition_domain, Behaviour, Buffers,
    Entered, ExitScope, Scratch, SliceVec, WalkStep,
};
use sce_rust_runtime::helpers::run_store::{RunStoreMut, RunStoreRef};
use sce_rust_runtime::helpers::state_set::{StateSet, StateSetRef, Word};
use sce_rust_runtime::helpers::tables::TablesFault;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, EventId, HistoryId, HistoryRow, StateId, StateKind,
    StateRow, Target, TransitionId, TransitionRow, Workspace, HISTORY, PARALLEL,
};

// ════════════════════════════════════════════════════════════════════════
// A document as lists, and its tables
// ════════════════════════════════════════════════════════════════════════

/// A transition as written: its source (`None`: the `<scxml>` element), its
/// targets as written, and its type.
#[derive(Debug, Clone)]
struct Written {
    source: Option<StateId>,
    targets: Vec<Target>,
    internal: bool,
    has_content: bool,
}

/// A document written out as lists, its states numbered in document order: what
/// its tables are built from, and what the reference recursion reads.
#[derive(Debug)]
struct Plain {
    parent: Vec<Option<StateId>>,
    kind: Vec<StateKind>,
    /// A compound state's initial transition targets; empty for any other.
    initial: Vec<Vec<Target>>,
    history_parent: Vec<StateId>,
    history_deep: Vec<bool>,
    history_defaults: Vec<Vec<Target>>,
    /// The written transitions; each one's `TransitionId` is its position.
    written: Vec<Written>,
    /// The targets of the document's initial transition.
    document_initial: Vec<Target>,
}

/// Lent for the life of the test process: the tables hand out `&'static`
/// slices, as a generated machine's are.
fn leak<T>(items: Vec<T>) -> &'static [T] {
    Box::leak(items.into_boxed_slice())
}

/// A row's targets as the generator packs them (docs/adr/0067): the only one
/// itself, none as [`Target::NONE`], and more than one as a list put in `pool`,
/// its length first.
fn row_targets(pool: &mut Vec<Target>, targets: &[Target]) -> Target {
    match targets {
        [] => Target::NONE,
        [only] => *only,
        _ => {
            let position = pool.len() as u16;
            pool.push(Target::count(targets.len() as u16));
            pool.extend_from_slice(targets);
            Target::list(position)
        }
    }
}

/// The tables of `doc`, in the layout the generator writes: the written
/// transitions first, then one initial transition for each compound state, one
/// default transition for each `<history>`, and the document's initial
/// transition last. The procedures read a row's source and targets and nothing
/// about where it sits, and `DocumentTables::check` holds the rows to the rules
/// the procedures assume.
fn tables_of(doc: &Plain) -> &'static DocumentTables {
    let n = doc.parent.len();

    // A parent's id is below its children's, so walking down the ids from the
    // end reaches every state after its whole subtree.
    let mut last: Vec<StateId> = (0..n).map(|state| StateId(state as u16)).collect();
    for state in (0..n).rev() {
        if let Some(parent) = doc.parent[state] {
            last[parent.index()] = last[parent.index()].max(last[state]);
        }
    }

    let mut target_pool: Vec<Target> = Vec::new();
    let mut transitions: Vec<TransitionRow> = Vec::new();
    let mut push_row =
        |source: Option<StateId>, targets: &[Target], internal: bool, has_content: bool| {
            let targets = row_targets(&mut target_pool, targets);
            transitions.push(TransitionRow::new(source, targets, internal, has_content));
            TransitionId((transitions.len() - 1) as u16)
        };
    for written in &doc.written {
        push_row(
            written.source,
            &written.targets,
            written.internal,
            written.has_content,
        );
    }
    let mut initial = Vec::with_capacity(n);
    for (state, (kind, targets)) in doc.kind.iter().zip(&doc.initial).enumerate() {
        initial.push(if *kind == StateKind::Compound {
            Some(push_row(Some(StateId(state as u16)), targets, false, false))
        } else {
            None
        });
    }
    let mut histories = Vec::new();
    for (history, &parent) in doc.history_parent.iter().enumerate() {
        let default = push_row(Some(parent), &doc.history_defaults[history], false, false);
        histories.push(HistoryRow::new(parent, default, doc.history_deep[history]));
    }
    let document_initial = push_row(None, &doc.document_initial, false, false);

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
    // The procedures of a feature the document uses, and only those, as the
    // generator writes them (docs/adr/0066): a document without a `<parallel>`
    // holds the core to the reference with removeConflictingTransitions absent.
    let has_parallel = doc.kind.contains(&StateKind::Parallel);
    let tables = DocumentTables {
        history: (!histories.is_empty()).then_some(&HISTORY),
        parallel: has_parallel.then_some(&PARALLEL),
        states: leak(states),
        histories: leak(histories),
        transitions: leak(transitions),
        targets: leak(target_pool),
        initial: document_initial,
        // The procedures under test are handed the events they select by; the
        // main loop, which asks for the null event and the error events, is
        // not driven here, and no engine names a state before it starts.
        default_state: StateId(0),
        null_event: EventId(0),
        error_events: &[],
    };
    assert_eq!(
        tables.check(),
        Ok(()),
        "the test wrote tables that break the rules `helpers::tables` states"
    );
    Box::leak(Box::new(tables))
}

// ════════════════════════════════════════════════════════════════════════
// The document above
// ════════════════════════════════════════════════════════════════════════

// Declared in document order, so a discriminant IS the state's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    S,
    P,
    R1,
    A1,
    A2,
    R2,
    B1,
    B2,
    R3,
    C1,
    C2,
    Q,
    Q1,
    Qf,
    M,
    Mp,
    M1,
    M1a,
    M1b,
    M2,
    M2a,
    M2b,
    T,
}

// Declared in document order, so a discriminant IS the history's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum H {
    Hb,
    Hs,
}

/// A target as the document above writes it.
#[derive(Debug, Clone, Copy)]
enum Tg {
    S(St),
    H(H),
}

// The events the transitions below answer; `Null` is the eventless
// selection's "no event". A discriminant IS the event's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ev {
    E,
    F,
    G,
    H,
    K,
    R,
    Null,
}

/// One state of the document: itself, its parent, its kind and its initial
/// transition's targets.
type TreeRow = (St, Option<St>, StateKind, &'static [Tg]);

/// The tree above, in document order.
const TREE: [TreeRow; 23] = [
    (St::S, None, StateKind::Compound, &[Tg::S(St::P)]),
    (St::P, Some(St::S), StateKind::Parallel, &[]),
    (St::R1, Some(St::P), StateKind::Compound, &[Tg::S(St::A1)]),
    (St::A1, Some(St::R1), StateKind::Atomic, &[]),
    (St::A2, Some(St::R1), StateKind::Atomic, &[]),
    (St::R2, Some(St::P), StateKind::Compound, &[Tg::S(St::B1)]),
    (St::B1, Some(St::R2), StateKind::Atomic, &[]),
    (St::B2, Some(St::R2), StateKind::Atomic, &[]),
    (St::R3, Some(St::P), StateKind::Compound, &[Tg::S(St::C1)]),
    (St::C1, Some(St::R3), StateKind::Atomic, &[]),
    (St::C2, Some(St::R3), StateKind::Atomic, &[]),
    (St::Q, Some(St::S), StateKind::Compound, &[Tg::S(St::Q1)]),
    (St::Q1, Some(St::Q), StateKind::Atomic, &[]),
    (St::Qf, Some(St::Q), StateKind::Final, &[]),
    (
        St::M,
        Some(St::S),
        StateKind::Compound,
        &[Tg::S(St::M1a), Tg::S(St::M2b)],
    ),
    (St::Mp, Some(St::M), StateKind::Parallel, &[]),
    (St::M1, Some(St::Mp), StateKind::Compound, &[Tg::S(St::M1a)]),
    (St::M1a, Some(St::M1), StateKind::Atomic, &[]),
    (St::M1b, Some(St::M1), StateKind::Atomic, &[]),
    (St::M2, Some(St::Mp), StateKind::Compound, &[Tg::S(St::M2a)]),
    (St::M2a, Some(St::M2), StateKind::Atomic, &[]),
    (St::M2b, Some(St::M2), StateKind::Atomic, &[]),
    (St::T, None, StateKind::Atomic, &[]),
];

/// One `<history>` of the document: itself, its parent, whether it is deep, and
/// its default transition's targets.
type HistoryDecl = (H, St, bool, &'static [Tg]);

const HISTORIES: [HistoryDecl; 2] = [
    (H::Hb, St::R2, false, &[Tg::S(St::B2)]),
    (H::Hs, St::S, true, &[Tg::S(St::A2), Tg::S(St::B2)]),
];

/// One written transition: `event` `None` for a transition no event selects,
/// written for the cases that hand the procedures their transitions directly.
/// A transition an event selects has content, and the others none.
struct Tr {
    source: St,
    event: Option<Ev>,
    targets: &'static [Tg],
    internal: bool,
}

// The transitions, grouped by source in document order, each source's in the
// order written:
//
//   s:  -> q1 (internal), -> q1
//   p:  F -> t,  G -> t,  H -> t,  K (targetless)
//   r1: -> a2, -> a2 (internal)
//   a1: E -> a2, F -> a2, H (targetless), K -> a2, R -> a1 (a self-transition),
//       -> hb
//   b1: E -> b2, G -> b2, K -> b2, R -> b2
//   t:  -> "a2 b2", -> hs, -> m
static WRITTEN: [Tr; 21] = [
    Tr {
        source: St::S,
        event: None,
        targets: &[Tg::S(St::Q1)],
        internal: true,
    },
    Tr {
        source: St::S,
        event: None,
        targets: &[Tg::S(St::Q1)],
        internal: false,
    },
    Tr {
        source: St::P,
        event: Some(Ev::F),
        targets: &[Tg::S(St::T)],
        internal: false,
    },
    Tr {
        source: St::P,
        event: Some(Ev::G),
        targets: &[Tg::S(St::T)],
        internal: false,
    },
    Tr {
        source: St::P,
        event: Some(Ev::H),
        targets: &[Tg::S(St::T)],
        internal: false,
    },
    Tr {
        source: St::P,
        event: Some(Ev::K),
        targets: &[],
        internal: false,
    },
    Tr {
        source: St::R1,
        event: None,
        targets: &[Tg::S(St::A2)],
        internal: false,
    },
    Tr {
        source: St::R1,
        event: None,
        targets: &[Tg::S(St::A2)],
        internal: true,
    },
    Tr {
        source: St::A1,
        event: Some(Ev::E),
        targets: &[Tg::S(St::A2)],
        internal: false,
    },
    Tr {
        source: St::A1,
        event: Some(Ev::F),
        targets: &[Tg::S(St::A2)],
        internal: false,
    },
    Tr {
        source: St::A1,
        event: Some(Ev::H),
        targets: &[],
        internal: false,
    },
    Tr {
        source: St::A1,
        event: Some(Ev::K),
        targets: &[Tg::S(St::A2)],
        internal: false,
    },
    Tr {
        source: St::A1,
        event: Some(Ev::R),
        targets: &[Tg::S(St::A1)],
        internal: false,
    },
    Tr {
        source: St::A1,
        event: None,
        targets: &[Tg::H(H::Hb)],
        internal: false,
    },
    Tr {
        source: St::B1,
        event: Some(Ev::E),
        targets: &[Tg::S(St::B2)],
        internal: false,
    },
    Tr {
        source: St::B1,
        event: Some(Ev::G),
        targets: &[Tg::S(St::B2)],
        internal: false,
    },
    Tr {
        source: St::B1,
        event: Some(Ev::K),
        targets: &[Tg::S(St::B2)],
        internal: false,
    },
    Tr {
        source: St::B1,
        event: Some(Ev::R),
        targets: &[Tg::S(St::B2)],
        internal: false,
    },
    Tr {
        source: St::T,
        event: None,
        targets: &[Tg::S(St::A2), Tg::S(St::B2)],
        internal: false,
    },
    Tr {
        source: St::T,
        event: None,
        targets: &[Tg::H(H::Hs)],
        internal: false,
    },
    Tr {
        source: St::T,
        event: None,
        targets: &[Tg::S(St::M)],
        internal: false,
    },
];

fn id(state: St) -> StateId {
    StateId(state as u16)
}

fn st(state: StateId) -> St {
    TREE[state.index()].0
}

fn hid(history: H) -> HistoryId {
    HistoryId(history as u16)
}

fn hist(history: HistoryId) -> H {
    HISTORIES[history.index()].0
}

fn ev_id(event: Ev) -> EventId {
    EventId(event as u16)
}

fn ids(states: &[St]) -> Vec<StateId> {
    states.iter().copied().map(id).collect()
}

fn chain(states: &[StateId]) -> Vec<St> {
    states.iter().copied().map(st).collect()
}

/// The words of the set of the document's states that holds `states`.
fn set_of(states: &[St]) -> [Word; CHART_WORDS] {
    let mut words = [0; CHART_WORDS];
    let mut set = StateSet::new(&mut words);
    for &state in states {
        set.insert(id(state));
    }
    words
}

fn sorted(mut states: Vec<St>) -> Vec<St> {
    states.sort_by_key(|&s| s as u32);
    states
}

fn to_targets(list: &[Tg]) -> Vec<Target> {
    list.iter()
        .map(|&target| match target {
            Tg::S(state) => Target::state(id(state)),
            Tg::H(history) => Target::history(hid(history)),
        })
        .collect()
}

fn chart_plain() -> Plain {
    for (index, &(state, ..)) in TREE.iter().enumerate() {
        assert_eq!(id(state).index(), index, "TREE is in document order");
    }
    for (index, &(history, ..)) in HISTORIES.iter().enumerate() {
        assert_eq!(
            hid(history).index(),
            index,
            "HISTORIES is in document order"
        );
    }
    Plain {
        parent: TREE.iter().map(|&(_, parent, ..)| parent.map(id)).collect(),
        kind: TREE.iter().map(|&(_, _, kind, _)| kind).collect(),
        initial: TREE
            .iter()
            .map(|&(_, _, _, initial)| to_targets(initial))
            .collect(),
        history_parent: HISTORIES
            .iter()
            .map(|&(_, parent, ..)| id(parent))
            .collect(),
        history_deep: HISTORIES.iter().map(|&(_, _, deep, _)| deep).collect(),
        history_defaults: HISTORIES
            .iter()
            .map(|&(_, _, _, defaults)| to_targets(defaults))
            .collect(),
        written: WRITTEN
            .iter()
            .map(|t| Written {
                source: Some(id(t.source)),
                targets: to_targets(t.targets),
                internal: t.internal,
                has_content: t.event.is_some(),
            })
            .collect(),
        document_initial: vec![Target::state(id(St::S))],
    }
}

/// The document's tables.
fn chart_tables() -> &'static DocumentTables {
    static TABLES: OnceLock<&'static DocumentTables> = OnceLock::new();
    TABLES.get_or_init(|| tables_of(&chart_plain()))
}

/// Tables that name a `<history>` or a `<parallel>` without carrying its
/// procedures are refused (docs/adr/0066): the core reaches a feature only
/// through them, and the generator writes them exactly for the documents that
/// use the feature.
#[test]
fn tables_that_use_a_feature_without_its_procedures_are_refused() {
    let tables = chart_tables();
    assert!(
        tables.history.is_some() && tables.parallel.is_some(),
        "the chart has a `<history>` and a `<parallel>`, so its tables carry both"
    );
    let without_history = DocumentTables {
        history: None,
        ..*tables
    };
    assert_eq!(without_history.check(), Err(TablesFault::ProceduresMissing));
    let without_parallel = DocumentTables {
        parallel: None,
        ..*tables
    };
    assert_eq!(
        without_parallel.check(),
        Err(TablesFault::ProceduresMissing)
    );
}

/// A row whose list of targets runs past the pool is refused, and its rows
/// read back what they were given (docs/adr/0067): the flags and the kind ride
/// in the bits above an id, and reading one must not disturb the other.
#[test]
fn a_row_reads_back_what_it_holds_and_a_list_past_the_pool_is_refused() {
    let state = StateRow::new(
        Some(StateId(0)),
        StateKind::Final,
        StateId(5),
        Some(TransitionId(7)),
    );
    assert_eq!(state.parent(), Some(StateId(0)));
    assert_eq!(state.kind(), StateKind::Final);
    assert_eq!(state.last_descendant(), StateId(5));
    assert_eq!(state.initial(), Some(TransitionId(7)));
    let transition = TransitionRow::new(None, Target::NONE, true, false);
    assert_eq!(transition.source(), None);
    assert!(transition.internal() && !transition.has_content());
    let history = HistoryRow::new(StateId(3), TransitionId(2), true);
    assert_eq!(
        (history.parent(), history.default(), history.deep()),
        (StateId(3), TransitionId(2), true)
    );

    // One atomic state; the document's initial transition names a list of
    // two targets the pool only claims to hold.
    const TABLES: DocumentTables = DocumentTables {
        states: &[StateRow::new(None, StateKind::Atomic, StateId(0), None)],
        histories: &[],
        transitions: &[TransitionRow::new(None, Target::list(0), false, false)],
        targets: &[Target::count(2), Target::state(StateId(0))],
        initial: TransitionId(0),
        default_state: StateId(0),
        null_event: EventId(0),
        error_events: &[],
        history: None,
        parallel: None,
    };
    assert_eq!(TABLES.check(), Err(TablesFault::ListOutOfRange));
}

/// The words of one set of the document's states.
const CHART_WORDS: usize = state_words(TREE.len());

/// The buffers the generator would size for the document. What the tree above
/// bounds: sets of its 23 states; a selection takes one transition from each
/// atomic state of one configuration (3, under `p`); the deepest state, `m1a`,
/// is at level 5; two parents have a `<history>`, so the walks owe three steps
/// for each of the five levels, one for each history and three of headroom.
type ChartWorkspace = Workspace<{ state_scratch(TREE.len()) }, 3, 20, 2>;

/// The `index`th transition `source` writes.
fn row(source: St, index: usize) -> TransitionId {
    let position = WRITTEN
        .iter()
        .enumerate()
        .filter(|(_, t)| t.source == source)
        .nth(index)
        .map(|(position, _)| position)
        .expect("the document writes that transition");
    TransitionId(position as u16)
}

/// A written transition as its source and its index among the source's.
fn label(transition: TransitionId) -> (St, usize) {
    let source = WRITTEN[transition.index()].source;
    let index = WRITTEN[..transition.index()]
        .iter()
        .filter(|t| t.source == source)
        .count();
    (source, index)
}

fn sources(transitions: &[TransitionId]) -> Vec<(St, usize)> {
    transitions.iter().copied().map(label).collect()
}

/// The document above, running: its run store — a configuration and what its
/// histories recorded, in slices of its own — and a record of what the
/// procedures asked the machine to do.
struct Chart {
    configuration: [Word; CHART_WORDS],
    records: [Word; CHART_WORDS * 2],
    recorded: [bool; 2],
    log: Vec<String>,
    /// For each exit, whether the state was still active when it ran.
    exited_while_active: Vec<bool>,
    /// What each history held when the microstep's first exit ran, by
    /// [`H`]; `None` until a state is exited.
    recorded_at_first_exit: Option<[Option<Vec<St>>; 2]>,
}

impl Default for Chart {
    fn default() -> Self {
        Chart {
            configuration: [0; CHART_WORDS],
            records: [0; CHART_WORDS * 2],
            recorded: [false; 2],
            log: Vec::new(),
            exited_while_active: Vec::new(),
            recorded_at_first_exit: None,
        }
    }
}

impl Chart {
    /// The document's initial configuration, entered by hand.
    fn at_initial_configuration() -> Self {
        Chart::at(&[St::S, St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1])
    }

    fn at(states: &[St]) -> Self {
        let mut chart = Chart::default();
        chart.set_configuration(states);
        chart
    }

    fn set_configuration(&mut self, states: &[St]) {
        self.store().set_configuration(ids(states));
    }

    /// The machine, with `history` having recorded `states`.
    fn recording(mut self, history: H, states: &[St]) -> Self {
        let mut store = self.store();
        let (_, mut record) = store.record_history(hid(history));
        for state in ids(states) {
            record.insert(state);
        }
        self
    }

    fn view(&self) -> RunStoreRef<'_> {
        RunStoreRef::new(&self.configuration, &self.records, &self.recorded)
    }

    /// The active states, in document order.
    fn configuration(&self) -> Vec<StateId> {
        self.view().configuration().iter().collect()
    }

    /// What `history` recorded, in document order; `None` before it ever did.
    fn recorded(&self, history: H) -> Option<Vec<St>> {
        self.view()
            .history_value(hid(history))
            .map(|recorded| recorded.iter().map(st).collect())
    }
}

impl Behaviour for Chart {
    fn store(&mut self) -> RunStoreMut<'_> {
        RunStoreMut::new(
            &mut self.configuration,
            &mut self.records,
            &mut self.recorded,
        )
    }

    fn first_enabled_transition(&mut self, state: StateId, event: EventId) -> Option<TransitionId> {
        WRITTEN
            .iter()
            .position(|t| id(t.source) == state && t.event.map(ev_id) == Some(event))
            .map(|position| TransitionId(position as u16))
    }

    fn exit_state(&mut self, state: StateId) {
        if self.recorded_at_first_exit.is_none() {
            self.recorded_at_first_exit = Some([self.recorded(H::Hb), self.recorded(H::Hs)]);
        }
        self.exited_while_active.push(self.view().is_active(state));
        self.log.push(format!("exit {:?}", st(state)));
    }

    fn execute_transition_content(&mut self, transition: TransitionId) {
        let (source, index) = label(transition);
        self.log.push(format!("content {source:?}#{index}"));
    }

    fn enter_state(&mut self, state: StateId, is_default_entry: bool) {
        let how = if is_default_entry { " (default)" } else { "" };
        self.log.push(format!("enter {:?}{how}", st(state)));
    }

    fn execute_history_default_content(&mut self, history: HistoryId) {
        self.log
            .push(format!("history default {:?}", hist(history)));
    }
}

/// What `compute_entry_set` answered, read out of the workspace it borrowed.
struct Entry {
    states_to_enter: Vec<St>,
    states_for_default_entry: Vec<St>,
    default_history_content: Vec<(St, H)>,
    /// `EntrySet::default_history_content_of`, asked of every state.
    owed: Vec<Option<H>>,
}

impl Entry {
    fn is_default_entry(&self, state: St) -> bool {
        self.states_for_default_entry.contains(&state)
    }

    fn default_history_content_of(&self, state: St) -> Option<H> {
        self.owed[id(state).index()]
    }
}

fn entry_set(chart: &Chart, transitions: &[TransitionId]) -> Entry {
    let mut workspace = ChartWorkspace::new();
    let (_, scratch) = workspace.split();
    let entry = compute_entry_set(chart_tables(), chart.view(), transitions, scratch);
    Entry {
        states_to_enter: entry.states_to_enter.as_ref().iter().map(st).collect(),
        states_for_default_entry: entry
            .states_for_default_entry
            .as_ref()
            .iter()
            .map(st)
            .collect(),
        default_history_content: entry
            .default_history_content
            .iter()
            .map(|&(parent, history)| (st(parent), hist(history)))
            .collect(),
        owed: TREE
            .iter()
            .map(|&(state, ..)| entry.default_history_content_of(id(state)).map(hist))
            .collect(),
    }
}

/// The effective targets of `list`, in document order, and the first of them
/// the list names.
fn effective(chart: &Chart, list: &[Tg]) -> (Vec<St>, Option<St>) {
    let mut workspace = ChartWorkspace::new();
    let (_, scratch) = workspace.split();
    let mut words = [0; CHART_WORDS];
    let mut out = StateSet::new(&mut words);
    let first = effective_target_states_on(
        chart_tables(),
        chart.view(),
        &to_targets(list),
        &mut out,
        scratch.walk,
    );
    (out.as_ref().iter().map(st).collect(), first.map(st))
}

/// `compute_exit_set`'s answer, and the states it exits read off the chart's
/// configuration.
fn exit_set(chart: &Chart, transition: TransitionId) -> (ExitScope, Vec<St>) {
    let mut workspace = ChartWorkspace::new();
    let (_, scratch) = workspace.split();
    let scope = compute_exit_set(chart_tables(), chart.view(), transition, scratch);
    let exited: Vec<StateId> = chart
        .configuration()
        .iter()
        .copied()
        .filter(|&state| scope.contains(chart_tables(), state))
        .collect();
    (scope, chain(&exited))
}

fn select(chart: &mut Chart, event: Ev) -> Vec<TransitionId> {
    let mut workspace = ChartWorkspace::new();
    let (mut enabled, scratch) = workspace.split();
    select_transitions(chart_tables(), chart, ev_id(event), &mut enabled, scratch);
    enabled.to_vec()
}

fn take(chart: &mut Chart, transitions: &[TransitionId]) -> Entered {
    let mut workspace = ChartWorkspace::new();
    let (_, scratch) = workspace.split();
    microstep(chart_tables(), chart, transitions, scratch)
}

// ════════════════════════════════════════════════════════════════════════
// The buffers
// ════════════════════════════════════════════════════════════════════════

#[test]
fn a_buffer_over_uninitialised_slots_holds_only_what_it_was_given() {
    let mut slots = [MaybeUninit::<TransitionId>::uninit(); 4];
    let mut buffer = SliceVec::new(&mut slots);
    assert!(buffer.is_empty());
    assert_eq!(buffer.pop(), None);

    buffer.push(TransitionId(7));
    buffer.add_once(TransitionId(3));
    buffer.add_once(TransitionId(7));
    assert_eq!(*buffer, [TransitionId(7), TransitionId(3)]);

    // Written through in place, and shown as the items it holds, not as the
    // slots behind them.
    buffer[0] = TransitionId(5);
    assert_eq!(format!("{buffer:?}"), "[TransitionId(5), TransitionId(3)]");

    assert_eq!(buffer.pop(), Some(TransitionId(3)));
    buffer.push(TransitionId(9));
    assert_eq!(*buffer, [TransitionId(5), TransitionId(9)]);
    buffer.truncate(1);
    assert_eq!(*buffer, [TransitionId(5)]);
    buffer.clear();
    assert!(buffer.is_empty());
    assert_eq!(buffer.pop(), None);
}

#[test]
#[should_panic(expected = "a microstep buffer is full")]
fn a_push_past_the_last_slot_stops_the_program() {
    let mut slots = [MaybeUninit::<TransitionId>::uninit(); 1];
    let mut buffer = SliceVec::new(&mut slots);
    buffer.push(TransitionId(1));
    buffer.push(TransitionId(2));
}

#[test]
fn the_empty_workspace_lends_empty_buffers_over_zeroed_sets() {
    let mut workspace = <ChartWorkspace as Buffers>::EMPTY;
    let (enabled, scratch) = workspace.split();
    assert!(enabled.is_empty());
    assert_eq!(scratch.sets.len(), state_scratch(TREE.len()));
    assert!(scratch.sets.iter().all(|&word| word == 0));
    assert_eq!(scratch.walk.len(), 20);
    assert_eq!(scratch.history_defaults.len(), 2);
}

// ════════════════════════════════════════════════════════════════════════
// computeEntrySet
// ════════════════════════════════════════════════════════════════════════

#[test]
fn the_initial_configuration_enters_every_region_by_default() {
    let entry = entry_set(&Chart::default(), &[chart_tables().initial]);

    assert_eq!(
        entry.states_to_enter,
        vec![St::S, St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1]
    );
    // A `<parallel>` is not compound, so it has no initial transition to run.
    assert_eq!(
        sorted(entry.states_for_default_entry),
        vec![St::S, St::R1, St::R2, St::R3]
    );
    assert!(entry.default_history_content.is_empty());
}

#[test]
fn a_target_set_enters_every_target_and_defaults_the_region_no_target_reaches() {
    let entry = entry_set(&Chart::default(), &[row(St::T, 0)]);

    assert_eq!(
        entry.states_to_enter,
        vec![St::S, St::P, St::R1, St::A2, St::R2, St::B2, St::R3, St::C1]
    );
    // `s`, `r1` and `r2` are entered as ANCESTORS of a named target, so their
    // initial transitions do not run; only `r3`, reached by nobody, defaults.
    assert_eq!(entry.states_for_default_entry, vec![St::R3]);
    assert!(!entry.is_default_entry(St::S));
    assert!(entry.is_default_entry(St::R3));
}

#[test]
fn an_unrecorded_shallow_history_takes_its_default_and_owes_its_content_to_its_parent() {
    let entry = entry_set(&Chart::default(), &[row(St::A1, 5)]);

    // The domain is `s`: the history stands for `b2`, and the least compound
    // ancestor of `a1` and `b2` walks past the `<parallel>`.
    assert_eq!(
        entry.states_to_enter,
        vec![St::P, St::R1, St::A1, St::R2, St::B2, St::R3, St::C1]
    );
    assert_eq!(
        sorted(entry.states_for_default_entry.clone()),
        vec![St::R1, St::R3]
    );
    assert_eq!(entry.default_history_content_of(St::R2), Some(H::Hb));
    assert_eq!(entry.default_history_content_of(St::P), None);
}

#[test]
fn a_recorded_shallow_history_restores_what_it_recorded_and_owes_no_content() {
    let chart = Chart::default().recording(H::Hb, &[St::B1]);
    let entry = entry_set(&chart, &[row(St::A1, 5)]);

    assert_eq!(
        entry.states_to_enter,
        vec![St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1]
    );
    assert!(entry.default_history_content.is_empty());
}

#[test]
fn a_deep_history_that_recorded_every_region_restores_all_of_them() {
    let chart = Chart::default().recording(H::Hs, &[St::A2, St::B1, St::C2]);
    let entry = entry_set(&chart, &[row(St::T, 1)]);

    assert_eq!(
        entry.states_to_enter,
        vec![St::S, St::P, St::R1, St::A2, St::R2, St::B1, St::R3, St::C2]
    );
    // Nothing is entered by default: every compound state on the way is an
    // ancestor of a recorded state.
    assert!(entry.states_for_default_entry.is_empty());
    assert!(entry.default_history_content.is_empty());
}

#[test]
fn an_unrecorded_deep_history_with_a_multi_state_default_enters_the_whole_set() {
    let entry = entry_set(&Chart::default(), &[row(St::T, 1)]);

    assert_eq!(
        entry.states_to_enter,
        vec![St::S, St::P, St::R1, St::A2, St::R2, St::B2, St::R3, St::C1]
    );
    assert_eq!(entry.states_for_default_entry, vec![St::R3]);
    assert_eq!(entry.default_history_content_of(St::S), Some(H::Hs));
}

#[test]
fn an_internal_transition_to_a_descendant_leaves_its_source_out_of_the_set() {
    let entry = entry_set(&Chart::default(), &[row(St::S, 0)]);

    assert_eq!(entry.states_to_enter, vec![St::Q, St::Q1]);
    assert!(entry.states_for_default_entry.is_empty());
}

#[test]
fn an_external_transition_to_a_descendant_reenters_its_source() {
    let entry = entry_set(&Chart::default(), &[row(St::S, 1)]);

    assert_eq!(entry.states_to_enter, vec![St::S, St::Q, St::Q1]);
    assert!(entry.states_for_default_entry.is_empty());
}

#[test]
fn an_external_transition_on_a_region_root_reenters_every_sibling_region() {
    let entry = entry_set(&Chart::default(), &[row(St::R1, 0)]);

    // A `<parallel>` is never a domain, so the domain is `s`, and the sibling
    // regions are exited and entered again at their defaults.
    assert_eq!(
        entry.states_to_enter,
        vec![St::P, St::R1, St::A2, St::R2, St::B1, St::R3, St::C1]
    );
    assert_eq!(sorted(entry.states_for_default_entry), vec![St::R2, St::R3]);
}

#[test]
fn an_internal_transition_on_a_region_root_stays_inside_its_region() {
    let entry = entry_set(&Chart::default(), &[row(St::R1, 1)]);

    assert_eq!(entry.states_to_enter, vec![St::A2]);
}

#[test]
fn transitions_of_one_microstep_enter_in_document_order() {
    // Selected second-region first, as a set is free to be; entry order is
    // document order all the same.
    let entry = entry_set(&Chart::default(), &[row(St::B1, 0), row(St::A1, 0)]);

    assert_eq!(entry.states_to_enter, vec![St::A2, St::B2]);
}

#[test]
fn a_deep_multi_target_initial_defaults_only_the_state_that_names_it() {
    let entry = entry_set(&Chart::default(), &[row(St::T, 2)]);

    assert_eq!(
        entry.states_to_enter,
        vec![St::S, St::M, St::Mp, St::M1, St::M1a, St::M2, St::M2b]
    );
    // `m` is the target and defaults; `m1` and `m2` are ancestors of its
    // initial targets, and `s` an ancestor of the target itself.
    assert_eq!(entry.states_for_default_entry, vec![St::M]);
}

#[test]
fn a_targetless_transition_enters_nothing() {
    let entry = entry_set(&Chart::default(), &[row(St::A1, 2)]);

    assert!(entry.states_to_enter.is_empty());
    assert!(entry.states_for_default_entry.is_empty());
}

#[test]
fn the_domain_is_asked_of_the_states_a_history_stands_for() {
    // Unrecorded, `hs` stands for "a2 b2", whose least compound ancestor with
    // `t` is the document itself.
    let (unrecorded, first) = effective(&Chart::default(), &[Tg::H(H::Hs)]);
    assert_eq!(unrecorded, vec![St::A2, St::B2]);
    assert_eq!(first, Some(St::A2));
    assert_eq!(
        transition_domain(
            chart_tables(),
            Some(id(St::T)),
            StateSetRef::new(&set_of(&unrecorded)),
            false
        ),
        None
    );

    // Recorded inside `q`, an internal transition on `q` keeps `q` as its
    // domain.
    let chart = Chart::default().recording(H::Hs, &[St::Q1]);
    let (recorded, _) = effective(&chart, &[Tg::H(H::Hs)]);
    assert_eq!(recorded, vec![St::Q1]);
    assert_eq!(
        transition_domain(
            chart_tables(),
            Some(id(St::Q)),
            StateSetRef::new(&set_of(&recorded)),
            true
        ),
        Some(id(St::Q))
    );
}

// ════════════════════════════════════════════════════════════════════════
// isDescendant: a subtree is a range of ids
// ════════════════════════════════════════════════════════════════════════

fn is_descendant(state: St, ancestor: St) -> bool {
    chart_tables().is_descendant(id(state), id(ancestor))
}

#[test]
fn a_child_and_a_grandchild_are_descendants() {
    assert!(is_descendant(St::A1, St::R1));
    assert!(is_descendant(St::A1, St::P));
    assert!(is_descendant(St::M2b, St::S));
}

#[test]
fn a_state_is_not_its_own_descendant() {
    // Appendix D's isDescendant is strict.
    assert!(!is_descendant(St::R1, St::R1));
}

#[test]
fn states_on_unrelated_branches_are_not_descendants() {
    assert!(!is_descendant(St::A1, St::R2));
    assert!(!is_descendant(St::Q1, St::P));
    assert!(!is_descendant(St::T, St::S));
}

// ════════════════════════════════════════════════════════════════════════
// Domains and exit sets
// ════════════════════════════════════════════════════════════════════════

#[test]
fn a_self_transition_exits_only_its_source() {
    let chart = Chart::at_initial_configuration();
    // §scxml-D-findLCCA chooses among the PROPER ancestors, so the domain of
    // `a1 -> a1` is `r1`, and the exit set is `a1` alone. A source that were
    // its own domain would exit nothing, and the self-transition would not
    // leave and re-enter its state at all.
    let (scope, exited) = exit_set(&chart, row(St::A1, 4));
    assert_eq!(scope, ExitScope::Below(id(St::R1)));
    assert_eq!(exited, vec![St::A1]);
}

#[test]
fn an_external_transition_leaving_a_parallel_exits_every_region() {
    let chart = Chart::at_initial_configuration();
    // `p -> t`: the domain is the `<scxml>` element, so the whole
    // configuration goes — the sibling regions of `p` included, which no walk
    // up from the source alone could name.
    let (scope, exited) = exit_set(&chart, row(St::P, 0));
    assert_eq!(scope, ExitScope::All);
    assert_eq!(sorted(exited), sorted(chain(&chart.configuration())));
}

#[test]
fn an_internal_transition_from_a_region_root_exits_only_inside_the_region() {
    let chart = Chart::at_initial_configuration();
    let (scope, exited) = exit_set(&chart, row(St::R1, 1));
    assert_eq!(scope, ExitScope::Below(id(St::R1)));
    assert_eq!(exited, vec![St::A1]);
    // Written external, the same transition's domain is `s` — a `<parallel>`
    // is never a domain — so every region goes with it.
    let (scope, exited) = exit_set(&chart, row(St::R1, 0));
    assert_eq!(scope, ExitScope::Below(id(St::S)));
    assert_eq!(
        sorted(exited),
        vec![St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1]
    );
}

#[test]
fn a_targetless_transition_exits_nothing() {
    let chart = Chart::at_initial_configuration();
    let (scope, exited) = exit_set(&chart, row(St::A1, 2));
    assert_eq!(scope, ExitScope::Nothing);
    assert!(exited.is_empty());
}

#[test]
fn the_states_to_exit_are_exited_in_reverse_document_order() {
    let mut chart = Chart::at_initial_configuration();
    let mut workspace = ChartWorkspace::new();
    let (_, scratch) = workspace.split();
    let mut words = [0; CHART_WORDS];
    let mut exited = StateSet::new(&mut words);
    compute_states_to_exit(
        chart_tables(),
        chart.view(),
        &[row(St::P, 0)],
        &mut exited,
        scratch,
    );
    let exited: Vec<St> = exited.as_ref().iter().map(st).collect();
    assert_eq!(
        exited,
        vec![St::S, St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1]
    );
    // exitStates reads the set backwards: descendants before their
    // ancestors, reverse document order among the rest.
    take(&mut chart, &[row(St::P, 0)]);
    let exits: Vec<&str> = chart
        .log
        .iter()
        .filter_map(|line| line.strip_prefix("exit "))
        .collect();
    assert_eq!(exits, vec!["C1", "R3", "B1", "R2", "A1", "R1", "P", "S"]);
}

// ════════════════════════════════════════════════════════════════════════
// selectTransitions / removeConflictingTransitions
// ════════════════════════════════════════════════════════════════════════

#[test]
fn every_region_takes_its_own_transition() {
    let mut chart = Chart::at_initial_configuration();
    // §scxml-3.4: two regions' transitions have domains in disjoint subtrees,
    // so their exit sets are disjoint and both survive.
    assert_eq!(
        sources(&select(&mut chart, Ev::E)),
        vec![(St::A1, 0), (St::B1, 0)]
    );
}

#[test]
fn a_transition_selected_first_preempts_a_later_one_that_is_not_its_descendant() {
    let mut chart = Chart::at_initial_configuration();
    // `a1` selects `a1 -> a2`; `b1` walks up to `p -> t`, which exits `a1`
    // too, and `p` does not descend from `a1`: it is preempted.
    assert_eq!(sources(&select(&mut chart, Ev::F)), vec![(St::A1, 1)]);
}

#[test]
fn a_descendant_source_preempts_an_ancestor_selected_before_it() {
    let mut chart = Chart::at_initial_configuration();
    // `a1` walks up to `p -> t` first; `b1` then selects `b1 -> b2`, which
    // conflicts with it and descends from `p`, so it wins. `c1` reaches
    // `p -> t` again — one transition, already in the set.
    assert_eq!(sources(&select(&mut chart, Ev::G)), vec![(St::B1, 1)]);
}

#[test]
fn a_targetless_transition_is_never_preempted() {
    let mut chart = Chart::at_initial_configuration();
    // W3C test 403c: an empty exit set conflicts with nothing, so `a1`'s
    // targetless transition survives the `p -> t` that exits `a1` itself.
    assert_eq!(
        sources(&select(&mut chart, Ev::H)),
        vec![(St::A1, 2), (St::P, 2)]
    );
}

#[test]
fn a_transition_reached_from_two_regions_is_selected_once() {
    let mut chart = Chart::at_initial_configuration();
    // In the initial configuration only `c1` walks up to `p`'s targetless K —
    // `a1` and `b1` answer K themselves. Move the first two regions to `a2`
    // and `b2`, which answer nothing, and all three atomic states reach it: it
    // is still one element of the set.
    let taken = select(&mut chart, Ev::K);
    assert_eq!(sources(&taken), vec![(St::A1, 3), (St::B1, 2), (St::P, 3)]);
    chart.set_configuration(&[St::S, St::P, St::R1, St::A2, St::R2, St::B2, St::R3, St::C1]);
    assert_eq!(sources(&select(&mut chart, Ev::K)), vec![(St::P, 3)]);
}

#[test]
fn an_eventless_selection_with_nothing_enabled_selects_nothing() {
    let mut chart = Chart::at_initial_configuration();
    assert!(select(&mut chart, Ev::Null).is_empty());
}

// ════════════════════════════════════════════════════════════════════════
// The microstep
// ════════════════════════════════════════════════════════════════════════

#[test]
fn a_microstep_exits_then_runs_content_in_selection_order_then_enters() {
    let mut chart = Chart::at_initial_configuration();
    let taken = select(&mut chart, Ev::K);
    let entered = take(&mut chart, &taken);

    // §scxml-D-executeTransitionContent runs content in SELECTION order:
    // `p`'s K was reached last, from `c1`, though `p` comes first in document
    // order.
    assert_eq!(
        chart.log,
        vec![
            "exit B1",
            "exit A1",
            "content A1#3",
            "content B1#2",
            "content P#3",
            "enter A2",
            "enter B2",
        ]
    );
    assert_eq!(
        sorted(chain(&chart.configuration())),
        vec![St::S, St::P, St::R1, St::A2, St::R2, St::B2, St::R3, St::C1]
    );
    // The last transition with targets is `b1 -> b2`: the targetless one after
    // it moves nothing.
    assert_eq!(entered.states, 2);
    assert_eq!(entered.current.map(st), Some(St::B2));
}

#[test]
fn every_state_is_exited_while_it_is_still_active() {
    let mut chart = Chart::at_initial_configuration();
    take(&mut chart, &[row(St::A1, 5)]);

    // onexit, then configuration.delete(s): `In(s)` inside s's own handler is
    // true, for each of the seven states the transition to `hb` exits.
    assert_eq!(chart.exited_while_active, vec![true; 7]);
}

#[test]
fn a_history_is_recorded_before_the_first_exit_and_restored_by_the_entries() {
    let mut chart = Chart::at_initial_configuration();
    let entered = take(&mut chart, &[row(St::A1, 5)]);

    // The domain is `s`, so all of `p` leaves and comes back. `hb` records
    // `r2`'s active child from the configuration as it stood before any exit —
    // `b1` is exited before `r2` — and the entries of the same microstep read
    // what it recorded, so `b1` comes back and no default content runs.
    assert_eq!(
        chart.recorded_at_first_exit,
        Some([Some(vec![St::B1]), None])
    );
    assert_eq!(
        chart.log,
        vec![
            "exit C1",
            "exit R3",
            "exit B1",
            "exit R2",
            "exit A1",
            "exit R1",
            "exit P",
            "enter P",
            "enter R1 (default)",
            "enter A1",
            "enter R2",
            "enter B1",
            "enter R3 (default)",
            "enter C1",
        ]
    );
    assert_eq!(chart.recorded(H::Hs), None);
    assert_eq!(entered.states, 7);
    assert_eq!(entered.current.map(st), Some(St::B1));
}

#[test]
fn a_self_transition_leaves_and_reenters_its_state() {
    let mut chart = Chart::at_initial_configuration();
    let taken = select(&mut chart, Ev::R);
    assert_eq!(sources(&taken), vec![(St::A1, 4), (St::B1, 3)]);
    take(&mut chart, &taken);

    assert_eq!(
        chart.log,
        vec![
            "exit B1",
            "exit A1",
            "content A1#4",
            "content B1#3",
            "enter A1",
            "enter B2",
        ]
    );
}

#[test]
fn a_history_default_content_runs_after_its_parent_is_entered() {
    let mut chart = Chart::at(&[St::T]);
    let entered = take(&mut chart, &[row(St::T, 1)]);

    // `s` was never exited, so `hs` recorded nothing and its default "a2 b2" is
    // taken. `s` is an ancestor of the default targets, not entered by
    // default, and the history's default content runs once `s` has been
    // entered, before anything below it.
    assert_eq!(
        chart.log,
        vec![
            "exit T",
            "enter S",
            "history default Hs",
            "enter P",
            "enter R1",
            "enter A2",
            "enter R2",
            "enter B2",
            "enter R3 (default)",
            "enter C1",
        ]
    );
    assert_eq!(entered.states, 8);
    assert_eq!(entered.current.map(st), Some(St::A2));
}

// ════════════════════════════════════════════════════════════════════════
// isInFinalState
// ════════════════════════════════════════════════════════════════════════
//
// run (parallel)
//   form > filling, formDone (final)
//   checks (parallel)
//     left  > leftPending,  leftDone (final)
//     right > rightPending, rightDone (final)
//
// `checks` is a `<parallel>` among the regions of a `<parallel>`: it has no
// `<final>` child of its own and is final only because both of its regions are.

// Declared in document order, so a discriminant IS the state's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum C {
    Run,
    Form,
    Filling,
    FormDone,
    Checks,
    Left,
    LeftPending,
    LeftDone,
    Right,
    RightPending,
    RightDone,
}

/// One state of the completion document: itself, its parent, its kind and its
/// initial child.
type CompletionRow = (C, Option<C>, StateKind, Option<C>);

const COMPLETION_TREE: [CompletionRow; 11] = [
    (C::Run, None, StateKind::Parallel, None),
    (C::Form, Some(C::Run), StateKind::Compound, Some(C::Filling)),
    (C::Filling, Some(C::Form), StateKind::Atomic, None),
    (C::FormDone, Some(C::Form), StateKind::Final, None),
    (C::Checks, Some(C::Run), StateKind::Parallel, None),
    (
        C::Left,
        Some(C::Checks),
        StateKind::Compound,
        Some(C::LeftPending),
    ),
    (C::LeftPending, Some(C::Left), StateKind::Atomic, None),
    (C::LeftDone, Some(C::Left), StateKind::Final, None),
    (
        C::Right,
        Some(C::Checks),
        StateKind::Compound,
        Some(C::RightPending),
    ),
    (C::RightPending, Some(C::Right), StateKind::Atomic, None),
    (C::RightDone, Some(C::Right), StateKind::Final, None),
];

fn cid(state: C) -> StateId {
    StateId(state as u16)
}

/// `run` with `form` and a state of it, and `checks` with both its regions and a
/// state of each: 8 states, 3 of them atomic, 4 levels down, no `<history>`.
fn completion_tables() -> &'static DocumentTables {
    static TABLES: OnceLock<&'static DocumentTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        for (index, &(state, ..)) in COMPLETION_TREE.iter().enumerate() {
            assert_eq!(cid(state).index(), index, "the tree is in document order");
        }
        let doc = Plain {
            parent: COMPLETION_TREE
                .iter()
                .map(|&(_, parent, ..)| parent.map(cid))
                .collect(),
            kind: COMPLETION_TREE
                .iter()
                .map(|&(_, _, kind, _)| kind)
                .collect(),
            initial: COMPLETION_TREE
                .iter()
                .map(|&(_, _, _, initial)| {
                    initial
                        .map(|child| Target::state(cid(child)))
                        .into_iter()
                        .collect()
                })
                .collect(),
            history_parent: Vec::new(),
            history_deep: Vec::new(),
            history_defaults: Vec::new(),
            written: Vec::new(),
            document_initial: vec![Target::state(cid(C::Run))],
        };
        tables_of(&doc)
    })
}

fn final_in(state: C, configuration: &[C]) -> bool {
    let mut words = [0; state_words(COMPLETION_TREE.len())];
    let mut set = StateSet::new(&mut words);
    for &state in configuration {
        set.insert(cid(state));
    }
    is_in_final_state(completion_tables(), cid(state), set.as_ref())
}

#[test]
fn a_compound_state_is_final_when_a_final_child_is_active() {
    assert!(final_in(
        C::Form,
        &[
            C::Run,
            C::Form,
            C::FormDone,
            C::Checks,
            C::Left,
            C::LeftPending,
            C::Right,
            C::RightPending
        ]
    ));
    assert!(!final_in(
        C::Form,
        &[
            C::Run,
            C::Form,
            C::Filling,
            C::Checks,
            C::Left,
            C::LeftPending,
            C::Right,
            C::RightPending
        ]
    ));
}

#[test]
fn a_nested_parallel_is_final_only_when_every_region_is() {
    assert!(final_in(
        C::Checks,
        &[C::Checks, C::Left, C::LeftDone, C::Right, C::RightDone]
    ));
    assert!(
        !final_in(
            C::Checks,
            &[C::Checks, C::Left, C::LeftDone, C::Right, C::RightPending]
        ),
        "one region still pending leaves the <parallel> short of final"
    );
}

#[test]
fn a_parallel_region_of_a_parallel_counts() {
    assert!(
        final_in(
            C::Run,
            &[
                C::Run,
                C::Form,
                C::FormDone,
                C::Checks,
                C::Left,
                C::LeftDone,
                C::Right,
                C::RightDone
            ]
        ),
        "`checks` has no <final> child of its own; it is final because both of its regions are"
    );
    assert!(!final_in(
        C::Run,
        &[
            C::Run,
            C::Form,
            C::FormDone,
            C::Checks,
            C::Left,
            C::LeftDone,
            C::Right,
            C::RightPending
        ]
    ));
}

#[test]
fn neither_an_atomic_state_nor_a_final_element_is_in_a_final_state() {
    let configuration = [
        C::Run,
        C::Form,
        C::FormDone,
        C::Checks,
        C::Left,
        C::LeftPending,
        C::Right,
        C::RightPending,
    ];
    assert!(
        !final_in(C::LeftPending, &configuration),
        "an atomic state has no <final> child to be active"
    );
    assert!(
        !final_in(C::FormDone, &configuration),
        "being a <final> element is not being IN a final state — the predicate asks about children"
    );
}

// ════════════════════════════════════════════════════════════════════════
// The stack walks and the exit scopes, held to what they replace
// ════════════════════════════════════════════════════════════════════════
//
// Appendix D writes the entry set, the effective targets and the completion check
// as recursions, and the runtime runs them over a stack of steps so that its
// worst-case stack assumes no depth. The two must answer alike for EVERY
// document, not only the ones written out above, and what the recursion answers
// depends on the ORDER it visits things in: a `<parallel>` region is judged
// against what the entry set holds when it is reached, and a later `<history>` of
// a parent replaces an earlier one's default content. So the reference below is
// the recursion itself, as the transcription first wrote it, over the document's
// lists rather than its tables — a parent walk, not a range of ids — and random
// documents ask both. The same documents show the stack never needs more steps
// than the capacity the generator gives it (`sce_build::microstep_bounds`): the
// walk slice lent to the procedures is exactly that long, so a step past it stops
// the test.
//
// The conflict rule of removeConflictingTransitions asks whether two exit sets
// intersect. The runtime answers it from the two transitions' scopes without
// listing either set (`ExitScope::meets`), and the reference lists both off the
// configuration, as computeExitSet does, and intersects them.

/// Appendix D's recursions, as the transcription first wrote them.
mod reference {
    use sce_rust_runtime::helpers::microstep::ExitScope;
    use sce_rust_runtime::{HistoryId, Named, StateId, StateKind, Target, TransitionId};

    use super::Plain;

    /// What each `<history>` recorded, by `HistoryId`.
    pub type Records = [Option<Vec<StateId>>];

    pub struct EntrySet {
        pub states_to_enter: Vec<StateId>,
        pub states_for_default_entry: Vec<StateId>,
        pub default_history_content: Vec<(StateId, HistoryId)>,
    }

    fn add_once(set: &mut Vec<StateId>, state: StateId) {
        if !set.contains(&state) {
            set.push(state);
        }
    }

    fn history_value(records: &Records, history: HistoryId) -> Option<&[StateId]> {
        records[history.index()].as_deref()
    }

    /// Appendix D's isDescendant, by walking up the parents.
    pub fn is_descendant(doc: &Plain, state: StateId, ancestor: StateId) -> bool {
        let mut current = doc.parent[state.index()];
        while let Some(up) = current {
            if up == ancestor {
                return true;
            }
            current = doc.parent[up.index()];
        }
        false
    }

    fn child_states(doc: &Plain, state: StateId) -> Vec<StateId> {
        (0..doc.parent.len())
            .map(|child| StateId(child as u16))
            .filter(|&child| doc.parent[child.index()] == Some(state))
            .collect()
    }

    pub fn effective_target_states(
        doc: &Plain,
        records: &Records,
        targets: &[Target],
    ) -> Vec<StateId> {
        let mut effective = Vec::new();
        add_effective_target_states(doc, records, targets, &mut effective);
        effective
    }

    fn add_effective_target_states(
        doc: &Plain,
        records: &Records,
        targets: &[Target],
        effective: &mut Vec<StateId>,
    ) {
        for &target in targets {
            match target.named() {
                Named::State(state) => add_once(effective, state),
                Named::History(history) => match history_value(records, history) {
                    Some(recorded) if !recorded.is_empty() => {
                        for &state in recorded {
                            add_once(effective, state);
                        }
                    }
                    _ => add_effective_target_states(
                        doc,
                        records,
                        &doc.history_defaults[history.index()],
                        effective,
                    ),
                },
            }
        }
    }

    /// Appendix D's getTransitionDomain, with its findLCCA over the parents.
    pub fn transition_domain(
        doc: &Plain,
        source: Option<StateId>,
        targets: &[StateId],
        internal: bool,
    ) -> Option<StateId> {
        let source = source?;
        if internal
            && doc.kind[source.index()] == StateKind::Compound
            && targets.iter().all(|&t| is_descendant(doc, t, source))
        {
            return Some(source);
        }
        let mut current = doc.parent[source.index()];
        while let Some(ancestor) = current {
            if doc.kind[ancestor.index()] == StateKind::Compound
                && targets.iter().all(|&t| is_descendant(doc, t, ancestor))
            {
                return Some(ancestor);
            }
            current = doc.parent[ancestor.index()];
        }
        None
    }

    /// The scope computeExitSet reads a configuration through, for one
    /// transition.
    pub fn exit_scope(doc: &Plain, records: &Records, transition: TransitionId) -> ExitScope {
        let written = &doc.written[transition.index()];
        if written.targets.is_empty() {
            return ExitScope::Nothing;
        }
        let effective = effective_target_states(doc, records, &written.targets);
        match transition_domain(doc, written.source, &effective, written.internal) {
            Some(domain) => ExitScope::Below(domain),
            None => ExitScope::All,
        }
    }

    /// Appendix D's computeExitSet for one transition, listed off
    /// `configuration`: every active state that is a descendant of the domain.
    pub fn exit_set(
        doc: &Plain,
        records: &Records,
        transition: TransitionId,
        configuration: &[StateId],
    ) -> Vec<StateId> {
        let written = &doc.written[transition.index()];
        if written.targets.is_empty() {
            return Vec::new();
        }
        let effective = effective_target_states(doc, records, &written.targets);
        let domain = transition_domain(doc, written.source, &effective, written.internal);
        configuration
            .iter()
            .copied()
            .filter(|&state| match domain {
                Some(domain) => is_descendant(doc, state, domain),
                None => true,
            })
            .collect()
    }

    pub fn compute_entry_set(
        doc: &Plain,
        records: &Records,
        transitions: &[TransitionId],
    ) -> EntrySet {
        let mut entry = EntrySet {
            states_to_enter: Vec::new(),
            states_for_default_entry: Vec::new(),
            default_history_content: Vec::new(),
        };
        for &transition in transitions {
            let written = &doc.written[transition.index()];
            for &target in &written.targets {
                add_descendant_states_to_enter(doc, records, target, &mut entry);
            }
            let effective = effective_target_states(doc, records, &written.targets);
            if effective.is_empty() {
                continue;
            }
            let domain = transition_domain(doc, written.source, &effective, written.internal);
            for &state in &effective {
                add_ancestor_states_to_enter(
                    doc,
                    records,
                    Target::state(state),
                    domain,
                    &mut entry,
                );
            }
        }
        // Document order is the order of the ids.
        entry.states_to_enter.sort();
        entry
    }

    fn add_descendant_states_to_enter(
        doc: &Plain,
        records: &Records,
        target: Target,
        entry: &mut EntrySet,
    ) {
        let state = match target.named() {
            Named::State(state) => state,
            Named::History(history) => {
                let parent = doc.history_parent[history.index()];
                match history_value(records, history) {
                    Some(recorded) if !recorded.is_empty() => {
                        for &state in recorded {
                            add_descendant_states_to_enter(
                                doc,
                                records,
                                Target::state(state),
                                entry,
                            );
                        }
                        for &state in recorded {
                            add_ancestor_states_to_enter(
                                doc,
                                records,
                                Target::state(state),
                                Some(parent),
                                entry,
                            );
                        }
                    }
                    _ => {
                        match entry
                            .default_history_content
                            .iter_mut()
                            .find(|slot| slot.0 == parent)
                        {
                            Some(slot) => slot.1 = history,
                            None => entry.default_history_content.push((parent, history)),
                        }
                        let defaults = &doc.history_defaults[history.index()];
                        for &default in defaults {
                            add_descendant_states_to_enter(doc, records, default, entry);
                        }
                        for &default in defaults {
                            add_ancestor_states_to_enter(
                                doc,
                                records,
                                default,
                                Some(parent),
                                entry,
                            );
                        }
                    }
                }
                return;
            }
        };

        add_once(&mut entry.states_to_enter, state);
        match doc.kind[state.index()] {
            StateKind::Compound => {
                add_once(&mut entry.states_for_default_entry, state);
                let initial = &doc.initial[state.index()];
                for &child in initial {
                    add_descendant_states_to_enter(doc, records, child, entry);
                }
                for &child in initial {
                    add_ancestor_states_to_enter(doc, records, child, Some(state), entry);
                }
            }
            StateKind::Parallel => add_region_defaults(doc, records, state, entry),
            StateKind::Atomic | StateKind::Final => {}
        }
    }

    fn add_ancestor_states_to_enter(
        doc: &Plain,
        records: &Records,
        target: Target,
        ancestor: Option<StateId>,
        entry: &mut EntrySet,
    ) {
        for anc in proper_ancestors(doc, target, ancestor) {
            add_once(&mut entry.states_to_enter, anc);
            if doc.kind[anc.index()] == StateKind::Parallel {
                add_region_defaults(doc, records, anc, entry);
            }
        }
    }

    /// Appendix D's getProperAncestors: up to, not including, `ancestor`, and
    /// none at all when `ancestor` is not above `target`.
    fn proper_ancestors(doc: &Plain, target: Target, ancestor: Option<StateId>) -> Vec<StateId> {
        let mut chain = Vec::new();
        let mut current = match target.named() {
            Named::State(state) => doc.parent[state.index()],
            Named::History(history) => Some(doc.history_parent[history.index()]),
        };
        while let Some(state) = current {
            if ancestor == Some(state) {
                return chain;
            }
            chain.push(state);
            current = doc.parent[state.index()];
        }
        if ancestor.is_some() {
            chain.clear();
        }
        chain
    }

    fn add_region_defaults(
        doc: &Plain,
        records: &Records,
        parallel: StateId,
        entry: &mut EntrySet,
    ) {
        for child in child_states(doc, parallel) {
            let taken = entry
                .states_to_enter
                .iter()
                .any(|&state| is_descendant(doc, state, child));
            if !taken {
                add_descendant_states_to_enter(doc, records, Target::state(child), entry);
            }
        }
    }

    pub fn is_in_final_state(doc: &Plain, state: StateId, configuration: &[StateId]) -> bool {
        match doc.kind[state.index()] {
            StateKind::Compound => child_states(doc, state).iter().any(|&child| {
                doc.kind[child.index()] == StateKind::Final && configuration.contains(&child)
            }),
            StateKind::Parallel => child_states(doc, state)
                .iter()
                .all(|&child| is_in_final_state(doc, child, configuration)),
            StateKind::Atomic | StateKind::Final => false,
        }
    }
}

/// A small xorshift generator: the documents are a function of the seed alone, so
/// a failure names the seed that reproduces it.
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

/// Names the seed of a document whose check panicked, so the failure can be
/// reproduced: a buffer sized too small panics inside the runtime, where the
/// seed is not known.
struct Seed(u64);

impl Drop for Seed {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("the failing document is the one of seed {}", self.0);
        }
    }
}

/// How many pairs of random transitions each random document writes.
const ROUNDS: u16 = 8;

/// A document of up to twelve states drawn at random, numbered in document
/// order, with `2 * ROUNDS` random transitions written into it, and what its
/// histories recorded.
struct RandomDocument {
    plain: Plain,
    tables: &'static DocumentTables,
    records: Vec<Option<Vec<StateId>>>,
    /// Levels, a `<history>` counting as one below its parent.
    depth: usize,
}

impl RandomDocument {
    fn states(&self) -> usize {
        self.plain.parent.len()
    }

    fn histories(&self) -> usize {
        self.plain.history_parent.len()
    }

    /// The capacity the generator gives the walks' stack for this document
    /// (`sce_build::microstep_bounds`).
    fn walk_bound(&self) -> usize {
        3 * self.depth + self.histories() + 3
    }

    /// The rows of `transitions`, for a failure's message.
    fn written(&self, transitions: &[TransitionId]) -> Vec<&Written> {
        transitions
            .iter()
            .map(|transition| &self.plain.written[transition.index()])
            .collect()
    }
}

/// A forest, each state with children a `<parallel>` or a compound state,
/// initial targets anywhere below a compound state, and up to three `<history>`
/// elements that recorded something, recorded nothing or recorded an empty set,
/// and whose default transitions may name a later `<history>` below their parent.
/// The tree is drawn with each state's parent drawn before it and then numbered
/// in document order, a pre-order walk, as the generator numbers a document.
fn random_document(rng: &mut Rng) -> RandomDocument {
    let n = 1 + rng.below(12);
    let mut drawn_parent: Vec<Option<usize>> = Vec::new();
    for i in 0..n {
        drawn_parent.push(if i == 0 || rng.chance(4) {
            None
        } else {
            Some(rng.below(i))
        });
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, p) in drawn_parent.iter().enumerate() {
        if let Some(p) = p {
            children[*p].push(i);
        }
    }
    let parallel: Vec<bool> = children
        .iter()
        .map(|c| !c.is_empty() && rng.chance(3))
        .collect();
    let is_final: Vec<bool> = children
        .iter()
        .map(|c| c.is_empty() && rng.chance(3))
        .collect();
    let below = |state: usize| -> Vec<usize> {
        let mut found = Vec::new();
        let mut frontier = children[state].clone();
        while let Some(next) = frontier.pop() {
            found.push(next);
            frontier.extend(children[next].iter().copied());
        }
        found
    };
    let pick = |rng: &mut Rng, from: &[usize]| from[rng.below(from.len())];

    // Document order: the roots in the order drawn, each followed by its
    // subtree, children in the order drawn.
    let mut order = Vec::with_capacity(n);
    let mut pending: Vec<usize> = (0..n)
        .rev()
        .filter(|&i| drawn_parent[i].is_none())
        .collect();
    while let Some(next) = pending.pop() {
        order.push(next);
        pending.extend(children[next].iter().rev());
    }
    let mut number = vec![StateId(0); n];
    for (position, &drawn) in order.iter().enumerate() {
        number[drawn] = StateId(position as u16);
    }

    let mut parent = vec![None; n];
    let mut kind = vec![StateKind::Atomic; n];
    for (drawn, kids) in children.iter().enumerate() {
        let state = number[drawn].index();
        parent[state] = drawn_parent[drawn].map(|p| number[p]);
        kind[state] = if parallel[drawn] {
            StateKind::Parallel
        } else if !kids.is_empty() {
            StateKind::Compound
        } else if is_final[drawn] {
            StateKind::Final
        } else {
            StateKind::Atomic
        };
    }

    let mut initial: Vec<Vec<Target>> = vec![Vec::new(); n];
    for (drawn, &state) in number.iter().enumerate() {
        if kind[state.index()] == StateKind::Compound {
            let inside = below(drawn);
            let count = 1 + rng.below(2);
            initial[state.index()] = (0..count)
                .map(|_| Target::state(number[pick(rng, &inside)]))
                .collect();
        }
    }

    let with_children: Vec<usize> = (0..n).filter(|&i| !children[i].is_empty()).collect();
    let histories = if with_children.is_empty() {
        0
    } else {
        rng.below(4)
    };
    let drawn_history_parent: Vec<usize> =
        (0..histories).map(|_| pick(rng, &with_children)).collect();
    let mut records: Vec<Option<Vec<StateId>>> = Vec::new();
    let mut history_deep: Vec<bool> = Vec::new();
    let mut history_defaults: Vec<Vec<Target>> = Vec::new();
    for (h, &history_parent) in drawn_history_parent.iter().enumerate() {
        let inside = below(history_parent);
        records.push(match rng.below(3) {
            0 => None,
            1 => Some(Vec::new()),
            // A set of states, as a record is a set of active states: each
            // once, listed in document order, the order a run store reads it.
            _ => {
                let mut recorded: Vec<StateId> = Vec::new();
                for _ in 0..1 + rng.below(3) {
                    let state = number[pick(rng, &inside)];
                    if !recorded.contains(&state) {
                        recorded.push(state);
                    }
                }
                recorded.sort();
                Some(recorded)
            }
        });
        history_deep.push(rng.chance(2));
        // A default transition may name a later history whose parent is this
        // one's or lies below it: a later one only, so the names never loop.
        let named: Vec<usize> = (h + 1..histories)
            .filter(|&j| {
                drawn_history_parent[j] == history_parent
                    || inside.contains(&drawn_history_parent[j])
            })
            .collect();
        let count = 1 + rng.below(2);
        history_defaults.push(
            (0..count)
                .map(|_| {
                    if !named.is_empty() && rng.chance(4) {
                        Target::history(HistoryId(pick(rng, &named) as u16))
                    } else {
                        Target::state(number[pick(rng, &inside)])
                    }
                })
                .collect(),
        );
    }

    let level = |mut state: usize| {
        let mut levels = 1;
        while let Some(up) = drawn_parent[state] {
            levels += 1;
            state = up;
        }
        levels
    };
    let mut depth = (0..n).map(level).max().unwrap_or(1);
    for &p in &drawn_history_parent {
        depth = depth.max(level(p) + 1);
    }

    let written = (0..2 * ROUNDS)
        .map(|_| random_transition(rng, n, histories))
        .collect();
    let plain = Plain {
        parent,
        kind,
        initial,
        history_parent: drawn_history_parent.iter().map(|&p| number[p]).collect(),
        history_deep,
        history_defaults,
        written,
        // The first root in document order is state 0.
        document_initial: vec![Target::state(StateId(0))],
    };
    let tables = tables_of(&plain);
    RandomDocument {
        plain,
        tables,
        records,
        depth,
    }
}

fn random_target(rng: &mut Rng, states: usize, histories: usize) -> Target {
    if histories > 0 && rng.chance(3) {
        Target::history(HistoryId(rng.below(histories) as u16))
    } else {
        Target::state(StateId(rng.below(states) as u16))
    }
}

fn random_transition(rng: &mut Rng, states: usize, histories: usize) -> Written {
    let source = if rng.chance(6) {
        None
    } else {
        Some(StateId(rng.below(states) as u16))
    };
    let count = 1 + rng.below(3);
    let targets = (0..count)
        .map(|_| random_target(rng, states, histories))
        .collect();
    Written {
        source,
        targets,
        internal: rng.chance(2),
        has_content: false,
    }
}

/// A random document's run as the procedures under test read it: what its
/// histories recorded, laid out as a run store keeps it, one set of the
/// document's states for each history, and the configuration a case reads.
struct Recorded {
    configuration: Vec<Word>,
    records: Vec<Word>,
    recorded: Vec<bool>,
}

impl Recorded {
    fn of(doc: &RandomDocument) -> Self {
        let words = state_words(doc.states());
        let mut records = vec![0; words * doc.histories()];
        let mut recorded = vec![false; doc.histories()];
        for (index, states) in doc.records.iter().enumerate() {
            if let Some(states) = states {
                let mut record = StateSet::new(&mut records[index * words..][..words]);
                for &state in states {
                    record.insert(state);
                }
                recorded[index] = true;
            }
        }
        Recorded {
            configuration: vec![0; words],
            records,
            recorded,
        }
    }

    /// The same records over `configuration`.
    fn with_configuration(&mut self, configuration: &[StateId]) -> &mut Self {
        let mut set = StateSet::new(&mut self.configuration);
        set.clear();
        for &state in configuration {
            set.insert(state);
        }
        self
    }

    fn view(&self) -> RunStoreRef<'_> {
        RunStoreRef::new(&self.configuration, &self.records, &self.recorded)
    }
}

/// A step no walk ever puts down: no document here has that many histories.
const UNTOUCHED: WalkStep = WalkStep::EffectiveDefaults {
    history: HistoryId(u16::MAX),
    index: u16::MAX,
};

/// The buffers a random document's procedures borrow, of the sizes the
/// generator's formulas give: `state_scratch` of the document's states, the
/// walk bound, one history default for each history. A procedure that needs
/// more than one of them stops the test.
///
/// The history defaults are lent uninitialised, as a [`Workspace`] lends them.
/// The walk's slots are each written with [`UNTOUCHED`] first, so how deep the
/// walks went can be read back; a walk writes only whole steps over them.
struct Lent {
    sets: Vec<Word>,
    walk: Vec<MaybeUninit<WalkStep>>,
    history_defaults: Vec<MaybeUninit<(StateId, HistoryId)>>,
    /// What a procedure that returns a set of states writes it into.
    out: Vec<Word>,
}

impl Lent {
    fn for_document(doc: &RandomDocument) -> Self {
        Lent {
            sets: vec![0; state_scratch(doc.states())],
            walk: vec![MaybeUninit::new(UNTOUCHED); doc.walk_bound()],
            history_defaults: vec![MaybeUninit::uninit(); doc.histories()],
            out: vec![0; state_words(doc.states())],
        }
    }

    fn scratch(&mut self) -> Scratch<'_> {
        Scratch {
            sets: &mut self.sets,
            walk: &mut self.walk,
            history_defaults: &mut self.history_defaults,
        }
    }

    fn split(&mut self) -> (StateSet<'_>, Scratch<'_>) {
        (
            StateSet::new(&mut self.out),
            Scratch {
                sets: &mut self.sets,
                walk: &mut self.walk,
                history_defaults: &mut self.history_defaults,
            },
        )
    }

    /// The most steps any walk has had on the stack: every walk pushes from the
    /// front of the slice, so the slots ever written are a prefix of it.
    fn walk_high_water(&self) -> usize {
        self.walk
            .iter()
            .position(|slot| {
                // SAFETY: `for_document` initialised every slot, and a walk
                // writes only whole steps into a slot, so each holds a step.
                let step = unsafe { slot.assume_init_read() };
                step == UNTOUCHED
            })
            .unwrap_or(self.walk.len())
    }
}

#[test]
fn the_stack_walks_answer_as_the_recursion_they_replace_for_any_document() {
    let mut deepest_walk = 0;
    for seed in 1..=4000u64 {
        let _seed = Seed(seed);
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let doc = random_document(&mut rng);
        let mut recorded = Recorded::of(&doc);
        let mut lent = Lent::for_document(&doc);

        for round in 0..ROUNDS {
            let first = TransitionId(2 * round);
            let second = TransitionId(2 * round + 1);

            // One transition, then two, so the walks also run one after another
            // over the stack the entry set lends them.
            for transitions in [&[first][..], &[first, second][..]] {
                let (states_to_enter, mut states_for_default_entry, mut default_history_content) = {
                    let entry =
                        compute_entry_set(doc.tables, recorded.view(), transitions, lent.scratch());
                    (
                        entry.states_to_enter.as_ref().iter().collect::<Vec<_>>(),
                        entry
                            .states_for_default_entry
                            .as_ref()
                            .iter()
                            .collect::<Vec<_>>(),
                        entry.default_history_content.to_vec(),
                    )
                };
                let mut expected =
                    reference::compute_entry_set(&doc.plain, &doc.records, transitions);
                assert_eq!(
                    states_to_enter,
                    expected.states_to_enter,
                    "seed {seed}: states to enter, for {:?}",
                    doc.written(transitions)
                );
                states_for_default_entry.sort();
                expected.states_for_default_entry.sort();
                assert_eq!(
                    states_for_default_entry,
                    expected.states_for_default_entry,
                    "seed {seed}: states entered by default, for {:?}",
                    doc.written(transitions)
                );
                // Keyed by parent, a later history replacing an earlier one's, so
                // what is compared is the pair each parent ends with.
                default_history_content.sort();
                expected.default_history_content.sort();
                assert_eq!(
                    default_history_content,
                    expected.default_history_content,
                    "seed {seed}: history default content, for {:?}",
                    doc.written(transitions)
                );
            }

            let targets = doc.tables.targets(first);
            let (walked, first_named) = {
                let (mut effective, scratch) = lent.split();
                let first_named = effective_target_states_on(
                    doc.tables,
                    recorded.view(),
                    targets,
                    &mut effective,
                    scratch.walk,
                );
                (effective.as_ref().iter().collect::<Vec<_>>(), first_named)
            };
            // A set, read in document order; the first state the list names
            // is the head of the reference's list.
            let mut expected =
                reference::effective_target_states(&doc.plain, &doc.records, targets);
            let expected_first = expected.first().copied();
            expected.sort();
            assert_eq!(
                (walked, first_named),
                (expected, expected_first),
                "seed {seed}: effective targets of {targets:?}"
            );
        }

        // Completion, for every state against random configurations.
        for state in 0..doc.states() {
            for _ in 0..6 {
                let configuration: Vec<StateId> = (0..doc.states())
                    .filter(|_| rng.chance(2))
                    .map(|i| StateId(i as u16))
                    .collect();
                let state = StateId(state as u16);
                let store = recorded.with_configuration(&configuration).view();
                assert_eq!(
                    is_in_final_state(doc.tables, state, store.configuration()),
                    reference::is_in_final_state(&doc.plain, state, &configuration),
                    "seed {seed}: is_in_final_state({state:?}) over {configuration:?}"
                );
            }
        }

        deepest_walk = deepest_walk.max(lent.walk_high_water());
    }
    // The documents do reach into the stack, so the bound the walk slice was
    // sized to was asked a real question and not an empty one.
    assert!(
        deepest_walk >= 6,
        "the deepest walk was {deepest_walk} steps"
    );
}

#[test]
fn two_exit_scopes_meet_exactly_when_the_exit_sets_listed_off_a_configuration_intersect() {
    let (mut met, mut apart) = (0, 0);
    for seed in 1..=4000u64 {
        let _seed = Seed(seed);
        let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
        let doc = random_document(&mut rng);
        let mut recorded = Recorded::of(&doc);
        let mut lent = Lent::for_document(&doc);

        for _ in 0..4 {
            // Any set of states, not only a legal configuration: the rule is
            // claimed for whatever configuration it is read off.
            let configuration: Vec<StateId> = (0..doc.states())
                .filter(|_| rng.chance(2))
                .map(|i| StateId(i as u16))
                .collect();
            let recorded = recorded.with_configuration(&configuration);
            for round in 0..ROUNDS {
                let pair = [TransitionId(2 * round), TransitionId(2 * round + 1)];
                let scopes = pair.map(|transition| {
                    compute_exit_set(doc.tables, recorded.view(), transition, lent.scratch())
                });
                let listed = pair.map(|transition| {
                    reference::exit_set(&doc.plain, &doc.records, transition, &configuration)
                });

                for (index, &transition) in pair.iter().enumerate() {
                    assert_eq!(
                        scopes[index],
                        reference::exit_scope(&doc.plain, &doc.records, transition),
                        "seed {seed}: the exit scope of {:?}",
                        doc.written(&[transition])
                    );
                    let read_off: Vec<StateId> = configuration
                        .iter()
                        .copied()
                        .filter(|&state| scopes[index].contains(doc.tables, state))
                        .collect();
                    assert_eq!(
                        read_off,
                        listed[index],
                        "seed {seed}: the exit set of {:?} over {configuration:?}",
                        doc.written(&[transition])
                    );
                }

                let intersect = listed[0].iter().any(|state| listed[1].contains(state));
                let active = recorded.view().configuration();
                assert_eq!(
                    scopes[0].meets(doc.tables, scopes[1], active),
                    intersect,
                    "seed {seed}: whether the exit sets of {:?} meet over {configuration:?}",
                    doc.written(&pair)
                );
                assert_eq!(
                    scopes[1].meets(doc.tables, scopes[0], active),
                    intersect,
                    "seed {seed}: the same question asked the other way round, for {:?}",
                    doc.written(&pair)
                );
                if intersect {
                    met += 1;
                } else {
                    apart += 1;
                }

                // The microstep's exit set: the union of the two, a set read in
                // document order.
                let walked = {
                    let (mut states_to_exit, scratch) = lent.split();
                    compute_states_to_exit(
                        doc.tables,
                        recorded.view(),
                        &pair,
                        &mut states_to_exit,
                        scratch,
                    );
                    states_to_exit.as_ref().iter().collect::<Vec<_>>()
                };
                let mut union = listed[0].clone();
                for &state in &listed[1] {
                    if !union.contains(&state) {
                        union.push(state);
                    }
                }
                union.sort();
                assert_eq!(
                    walked,
                    union,
                    "seed {seed}: the states {:?} exit from {configuration:?}",
                    doc.written(&pair)
                );
            }
        }
    }
    // Both answers came up many times, so the rule was held to both.
    assert!(
        met > 100 && apart > 100,
        "the pairs met {met} times and stayed apart {apart} times"
    );
}
