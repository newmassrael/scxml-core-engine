// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which fields of a statechart model the ANALYZER writes, as opposed
//! to the ones an author wrote.
//!
//! [`SCXMLModel`](sce_build::model::SCXMLModel) mixes both on one
//! struct: `states` and `datamodel` come from the document, while
//! `needs_script_engine`, `has_parallel_states` and some forty others
//! are computed for template dispatch. A surface built to be *approved*
//! must show the first set and not the second — a reviewer shown
//! `needs_foreach: true` is being shown SCE's arithmetic, not their
//! document.
//!
//! # How the split is decided
//!
//! Not by reading field names, and not by reading the analyzer. The
//! parse path and [`analyze`](sce_build::analyzer::analyze) are separate
//! passes, so the question is asked of the behaviour: serialise a model
//! straight from the parser, run the analyzer, serialise again, and
//! collect every key whose value moved. A field the analyzer writes is
//! a field the analyzer wrote — whatever its name suggests.
//!
//! ⚠ A field can be BOTH: authored and then normalised in place. Such a
//! field shows up here, and that is the right answer for this test's
//! purpose — anything the analyzer can move is something a rendering
//! must take from before the move, or not at all.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every `.scxml` in the checkout.
///
/// ⚠ This gate and `an_action_uses_only_the_fields_its_tag_declares`
/// are the two the round-trip module names as carrying the half its
/// text layer cannot — so a statechart claim resting on them rests on
/// whatever they sweep. Both swept three directories, and the 253
/// tracked W3C documents in `resources/`, where the statecharts in
/// this tree live in bulk, were in neither. **The prop was measured on
/// a corpus that left out most of what it props up.** What the checkout
/// holds is [`common::repository::files_with_extension`]'s answer.
fn fixture_files() -> Vec<PathBuf> {
    common::repository::files_with_extension("scxml")
}

/// Every property name any type in the wire schema declares.
///
/// ⚠ This is what separates a STRUCT FIELD from a MAP KEY, and without
/// it the measurement is nonsense: the model holds maps keyed by state
/// id (`ancestor_chains`, `leaf_map`, …), so a walk that treats every
/// JSON key as a field name reports `idle`, `green` and `s0` as things
/// the analyzer writes. Measured first without this filter — 148 names,
/// most of them state ids from the fixtures.
///
/// Derived from the schema rather than listed here, so a field added to
/// the IR is covered the moment the schema regenerates.
fn schema_property_names() -> BTreeSet<String> {
    let path = repo_root().join("apis/forge-ast.v1.schema.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse schema: {e}"));
    let mut out = BTreeSet::new();
    fn walk(v: &serde_json::Value, out: &mut BTreeSet<String>) {
        if let serde_json::Value::Object(map) = v {
            if let Some(serde_json::Value::Object(props)) = map.get("properties") {
                for k in props.keys() {
                    out.insert(k.clone());
                }
            }
            for x in map.values() {
                walk(x, out);
            }
        } else if let serde_json::Value::Array(items) = v {
            for x in items {
                walk(x, out);
            }
        }
    }
    walk(&doc, &mut out);
    out
}

/// Every key name whose value differs between the two trees.
///
/// Keyed by NAME rather than by path: the question is which fields the
/// analyzer writes, and a field that moves at one state moves for the
/// same reason at every other.
fn moved_keys(before: &serde_json::Value, after: &serde_json::Value, out: &mut BTreeSet<String>) {
    match (before, after) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            for (k, av) in a {
                match b.get(k) {
                    Some(bv) if av == bv => {}
                    Some(bv) => {
                        out.insert(k.clone());
                        moved_keys(av, bv, out);
                    }
                    None => {
                        out.insert(k.clone());
                    }
                }
            }
            for k in b.keys() {
                if !a.contains_key(k) {
                    out.insert(k.clone());
                }
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            for (av, bv) in a.iter().zip(b) {
                moved_keys(av, bv, out);
            }
        }
        _ => {}
    }
}

#[test]
fn the_fields_the_analyzer_writes_are_the_ones_it_declares() {
    let files = fixture_files();
    let mut moved: BTreeSet<String> = BTreeSet::new();
    let mut measured = 0usize;

    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture");
        let Ok(expanded) = sce_build::parser::expand_preprocessors(&text, stem, path.parent(), &[])
        else {
            continue;
        };
        // Statecharts only — a forge kind has no analyzer pass of this
        // shape, and asking one would measure nothing while looking
        // like coverage.
        let Ok(mut model) = sce_build::parser::SCXMLParser::new().parse_string(&expanded.0, stem)
        else {
            continue;
        };
        let Ok(before) = serde_json::to_value(&model) else {
            continue;
        };
        sce_build::analyzer::analyze(&mut model, stem);
        let Ok(after) = serde_json::to_value(&model) else {
            continue;
        };
        measured += 1;
        moved_keys(&before, &after, &mut moved);
    }

    let fields = schema_property_names();
    let moved: BTreeSet<String> = moved.into_iter().filter(|k| fields.contains(k)).collect();

    println!("documents measured       : {measured}");
    println!("schema property names    : {}", fields.len());
    println!("fields the analyzer wrote: {}", moved.len());
    for k in &moved {
        println!("   {k}");
    }

    // A floor: a sweep that parsed nothing would report an empty set and
    // read as "the analyzer writes nothing", which is the opposite of
    // the truth.
    assert!(
        measured > 0,
        "no statechart parsed — the sweep is vacuous, not clean"
    );

    let expected: BTreeSet<String> = ANALYZER_WRITTEN.iter().map(|s| s.to_string()).collect();
    let added: Vec<&String> = moved.difference(&expected).collect();
    let gone: Vec<&String> = expected.difference(&moved).collect();

    assert!(
        added.is_empty(),
        "the analyzer writes fields this file does not list. A rendering \
         built for review must not show them, so each one needs a \
         decision rather than an append:\n  {added:?}"
    );
    assert!(
        gone.is_empty(),
        "these fields are listed as analyzer-written and the analyzer no \
         longer moves them. Either the pass changed or no fixture \
         exercises them — the list must not keep claiming evidence it \
         does not have:\n  {gone:?}"
    );
}

/// Every field the analyzer writes, measured rather than enumerated.
///
/// ⚠ Five of these are AUTHORED fields the analyzer nonetheless
/// rewrites — `datamodel`, `states`, `transitions`, `variables` and a
/// transition's `type`. They are the reason this list is not simply
/// "the `needs_*` family": a rendering that takes them from after the
/// pass can show a reviewer something the author did not write.
///
/// Refresh by running this test with `--nocapture` and reading the
/// printed set. Do not append to it to make a red go green: a new entry
/// means the analyzer started writing somewhere, which is a decision,
/// not a list update.
const ANALYZER_WRITTEN: &[&str] = &[
    "datamodel",
    "events",
    "execute_entry_actions_needs_this",
    "external_ingress_events",
    "externally_drivable_events",
    "internal_source",
    "is_true_internal",
    "matches_any_event",
    "matching_enum_values",
    "needs_assign_helper",
    "needs_dom_helper",
    "needs_donedata_helper",
    "needs_event_data",
    "needs_event_data_helper",
    "needs_event_invokeid",
    "needs_event_matching_helper",
    "needs_event_name",
    "needs_event_origin",
    "needs_event_origintype",
    "needs_event_scheduler",
    "needs_event_sendid",
    "needs_event_type",
    "needs_event_type_helper",
    "needs_external_flag",
    "needs_foreach",
    "needs_guard_helper",
    "needs_http_send",
    "needs_namelist_helper",
    // Template dispatch, both of them, and neither reaches the page.
    // ⚠ They appeared when this gate stopped sweeping three
    // directories: `needs_parent_template`'s own doc comment names the
    // fixtures that move it — the W3C local-invoke tests 233 and 338 —
    // and those live in `resources/`, which no list here had ever
    // included. **The field said where to look and nobody was
    // looking there.**
    "needs_nonstatic_method",
    "needs_parent_template",
    "needs_send_helper",
    "needs_string_matching",
    "needs_tick_driving",
    "needs_transition_helper",
    // A derived index over the authored `on_sample_blocks`, and a
    // derived parent link over the authored nesting. The renderer
    // takes the authored side of both — `render_scxml_state` says so
    // at the `on sample` loop, where printing the index too "would
    // say each link twice" — so the decision these two need is the
    // one already made: analyzer arithmetic, not shown.
    "on_sample_links",
    "parent",
    "prefix_matching_events",
    "readable_variables",
    "states",
    "transitions",
    "type",
    "uses_cancel",
    "variables",
];
