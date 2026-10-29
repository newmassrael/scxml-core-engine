// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document's structure as data: the tables Appendix D's procedures
//! ([`microstep`](super::microstep)) read, one `static` for each machine.
//!
//! The procedures are compiled once for every program, whatever machines it
//! holds (docs/adr/0064). So they cannot name a machine's own types, and a
//! document reaches them as numbers:
//!
//! - a state is a [`StateId`], its position in document order. Document order is
//!   a pre-order walk, so a state's subtree is the contiguous range of ids from
//!   the state to its [`StateRow::last_descendant`], entry order is ascending
//!   order and exit order is descending order;
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
//! `sce_build::document_tables` computes the rows from the document, and the
//! generator writes them. A table written by hand states the same rows, and
//! [`DocumentTables::check`] says whether they keep the rules above.

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

/// A run of rows in one of the pools of [`DocumentTables`].
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
    /// Its parent; `None` when the parent is the `<scxml>` element.
    pub parent: Option<StateId>,
    /// What kind of state it is.
    pub kind: StateKind,
    /// The largest id in its subtree, itself when it has no child states. The
    /// subtree is every id from the state's to this one.
    pub last_descendant: StateId,
    /// Its child states in document order, in [`DocumentTables::children`].
    pub children: Span,
    /// Its initial transition, for a compound state.
    pub initial: Option<TransitionId>,
}

/// One `<history>` of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryRow {
    /// The state it is declared in.
    pub parent: StateId,
    /// Whether it is `type="deep"`.
    pub deep: bool,
    /// Its default transition.
    pub default: TransitionId,
    /// Where what it records is kept, in a pool of
    /// [`DocumentTables::history_records`] states the engine holds.
    pub record: Span,
}

/// One transition of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRow {
    /// Its source; `None` for the document's initial transition, whose source
    /// is the `<scxml>` element.
    pub source: Option<StateId>,
    /// Its targets as written, in [`DocumentTables::targets`]; empty for a
    /// targetless transition.
    pub targets: Span,
    /// Whether it was written `type="internal"`.
    pub internal: bool,
    /// Whether it has executable content.
    pub has_content: bool,
}

/// One target of a target list, as the document wrote it.
///
/// A history is not a state: it is never in a configuration, and the entry
/// procedures dereference it to what it recorded or to its default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A `<state>`, `<parallel>` or `<final>`.
    State(StateId),
    /// A `<history>`.
    History(HistoryId),
}

/// A document's tables, and the numbers its buffers are sized by.
#[derive(Debug)]
pub struct DocumentTables {
    /// One row for each state, by [`StateId`].
    pub states: &'static [StateRow],
    /// The pool [`StateRow::children`] names.
    pub children: &'static [StateId],
    /// One row for each `<history>`, by [`HistoryId`].
    pub histories: &'static [HistoryRow],
    /// One row for each transition, by [`TransitionId`].
    pub transitions: &'static [TransitionRow],
    /// The pool [`TransitionRow::targets`] names.
    pub targets: &'static [Target],
    /// The document's initial transition, whose source is the `<scxml>`
    /// element.
    pub initial: TransitionId,
    /// How many states the history records of the document hold together.
    pub history_records: u16,
    /// The most states one configuration holds (docs/adr/0056).
    pub max_configuration: u16,
    /// The most transitions one selection enables.
    pub max_enabled: u16,
    /// The most `<history>` default transitions one microstep owes.
    pub max_history_defaults: u16,
    /// The most steps the entry walks owe at once.
    pub max_walk: u16,
}

/// Which rule a [`DocumentTables`] breaks; [`DocumentTables::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TablesFault {
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
        self.state(state).parent
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

    /// Appendix D's getChildStates: `state`'s child states, in document order.
    #[inline]
    pub fn child_states(&self, state: StateId) -> &[StateId] {
        self.state(state).children.of(self.children)
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

    /// Whether the rows keep the rules of this module: every parent before its
    /// child, every subtree the range of ids its children's subtrees fill, every
    /// span inside its pool, an initial transition for exactly the compound
    /// states, and every id a row names a row of its table. The procedures assume
    /// all of them; a debug build checks a machine's tables when it is built.
    pub fn check(&self) -> Result<(), TablesFault> {
        let states = self.states.len();
        let state_ok = |id: StateId| id.index() < states;
        let transition_ok = |id: TransitionId| id.index() < self.transitions.len();
        for (index, row) in self.states.iter().enumerate() {
            let id = StateId(index as u16);
            if let Some(parent) = row.parent {
                if parent >= id {
                    return Err(TablesFault::ParentNotBefore(id));
                }
            }
            if row.children.end() > self.children.len() {
                return Err(TablesFault::SpanOutOfRange);
            }
            if !state_ok(row.last_descendant) || row.last_descendant < id {
                return Err(TablesFault::Subtree(id));
            }
            // The children's subtrees, one after another, fill the range from
            // the state's next id to its last descendant.
            let mut next = index + 1;
            for &child in row.children.of(self.children) {
                if child.index() != next || self.parent_row(child) != Some(id) {
                    return Err(TablesFault::Subtree(id));
                }
                next = self.state(child).last_descendant.index() + 1;
            }
            if next != row.last_descendant.index() + 1 {
                return Err(TablesFault::Subtree(id));
            }
            let compound = row.kind == StateKind::Compound;
            match row.initial {
                Some(initial) if compound && transition_ok(initial) => {}
                None if !compound => {}
                _ => return Err(TablesFault::Initial(id)),
            }
            let kind_ok = match row.kind {
                StateKind::Compound => row.children.len > 0,
                StateKind::Atomic | StateKind::Final => row.children.len == 0,
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
            if row.record.end() > usize::from(self.history_records) {
                return Err(TablesFault::SpanOutOfRange);
            }
        }
        for row in self.transitions {
            if row.targets.end() > self.targets.len() {
                return Err(TablesFault::SpanOutOfRange);
            }
            if row.source.is_some_and(|source| !state_ok(source)) {
                return Err(TablesFault::Dangling);
            }
        }
        for target in self.targets {
            let ok = match *target {
                Target::State(state) => state_ok(state),
                Target::History(history) => history.index() < self.histories.len(),
            };
            if !ok {
                return Err(TablesFault::Dangling);
            }
        }
        if !transition_ok(self.initial) {
            return Err(TablesFault::Dangling);
        }
        Ok(())
    }

    fn parent_row(&self, state: StateId) -> Option<StateId> {
        self.states.get(state.index()).and_then(|row| row.parent)
    }
}
