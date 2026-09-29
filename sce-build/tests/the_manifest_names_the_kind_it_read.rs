// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The manifest names the kind the run read its document as.
//!
//! # What was wrong
//!
//! A document with no `sce:kind` is a statechart (SCE_FORGE.md §3.2). An
//! author choosing a kind from a prose specification — through the
//! authoring MCP, with no access to this tree — writes the document, runs
//! `check`, and reads the verdict. When the attribute was forgotten, or
//! written outside the SCE namespace, the verdict was about a statechart,
//! and nothing on it said so: a well-formed statechart produces no
//! diagnostic, so "accepted" read the same as "accepted as the transform I
//! meant".
//!
//! So the reading is published: `document_kind.name` is the kind the run
//! compiled, and `document_kind.declared` is `false` exactly when the
//! default supplied it. These cases hold both to the document, on the
//! binary a host runs, and to the wire schema a consumer reads it with.

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

/// The manifest `sce-codegen <args> <document> -l rust` writes for `body`,
/// validated against the checked-in schema before it is returned.
fn manifest(label: &str, subcommand: &str, body: &str) -> serde_json::Value {
    let dir = scratch(&format!("{label}-{subcommand}"));
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    let mut command = Command::new(sce_codegen_bin());
    command
        .args(["--error-format", "json", subcommand])
        .arg(&path)
        .args(["-l", "rust"]);
    if subcommand == "generate" {
        command.arg("-o").arg(dir.join("out"));
    }
    let out = command.output().expect("run sce-codegen");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{label} is a valid document and must {subcommand}: {}",
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

/// Both single-document producers publish the reading, in one shape.
fn both(label: &str, body: &str) -> [serde_json::Value; 2] {
    ["check", "generate"]
        .map(|subcommand| manifest(label, subcommand, body)["document_kind"].clone())
}

const TRANSFORM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="scaled">
  <datamodel>
    <data id="raw" sce:type="uint16" sce:direction="in"/>
    <data id="scaled" sce:type="float64" sce:direction="out" expr="raw * 0.5"/>
  </datamodel>
</scxml>
"#;

const UNDECLARED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="undeclared" initial="idle">
  <state id="idle"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"#;

const DECLARED_STATECHART: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="statechart" version="1.0" name="declared" initial="idle">
  <state id="idle"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"#;

/// A forge kind is read as the kind it declares.
#[test]
fn a_declared_forge_kind_is_published_as_declared() {
    for reading in both("scaled", TRANSFORM) {
        assert_eq!(
            reading,
            serde_json::json!({"name": "transform", "declared": true})
        );
    }
}

/// The default is named as the default: the same statechart reading, with
/// `declared` the one bit that tells the two documents apart.
#[test]
fn the_statechart_default_is_published_as_undeclared() {
    for reading in both("undeclared", UNDECLARED) {
        assert_eq!(
            reading,
            serde_json::json!({"name": "statechart", "declared": false})
        );
    }
    for reading in both("declared", DECLARED_STATECHART) {
        assert_eq!(
            reading,
            serde_json::json!({"name": "statechart", "declared": true})
        );
    }
}

/// A `kind` attribute outside the SCE namespace is not a declaration. The
/// product reads the document as a statechart, and the manifest says the
/// kind was not declared rather than echoing what the author typed.
#[test]
fn a_kind_outside_the_sce_namespace_is_not_a_declaration() {
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="urn:example:not-sce"
       x:kind="transform" version="1.0" name="elsewhere" initial="idle">
  <state id="idle"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"#;
    assert_eq!(
        manifest("elsewhere", "check", body)["document_kind"],
        serde_json::json!({"name": "statechart", "declared": false})
    );
}
