// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What enables a transition is a row of its document's tables (ADR 0127),
//! made from the events the generated event enum numbers.
//!
//! A Rust machine used to answer the selection itself: for each written
//! transition its template compared the event with the enum members the
//! analyzer listed for a single literal descriptor, matched the event's name
//! against any other descriptor at run time, and took an eventless transition
//! for the null event alone. The rows replace that comparison. For every
//! document the generator accepts, as the Rust backend lowers it, every
//! written transition and every member of its event enum, the null event among
//! them, the rows say a member enables a transition exactly when that
//! comparison said so; each state's range is its own written transitions in
//! the order they are written; no other transition answers an event; and the
//! rows, written as the runtime's own, pass the runtime's
//! `DocumentTables::check`.
//!
//! The population is the one `a_documents_tables_are_its_tree_in_document_order`
//! walks: every tracked `.scxml` that parses and that
//! `analyzer::can_generate_static` accepts. Each count carries a floor, so a
//! walk that stopped looking does not pass as one that found every document
//! clean.

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use sce_build::document_tables::{DocumentTables, Selection, StateKind, TransitionOrigin};
use sce_build::generator::{rust_numbered_events, Language};
use sce_build::model::{SCXMLModel, Transition};
use sce_build::parser::SCXMLParser;
use sce_rust_runtime::helpers::event_matching::matches_event_descriptor;
use sce_rust_runtime::helpers::tables as rt;

/// The model the Rust backend renders `path` from, or `None` when the document
/// does not parse or the generator refuses it.
fn generatable(path: &Path) -> Option<SCXMLModel> {
    let label = path.to_str()?;
    let mut model = SCXMLParser::new().parse_file(label).ok()?;
    sce_build::analyzer::analyze(&mut model, label);
    sce_build::analyzer::can_generate_static(&model, label).ok()?;
    Some(sce_build::host_processor_analyzer::lower_mesh(&model, Language::Rust).into_owned())
}

/// Whether the generated `first_enabled_transition` took `transition` for the
/// member `member` of the enum whose named members are `events`, the null
/// event numbered after them, as its `event_match` compared them: an
/// eventless transition for the null event alone; a descriptor the analyzer
/// left to run time by the member's name (`names`, what `get_event_name`
/// answers) and never for the null event; a single literal by the enum
/// members the analyzer listed for it; any other, never.
fn former_event_match(
    transition: &Transition,
    member: usize,
    events: &[&str],
    names: &[String],
) -> bool {
    let null = events.len();
    if transition.event.is_empty() {
        return member == null;
    }
    if member == null {
        return false;
    }
    if transition.needs_string_matching {
        return matches_event_descriptor(&names[member], &transition.event);
    }
    let variant = sce_build::filters::to_event_variant(events[member].to_string());
    transition
        .matching_enum_values
        .iter()
        .any(|listed| sce_build::filters::to_event_variant(listed.clone()) == variant)
}

/// What one document's rows were held to, counted.
#[derive(Default)]
struct Seen {
    literal: usize,
    string_matching: usize,
    eventless: usize,
    enabling_pairs: usize,
}

/// Every claim of the selection rows, asked of one document.
fn holds(name: &str, model: &SCXMLModel, tables: &DocumentTables, seen: &mut Seen) {
    let events = rust_numbered_events(model);
    let names: Vec<String> = events
        .iter()
        .map(|event| sce_build::filters::event_wire_name((*event).to_string()))
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let selection = tables
        .selection(model, &name_refs)
        .unwrap_or_else(|refusal| panic!("{name}: no selection rows: {refusal}"));
    assert_eq!(
        selection.first_written.len(),
        tables.states.len() + 1,
        "{name}: one entry for each state and one past the last"
    );
    assert_eq!(
        selection.triggers.len(),
        tables.transitions.len(),
        "{name}: one row for each transition"
    );

    // Each state's range is its own written transitions, in the order they are
    // written: the k-th of the range is its k-th.
    for (state, range) in selection.first_written.windows(2).enumerate() {
        for (k, number) in (range[0]..range[1]).enumerate() {
            let row = &tables.transitions[usize::from(number)];
            assert_eq!(
                (row.source, row.origin),
                (
                    Some(state as u16),
                    TransitionOrigin::Written {
                        index_in_source: k as u16
                    }
                ),
                "{name}: state {state}'s range holds its own transitions in order"
            );
        }
    }

    for (number, row) in tables.transitions.iter().enumerate() {
        let list = &selection.triggers[number];
        assert!(
            list.windows(2).all(|pair| pair[0] < pair[1]),
            "{name}: transition {number}'s events ascend"
        );
        let (TransitionOrigin::Written { index_in_source }, Some(source)) =
            (row.origin, row.source)
        else {
            assert!(
                list.is_empty(),
                "{name}: transition {number} is no written one, so no selection returns it"
            );
            continue;
        };
        let id = &tables.states[usize::from(source)].id;
        let transition = &model.states[id].transitions[usize::from(index_in_source)];
        if transition.event.is_empty() {
            seen.eventless += 1;
        } else if transition.needs_string_matching {
            seen.string_matching += 1;
        } else {
            seen.literal += 1;
        }
        for member in 0..=events.len() {
            let row_says = list.contains(&(member as u16));
            let former = former_event_match(transition, member, &events, &names);
            assert_eq!(
                row_says,
                former,
                "{name}: state `{id}`, transition {index_in_source} (event `{}`), member {member} \
                 ({:?}): the rows and the former comparison disagree",
                transition.event,
                events.get(member).copied().unwrap_or("the null event")
            );
            seen.enabling_pairs += usize::from(row_says);
        }
    }

    assert_eq!(
        runtime_tables(tables, &selection, events.len()).check(),
        Ok(()),
        "{name}: the runtime's check of the selection rows"
    );
}

/// `tables` with `selection` as the runtime's `DocumentTables`, each list once
/// in the pool, its length first, as the generator writes them. The targets are
/// not what is asked here and are left out. Each pool is leaked, for the life of
/// the test process.
fn runtime_tables(
    tables: &DocumentTables,
    selection: &Selection,
    events: usize,
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
    let transitions: Vec<rt::TransitionRow> = tables
        .transitions
        .iter()
        .map(|row| {
            rt::TransitionRow::new(
                row.source.map(rt::StateId),
                rt::Target::NONE,
                row.internal,
                row.has_content,
            )
        })
        .collect();
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
    rt::DocumentTables {
        states: states.leak(),
        histories: histories.leak(),
        transitions: transitions.leak(),
        targets: &[],
        initial: rt::TransitionId(tables.initial),
        default_state: rt::StateId(0),
        null_event: rt::EventId(events as u16),
        error_events: &[],
        walk: &sce_rust_runtime::GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: tables.uses_history().then_some(&sce_rust_runtime::HISTORY),
        parallel: tables
            .uses_parallel()
            .then_some(&sce_rust_runtime::PARALLEL),
        selection: Some(rt::SelectionRows {
            first_written: first_written.leak(),
            triggers: triggers.leak(),
            trigger_events: pool.leak(),
        }),
    }
}

#[test]
fn every_generatable_documents_rows_enable_what_its_descriptors_matched() {
    let corpus = common::repository::files_git_tracks(&["*.scxml"]);
    let mut judged = 0usize;
    let mut seen = Seen::default();
    for path in &corpus {
        let Some(model) = generatable(path) else {
            continue;
        };
        let name = path.display().to_string();
        let tables = DocumentTables::of(&model)
            .unwrap_or_else(|refusal| panic!("{name}: no tables: {refusal}"));
        holds(&name, &model, &tables, &mut seen);
        judged += 1;
    }
    // Every kind of transition the former comparison distinguished was asked
    // about, over real documents: single literals, descriptors matched by
    // name, eventless transitions, and members that enable one.
    assert!(seen.literal >= 500, "{} single literals", seen.literal);
    assert!(
        seen.string_matching >= 20,
        "{} descriptors matched by name",
        seen.string_matching
    );
    assert!(seen.eventless >= 100, "{} eventless", seen.eventless);
    assert!(
        seen.enabling_pairs >= 1000,
        "{} members enable a transition",
        seen.enabling_pairs
    );
    assert!(corpus.len() >= 1000, "walked {} documents", corpus.len());
    assert!(
        judged >= 100,
        "judged {judged} of {} documents",
        corpus.len()
    );
}
