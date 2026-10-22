// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! W3C SCXML Appendix D's microstep — which transitions an event selects,
//! which of them survive preemption, which states they exit and in which
//! order, the order their content runs in, which states they enter — written
//! once for every machine this crate runs.
//!
//! The procedures are transcribed rather than paraphrased, one function per
//! procedure and under the appendix's names, so a reader can hold this file
//! against the specification line by line — and against the C++ engines'
//! shared transcription (`sce/include/core/MicrostepAlgorithms.h`,
//! `EntrySetHelper.h`, `ParallelTransitionHelper.h`,
//! `ConflictResolutionHelper.h`, `ParallelCompletionHelper.h`), which answers
//! the same questions the same way.
//!
//! What differs between machines is injected. [`Document`] describes the
//! document — its structure, and what each `<history>` recorded. [`Run`] is the
//! running machine: its configuration, which of a state's transitions an event
//! enables, and what exiting a state, running a transition's content and
//! entering a state DO. Nothing here knows how a machine stores either, so a
//! table written by hand answers them as well as a machine's own code does.
//!
//! # Capacity
//!
//! Every collection here is bounded by the document, and the document says how
//! much ([`helpers::hierarchy`](super::hierarchy)): the buffers are the
//! [`Document`]'s associated types, spelled by the generator with the capacity
//! its state tree needs. A configuration, an exit set, an entry set and a list
//! of effective targets are all sets of states that are, or are about to be,
//! active, so they are of the configuration's [`Configuration`](Document::Configuration)
//! type; the enabled transitions are of its [`EnabledSet`](Document::EnabledSet),
//! one at most per atomic state. No capacity constant is introduced here.

use core::fmt::Debug;

use crate::helpers::hierarchy::Bounded;
use crate::{stable_sort_by, stable_sort_by_key, BoundedPush};

/// One token of a target list, as the document wrote it.
///
/// A transition's `target`, a state's initial transition, and a `<history>`
/// element's default transition all name either states or `<history>`
/// pseudo-states. A history is not a state — it is never in a configuration —
/// so the two kinds are kept apart rather than folded into one identifier: the
/// entry procedures dereference a history (to what it recorded, or to its
/// default) and add a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryTarget<S, H> {
    /// A `<state>`, `<parallel>` or `<final>`.
    State(S),
    /// A `<history>` pseudo-state.
    History(H),
}

/// The history type of a document that declares no `<history>`.
///
/// Uninhabited, so no target list can name one and every procedure that would
/// dereference one is statically unreachable — a document without histories
/// pays nothing for the history half of the entry procedures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoHistory {}

/// A transition selection enabled: one member of Appendix D's
/// `enabledTransitions`.
///
/// What the microstep reads of it — its source, its target list as written,
/// whether it is internal — plus the index its source state knows it by, so the
/// machine that owns its executable content can run it. A transition with no
/// targets exits and enters nothing and only runs its content.
///
/// The target list is `'static` because it is the document's: a generated
/// machine hands out slices of its own static tables, and a transition is a
/// fact about the document, not about the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnabledTransition<S: 'static, H: 'static> {
    /// The state whose transition this is.
    pub source: S,
    /// The target list as written — empty for a targetless transition.
    pub targets: &'static [EntryTarget<S, H>],
    /// The position of this transition among its source's own transitions.
    pub transition_index: usize,
    /// Whether the transition has executable content to run.
    pub has_actions: bool,
    /// Whether the transition was written `type="internal"`.
    pub is_internal: bool,
}

impl<S: Copy + 'static, H: Copy + 'static> EnabledTransition<S, H> {
    /// §scxml-D-computeExitSet guards the whole computation with `if
    /// t.target`: a transition without targets exits and enters nothing.
    pub fn is_targetless(&self) -> bool {
        self.targets.is_empty()
    }

    /// The same transition as the entry procedures read it.
    pub fn entry_transition(&self) -> EntryTransition<S, H> {
        EntryTransition {
            source: Some(self.source),
            targets: self.targets,
            is_internal: self.is_internal,
        }
    }
}

/// The part of a transition the entry procedures read.
///
/// `source` is `None` for the document's own initial transition, whose source
/// is the `<scxml>` element — which has no state identifier here, and whose
/// domain is the whole document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryTransition<S: 'static, H: 'static> {
    /// The transition's source state, or `None` for the `<scxml>` element.
    pub source: Option<S>,
    /// The target list as written.
    pub targets: &'static [EntryTarget<S, H>],
    /// Whether the transition was written `type="internal"`.
    pub is_internal: bool,
}

/// What a microstep enters: the appendix's three out-parameters.
///
/// `states_to_enter` is already sorted into entry order, so a machine enters it
/// front to back. `states_for_default_entry` answers whether a state's initial
/// transition content runs; `default_history_content` whether, and whose,
/// `<history>` default transition content runs after that state's own entry.
///
/// Generic over its two buffers and not over the [`Document`] that sized them,
/// so it is named by `EntrySet<D::Configuration, D::HistoryDefaults>`: a
/// document that borrows (the engine's, which holds the engine it reads) would
/// otherwise keep that borrow alive in the set it hands back.
#[derive(Debug, Clone)]
pub struct EntrySet<C, X> {
    /// The states to enter, in entry order.
    pub states_to_enter: C,
    /// The compound states whose initial state is entered by default.
    pub states_for_default_entry: C,
    /// Keyed by the history's parent, as the appendix keys it: the content
    /// runs when that parent is entered.
    pub default_history_content: X,
}

/// The entry set of a document `D`: its buffers are the ones `D` sized.
pub type EntrySetOf<D> = EntrySet<<D as Document>::Configuration, <D as Document>::HistoryDefaults>;

impl<C: Default, X: Default> EntrySet<C, X> {
    fn new() -> Self {
        Self {
            states_to_enter: C::default(),
            states_for_default_entry: C::default(),
            default_history_content: X::default(),
        }
    }
}

impl<C, X> EntrySet<C, X> {
    /// Whether `state` is entered by this microstep.
    pub fn contains<S: PartialEq>(&self, state: S) -> bool
    where
        C: Bounded<S>,
    {
        self.states_to_enter.contains(&state)
    }

    /// Whether `state`'s initial state is entered by default, so its initial
    /// transition's executable content runs.
    pub fn is_default_entry<S: PartialEq>(&self, state: S) -> bool
    where
        C: Bounded<S>,
    {
        self.states_for_default_entry.contains(&state)
    }

    /// The `<history>` whose default transition content runs after `state` is
    /// entered, if a history of `state` was taken with nothing recorded.
    pub fn default_history_content_of<S: PartialEq + Copy, H: Copy>(&self, state: S) -> Option<H>
    where
        X: Bounded<(S, H)>,
    {
        self.default_history_content
            .iter()
            .find(|(parent, _)| *parent == state)
            .map(|&(_, history)| history)
    }
}

/// The document, as the entry and exit procedures read it.
///
/// Structure only, plus the one piece of run-time state the entry procedures
/// need: what each `<history>` recorded. A document whose `<history>` defaults
/// name one another in a cycle has no entry set — dereferencing never reaches a
/// state — so it must be refused before anything here runs; the procedures
/// assume a legal document.
///
/// It also says how much room the procedures need, since only the document
/// knows: the buffers they build are its associated types, and the walks up its
/// hierarchy are held to its depth. A generated machine gets all four from its
/// state tree ([`helpers::hierarchy`](super::hierarchy)); a hand-written table
/// states the bounds of its own.
pub trait Document {
    /// A state identifier.
    type State: Copy + Eq + Debug + 'static;
    /// A `<history>` identifier — [`NoHistory`] for a document with none.
    type History: Copy + Eq + Debug + 'static;

    /// A set of states that are, or are about to be, in one configuration: the
    /// configuration itself, an exit set, an entry set, a list of effective
    /// targets, a history's record. Its capacity is the most states one
    /// configuration of this document holds.
    type Configuration: Bounded<Self::State>;

    /// The transitions one selection enables, at most one from each atomic state
    /// of a configuration.
    type EnabledSet: Bounded<EnabledTransition<Self::State, Self::History>>;

    /// What a microstep owes the default transition of each `<history>` it took
    /// with nothing recorded, one pair for each parent: at most the number of
    /// states that have a `<history>`.
    type HistoryDefaults: Bounded<(Self::State, Self::History)>;

    /// How many levels the deepest state sits at, counting itself — the number of
    /// steps that always reach the `<scxml>` element from any state. A walk up the
    /// hierarchy that has not got there by then is walking a cycle.
    const MAX_DEPTH: usize;

    /// The state's parent; `None` when its parent is the `<scxml>` element.
    fn parent_of(&self, state: Self::State) -> Option<Self::State>;

    /// Whether the state is a `<state>` with child states. A `<parallel>`
    /// answers `false`: this is the appendix's `isCompoundState`, and with
    /// the `<scxml>` element (`None`) the set `findLCCA` chooses a domain from.
    fn is_compound(&self, state: Self::State) -> bool;

    /// Whether the state is a `<parallel>`.
    fn is_parallel(&self, state: Self::State) -> bool;

    /// Whether the state is a `<final>` element — not whether it is IN a final
    /// state; that is [`is_in_final_state`].
    fn is_final(&self, state: Self::State) -> bool;

    /// §scxml-D-getChildStates: the state's `<state>`, `<parallel>` and
    /// `<final>` children, in document order.
    fn child_states(&self, state: Self::State) -> &'static [Self::State];

    /// A compound state's initial transition target, as written — the first
    /// child state when the document names none.
    fn initial_targets(
        &self,
        state: Self::State,
    ) -> &'static [EntryTarget<Self::State, Self::History>];

    /// The state a `<history>` is declared in.
    fn history_parent(&self, history: Self::History) -> Self::State;

    /// What the history recorded when its parent was last exited; `None`
    /// before that ever happened.
    fn history_value(&self, history: Self::History) -> Option<&[Self::State]>;

    /// A `<history>`'s default transition target, as written.
    fn history_default_targets(
        &self,
        history: Self::History,
    ) -> &'static [EntryTarget<Self::State, Self::History>];

    /// The state's position in document order, which is also entry order.
    fn document_order(&self, state: Self::State) -> u32;
}

/// The running machine, as the microstep drives it.
pub trait Run: Document {
    /// An event, including the machine's own "no event" for an eventless
    /// selection; this module only hands it back.
    type Event: Copy;

    /// The active states, in any order.
    fn configuration(&self) -> Self::Configuration;

    /// This state's first transition, in document order, that the event
    /// enables and whose guard holds; for the machine's "no event", its first
    /// eventless transition whose guard holds. The only place a guard is
    /// evaluated.
    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
    ) -> Option<EnabledTransition<Self::State, Self::History>>;

    /// Record this state's histories from the configuration as it stood before
    /// the microstep's first exit, run its onexit, cancel its invocations,
    /// remove it from the configuration.
    fn exit_state(&mut self, state: Self::State, configuration_before_exit: &[Self::State]);

    /// Run one transition's executable content.
    fn execute_transition_content(
        &mut self,
        transition: &EnabledTransition<Self::State, Self::History>,
    );

    /// Add the state to the configuration and schedule its invocations, run its
    /// onentry, then its initial transition's content when `is_default_entry`;
    /// for a `<final>`, what the appendix does on entering one.
    fn enter_state(&mut self, state: Self::State, is_default_entry: bool);

    /// Run a `<history>`'s default transition content.
    fn execute_history_default_content(&mut self, history: Self::History);
}

// ════════════════════════════════════════════════════════════════════════
// Selection
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's selectTransitions, or its selectEventlessTransitions when
/// `event` is the machine's "no event".
///
/// Returns the optimal enabled transition set, in selection order.
pub fn select_transitions<R: Run>(run: &mut R, event: R::Event) -> R::EnabledSet {
    let configuration = run.configuration();

    let mut atomic_states = R::Configuration::default();
    for &state in configuration.iter() {
        if !run.is_compound(state) && !run.is_parallel(state) {
            atomic_states.push_bounded(state);
        }
    }
    stable_sort_by_key(&mut atomic_states, |&state| run.document_order(state));

    let mut enabled = R::EnabledSet::default();
    for &atomic in atomic_states.iter() {
        // §scxml-D-selectTransitions: the atomic state first, then its proper
        // ancestors, and the first enabled transition in document order ends
        // the walk for this atomic state. The set is ORDERED and a set: two
        // atomic states under one ancestor both reach its transition, and it is
        // one transition, taken once. With the machine's "no event" this is
        // §scxml-D-selectEventlessTransitions, the same walk over transitions
        // that have no event.
        let mut current = Some(atomic);
        while let Some(state) = current {
            if let Some(found) = run.first_enabled_transition(state, event) {
                let seen = enabled.iter().any(|t| {
                    t.source == found.source && t.transition_index == found.transition_index
                });
                if !seen {
                    enabled.push_bounded(found);
                }
                break;
            }
            current = run.parent_of(state);
        }
    }
    remove_conflicting_transitions(run, &mut enabled, &configuration);
    enabled
}

/// Appendix D's removeConflictingTransitions, over `enabled` itself: what
/// remains in it is the filtered set, in selection order.
///
/// Two transitions conflict when their exit sets intersect, and the one whose
/// source is a descendant of the other's wins; otherwise the one selected
/// first does.
///
/// The kept transitions are the front of the buffer the candidates are read
/// from, so the filtered set is never a second buffer beside the first. A
/// candidate is read before the slot a kept one is written to, which is never
/// past it.
pub fn remove_conflicting_transitions<D: Document>(
    doc: &D,
    enabled: &mut D::EnabledSet,
    configuration: &[D::State],
) {
    let mut kept = 0;
    for candidate in 0..enabled.len() {
        let t1 = enabled[candidate];
        // A kept transition's exit set is computed again for each later one
        // held against it rather than stored beside it: on an MCU the stack a
        // stored set per transition costs is scarcer than the cycles, and the
        // sets are read off the one configuration the selection saw, so the
        // answer is the same either way.
        let t1_exit = compute_exit_set(doc, &t1, configuration);
        let conflicts = |t2: &EnabledTransition<D::State, D::History>| {
            intersects(&t1_exit, &compute_exit_set(doc, t2, configuration))
        };
        // §scxml-D-removeConflictingTransitions: t1 is preempted by any kept
        // transition it conflicts with whose source it does not descend from.
        // A transition that exits nothing — a targetless one — conflicts with
        // nothing and can never be preempted.
        let preempted = enabled[..kept]
            .iter()
            .any(|t2| conflicts(t2) && !is_descendant(doc, t1.source, t2.source));
        if preempted {
            continue;
        }
        // Not preempted means every kept transition t1 conflicts with has a
        // source t1 descends from: the appendix removes exactly those.
        let mut surviving = 0;
        for index in 0..kept {
            let t2 = enabled[index];
            if !conflicts(&t2) {
                enabled[surviving] = t2;
                surviving += 1;
            }
        }
        enabled[surviving] = t1;
        kept = surviving + 1;
    }
    enabled.truncate(kept);
}

// ════════════════════════════════════════════════════════════════════════
// Domains and exit sets
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's getEffectiveTargetStates, over a target list.
///
/// Returns the states the list stands for, histories dereferenced — to what
/// each recorded or, before its parent was ever exited, to its default — each
/// state once and in the order first named. The domain, and so the exit set,
/// is a question about these.
pub fn effective_target_states<D: Document>(
    doc: &D,
    targets: &[EntryTarget<D::State, D::History>],
) -> D::Configuration {
    let mut effective = D::Configuration::default();
    add_effective_target_states(doc, targets, &mut effective);
    effective
}

fn add_effective_target_states<D: Document>(
    doc: &D,
    targets: &[EntryTarget<D::State, D::History>],
    effective: &mut D::Configuration,
) {
    for &target in targets {
        match target {
            EntryTarget::State(state) => add_once(effective, state),
            // §scxml-D-getEffectiveTargetStates: a history stands for the
            // configuration it recorded, and before its parent was ever exited
            // for the targets of its default transition — which may name
            // histories themselves, hence the recursion.
            EntryTarget::History(history) => match doc.history_value(history) {
                Some(recorded) if !recorded.is_empty() => {
                    for &state in recorded {
                        add_once(effective, state);
                    }
                }
                _ => add_effective_target_states(
                    doc,
                    doc.history_default_targets(history),
                    effective,
                ),
            },
        }
    }
}

/// Appendix D's getTransitionDomain, for a transition that has targets.
///
/// `targets` are the transition's EFFECTIVE targets: a history that recorded a
/// state deep inside its parent is a target that deep. Returns the domain, or
/// `None` when it is the `<scxml>` element — always so for the document's
/// initial transition (`source` `None`).
pub fn transition_domain<D: Document>(
    doc: &D,
    source: Option<D::State>,
    targets: &[D::State],
    is_internal: bool,
) -> Option<D::State> {
    let source = source?;
    debug_assert!(
        !targets.is_empty(),
        "a targetless transition has no domain to ask for"
    );

    // §scxml-D-getTransitionDomain: an internal transition whose targets all
    // lie below a compound source has the SOURCE as its domain, so the source
    // stays active and only its active descendants are exited. That is not the
    // same as exiting nothing: a transition rooted at one of those descendants
    // exits it too, and the appendix expects the two to be found in conflict.
    if is_internal
        && doc.is_compound(source)
        && targets.iter().all(|&t| is_descendant(doc, t, source))
    {
        return Some(source);
    }
    find_lcca(doc, source, targets)
}

/// Appendix D's findLCCA over `[head] + tail`: the first proper ancestor of
/// `head` that is a compound state — or the `<scxml>` element, `None` — and
/// contains every state of `tail`.
///
/// Asked of the source and EVERY target at once: combining pairwise answers
/// can only widen the domain.
fn find_lcca<D: Document>(doc: &D, head: D::State, tail: &[D::State]) -> Option<D::State> {
    let mut current = head;
    for _ in 0..D::MAX_DEPTH {
        let ancestor = doc.parent_of(current)?;
        // §scxml-D-findLCCA filters the ancestors with
        // isCompoundStateOrScxmlElement: a `<parallel>` is never a domain.
        if doc.is_compound(ancestor)
            && tail
                .iter()
                .all(|&state| is_descendant(doc, state, ancestor))
        {
            return Some(ancestor);
        }
        current = ancestor;
    }
    panic!(
        "find_lcca: cyclic parent relationship detected walking from {:?}",
        head
    );
}

/// Appendix D's computeExitSet, for one transition: the active proper
/// descendants of its domain.
///
/// Returned in the configuration's own order.
pub fn compute_exit_set<D: Document>(
    doc: &D,
    transition: &EnabledTransition<D::State, D::History>,
    configuration: &[D::State],
) -> D::Configuration {
    let mut exit_set = D::Configuration::default();

    // §scxml-D-computeExitSet: the appendix guards the whole computation with
    // `if t.target`, so a transition without one exits nothing at all and can
    // therefore never be preempted.
    if transition.is_targetless() {
        return exit_set;
    }

    let targets = effective_target_states(doc, transition.targets);
    let domain = transition_domain(
        doc,
        Some(transition.source),
        &targets,
        transition.is_internal,
    );
    for &state in configuration {
        let exits = match domain {
            // The domain itself is not exited; everything active below it is.
            Some(domain) => is_descendant(doc, state, domain),
            // The domain is the `<scxml>` element and every active state is a
            // descendant of it — the sibling regions of an enclosing
            // `<parallel>` included.
            None => true,
        };
        if exits {
            exit_set.push_bounded(state);
        }
    }
    exit_set
}

/// Appendix D's computeExitSet over the microstep's transitions, in exitOrder —
/// the states exitStates exits.
pub fn compute_states_to_exit<D: Document>(
    doc: &D,
    transitions: &[EnabledTransition<D::State, D::History>],
    configuration: &[D::State],
) -> D::Configuration {
    // §scxml-D-computeExitSet takes the microstep's whole transition list and
    // unions the exit sets; §scxml-D-exitStates exits that union in exitOrder —
    // descendants before their ancestors and reverse document order among the
    // rest, which together are exactly reverse document order.
    let mut states_to_exit = D::Configuration::default();
    for transition in transitions {
        for &state in compute_exit_set(doc, transition, configuration).iter() {
            add_once(&mut states_to_exit, state);
        }
    }
    sort_in_exit_order(doc, &mut states_to_exit);
    states_to_exit
}

/// Appendix D's exitOrder: descendants before their ancestors and reverse
/// document order among the rest, which together are exactly reverse document
/// order. The one sort both exitStates and exitInterpreter exit by.
fn sort_in_exit_order<D: Document>(doc: &D, states: &mut [D::State]) {
    stable_sort_by(states, |a, b| {
        doc.document_order(*b).cmp(&doc.document_order(*a))
    });
}

// ════════════════════════════════════════════════════════════════════════
// The microstep
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's microstep: exit, run the transitions' content, enter.
///
/// Returns what was entered — the entry set, `states_to_enter` in entry order.
pub fn microstep<R: Run>(
    run: &mut R,
    transitions: &[EnabledTransition<R::State, R::History>],
) -> EntrySetOf<R> {
    // §scxml-D-microstepProcedure: every exit, then every transition's
    // content, then every entry — for the whole set at once, which is what lets
    // the regions of a `<parallel>` each take their own transition in one step.
    exit_states(run, transitions);
    execute_transition_content(run, transitions);
    // Each transition is read as the entry procedures read it, one at a time as
    // it is reached, and never gathered into a list of those: that list was a
    // buffer of the enabled bound on the stack of the deepest call chain, the
    // stack the entered states' own `<onentry>` code then ran on top of.
    let mut entry = EntrySet::new();
    for transition in transitions {
        add_transition_to_entry_set(run, &transition.entry_transition(), &mut entry);
    }
    sort_in_entry_order(run, &mut entry);
    enter_entry_set(run, &entry);
    entry
}

/// Appendix D's exitStates.
pub fn exit_states<R: Run>(run: &mut R, transitions: &[EnabledTransition<R::State, R::History>]) {
    // §scxml-D-exitStates: the union of the transitions' exit sets, exited in
    // exitOrder. Every history is recorded from the configuration as it stood
    // BEFORE the first exit, which is the snapshot each exit is handed.
    let configuration = run.configuration();
    let states_to_exit = compute_states_to_exit(run, transitions, &configuration);
    for &state in states_to_exit.iter() {
        run.exit_state(state, &configuration);
    }
}

/// Appendix D's exitInterpreter (§scxml-D-exitInterpreter): every state still
/// in the configuration, exited in exitOrder, each the way exitStates exits one.
///
/// ```text
/// statesToExit = configuration.toList().sort(exitOrder)
/// for s in statesToExit: onexit, cancel its invocations, configuration.delete(s)
/// ```
///
/// Reached two ways, the two the procedure names: the main event loop ends
/// because the run entered a top-level `<final>`, or the host stops a run that
/// has not ended. The list is sorted in place and is also the configuration
/// each exit records history from — that reads it as a set, so its order is
/// free, and one list rather than a copy is bytes every machine pays for.
pub fn exit_interpreter<R: Run>(run: &mut R) {
    let mut states_to_exit = run.configuration();
    sort_in_exit_order(run, &mut states_to_exit);
    for &state in states_to_exit.iter() {
        run.exit_state(state, &states_to_exit);
    }
}

/// Appendix D's executeTransitionContent.
pub fn execute_transition_content<R: Run>(
    run: &mut R,
    transitions: &[EnabledTransition<R::State, R::History>],
) {
    // §scxml-D-executeTransitionContent: in the order the transitions were
    // selected, which is not their sources' document order once an ancestor's
    // transition is reached from a later region.
    for transition in transitions {
        if transition.has_actions {
            run.execute_transition_content(transition);
        }
    }
}

/// Appendix D's enterStates.
///
/// Also the whole of the appendix's entry into the initial configuration
/// (§scxml-D-interpret): hand it the document's initial transition, whose
/// source is the `<scxml>` element (`source` `None`).
pub fn enter_states<R: Run>(
    run: &mut R,
    transitions: &[EntryTransition<R::State, R::History>],
) -> EntrySetOf<R> {
    let entry = compute_entry_set(run, transitions);
    enter_entry_set(run, &entry);
    entry
}

/// The second half of Appendix D's enterStates: every state of an entry set
/// entered, in entry order. The set is lent, not passed on, so a caller that
/// hands it back holds one copy of it and not two.
fn enter_entry_set<R: Run>(run: &mut R, entry: &EntrySetOf<R>) {
    for &state in entry.states_to_enter.iter() {
        // §scxml-D-enterStates: onentry, then the initial transition's content
        // if and only if this state's initial state is being entered by
        // default, then a history's default content owed to it.
        run.enter_state(state, entry.is_default_entry(state));
        if let Some(history) = entry.default_history_content_of(state) {
            run.execute_history_default_content(history);
        }
    }
}

// ════════════════════════════════════════════════════════════════════════
// The entry set
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's computeEntrySet.
///
/// `transitions` are the microstep's, in the order they were selected; a
/// transition without targets enters nothing. Returns the entry set,
/// `states_to_enter` sorted into entry order.
pub fn compute_entry_set<D: Document>(
    doc: &D,
    transitions: &[EntryTransition<D::State, D::History>],
) -> EntrySetOf<D> {
    let mut entry = EntrySet::new();
    for transition in transitions {
        add_transition_to_entry_set(doc, transition, &mut entry);
    }
    sort_in_entry_order(doc, &mut entry);
    entry
}

/// One transition's part of Appendix D's computeEntrySet: what it adds to the
/// entry set, before the set is put into entry order.
fn add_transition_to_entry_set<D: Document>(
    doc: &D,
    transition: &EntryTransition<D::State, D::History>,
    entry: &mut EntrySetOf<D>,
) {
    // §scxml-D-computeEntrySet: first every target with its default
    // descendants, then the ancestors that are entered inside the domain —
    // ancestors outside it were never exited. The order matters for a
    // target SET: the regions of a `<parallel>` that another target already
    // descends into must be seen as taken before the ancestor walk fills
    // the rest with defaults.
    for &target in transition.targets {
        add_descendant_states_to_enter(doc, target, entry);
    }
    let effective = effective_target_states(doc, transition.targets);
    if effective.is_empty() {
        return;
    }
    let domain = transition_domain(doc, transition.source, &effective, transition.is_internal);
    for &state in effective.iter() {
        add_ancestor_states_to_enter(doc, EntryTarget::State(state), domain, entry);
    }
}

/// §scxml-D-enterStates: states are entered in entryOrder — ancestors before
/// descendants, document order between the rest, which is exactly document
/// order, because that is a pre-order walk.
fn sort_in_entry_order<D: Document>(doc: &D, entry: &mut EntrySetOf<D>) {
    stable_sort_by_key(&mut entry.states_to_enter, |&state| {
        doc.document_order(state)
    });
}

/// Appendix D's addDescendantStatesToEnter.
///
/// Adds `target` and every descendant entering it enters: a history's recorded
/// or default configuration, a compound state's initial state(s), every region
/// of a `<parallel>` not already on the set.
pub fn add_descendant_states_to_enter<D: Document>(
    doc: &D,
    target: EntryTarget<D::State, D::History>,
    entry: &mut EntrySetOf<D>,
) {
    let state = match target {
        EntryTarget::State(state) => state,
        EntryTarget::History(history) => {
            let parent = doc.history_parent(history);
            // §scxml-3.10: a transition to a history behaves as a transition to
            // the configuration it stored, or — before its parent was ever
            // visited — to its default stored configuration.
            match doc.history_value(history) {
                Some(recorded) if !recorded.is_empty() => {
                    // §scxml-D-addDescendantStatesToEnter: a history that has
                    // recorded a configuration enters it, and the ancestors
                    // between it and the history's parent — a deep history
                    // records atomic states, which need not be children.
                    for &state in recorded {
                        add_descendant_states_to_enter(doc, EntryTarget::State(state), entry);
                    }
                    for &state in recorded {
                        add_ancestor_states_to_enter(
                            doc,
                            EntryTarget::State(state),
                            Some(parent),
                            entry,
                        );
                    }
                }
                _ => {
                    // §scxml-D-addDescendantStatesToEnter: nothing recorded
                    // yet, so the default transition is taken, and its content
                    // is owed once the parent has been entered — keyed by the
                    // parent, a later history of the same parent replacing it.
                    set_default_history_content(
                        &mut entry.default_history_content,
                        parent,
                        history,
                    );
                    let defaults = doc.history_default_targets(history);
                    for &default in defaults {
                        add_descendant_states_to_enter(doc, default, entry);
                    }
                    for &default in defaults {
                        add_ancestor_states_to_enter(doc, default, Some(parent), entry);
                    }
                }
            }
            return;
        }
    };

    add_once(&mut entry.states_to_enter, state);
    if doc.is_compound(state) {
        // §scxml-D-addDescendantStatesToEnter: a compound state entered as a
        // target is entered by DEFAULT — the one condition under which its
        // initial transition's content runs. Its initial transition names the
        // child or children it enters (§scxml-3.3), possibly several and
        // possibly deep (§scxml-3.6).
        add_once(&mut entry.states_for_default_entry, state);
        let initial = doc.initial_targets(state);
        for &child in initial {
            add_descendant_states_to_enter(doc, child, entry);
        }
        for &child in initial {
            add_ancestor_states_to_enter(doc, child, Some(state), entry);
        }
    } else if doc.is_parallel(state) {
        add_region_defaults(doc, state, entry);
    }
}

/// Appendix D's addAncestorStatesToEnter.
///
/// Adds the proper ancestors of `target` up to, not including, `ancestor`
/// (`None`: up to the `<scxml>` element), filling in the regions of every
/// `<parallel>` among them.
pub fn add_ancestor_states_to_enter<D: Document>(
    doc: &D,
    target: EntryTarget<D::State, D::History>,
    ancestor: Option<D::State>,
    entry: &mut EntrySetOf<D>,
) {
    for &anc in proper_ancestors(doc, target, ancestor).iter() {
        // §scxml-D-addAncestorStatesToEnter: an ancestor is entered WITHOUT its
        // default initial state — the set already holds the descendant it leads
        // to. A `<parallel>` still gives its other regions their defaults,
        // because all of them are entered.
        add_once(&mut entry.states_to_enter, anc);
        if doc.is_parallel(anc) {
            add_region_defaults(doc, anc, entry);
        }
    }
}

/// Appendix D's getProperAncestors: the ancestors of `target` in ancestry
/// order up to, not including, `ancestor` — and nothing at all when `ancestor`
/// is not above it.
///
/// A `<history>` sits in its parent, so the parent is its first proper
/// ancestor. `ancestor` `None` is the `<scxml>` element, which is above every
/// state and is never entered, so it is not in the list.
fn proper_ancestors<D: Document>(
    doc: &D,
    target: EntryTarget<D::State, D::History>,
    ancestor: Option<D::State>,
) -> D::Configuration {
    let mut chain = D::Configuration::default();
    let mut current = match target {
        EntryTarget::State(state) => doc.parent_of(state),
        EntryTarget::History(history) => Some(doc.history_parent(history)),
    };
    for _ in 0..D::MAX_DEPTH {
        let Some(state) = current else {
            // §scxml-D-getProperAncestors: `ancestor` is the state's parent,
            // the state itself, or one of its descendants — none of which the
            // walk can meet — so the answer is the empty set.
            if ancestor.is_some() {
                chain.clear();
            }
            return chain;
        };
        if ancestor == Some(state) {
            return chain;
        }
        chain.push_bounded(state);
        current = doc.parent_of(state);
    }
    panic!(
        "proper_ancestors: cyclic parent relationship detected walking from {:?}",
        target
    );
}

fn add_region_defaults<D: Document>(doc: &D, parallel: D::State, entry: &mut EntrySetOf<D>) {
    // §scxml-3.4: every child of an active `<parallel>` is active, so a region
    // nothing on the set descends into is entered by default.
    for &child in doc.child_states(parallel) {
        let taken = entry
            .states_to_enter
            .iter()
            .any(|&state| is_descendant(doc, state, child));
        if !taken {
            add_descendant_states_to_enter(doc, EntryTarget::State(child), entry);
        }
    }
}

fn set_default_history_content<S: PartialEq, H, X: Bounded<(S, H)>>(
    content: &mut X,
    parent: S,
    history: H,
) {
    for slot in content.iter_mut() {
        if slot.0 == parent {
            slot.1 = history;
            return;
        }
    }
    content.push_bounded((parent, history));
}

// ════════════════════════════════════════════════════════════════════════
// History
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's exitStates, its history half: what a `<history>` of `parent`
/// records as `parent` is exited — for a deep history the active atomic states
/// below `parent`, for a shallow one its active children.
///
/// Read off `configuration_before_exit`, the configuration as it stood before
/// the microstep's first exit: the appendix records every history before it
/// exits any state, so every history of one microstep reads the same
/// configuration. Returned in that configuration's order.
pub fn recorded_history<D: Document>(
    doc: &D,
    parent: D::State,
    deep: bool,
    configuration_before_exit: &[D::State],
) -> D::Configuration {
    let mut recorded = D::Configuration::default();
    for &state in configuration_before_exit {
        let records = if deep {
            // §scxml-D-exitStates: `isAtomicState(s0) and isDescendant(s0, s)`.
            !doc.is_compound(state) && !doc.is_parallel(state) && is_descendant(doc, state, parent)
        } else {
            // §scxml-D-exitStates: `s0.parent == s`.
            doc.parent_of(state) == Some(parent)
        };
        if records {
            recorded.push_bounded(state);
        }
    }
    recorded
}

// ════════════════════════════════════════════════════════════════════════
// Predicates
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's isInFinalState: a compound state is in a final state when one
/// of its `<final>` children is active; a `<parallel>` when EVERY one of its
/// child states is — asked recursively, so a region that is itself a
/// `<parallel>` counts only once all of ITS regions do. Nothing else ever is.
pub fn is_in_final_state<D: Document>(
    doc: &D,
    state: D::State,
    configuration: &[D::State],
) -> bool {
    if doc.is_compound(state) {
        doc.child_states(state)
            .iter()
            .any(|&child| doc.is_final(child) && configuration.contains(&child))
    } else if doc.is_parallel(state) {
        doc.child_states(state)
            .iter()
            .all(|&child| is_in_final_state(doc, child, configuration))
    } else {
        false
    }
}

/// Appendix D's isDescendant: whether `state` lies strictly below `ancestor`.
pub fn is_descendant<D: Document>(doc: &D, state: D::State, ancestor: D::State) -> bool {
    let mut current = state;
    for _ in 0..D::MAX_DEPTH {
        match doc.parent_of(current) {
            None => return false,
            Some(parent) if parent == ancestor => return true,
            Some(parent) => current = parent,
        }
    }
    panic!(
        "is_descendant: cyclic parent relationship detected walking from {:?}",
        state
    );
}

fn intersects<S: PartialEq>(a: &[S], b: &[S]) -> bool {
    a.iter().any(|state| b.contains(state))
}

fn add_once<S: Copy + PartialEq + Debug, B: Bounded<S>>(set: &mut B, state: S) {
    if !set.contains(&state) {
        set.push_bounded(state);
    }
}
