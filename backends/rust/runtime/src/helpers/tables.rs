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
//! table for each machine: what a pre-order numbering already says (a state's
//! children) is not stored, an absent state or transition is the one number no
//! document reaches, and a target is one number that says whether it names a
//! state or a history.
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

/// The number a row holds where a state or a transition is absent: one past
/// the most ids a table can name, so no document reaches it.
const ABSENT: u16 = u16::MAX;

/// The most states, histories or transitions one table names: a [`Target`]
/// keeps the top bit of its number for whether it names a `<history>`, and an
/// absent row is the largest number.
pub const MAX_IDS: usize = 1 << 15;

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

/// A run of rows in the pool of targets of [`DocumentTables`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The first row.
    pub start: u16,
    /// How many rows.
    pub len: u16,
}

impl Span {
    /// No rows.
    pub const EMPTY: Span = Span { start: 0, len: 0 };

    /// The rows of `pool` this span names.
    #[inline]
    pub fn of<T>(self, pool: &[T]) -> &[T] {
        let start = usize::from(self.start);
        &pool[start..start + usize::from(self.len)]
    }

    /// One past the last row.
    #[inline]
    const fn end(self) -> usize {
        self.start as usize + self.len as usize
    }
}

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

/// One state of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateRow {
    parent: u16,
    last_descendant: StateId,
    initial: u16,
    kind: StateKind,
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
        Self {
            parent: match parent {
                Some(parent) => parent.0,
                None => ABSENT,
            },
            last_descendant,
            initial: match initial {
                Some(initial) => initial.0,
                None => ABSENT,
            },
            kind,
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
        self.kind
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
        if self.initial == ABSENT {
            None
        } else {
            Some(TransitionId(self.initial))
        }
    }
}

/// One `<history>` of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryRow {
    /// The state it is declared in.
    pub parent: StateId,
    /// Its default transition.
    pub default: TransitionId,
    /// Whether it is `type="deep"`.
    pub deep: bool,
}

/// One transition of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRow {
    source: u16,
    targets: Span,
    internal: bool,
    has_content: bool,
}

impl TransitionRow {
    /// A transition from `source` (`None`: the `<scxml>` element, the
    /// document's initial transition) to the targets `targets` names in
    /// [`DocumentTables::targets`], written `type="internal"` when `internal`,
    /// with executable content when `has_content`.
    pub const fn new(
        source: Option<StateId>,
        targets: Span,
        internal: bool,
        has_content: bool,
    ) -> Self {
        Self {
            source: match source {
                Some(source) => source.0,
                None => ABSENT,
            },
            targets,
            internal,
            has_content,
        }
    }

    /// Its source; `None` for the document's initial transition, whose source
    /// is the `<scxml>` element.
    #[inline]
    pub const fn source(&self) -> Option<StateId> {
        if self.source == ABSENT {
            None
        } else {
            Some(StateId(self.source))
        }
    }

    /// Its targets as written, in [`DocumentTables::targets`]; empty for a
    /// targetless transition.
    #[inline]
    pub const fn targets(&self) -> Span {
        self.targets
    }

    /// Whether it was written `type="internal"`.
    #[inline]
    pub const fn internal(&self) -> bool {
        self.internal
    }

    /// Whether it has executable content.
    #[inline]
    pub const fn has_content(&self) -> bool {
        self.has_content
    }
}

/// One target of a target list, as the document wrote it: a state or a
/// `<history>`, in one number whose top bit says which.
///
/// A history is not a state: it is never in a configuration, and the entry
/// procedures dereference it to what it recorded or to its default.
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

impl Target {
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

    /// What it names.
    #[inline]
    pub const fn named(self) -> Named {
        if self.0 & HISTORY_BIT == 0 {
            Named::State(StateId(self.0))
        } else {
            Named::History(HistoryId(self.0 & !HISTORY_BIT))
        }
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
    /// The pool [`TransitionRow::targets`] names.
    pub targets: &'static [Target],
    /// The document's initial transition, whose source is the `<scxml>`
    /// element.
    pub initial: TransitionId,
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
    /// More states, histories or transitions than [`MAX_IDS`].
    TooMany,
    /// A state's parent is not before it in document order.
    ParentNotBefore(StateId),
    /// A state's subtree is not the range of ids its children's subtrees fill.
    Subtree(StateId),
    /// A span names rows past the end of its pool.
    SpanOutOfRange,
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
        self.state(state).kind == StateKind::Compound
    }

    /// Whether `state` is a `<parallel>`.
    #[inline]
    pub fn is_parallel(&self, state: StateId) -> bool {
        self.state(state).kind == StateKind::Parallel
    }

    /// Whether `state` is a `<final>` element.
    #[inline]
    pub fn is_final(&self, state: StateId) -> bool {
        self.state(state).kind == StateKind::Final
    }

    /// Whether `state` has no child states: the appendix's isAtomicState, which
    /// a `<final>` is.
    #[inline]
    pub fn is_atomic(&self, state: StateId) -> bool {
        matches!(self.state(state).kind, StateKind::Atomic | StateKind::Final)
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

    /// `transition`'s targets as written.
    #[inline]
    pub fn targets(&self, transition: TransitionId) -> &[Target] {
        self.transition(transition).targets.of(self.targets)
    }

    /// Appendix D's isDescendant: whether `state` lies strictly below
    /// `ancestor`. A subtree is a range of ids, so this is two comparisons.
    #[inline]
    pub fn is_descendant(&self, state: StateId, ancestor: StateId) -> bool {
        ancestor < state && state <= self.state(ancestor).last_descendant
    }

    /// Whether the rows keep the rules of this module: every id below
    /// [`MAX_IDS`], every parent before its child, every subtree the range of
    /// ids its children's subtrees fill, every span inside its pool, an initial
    /// transition for exactly the compound states, every id a row names a row
    /// of its table, and the procedures of each feature the document uses
    /// carried. The procedures assume all of them; a debug build checks a
    /// machine's tables when it is built.
    pub fn check(&self) -> Result<(), TablesFault> {
        let states = self.states.len();
        if states > MAX_IDS || self.histories.len() > MAX_IDS || self.transitions.len() > MAX_IDS {
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
            let compound = row.kind == StateKind::Compound;
            match row.initial() {
                Some(initial) if compound && transition_ok(initial) => {}
                None if !compound => {}
                _ => return Err(TablesFault::Initial(id)),
            }
            let kind_ok = match row.kind {
                StateKind::Compound => children > 0,
                StateKind::Atomic | StateKind::Final => children == 0,
                StateKind::Parallel => true,
            };
            if !kind_ok {
                return Err(TablesFault::Kind(id));
            }
        }
        for row in self.histories {
            if !state_ok(row.parent) || !transition_ok(row.default) {
                return Err(TablesFault::Dangling);
            }
        }
        for row in self.transitions {
            if row.targets.end() > self.targets.len() {
                return Err(TablesFault::SpanOutOfRange);
            }
            if row.source().is_some_and(|source| !state_ok(source)) {
                return Err(TablesFault::Dangling);
            }
        }
        for target in self.targets {
            let ok = match target.named() {
                Named::State(state) => state_ok(state),
                Named::History(history) => history.index() < self.histories.len(),
            };
            if !ok {
                return Err(TablesFault::Dangling);
            }
        }
        if !transition_ok(self.initial) {
            return Err(TablesFault::Dangling);
        }
        let has_parallel = self
            .states
            .iter()
            .any(|row| row.kind == StateKind::Parallel);
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
