// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The manifest names the markers a document carries.
//!
//! # What was wrong
//!
//! A `sce:unresolved` marker blocks only `--strict-unresolved`. That is the
//! right rule for a draft still being written, and it meant a run over a
//! document with an open question ended exactly like a run over one with
//! none: measured 2026-09-30, a draft that left a retry count
//! `sce:unresolved` was reported `accepted` by `check --lint`, with nothing
//! in the manifest to say a question was still open. An author reading only
//! the verdict, as a specification owner working through the authoring MCP
//! does, saw a finished document.
//!
//! So the markers are published: `unresolved` lists what
//! `sce-codegen unresolved` prints, for a statechart and for a forge kind,
//! in one shape. These cases hold it to the document, to the command that
//! already lists them, and to the wire schema a consumer reads it with.

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

/// The manifest `sce-codegen <subcommand> <document> -l rust` writes for
/// `body`, validated against the checked-in schema before it is returned.
fn manifest(label: &str, subcommand: &str, extra: &[&str], body: &str) -> serde_json::Value {
    let dir = scratch(&format!("{label}-{subcommand}"));
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    let mut command = Command::new(sce_codegen_bin());
    command
        .args(["--error-format", "json", subcommand])
        .arg(&path)
        .args(["-l", "rust"])
        .args(extra);
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

/// What `sce-codegen unresolved` prints for `body`: one JSON record a line.
fn listed(label: &str, body: &str) -> Vec<serde_json::Value> {
    let dir = scratch(&format!("{label}-unresolved"));
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    let out = Command::new(sce_codegen_bin())
        .args(["unresolved"])
        .arg(&path)
        .output()
        .expect("run sce-codegen unresolved");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("one record a line"))
        .collect()
}

/// The retried draft: a question the specification left open, a value
/// chosen without it, and a clean rest.
const OPEN_QUESTION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="retry_open" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="limit" expr="3" sce:assumed="retry-limit"
          sce:assumed-reason="the specification gives no number"
          sce:assumed-candidates="3 5"/>
  </datamodel>
  <state id="idle">
    <transition event="request" target="waiting"/>
  </state>
  <state id="waiting">
    <transition event="response" target="idle"/>
    <transition event="timeout" target="idle" sce:unresolved="retry-count"
                sce:unresolved-reason="the specification does not say how many retries"/>
  </state>
</scxml>
"#;

const CLEAN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="clean" initial="idle">
  <state id="idle"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"#;

const TRANSFORM_OPEN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="scaled">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="raw * 2"
          sce:assumed="gain" sce:assumed-reason="the gain is not stated"
          sce:assumed-candidates="2 3"/>
  </datamodel>
</scxml>
"#;

/// The records of `unresolved`, each `location.file` reduced to its file
/// name. Every run here writes the document into a directory of its own, and
/// the record carries the path it was read from, so a comparison across runs
/// has to compare the document and not the directory.
fn records(list: &serde_json::Value) -> Vec<serde_json::Value> {
    list.as_array()
        .expect("unresolved is a list")
        .iter()
        .cloned()
        .map(|mut record| {
            if let Some(file) = record.pointer_mut("/location/file") {
                let name = Path::new(file.as_str().expect("file is text"))
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                *file = serde_json::Value::String(name);
            }
            record
        })
        .collect()
}

fn ids(manifest: &serde_json::Value) -> Vec<(String, String)> {
    manifest["unresolved"]
        .as_array()
        .map(|records| {
            records
                .iter()
                .map(|r| {
                    (
                        r["kind"].as_str().expect("kind").to_string(),
                        r["id"].as_str().expect("id").to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The measured case: `check --lint` succeeds, and its manifest says what is
/// still open.
#[test]
fn a_run_that_succeeds_says_which_questions_are_still_open() {
    let checked = manifest("open", "check", &["--lint"], OPEN_QUESTION);
    assert_eq!(
        ids(&checked),
        vec![
            ("assumed".to_string(), "retry-limit".to_string()),
            ("unresolved".to_string(), "retry-count".to_string()),
        ],
        "{checked}"
    );
}

/// `check` and `generate` are contracted to reach one verdict, so they
/// publish one list.
#[test]
fn check_and_generate_publish_the_same_list() {
    let checked = manifest("open", "check", &[], OPEN_QUESTION);
    let generated = manifest("open", "generate", &[], OPEN_QUESTION);
    assert_eq!(
        records(&checked["unresolved"]),
        records(&generated["unresolved"])
    );
    assert!(!ids(&checked).is_empty());
}

/// The manifest and the command that already lists the markers are built by
/// one reader, so they cannot disagree about which markers a document has.
#[test]
fn the_manifest_lists_what_the_unresolved_command_prints() {
    let checked = manifest("open", "check", &[], OPEN_QUESTION);
    let printed = listed("open", OPEN_QUESTION);
    assert!(!printed.is_empty());
    assert_eq!(
        records(&checked["unresolved"]),
        records(&serde_json::Value::Array(printed)),
        "{checked}"
    );
}

/// A forge kind reads its markers through its own channel, and publishes
/// them in the same shape.
#[test]
fn a_forge_document_publishes_its_markers_too() {
    let checked = manifest("scaled", "check", &[], TRANSFORM_OPEN);
    assert_eq!(
        ids(&checked),
        vec![("assumed".to_string(), "gain".to_string())],
        "{checked}"
    );
    assert_eq!(
        records(&checked["unresolved"]),
        records(&serde_json::Value::Array(listed("scaled", TRANSFORM_OPEN))),
    );
}

/// The control: a document that marks nothing has no field, not an empty
/// one, so a manifest that is unchanged for every existing document stays
/// byte for byte what it was.
#[test]
fn a_document_that_marks_nothing_has_no_field() {
    let checked = manifest("clean", "check", &["--lint"], CLEAN);
    assert!(checked.get("unresolved").is_none(), "{checked}");
}

/// The strict check still refuses the same document: publishing the marker
/// does not make it non-blocking where it blocked before.
#[test]
fn the_strict_check_still_refuses_an_open_question() {
    let dir = scratch("strict");
    let path = dir.join("open.scxml");
    std::fs::write(&path, OPEN_QUESTION).expect("write fixture");
    let out = Command::new(sce_codegen_bin())
        .args(["--error-format", "json", "check", "--strict-unresolved"])
        .arg(&path)
        .args(["-l", "rust"])
        .output()
        .expect("run sce-codegen");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("validation/unresolved-placeholder"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
