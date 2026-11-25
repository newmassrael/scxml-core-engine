// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A corpus document's tables as the runtime's own `DocumentTables`, built
//! from one model the way the Rust generator builds them (ADR 0064, ADR 0125,
//! ADR 0127, ADR 0135), for the suites that ask the runtime's procedures about
//! real documents.
//!
//! The population those suites walk is the one the generator accepts: every
//! tracked `.scxml` that parses and that `analyzer::can_generate_static`
//! accepts, as the Rust backend lowers it ([`generatable`]). The rows are
//! packed as the generator packs them (docs/adr/0067): a transition holds its
//! only target itself, and every pool holds each list once or in row order,
//! its length first. Each pool is leaked, for the life of the test process,
//! since the runtime's rows borrow for `'static`.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use sce_build::document_tables::{DocumentTables, PathStep, Selection, StateKind, TargetRef};
use sce_build::generator::{rust_numbered_events, Language};
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;
use sce_rust_runtime::helpers::tables as rt;

/// The model the Rust backend renders `path` from, or `None` when the
/// document does not parse or the generator refuses it: the corpus carries
/// deliberate negative fixtures and forge documents, which the stages that
/// judge them report.
pub fn generatable(path: &Path) -> Option<SCXMLModel> {
    let label = path.to_str()?;
    let mut model = SCXMLParser::new().parse_file(label).ok()?;
    sce_build::analyzer::analyze(&mut model, label);
    sce_build::analyzer::can_generate_static(&model, label).ok()?;
    Some(sce_build::host_processor_analyzer::lower_mesh(&model, Language::Rust).into_owned())
}

/// The names a Rust machine of `model` answers for the events it numbers, in
/// the order it numbers them: what its `get_event_name` answers. The null
/// event is numbered after the last.
pub fn event_names(model: &SCXMLModel) -> Vec<String> {
    rust_numbered_events(model)
        .iter()
        .map(|event| sce_build::filters::event_wire_name((*event).to_string()))
        .collect()
}

/// What enables each of `tables`' transitions (ADR 0127), numbered by
/// `names`, as the generator computes it.
pub fn selection(tables: &DocumentTables, model: &SCXMLModel, names: &[String]) -> Selection {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    tables
        .selection(model, &names)
        .unwrap_or_else(|refusal| panic!("no selection rows: {refusal}"))
}

/// `tables` as the runtime's `DocumentTables` of the general walk, which
/// serves every document: its rows, `selection` as its selection rows when
/// given, and `null_event` as the null event. Without selection rows the
/// tables answer no selection, and serve what asks only the tree.
pub fn general(
    tables: &DocumentTables,
    selection: Option<&Selection>,
    null_event: u16,
) -> rt::DocumentTables<'static> {
    let states: Vec<rt::StateRow> = tables
        .states
        .iter()
        .map(|row| {
            let kind = match row.kind {
                StateKind::Atomic => rt::StateKind::Atomic,
                StateKind::Compound => rt::StateKind::Compound,
                StateKind::Parallel => rt::StateKind::Parallel,
                StateKind::Final => rt::StateKind::Final,
            };
            rt::StateRow::new(
                row.parent.map(rt::StateId),
                kind,
                rt::StateId(row.last_descendant),
                row.initial.map(rt::TransitionId),
            )
        })
        .collect();
    let histories: Vec<rt::HistoryRow> = tables
        .histories
        .iter()
        .map(|row| {
            rt::HistoryRow::new(
                rt::StateId(row.parent),
                rt::TransitionId(row.default),
                row.deep,
            )
        })
        .collect();
    let target_of = |target: &TargetRef| match *target {
        TargetRef::State(id) => rt::Target::state(rt::StateId(id)),
        TargetRef::History(id) => rt::Target::history(rt::HistoryId(id)),
    };
    let mut pool: Vec<rt::Target> = Vec::new();
    let transitions: Vec<rt::TransitionRow> = tables
        .transitions
        .iter()
        .map(|row| {
            let targets = match &tables.targets[row.targets.range()] {
                [] => rt::Target::NONE,
                [only] => target_of(only),
                list => {
                    let position = pool.len() as u16;
                    pool.push(rt::Target::count(list.len() as u16));
                    pool.extend(list.iter().map(target_of));
                    rt::Target::list(position)
                }
            };
            rt::TransitionRow::new(
                row.source.map(rt::StateId),
                targets,
                row.internal,
                row.has_content,
            )
        })
        .collect();
    rt::DocumentTables {
        states: states.leak(),
        histories: histories.leak(),
        transitions: transitions.leak(),
        targets: pool.leak(),
        initial: rt::TransitionId(tables.initial),
        // No engine names a state before it starts here, and the main loop's
        // error events are the template's (docs/adr/0068), not this model's.
        default_state: rt::StateId(0),
        null_event: rt::EventId(null_event),
        error_events: &[],
        walk: &sce_rust_runtime::GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        // The feature procedures exactly when the document uses the feature,
        // as the generator writes them (docs/adr/0066).
        history: tables.uses_history().then_some(&sce_rust_runtime::HISTORY),
        parallel: tables
            .uses_parallel()
            .then_some(&sce_rust_runtime::PARALLEL),
        selection: selection.map(selection_rows),
        candidates: None,
    }
}

/// `tables` as the runtime's `DocumentTables` of the row walk, for a document
/// that fixes its sets (ADR 0125): the rows of [`general`], `paths` (its
/// entry paths) as entry rows, and the candidate rows made from `selection`
/// (ADR 0135) in place of the selection rows.
pub fn row_walk(
    tables: &DocumentTables,
    paths: &[Vec<PathStep>],
    selection: &Selection,
    null_event: u16,
) -> rt::DocumentTables<'static> {
    let mut pool: Vec<rt::EntryStep> = Vec::new();
    let rows: Vec<rt::EntryRow> = paths
        .iter()
        .map(|path| {
            if path.is_empty() {
                return rt::EntryRow::NONE;
            }
            let position = pool.len() as u16;
            pool.push(rt::EntryStep::count(path.len() as u16));
            pool.extend(
                path.iter()
                    .map(|step| rt::EntryStep::new(rt::StateId(step.state), step.by_default)),
            );
            rt::EntryRow::at(position)
        })
        .collect();
    rt::DocumentTables {
        walk: &sce_rust_runtime::ROW_WALK,
        entry_rows: rows.leak(),
        entry_paths: pool.leak(),
        selection: None,
        candidates: Some(candidate_rows(tables, selection, null_event)),
        ..general(tables, None, null_event)
    }
}

/// `selection` as the runtime's `SelectionRows` (ADR 0127): each list once in
/// the pool, its length first.
pub fn selection_rows(selection: &Selection) -> rt::SelectionRows<'static> {
    let mut pool: Vec<rt::TriggerEvent> = Vec::new();
    let mut positions: BTreeMap<&[u16], u16> = BTreeMap::new();
    let triggers: Vec<rt::TriggerRow> = selection
        .triggers
        .iter()
        .map(|list| {
            if list.is_empty() {
                return rt::TriggerRow::NONE;
            }
            let position = *positions.entry(list.as_slice()).or_insert_with(|| {
                let position = pool.len() as u16;
                pool.push(rt::TriggerEvent::count(list.len() as u16));
                pool.extend(
                    list.iter()
                        .map(|&event| rt::TriggerEvent::new(rt::EventId(event))),
                );
                position
            });
            rt::TriggerRow::at(position)
        })
        .collect();
    let first_written: Vec<rt::TransitionId> = selection
        .first_written
        .iter()
        .map(|&number| rt::TransitionId(number))
        .collect();
    rt::SelectionRows {
        first_written: first_written.leak(),
        triggers: triggers.leak(),
        trigger_events: pool.leak(),
    }
}

/// The candidate rows of `tables` (ADR 0135), from `selection`, as the
/// runtime's `CandidateRows`: each state's range of cells, each cell an event
/// and the position of its list, each list once in the pool, its length
/// first.
pub fn candidate_rows(
    tables: &DocumentTables,
    selection: &Selection,
    null_event: u16,
) -> rt::CandidateRows<'static> {
    let mut first_cell: Vec<u16> = Vec::with_capacity(tables.states.len() + 1);
    let mut cells: Vec<rt::CandidateCell> = Vec::new();
    let mut pool: Vec<rt::Candidate> = Vec::new();
    let mut positions: BTreeMap<Vec<u16>, u16> = BTreeMap::new();
    for state_cells in tables.candidates(selection, null_event) {
        first_cell.push(cells.len() as u16);
        for cell in state_cells {
            let position = *positions.entry(cell.candidates.clone()).or_insert_with(|| {
                let position = pool.len() as u16;
                pool.push(rt::Candidate::count(cell.candidates.len() as u16));
                pool.extend(
                    cell.candidates
                        .iter()
                        .map(|&transition| rt::Candidate::new(rt::TransitionId(transition))),
                );
                position
            });
            cells.push(rt::CandidateCell::new(rt::EventId(cell.event), position));
        }
    }
    first_cell.push(cells.len() as u16);
    rt::CandidateRows {
        first_cell: first_cell.leak(),
        cells: cells.leak(),
        candidates: pool.leak(),
    }
}
