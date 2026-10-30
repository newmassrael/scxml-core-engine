// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document's structure as data: the tables Appendix D's procedures
//! ([`microstep`](super::microstep)) read, one constant for each machine.
//!
//! The procedures are compiled once for every program, whatever machines it
//! holds (docs/adr/0064). So they cannot name a machine's own types, and a
//! document reaches them as numbers:
//!
//! - a state is a [`StateId`], its position in document order. Document order is
//!   a pre-order walk, so a state's subtree is the contiguous range of ids from
//!   the state to its [`StateRow::last_descendant`], its first child is the next
//!   id and each further child the id after the subtree of the one before, entry
//!   order is ascending order and exit order is descending order;
//! - a `<history>` is a [`HistoryId`], its position in document order among the
//!   document's histories;
//! - a transition is a [`TransitionId`]. The rows are the written transitions,
//!   grouped by source in state order, then one initial transition for each
//!   compound state, one default transition for each `<history>`, and the
//!   document's own initial transition. Appendix D treats the last three as
//!   transitions too, and the entry procedures read their targets the same way;
//! - an event is an [`EventId`], the number of the machine's event enum's
//!   variant.
//!
//! The rows are as small as these numbers allow, since a program holds one
//! table for each machine (docs/adr/0067):
//! - what a pre-order numbering already says (a state's children) is not stored;
//! - an id is 14 bits, and the largest 14-bit number stands for an absent state
//!   or transition;
//! - the two bits above an id hold what the row would otherwise spend a byte on
//!   (a state's kind, a transition's flags, a history's depth);
//! - a target is one number that says whether it names a state or a history;
//! - a transition holds its only target itself, and the pool holds only lists
//!   of more than one.
//!
//! `sce_build::document_tables` computes the rows from the document, and the
//! generator writes them. A table written by hand states the same rows, and
//! [`DocumentTables::check`] says whether they keep the rules above.

use crate::helpers::microstep::{HistoryProcedures, ParallelProcedures};

/// A state: `<state>`, `<parallel>` or `<final>`, by its position in document
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct StateId(pub u16);

/// A `<history>` pseudo-state, by its position in document order among the
/// document's histories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct HistoryId(pub u16);

/// A transition: a written one, a compound state's initial transition, a
/// `<history>`'s default transition, or the document's initial transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct TransitionId(pub u16);

/// An event, by the number of its variant in the machine's event enum. The
/// machine's "no event", which selects eventless transitions, is one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct EventId(pub u16);

/// The bits of a row's field that hold an id (docs/adr/0067): 14, so the top
/// two bits of the field are free for what the row would otherwise spend a byte
/// on.
const ID_BITS: u16 = (1 << 14) - 1;

/// The number an id field holds where a state, a history or a transition is
/// absent: the largest 14-bit number, which no document reaches.
const ABSENT: u16 = ID_BITS;

/// The most states, histories or transitions one table names, and the most
/// entries its pool of target lists holds: every 14-bit number but the
/// largest, which stands for an absent state or transition.
pub const MAX_IDS: usize = ABSENT as usize;

impl StateId {
    /// The id as an index into a table.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl HistoryId {
    /// The id as an index into a table.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl TransitionId {
    /// The id as an index into a table.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// The history type of a document that declares no `<history>`.
///
/// Uninhabited, so its [`StatePolicy::history_id`](crate::StatePolicy::history_id)
/// is a `match` with no arm and no table of histories is ever indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoHistory {}

/// What kind of state a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StateKind {
    /// A `<state>` with no child states.
    Atomic,
    /// A `<state>` with child states.
    Compound,
    /// A `<parallel>`.
    Parallel,
    /// A `<final>`.
    Final,
}

/// The id a 14-bit field holds, or `None` for [`ABSENT`].
#[inline]
const fn id_of(field: u16) -> Option<u16> {
    let id = field & ID_BITS;
    if id == ABSENT {
        None
    } else {
        Some(id)
    }
}

/// The 14-bit field of an id that may be absent.
#[inline]
const fn field_of(id: Option<u16>) -> u16 {
    match id {
        Some(id) => id,
        None => ABSENT,
    }
}

/// One state of the document, in 6 bytes (docs/adr/0067): its parent, its last
/// descendant, and its kind in the top two bits of its initial transition.
///
/// The kind rides with the initial transition rather than the parent because
/// the parent is read on every walk up the tree and the initial transition only
/// when a compound state is entered by default: the field read most is the one
/// that needs no mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateRow {
    parent: u16,
    last_descendant: StateId,
    kind_initial: u16,
}

impl StateRow {
    /// A state whose parent is `parent` (`None`: the `<scxml>` element), whose
    /// subtree ends at `last_descendant` (itself when it has no child states),
    /// and whose initial transition is `initial` (a compound state's only).
    pub const fn new(
        parent: Option<StateId>,
        kind: StateKind,
        last_descendant: StateId,
        initial: Option<TransitionId>,
    ) -> Self {
        let parent = match parent {
            Some(parent) => Some(parent.0),
            None => None,
        };
        let initial = match initial {
            Some(initial) => Some(initial.0),
            None => None,
        };
        Self {
            parent: field_of(parent),
            last_descendant,
            kind_initial: ((kind as u16) << 14) | field_of(initial),
        }
    }

    /// Its parent; `None` when the parent is the `<scxml>` element.
    #[inline]
    pub const fn parent(&self) -> Option<StateId> {
        if self.parent == ABSENT {
            None
        } else {
            Some(StateId(self.parent))
        }
    }

    /// What kind of state it is.
    #[inline]
    pub const fn kind(&self) -> StateKind {
        match self.kind_initial >> 14 {
            0 => StateKind::Atomic,
            1 => StateKind::Compound,
            2 => StateKind::Parallel,
            _ => StateKind::Final,
        }
    }

    /// The largest id in its subtree, itself when it has no child states. The
    /// subtree is every id from the state's to this one.
    #[inline]
    pub const fn last_descendant(&self) -> StateId {
        self.last_descendant
    }

    /// Its initial transition, for a compound state.
    #[inline]
    pub const fn initial(&self) -> Option<TransitionId> {
        match id_of(self.kind_initial) {
            Some(initial) => Some(TransitionId(initial)),
            None => None,
        }
    }
}

// The sizes docs/adr/0067 decided, held where the rows are defined: a field
// added to a row, or a type that pads, fails the build rather than growing every
// machine's tables unseen.
const _: () = assert!(core::mem::size_of::<StateRow>() == 6);
const _: () = assert!(core::mem::size_of::<TransitionRow>() == 4);
const _: () = assert!(core::mem::size_of::<HistoryRow>() == 4);
const _: () = assert!(core::mem::size_of::<Target>() == 2);

/// The bit of a [`HistoryRow`]'s parent field that says it is deep.
const DEEP_BIT: u16 = 1 << 15;

/// One `<history>` of the document, in 4 bytes (docs/adr/0067): whether it is
/// deep in the top bit of its parent, and its default transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryRow {
    deep_parent: u16,
    default: TransitionId,
}

impl HistoryRow {
    /// A `<history>` declared in `parent`, whose default transition is
    /// `default`, `type="deep"` when `deep`.
    pub const fn new(parent: StateId, default: TransitionId, deep: bool) -> Self {
        Self {
            deep_parent: if deep { parent.0 | DEEP_BIT } else { parent.0 },
            default,
        }
    }

    /// The state it is declared in.
    #[inline]
    pub const fn parent(&self) -> StateId {
        StateId(self.deep_parent & ID_BITS)
    }

    /// Its default transition.
    #[inline]
    pub const fn default(&self) -> TransitionId {
        self.default
    }

    /// Whether it is `type="deep"`.
    #[inline]
    pub const fn deep(&self) -> bool {
        self.deep_parent & DEEP_BIT != 0
    }
}

/// The bits of a [`TransitionRow`]'s source field that say it is internal and
/// that it has content.
const INTERNAL_BIT: u16 = 1 << 15;
const CONTENT_BIT: u16 = 1 << 14;

/// One transition of the document, in 4 bytes (docs/adr/0067): whether it is
/// internal and whether it has content in the top two bits of its source, and
/// its targets as one [`Target`]: the only one it has, none, or a list in the
/// pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRow {
    flags_source: u16,
    targets: Target,
}

impl TransitionRow {
    /// A transition from `source` (`None`: the `<scxml>` element, the
    /// document's initial transition) to `targets` (a single target,
    /// [`Target::NONE`], or a [`Target::list`] in [`DocumentTables::targets`]),
    /// written `type="internal"` when `internal`, with executable content when
    /// `has_content`.
    pub const fn new(
        source: Option<StateId>,
        targets: Target,
        internal: bool,
        has_content: bool,
    ) -> Self {
        let source = match source {
            Some(source) => Some(source.0),
            None => None,
        };
        let mut flags_source = field_of(source);
        if internal {
            flags_source |= INTERNAL_BIT;
        }
        if has_content {
            flags_source |= CONTENT_BIT;
        }
        Self {
            flags_source,
            targets,
        }
    }

    /// Its source; `None` for the document's initial transition, whose source
    /// is the `<scxml>` element.
    #[inline]
    pub const fn source(&self) -> Option<StateId> {
        match id_of(self.flags_source) {
            Some(source) => Some(StateId(source)),
            None => None,
        }
    }

    /// Whether it was written `type="internal"`.
    #[inline]
    pub const fn internal(&self) -> bool {
        self.flags_source & INTERNAL_BIT != 0
    }

    /// Whether it has executable content.
    #[inline]
    pub const fn has_content(&self) -> bool {
        self.flags_source & CONTENT_BIT != 0
    }
}

/// One target of a target list, as the document wrote it: a state or a
/// `<history>`, in one number whose top bit says which.
///
/// A history is not a state: it is never in a configuration, and the entry
/// procedures dereference it to what it recorded or to its default.
///
/// A [`TransitionRow`] holds its targets as one of these (docs/adr/0067): the
/// only target it has, which is most transitions; [`Target::NONE`] for a
/// targetless one; or [`Target::list`], the position of a list in the pool,
/// whose first entry is its length ([`Target::count`]) and the rest its
/// targets. [`DocumentTables::targets`] reads all three as a slice of targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Target(u16);

/// What a [`Target`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Named {
    /// A `<state>`, `<parallel>` or `<final>`.
    State(StateId),
    /// A `<history>`.
    History(HistoryId),
}

/// The bit of a [`Target`] that says it names a `<history>`.
const HISTORY_BIT: u16 = 1 << 15;

/// The bit of a row's [`Target`] that says it is the position of a list in the
/// pool rather than a target.
const LIST_BIT: u16 = 1 << 14;

impl Target {
    /// A row's targets when it has none.
    pub const NONE: Self = Self(LIST_BIT | ABSENT);

    /// A target that names `state`.
    #[inline]
    pub const fn state(state: StateId) -> Self {
        Self(state.0)
    }

    /// A target that names `history`.
    #[inline]
    pub const fn history(history: HistoryId) -> Self {
        Self(history.0 | HISTORY_BIT)
    }

    /// A row's targets when they are the list at `position` of the pool.
    #[inline]
    pub const fn list(position: u16) -> Self {
        Self(position | LIST_BIT)
    }

    /// The first entry of a list in the pool: how many targets follow it.
    #[inline]
    pub const fn count(len: u16) -> Self {
        Self(len)
    }

    /// What it names.
    #[inline]
    pub const fn named(self) -> Named {
        if self.0 & HISTORY_BIT == 0 {
            Named::State(StateId(self.0 & ID_BITS))
        } else {
            Named::History(HistoryId(self.0 & ID_BITS))
        }
    }

    /// For a row's targets that are a list, its position in the pool, or
    /// `None` when there are none; not called for a single target.
    #[inline]
    const fn list_position(self) -> Option<usize> {
        match id_of(self.0) {
            Some(position) => Some(position as usize),
            None => None,
        }
    }

    /// Whether a row's targets are a list in the pool (or none).
    #[inline]
    const fn is_list(self) -> bool {
        self.0 & HISTORY_BIT == 0 && self.0 & LIST_BIT != 0
    }
}

/// A document's tables.
#[derive(Debug)]
pub struct DocumentTables {
    /// One row for each state, by [`StateId`].
    pub states: &'static [StateRow],
    /// One row for each `<history>`, by [`HistoryId`].
    pub histories: &'static [HistoryRow],
    /// One row for each transition, by [`TransitionId`].
    pub transitions: &'static [TransitionRow],
    /// The lists of a transition with more than one target, each its length
    /// ([`Target::count`]) followed by its targets; a row names one by
    /// [`Target::list`] (docs/adr/0067).
    pub targets: &'static [Target],
    /// The document's initial transition, whose source is the `<scxml>`
    /// element.
    pub initial: TransitionId,
    /// The state the machine names before its initial configuration is
    /// entered: the state type's `Default` and `StatePolicy::initial_state`
    /// (§scxml-3.2). A number in the tables, so an engine can hold its current
    /// state as a [`StateId`] from a constant expression (docs/adr/0068).
    pub default_state: StateId,
    /// The machine's "no event", which selects eventless transitions. A
    /// generated machine numbers it last.
    pub null_event: EventId,
    /// The events whose names begin with `error.`, in ascending order: the
    /// ones §scxml-3.12.2 has the processor raise, which the main loop counts
    /// when one matches nothing and bounds when an error handler raises
    /// another (docs/adr/0068). Written by the generator from the names, so
    /// the loop compares numbers and no machine carries a copy of the test.
    pub error_events: &'static [EventId],
    /// The procedures only a `<history>` reaches, `Some(&HISTORY)` exactly
    /// when the document has one (docs/adr/0066).
    pub history: Option<&'static HistoryProcedures>,
    /// The procedures only a `<parallel>` reaches, `Some(&PARALLEL)` exactly
    /// when the document has one.
    pub parallel: Option<&'static ParallelProcedures>,
}

/// Which rule a [`DocumentTables`] breaks; [`DocumentTables::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TablesFault {
    /// More states, histories or transitions than [`MAX_IDS`], or a pool of
    /// target lists longer than that.
    TooMany,
    /// A state's parent is not before it in document order.
    ParentNotBefore(StateId),
    /// A state's subtree is not the range of ids its children's subtrees fill.
    Subtree(StateId),
    /// A transition's list of targets runs past the end of the pool.
    ListOutOfRange,
    /// A compound state has no initial transition, or another state has one.
    Initial(StateId),
    /// A state's kind disagrees with its children: a compound state without
    /// child states, or an atomic or final one with some.
    Kind(StateId),
    /// A row names a state, a history or a transition that does not exist.
    Dangling,
    /// The document has a `<history>` and the tables carry no
    /// [`DocumentTables::history`], or a `<parallel>` and no
    /// [`DocumentTables::parallel`].
    ProceduresMissing,
}

impl DocumentTables {
    /// The row of `state`.
    #[inline]
    pub fn state(&self, state: StateId) -> &StateRow {
        &self.states[state.index()]
    }

    /// The row of `history`.
    #[inline]
    pub fn history(&self, history: HistoryId) -> &HistoryRow {
        &self.histories[history.index()]
    }

    /// The row of `transition`.
    #[inline]
    pub fn transition(&self, transition: TransitionId) -> &TransitionRow {
        &self.transitions[transition.index()]
    }

    /// `state`'s parent; `None` when it is the `<scxml>` element.
    #[inline]
    pub fn parent(&self, state: StateId) -> Option<StateId> {
        self.state(state).parent()
    }

    /// The largest id in `state`'s subtree.
    #[inline]
    pub fn last_descendant(&self, state: StateId) -> StateId {
        self.state(state).last_descendant
    }

    /// Whether `state` is a `<state>` with child states. A `<parallel>` is not:
    /// this is the appendix's isCompoundState.
    #[inline]
    pub fn is_compound(&self, state: StateId) -> bool {
        self.state(state).kind() == StateKind::Compound
    }

    /// Whether `state` is a `<parallel>`.
    #[inline]
    pub fn is_parallel(&self, state: StateId) -> bool {
        self.state(state).kind() == StateKind::Parallel
    }

    /// Whether `state` is a `<final>` element.
    #[inline]
    pub fn is_final(&self, state: StateId) -> bool {
        self.state(state).kind() == StateKind::Final
    }

    /// Whether `state` has no child states: the appendix's isAtomicState, which
    /// a `<final>` is.
    #[inline]
    pub fn is_atomic(&self, state: StateId) -> bool {
        matches!(
            self.state(state).kind(),
            StateKind::Atomic | StateKind::Final
        )
    }

    /// Appendix D's getChildStates: `state`'s child states, in document order —
    /// the next id, then each id after the subtree of the one before, up to
    /// the end of `state`'s subtree.
    #[inline]
    pub fn children(&self, state: StateId) -> Children<'_> {
        Children {
            tables: self,
            next: state.0 + 1,
            last: self.state(state).last_descendant,
        }
    }

    /// `transition`'s targets as written: the row's own target when it has one,
    /// none, or its list in the pool after the list's length.
    ///
    /// Out of line: the procedures ask it in a handful of places, and they are
    /// compiled once for every program, so one copy and a call is smaller than
    /// the decoding written into each of them.
    #[inline(never)]
    pub fn targets(&self, transition: TransitionId) -> &[Target] {
        let targets = &self.transition(transition).targets;
        if !targets.is_list() {
            return core::slice::from_ref(targets);
        }
        match targets.list_position() {
            Some(position) => {
                let len = usize::from(self.targets[position].0);
                &self.targets[position + 1..position + 1 + len]
            }
            None => &[],
        }
    }

    /// Whether `event` is an `error.*` event ([`DocumentTables::error_events`]).
    #[inline]
    pub fn is_error_event(&self, event: EventId) -> bool {
        self.error_events.contains(&event)
    }

    /// Appendix D's isDescendant: whether `state` lies strictly below
    /// `ancestor`. A subtree is a range of ids, so this is two comparisons.
    #[inline]
    pub fn is_descendant(&self, state: StateId, ancestor: StateId) -> bool {
        ancestor < state && state <= self.state(ancestor).last_descendant
    }

    /// Whether the rows keep the rules of this module: every id below
    /// [`MAX_IDS`], every parent before its child, every subtree the range of
    /// ids its children's subtrees fill, every list of targets inside the pool,
    /// an initial transition for exactly the compound states, every id a row
    /// names a row of its table, and the procedures of each feature the
    /// document uses carried. The procedures assume all of them; a debug build
    /// checks a machine's tables when it is built.
    pub fn check(&self) -> Result<(), TablesFault> {
        let states = self.states.len();
        if states > MAX_IDS
            || self.histories.len() > MAX_IDS
            || self.transitions.len() > MAX_IDS
            || self.targets.len() > MAX_IDS
        {
            return Err(TablesFault::TooMany);
        }
        let state_ok = |id: StateId| id.index() < states;
        let transition_ok = |id: TransitionId| id.index() < self.transitions.len();
        for (index, row) in self.states.iter().enumerate() {
            let id = StateId(index as u16);
            if let Some(parent) = row.parent() {
                if parent >= id {
                    return Err(TablesFault::ParentNotBefore(id));
                }
            }
            if !state_ok(row.last_descendant) || row.last_descendant < id {
                return Err(TablesFault::Subtree(id));
            }
            // The children's subtrees, one after another, fill the range from
            // the state's next id to its last descendant, each the child of
            // this state.
            let mut children = 0;
            for child in self.children(id) {
                let child_row = &self.states[child.index()];
                if child_row.parent() != Some(id)
                    || child_row.last_descendant < child
                    || child_row.last_descendant > row.last_descendant
                {
                    return Err(TablesFault::Subtree(id));
                }
                children += 1;
            }
            let compound = row.kind() == StateKind::Compound;
            match row.initial() {
                Some(initial) if compound && transition_ok(initial) => {}
                None if !compound => {}
                _ => return Err(TablesFault::Initial(id)),
            }
            let kind_ok = match row.kind() {
                StateKind::Compound => children > 0,
                StateKind::Atomic | StateKind::Final => children == 0,
                StateKind::Parallel => true,
            };
            if !kind_ok {
                return Err(TablesFault::Kind(id));
            }
        }
        for row in self.histories {
            if !state_ok(row.parent()) || !transition_ok(row.default()) {
                return Err(TablesFault::Dangling);
            }
        }
        for (index, row) in self.transitions.iter().enumerate() {
            if row.targets.is_list() {
                if let Some(position) = row.targets.list_position() {
                    let fits = self
                        .targets
                        .get(position)
                        .is_some_and(|len| position + 1 + usize::from(len.0) <= self.targets.len());
                    if !fits {
                        return Err(TablesFault::ListOutOfRange);
                    }
                }
            }
            if row.source().is_some_and(|source| !state_ok(source)) {
                return Err(TablesFault::Dangling);
            }
            for target in self.targets(TransitionId(index as u16)) {
                let ok = match target.named() {
                    Named::State(state) => state_ok(state),
                    Named::History(history) => history.index() < self.histories.len(),
                };
                if !ok {
                    return Err(TablesFault::Dangling);
                }
            }
        }
        if !transition_ok(self.initial) || !state_ok(self.default_state) {
            return Err(TablesFault::Dangling);
        }
        // The error events are named events of the machine, so none is the
        // null event; ascending, so none repeats.
        let ascending = self.error_events.windows(2).all(|pair| pair[0] < pair[1]);
        let named = !self.error_events.contains(&self.null_event);
        if !ascending || !named {
            return Err(TablesFault::Dangling);
        }
        let has_parallel = self
            .states
            .iter()
            .any(|row| row.kind() == StateKind::Parallel);
        if (!self.histories.is_empty() && self.history.is_none())
            || (has_parallel && self.parallel.is_none())
        {
            return Err(TablesFault::ProceduresMissing);
        }
        Ok(())
    }
}

/// The child states of one state, in document order: [`DocumentTables::children`].
#[derive(Debug, Clone)]
pub struct Children<'t> {
    tables: &'t DocumentTables,
    next: u16,
    last: StateId,
}

impl Iterator for Children<'_> {
    type Item = StateId;

    #[inline]
    fn next(&mut self) -> Option<StateId> {
        if self.next > self.last.0 {
            return None;
        }
        let child = StateId(self.next);
        self.next = self.tables.state(child).last_descendant.0 + 1;
        Some(child)
    }
}
