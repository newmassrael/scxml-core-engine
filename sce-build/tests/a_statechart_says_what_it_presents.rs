// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A statechart says on its manifest what it presents to the outside.
//!
//! # What was wrong
//!
//! A scenario set proposes the names its examples are written against, and the
//! owner accepts them with the examples; a design is drafted separately, from the
//! same prose. Nothing held the two to each other. Measured 2026-09-29, five
//! drafts of one specification invented five interfaces, and an example that
//! names a state the design calls something else FAILED, which reads as the
//! design misbehaving when it is two names for one thing.
//!
//! So the design's half is PUBLISHED (`Manifest::surface`: the events a caller
//! can deliver, the events it sends out of the session, its states, its data), and
//! a driver copies it into the observation trace for the judge to compare. These
//! cases hold the published half to the document, on the binary a host runs, and
//! to the wire schema a consumer reads it with.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The manifest `check -l rust` writes for `body`, validated against the
/// checked-in schema before it is returned.
fn manifest(label: &str, body: &str) -> serde_json::Value {
    let dir = scratch(label);
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    let out = Command::new(sce_codegen_bin())
        .args(["--error-format", "json", "check"])
        .arg(&path)
        .args(["-l", "rust"])
        .output()
        .expect("run sce-codegen check");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{label} is valid SCXML and must check: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the manifest is one JSON line");

    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("schemas/sce-manifest.v1.schema.json"))
            .expect("read manifest schema"),
    )
    .expect("manifest schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("manifest schema compiles");
    let violations: Vec<String> = match validator.validate(&manifest) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    assert!(violations.is_empty(), "{label}: {violations:?}\n{manifest}");
    manifest
}

fn names(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("a list of names, got {value}"))
        .iter()
        .map(|name| name.as_str().expect("a name is text"))
        .collect()
}

const VENDING: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="vending" initial="idle" datamodel="ecmascript">
  <datamodel><data id="credit" expr="0"/></datamodel>
  <state id="idle"><transition event="coin" target="paid"/></state>
  <state id="paid">
    <onentry>
      <send event="dispense" type="x-sce-host"/>
      <send event="retry" delay="1s"/>
    </onentry>
    <transition event="refund" target="idle">
      <if cond="true"><send event="refunded" type="x-sce-host"/></if>
    </transition>
    <transition event="retry" target="idle"/>
  </state>
</scxml>
"#;

/// What a caller can deliver, what leaves the session, the states and the data:
/// each read from the analyzer's own facts, a nested send included.
#[test]
fn a_statechart_publishes_the_names_it_presents() {
    let m = manifest("vending", VENDING);
    let surface = &m["surface"];
    // `retry` is sent to the session with no target, which is the EXTERNAL queue
    // (W3C SCXML 6.2.4), so a caller can deliver it as well.
    assert_eq!(
        names(&surface["inputs"]),
        ["coin", "refund", "retry"],
        "{m}"
    );
    assert_eq!(names(&surface["outputs"]), ["dispense", "refunded"], "{m}");
    let states = names(&surface["states"]);
    for state in ["idle", "paid"] {
        assert!(states.contains(&state), "{state} missing from {states:?}");
    }
    assert_eq!(names(&surface["data"]), ["credit"], "{m}");
    // Absent flags are omitted, not false: nothing here takes `*` or names an
    // event by expression.
    assert!(surface.get("takes_any_input").is_none(), "{m}");
    assert!(surface.get("computed_outputs").is_none(), "{m}");
}

#[test]
fn a_send_that_names_its_event_by_expression_is_marked_and_not_listed() {
    let m = manifest(
        "computed",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="computed" initial="a" datamodel="ecmascript">
  <state id="a"><onentry><send eventexpr="'out'" type="x-sce-host"/></onentry></state>
</scxml>
"#,
    );
    assert_eq!(m["surface"]["computed_outputs"], true, "{m}");
    assert_eq!(names(&m["surface"]["outputs"]), Vec::<&str>::new(), "{m}");
}

/// A document that is not a statechart has no surface: its inputs and outputs are
/// parameters and returns, and a field that said otherwise would be read as a
/// statechart's.
#[test]
fn a_document_that_is_not_a_statechart_publishes_no_surface() {
    let body = std::fs::read_to_string(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/algorithm_days_in_month.scxml"),
    )
    .expect("fixture");
    let m = manifest("days_in_month", &body);
    assert!(m.get("surface").is_none(), "{m}");
}
