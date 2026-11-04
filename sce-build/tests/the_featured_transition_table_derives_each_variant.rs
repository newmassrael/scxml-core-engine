// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The featured transition table (ADR 0057 §7, `docs/SCE_TRANSITION_TABLE.md`):
//! `transition-table` of a document written against a feature model lists every
//! node of every variant, a node not in every variant with its condition in
//! `presence`, and the rows of each variant are the featured rows its
//! configuration keeps. `transition-table --features` writes a variant's own
//! table, which is what each derived table is held to here, for every
//! configuration of the model.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const FEATURES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="session_features">
  <sce:feature name="session-unicast"/>
  <sce:feature name="session-unicast-accept" requires="session-unicast"/>
  <sce:feature name="session-tls"/>
</scxml>
"#;

/// Every configuration of [`FEATURES`]: the sets closed under *requires*.
const CONFIGURATIONS: [&[&str]; 6] = [
    &[],
    &["session-unicast"],
    &["session-unicast", "session-unicast-accept"],
    &["session-tls"],
    &["session-tls", "session-unicast"],
    &["session-tls", "session-unicast", "session-unicast-accept"],
];

/// A machine with a node of each shape a condition reaches: a gated state, a
/// transition and an action under a gate of their own, an `<onentry>` whose
/// actions are under its gate, a condition of two terms, one of two features
/// where one is implied, and the three values a variant computes from nodes
/// another variant lacks: the position of a transition (`connect` follows a
/// gated one), a state's default initial child (`open`'s first child is
/// gated), and a transition's summary of its actions.
const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="machine">
  <sce:import kind="feature-model" src="features.scxml" as="features"/>
  <state id="probe" sce:requires="session-unicast">
    <transition event="ready" target="idle"/>
  </state>
  <state id="idle">
    <onentry sce:requires="session-unicast"><raise event="close"/></onentry>
    <transition event="inbound.start" target="accepting" sce:requires="session-unicast-accept"/>
    <transition event="connect" target="open">
      <raise event="close" sce:requires="session-unicast"/>
      <raise event="close"/>
    </transition>
    <transition event="secure" target="open" sce:requires="any(session-unicast-accept, session-tls)"/>
    <transition event="close" target="done"/>
  </state>
  <state id="accepting" sce:requires="session-unicast-accept">
    <transition event="done" target="open"/>
    <transition event="secure" target="open" sce:requires="session-tls"/>
  </state>
  <state id="open">
    <state id="sealed" sce:requires="session-tls">
      <transition event="close" target="done"/>
    </state>
    <state id="plain">
      <transition event="close" target="done"/>
    </state>
  </state>
  <final id="done"/>
</scxml>
"#;

/// The columns a row states of its node alone, which every variant that keeps
/// the node agrees on. `to` and `action` are two more, except on a state row,
/// where both name the initial child, and `action` on a transition row, where
/// it lists the transition's actions.
const CARRIED: [&str; 7] = [
    "source", "from", "event", "guard", "after", "kind", "targets",
];

/// A directory holding the model and the machine; the machine's path.
fn product_line() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("features.scxml"), FEATURES).unwrap();
    let path = dir.path().join("machine.scxml");
    std::fs::write(&path, MACHINE).unwrap();
    (dir, path)
}

/// The records `transition-table` writes for `machine`, with `--features`
/// when `features` is given.
fn table(machine: &Path, features: Option<&[&str]>, header: bool) -> Vec<Value> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command.arg("transition-table").arg(machine);
    if header {
        command.arg("--header");
    }
    if let Some(features) = features {
        command.arg(format!("--features={}", features.join(",")));
    }
    let run = command.output().expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "transition-table {features:?} failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("a record is JSON"))
        .collect()
}

/// Whether the condition of a row holds where exactly `on` is on: a row with
/// none is in every variant, and one with `presence` where every feature of
/// one of its terms is on.
fn holds(row: &Value, on: &[&str]) -> bool {
    match row.get("presence") {
        None => true,
        Some(terms) => terms.as_array().unwrap().iter().any(|term| {
            term.as_array()
                .unwrap()
                .iter()
                .all(|feature| on.contains(&feature.as_str().unwrap()))
        }),
    }
}

#[test]
fn the_rows_of_each_variant_are_the_featured_rows_its_configuration_keeps() {
    let (_dir, machine) = product_line();
    let featured = table(&machine, None, false);
    let mut positions_moved = false;
    let mut a_default_initial_moved = false;
    let mut a_summary_moved = false;
    for on in CONFIGURATIONS {
        let variant = table(&machine, Some(on), false);
        assert!(
            variant.iter().all(|row| row.get("presence").is_none()),
            "a variant's own rows carry no condition: {on:?}"
        );
        let kept: Vec<&Value> = featured.iter().filter(|row| holds(row, on)).collect();
        assert_eq!(
            kept.len(),
            variant.len(),
            "{on:?}: the kept rows are not the variant's rows one to one"
        );
        let mut kept_transitions: std::collections::BTreeMap<String, u64> = Default::default();
        for (derived, own) in kept.iter().zip(&variant) {
            let at = format!("{on:?} {}", derived["node_path"]);
            for column in CARRIED {
                assert_eq!(derived.get(column), own.get(column), "{at}: {column}");
            }
            if derived["node_path"] != own["node_path"] {
                positions_moved = true;
            }
            let kind = derived["kind"].as_str().unwrap();
            if kind == "state" {
                // Both name the state's initial child.
                if derived["to"] != own["to"] || derived["action"] != own["action"] {
                    a_default_initial_moved = true;
                }
                continue;
            }
            assert_eq!(derived["to"], own["to"], "{at}: to");
            if kind != "transition" {
                assert_eq!(derived["action"], own["action"], "{at}: action");
            } else {
                let rank = kept_transitions
                    .entry(derived["from"].as_str().unwrap().to_string())
                    .or_insert(0);
                assert_eq!(own["index"], Value::from(*rank), "{at}: index");
                *rank += 1;
                if derived["action"] != own["action"] {
                    a_summary_moved = true;
                }
            }
        }
    }
    // Each column the rule leaves to the variant is one some variant does
    // compute differently, so the rule could not have carried it.
    assert!(positions_moved, "no variant renumbered a node_path");
    assert!(
        a_default_initial_moved,
        "no variant moved a default initial child"
    );
    assert!(a_summary_moved, "no variant changed a transition's summary");
}

#[test]
fn a_featured_row_names_its_effective_condition_in_canonical_form() {
    let (_dir, machine) = product_line();
    let featured = table(&machine, None, false);
    let presence = |from: &str, event: &str| -> Value {
        let row = featured
            .iter()
            .find(|row| row["from"] == from && row["event"] == event)
            .unwrap_or_else(|| panic!("no row {from} {event}"));
        row.get("presence").cloned().unwrap_or(Value::Null)
    };
    assert_eq!(
        presence("probe", "(state)"),
        serde_json::json!([["session-unicast"]])
    );
    assert_eq!(
        presence("probe", "ready"),
        serde_json::json!([["session-unicast"]]),
        "a node takes the condition of the gate around it"
    );
    assert_eq!(presence("idle", "(state)"), Value::Null);
    assert_eq!(
        presence("idle", "(entry)"),
        serde_json::json!([["session-unicast"]]),
        "an action takes the condition of its gated block"
    );
    assert_eq!(
        presence("idle", "secure"),
        serde_json::json!([["session-tls"], ["session-unicast-accept"]]),
        "two terms, in the order of their features"
    );
    assert_eq!(
        presence("accepting", "secure"),
        serde_json::json!([["session-tls", "session-unicast-accept"]]),
        "one term, the feature that `session-unicast-accept` implies left out"
    );
}

#[test]
fn a_variants_table_carries_the_hash_of_the_document() {
    let (_dir, machine) = product_line();
    let featured = table(&machine, None, true);
    for on in CONFIGURATIONS {
        let variant = table(&machine, Some(on), true);
        assert_eq!(variant[0]["record"], "header");
        assert_eq!(
            variant[0]["source_hash"], featured[0]["source_hash"],
            "{on:?}: the hash covers the source set, not the selection"
        );
    }
}
