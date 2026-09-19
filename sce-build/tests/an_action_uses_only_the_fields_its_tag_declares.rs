// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An action uses only the fields its tag declares.
//!
//! [`Action`](sce_build::model::Action) holds nine kinds of executable
//! content in one flat struct with a `String` tag, and
//! `Action::authored_fields` / `derived_fields` is the one place that
//! says which fields belong to which tag. This file is what keeps that
//! statement true: it walks the fixture corpus, groups every action by
//! its tag, and requires each non-default field to be declared for it.
//!
//! # ⚠ Non-default value, not key presence
//!
//! Most of the struct's fields are plain `String` / `bool` / `i64`, so
//! they serialize for every tag whether or not the author wrote
//! anything. A check keyed on presence sees 38 fields under every tag
//! and passes on a table that says nothing. The unit here is therefore
//! "the value differs from the type's default".
//!
//! # What a failure means
//!
//! Either a field reached a tag the table does not list — the table is
//! stale, or the parser is writing across tags — or a tag is unlisted
//! entirely. Both are answers about the tree; neither is a reason to
//! widen the table without reading why.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sce_build::model::Action;
use sce_build::DocumentLabel;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

fn fixture_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut out = Vec::new();
    for sub in ["tests/forge/resources", "integration_resources", "examples"] {
        collect(&root.join(sub), &mut out);
    }
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "scxml") {
            out.push(p);
        }
    }
}

/// Whether a serialized value is anything other than its type's default.
fn non_default(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64() != Some(0.0),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

/// Walk a serialized document and report, per action tag, the fields
/// carrying a non-default value.
fn tally(node: &serde_json::Value, out: &mut BTreeMap<String, BTreeSet<String>>) {
    match node {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(tag)) = map.get("type") {
                if Action::TAGS.contains(&tag.as_str()) {
                    let entry = out.entry(tag.clone()).or_default();
                    for (k, v) in map {
                        if k != "type" && non_default(v) {
                            entry.insert(k.clone());
                        }
                    }
                }
            }
            for v in map.values() {
                tally(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                tally(v, out);
            }
        }
        _ => {}
    }
}

/// Documents this test writes so that every tag has a case.
///
/// ⚠ Three tags — `if`, `foreach`, `native_action` — appear in no
/// fixture under the roots above; their cases live in the generated W3C
/// corpus, which is a build artefact and not part of this checkout. A
/// table row nothing exercises is a row that can go stale in silence,
/// so the inputs are built here instead of depending on another tree or
/// on a fixture some other suite also sweeps.
fn synthesised_documents() -> Vec<(&'static str, String)> {
    let wrap = |body: &str| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="s0">
  <datamodel><data id="v" expr="0"/></datamodel>
  <state id="s0">
    <onentry>{body}</onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>"#
        )
    };
    vec![
        (
            "synth_if",
            wrap(
                r#"<if cond="v == 0"><raise event="a"/>
                   <elseif cond="v == 1"/><raise event="b"/>
                   <else/><raise event="c"/></if>"#,
            ),
        ),
        (
            "synth_foreach",
            wrap(r#"<foreach array="v" item="it" index="ix"><raise event="a"/></foreach>"#),
        ),
        (
            "synth_native_action",
            wrap(r#"<sce:action name="op"><sce:arg name="x" expr="v"/></sce:action>"#),
        ),
    ]
}

#[test]
fn every_field_an_action_carries_is_declared_for_its_tag() {
    let files = fixture_files();
    assert!(!files.is_empty(), "no fixture found — the sweep is vacuous");

    let mut seen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut parsed = 0usize;

    for (name, text) in synthesised_documents() {
        let model = sce_build::parser::SCXMLParser::new()
            .parse_string(&text, name)
            .unwrap_or_else(|e| panic!("the test's own {name} document must parse: {e:?}"));
        let value = serde_json::to_value(model).expect("a model serialises");
        parsed += 1;
        tally(&value, &mut seen);
    }

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
        let label = DocumentLabel {
            identifier: stem,
            diagnostic_label: stem,
        };
        // ⚠ BOTH pipelines. Executable content lives in statechart
        // documents, and `parse_forge_with_imports` answers `Ok(None)`
        // for exactly those — so a sweep through the forge entry point
        // alone parses 162 documents and finds zero actions. Measured
        // while writing this file; the floor below is what caught it.
        let value = match sce_build::forge::parser::parse_forge_with_imports(&expanded.0, label) {
            Ok(Some(doc)) => serde_json::to_value(&doc).ok(),
            Ok(None) => sce_build::parser::SCXMLParser::new()
                .parse_string(&expanded.0, stem)
                .ok()
                .and_then(|m| serde_json::to_value(m).ok()),
            Err(_) => None,
        };
        let Some(value) = value else {
            continue;
        };
        parsed += 1;
        tally(&value, &mut seen);
    }

    // `source_location` travels on every action regardless of tag, so
    // it is not a per-tag question. Named once here rather than added
    // to nine rows, which would read as nine separate decisions.
    let universal: BTreeSet<&str> = ["source_location", "req", "provenance", "unresolved"]
        .into_iter()
        .collect();

    let mut undeclared: Vec<String> = Vec::new();
    for (tag, fields) in &seen {
        let declared: BTreeSet<&str> = Action::authored_fields(tag)
            .iter()
            .chain(Action::derived_fields(tag))
            .copied()
            .collect();
        for f in fields {
            if !declared.contains(f.as_str()) && !universal.contains(f.as_str()) {
                undeclared.push(format!("<{tag}> carries `{f}`"));
            }
        }
    }

    let unexercised: Vec<&str> = Action::TAGS
        .iter()
        .copied()
        .filter(|t| !seen.contains_key(*t))
        .collect();

    println!("documents parsed : {parsed}");
    println!("tags exercised   : {}", seen.len());
    println!("tags unexercised : {unexercised:?}");

    // A floor: a sweep that parsed nothing, or found no action at all,
    // would satisfy the assertion below by looking at nothing.
    assert!(
        parsed > 0 && !seen.is_empty(),
        "nothing was measured — {parsed} document(s) parsed, {} tag(s) seen",
        seen.len()
    );

    assert!(
        undeclared.is_empty(),
        "these fields reach an action tag that `Action::authored_fields` \
         / `derived_fields` does not list for it — the table is stale, \
         or the parser writes a field across tags:\n  {}",
        undeclared.join("\n  ")
    );
}
