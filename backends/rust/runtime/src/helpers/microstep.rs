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
//! events as numbers, its configuration and history records as a run store of
//! numbers ([`RunStoreRef`], [`RunStoreMut`]), and the running machine as one
//! object-safe trait, [`Behaviour`]: which of a state's transitions an event
//! enables, and what exiting a state, running a transition's content and
//! entering a state DO. So a program that holds two machines holds one copy of
//! this module. A procedure that only reads takes the store alone.
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

use core::mem::MaybeUninit;

use crate::helpers::run_store::{RunStoreMut, RunStoreRef};
use crate::helpers::state_set::{state_words, StateSet, StateSetRef, Word};
use crate::helpers::tables::{
    DocumentTables, EventId, HistoryId, Named, StateId, StateKind, Target, TransitionId,
};

// ════════════════════════════════════════════════════════════════════════
// Buffers
// ════════════════════════════════════════════════════════════════════════

/// The sets of states a microstep holds at once on its longest path: the entry
/// set holds the states to enter and the states entered by default while it
/// computes one transition's effective targets. exitStates holds two (the
/// states it exits, and one transition's effective targets), selection one.
const STATE_SETS: usize = 3;

/// The words the scratch of a document with `states` states must have room
/// for: three sets of [`state_words`] each. The generator writes the
/// workspace's size as a call of this function, so the rule is stated once.
pub const fn state_scratch(states: usize) -> usize {
    STATE_SETS * state_words(states)
}

/// A buffer of a fixed capacity over borrowed slots: what a procedure builds a
/// set of states, of transitions or of steps in.
///
/// The slots are uninitialised until the buffer writes them, and only the
/// first `len` are ever read: the held items are the front of the slots, the
/// rest is room. So a buffer lent empty asks nothing of what its slots held,
/// and a [`Workspace`] is not filled before a turn lends it (docs/adr/0071).
///
/// Every `unsafe` of this module is in this type's methods, and each rests on
/// the one invariant the fields keep: `len <= slots.len()`, and `slots[..len]`
/// are initialised.
/// The items are `Copy`, so nothing a buffer forgets is owed a drop.
pub struct SliceVec<'a, T> {
    slots: &'a mut [MaybeUninit<T>],
    len: usize,
}

impl<'a, T: Copy + PartialEq> SliceVec<'a, T> {
    /// An empty buffer over `slots`, whatever they hold.
    pub fn new(slots: &'a mut [MaybeUninit<T>]) -> Self {
        Self { slots, len: 0 }
    }

    /// Add `item` at the end.
    #[inline]
    pub fn push(&mut self, item: T) {
        let Some(slot) = self.slots.get_mut(self.len) else {
            workspace_too_small()
        };
        *slot = MaybeUninit::new(item);
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
        // SAFETY: `self.len` was below the old length, which is at most
        // `slots.len()`, and every slot below the old length is initialised.
        // The item is `Copy`, so reading it leaves the slot as it was.
        let item = unsafe { self.slots[self.len].assume_init_read() };
        Some(item)
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

    /// The items held, front first.
    #[inline]
    fn deref(&self) -> &[T] {
        // SAFETY: `MaybeUninit<T>` has the size, alignment and layout of `T`,
        // `len <= slots.len()`, and the first `len` slots are initialised, so
        // they are `len` valid items in one allocation, borrowed for as long
        // as the buffer is.
        unsafe { core::slice::from_raw_parts(self.slots.as_ptr().cast::<T>(), self.len) }
    }
}

impl<T> core::ops::DerefMut for SliceVec<'_, T> {
    /// The items held, front first, to overwrite in place.
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: as for `deref`; the borrow is unique because `self` is, and
        // what is written through it is a `T`, so the prefix stays initialised.
        unsafe { core::slice::from_raw_parts_mut(self.slots.as_mut_ptr().cast::<T>(), self.len) }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SliceVec<'_, T> {
    /// The items held: the slots past them hold nothing to show.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
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
///
/// The walk and the history defaults are slots a [`SliceVec`] writes before it
/// reads, so they are lent uninitialised. The words of the sets are
/// initialised, as a [`StateSet`] reads them as words; a procedure clears a
/// set when it takes one.
#[derive(Debug)]
pub struct Scratch<'a> {
    /// The words of the sets of states.
    pub sets: &'a mut [Word],
    /// The slots of the walks' steps.
    pub walk: &'a mut [MaybeUninit<WalkStep>],
    /// The slots of the history default content an entry set owes.
    pub history_defaults: &'a mut [MaybeUninit<(StateId, HistoryId)>],
}

impl<'a> Scratch<'a> {
    /// The same buffers, for a call that returns them.
    #[inline]
    pub fn reborrow(&mut self) -> Scratch<'_> {
        Scratch {
            sets: &mut *self.sets,
            walk: &mut *self.walk,
            history_defaults: &mut *self.history_defaults,
        }
    }

    /// An empty set of the states of `tables` from the front, and the rest.
    #[inline]
    fn take_set(self, tables: &DocumentTables) -> (StateSet<'a>, Scratch<'a>) {
        let words = state_words(tables.states.len());
        if words > self.sets.len() {
            workspace_too_small()
        }
        let (taken, sets) = self.sets.split_at_mut(words);
        let mut set = StateSet::new(taken);
        set.clear();
        let rest = Scratch {
            sets,
            walk: self.walk,
            history_defaults: self.history_defaults,
        };
        (set, rest)
    }
}

/// A machine's buffers for one call into this module, sized by its document:
/// `SETS` is [`state_scratch`] of its number of states, `ENABLED` the most
/// transitions a selection enables, `WALK` the most steps the walks owe and
/// `DEFAULTS` the most history default transitions a microstep owes. The
/// generated policy spells its own as `StatePolicy::Workspace`.
///
/// A local of the call that runs a macrostep, never a field of the engine, so
/// it costs stack only while a macrostep runs (docs/adr/0056 §4).
///
/// The enabled set, the walk and the history defaults are slots left
/// uninitialised until a [`SliceVec`] writes them, which is before it reads
/// them. Filled with a placeholder, as they once were, every turn paid a loop
/// that wrote each slot; the placeholder step was not zero bytes, so the
/// compiler built the fill in a temporary and copied it, and the turn's frame
/// held the buffers twice (docs/adr/0071). Only the words of the sets are
/// written, as zeros.
#[derive(Debug, Clone)]
pub struct Workspace<
    const SETS: usize,
    const ENABLED: usize,
    const WALK: usize,
    const DEFAULTS: usize,
> {
    sets: [Word; SETS],
    enabled: [MaybeUninit<TransitionId>; ENABLED],
    walk: [MaybeUninit<WalkStep>; WALK],
    history_defaults: [MaybeUninit<(StateId, HistoryId)>; DEFAULTS],
}

impl<const SETS: usize, const ENABLED: usize, const WALK: usize, const DEFAULTS: usize>
    Workspace<SETS, ENABLED, WALK, DEFAULTS>
{
    /// Empty buffers: the words of the sets zero, every other slot
    /// uninitialised.
    pub const fn new() -> Self {
        Self {
            sets: [0; SETS],
            enabled: [MaybeUninit::uninit(); ENABLED],
            walk: [MaybeUninit::uninit(); WALK],
            history_defaults: [MaybeUninit::uninit(); DEFAULTS],
        }
    }
}

impl<const SETS: usize, const ENABLED: usize, const WALK: usize, const DEFAULTS: usize> Default
    for Workspace<SETS, ENABLED, WALK, DEFAULTS>
{
    fn default() -> Self {
        Self::new()
    }
}

/// A [`Workspace`] of any size, as the engine lends it.
pub trait Buffers {
    /// Empty buffers, as a constant: a turn that starts from it writes the
    /// zeroed words of the sets and nothing else, and builds no value to copy
    /// into its frame (docs/adr/0071).
    const EMPTY: Self;

    /// The buffer of the enabled set, and the scratch for everything else.
    fn split(&mut self) -> (SliceVec<'_, TransitionId>, Scratch<'_>);
}

impl<const SETS: usize, const ENABLED: usize, const WALK: usize, const DEFAULTS: usize> Buffers
    for Workspace<SETS, ENABLED, WALK, DEFAULTS>
{
    const EMPTY: Self = Self::new();

    #[inline]
    fn split(&mut self) -> (SliceVec<'_, TransitionId>, Scratch<'_>) {
        (
            SliceVec::new(&mut self.enabled),
            Scratch {
                sets: &mut self.sets,
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
    /// The machine's configuration and history records, which the procedures
    /// read and write themselves: they are numbers, the same for every machine,
    /// so nothing about keeping them is the machine's own.
    fn store(&mut self) -> RunStoreMut<'_>;

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
    /// The regions of a `<parallel>` from child `next` on, each entered by
    /// default unless the entry set already goes into it.
    Regions {
        /// The `<parallel>`.
        parallel: StateId,
        /// The next child to look at.
        next: StateId,
    },
    /// A list of targets, in the pass `ancestors` names, from `cursor` on: the
    /// position of the next target in a list of targets, or for what a
    /// `<history>` recorded, the state to look on from.
    Listed {
        /// Whose list it is.
        source: Listed,
        /// Where the rest of the list starts.
        cursor: u16,
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
// The features a document may use
// ════════════════════════════════════════════════════════════════════════

/// The procedures of Appendix D that only a `<history>` reaches, carried by the
/// tables of a document that has one ([`DocumentTables::history`]) and reached
/// through them alone (docs/adr/0066).
///
/// A program none of whose documents has a `<history>` therefore references
/// neither [`HISTORY`] nor any function only it names, and the link drops them:
/// what `NoHistory` once removed from a machine that could not take a history,
/// removed from the program. The fields are private, so [`HISTORY`] is the only
/// value there is.
pub struct HistoryProcedures {
    /// getEffectiveTargetStates for a `<history>` target.
    effective: fn(
        &DocumentTables,
        RunStoreRef<'_>,
        HistoryId,
        &mut StateSet<'_>,
        &mut [MaybeUninit<WalkStep>],
    ) -> Option<StateId>,
    /// addDescendantStatesToEnter for a `<history>` target: the list it
    /// enters, with the default content it owes put in the entry set.
    descend: fn(&DocumentTables, RunStoreRef<'_>, HistoryId, &mut EntrySet<'_>) -> Listed,
    /// The first state a `<history>` recorded from a state on.
    recorded_from: fn(RunStoreRef<'_>, HistoryId, StateId) -> Option<StateId>,
    /// exitStates' records of the histories of the states it exits.
    record: fn(&DocumentTables, RunStoreMut<'_>, StateSetRef<'_>),
}

/// The procedures of Appendix D that only a `<parallel>` reaches, carried by the
/// tables of a document that has one ([`DocumentTables::parallel`]); see
/// [`HistoryProcedures`] for why.
pub struct ParallelProcedures {
    /// The entry walk's defaults for the regions of a `<parallel>` nothing on
    /// the entry set goes into.
    regions: fn(&DocumentTables, StateId, StateId, &mut SliceVec<'_, WalkStep>, StateSetRef<'_>),
    /// removeConflictingTransitions.
    remove_conflicting:
        fn(&DocumentTables, RunStoreRef<'_>, &mut SliceVec<'_, TransitionId>, Scratch<'_>),
    /// isInFinalState of a `<parallel>`.
    in_final: fn(&DocumentTables, StateId, StateSetRef<'_>) -> bool,
}

/// The `<history>` procedures, the value a generated table writes as
/// `history: Some(&HISTORY)`.
///
/// A constant rather than a `static`: a document's tables are a constant, and a
/// constant cannot refer to a `static` at the runtime's `rust-version`. The
/// reference a table takes is promoted to one constant allocation, and a program
/// that takes none emits none.
pub const HISTORY: HistoryProcedures = HistoryProcedures {
    effective: effective_history_targets,
    descend: descend_into_history,
    recorded_from: history_recorded_from,
    record: record_histories,
};

/// The `<parallel>` procedures, the value a generated table writes as
/// `parallel: Some(&PARALLEL)`; see [`HISTORY`].
pub const PARALLEL: ParallelProcedures = ParallelProcedures {
    regions: default_regions,
    remove_conflicting: remove_conflicting_transitions,
    in_final: parallel_in_final_state,
};

impl core::fmt::Debug for HistoryProcedures {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("HISTORY")
    }
}

impl core::fmt::Debug for ParallelProcedures {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("PARALLEL")
    }
}

/// The `<history>` procedures of `tables`, which reach a `<history>`.
#[inline]
fn history_procedures(tables: &DocumentTables) -> &'static HistoryProcedures {
    match tables.history {
        Some(procedures) => procedures,
        None => procedures_missing(),
    }
}

/// The `<parallel>` procedures of `tables`, which reach a `<parallel>`.
#[inline]
fn parallel_procedures(tables: &DocumentTables) -> &'static ParallelProcedures {
    match tables.parallel {
        Some(procedures) => procedures,
        None => procedures_missing(),
    }
}

/// Tables that name a `<history>` or a `<parallel>` without carrying its
/// procedures, which [`DocumentTables::check`] refuses: a generator fault. No
/// formatted message, for the reason [`workspace_too_small`] has none.
#[cold]
#[inline(never)]
fn procedures_missing() -> ! {
    panic!("a document's tables use a feature without its procedures: sce-codegen wrote them wrong")
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
    // The atomic states of the configuration in document order, which is the
    // order of the ids: each is looked up again after the guards of the one
    // before have run, and a guard does not change the configuration.
    let mut next = Some(StateId(0));
    while let Some(atomic) = next.and_then(|from| {
        machine
            .store()
            .as_ref()
            .configuration()
            .iter_from(from)
            .find(|&state| tables.is_atomic(state))
    }) {
        next = atomic.0.checked_add(1).map(StateId);
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
    // Without a `<parallel>` a configuration has one atomic state, so the walk
    // above enables one transition at most and there is nothing to conflict.
    if let Some(parallel) = tables.parallel {
        (parallel.remove_conflicting)(tables, machine.store().as_ref(), enabled, scratch);
    }
}

/// Appendix D's removeConflictingTransitions, over `enabled` itself: what
/// remains in it is the filtered set, in selection order. A
/// [`ParallelProcedures`] procedure ([`PARALLEL`]).
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
    store: RunStoreRef<'_>,
    enabled: &mut SliceVec<'_, TransitionId>,
    mut scratch: Scratch<'_>,
) {
    let configuration = store.configuration();
    let mut kept = 0;
    for candidate in 0..enabled.len() {
        let t1 = enabled[candidate];
        // A kept transition's exit set is asked for again for each later one
        // held against it rather than stored beside it: on an MCU the stack a
        // stored set per transition costs is scarcer than the cycles, and the
        // answer is read off the one configuration the selection saw.
        let t1_exits = compute_exit_set(tables, store, t1, scratch.reborrow());
        let mut conflicts = |t2: TransitionId| {
            let t2_exits = compute_exit_set(tables, store, t2, scratch.reborrow());
            t1_exits.meets(tables, t2_exits, configuration)
        };
        let t1_source = tables.transition(t1).source();
        // §scxml-D-removeConflictingTransitions: t1 is preempted by any kept
        // transition it conflicts with whose source it does not descend from.
        // A transition that exits nothing — a targetless one — conflicts with
        // nothing and can never be preempted.
        let preempted = enabled[..kept].iter().any(|&t2| {
            conflicts(t2) && !descends(tables, t1_source, tables.transition(t2).source())
        });
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
    /// exits, or none: whether the configuration holds a state of one range.
    pub fn meets(
        self,
        tables: &DocumentTables,
        other: ExitScope,
        configuration: StateSetRef<'_>,
    ) -> bool {
        let domain = match (self, other) {
            (ExitScope::Nothing, _) | (_, ExitScope::Nothing) => return false,
            (ExitScope::All, ExitScope::All) => return !configuration.is_empty(),
            (ExitScope::All, ExitScope::Below(domain))
            | (ExitScope::Below(domain), ExitScope::All) => domain,
            (ExitScope::Below(a), ExitScope::Below(b)) => {
                if a == b || tables.is_descendant(b, a) {
                    b
                } else if tables.is_descendant(a, b) {
                    a
                } else {
                    return false;
                }
            }
        };
        configuration.meets(StateId(domain.0 + 1), tables.last_descendant(domain))
    }
}

/// Appendix D's computeExitSet for one transition, as the scope a
/// configuration is read through: the active proper descendants of its domain.
pub fn compute_exit_set(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
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
    let (mut effective, rest) = scratch.take_set(tables);
    effective_target_states_on(tables, store, targets, &mut effective, rest.walk);
    match transition_domain(tables, row.source(), effective.as_ref(), row.internal()) {
        Some(domain) => ExitScope::Below(domain),
        None => ExitScope::All,
    }
}

/// Appendix D's getEffectiveTargetStates, over a target list, into `effective`,
/// on the walk stack `walk` the caller lends.
///
/// The states the list stands for, histories dereferenced — to what each
/// recorded or, before its parent was ever exited, to its default. The domain,
/// and so the exit set, is a question about these. Returns the first of them
/// the list names, which is where the machine's current state is read from.
pub fn effective_target_states_on(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
    targets: &[Target],
    effective: &mut StateSet<'_>,
    walk: &mut [MaybeUninit<WalkStep>],
) -> Option<StateId> {
    effective.clear();
    let mut first = None;
    for &target in targets {
        let added = match target.named() {
            Named::State(state) => {
                effective.insert(state);
                Some(state)
            }
            Named::History(history) => {
                (history_procedures(tables).effective)(tables, store, history, effective, walk)
            }
        };
        if first.is_none() {
            first = added;
        }
    }
    first
}

/// The effective targets of the `<history>` target `history`, added to
/// `effective` on the walk stack `walk`; the first of them it adds.
///
/// A [`HistoryProcedures`] procedure ([`HISTORY`]).
fn effective_history_targets(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
    history: HistoryId,
    effective: &mut StateSet<'_>,
    walk: &mut [MaybeUninit<WalkStep>],
) -> Option<StateId> {
    let mut first = None;
    let mut add = |state: StateId| {
        effective.insert(state);
        first.get_or_insert(state);
    };
    let mut stack = SliceVec::new(walk);
    stack.push(WalkStep::EffectiveTarget(Target::history(history)));
    while let Some(step) = stack.pop() {
        match step {
            WalkStep::EffectiveTarget(target) => match target.named() {
                Named::State(state) => add(state),
                // §scxml-D-getEffectiveTargetStates: a history stands for the
                // configuration it recorded, and before its parent was ever
                // exited for the targets of its default transition — which may
                // name histories themselves, so those are walked in turn.
                Named::History(history) => match store.history_value(history) {
                    Some(recorded) if !recorded.is_empty() => recorded.iter().for_each(&mut add),
                    _ => stack.push(WalkStep::EffectiveDefaults { history, index: 0 }),
                },
            },
            WalkStep::EffectiveDefaults { history, index } => {
                let defaults = tables.targets(tables.history(history).default());
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
    first
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
    targets: StateSetRef<'_>,
    is_internal: bool,
) -> Option<StateId> {
    let source = source?;
    // A set of states lies below a state when its first and its last do: a
    // subtree is a range of ids.
    let (Some(first), Some(last)) = (targets.first_from(StateId(0)), targets.last()) else {
        debug_assert!(false, "a targetless transition has no domain to ask for");
        return None;
    };

    // §scxml-D-getTransitionDomain: an internal transition whose targets all
    // lie below a compound source has the SOURCE as its domain, so the source
    // stays active and only its active descendants are exited. That is not the
    // same as exiting nothing: a transition rooted at one of those descendants
    // exits it too, and the appendix expects the two to be found in conflict.
    if is_internal
        && tables.is_compound(source)
        && tables.is_descendant(first, source)
        && tables.is_descendant(last, source)
    {
        return Some(source);
    }
    find_lcca(tables, source, first, last)
}

/// Appendix D's findLCCA over `[head] + tail`, `tail` the states from `first`
/// to `last` in document order: the first proper ancestor of `head` that is a
/// compound state — or the `<scxml>` element, `None` — and contains every state
/// of `tail`, which is to contain its first and its last.
///
/// Asked of the source and EVERY target at once: combining pairwise answers
/// can only widen the domain. A parent's id is below its child's, so the walk
/// up ends.
fn find_lcca(
    tables: &DocumentTables,
    head: StateId,
    first: StateId,
    last: StateId,
) -> Option<StateId> {
    let mut current = head;
    loop {
        let ancestor = tables.parent(current)?;
        // §scxml-D-findLCCA filters the ancestors with
        // isCompoundStateOrScxmlElement: a `<parallel>` is never a domain.
        if tables.is_compound(ancestor)
            && tables.is_descendant(first, ancestor)
            && tables.is_descendant(last, ancestor)
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

/// Appendix D's computeExitSet over the microstep's transitions — the states
/// exitStates exits — read off the configuration into `states_to_exit`.
pub fn compute_states_to_exit(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
    transitions: &[TransitionId],
    states_to_exit: &mut StateSet<'_>,
    mut scratch: Scratch<'_>,
) {
    states_to_exit.clear();
    let configuration = store.configuration();
    // §scxml-D-computeExitSet takes the microstep's whole transition list and
    // unions the exit sets: each is the active states of one subtree, a range
    // of ids, or the whole configuration.
    for &transition in transitions {
        match compute_exit_set(tables, store, transition, scratch.reborrow()) {
            ExitScope::Nothing => {}
            ExitScope::All => states_to_exit.copy_from(configuration),
            ExitScope::Below(domain) => {
                let last = tables.last_descendant(domain);
                for state in configuration
                    .iter_from(StateId(domain.0 + 1))
                    .take_while(|&state| state <= last)
                {
                    states_to_exit.insert(state);
                }
            }
        }
    }
}

/// Appendix D's exitStates.
pub fn exit_states(
    tables: &DocumentTables,
    machine: &mut dyn Behaviour,
    transitions: &[TransitionId],
    scratch: Scratch<'_>,
) {
    let (mut states_to_exit, rest) = scratch.take_set(tables);
    compute_states_to_exit(
        tables,
        machine.store().as_ref(),
        transitions,
        &mut states_to_exit,
        rest,
    );

    if let Some(history) = tables.history {
        (history.record)(tables, machine.store(), states_to_exit.as_ref());
    }
    // §scxml-D-exitStates exits the union in exitOrder — descendants before
    // their ancestors and reverse document order among the rest, which
    // together are exactly reverse document order.
    for state in states_to_exit.as_ref().iter_rev() {
        // §scxml-D-exitStates: onexit, then cancelInvoke, then
        // configuration.delete(s), so `In(s)` inside s's own handler is true.
        machine.exit_state(state);
        machine.store().remove(state);
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
///
/// The configuration's last state in document order is the first in
/// exitOrder, and it leaves the configuration before the next is asked for,
/// so the set is read as it shrinks and never copied.
pub fn exit_interpreter(machine: &mut dyn Behaviour) {
    while let Some(state) = machine.store().as_ref().configuration().last() {
        machine.exit_state(state);
        machine.store().remove(state);
    }
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
        if tables.transition(transition).has_content() {
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
    scratch: Scratch<'_>,
) -> Entered {
    let entry = compute_entry_set(tables, machine.store().as_ref(), transitions, scratch);
    enter_entry_set(machine, &entry);
    let entered = entry.states_to_enter.as_ref().len();
    Entered {
        states: entered,
        current: if entered == 0 {
            None
        } else {
            entry
                .last_target
                .map(|target| current_state(tables, machine.store().as_ref(), target))
        },
    }
}

/// The second half of Appendix D's enterStates: every state of an entry set
/// entered, in entry order, which is document order.
fn enter_entry_set(machine: &mut dyn Behaviour, entry: &EntrySet<'_>) {
    for state in entry.states_to_enter.as_ref().iter() {
        // §scxml-D-enterStates: configuration.add(s), then onentry, then the
        // initial transition's content if and only if this state's initial
        // state is being entered by default, then a history's default content
        // owed to it. A state already active is not entered twice.
        if machine.store().add(state) {
            machine.enter_state(state, entry.states_for_default_entry.contains(state));
        }
        if let Some(history) = entry.default_history_content_of(state) {
            machine.execute_history_default_content(history);
        }
    }
}

/// The atomic state the last targeted transition brought the machine into:
/// `target`, the first state that transition names, dereferenced
/// ([`EntrySet::last_target`]), then down through the configuration to an
/// atomic state, taking the first active region of a `<parallel>` in document
/// order. The answer every backend gives as the machine's current state.
fn current_state(tables: &DocumentTables, store: RunStoreRef<'_>, target: StateId) -> StateId {
    let mut current = target;
    let configuration = store.configuration();
    // The configuration is what the microstep entered, so a descent through it
    // cannot disagree with what happened. A compound state with no active child
    // is not a configuration this can repair, so the descent stops there.
    while !tables.is_atomic(current) {
        match tables
            .children(current)
            .find(|&child| configuration.contains(child))
        {
            Some(child) => current = child,
            None => break,
        }
    }
    current
}

// ════════════════════════════════════════════════════════════════════════
// The entry set
// ════════════════════════════════════════════════════════════════════════

/// What a microstep enters: the appendix's three out-parameters.
///
/// `states_to_enter` read in document order is in entry order, so a machine
/// enters it front to back. `states_for_default_entry` answers whether a
/// state's initial transition content runs; `default_history_content` whether,
/// and whose, `<history>` default transition content runs after that state's
/// own entry.
#[derive(Debug)]
pub struct EntrySet<'a> {
    /// The states to enter.
    pub states_to_enter: StateSet<'a>,
    /// The compound states whose initial state is entered by default.
    pub states_for_default_entry: StateSet<'a>,
    /// Keyed by the history's parent, as the appendix keys it: the content
    /// runs when that parent is entered.
    pub default_history_content: SliceVec<'a, (StateId, HistoryId)>,
    /// The first effective target of the last transition that has one, in the
    /// order the transitions were selected: where the machine's current state
    /// is read down from once the set is entered. Kept here because the set's
    /// computation already dereferences every target, and what a `<history>`
    /// recorded does not change while the set is entered.
    pub last_target: Option<StateId>,
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
/// transition without targets enters nothing. Returns the entry set.
///
/// §scxml-D-enterStates enters the set in entryOrder — ancestors before
/// descendants, document order between the rest, which is exactly document
/// order, because that is a pre-order walk: ascending ids, the order a set of
/// states is read in. So the set is never sorted.
pub fn compute_entry_set<'a>(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
    transitions: &[TransitionId],
    scratch: Scratch<'a>,
) -> EntrySet<'a> {
    let (states_to_enter, rest) = scratch.take_set(tables);
    let (states_for_default_entry, rest) = rest.take_set(tables);
    let (mut effective, rest) = rest.take_set(tables);
    let mut entry = EntrySet {
        states_to_enter,
        states_for_default_entry,
        default_history_content: SliceVec::new(rest.history_defaults),
        last_target: None,
    };
    // One stack for the whole entry set: the walks run one after another, so
    // one stack of the document's size serves them all.
    let mut stack = SliceVec::new(rest.walk);
    for &transition in transitions {
        add_transition_to_entry_set(
            tables,
            store,
            transition,
            &mut stack,
            &mut effective,
            &mut entry,
        );
    }
    entry
}

/// One transition's part of Appendix D's computeEntrySet: what it adds to the
/// entry set.
fn add_transition_to_entry_set(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
    transition: TransitionId,
    stack: &mut SliceVec<'_, WalkStep>,
    effective: &mut StateSet<'_>,
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
        walk_entry(tables, store, WalkStep::Descendants(target), stack, entry);
    }
    let Some(first) =
        effective_target_states_on(tables, store, targets, effective, stack.slots_mut())
    else {
        return;
    };
    entry.last_target = Some(first);
    let domain = transition_domain(tables, row.source(), effective.as_ref(), row.internal());
    // Every effective target's own subtree went in with the descendants above,
    // so which ancestor walk runs first changes nothing: a region one of them
    // lies in is already taken. They are walked in document order.
    for state in effective.as_ref().iter() {
        walk_entry(
            tables,
            store,
            WalkStep::Ancestors(Target::state(state), domain),
            stack,
            entry,
        );
    }
}

impl<T> SliceVec<'_, T> {
    /// Every slot, for a walk that empties the buffer before it returns and
    /// lends its slots to another walk in between.
    ///
    /// The buffer is emptied as it lends them, so whatever the borrower leaves
    /// in a slot, initialised or not, is never read as an item.
    fn slots_mut(&mut self) -> &mut [MaybeUninit<T>] {
        debug_assert!(
            self.len == 0,
            "a walk lends its stack only while it is empty"
        );
        self.len = 0;
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
    store: RunStoreRef<'_>,
    first: WalkStep,
    stack: &mut SliceVec<'_, WalkStep>,
    entry: &mut EntrySet<'_>,
) {
    stack.push(first);
    while let Some(step) = stack.pop() {
        match step {
            WalkStep::Descendants(target) => match target.named() {
                Named::State(state) => {
                    entry.states_to_enter.insert(state);
                    match tables.state(state).kind() {
                        StateKind::Compound => {
                            // §scxml-D-addDescendantStatesToEnter: a compound
                            // state entered as a target is entered by DEFAULT —
                            // the one condition under which its initial
                            // transition's content runs. Its initial transition
                            // names the child or children it enters
                            // (§scxml-3.3), possibly several and possibly deep
                            // (§scxml-3.6).
                            entry.states_for_default_entry.insert(state);
                            stack.push(WalkStep::Listed {
                                source: Listed::Initial(state),
                                cursor: 0,
                                ancestors: false,
                            });
                        }
                        StateKind::Parallel => stack.push(WalkStep::Regions {
                            parallel: state,
                            next: StateId(state.0 + 1),
                        }),
                        StateKind::Atomic | StateKind::Final => {}
                    }
                }
                // §scxml-3.10: a transition to a history behaves as a transition
                // to the configuration it stored, or — before its parent was ever
                // visited — to its default stored configuration.
                Named::History(history) => {
                    let source =
                        (history_procedures(tables).descend)(tables, store, history, entry);
                    stack.push(WalkStep::Listed {
                        source,
                        cursor: 0,
                        ancestors: false,
                    });
                }
            },
            WalkStep::Listed {
                source,
                cursor,
                ancestors,
            } => {
                if let Some((target, cursor)) = listed_target(tables, store, source, cursor) {
                    // The rest of the list resumes once this target is done.
                    stack.push(WalkStep::Listed {
                        source,
                        cursor,
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
                        cursor: 0,
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
                entry.states_to_enter.insert(from);
                if let Some(next) = tables.parent(from).filter(|&next| Some(next) != stop) {
                    stack.push(WalkStep::AncestorsFrom { from: next, stop });
                }
                if tables.is_parallel(from) {
                    // Above the step that resumes the walk up, so the regions
                    // are done before the next ancestor is entered.
                    stack.push(WalkStep::Regions {
                        parallel: from,
                        next: StateId(from.0 + 1),
                    });
                }
            }
            WalkStep::Regions { parallel, next } => (parallel_procedures(tables).regions)(
                tables,
                parallel,
                next,
                stack,
                entry.states_to_enter.as_ref(),
            ),
            // The walk puts down steps of its own kind only; see
            // `effective_target_states_on` for why there is no message.
            _ => unreachable!(),
        }
    }
}

/// The target of `source`'s list at `cursor` and the cursor of the one after
/// it, or `None` once the list has ended. A list of targets is read by
/// position; what a `<history>` recorded is a set, read in document order from
/// the state `cursor` names.
fn listed_target(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
    source: Listed,
    cursor: u16,
) -> Option<(Target, u16)> {
    let targets = match source {
        Listed::Initial(state) => tables.targets(tables.state(state).initial()?),
        Listed::Recorded(history) => {
            let recorded_from = history_procedures(tables).recorded_from;
            let state = recorded_from(store, history, StateId(cursor))?;
            return Some((Target::state(state), state.0 + 1));
        }
        Listed::Defaults(history) => tables.targets(tables.history(history).default()),
    };
    let target = *targets.get(usize::from(cursor))?;
    Some((target, cursor + 1))
}

/// The state the ancestors of `source`'s targets are entered up to, not
/// including it: the compound state itself for its initial targets, the
/// `<history>`'s parent for a history's.
fn listed_stop(tables: &DocumentTables, source: Listed) -> StateId {
    match source {
        Listed::Initial(state) => state,
        Listed::Recorded(history) | Listed::Defaults(history) => tables.history(history).parent(),
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
    let first = match target.named() {
        Named::State(state) => tables.parent(state),
        Named::History(history) => Some(tables.history(history).parent()),
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

/// The step [`WalkStep::Regions`] of the `<parallel>` `parallel` from child
/// `next` on: the first region `states_to_enter` does not go into is entered by
/// default, with a step for the regions after it put down first. A
/// [`ParallelProcedures`] procedure ([`PARALLEL`]).
fn default_regions(
    tables: &DocumentTables,
    parallel: StateId,
    next: StateId,
    stack: &mut SliceVec<'_, WalkStep>,
    states_to_enter: StateSetRef<'_>,
) {
    // §scxml-3.4: every child of an active `<parallel>` is active, so a region
    // nothing on the set descends into is entered by default. A region's proper
    // descendants are the ids after it up to its last descendant.
    let last = tables.last_descendant(parallel);
    let mut child = next;
    while child <= last {
        let child_last = tables.last_descendant(child);
        let after = StateId(child_last.0 + 1);
        if !states_to_enter.meets(StateId(child.0 + 1), child_last) {
            if after <= last {
                stack.push(WalkStep::Regions {
                    parallel,
                    next: after,
                });
            }
            stack.push(WalkStep::Descendants(Target::state(child)));
            return;
        }
        child = after;
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

/// addDescendantStatesToEnter for the `<history>` target `history`: the list
/// the walk enters, and the default content it owes put in `entry`. A
/// [`HistoryProcedures`] procedure ([`HISTORY`]).
fn descend_into_history(
    tables: &DocumentTables,
    store: RunStoreRef<'_>,
    history: HistoryId,
    entry: &mut EntrySet<'_>,
) -> Listed {
    match store.history_value(history) {
        // §scxml-D-addDescendantStatesToEnter: a history that has recorded a
        // configuration enters it, and the ancestors between it and the
        // history's parent — a deep history records atomic states, which need
        // not be children.
        Some(recorded) if !recorded.is_empty() => Listed::Recorded(history),
        _ => {
            // §scxml-D-addDescendantStatesToEnter: nothing recorded yet, so the
            // default transition is taken, and its content is owed once the
            // parent has been entered — keyed by the parent, a later history of
            // the same parent replacing it.
            set_default_history_content(
                &mut entry.default_history_content,
                tables.history(history).parent(),
                history,
            );
            Listed::Defaults(history)
        }
    }
}

/// The first state `history` recorded from `from` on, in document order. A
/// [`HistoryProcedures`] procedure ([`HISTORY`]).
fn history_recorded_from(
    store: RunStoreRef<'_>,
    history: HistoryId,
    from: StateId,
) -> Option<StateId> {
    store.history_value(history)?.first_from(from)
}

/// Appendix D's exitStates, its history half over every `<history>`: each
/// whose parent is in `exited` records what it records. A
/// [`HistoryProcedures`] procedure ([`HISTORY`]).
fn record_histories(tables: &DocumentTables, mut store: RunStoreMut<'_>, exited: StateSetRef<'_>) {
    // §scxml-D-exitStates: every history of every state exited is recorded
    // from the configuration as it stood before the first exit.
    for (index, history) in tables.histories.iter().enumerate() {
        if exited.contains(history.parent()) {
            let (configuration, mut record) = store.record_history(HistoryId(index as u16));
            recorded_history(
                tables,
                history.parent(),
                history.deep(),
                configuration,
                &mut record,
            );
        }
    }
}

/// Appendix D's exitStates, its history half: what a `<history>` of `parent`
/// records as `parent` is exited — for a deep history the active atomic states
/// below `parent`, for a shallow one its active children — into `recorded`.
///
/// Read off `configuration_before_exit`, the configuration as it stood before
/// the microstep's first exit: the appendix records every history before it
/// exits any state, so every history of one microstep reads the same
/// configuration. Only `parent`'s subtree, a range of ids, is read.
pub fn recorded_history(
    tables: &DocumentTables,
    parent: StateId,
    deep: bool,
    configuration_before_exit: StateSetRef<'_>,
    recorded: &mut StateSet<'_>,
) {
    recorded.clear();
    let last = tables.last_descendant(parent);
    for state in configuration_before_exit
        .iter_from(StateId(parent.0 + 1))
        .take_while(|&state| state <= last)
    {
        let records = if deep {
            // §scxml-D-exitStates: `isAtomicState(s0) and isDescendant(s0, s)`,
            // the second of which is the range read.
            tables.is_atomic(state)
        } else {
            // §scxml-D-exitStates: `s0.parent == s`.
            tables.parent(state) == Some(parent)
        };
        if records {
            recorded.insert(state);
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
    configuration: StateSetRef<'_>,
) -> bool {
    match tables.state(state).kind() {
        StateKind::Compound => in_final_child(tables, state, configuration),
        StateKind::Parallel => (parallel_procedures(tables).in_final)(tables, state, configuration),
        StateKind::Atomic | StateKind::Final => false,
    }
}

/// [`is_in_final_state`] of the `<parallel>` `state`. A [`ParallelProcedures`]
/// procedure ([`PARALLEL`]).
fn parallel_in_final_state(
    tables: &DocumentTables,
    state: StateId,
    configuration: StateSetRef<'_>,
) -> bool {
    let last = tables.last_descendant(state);
    let mut next = state.0 + 1;
    while next <= last.0 {
        let region = StateId(next);
        match tables.state(region).kind() {
            // A nested `<parallel>`: its own regions are the next ids.
            StateKind::Parallel => next += 1,
            StateKind::Compound => {
                if !in_final_child(tables, region, configuration) {
                    return false;
                }
                next = tables.last_descendant(region).0 + 1;
            }
            StateKind::Atomic | StateKind::Final => return false,
        }
    }
    true
}

/// Whether one of the `<final>` children of the compound state `state` is active.
fn in_final_child(tables: &DocumentTables, state: StateId, configuration: StateSetRef<'_>) -> bool {
    tables
        .children(state)
        .any(|child| tables.is_final(child) && configuration.contains(child))
}
