// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Every document the generator accepts has tables (ADR 0064), and the tables
//! hold the numbering the runtime's one Appendix D core relies on.
//!
//! The population is every tracked `.scxml` that parses and that
//! `analyzer::can_generate_static` accepts, the gate both pipelines share, as
//! the Rust backend lowers it. A document refused before generation is never
//! tabled, so it is walked and not judged; both counts carry a floor.
//!
//! The rows are also held to the model fields the generated structural methods
//! of `StatePolicy` are emitted from today (`get_parent`, `get_child_states`,
//! `is_compound_state`, `get_initial_targets`, `get_history_default_targets`,
//! `get_document_initial_targets`, `first_enabled_transition`'s targets,
//! `transition_index`, `has_actions` and `is_internal`), so the tables answer
//! what those methods answer. No row lists a state's children; they are read
//! from the pre-order numbering as the runtime reads them. And each document's
//! rows, written as the runtime's own, pass the runtime's
//! `DocumentTables::check`.

mod common;

use std::path::Path;

use sce_build::document_tables::{
    DocumentTables, Span, StateKind, TargetRef, TransitionOrigin, TransitionRow,
};
use sce_build::generator::Language;
use sce_build::microstep_bounds::MicrostepBounds;
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;
use sce_rust_runtime::helpers::tables as rt;

/// The model the Rust backend renders `path` from, or `None` when the document
/// does not parse or the generator refuses it: the corpus carries deliberate
/// negative fixtures and forge documents, which the stages that judge them
/// report.
fn generatable(path: &Path) -> Option<SCXMLModel> {
    let label = path.to_str()?;
    let mut model = SCXMLParser::new().parse_file(label).ok()?;
    sce_build::analyzer::analyze(&mut model, label);
    sce_build::analyzer::can_generate_static(&model, label).ok()?;
    Some(sce_build::host_processor_analyzer::lower_mesh(&model, Language::Rust).into_owned())
}

/// The ids a span of the target pool names, a `<history>` by its own id.
fn target_ids(tables: &DocumentTables, span: Span) -> Vec<String> {
    tables.targets[span.range()]
        .iter()
        .map(|target| match *target {
            TargetRef::State(id) => tables.states[usize::from(id)].id.clone(),
            TargetRef::History(id) => tables.histories[usize::from(id)].id.clone(),
        })
        .collect()
}

/// Whether `ancestor` is a proper ancestor of `state`, walking the parent
/// column up.
fn is_proper_ancestor(tables: &DocumentTables, ancestor: usize, state: usize) -> bool {
    let mut current = tables.states[state].parent;
    while let Some(parent) = current {
        if usize::from(parent) == ancestor {
            return true;
        }
        current = tables.states[usize::from(parent)].parent;
    }
    false
}

/// The child states of `state` as the runtime reads them from the rows: the
/// next number, then each number past the subtree of the one before, up to the
/// end of `state`'s subtree.
fn children_of(tables: &DocumentTables, state: usize) -> Vec<usize> {
    let last = usize::from(tables.states[state].last_descendant);
    let mut children = Vec::new();
    let mut next = state + 1;
    while next <= last {
        children.push(next);
        let subtree_end = usize::from(tables.states[next].last_descendant);
        assert!(
            subtree_end >= next,
            "state {next}'s subtree ends before it, so the walk would not advance"
        );
        next = subtree_end + 1;
    }
    children
}

/// Every claim of the numbering, asked of one document's tables.
fn holds(name: &str, model: &SCXMLModel, tables: &DocumentTables) {
    let states = tables.states.len();

    // StateIds are the states in document order, each once.
    let mut by_order: Vec<(u32, &str)> = model
        .states
        .iter()
        .map(|(id, state)| (state.document_order, id.as_str()))
        .collect();
    by_order.sort();
    let numbered: Vec<&str> = tables.states.iter().map(|row| row.id.as_str()).collect();
    let ordered: Vec<&str> = by_order.iter().map(|(_, id)| *id).collect();
    assert_eq!(numbered, ordered, "{name}: states in document order");
    assert!(
        by_order.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "{name}: no two states share a document-order rank"
    );

    for (index, row) in tables.states.iter().enumerate() {
        let state = &model.states[&row.id];
        let at = |what: &str| format!("{name}: state `{}`: {what}", row.id);

        // A parent precedes its child, and is the state the model names.
        if let Some(parent) = row.parent {
            assert!(usize::from(parent) < index, "{}", at("parent precedes it"));
        }
        assert_eq!(
            row.parent
                .map(|p| tables.states[usize::from(p)].id.as_str()),
            state.parent.as_deref(),
            "{}",
            at("parent as in the model")
        );

        // No row lists the children: the runtime reads them from the
        // pre-order numbering, the next number and then each number past the
        // subtree of the one before, and they are `get_child_states`', in
        // document order.
        let children: Vec<&str> = children_of(tables, index)
            .into_iter()
            .map(|c| tables.states[c].id.as_str())
            .collect();
        assert_eq!(children, state.children, "{}", at("children"));

        // The kind is what `is_final_state`, `is_parallel_state` and
        // `is_compound_state` say, and a compound state is exactly a <state>
        // with child states.
        let expected = if state.is_parallel {
            StateKind::Parallel
        } else if state.is_final {
            StateKind::Final
        } else if !state.initial_targets.is_empty() {
            StateKind::Compound
        } else {
            StateKind::Atomic
        };
        assert_eq!(row.kind, expected, "{}", at("kind"));
        if !state.is_parallel && !state.is_final {
            assert_eq!(
                row.kind == StateKind::Compound,
                !state.children.is_empty(),
                "{}",
                at("compound exactly when it has child states")
            );
        }

        // Every compound state has an initial transition row, and only one.
        match row.initial {
            Some(initial) => {
                assert_eq!(
                    row.kind,
                    StateKind::Compound,
                    "{}",
                    at("initial only when compound")
                );
                let transition = tables.transitions[usize::from(initial)];
                assert_eq!(
                    transition.origin,
                    TransitionOrigin::Initial,
                    "{}",
                    at("initial row")
                );
                assert_eq!(
                    transition.source,
                    Some(index as u16),
                    "{}",
                    at("initial source")
                );
                assert_eq!(
                    target_ids(tables, transition.targets),
                    state.initial_targets,
                    "{}",
                    at("initial targets as get_initial_targets emits them")
                );
                assert_eq!(
                    transition.has_content,
                    !state.initial_transition_actions.is_empty(),
                    "{}",
                    at("initial content")
                );
            }
            None => assert_ne!(
                row.kind,
                StateKind::Compound,
                "{}",
                at("compound has initial")
            ),
        }

        // The subtree is exactly the run (index, last_descendant]: every state
        // in it has this one as an ancestor, and no state after it does.
        let last = usize::from(row.last_descendant);
        assert!(
            (index..states).contains(&last),
            "{}",
            at("last descendant in range")
        );
        for (other, other_row) in tables.states.iter().enumerate().skip(index + 1) {
            assert_eq!(
                is_proper_ancestor(tables, index, other),
                other <= last,
                "{}",
                at(&format!(
                    "subtree holds `{}` exactly when in its run",
                    other_row.id
                ))
            );
        }
        if let Some(parent) = row.parent {
            let parent_last = tables.states[usize::from(parent)].last_descendant;
            assert!(
                row.last_descendant <= parent_last,
                "{}",
                at("its run lies inside its parent's")
            );
        }
    }

    // Written transitions: contiguous by source in StateId order, each its
    // source's `transition_index`-th, with the targets, content and type the
    // generated `first_enabled_transition` emits.
    let written: Vec<(usize, &TransitionRow)> = tables
        .transitions
        .iter()
        .enumerate()
        .take_while(|(_, row)| matches!(row.origin, TransitionOrigin::Written { .. }))
        .collect();
    let total: usize = model.states.values().map(|s| s.transitions.len()).sum();
    assert_eq!(
        written.len(),
        total,
        "{name}: written transitions come first"
    );
    let mut previous_source = 0u16;
    for (id, row) in &written {
        let source = row.source.expect("a written transition has a source");
        assert!(
            source >= previous_source,
            "{name}: written rows grouped by source"
        );
        previous_source = source;
        let TransitionOrigin::Written { index_in_source } = row.origin else {
            unreachable!("taken while written");
        };
        let state_row = &tables.states[usize::from(source)];
        let first = tables
            .transition_id(&state_row.id, 0)
            .expect("its source's first transition");
        assert_eq!(
            usize::from(first) + usize::from(index_in_source),
            *id,
            "{name}: a written transition's id less its source's first is its index"
        );
        let transition = &model.states[&state_row.id].transitions[usize::from(index_in_source)];
        assert_eq!(
            transition.transition_index,
            usize::from(index_in_source),
            "{name}: index in source is the generator's transition_index"
        );
        assert_eq!(
            target_ids(tables, row.targets),
            transition.targets,
            "{name}: targets"
        );
        assert_eq!(
            row.internal,
            transition.transition_type == "internal",
            "{name}: type"
        );
        assert_eq!(
            row.has_content,
            !transition.actions.is_empty(),
            "{name}: content"
        );
    }

    // Then the initial rows in StateId order, the history defaults in
    // HistoryId order, and the document's last.
    let compound = tables
        .states
        .iter()
        .filter(|row| row.kind == StateKind::Compound)
        .count();
    let after_written = written.len();
    let after_initial = after_written + compound;
    let after_defaults = after_initial + tables.histories.len();
    assert_eq!(
        tables.transitions.len(),
        after_defaults + 1,
        "{name}: row count"
    );
    let initial_sources: Vec<Option<u16>> = tables.transitions[after_written..after_initial]
        .iter()
        .map(|row| row.source)
        .collect();
    let mut sorted = initial_sources.clone();
    sorted.sort();
    assert_eq!(
        initial_sources, sorted,
        "{name}: initial rows in StateId order"
    );
    assert_eq!(
        usize::from(tables.initial),
        after_defaults,
        "{name}: document row last"
    );
    let document = tables.transitions[after_defaults];
    assert_eq!(
        document.origin,
        TransitionOrigin::Document,
        "{name}: document row"
    );
    assert_eq!(document.source, None, "{name}: document row has no source");
    assert_eq!(
        target_ids(tables, document.targets),
        model.initial_targets,
        "{name}: document targets as get_document_initial_targets emits them"
    );

    // Histories: in the order the History enum declares them, each with its
    // default row.
    let history_ids: Vec<&str> = model.history_states.keys().map(String::as_str).collect();
    let tabled: Vec<&str> = tables.histories.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(tabled, history_ids, "{name}: histories in id order");
    for (number, row) in tables.histories.iter().enumerate() {
        let info = &model.history_states[&row.id];
        assert_eq!(
            tables.states[usize::from(row.parent)].id,
            info.parent,
            "{name}: history `{}` parent",
            row.id
        );
        assert_eq!(
            row.deep,
            info.history_type == "deep",
            "{name}: history depth"
        );
        assert_eq!(
            usize::from(row.default),
            after_initial + number,
            "{name}: default row"
        );
        let default = tables.transitions[usize::from(row.default)];
        assert_eq!(
            default.origin,
            TransitionOrigin::HistoryDefault(number as u16),
            "{name}: default origin"
        );
        assert_eq!(
            target_ids(tables, default.targets),
            info.default_targets,
            "{name}: defaults as get_history_default_targets emits them"
        );
        assert_eq!(
            default.has_content,
            !info.default_actions.is_empty(),
            "{name}: default content"
        );
    }

    // Every span is inside its pool.
    for row in &tables.transitions {
        assert!(
            row.targets.range().end <= tables.targets.len(),
            "{name}: target span"
        );
    }

    assert_eq!(tables.bounds, MicrostepBounds::of(model), "{name}: bounds");

    // And the runtime, reading the rows as a generated machine states them,
    // finds every rule it relies on kept.
    assert_eq!(
        runtime_tables(tables).check(),
        Ok(()),
        "{name}: runtime check"
    );
}

/// `tables` as the runtime's `DocumentTables`, the rows a generated machine's
/// constant states. The runtime's rows borrow for `'static`, so each pool is
/// leaked, for the life of the test process.
fn runtime_tables(tables: &DocumentTables) -> rt::DocumentTables {
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
    // Packed as the generator packs them (docs/adr/0067): a transition holds
    // its only target itself, and the pool holds only lists of more than one,
    // each its length first.
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
        // What these tests ask is the state tree; the main loop's events and
        // the state an engine names before it starts are the template's
        // (docs/adr/0068), not this model's.
        default_state: rt::StateId(0),
        null_event: rt::EventId(0),
        error_events: &[],
        // The feature procedures exactly when the document uses the feature,
        // as the generator writes them (docs/adr/0066).
        history: tables.uses_history().then_some(&sce_rust_runtime::HISTORY),
        parallel: tables
            .uses_parallel()
            .then_some(&sce_rust_runtime::PARALLEL),
    }
}

#[test]
fn the_generator_numbers_no_more_ids_than_the_runtime_reads() {
    assert_eq!(
        sce_build::document_tables::MAX_IDS,
        rt::MAX_IDS,
        "a table the generator accepts is one the runtime can number"
    );
}

#[test]
fn every_generatable_document_has_tables_in_document_order() {
    let corpus = common::repository::files_git_tracks(&["*.scxml"]);
    let mut judged = 0usize;
    for path in &corpus {
        let Some(model) = generatable(path) else {
            continue;
        };
        let name = path.display().to_string();
        let tables = DocumentTables::of(&model)
            .unwrap_or_else(|refusal| panic!("{name}: no tables: {refusal}"));
        holds(&name, &model, &tables);
        judged += 1;
    }
    // 1,362 documents were tracked on 2026-10-29, 253 of them the W3C
    // corpus's; a walk that sees far fewer, or judges fewer than a hundred,
    // has stopped looking rather than found every document clean.
    assert!(corpus.len() >= 1000, "walked {} documents", corpus.len());
    assert!(
        judged >= 100,
        "judged {judged} of {} documents",
        corpus.len()
    );
}
