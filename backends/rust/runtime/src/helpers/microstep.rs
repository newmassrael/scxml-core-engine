// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! W3C SCXML Appendix D's microstep — which transitions an event selects,
//! which of them survive preemption, which states they exit and in which
//! order, the order their content runs in, which states they enter — compiled
//! once for every program, whatever machines it holds (docs/adr/0064).
//!
//! The procedures are transcribed rather than paraphrased, one function per
//! procedure and under the appendix's names, so a reader can hold this file
//! against the specification line by line — and against the C++ engines'
//! shared transcription (`sce/include/core/MicrostepAlgorithms.h`,
//! `EntrySetHelper.h`, `ParallelTransitionHelper.h`,
//! `ConflictResolutionHelper.h`, `ParallelCompletionHelper.h`), which answers
//! the same questions the same way.
//!
//! No function here is generic over a machine. The document reaches them as
//! its tables ([`DocumentTables`]), its states, histories, transitions and
//! events as numbers, and the running machine as one object-safe trait,
//! [`Behaviour`]: its configuration and history records, which of a state's
//! transitions an event enables, and what exiting a state, running a
//! transition's content and entering a state DO. So a program that holds two
//! machines holds one copy of this module.
//!
//! # Capacity
//!
//! Every set a procedure builds is a [`SliceVec`] over the [`Scratch`] the
//! caller lends: the machine's [`Workspace`], one buffer of each kind, of the
//! size the generator read from the document's tree (docs/adr/0056). A
//! procedure takes what it needs from the front of the scratch and hands the
//! rest to the procedures it calls, so the most a microstep holds at once is a
//! sum the generator can write: [`state_scratch`] states, the enabled set, the
//! walk's steps and the history defaults. No capacity constant is introduced
//! here, and a push past a buffer's end stops the program with the generator's
//! name on it.

use crate::helpers::tables::{
    DocumentTables, EventId, HistoryId, StateId, StateKind, Target, TransitionId,
};

// ════════════════════════════════════════════════════════════════════════
// Buffers
// ════════════════════════════════════════════════════════════════════════

/// The states the scratch of a document whose configurations hold at most
/// `max_configuration` states must have room for.
///
/// Three sets of the configuration's size are live at once on the longest path
/// of a microstep: exitStates holds the configuration it started from and the
/// states it exits while it computes one transition's effective targets, and
/// the entry set holds the states to enter and the states entered by default
/// while it computes the same. The generator writes the workspace's size as a
/// call of this function, so the rule is stated once.
pub const fn state_scratch(max_configuration: usize) -> usize {
    3 * max_configuration
}

/// A buffer of a fixed capacity over borrowed slots: what a procedure builds a
/// set of states, of transitions or of steps in.
#[derive(Debug)]
pub struct SliceVec<'a, T> {
    slots: &'a mut [T],
    len: usize,
}

impl<'a, T: Copy + PartialEq> SliceVec<'a, T> {
    /// An empty buffer over `slots`.
    pub fn new(slots: &'a mut [T]) -> Self {
        Self { slots, len: 0 }
    }

    /// Add `item` at the end.
    #[inline]
    pub fn push(&mut self, item: T) {
        let Some(slot) = self.slots.get_mut(self.len) else {
            workspace_too_small()
        };
        *slot = item;
        self.len += 1;
    }

    /// Add `item` at the end unless it is already held.
    #[inline]
    pub fn add_once(&mut self, item: T) {
        if !self.contains(&item) {
            self.push(item);
        }
    }

    /// Remove the last item.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.slots[self.len])
    }

    /// Keep the first `len` items.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        self.len = self.len.min(len);
    }

    /// Remove every item.
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<T> core::ops::Deref for SliceVec<'_, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        &self.slots[..self.len]
    }
}

impl<T> core::ops::DerefMut for SliceVec<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.slots[..self.len]
    }
}

/// A push past the end of a buffer: the generator sized the workspace below
/// what the document needs. No formatted message, so a bare-metal image does
/// not carry the formatting machinery for a branch a correct generator never
/// reaches.
#[cold]
#[inline(never)]
fn workspace_too_small() -> ! {
    panic!("a microstep buffer is full: sce-codegen sized the machine's workspace too small")
}

/// The buffers a microstep borrows, each a slice of the machine's
/// [`Workspace`].
#[derive(Debug)]
pub struct Scratch<'a> {
    /// Sets of states.
    pub states: &'a mut [StateId],
    /// The steps of the walks.
    pub walk: &'a mut [WalkStep],
    /// The history default content an entry set owes.
    pub history_defaults: &'a mut [(StateId, HistoryId)],
}

impl<'a> Scratch<'a> {
    /// The same buffers, for a call that returns them.
    #[inline]
    pub fn reborrow(&mut self) -> Scratch<'_> {
        Scratch {
            states: &mut *self.states,
            walk: &mut *self.walk,
            history_defaults: &mut *self.history_defaults,
        }
    }

    /// A buffer of `count` states from the front, and the rest.
    #[inline]
    fn take_states(self, count: usize) -> (SliceVec<'a, StateId>, Scratch<'a>) {
        if count > self.states.len() {
            workspace_too_small()
        }
        let (taken, states) = self.states.split_at_mut(count);
        let rest = Scratch {
            states,
            walk: self.walk,
            history_defaults: self.history_defaults,
        };
        (SliceVec::new(taken), rest)
    }
}

/// A machine's buffers for one call into this module, sized by its document:
/// `STATES` is [`state_scratch`] of its configuration bound, `ENABLED` the most
/// transitions a selection enables, `WALK` the most steps the walks owe and
/// `DEFAULTS` the most history default transitions a microstep owes. The
/// generated policy spells its own as `StatePolicy::Workspace`.
///
/// A local of the call that runs a macrostep, never a field of the engine, so
/// it costs stack only while a macrostep runs (docs/adr/0056 §4).
#[derive(Debug, Clone)]
pub struct Workspace<
    const STATES: usize,
    const ENABLED: usize,
    const WALK: usize,
    const DEFAULTS: usize,
> {
    states: [StateId; STATES],
    enabled: [TransitionId; ENABLED],
    walk: [WalkStep; WALK],
    history_defaults: [(StateId, HistoryId); DEFAULTS],
}

impl<const STATES: usize, const ENABLED: usize, const WALK: usize, const DEFAULTS: usize>
    Workspace<STATES, ENABLED, WALK, DEFAULTS>
{
    /// Empty buffers.
    pub const fn new() -> Self {
        Self {
            states: [StateId(0); STATES],
            enabled: [TransitionId(0); ENABLED],
            walk: [WalkStep::Descendants(Target::State(StateId(0))); WALK],
            history_defaults: [(StateId(0), HistoryId(0)); DEFAULTS],
        }
    }
}

impl<const STATES: usize, const ENABLED: usize, const WALK: usize, const DEFAULTS: usize> Default
    for Workspace<STATES, ENABLED, WALK, DEFAULTS>
{
    fn default() -> Self {
        Self::new()
    }
}

/// A [`Workspace`] of any size, as the engine lends it.
pub trait Buffers {
    /// The buffer of the enabled set, and the scratch for everything else.
    fn split(&mut self) -> (SliceVec<'_, TransitionId>, Scratch<'_>);
}

impl<const STATES: usize, const ENABLED: usize, const WALK: usize, const DEFAULTS: usize> Buffers
    for Workspace<STATES, ENABLED, WALK, DEFAULTS>
{
    #[inline]
    fn split(&mut self) -> (SliceVec<'_, TransitionId>, Scratch<'_>) {
        (
            SliceVec::new(&mut self.enabled),
            Scratch {
                states: &mut self.states,
                walk: &mut self.walk,
                history_defaults: &mut self.history_defaults,
            },
        )
    }
}

// ════════════════════════════════════════════════════════════════════════
// The machine
// ════════════════════════════════════════════════════════════════════════

/// The running machine, as the procedures drive it.
///
/// Object-safe, and called as `&mut dyn Behaviour`, so the procedures are one
/// copy for every machine. The engine implements it once for each machine, as
/// the conversion between these numbers and the machine's own types.
pub trait Behaviour {
    /// The active states, in the order they were added.
    fn configuration(&self) -> &[StateId];

    /// Add `state` to the configuration; `false` when it was already there.
    fn add_to_configuration(&mut self, state: StateId) -> bool;

    /// Remove `state` from the configuration.
    fn remove_from_configuration(&mut self, state: StateId);

    /// What `history` recorded when its parent was last exited; `None` before
    /// that ever happened.
    fn history_value(&self, history: HistoryId) -> Option<&[StateId]>;

    /// Keep `states` as what `history` recorded.
    fn record_history(&mut self, history: HistoryId, states: &[StateId]);

    /// This state's first transition, in document order, that the event
    /// enables and whose guard holds; for the machine's "no event", its first
    /// eventless transition whose guard holds. The only place a guard is
    /// evaluated.
    fn first_enabled_transition(&mut self, state: StateId, event: EventId) -> Option<TransitionId>;

    /// Run the state's onexit and cancel its invocations. The procedure removes
    /// it from the configuration afterwards.
    fn exit_state(&mut self, state: StateId);

    /// Run one transition's executable content.
    fn execute_transition_content(&mut self, transition: TransitionId);

    /// Schedule the state's invocations, run its onentry, then its initial
    /// transition's content when `is_default_entry`; for a `<final>`, what the
    /// appendix does on entering one. The procedure has added it to the
    /// configuration already.
    fn enter_state(&mut self, state: StateId, is_default_entry: bool);

    /// Run a `<history>`'s default transition content.
    fn execute_history_default_content(&mut self, history: HistoryId);
}

// ════════════════════════════════════════════════════════════════════════
// The walks' steps
// ════════════════════════════════════════════════════════════════════════

/// A list of targets the entry walk goes through twice, as Appendix D's
/// addDescendantStatesToEnter does: every descendant first, then every ancestor.
///
/// Named by the state or the `<history>` it belongs to and read from the tables
/// at each step, so a step of the walk holds an index and not a slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listed {
    /// A compound state's initial transition targets.
    Initial(StateId),
    /// The states a `<history>` recorded.
    Recorded(HistoryId),
    /// The targets of a `<history>`'s default transition.
    Defaults(HistoryId),
}

/// One step a walk over the document still owes: Appendix D writes the entry set
/// (addDescendantStatesToEnter, addAncestorStatesToEnter) and the effective
/// targets (getEffectiveTargetStates) as recursions, and a bare-metal program
/// cannot say how deep a recursion goes without assuming it. Each is run here
/// over a stack of these steps instead, in the order the recursion would have
/// run them, so the stack a call needs is a number of the document and the
/// machine's worst-case stack is the sum of its frames, with nothing stated
/// beyond them.
///
/// A step that has more to do after the step it spawns puts itself back first,
/// with its next index, so a stack holds one step for each level the recursion
/// would have been at, and never a list of siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkStep {
    /// addDescendantStatesToEnter(target).
    Descendants(Target),
    /// addAncestorStatesToEnter(target, stop): the ancestors of `target` below
    /// `stop` (`None`: up to the `<scxml>` element).
    Ancestors(Target, Option<StateId>),
    /// What is left of an addAncestorStatesToEnter loop: `from` is the next
    /// ancestor to enter.
    AncestorsFrom {
        /// The next ancestor to enter.
        from: StateId,
        /// The state the loop ends below.
        stop: Option<StateId>,
    },
    /// The regions of a `<parallel>` from child `index` on, each entered by
    /// default unless the entry set already goes into it.
    Regions {
        /// The `<parallel>`.
        parallel: StateId,
        /// The next child to look at.
        index: u16,
    },
    /// A list of targets, in the pass `ancestors` names, from `index` on.
    Listed {
        /// Whose list it is.
        source: Listed,
        /// The next target of the list.
        index: u16,
        /// Whether this is the ancestors pass (the descendants pass otherwise).
        ancestors: bool,
    },
    /// One target of getEffectiveTargetStates.
    EffectiveTarget(Target),
    /// What is left of the default transition targets of a `<history>` that
    /// recorded nothing, from `index` on.
    EffectiveDefaults {
        /// The `<history>`.
        history: HistoryId,
        /// The next target of its default transition.
        index: u16,
    },
}

// ════════════════════════════════════════════════════════════════════════
// Selection
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's selectTransitions, or its selectEventlessTransitions when
/// `event` is the machine's "no event".
///
/// Leaves the optimal enabled transition set in `enabled`, in selection order.
pub fn select_transitions(
    tables: &DocumentTables,
    machine: &mut dyn Behaviour,
    event: EventId,
    enabled: &mut SliceVec<'_, TransitionId>,
    scratch: Scratch<'_>,
) {
    enabled.clear();
    let capacity = usize::from(tables.max_configuration);
    let (mut configuration, mut rest) = scratch.take_states(capacity);
    for &state in machine.configuration() {
        configuration.push(state);
    }

    {
        let (mut atomic_states, _) = rest.reborrow().take_states(capacity);
        for &state in configuration.iter() {
            if tables.is_atomic(state) {
                atomic_states.push(state);
            }
        }
        // Document order is the order of the ids.
        atomic_states.sort_unstable();

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
                if let Some(found) = machine.first_enabled_transition(state, event) {
                    enabled.add_once(found);
                    break;
                }
                current = tables.parent(state);
            }
        }
    }
    remove_conflicting_transitions(tables, &*machine, enabled, &configuration, rest);
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
pub fn remove_conflicting_transitions(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    enabled: &mut SliceVec<'_, TransitionId>,
    configuration: &[StateId],
    mut scratch: Scratch<'_>,
) {
    let mut kept = 0;
    for candidate in 0..enabled.len() {
        let t1 = enabled[candidate];
        // A kept transition's exit set is asked for again for each later one
        // held against it rather than stored beside it: on an MCU the stack a
        // stored set per transition costs is scarcer than the cycles, and the
        // answer is read off the one configuration the selection saw.
        let t1_exits = compute_exit_set(tables, machine, t1, scratch.reborrow());
        let mut conflicts = |t2: TransitionId| {
            let t2_exits = compute_exit_set(tables, machine, t2, scratch.reborrow());
            t1_exits.meets(tables, t2_exits, configuration)
        };
        let t1_source = tables.transition(t1).source;
        // §scxml-D-removeConflictingTransitions: t1 is preempted by any kept
        // transition it conflicts with whose source it does not descend from.
        // A transition that exits nothing — a targetless one — conflicts with
        // nothing and can never be preempted.
        let preempted = enabled[..kept]
            .iter()
            .any(|&t2| conflicts(t2) && !descends(tables, t1_source, tables.transition(t2).source));
        if preempted {
            continue;
        }
        // Not preempted means every kept transition t1 conflicts with has a
        // source t1 descends from: the appendix removes exactly those.
        let mut surviving = 0;
        for index in 0..kept {
            let t2 = enabled[index];
            if !conflicts(t2) {
                enabled[surviving] = t2;
                surviving += 1;
            }
        }
        enabled[surviving] = t1;
        kept = surviving + 1;
    }
    enabled.truncate(kept);
}

/// Whether `state` lies strictly below `ancestor`, both sources of transitions:
/// the `<scxml>` element (`None`) is below nothing.
fn descends(tables: &DocumentTables, state: Option<StateId>, ancestor: Option<StateId>) -> bool {
    match (state, ancestor) {
        (Some(state), Some(ancestor)) => tables.is_descendant(state, ancestor),
        _ => false,
    }
}

// ════════════════════════════════════════════════════════════════════════
// Domains and exit sets
// ════════════════════════════════════════════════════════════════════════

/// Which active states a transition exits: Appendix D's computeExitSet for
/// one transition, before it is read off a configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitScope {
    /// None: the transition has no targets.
    Nothing,
    /// Every active state: the domain is the `<scxml>` element.
    All,
    /// Every active proper descendant of the domain.
    Below(StateId),
}

impl ExitScope {
    /// Whether a transition of this scope exits `state` when it is active.
    #[inline]
    pub fn contains(self, tables: &DocumentTables, state: StateId) -> bool {
        match self {
            ExitScope::Nothing => false,
            ExitScope::All => true,
            ExitScope::Below(domain) => tables.is_descendant(state, domain),
        }
    }

    /// Whether the exit sets of two transitions, read off `configuration`,
    /// share a state.
    ///
    /// A subtree is a range of ids, and two ranges of one tree are nested or
    /// apart, so the states both scopes exit are the states the narrower one
    /// exits, or none.
    pub fn meets(
        self,
        tables: &DocumentTables,
        other: ExitScope,
        configuration: &[StateId],
    ) -> bool {
        let common = match (self, other) {
            (ExitScope::Nothing, _) | (_, ExitScope::Nothing) => return false,
            (ExitScope::All, scope) | (scope, ExitScope::All) => scope,
            (ExitScope::Below(a), ExitScope::Below(b)) => {
                if a == b || tables.is_descendant(b, a) {
                    ExitScope::Below(b)
                } else if tables.is_descendant(a, b) {
                    ExitScope::Below(a)
                } else {
                    return false;
                }
            }
        };
        configuration
            .iter()
            .any(|&state| common.contains(tables, state))
    }
}

/// Appendix D's computeExitSet for one transition, as the scope a
/// configuration is read through: the active proper descendants of its domain.
pub fn compute_exit_set(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    transition: TransitionId,
    scratch: Scratch<'_>,
) -> ExitScope {
    let row = tables.transition(transition);
    let targets = tables.targets(transition);
    // §scxml-D-computeExitSet: the appendix guards the whole computation with
    // `if t.target`, so a transition without one exits nothing at all and can
    // therefore never be preempted.
    if targets.is_empty() {
        return ExitScope::Nothing;
    }
    let (mut effective, rest) = scratch.take_states(usize::from(tables.max_configuration));
    effective_target_states_on(tables, machine, targets, &mut effective, rest);
    match transition_domain(tables, row.source, &effective, row.internal) {
        Some(domain) => ExitScope::Below(domain),
        None => ExitScope::All,
    }
}

/// Appendix D's getEffectiveTargetStates, over a target list, into `effective`,
/// on the walk stack of the scratch the caller lends.
///
/// The states the list stands for, histories dereferenced — to what each
/// recorded or, before its parent was ever exited, to its default — each state
/// once and in the order first named. The domain, and so the exit set, is a
/// question about these.
pub fn effective_target_states_on(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    targets: &[Target],
    effective: &mut SliceVec<'_, StateId>,
    scratch: Scratch<'_>,
) {
    effective.clear();
    let mut stack = SliceVec::new(scratch.walk);
    for &target in targets {
        stack.push(WalkStep::EffectiveTarget(target));
        while let Some(step) = stack.pop() {
            match step {
                WalkStep::EffectiveTarget(Target::State(state)) => effective.add_once(state),
                // §scxml-D-getEffectiveTargetStates: a history stands for the
                // configuration it recorded, and before its parent was ever
                // exited for the targets of its default transition — which may
                // name histories themselves, so those are walked in turn.
                WalkStep::EffectiveTarget(Target::History(history)) => {
                    match machine.history_value(history) {
                        Some(recorded) if !recorded.is_empty() => {
                            for &state in recorded {
                                effective.add_once(state);
                            }
                        }
                        _ => stack.push(WalkStep::EffectiveDefaults { history, index: 0 }),
                    }
                }
                WalkStep::EffectiveDefaults { history, index } => {
                    let defaults = tables.targets(tables.history(history).default);
                    if let Some(&next) = defaults.get(usize::from(index)) {
                        stack.push(WalkStep::EffectiveDefaults {
                            history,
                            index: index + 1,
                        });
                        stack.push(WalkStep::EffectiveTarget(next));
                    }
                }
                // The walk puts down steps of its own kind only. No message: a
                // formatted one would bring the formatting machinery into every
                // bare-metal image for a branch nothing reaches.
                _ => unreachable!(),
            }
        }
    }
}

/// Appendix D's getTransitionDomain, for a transition that has targets.
///
/// `targets` are the transition's EFFECTIVE targets: a history that recorded a
/// state deep inside its parent is a target that deep. Returns the domain, or
/// `None` when it is the `<scxml>` element — always so for the document's
/// initial transition (`source` `None`).
pub fn transition_domain(
    tables: &DocumentTables,
    source: Option<StateId>,
    targets: &[StateId],
    is_internal: bool,
) -> Option<StateId> {
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
        && tables.is_compound(source)
        && targets.iter().all(|&t| tables.is_descendant(t, source))
    {
        return Some(source);
    }
    find_lcca(tables, source, targets)
}

/// Appendix D's findLCCA over `[head] + tail`: the first proper ancestor of
/// `head` that is a compound state — or the `<scxml>` element, `None` — and
/// contains every state of `tail`.
///
/// Asked of the source and EVERY target at once: combining pairwise answers
/// can only widen the domain. A parent's id is below its child's, so the walk
/// up ends.
fn find_lcca(tables: &DocumentTables, head: StateId, tail: &[StateId]) -> Option<StateId> {
    let mut current = head;
    loop {
        let ancestor = tables.parent(current)?;
        // §scxml-D-findLCCA filters the ancestors with
        // isCompoundStateOrScxmlElement: a `<parallel>` is never a domain.
        if tables.is_compound(ancestor)
            && tail
                .iter()
                .all(|&state| tables.is_descendant(state, ancestor))
        {
            return Some(ancestor);
        }
        current = ancestor;
    }
}

// ════════════════════════════════════════════════════════════════════════
// The microstep
// ════════════════════════════════════════════════════════════════════════

/// What a microstep or an entry into a configuration did, as the engine reports
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entered {
    /// How many states the entry set held.
    pub states: usize,
    /// The atomic state the last targeted transition brought the machine
    /// into; `None` when nothing was entered.
    pub current: Option<StateId>,
}

/// Appendix D's microstep: exit, run the transitions' content, enter.
pub fn microstep(
    tables: &DocumentTables,
    machine: &mut dyn Behaviour,
    transitions: &[TransitionId],
    mut scratch: Scratch<'_>,
) -> Entered {
    // §scxml-D-microstepProcedure: every exit, then every transition's
    // content, then every entry — for the whole set at once, which is what lets
    // the regions of a `<parallel>` each take their own transition in one step.
    exit_states(tables, machine, transitions, scratch.reborrow());
    execute_transition_content(tables, machine, transitions);
    enter_states(tables, machine, transitions, scratch)
}

/// Appendix D's computeExitSet over the microstep's transitions, in exitOrder —
/// the states exitStates exits — read off `configuration` into `states_to_exit`.
pub fn compute_states_to_exit(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    transitions: &[TransitionId],
    configuration: &[StateId],
    states_to_exit: &mut SliceVec<'_, StateId>,
    mut scratch: Scratch<'_>,
) {
    states_to_exit.clear();
    // §scxml-D-computeExitSet takes the microstep's whole transition list and
    // unions the exit sets; §scxml-D-exitStates exits that union in exitOrder —
    // descendants before their ancestors and reverse document order among the
    // rest, which together are exactly reverse document order.
    for &transition in transitions {
        let scope = compute_exit_set(tables, machine, transition, scratch.reborrow());
        for &state in configuration {
            if scope.contains(tables, state) {
                states_to_exit.add_once(state);
            }
        }
    }
    sort_in_exit_order(states_to_exit);
}

/// Appendix D's exitStates.
pub fn exit_states(
    tables: &DocumentTables,
    machine: &mut dyn Behaviour,
    transitions: &[TransitionId],
    scratch: Scratch<'_>,
) {
    let capacity = usize::from(tables.max_configuration);
    let (mut configuration, rest) = scratch.take_states(capacity);
    for &state in machine.configuration() {
        configuration.push(state);
    }
    let (mut states_to_exit, mut rest) = rest.take_states(capacity);
    compute_states_to_exit(
        tables,
        &*machine,
        transitions,
        &configuration,
        &mut states_to_exit,
        rest.reborrow(),
    );

    // §scxml-D-exitStates: every history of every state exited is recorded
    // from the configuration as it stood before the first exit.
    let (mut recorded, _) = rest.take_states(capacity);
    for &state in states_to_exit.iter() {
        for (index, history) in tables.histories.iter().enumerate() {
            if history.parent == state {
                recorded_history(tables, state, history.deep, &configuration, &mut recorded);
                machine.record_history(HistoryId(index as u16), &recorded);
            }
        }
    }
    for &state in states_to_exit.iter() {
        // §scxml-D-exitStates: onexit, then cancelInvoke, then
        // configuration.delete(s), so `In(s)` inside s's own handler is true.
        machine.exit_state(state);
        machine.remove_from_configuration(state);
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
/// has not ended. No history is recorded: the run is over.
pub fn exit_interpreter(
    tables: &DocumentTables,
    machine: &mut dyn Behaviour,
    scratch: Scratch<'_>,
) {
    let (mut states_to_exit, _) = scratch.take_states(usize::from(tables.max_configuration));
    for &state in machine.configuration() {
        states_to_exit.push(state);
    }
    sort_in_exit_order(&mut states_to_exit);
    for &state in states_to_exit.iter() {
        machine.exit_state(state);
        machine.remove_from_configuration(state);
    }
}

/// Appendix D's exitOrder: descendants before their ancestors and reverse
/// document order among the rest, which together are exactly reverse document
/// order — descending ids.
fn sort_in_exit_order(states: &mut [StateId]) {
    states.sort_unstable_by(|a, b| b.cmp(a));
}

/// Appendix D's executeTransitionContent.
pub fn execute_transition_content(
    tables: &DocumentTables,
    machine: &mut dyn Behaviour,
    transitions: &[TransitionId],
) {
    // §scxml-D-executeTransitionContent: in the order the transitions were
    // selected, which is not their sources' document order once an ancestor's
    // transition is reached from a later region.
    for &transition in transitions {
        if tables.transition(transition).has_content {
            machine.execute_transition_content(transition);
        }
    }
}

/// Appendix D's enterStates, for the transitions a selection enabled or for
/// the document's initial transition alone, which is the whole of the
/// appendix's entry into the initial configuration (§scxml-D-interpret).
pub fn enter_states(
    tables: &DocumentTables,
    machine: &mut dyn Behaviour,
    transitions: &[TransitionId],
    mut scratch: Scratch<'_>,
) -> Entered {
    let entered = {
        let entry = compute_entry_set(tables, &*machine, transitions, scratch.reborrow());
        enter_entry_set(tables, machine, &entry);
        entry.states_to_enter.len()
    };
    Entered {
        states: entered,
        current: if entered == 0 {
            None
        } else {
            current_state(tables, &*machine, transitions, scratch)
        },
    }
}

/// The second half of Appendix D's enterStates: every state of an entry set
/// entered, in entry order.
fn enter_entry_set(tables: &DocumentTables, machine: &mut dyn Behaviour, entry: &EntrySet<'_>) {
    for &state in entry.states_to_enter.iter() {
        // §scxml-D-enterStates: configuration.add(s), then onentry, then the
        // initial transition's content if and only if this state's initial
        // state is being entered by default, then a history's default content
        // owed to it. A state already active is not entered twice.
        if machine.add_to_configuration(state) {
            machine.enter_state(state, entry.states_for_default_entry.contains(&state));
        }
        if let Some(history) = entry.default_history_content_of(state) {
            machine.execute_history_default_content(history);
        }
    }
    debug_assert!(
        machine.configuration().len() <= usize::from(tables.max_configuration),
        "the configuration holds more states than the document's tree allows: \
         sce-codegen bounded it too small"
    );
}

/// The atomic state the last targeted transition of `transitions` brought the
/// machine into: the first state it names, dereferenced, then down through the
/// configuration to an atomic state, taking the first active region of a
/// `<parallel>` in document order. The answer every backend gives as the
/// machine's current state.
fn current_state(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    transitions: &[TransitionId],
    scratch: Scratch<'_>,
) -> Option<StateId> {
    let (mut effective, mut rest) = scratch.take_states(usize::from(tables.max_configuration));
    let mut current = None;
    for &transition in transitions.iter().rev() {
        effective_target_states_on(
            tables,
            machine,
            tables.targets(transition),
            &mut effective,
            rest.reborrow(),
        );
        if let Some(&first) = effective.first() {
            current = Some(first);
            break;
        }
    }
    let mut current = current?;
    let configuration = machine.configuration();
    // The configuration is what the microstep entered, so a descent through it
    // cannot disagree with what happened. A compound state with no active child
    // is not a configuration this can repair, so the descent stops there.
    while !tables.is_atomic(current) {
        match tables
            .child_states(current)
            .iter()
            .find(|child| configuration.contains(child))
        {
            Some(&child) => current = child,
            None => break,
        }
    }
    Some(current)
}

// ════════════════════════════════════════════════════════════════════════
// The entry set
// ════════════════════════════════════════════════════════════════════════

/// What a microstep enters: the appendix's three out-parameters.
///
/// `states_to_enter` is sorted into entry order, so a machine enters it front to
/// back. `states_for_default_entry` answers whether a state's initial
/// transition content runs; `default_history_content` whether, and whose,
/// `<history>` default transition content runs after that state's own entry.
#[derive(Debug)]
pub struct EntrySet<'a> {
    /// The states to enter, in entry order.
    pub states_to_enter: SliceVec<'a, StateId>,
    /// The compound states whose initial state is entered by default.
    pub states_for_default_entry: SliceVec<'a, StateId>,
    /// Keyed by the history's parent, as the appendix keys it: the content
    /// runs when that parent is entered.
    pub default_history_content: SliceVec<'a, (StateId, HistoryId)>,
}

impl EntrySet<'_> {
    /// The `<history>` whose default transition content runs after `state` is
    /// entered, if a history of `state` was taken with nothing recorded.
    pub fn default_history_content_of(&self, state: StateId) -> Option<HistoryId> {
        self.default_history_content
            .iter()
            .find(|(parent, _)| *parent == state)
            .map(|&(_, history)| history)
    }
}

/// Appendix D's computeEntrySet.
///
/// `transitions` are the microstep's, in the order they were selected; a
/// transition without targets enters nothing. Returns the entry set,
/// `states_to_enter` sorted into entry order.
pub fn compute_entry_set<'a>(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    transitions: &[TransitionId],
    scratch: Scratch<'a>,
) -> EntrySet<'a> {
    let capacity = usize::from(tables.max_configuration);
    let (states_to_enter, rest) = scratch.take_states(capacity);
    let (states_for_default_entry, rest) = rest.take_states(capacity);
    let (mut effective, rest) = rest.take_states(capacity);
    let mut entry = EntrySet {
        states_to_enter,
        states_for_default_entry,
        default_history_content: SliceVec::new(rest.history_defaults),
    };
    // One stack for the whole entry set: the walks run one after another, so
    // one stack of the document's size serves them all.
    let mut stack = SliceVec::new(rest.walk);
    for &transition in transitions {
        add_transition_to_entry_set(
            tables,
            machine,
            transition,
            &mut stack,
            &mut effective,
            &mut entry,
        );
    }
    // §scxml-D-enterStates: states are entered in entryOrder — ancestors before
    // descendants, document order between the rest, which is exactly document
    // order, because that is a pre-order walk: ascending ids.
    entry.states_to_enter.sort_unstable();
    entry
}

/// One transition's part of Appendix D's computeEntrySet: what it adds to the
/// entry set, before the set is put into entry order.
fn add_transition_to_entry_set(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    transition: TransitionId,
    stack: &mut SliceVec<'_, WalkStep>,
    effective: &mut SliceVec<'_, StateId>,
    entry: &mut EntrySet<'_>,
) {
    let row = tables.transition(transition);
    let targets = tables.targets(transition);
    // §scxml-D-computeEntrySet: first every target with its default
    // descendants, then the ancestors that are entered inside the domain —
    // ancestors outside it were never exited. The order matters for a
    // target SET: the regions of a `<parallel>` that another target already
    // descends into must be seen as taken before the ancestor walk fills
    // the rest with defaults.
    for &target in targets {
        walk_entry(tables, machine, WalkStep::Descendants(target), stack, entry);
    }
    let walk = Scratch {
        states: &mut [],
        walk: stack.slots_mut(),
        history_defaults: &mut [],
    };
    effective_target_states_on(tables, machine, targets, effective, walk);
    if effective.is_empty() {
        return;
    }
    let domain = transition_domain(tables, row.source, effective, row.internal);
    for &state in effective.iter() {
        walk_entry(
            tables,
            machine,
            WalkStep::Ancestors(Target::State(state), domain),
            stack,
            entry,
        );
    }
}

impl<T> SliceVec<'_, T> {
    /// Every slot, for a walk that empties the buffer before it returns and
    /// lends its slots to another walk in between.
    fn slots_mut(&mut self) -> &mut [T] {
        debug_assert!(
            self.len == 0,
            "a walk lends its stack only while it is empty"
        );
        self.slots
    }
}

/// Runs the entry-set procedures from `first` to the end over `stack`, which is
/// empty on entry and on return.
///
/// Appendix D writes addDescendantStatesToEnter, addAncestorStatesToEnter and the
/// region defaults they share as one recursion. Each step here is what one call
/// of it does before and between the calls it makes, and each call it would make
/// is a step put on the stack above the step that resumes after it, so the steps
/// run in exactly the order the calls would have: that order is not incidental,
/// since the regions of a `<parallel>` are judged against what the entry set
/// holds when each is reached.
fn walk_entry(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    first: WalkStep,
    stack: &mut SliceVec<'_, WalkStep>,
    entry: &mut EntrySet<'_>,
) {
    stack.push(first);
    while let Some(step) = stack.pop() {
        match step {
            WalkStep::Descendants(Target::State(state)) => {
                entry.states_to_enter.add_once(state);
                match tables.state(state).kind {
                    StateKind::Compound => {
                        // §scxml-D-addDescendantStatesToEnter: a compound state
                        // entered as a target is entered by DEFAULT — the one
                        // condition under which its initial transition's content
                        // runs. Its initial transition names the child or children
                        // it enters (§scxml-3.3), possibly several and possibly deep
                        // (§scxml-3.6).
                        entry.states_for_default_entry.add_once(state);
                        stack.push(WalkStep::Listed {
                            source: Listed::Initial(state),
                            index: 0,
                            ancestors: false,
                        });
                    }
                    StateKind::Parallel => stack.push(WalkStep::Regions {
                        parallel: state,
                        index: 0,
                    }),
                    StateKind::Atomic | StateKind::Final => {}
                }
            }
            WalkStep::Descendants(Target::History(history)) => {
                // §scxml-3.10: a transition to a history behaves as a transition
                // to the configuration it stored, or — before its parent was ever
                // visited — to its default stored configuration.
                let source = match machine.history_value(history) {
                    // §scxml-D-addDescendantStatesToEnter: a history that has
                    // recorded a configuration enters it, and the ancestors
                    // between it and the history's parent — a deep history
                    // records atomic states, which need not be children.
                    Some(recorded) if !recorded.is_empty() => Listed::Recorded(history),
                    _ => {
                        // §scxml-D-addDescendantStatesToEnter: nothing recorded
                        // yet, so the default transition is taken, and its
                        // content is owed once the parent has been entered —
                        // keyed by the parent, a later history of the same
                        // parent replacing it.
                        set_default_history_content(
                            &mut entry.default_history_content,
                            tables.history(history).parent,
                            history,
                        );
                        Listed::Defaults(history)
                    }
                };
                stack.push(WalkStep::Listed {
                    source,
                    index: 0,
                    ancestors: false,
                });
            }
            WalkStep::Listed {
                source,
                index,
                ancestors,
            } => {
                if let Some(target) = listed_target(tables, machine, source, index) {
                    // The rest of the list resumes once this target is done.
                    stack.push(WalkStep::Listed {
                        source,
                        index: index + 1,
                        ancestors,
                    });
                    stack.push(if ancestors {
                        WalkStep::Ancestors(target, Some(listed_stop(tables, source)))
                    } else {
                        WalkStep::Descendants(target)
                    });
                } else if !ancestors {
                    // Every descendant is in; now the ancestors of each.
                    stack.push(WalkStep::Listed {
                        source,
                        index: 0,
                        ancestors: true,
                    });
                }
            }
            WalkStep::Ancestors(target, stop) => {
                if let Some(first) = first_proper_ancestor(tables, target, stop) {
                    stack.push(WalkStep::AncestorsFrom { from: first, stop });
                }
            }
            WalkStep::AncestorsFrom { from, stop } => {
                // §scxml-D-addAncestorStatesToEnter: an ancestor is entered
                // WITHOUT its default initial state — the set already holds the
                // descendant it leads to. A `<parallel>` still gives its other
                // regions their defaults, because all of them are entered.
                entry.states_to_enter.add_once(from);
                if let Some(next) = tables.parent(from).filter(|&next| Some(next) != stop) {
                    stack.push(WalkStep::AncestorsFrom { from: next, stop });
                }
                if tables.is_parallel(from) {
                    // Above the step that resumes the walk up, so the regions
                    // are done before the next ancestor is entered.
                    stack.push(WalkStep::Regions {
                        parallel: from,
                        index: 0,
                    });
                }
            }
            WalkStep::Regions { parallel, index } => {
                // §scxml-3.4: every child of an active `<parallel>` is active, so
                // a region nothing on the set descends into is entered by
                // default.
                let children = tables.child_states(parallel);
                let mut next = usize::from(index);
                while let Some(&child) = children.get(next) {
                    next += 1;
                    let taken = entry
                        .states_to_enter
                        .iter()
                        .any(|&state| tables.is_descendant(state, child));
                    if !taken {
                        if next < children.len() {
                            stack.push(WalkStep::Regions {
                                parallel,
                                index: next as u16,
                            });
                        }
                        stack.push(WalkStep::Descendants(Target::State(child)));
                        break;
                    }
                }
            }
            // The walk puts down steps of its own kind only; see
            // `effective_target_states_on` for why there is no message.
            _ => unreachable!(),
        }
    }
}

/// The `index`th target of `source`'s list, or `None` once the list has ended.
fn listed_target(
    tables: &DocumentTables,
    machine: &dyn Behaviour,
    source: Listed,
    index: u16,
) -> Option<Target> {
    let index = usize::from(index);
    match source {
        Listed::Initial(state) => {
            let initial = tables.state(state).initial?;
            tables.targets(initial).get(index).copied()
        }
        Listed::Recorded(history) => machine
            .history_value(history)
            .and_then(|recorded| recorded.get(index))
            .map(|&state| Target::State(state)),
        Listed::Defaults(history) => tables
            .targets(tables.history(history).default)
            .get(index)
            .copied(),
    }
}

/// The state the ancestors of `source`'s targets are entered up to, not
/// including it: the compound state itself for its initial targets, the
/// `<history>`'s parent for a history's.
fn listed_stop(tables: &DocumentTables, source: Listed) -> StateId {
    match source {
        Listed::Initial(state) => state,
        Listed::Recorded(history) | Listed::Defaults(history) => tables.history(history).parent,
    }
}

/// Appendix D's getProperAncestors, as far as the walk up needs it: the first of
/// the ancestors of `target` in ancestry order up to, not including, `stop` —
/// which is none at all when `stop` is not above `target`.
///
/// A `<history>` sits in its parent, so the parent is its first proper
/// ancestor. `stop` `None` is the `<scxml>` element, which is above every
/// state and is never entered, so it is not in the list. The rest of the list
/// is each one's parent in turn, which [`WalkStep::AncestorsFrom`] reads off the
/// tables as it goes.
fn first_proper_ancestor(
    tables: &DocumentTables,
    target: Target,
    stop: Option<StateId>,
) -> Option<StateId> {
    let first = match target {
        Target::State(state) => tables.parent(state),
        Target::History(history) => Some(tables.history(history).parent),
    }?;
    match stop {
        None => Some(first),
        // §scxml-D-getProperAncestors: when `stop` is the first ancestor
        // itself, nothing lies between; when it is not above `first` at all —
        // the state's parent, the state itself, or one of its descendants — the
        // answer is the empty set.
        Some(stop) if tables.is_descendant(first, stop) => Some(first),
        Some(_) => None,
    }
}

fn set_default_history_content(
    content: &mut SliceVec<'_, (StateId, HistoryId)>,
    parent: StateId,
    history: HistoryId,
) {
    for slot in content.iter_mut() {
        if slot.0 == parent {
            slot.1 = history;
            return;
        }
    }
    content.push((parent, history));
}

// ════════════════════════════════════════════════════════════════════════
// History
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's exitStates, its history half: what a `<history>` of `parent`
/// records as `parent` is exited — for a deep history the active atomic states
/// below `parent`, for a shallow one its active children — into `recorded`.
///
/// Read off `configuration_before_exit`, the configuration as it stood before
/// the microstep's first exit: the appendix records every history before it
/// exits any state, so every history of one microstep reads the same
/// configuration. In that configuration's order.
pub fn recorded_history(
    tables: &DocumentTables,
    parent: StateId,
    deep: bool,
    configuration_before_exit: &[StateId],
    recorded: &mut SliceVec<'_, StateId>,
) {
    recorded.clear();
    for &state in configuration_before_exit {
        let records = if deep {
            // §scxml-D-exitStates: `isAtomicState(s0) and isDescendant(s0, s)`.
            tables.is_atomic(state) && tables.is_descendant(state, parent)
        } else {
            // §scxml-D-exitStates: `s0.parent == s`.
            tables.parent(state) == Some(parent)
        };
        if records {
            recorded.push(state);
        }
    }
}

// ════════════════════════════════════════════════════════════════════════
// Predicates
// ════════════════════════════════════════════════════════════════════════

/// Appendix D's isInFinalState: a compound state is in a final state when one
/// of its `<final>` children is active; a `<parallel>` when EVERY one of its
/// child states is — asked of each region in turn, so a region that is itself a
/// `<parallel>` counts only once all of ITS regions do. Nothing else ever is.
///
/// The question is asked of the states a chain of `<parallel>`s leads to from
/// `state`, and a subtree is a range of ids, so this is one pass over the range
/// that skips the subtree of every state it has answered for: no stack.
pub fn is_in_final_state(
    tables: &DocumentTables,
    state: StateId,
    configuration: &[StateId],
) -> bool {
    match tables.state(state).kind {
        StateKind::Compound => return in_final_child(tables, state, configuration),
        StateKind::Parallel => {}
        StateKind::Atomic | StateKind::Final => return false,
    }
    let last = tables.state(state).last_descendant;
    let mut next = state.0 + 1;
    while next <= last.0 {
        let region = StateId(next);
        match tables.state(region).kind {
            // A nested `<parallel>`: its own regions are the next ids.
            StateKind::Parallel => next += 1,
            StateKind::Compound => {
                if !in_final_child(tables, region, configuration) {
                    return false;
                }
                next = tables.state(region).last_descendant.0 + 1;
            }
            StateKind::Atomic | StateKind::Final => return false,
        }
    }
    true
}

/// Whether one of the `<final>` children of the compound state `state` is active.
fn in_final_child(tables: &DocumentTables, state: StateId, configuration: &[StateId]) -> bool {
    tables
        .child_states(state)
        .iter()
        .any(|&child| tables.is_final(child) && configuration.contains(&child))
}
