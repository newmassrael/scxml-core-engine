// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

/// The wire schema the model serialises to, walked alongside a model's
/// JSON so that each key is read as what the schema says it is.
///
/// ⚠ This is what separates a STRUCT FIELD from a MAP KEY, and without
/// it the measurement is nonsense: the model holds maps keyed by state
/// id (`states`, `history_states`, …), so a walk that treats every JSON
/// key as a field name reports `idle`, `green` and `s0` as things the
/// analyzer writes. Measured first without the schema — 148 names, most
/// of them state ids from the fixtures.
///
/// The split is by POSITION, not by name. A filter that kept the keys
/// whose name some schema type declares let a state id through whenever
/// it spelled a field: a document with a state `link` read as the
/// analyzer writing `<sce:on-sample link>`.
///
/// Derived from the schema rather than listed here, so a field added to
/// the IR is covered the moment the schema regenerates.
struct Schema {
    doc: serde_json::Value,
}

impl Schema {
    fn load() -> Self {
        let path = repo_root().join("apis/forge-ast.v1.schema.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let doc = serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse schema: {e}"));
        Self { doc }
    }

    /// The schema of a statechart model: one of `ForgeDocument`'s kinds.
    /// The walk takes every kind, so a key is a field when any kind
    /// declares it at that position.
    fn model(&self) -> Vec<&serde_json::Value> {
        let document = self
            .doc
            .pointer("/definitions/ForgeDocument")
            .expect("the schema defines ForgeDocument");
        self.alternatives(&[document])
    }

    /// The schemas a value under `schemas` may have: each `$ref` followed
    /// and each branch of `allOf` / `anyOf` / `oneOf` taken, alongside the
    /// schema that holds them.
    fn alternatives<'a>(&'a self, schemas: &[&'a serde_json::Value]) -> Vec<&'a serde_json::Value> {
        let mut out: Vec<&serde_json::Value> = Vec::new();
        let mut pending: Vec<&serde_json::Value> = schemas.to_vec();
        while let Some(s) = pending.pop() {
            if out.iter().any(|seen| std::ptr::eq(*seen, s)) {
                continue;
            }
            out.push(s);
            if let Some(target) = s.get("$ref").and_then(|r| r.as_str()) {
                let pointer = target.strip_prefix('#').expect("a local $ref");
                pending.push(
                    self.doc
                        .pointer(pointer)
                        .unwrap_or_else(|| panic!("the schema defines {target}")),
                );
            }
            for combinator in ["allOf", "anyOf", "oneOf"] {
                if let Some(serde_json::Value::Array(branches)) = s.get(combinator) {
                    pending.extend(branches);
                }
            }
        }
        out
    }

    /// The schemas of `key` where one of `schemas` declares it a field.
    fn field<'a>(schemas: &[&'a serde_json::Value], key: &str) -> Vec<&'a serde_json::Value> {
        schemas
            .iter()
            .filter_map(|s| s.get("properties").and_then(|p| p.get(key)))
            .collect()
    }

    /// The schemas of a map's values, where one of `schemas` is a map.
    fn map_value<'a>(schemas: &[&'a serde_json::Value]) -> Vec<&'a serde_json::Value> {
        schemas
            .iter()
            .filter_map(|s| s.get("additionalProperties").filter(|v| v.is_object()))
            .collect()
    }

    /// The schemas of an array's items.
    fn items<'a>(schemas: &[&'a serde_json::Value]) -> Vec<&'a serde_json::Value> {
        schemas
            .iter()
            .filter_map(|s| s.get("items"))
            .flat_map(|items| match items {
                serde_json::Value::Array(tuple) => tuple.iter().collect::<Vec<_>>(),
                single => vec![single],
            })
            .collect()
    }
}

/// What a walk of two models found.
#[derive(Default)]
struct Moved {
    /// Every field, by name, whose value differs between the two trees.
    ///
    /// Keyed by NAME rather than by path: the question is which fields
    /// the analyzer writes, and a field that moves at one state moves for
    /// the same reason at every other.
    fields: BTreeSet<String>,
    /// Keys that moved where the schema describes neither a field nor a
    /// map, so the walk could not say what they are.
    undescribed: BTreeSet<String>,
}

/// Walk `before` and `after` under `schemas`, recording into `moved`.
fn moved_fields(
    schema: &Schema,
    schemas: &[&serde_json::Value],
    before: &serde_json::Value,
    after: &serde_json::Value,
    moved: &mut Moved,
) {
    let schemas = schema.alternatives(schemas);
    match (before, after) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for k in keys {
                let (av, bv) = (a.get(k), b.get(k));
                if av == bv {
                    continue;
                }
                let field = Schema::field(&schemas, k);
                let below = if !field.is_empty() {
                    moved.fields.insert(k.clone());
                    field
                } else {
                    let values = Schema::map_value(&schemas);
                    if values.is_empty() {
                        moved.undescribed.insert(k.clone());
                        continue;
                    }
                    values
                };
                if let (Some(av), Some(bv)) = (av, bv) {
                    moved_fields(schema, &below, av, bv, moved);
                }
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            let items = Schema::items(&schemas);
            for (av, bv) in a.iter().zip(b) {
                moved_fields(schema, &items, av, bv, moved);
            }
        }
        _ => {}
    }
}

#[test]
fn the_fields_the_analyzer_writes_are_the_ones_it_declares() {
    let files = fixture_files();
    let schema = Schema::load();
    let model_schema = schema.model();
    let mut walk = Moved::default();
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
        moved_fields(&schema, &model_schema, &before, &after, &mut walk);
    }
    let moved = walk.fields;

    println!("documents measured       : {measured}");
    println!("fields the analyzer wrote: {}", moved.len());
    for k in &moved {
        println!("   {k}");
    }
    println!("undescribed keys         : {:?}", walk.undescribed);

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

    // The keys the walk could not place are the model's fields the schema
    // leaves out. Held to their own list, so a key the walk stops reading
    // (a schema type it no longer resolves) is a red, not a smaller set.
    let outside: BTreeSet<String> = OUTSIDE_THE_SCHEMA.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        walk.undescribed, outside,
        "the keys the analyzer moved where the schema describes no field \
         and no map are not the ones listed in OUTSIDE_THE_SCHEMA"
    );
}

/// Fields the analyzer writes that the wire schema leaves out on purpose
/// (`schemars(skip)`), so a rendering built from the schema cannot show
/// them.
const OUTSIDE_THE_SCHEMA: &[&str] = &[
    // Whether a `<send idlocation>` under `sce-static` makes the machine
    // count generated ids; transient and outside the AST contract, as its
    // doc comment on the model says.
    "needs_auto_send_id",
];

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
    // An AUTHORED list that moves because a `<send>` inside it receives
    // `type_decisions` / `target_decisions` below: the key moves with its
    // content, and the authored actions are not rewritten. A rendering takes
    // the list as the author wrote it.
    "actions",
    "datamodel",
    // Template dispatch, this and `has_delayed_host_send` below, and neither
    // reaches the page: whether a delayed `<send>` waits for a host to serve
    // it, and the most `<param>`s such a send carries, which is how much room
    // the C11 backend's fixed-size queue entry must make. Both are arithmetic
    // over the authored `delay` and `<param>`, which a rendering shows as
    // themselves — so this is not a second thing to approve. The analyzer
    // writes them itself, not only the host declaration, because a delayed
    // BasicHTTP send owes the same storage whether or not the build declared
    // a host processor.
    "delayed_host_send_max_params",
    "events",
    "execute_entry_actions_needs_this",
    "external_ingress_events",
    "externally_drivable_events",
    // The other half of `delayed_host_send_max_params` above: the yes-or-no.
    "has_delayed_host_send",
    // Template dispatch, as `has_delayed_host_send` above: whether a delayed
    // `<send>` carries a payload, which the C11 backend's fixed-size queue
    // entry must make room for. Arithmetic over the authored `delay` and
    // `<param>`/`namelist`/`<content>`, which a rendering shows as themselves.
    "has_delayed_send_payload",
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
    "needs_foreach",
    "needs_guard_helper",
    "needs_http_send",
    // Template dispatch: whether the Go, Python and Kotlin policies emit
    // the per-policy `idlocation` store helper. Derived from the authored
    // `idlocation` attributes, which is what a rendering shows; the flag
    // itself never reaches the page.
    "needs_idlocation_store",
    "needs_namelist_helper",
    // Template dispatch, both of them, and neither reaches the page.
    // ⚠ They appeared when this gate stopped sweeping three
    // directories: `needs_parent_template`'s own doc comment names the
    // fixtures that move it — the W3C local-invoke tests 233 and 338 —
    // and those live in `resources/`, which no list here had ever
    // included. **The field said where to look and nobody was
    // looking there.**
    "needs_nonstatic_method",
    // Template dispatch, from the one analyzer both `needs_parent`'s own
    // doc comment and the manifest read
    // (`crate::parent_send_analyzer::needs_parent`). A rendering must not
    // show it: what an author wrote is the literal `<send
    // target="#_parent">` sites, which a review surface shows as
    // themselves; this boolean is a summary of that fact for the root-
    // start policy template, not a second thing to approve.
    "needs_parent",
    "needs_parent_template",
    "needs_send_helper",
    "needs_string_matching",
    "needs_tick_driving",
    "needs_transition_helper",
    // A derived index over the authored `on_sample_blocks`. The renderer
    // takes the authored side — `render_scxml_state` says so at the
    // `on sample` loop, where printing the index too "would say each
    // link twice" — so the decision it needs is the one already made:
    // analyzer arithmetic, not shown.
    // ⚠ `parent` was listed beside it as "a derived parent link". The
    // parser writes a state's `parent`, not the analyzer; the entry was
    // a state whose id is `parent`, read as a field by a filter that
    // went by name.
    "on_sample_links",
    "prefix_matching_events",
    "readable_variables",
    "states",
    // The open questions a `<send>`'s target and type rest on, summed from
    // the `sce:unresolved` markers on the send and on the data its
    // expressions name (`computed_route_analyzer::annotate`). A rendering
    // shows the markers themselves, which are authored and are the open
    // questions; these two are the analyzer's per-expression summary of them
    // for the Python send template, not a second thing to approve.
    "target_decisions",
    "transitions",
    "type",
    "type_decisions",
    "uses_cancel",
    "variables",
];
