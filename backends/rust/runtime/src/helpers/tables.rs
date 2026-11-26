// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! A document whose microsteps' sets the document itself fixes (no
//! `<history>`, no `<parallel>`, no transition of more than one target) also
//! carries, for each transition, the states it enters as one more row: its
//! entry path, which Appendix D's procedures computed when the document was
//! built, and which the row walk reads instead of computing it again
//! (docs/adr/0125).
//!
//! A generated document also carries what enables each of its written
//! transitions ([`SelectionRows`], docs/adr/0127): each state's range of
//! written transitions, and for each transition the events its descriptor
//! matches, as numbers of the machine's event enum. The generator computed the
//! lists from the events that enum numbers, so the one selection
//! ([`DocumentTables::first_enabled`]) compares numbers and asks the machine
//! only whether a guard holds. A table written by hand carries none, and its
//! machine answers the selection itself.
//!
//! A document the row walk serves selects from its candidate rows instead
//! ([`CandidateRows`], docs/adr/0135): the same lists fused over each atomic
//! state's ancestors, so for an atomic state and an event the rows name every
//! transition the selection tries, in the order it tries them, and the row
//! walk's selection reads one list and asks the machine only whether a guard
//! holds.
//!
//! `sce_build::document_tables` computes the rows from the document, and the
//! generator writes them. A table written by hand states the same rows, and
//! [`DocumentTables::check`] says whether they keep the rules above.

use crate::helpers::microstep::{Behaviour, HistoryProcedures, ParallelProcedures, WalkProcedures};

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

impl EventId {
    /// The id as an index into a table: the machine's events are numbered by
    /// their variants' order.
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
/// A compound state's initial transition is the one its `initial` attribute or
/// its `<initial>` child names, or, without either, one to its first child state
/// in document order (§scxml-3.3).
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
const _: () = assert!(core::mem::size_of::<EntryRow>() == 2);
const _: () = assert!(core::mem::size_of::<EntryStep>() == 2);
const _: () = assert!(core::mem::size_of::<TriggerRow>() == 2);
const _: () = assert!(core::mem::size_of::<TriggerEvent>() == 2);
const _: () = assert!(core::mem::size_of::<CandidateCell>() == 4);
const _: () = assert!(core::mem::size_of::<Candidate>() == 2);

/// The bit of a [`HistoryRow`]'s parent field that says it is deep.
const DEEP_BIT: u16 = 1 << 15;

/// One `<history>` of the document, in 4 bytes (docs/adr/0067): whether it is
/// deep in the top bit of its parent, and its default transition: the one its
/// `<transition>` child writes, whose target is the default history
/// configuration (§scxml-3.10.2).
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

/// What one transition enters, as the row walk reads it (docs/adr/0125): the
/// position of its entry path in [`DocumentTables::entry_paths`], or
/// [`EntryRow::NONE`] for a transition with no target, which exits and enters
/// nothing.
///
/// Transitions that enter the same states in the same way, which is every
/// transition to one target from under one domain, name one list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EntryRow(u16);

impl EntryRow {
    /// The row of a transition with no target.
    pub const NONE: Self = Self(ABSENT);

    /// The row of a transition whose entry path is the list at `position` of
    /// [`DocumentTables::entry_paths`].
    #[inline]
    pub const fn at(position: u16) -> Self {
        Self(position)
    }

    /// The position of its list, or `None` for a transition with no target.
    #[inline]
    const fn position(self) -> Option<usize> {
        match id_of(self.0) {
            Some(position) => Some(position as usize),
            None => None,
        }
    }
}

/// The bit of an [`EntryStep`] that says its state is entered by default.
const DEFAULT_ENTRY_BIT: u16 = 1 << 15;

/// One entry of a list of [`DocumentTables::entry_paths`] (docs/adr/0125): the
/// first entry of a list is its length ([`EntryStep::count`]), and each one
/// after it a state the transition enters, in entry order, with whether it is
/// entered by default, Appendix D's statesForDefaultEntry, so its initial
/// transition's content runs after its onentry. The state is the 14 bits of
/// an id and the flag the top bit (docs/adr/0067).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EntryStep(u16);

impl EntryStep {
    /// A state of an entry path, entered by default when `by_default`.
    #[inline]
    pub const fn new(state: StateId, by_default: bool) -> Self {
        if by_default {
            Self(state.0 | DEFAULT_ENTRY_BIT)
        } else {
            Self(state.0)
        }
    }

    /// The first entry of a list: how many states follow it.
    #[inline]
    pub const fn count(len: u16) -> Self {
        Self(len)
    }

    /// The state it enters.
    #[inline]
    pub const fn state(self) -> StateId {
        StateId(self.0 & ID_BITS)
    }

    /// Whether the state is entered by default.
    #[inline]
    pub const fn by_default(self) -> bool {
        self.0 & DEFAULT_ENTRY_BIT != 0
    }
}

/// What enables one transition, as the selection reads it (docs/adr/0127): the
/// position of its list of events in [`SelectionRows::trigger_events`], or
/// [`TriggerRow::NONE`] for a transition no selection returns. That is a
/// compound state's initial transition, a `<history>`'s default transition,
/// the document's initial transition, and a written transition whose
/// descriptor matches no event of the machine.
///
/// Transitions that the same events enable, which is every transition written
/// for one event and every eventless one, name one list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TriggerRow(u16);

impl TriggerRow {
    /// The row of a transition no selection returns.
    pub const NONE: Self = Self(ABSENT);

    /// The row of a transition whose events are the list at `position` of
    /// [`SelectionRows::trigger_events`].
    #[inline]
    pub const fn at(position: u16) -> Self {
        Self(position)
    }

    /// The position of its list, or `None` for a transition no selection
    /// returns.
    #[inline]
    const fn position(self) -> Option<usize> {
        match id_of(self.0) {
            Some(position) => Some(position as usize),
            None => None,
        }
    }
}

/// One entry of a list of [`SelectionRows::trigger_events`] (docs/adr/0127):
/// the first entry of a list is its length ([`TriggerEvent::count`]), and each
/// one after it an event that enables the transition, in ascending order of
/// the events' numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TriggerEvent(u16);

impl TriggerEvent {
    /// An event of a list.
    #[inline]
    pub const fn new(event: EventId) -> Self {
        Self(event.0)
    }

    /// The first entry of a list: how many events follow it.
    #[inline]
    pub const fn count(len: u16) -> Self {
        Self(len)
    }

    /// The event it names.
    #[inline]
    pub const fn event(self) -> EventId {
        EventId(self.0)
    }
}

/// What enables each written transition of a document, as its generator
/// computed it (docs/adr/0127): the rows the one selection
/// ([`DocumentTables::first_enabled`]) reads in place of a machine's own
/// `first_enabled_transition`.
///
/// The written transitions are the first rows of the transitions table,
/// grouped by source in state order and within a source in the order they are
/// written, so a state's own transitions are one range of numbers, read from
/// [`first_written`](Self::first_written). Each transition's
/// [`TriggerRow`] names the events that enable it, numbered as the machine's
/// event enum numbers them: those its event descriptor matches for a written
/// transition with an event, the tables' [`null_event`](DocumentTables::null_event)
/// alone for an eventless one. Nothing here says whether a guard holds: the
/// machine answers that ([`Behaviour::guard_holds`]), so the generated code
/// stays the one judge of which transitions are guarded.
#[derive(Debug, Clone, Copy)]
pub struct SelectionRows<'t> {
    /// For each state, by [`StateId`], the number of its first written
    /// transition, and one entry past the last state, the number of written
    /// transitions: a state's own are the numbers from its entry up to the
    /// next one's.
    pub first_written: &'t [TransitionId],
    /// For each transition, by [`TransitionId`], what enables it.
    pub triggers: &'t [TriggerRow],
    /// The lists the rows name, each its length ([`TriggerEvent::count`])
    /// followed by its events in ascending order. Identical lists are held
    /// once.
    pub trigger_events: &'t [TriggerEvent],
}

impl SelectionRows<'_> {
    /// The written transitions of `state`, in the order they are written, as
    /// the range of their numbers; see [`DocumentTables::state`] for how a
    /// missing row stops.
    #[inline]
    fn written(&self, state: StateId) -> core::ops::Range<u16> {
        match (
            self.first_written.get(state.index()),
            self.first_written.get(state.index() + 1),
        ) {
            (Some(first), Some(end)) => first.0..end.0,
            _ => row_missing(),
        }
    }

    /// The events that enable `transition`, in ascending order; none for a
    /// transition no selection returns.
    pub fn events(&self, transition: TransitionId) -> &[TriggerEvent] {
        let row = match self.triggers.get(transition.index()) {
            Some(row) => *row,
            None => row_missing(),
        };
        let Some(position) = row.position() else {
            return &[];
        };
        let len = match self.trigger_events.get(position) {
            Some(len) => usize::from(len.0),
            None => row_missing(),
        };
        match self.trigger_events.get(position + 1..position + 1 + len) {
            Some(list) => list,
            None => row_missing(),
        }
    }

    /// Whether `event` enables `transition`.
    #[inline]
    pub fn enables(&self, transition: TransitionId, event: EventId) -> bool {
        self.events(transition)
            .iter()
            .any(|entry| entry.event() == event)
    }
}

/// One cell of a document's candidate rows (docs/adr/0135): an event an atomic
/// state answers, and the position of the list of transitions the row walk's
/// selection tries for it in [`CandidateRows::candidates`]. Four bytes, two
/// numbers (docs/adr/0067).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateCell {
    event: EventId,
    list: u16,
}

impl CandidateCell {
    /// The cell of `event`, whose transitions are the list at `position` of
    /// [`CandidateRows::candidates`].
    #[inline]
    pub const fn new(event: EventId, position: u16) -> Self {
        Self {
            event,
            list: position,
        }
    }

    /// The event the cell answers.
    #[inline]
    pub const fn event(self) -> EventId {
        self.event
    }

    /// The position of its list in [`CandidateRows::candidates`].
    #[inline]
    const fn position(self) -> usize {
        self.list as usize
    }
}

/// One entry of a list of [`CandidateRows::candidates`] (docs/adr/0135): the
/// first entry of a list is its length ([`Candidate::count`]), and each one
/// after it a transition, in the order the selection tries them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Candidate(u16);

impl Candidate {
    /// A transition of a list.
    #[inline]
    pub const fn new(transition: TransitionId) -> Self {
        Self(transition.0)
    }

    /// The first entry of a list: how many transitions follow it.
    #[inline]
    pub const fn count(len: u16) -> Self {
        Self(len)
    }

    /// The transition it names.
    #[inline]
    pub const fn transition(self) -> TransitionId {
        TransitionId(self.0)
    }
}

/// What a document the row walk serves selects from (docs/adr/0135): for each
/// atomic state and each event the machine numbers, the null event among them,
/// the written transitions Appendix D's selection tries in a configuration
/// whose atomic state it is, in the order it tries them: the state's own in
/// the order they are written, then each ancestor's, outward. The row walk's
/// selection reads the list of the configuration's atomic state and asks the
/// machine [`Behaviour::guard_holds`] for each transition in turn, until one
/// holds.
///
/// The relation is sparse, since a state answers a few of the events, and is
/// held as compressed sparse rows: [`first_cell`](Self::first_cell) gives each
/// state's range of [`cells`](Self::cells), a cell for each event the state
/// answers in ascending order of the event, and each cell names its list in
/// [`candidates`](Self::candidates). A state that is not atomic has no cell.
/// Identical lists are held once.
#[derive(Debug, Clone, Copy)]
pub struct CandidateRows<'t> {
    /// For each state, by [`StateId`], the position of its first cell, and
    /// one entry past the last state, the number of cells: a state's cells are
    /// the positions from its entry up to the next one's.
    pub first_cell: &'t [u16],
    /// The cells, each state's in ascending order of the event.
    pub cells: &'t [CandidateCell],
    /// The lists the cells name, each its length ([`Candidate::count`])
    /// followed by its transitions.
    pub candidates: &'t [Candidate],
}

impl CandidateRows<'_> {
    /// The cells of `state`, in ascending order of the event; see
    /// [`DocumentTables::state`] for how a missing row stops.
    #[inline]
    fn cells_of(&self, state: StateId) -> &[CandidateCell] {
        let range = match (
            self.first_cell.get(state.index()),
            self.first_cell.get(state.index() + 1),
        ) {
            (Some(first), Some(end)) => usize::from(*first)..usize::from(*end),
            _ => row_missing(),
        };
        match self.cells.get(range) {
            Some(cells) => cells,
            None => row_missing(),
        }
    }

    /// The transitions the selection tries for `event` in a configuration
    /// whose atomic state is `atomic`, in the order it tries them; none when
    /// `event` enables no transition of `atomic` or of an ancestor.
    pub fn candidates_of(&self, atomic: StateId, event: EventId) -> &[Candidate] {
        let cells = self.cells_of(atomic);
        let Ok(found) = cells.binary_search_by_key(&event, |cell| cell.event) else {
            return &[];
        };
        match cells.get(found) {
            Some(cell) => listed(self.candidates, cell.position(), |count| count.0),
            None => row_missing(),
        }
    }
}

/// A document's tables, their rows borrowed for `'t`.
///
/// A generated machine's rows are constants of its program,
/// `DocumentTables<'static>`, as `StatePolicy::TABLES` names them. A reader
/// that builds the rows of a document it reads, and asks the procedures about
/// it, lends rows it owns for the length of the question, and leaves nothing
/// behind. The procedures a document is given by these rows,
/// [`walk`](Self::walk), [`history`](Self::history) and
/// [`parallel`](Self::parallel), are the runtime's own constants, so they are
/// `'static` whoever owns the rows.
#[derive(Debug)]
pub struct DocumentTables<'t> {
    /// One row for each state, by [`StateId`].
    pub states: &'t [StateRow],
    /// One row for each `<history>`, by [`HistoryId`].
    pub histories: &'t [HistoryRow],
    /// One row for each transition, by [`TransitionId`].
    pub transitions: &'t [TransitionRow],
    /// The lists of a transition with more than one target, each its length
    /// ([`Target::count`]) followed by its targets; a row names one by
    /// [`Target::list`] (docs/adr/0067).
    pub targets: &'t [Target],
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
    pub error_events: &'t [EventId],
    /// How the document's microsteps form their exit and entry sets
    /// (docs/adr/0125): `&GENERAL_WALK`, Appendix D's procedures computing them
    /// from the tree and the configuration, which serves every document; or
    /// `&ROW_WALK`, which reads them from [`entry_rows`](Self::entry_rows) and
    /// selects from [`candidates`](Self::candidates) (docs/adr/0135), for a
    /// document with no `<history>`, no `<parallel>` and no transition of more
    /// than one target. A program none of whose documents carries the general
    /// walk links none of it.
    pub walk: &'static WalkProcedures,
    /// For each transition, by [`TransitionId`], what it enters: one row each
    /// when [`walk`](Self::walk) is the row walk, none otherwise.
    pub entry_rows: &'t [EntryRow],
    /// The entry paths the rows name, each its length
    /// ([`EntryStep::count`]) followed by the states it enters in entry order.
    pub entry_paths: &'t [EntryStep],
    /// The procedures only a `<history>` reaches, `Some(&HISTORY)` exactly
    /// when the document has one (docs/adr/0066).
    pub history: Option<&'static HistoryProcedures>,
    /// The procedures only a `<parallel>` reaches, `Some(&PARALLEL)` exactly
    /// when the document has one.
    pub parallel: Option<&'static ParallelProcedures>,
    /// What enables each written transition (docs/adr/0127): `Some` in every
    /// generated machine's tables, whose selection reads them
    /// ([`first_enabled`](Self::first_enabled)); `None` in tables written by
    /// hand, whose machine answers the selection itself
    /// ([`Behaviour::first_enabled_transition`]). `None` too in the tables of
    /// the row walk, which select from [`candidates`](Self::candidates).
    pub selection: Option<SelectionRows<'t>>,
    /// What the row walk's selection reads (docs/adr/0135): `Some` exactly
    /// when [`walk`](Self::walk) is the row walk, `None` in the tables of the
    /// general walk, whose selection reads [`selection`](Self::selection).
    pub candidates: Option<CandidateRows<'t>>,
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
    /// The tables carry the row walk, and the document has a `<history>`, a
    /// `<parallel>` or a transition of more than one target, whose sets its
    /// rows do not fix (docs/adr/0125).
    RowWalkOutsideClass,
    /// The row walk without one entry row for each transition, or entry rows
    /// in tables whose walk reads none.
    EntryRows,
    /// A transition's entry row breaks a rule of an entry path
    /// ([`DocumentTables::check`]).
    EntryRow(TransitionId),
    /// The selection rows do not have one entry for each state and one past
    /// the last, one row for each transition, or ranges that are the written
    /// transitions of each state in turn (docs/adr/0127).
    Selection,
    /// A transition's list of events breaks a rule of the selection rows
    /// ([`DocumentTables::check`]).
    Triggers(TransitionId),
    /// The row walk's tables carry no candidate rows or carry selection rows
    /// beside them, the general walk's carry candidate rows, or the rows do
    /// not have one entry for each state and one past the last, rising from
    /// the first cell to the last (docs/adr/0135).
    Candidates,
    /// A state's cells break a rule of the candidate rows
    /// ([`DocumentTables::check`]).
    CandidateCell(StateId),
}

impl DocumentTables<'_> {
    /// The row of `state`.
    ///
    /// The procedures read a row at almost every step, and each read is
    /// inlined where it is made. An id past the table is a generator fault
    /// that [`check`](Self::check) refuses, so every read stops at one shared
    /// cold call that takes no argument (`row_missing`) rather than at a
    /// bounds check of its own that loads the index, the length and a location
    /// for the formatting machinery: the read is a compare and a branch.
    #[inline]
    pub fn state(&self, state: StateId) -> &StateRow {
        match self.states.get(state.index()) {
            Some(row) => row,
            None => row_missing(),
        }
    }

    /// The row of `history`; see [`state`](Self::state) for how a missing
    /// one stops.
    #[inline]
    pub fn history(&self, history: HistoryId) -> &HistoryRow {
        match self.histories.get(history.index()) {
            Some(row) => row,
            None => row_missing(),
        }
    }

    /// The row of `transition`; see [`state`](Self::state) for how a missing
    /// one stops.
    #[inline]
    pub fn transition(&self, transition: TransitionId) -> &TransitionRow {
        match self.transitions.get(transition.index()) {
            Some(row) => row,
            None => row_missing(),
        }
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
            Some(position) => listed(self.targets, position, |count| count.0),
            None => &[],
        }
    }

    /// `transition`'s entry path (docs/adr/0125): the states it enters, in entry
    /// order, each with whether it is entered by default. Empty for a
    /// transition with no target, and for every transition of tables whose walk
    /// reads no rows.
    ///
    /// Out of line, for the reason [`targets`](Self::targets) is: the row walk
    /// asks it as it exits and as it enters.
    #[inline(never)]
    pub fn entry_path(&self, transition: TransitionId) -> &[EntryStep] {
        let Some(position) = self
            .entry_rows
            .get(transition.index())
            .and_then(|row| row.position())
        else {
            return &[];
        };
        listed(self.entry_paths, position, |count| count.0)
    }

    /// A written transition as the emitted transition table names its row: its
    /// source, and its position among the source's own transitions in document
    /// order (the row's `index`, docs/SCE_TRANSITION_TABLE.md).
    ///
    /// The written transitions are the first rows, grouped by source in state
    /// order and within a source in the order they are written (the module
    /// documentation), so the position is the transition's number less the
    /// number of the first row with the same source, and no table of first rows
    /// is carried for it. `None` for a row with no source, which is the
    /// document's initial transition. Asked of a transition the selection
    /// returned, which is always a written one; for a compound state's initial
    /// transition or a history's default the answer means nothing.
    pub fn position_in_source(&self, transition: TransitionId) -> Option<(StateId, u32)> {
        let source = self.transition(transition).source()?;
        let first = self
            .transitions
            .get(..transition.index())?
            .iter()
            .rposition(|row| row.source() != Some(source))
            .map_or(0, |other| other + 1);
        Some((source, (transition.index() - first) as u32))
    }

    /// Appendix D's selectTransitions, the part one state answers: the first of
    /// `state`'s own transitions, in the order they are written, that `event`
    /// enables and whose guard holds. The tables'
    /// [`null_event`](Self::null_event) asks for eventless transitions.
    ///
    /// Tables that carry [`selection`](Self::selection) rows answer it here,
    /// for every machine at once (docs/adr/0127): the state's written
    /// transitions are a range, each one's events a short list of numbers, and
    /// `machine` is asked [`Behaviour::guard_holds`] only for a transition
    /// `event` enables, in the order they are written, until one holds. So a
    /// guard runs exactly where the machine's own selection would have run it,
    /// and a transition that `event` does not enable runs none. Tables written
    /// by hand carry no rows, and the machine answers
    /// ([`Behaviour::first_enabled_transition`]).
    pub fn first_enabled(
        &self,
        state: StateId,
        event: EventId,
        machine: &mut dyn Behaviour,
    ) -> Option<TransitionId> {
        let Some(selection) = &self.selection else {
            return machine.first_enabled_transition(state, event);
        };
        selection
            .written(state)
            .map(TransitionId)
            .find(|&transition| {
                selection.enables(transition, event) && machine.guard_holds(transition)
            })
    }

    /// The transitions the row walk's selection tries for `event` in a
    /// configuration whose atomic state is `atomic`, in the order it tries
    /// them, read from the [`candidates`](Self::candidates) rows
    /// (docs/adr/0135). Tables that carry none are not the row walk's, and
    /// stop as a row read past its table does.
    #[inline]
    pub fn candidates_of(&self, atomic: StateId, event: EventId) -> &[Candidate] {
        match &self.candidates {
            Some(rows) => rows.candidates_of(atomic, event),
            None => row_missing(),
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
    /// names a row of its table, the procedures of each feature the
    /// document uses carried, for the row walk one entry row for each
    /// transition that keeps the rules of an entry path (docs/adr/0125), and
    /// selection rows whose ranges are each state's written transitions and
    /// whose lists keep the rules of [`SelectionRows`] (docs/adr/0127), and
    /// for the row walk candidate rows that keep the rules of
    /// [`CandidateRows`] (docs/adr/0135). The
    /// procedures assume all of them; a debug build checks a machine's tables
    /// when it is built.
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
                let child_row = self.state(child);
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
        let ascending = self
            .error_events
            .windows(2)
            .all(|pair| matches!(pair, [low, high] if low < high));
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
        self.check_entry_rows(has_parallel)?;
        self.check_candidates()?;
        self.check_selection()
    }

    /// The part of [`check`](Self::check) the candidate rows answer
    /// (docs/adr/0135): the row walk's tables carry them and no selection rows,
    /// and the general walk's carry none; one entry for each state and one past
    /// the last, rising from the first cell to the last; cells only for an
    /// atomic state, in ascending order of the event and none past the null
    /// event; and each cell's list inside the pool, never empty, of written
    /// transitions of the state or of one of its ancestors, in the order the
    /// selection tries them: a state's own before an ancestor's, and one
    /// state's in the order they are written.
    fn check_candidates(&self) -> Result<(), TablesFault> {
        let Some(rows) = &self.candidates else {
            if self.walk.reads_rows() {
                return Err(TablesFault::Candidates);
            }
            return Ok(());
        };
        if !self.walk.reads_rows() || self.selection.is_some() {
            return Err(TablesFault::Candidates);
        }
        if rows.cells.len() > MAX_IDS || rows.candidates.len() > MAX_IDS {
            return Err(TablesFault::TooMany);
        }
        let rising = rows
            .first_cell
            .windows(2)
            .all(|pair| matches!(pair, [low, high] if low <= high));
        if rows.first_cell.len() != self.states.len() + 1
            || rows.first_cell.first() != Some(&0)
            || rows.first_cell.last().map(|end| usize::from(*end)) != Some(rows.cells.len())
            || !rising
        {
            return Err(TablesFault::Candidates);
        }
        for (index, range) in rows.first_cell.windows(2).enumerate() {
            let state = StateId(index as u16);
            let cells = match rows.cells.get(usize::from(range[0])..usize::from(range[1])) {
                Some(cells) => cells,
                None => return Err(TablesFault::Candidates),
            };
            let ascending = cells
                .windows(2)
                .all(|pair| pair[0].event() < pair[1].event());
            if (!cells.is_empty() && !self.is_atomic(state)) || !ascending {
                return Err(TablesFault::CandidateCell(state));
            }
            for cell in cells {
                if cell.event() > self.null_event || !self.candidates_hold(state, rows, *cell) {
                    return Err(TablesFault::CandidateCell(state));
                }
            }
        }
        Ok(())
    }

    /// Whether `cell`, a cell of the atomic state `atomic`, names a list
    /// inside the pool that is not empty and holds written transitions of
    /// `atomic` or of its ancestors in the order the selection tries them
    /// ([`check`](Self::check)).
    fn candidates_hold(
        &self,
        atomic: StateId,
        rows: &CandidateRows<'_>,
        cell: CandidateCell,
    ) -> bool {
        let position = cell.position();
        let Some(list) = rows.candidates.get(position).and_then(|len| {
            rows.candidates
                .get(position + 1..position + 1 + usize::from(len.0))
        }) else {
            return false;
        };
        if list.is_empty() {
            return false;
        }
        let mut previous: Option<(StateId, TransitionId)> = None;
        for candidate in list {
            let transition = candidate.transition();
            let Some(row) = self.transitions.get(transition.index()) else {
                return false;
            };
            // A written transition has a source, and is not that source's
            // initial transition; the class has no `<history>`, so no default.
            let Some(source) = row.source() else {
                return false;
            };
            let on_the_chain = source == atomic || self.is_descendant(atomic, source);
            if self.state(source).initial() == Some(transition) || !on_the_chain {
                return false;
            }
            // An ancestor is numbered before its descendants, so the order of
            // the selection is falling sources, and within one source rising
            // transitions.
            let ordered = match previous {
                None => true,
                Some((before, earlier)) => {
                    before > source || (before == source && earlier < transition)
                }
            };
            if !ordered {
                return false;
            }
            previous = Some((source, transition));
        }
        true
    }

    /// The part of [`check`](Self::check) the selection rows answer
    /// (docs/adr/0127): one entry for each state and one past the last, which
    /// start at the first row and never go back, so the ranges cover the
    /// written transitions once, each in the range of its own source; one row
    /// for each transition, every list inside the pool; and a list only for a
    /// written transition, never empty, its events ascending, none past the
    /// null event, and the null event only alone, which is an eventless
    /// transition's list.
    fn check_selection(&self) -> Result<(), TablesFault> {
        let Some(selection) = &self.selection else {
            return Ok(());
        };
        if selection.trigger_events.len() > MAX_IDS {
            return Err(TablesFault::TooMany);
        }
        if selection.first_written.len() != self.states.len() + 1
            || selection.triggers.len() != self.transitions.len()
            || selection.first_written.first() != Some(&TransitionId(0))
        {
            return Err(TablesFault::Selection);
        }
        for (index, range) in selection.first_written.windows(2).enumerate() {
            let state = StateId(index as u16);
            if range[0] > range[1] || range[1].index() > self.transitions.len() {
                return Err(TablesFault::Selection);
            }
            let own = self.transitions[range[0].index()..range[1].index()]
                .iter()
                .all(|row| row.source() == Some(state));
            if !own {
                return Err(TablesFault::Selection);
            }
        }
        // Past the last state's range are the rows no selection returns: the
        // initial transitions, the history defaults and the document's own.
        let written = selection.first_written.last().map_or(0, |end| end.index());
        for (index, row) in selection.triggers.iter().enumerate() {
            let transition = TransitionId(index as u16);
            let Some(position) = row.position() else {
                continue;
            };
            let list = selection.trigger_events.get(position).and_then(|len| {
                selection
                    .trigger_events
                    .get(position + 1..position + 1 + usize::from(len.0))
            });
            let holds = list.is_some_and(|list| {
                let ascending = list
                    .windows(2)
                    .all(|pair| pair[0].event() < pair[1].event());
                let numbered = list.iter().all(|entry| entry.event() <= self.null_event);
                let eventless_alone =
                    list.len() == 1 || list.iter().all(|entry| entry.event() != self.null_event);
                !list.is_empty() && ascending && numbered && eventless_alone
            });
            if index >= written || !holds {
                return Err(TablesFault::Triggers(transition));
            }
        }
        Ok(())
    }

    /// The part of [`check`](Self::check) the entry rows answer
    /// (docs/adr/0125): tables state rows exactly when their walk reads them,
    /// and the row walk only for a document whose sets the rows fix.
    fn check_entry_rows(&self, has_parallel: bool) -> Result<(), TablesFault> {
        if !self.walk.reads_rows() {
            if self.entry_rows.is_empty() && self.entry_paths.is_empty() {
                return Ok(());
            }
            return Err(TablesFault::EntryRows);
        }
        // A `<history>` makes what a transition enters depend on what it
        // recorded, a `<parallel>` makes a configuration more than one path
        // down the tree, and a transition of several targets enters more than
        // one: none of their sets is one row.
        let one_target = (0..self.transitions.len())
            .all(|index| self.targets(TransitionId(index as u16)).len() <= 1);
        if !self.histories.is_empty() || has_parallel || !one_target {
            return Err(TablesFault::RowWalkOutsideClass);
        }
        if self.entry_rows.len() != self.transitions.len() {
            return Err(TablesFault::EntryRows);
        }
        for (index, row) in self.entry_rows.iter().enumerate() {
            let transition = TransitionId(index as u16);
            let fits = match row.position() {
                Some(position) => self
                    .entry_paths
                    .get(position)
                    .is_some_and(|len| position + 1 + usize::from(len.0) <= self.entry_paths.len()),
                None => true,
            };
            if !fits || !self.entry_path_holds(transition) {
                return Err(TablesFault::EntryRow(transition));
            }
        }
        Ok(())
    }

    /// Whether `transition`'s entry path is one the row walk can take for it
    /// (docs/adr/0125). None for a compound state's initial transition, which
    /// no microstep takes (what it enters is part of the path of whatever
    /// enters its state), and none for a transition with no target. For one
    /// with a target, a path down the tree, each state a child of the one before,
    /// that holds the target and ends in an atomic state; that enters by
    /// default no state above the target, the target exactly when it is a
    /// compound state, and below it only compound states; and whose first
    /// state's parent, the transition's domain, below which the row walk exits
    /// the active states, is the source or one of its ancestors.
    fn entry_path_holds(&self, transition: TransitionId) -> bool {
        let path = self.entry_path(transition);
        if self
            .states
            .iter()
            .any(|row| row.initial() == Some(transition))
        {
            return path.is_empty();
        }
        let target = match self.targets(transition).first() {
            None => return path.is_empty(),
            Some(target) => match target.named() {
                Named::State(state) => state,
                Named::History(_) => return false,
            },
        };
        if path
            .iter()
            .any(|step| step.state().index() >= self.states.len())
        {
            return false;
        }
        let Some((first, rest)) = path.split_first() else {
            return false;
        };
        let mut last = first.state();
        for step in rest {
            if self.parent(step.state()) != Some(last) {
                return false;
            }
            last = step.state();
        }
        let Some(at_target) = path.iter().position(|step| step.state() == target) else {
            return false;
        };
        let defaults_hold = path.iter().enumerate().all(|(index, step)| {
            let compound = self.is_compound(step.state());
            match index.cmp(&at_target) {
                core::cmp::Ordering::Less => !step.by_default(),
                core::cmp::Ordering::Equal => step.by_default() == compound,
                core::cmp::Ordering::Greater => !step.by_default() || compound,
            }
        });
        let domain_holds = match (
            self.parent(first.state()),
            self.transition(transition).source(),
        ) {
            (None, _) => true,
            (Some(domain), Some(source)) => domain == source || self.is_descendant(source, domain),
            (Some(_), None) => false,
        };
        self.is_atomic(last) && defaults_hold && domain_holds
    }
}

/// The list at `position` of a pool whose lists are each their length
/// (`count` reads it off the first entry) followed by their entries: the
/// targets of a transition with more than one, an entry path. A list that runs
/// past the pool is a generator fault [`DocumentTables::check`] refuses, and
/// stops at [`row_missing`] as a row read past its table does; the pool is
/// read through `get`, so no bounds check of its own formats an index.
#[inline]
fn listed<T>(pool: &[T], position: usize, count: impl FnOnce(&T) -> u16) -> &[T] {
    let first = position + 1;
    match pool
        .get(position)
        .and_then(|head| pool.get(first..first + usize::from(count(head))))
    {
        Some(list) => list,
        None => row_missing(),
    }
}

/// A row read past the end of its table: the generator wrote an id the tables
/// do not hold, which [`DocumentTables::check`] refuses. No formatted message
/// and no argument, so every read of a row shares this one call and a
/// bare-metal image does not carry the formatting machinery for a branch a
/// correct generator never reaches. The policy's reads of its own values by
/// number ([`StatePolicy::state_of`](crate::StatePolicy::state_of) and its
/// kin) stop here too.
///
/// Public, and hidden from the documentation, because a generated policy's
/// reads of its values by number stop here as the runtime's do.
#[doc(hidden)]
#[cold]
#[inline(never)]
pub fn row_missing() -> ! {
    panic!("a document's tables name a row they do not hold: sce-codegen wrote them wrong")
}

/// The child states of one state, in document order: [`DocumentTables::children`].
#[derive(Debug, Clone)]
pub struct Children<'t> {
    tables: &'t DocumentTables<'t>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A list of the pool is its length followed by its entries, read where a
    /// row names it, and the length-zero list is empty.
    #[test]
    fn a_listed_pool_entry_is_its_length_and_the_entries_after_it() {
        let pool = [
            EntryStep::count(2),
            EntryStep::new(StateId(1), false),
            EntryStep::new(StateId(2), true),
            EntryStep::count(0),
        ];
        let first = listed(&pool, 0, |count| count.0);
        assert_eq!(first, &pool[1..3]);
        assert_eq!(first[1].state(), StateId(2));
        assert!(first[1].by_default());
        assert!(listed(&pool, 3, |count| count.0).is_empty());
    }

    /// A list whose length runs past the pool stops at the one cold call a row
    /// read past its table stops at, not at a bounds check of its own.
    #[test]
    #[should_panic(expected = "a document's tables name a row they do not hold")]
    fn a_list_past_the_pool_stops_at_the_row_read() {
        let pool = [EntryStep::count(3), EntryStep::new(StateId(1), false)];
        listed(&pool, 0, |count| count.0);
    }

    /// A list named past the end of the pool stops in the same place.
    #[test]
    #[should_panic(expected = "a document's tables name a row they do not hold")]
    fn a_list_named_past_the_pool_stops_at_the_row_read() {
        let pool = [EntryStep::count(0)];
        listed(&pool, 1, |count| count.0);
    }
}
