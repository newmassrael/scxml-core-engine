// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document's state tree as the tables the Rust runtime's one Appendix D
//! core reads (ADR 0064).
//!
//! The core is compiled once for a program and runs every machine in it, so it
//! names no type of any machine: a state, a `<history>` and a transition reach
//! it as numbers, and the tree as rows of plain data indexed by them. This is
//! the one function from the model to those rows, beside
//! [`MicrostepBounds::of`] and read from the same model the generator lowers,
//! so every template that emits the tables emits the same rows.
//!
//! The numbering is the contract the core relies on:
//!
//! - a state's number (`StateId`) is its position in document order, the order
//!   the parser ranks `<state>`, `<parallel>` and `<final>` elements in
//!   ([`State::document_order`]). Document order is a pre-order walk of the
//!   tree, so a parent's number is below each of its children's, and a state's
//!   subtree is the contiguous range from its own number to
//!   [`StateRow::last_descendant`]. Its children are therefore not stored: the
//!   first is the number after its own, and each further one the number after
//!   the subtree of the one before. Entry order is ascending order and exit
//!   order descending. A model numbered otherwise is refused rather than
//!   tabled, since a core reading its rows would answer wrongly in silence;
//! - a `<history>`'s number (`HistoryId`) is its position among the
//!   document's histories in the order the generated `History` enum declares
//!   them, which is their ids' order: the model ranks no `<history>` in
//!   document order, because a history is not a state;
//! - a transition's number (`TransitionId`) counts first every written
//!   transition, grouped by source state in `StateId` order and within a state
//!   in the order it is written there (the generator's `transition_index`),
//!   then one initial transition for each compound state in `StateId` order,
//!   then the default transition of each `<history>` in `HistoryId` order, and
//!   last the document's own initial transition. A written transition's number
//!   less the number of its source's first is its index in that source;
//! - a target is as written: a token naming a `<history>` stays a history, for
//!   the entry procedures to dereference at run time.
//!
//! Every number is a `u16`. The runtime keeps the top bit of a target for
//! whether it names a history and the largest `u16` for an absent state or
//! transition, so a document with more than [`MAX_IDS`] states, histories or
//! transitions, or more than [`MAX_TARGETS`] targets, is refused, and the
//! refusal names the table.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use crate::microstep_bounds::MicrostepBounds;
use crate::model::{HistoryInfo, SCXMLModel, State};

/// The most states, histories or transitions one table numbers: every id
/// below `1 << 15`, since a target keeps its top bit for whether it names a
/// state or a history, and no id reaches the largest `u16`, which a row holds
/// where a state or a transition is absent.
pub const MAX_IDS: usize = 1 << 15;

/// The most targets the pool holds: a transition names its targets as a
/// [`Span`] of two `u16`s.
pub const MAX_TARGETS: usize = u16::MAX as usize;

/// A document's tables: every row the core reads, indexed by the numbers
/// the module documentation defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTables {
    /// One row per state, indexed by `StateId`.
    pub states: Vec<StateRow>,
    /// One row per `<history>`, indexed by `HistoryId`.
    pub histories: Vec<HistoryRow>,
    /// One row per transition, indexed by `TransitionId`.
    pub transitions: Vec<TransitionRow>,
    /// The pool of targets that [`TransitionRow::targets`] spans into, laid out
    /// transition by transition in `TransitionId` order.
    pub targets: Vec<TargetRef>,
    /// The `TransitionId` of the document's own initial transition.
    pub initial: u16,
    /// The bounds of the microstep's buffers, carried with the rows whose
    /// procedures they size. The tables themselves hold none of them: the
    /// generator writes them as the sizes of the machine's workspace and
    /// configuration types.
    pub bounds: MicrostepBounds,
}

/// A slice of a pool: `len` entries from `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u16,
    pub len: u16,
}

impl Span {
    /// The entries of the pool this span covers.
    pub fn range(self) -> Range<usize> {
        let start = usize::from(self.start);
        start..start + usize::from(self.len)
    }
}

/// What kind of state a row is, as the generator decides it today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    /// A `<state>` with no child states.
    Atomic,
    /// A `<state>` with child states: the states that have an initial
    /// transition.
    Compound,
    /// A `<parallel>`.
    Parallel,
    /// A `<final>`.
    Final,
}

/// One state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRow {
    /// The state's id, as written or as the parser generated it.
    pub id: String,
    /// The `StateId` of the state it is declared in; `None` for a child of the
    /// `<scxml>` element. Always below this row's own number.
    pub parent: Option<u16>,
    pub kind: StateKind,
    /// The `TransitionId` of its initial transition: `Some` exactly for a
    /// [`StateKind::Compound`] state.
    pub initial: Option<u16>,
    /// The largest `StateId` in its subtree, its own when it has no child
    /// states: a state `s` is a proper descendant of this one exactly when
    /// this row's number is below `s` and `s` is at most this. Its child
    /// states, which no row lists, follow from it: the first is this row's
    /// number plus one, and each further one is one past the
    /// `last_descendant` of the one before, up to this. A `<history>` is not
    /// among them.
    pub last_descendant: u16,
}

/// One `<history>`. What it records is kept by the engine as one set of the
/// document's states, as every set the core keeps is, so its row says nothing
/// of how many states that can be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    /// The `<history>`'s id.
    pub id: String,
    /// The `StateId` of the state it is declared in.
    pub parent: u16,
    /// Whether it records the deep configuration (`type="deep"`) rather
    /// than its parent's active children.
    pub deep: bool,
    /// The `TransitionId` of its default transition.
    pub default: u16,
}

/// One transition: a written `<transition>`, a compound state's initial
/// transition, a `<history>`'s default transition, or the document's own
/// initial transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRow {
    /// The `StateId` the transition leaves: the state it is written in, the
    /// compound state an initial transition belongs to, or the state a
    /// `<history>` is declared in. `None` for the document's own initial
    /// transition, which the `<scxml>` element takes.
    pub source: Option<u16>,
    pub origin: TransitionOrigin,
    /// Its targets as written, as a span of [`DocumentTables::targets`];
    /// empty for a targetless transition.
    pub targets: Span,
    /// Whether it is written with `type="internal"` and the analyzer kept it
    /// so; false for every transition that is not written.
    pub internal: bool,
    /// Whether it carries executable content: that of the `<transition>`, of
    /// the `<initial>` element's transition, or of the `<history>`'s default
    /// transition. False for the document's own initial transition, which has
    /// none.
    pub has_content: bool,
}

/// Where a transition row comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionOrigin {
    /// A `<transition>`, the `index_in_source`-th written in its source.
    Written { index_in_source: u16 },
    /// A compound state's initial transition: its `<initial>` element's, its
    /// `initial` attribute's, or its first child's when it names none.
    Initial,
    /// The default transition of the `<history>` of this `HistoryId`.
    HistoryDefault(u16),
    /// The document's own initial transition.
    Document,
}

/// One target of a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetRef {
    /// A state, by `StateId`.
    State(u16),
    /// A `<history>`, by `HistoryId`.
    History(u16),
}

/// The table a refusal is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    States,
    Histories,
    Transitions,
    Targets,
}

impl Table {
    /// The most rows the table holds: [`MAX_IDS`] for a table whose rows
    /// are ids, [`MAX_TARGETS`] for the pool of targets.
    pub const fn max_rows(self) -> usize {
        match self {
            Table::States | Table::Histories | Table::Transitions => MAX_IDS,
            Table::Targets => MAX_TARGETS,
        }
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Table::States => "states",
            Table::Histories => "<history> elements",
            Table::Transitions => "transitions (written, initial and history default)",
            Table::Targets => "transition targets",
        })
    }
}

/// Why a model has no tables.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TablesError {
    /// A table would hold more rows than its numbers count.
    #[error(
        "the document has {count} {table}, and a table numbers at most {max} of them \
         (ADR 0064)",
        max = .table.max_rows()
    )]
    TooMany { table: Table, count: usize },
    /// A reference names neither a state nor a `<history>` of the document.
    #[error("{referrer} names `{id}`, which is no state or <history> of the document")]
    Unknown { referrer: String, id: String },
    /// The model's document order is not a pre-order walk of its tree at this
    /// state: it follows one of its descendants, or its subtree is not one
    /// contiguous run of the order.
    #[error(
        "state `{state}` is not numbered in document order: it precedes its parent, or its \
         subtree is not the run of states that follows it"
    )]
    NotDocumentOrder { state: String },
}

/// `count` as a `u16` when `table` holds that many rows, or the refusal that
/// names `table`.
fn fits(count: usize, table: Table) -> Result<u16, TablesError> {
    if count > table.max_rows() {
        return Err(TablesError::TooMany { table, count });
    }
    u16::try_from(count).map_err(|_| TablesError::TooMany { table, count })
}

/// The span of a pool from `start` to `end`, both of which must be numbers of
/// `table`.
fn span(start: usize, end: usize, table: Table) -> Result<Span, TablesError> {
    fits(end, table)?;
    Ok(Span {
        start: fits(start, table)?,
        len: fits(end - start, table)?,
    })
}

/// The numbers of a document's states and histories, by id.
struct Names<'m> {
    states: BTreeMap<&'m str, u16>,
    histories: BTreeMap<&'m str, u16>,
}

impl Names<'_> {
    fn state(&self, id: &str, referrer: impl FnOnce() -> String) -> Result<u16, TablesError> {
        self.states
            .get(id)
            .copied()
            .ok_or_else(|| TablesError::Unknown {
                referrer: referrer(),
                id: id.to_string(),
            })
    }

    fn target(&self, id: &str, referrer: &dyn Fn() -> String) -> Result<TargetRef, TablesError> {
        if let Some(state) = self.states.get(id) {
            return Ok(TargetRef::State(*state));
        }
        if let Some(history) = self.histories.get(id) {
            return Ok(TargetRef::History(*history));
        }
        Err(TablesError::Unknown {
            referrer: referrer(),
            id: id.to_string(),
        })
    }

    /// Append `tokens` to the target pool and return their span.
    fn push_targets(
        &self,
        pool: &mut Vec<TargetRef>,
        tokens: &[String],
        referrer: &dyn Fn() -> String,
    ) -> Result<Span, TablesError> {
        let start = pool.len();
        for token in tokens {
            pool.push(self.target(token, referrer)?);
        }
        span(start, pool.len(), Table::Targets)
    }
}

/// The kind of `state`, from the facts the generator emits
/// `is_final_state`, `is_parallel_state` and `is_compound_state` from: a
/// `<state>` is compound when it has an initial transition, which the parser
/// gives exactly the `<state>`s that have child states.
fn kind_of(state: &State) -> StateKind {
    if state.is_parallel {
        StateKind::Parallel
    } else if state.is_final {
        StateKind::Final
    } else if !state.initial_targets.is_empty() {
        StateKind::Compound
    } else {
        StateKind::Atomic
    }
}

impl DocumentTables {
    /// The tables of `model`'s state tree.
    pub fn of(model: &SCXMLModel) -> Result<Self, TablesError> {
        // The order the generator sorts states in. The sort is stable over the
        // map's id order, which settles a tie only in a model no parser built.
        let mut ordered: Vec<(&str, &State)> = model
            .states
            .iter()
            .map(|(id, state)| (id.as_str(), state))
            .collect();
        ordered.sort_by_key(|(_, state)| state.document_order);
        fits(ordered.len(), Table::States)?;
        // The order the generated `History` enum declares its variants in.
        let histories: Vec<(&str, &HistoryInfo)> = model
            .history_states
            .iter()
            .map(|(id, info)| (id.as_str(), info))
            .collect();
        fits(histories.len(), Table::Histories)?;

        let names = Names {
            states: ordered.iter().map(|(id, _)| *id).zip(0u16..).collect(),
            histories: histories.iter().map(|(id, _)| *id).zip(0u16..).collect(),
        };

        let compound = ordered
            .iter()
            .filter(|(_, state)| kind_of(state) == StateKind::Compound)
            .count();
        let written: usize = ordered.iter().map(|(_, s)| s.transitions.len()).sum();
        fits(written + compound + histories.len() + 1, Table::Transitions)?;

        // The states, each with its parent.
        let mut states: Vec<StateRow> = Vec::with_capacity(ordered.len());
        for ((id, state), number) in ordered.iter().zip(0u16..) {
            let parent = match state.parent.as_deref() {
                Some(parent) => {
                    let parent = names.state(parent, || format!("the parent of state `{id}`"))?;
                    if parent >= number {
                        return Err(TablesError::NotDocumentOrder {
                            state: (*id).to_string(),
                        });
                    }
                    Some(parent)
                }
                None => None,
            };
            states.push(StateRow {
                id: (*id).to_string(),
                parent,
                kind: kind_of(state),
                initial: None,
                last_descendant: number,
            });
        }

        // A subtree is the run of numbers from its root to its last
        // descendant only if it holds as many states as that run does, and
        // only then are its children where the runtime looks for them. Every
        // child is numbered above its parent (checked above), so a walk from
        // the last state back meets each child before its parent.
        let mut subtree: Vec<usize> = vec![1; states.len()];
        let parents: Vec<Option<u16>> = states.iter().map(|row| row.parent).collect();
        for (index, parent) in parents.iter().enumerate().rev() {
            if let Some(parent) = parent {
                let parent = usize::from(*parent);
                let last = states[index].last_descendant;
                states[parent].last_descendant = states[parent].last_descendant.max(last);
                subtree[parent] += subtree[index];
            }
        }
        for (index, (row, size)) in states.iter().zip(&subtree).enumerate() {
            if usize::from(row.last_descendant) + 1 != index + size {
                return Err(TablesError::NotDocumentOrder {
                    state: row.id.clone(),
                });
            }
        }

        let mut transitions: Vec<TransitionRow> = Vec::new();
        let mut targets: Vec<TargetRef> = Vec::new();

        // Every written transition, by source in `StateId` order.
        for ((id, state), number) in ordered.iter().zip(0u16..) {
            for (index, transition) in state.transitions.iter().enumerate() {
                let referrer = || format!("transition {index} of state `{id}`");
                transitions.push(TransitionRow {
                    source: Some(number),
                    origin: TransitionOrigin::Written {
                        index_in_source: fits(index, Table::Transitions)?,
                    },
                    targets: names.push_targets(&mut targets, &transition.targets, &referrer)?,
                    internal: transition.transition_type == "internal",
                    has_content: !transition.actions.is_empty(),
                });
            }
        }

        // One initial transition for each compound state.
        for ((id, state), number) in ordered.iter().zip(0u16..) {
            if kind_of(state) != StateKind::Compound {
                continue;
            }
            let referrer = || format!("the initial transition of state `{id}`");
            states[usize::from(number)].initial =
                Some(fits(transitions.len(), Table::Transitions)?);
            transitions.push(TransitionRow {
                source: Some(number),
                origin: TransitionOrigin::Initial,
                targets: names.push_targets(&mut targets, &state.initial_targets, &referrer)?,
                internal: false,
                has_content: !state.initial_transition_actions.is_empty(),
            });
        }

        // One default transition for each `<history>`.
        let mut history_rows: Vec<HistoryRow> = Vec::with_capacity(histories.len());
        for ((id, info), number) in histories.iter().zip(0u16..) {
            let parent = names.state(&info.parent, || format!("the parent of <history> `{id}`"))?;
            let default = fits(transitions.len(), Table::Transitions)?;
            let referrer = || format!("the default transition of <history> `{id}`");
            transitions.push(TransitionRow {
                source: Some(parent),
                origin: TransitionOrigin::HistoryDefault(number),
                targets: names.push_targets(&mut targets, &info.default_targets, &referrer)?,
                internal: false,
                has_content: !info.default_actions.is_empty(),
            });
            history_rows.push(HistoryRow {
                id: (*id).to_string(),
                parent,
                deep: info.history_type == "deep",
                default,
            });
        }

        // The document's own initial transition, last.
        let initial = fits(transitions.len(), Table::Transitions)?;
        let referrer = || "the document's initial transition".to_string();
        transitions.push(TransitionRow {
            source: None,
            origin: TransitionOrigin::Document,
            targets: names.push_targets(&mut targets, &model.initial_targets, &referrer)?,
            internal: false,
            has_content: false,
        });

        Ok(Self {
            states,
            histories: history_rows,
            transitions,
            targets,
            initial,
            bounds: MicrostepBounds::of(model),
        })
    }

    /// The `StateId` of the state `id`.
    pub fn state_id(&self, id: &str) -> Option<u16> {
        let index = self.states.iter().position(|row| row.id == id)?;
        u16::try_from(index).ok()
    }

    /// The `HistoryId` of the `<history>` `id`.
    pub fn history_id(&self, id: &str) -> Option<u16> {
        let index = self.histories.iter().position(|row| row.id == id)?;
        u16::try_from(index).ok()
    }

    /// The `TransitionId` of the `index_in_source`-th transition written in
    /// the state `source_state`.
    pub fn transition_id(&self, source_state: &str, index_in_source: usize) -> Option<u16> {
        let source = self.state_id(source_state)?;
        let index_in_source = u16::try_from(index_in_source).ok()?;
        let index = self.transitions.iter().position(|row| {
            row.source == Some(source)
                && row.origin == TransitionOrigin::Written { index_in_source }
        })?;
        u16::try_from(index).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Transition;
    use crate::parser::SCXMLParser;

    /// `body` as a document of the null data model, parsed and analysed as
    /// the generator receives it.
    fn tables_of(body: &str) -> (SCXMLModel, DocumentTables) {
        let text = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null"{body}</scxml>"#
        );
        let mut model = SCXMLParser::new()
            .parse_string(&text, "document_tables")
            .expect("parse");
        crate::analyzer::analyze(&mut model, "");
        let tables = DocumentTables::of(&model).expect("tables");
        (model, tables)
    }

    fn row(
        id: &str,
        parent: Option<u16>,
        kind: StateKind,
        initial: Option<u16>,
        last_descendant: u16,
    ) -> StateRow {
        StateRow {
            id: id.to_string(),
            parent,
            kind,
            initial,
            last_descendant,
        }
    }

    /// The child states of `state` as the runtime reads them from the rows:
    /// the next number, then each number past the subtree of the one before,
    /// up to the end of `state`'s subtree.
    fn children_of(tables: &DocumentTables, state: u16) -> Vec<u16> {
        let last = tables.states[usize::from(state)].last_descendant;
        let mut children = Vec::new();
        let mut next = state + 1;
        while next <= last {
            children.push(next);
            next = tables.states[usize::from(next)].last_descendant + 1;
        }
        children
    }

    fn transition(
        source: Option<u16>,
        origin: TransitionOrigin,
        targets: (u16, u16),
        internal: bool,
        has_content: bool,
    ) -> TransitionRow {
        TransitionRow {
            source,
            origin,
            targets: Span {
                start: targets.0,
                len: targets.1,
            },
            internal,
            has_content,
        }
    }

    fn written(index_in_source: u16) -> TransitionOrigin {
        TransitionOrigin::Written { index_in_source }
    }

    #[test]
    fn a_flat_document_numbers_its_states_in_the_order_they_are_written() {
        // Written idle, busy, done: their ids sort busy, done, idle, which is
        // not the order the numbers follow.
        let (model, tables) = tables_of(
            r#" initial="idle">
  <state id="idle"><transition event="go" target="busy"/></state>
  <state id="busy">
    <transition event="back" target="idle"><raise event="again"/></transition>
    <transition event="stay"/>
  </state>
  <final id="done"/>
"#,
        );
        use StateKind::*;
        use TargetRef::State as S;
        assert_eq!(
            tables,
            DocumentTables {
                states: vec![
                    row("idle", None, Atomic, None, 0),
                    row("busy", None, Atomic, None, 1),
                    row("done", None, Final, None, 2),
                ],
                histories: vec![],
                transitions: vec![
                    transition(Some(0), written(0), (0, 1), false, false),
                    transition(Some(1), written(0), (1, 1), false, true),
                    transition(Some(1), written(1), (2, 0), false, false),
                    transition(None, TransitionOrigin::Document, (2, 1), false, false),
                ],
                targets: vec![S(1), S(0), S(0)],
                initial: 3,
                bounds: MicrostepBounds::of(&model),
            }
        );
        assert_eq!(tables.state_id("busy"), Some(1));
        assert_eq!(tables.state_id("nowhere"), None);
        assert_eq!(tables.transition_id("busy", 1), Some(2));
        assert_eq!(tables.transition_id("busy", 2), None);
        assert_eq!(tables.transition_id("done", 0), None);
    }

    #[test]
    fn a_compound_state_has_its_initial_transition_after_every_written_one() {
        // `reset` is internal and targets a child, so the analyzer keeps it
        // internal; `leave` is written internal and targets a state outside,
        // so it is external, and the row says what the analyzer decided.
        let (model, tables) = tables_of(
            r#">
  <state id="outer">
    <initial><transition target="second"><raise event="entered"/></transition></initial>
    <transition event="reset" type="internal" target="first"/>
    <transition event="leave" type="internal" target="gone"/>
    <state id="first"><transition event="next" target="second"/></state>
    <state id="second"/>
  </state>
  <final id="gone"/>
"#,
        );
        use StateKind::*;
        use TargetRef::State as S;
        assert_eq!(
            tables,
            DocumentTables {
                states: vec![
                    row("outer", None, Compound, Some(3), 2),
                    row("first", Some(0), Atomic, None, 1),
                    row("second", Some(0), Atomic, None, 2),
                    row("gone", None, Final, None, 3),
                ],
                histories: vec![],
                transitions: vec![
                    transition(Some(0), written(0), (0, 1), true, false),
                    transition(Some(0), written(1), (1, 1), false, false),
                    transition(Some(1), written(0), (2, 1), false, false),
                    transition(Some(0), TransitionOrigin::Initial, (3, 1), false, true),
                    transition(None, TransitionOrigin::Document, (4, 1), false, false),
                ],
                targets: vec![S(1), S(3), S(2), S(2), S(0)],
                initial: 4,
                bounds: MicrostepBounds::of(&model),
            }
        );
        assert_eq!(tables.transition_id("outer", 1), Some(1));
        assert_eq!(tables.transition_id("first", 0), Some(2));
        assert_eq!(children_of(&tables, 0), [1u16, 2]);
        assert!(children_of(&tables, 3).is_empty());
    }

    #[test]
    fn histories_are_numbered_by_id_and_children_follow_from_the_subtrees() {
        // machine (compound, initial is the history `resume`)
        //   resume: deep history of machine, with content
        //   panel (parallel)
        //     regions: shallow history of panel
        //     everything: deep history of panel
        //     left > l1 l2
        //     right
        //   off, whose transition targets `resume`
        let (model, tables) = tables_of(
            r#" initial="machine">
  <state id="machine" initial="resume">
    <history id="resume" type="deep"><transition target="panel"><raise event="restored"/></transition></history>
    <parallel id="panel">
      <history id="regions" type="shallow"><transition target="left right"/></history>
      <history id="everything" type="deep"><transition target="l2 right"/></history>
      <state id="left">
        <state id="l1"><transition event="flip" target="l2"/></state>
        <state id="l2"/>
      </state>
      <state id="right"/>
    </parallel>
    <state id="off"><transition event="on" target="resume"/></state>
  </state>
"#,
        );
        use StateKind::*;
        use TargetRef::{History as H, State as S};
        // The largest configuration is machine, panel, left, a leaf of left
        // and right.
        assert_eq!(tables.bounds.configuration, 5);
        assert_eq!(
            tables,
            DocumentTables {
                states: vec![
                    row("machine", None, Compound, Some(2), 6),
                    row("panel", Some(0), Parallel, None, 5),
                    row("left", Some(1), Compound, Some(3), 4),
                    row("l1", Some(2), Atomic, None, 3),
                    row("l2", Some(2), Atomic, None, 4),
                    row("right", Some(1), Atomic, None, 5),
                    row("off", Some(0), Atomic, None, 6),
                ],
                histories: vec![
                    HistoryRow {
                        id: "everything".to_string(),
                        parent: 1,
                        deep: true,
                        default: 4,
                    },
                    HistoryRow {
                        id: "regions".to_string(),
                        parent: 1,
                        deep: false,
                        default: 5,
                    },
                    HistoryRow {
                        id: "resume".to_string(),
                        parent: 0,
                        deep: true,
                        default: 6,
                    },
                ],
                transitions: vec![
                    transition(Some(3), written(0), (0, 1), false, false),
                    transition(Some(6), written(0), (1, 1), false, false),
                    transition(Some(0), TransitionOrigin::Initial, (2, 1), false, false),
                    transition(Some(2), TransitionOrigin::Initial, (3, 1), false, false),
                    transition(
                        Some(1),
                        TransitionOrigin::HistoryDefault(0),
                        (4, 2),
                        false,
                        false
                    ),
                    transition(
                        Some(1),
                        TransitionOrigin::HistoryDefault(1),
                        (6, 2),
                        false,
                        false
                    ),
                    transition(
                        Some(0),
                        TransitionOrigin::HistoryDefault(2),
                        (8, 1),
                        false,
                        true
                    ),
                    transition(None, TransitionOrigin::Document, (9, 1), false, false),
                ],
                targets: vec![S(4), H(2), H(2), S(3), S(4), S(5), S(2), S(5), S(1), S(0),],
                initial: 7,
                bounds: MicrostepBounds::of(&model),
            }
        );
        assert_eq!(tables.history_id("resume"), Some(2));
        assert_eq!(tables.history_id("machine"), None);
        // No row lists a state's children: machine's are panel and, past
        // panel's subtree, off; panel's regions are left and, past left's
        // subtree, right; a history is none of them.
        for (state, children) in [(0u16, vec![1u16, 6]), (1, vec![2, 5]), (2, vec![3, 4])] {
            assert_eq!(children_of(&tables, state), children, "children of {state}");
            let id = &tables.states[usize::from(state)].id;
            let named: Vec<&str> = children
                .iter()
                .map(|child| tables.states[usize::from(*child)].id.as_str())
                .collect();
            assert_eq!(named, model.states[id].children, "children of `{id}`");
        }
        for leaf in [3, 4, 5, 6] {
            assert!(children_of(&tables, leaf).is_empty(), "{leaf} is a leaf");
        }
    }

    #[test]
    fn a_document_with_no_states_has_only_its_own_initial_transition() {
        let model = SCXMLModel::default();
        assert_eq!(
            DocumentTables::of(&model),
            Ok(DocumentTables {
                states: vec![],
                histories: vec![],
                transitions: vec![transition(
                    None,
                    TransitionOrigin::Document,
                    (0, 0),
                    false,
                    false
                )],
                targets: vec![],
                initial: 0,
                bounds: MicrostepBounds::of(&model),
            })
        );
    }

    /// A model of the given states, `(id, parent, document_order)`, built
    /// without a parser so that its numbering can be wrong.
    fn model_of(states: &[(&str, Option<&str>, u32)]) -> SCXMLModel {
        let mut model = SCXMLModel::default();
        for (id, parent, document_order) in states {
            model.states.insert(
                (*id).to_string(),
                State {
                    id: (*id).to_string(),
                    parent: parent.map(str::to_string),
                    document_order: *document_order,
                    ..State::default()
                },
            );
        }
        model
    }

    #[test]
    fn a_state_numbered_before_its_parent_is_refused() {
        let model = model_of(&[("a1", Some("a"), 0), ("a", None, 1)]);
        assert_eq!(
            DocumentTables::of(&model),
            Err(TablesError::NotDocumentOrder {
                state: "a1".to_string()
            })
        );
    }

    #[test]
    fn a_subtree_split_by_a_state_outside_it_is_refused() {
        // b sits between a and a's child a1.
        let model = model_of(&[("a", None, 0), ("b", None, 1), ("a1", Some("a"), 2)]);
        assert_eq!(
            DocumentTables::of(&model),
            Err(TablesError::NotDocumentOrder {
                state: "a".to_string()
            })
        );
    }

    #[test]
    fn a_reference_to_nothing_is_refused_with_who_made_it() {
        let mut model = model_of(&[("a", None, 0)]);
        model
            .states
            .get_mut("a")
            .expect("a")
            .transitions
            .push(Transition {
                targets: vec!["ghost".to_string()],
                ..Transition::default()
            });
        assert_eq!(
            DocumentTables::of(&model),
            Err(TablesError::Unknown {
                referrer: "transition 0 of state `a`".to_string(),
                id: "ghost".to_string()
            })
        );

        let orphan = model_of(&[("a", Some("nowhere"), 0)]);
        assert_eq!(
            DocumentTables::of(&orphan),
            Err(TablesError::Unknown {
                referrer: "the parent of state `a`".to_string(),
                id: "nowhere".to_string()
            })
        );

        let mut stray = model_of(&[("a", None, 0)]);
        stray.history_states.insert(
            "h".to_string(),
            HistoryInfo {
                parent: "gone".to_string(),
                history_type: "shallow".to_string(),
                ..HistoryInfo::default()
            },
        );
        assert_eq!(
            DocumentTables::of(&stray),
            Err(TablesError::Unknown {
                referrer: "the parent of <history> `h`".to_string(),
                id: "gone".to_string()
            })
        );
    }

    #[test]
    fn a_count_past_what_its_table_numbers_names_its_table() {
        for (table, max) in [
            (Table::States, MAX_IDS),
            (Table::Histories, MAX_IDS),
            (Table::Transitions, MAX_IDS),
            (Table::Targets, MAX_TARGETS),
        ] {
            assert_eq!(table.max_rows(), max, "{table}");
            assert_eq!(fits(max, table), Ok(max as u16), "{table}");
            assert_eq!(
                fits(max + 1, table),
                Err(TablesError::TooMany {
                    table,
                    count: max + 1
                })
            );
        }
        // Every id leaves a target's top bit clear, and none is the largest
        // `u16`, which stands for an absent state or transition.
        assert_eq!(MAX_IDS - 1, usize::from(u16::MAX >> 1));
        let message = TablesError::TooMany {
            table: Table::Transitions,
            count: 40_000,
        }
        .to_string();
        assert!(message.contains("40000 transitions"), "{message}");
        assert!(message.contains("32768"), "{message}");
        let message = TablesError::TooMany {
            table: Table::Targets,
            count: 70_000,
        }
        .to_string();
        assert!(message.contains("70000 transition targets"), "{message}");
        assert!(message.contains("65535"), "{message}");
    }

    #[test]
    fn a_document_of_more_states_than_ids_is_refused_for_its_states() {
        let mut model = SCXMLModel::default();
        for order in 0..=(MAX_IDS as u32) {
            let id = format!("s{order}");
            model.states.insert(
                id.clone(),
                State {
                    id,
                    document_order: order,
                    ..State::default()
                },
            );
        }
        assert_eq!(
            DocumentTables::of(&model),
            Err(TablesError::TooMany {
                table: Table::States,
                count: MAX_IDS + 1
            })
        );
    }
}
