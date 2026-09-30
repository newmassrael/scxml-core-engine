// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A design the product accepts says what it still leaves to a person, on
//! every surface an owner reads it from.
//!
//! # What was wrong
//!
//! `accepted` means the product found nothing to refuse. Drafts an owner
//! was shown as `accepted` were not finished: one left a count
//! `sce:unresolved`, one sent an output to `#_parent` where nothing invokes
//! it, one sent it to the machine itself. The acceptance report — the page
//! an owner signs — showed none of it, and the record kept none of it, so a
//! design accepted with a question open read later as a design that had
//! none.
//!
//! # One list, three readers
//!
//! `sce_build::open_matters` writes the sentences once. These cases hold
//! the three surfaces to it, through the binary a host runs: the manifest's
//! `open`, the top of the report's block B, and the record's
//! `open_at_acceptance` — and hold the control that matters as hard: a
//! design that leaves nothing open gets no new line, no new field.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

const MANIFEST: &str = "spec/manifest.json";
const RECORD: &str = "acceptance/base.json";

/// A draft with a question the specification leaves open, a value chosen
/// without it, and a send to a parent nothing here invokes.
const OPEN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="gate" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="limit" expr="3" sce:assumed="wait-limit"
          sce:assumed-reason="the specification gives no number"
          sce:assumed-candidates="3 5"/>
  </datamodel>
  <state id="idle">
    <transition event="start" target="waiting"/>
  </state>
  <state id="waiting">
    <onentry><send event="announce" target="#_parent"/></onentry>
    <transition event="finish" target="idle"/>
    <transition event="timeout" target="idle" sce:unresolved="timeout-policy"
                sce:unresolved-reason="the specification does not say what a timeout does"/>
  </state>
</scxml>
"##;

const FINISHED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="finished" initial="idle">
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

fn design_root(document: &str, body: &str) -> tempfile::TempDir {
    let root = tempfile::TempDir::new().expect("tempdir");
    let put = |rel: &str, bytes: &[u8]| {
        let path = root.path().join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        fs::write(path, bytes).expect("write");
    };
    put(
        MANIFEST,
        &fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "tests/fixtures/requirement_closure/iso13400_2_nl_socket_handling.manifest.json",
        ))
        .expect("the committed manifest is readable"),
    );
    put(document, body.as_bytes());
    fs::create_dir_all(root.path().join("acceptance")).expect("mkdir");
    root
}

fn run(args: &[&str], root: &Path) -> Output {
    Command::new(CODEGEN)
        .arg("--error-format=json")
        .args(args)
        .current_dir(root)
        .output()
        .expect("spawn sce-codegen")
}

fn ok(out: Output, what: &str) -> Output {
    assert!(
        out.status.success(),
        "{what}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn report(root: &Path, document: &str) -> String {
    let out = ok(
        run(
            &[
                "acceptance-report",
                document,
                "--manifest",
                MANIFEST,
                "--variant",
                "base",
            ],
            root,
        ),
        "the report",
    );
    String::from_utf8(out.stdout).expect("utf-8")
}

fn accepted(root: &Path, document: &str) -> serde_json::Value {
    ok(
        run(
            &[
                "accept",
                document,
                "--manifest",
                MANIFEST,
                "--variant",
                "base",
                "--root",
                &root.display().to_string(),
                "--out",
                RECORD,
            ],
            root,
        ),
        "accept",
    );
    serde_json::from_str(&fs::read_to_string(root.join(RECORD)).expect("the record"))
        .expect("the record is JSON")
}

fn manifest_open(root: &Path, document: &str) -> Vec<(String, String)> {
    let out = ok(run(&["check", document, "-l", "rust"], root), "check");
    let manifest: serde_json::Value = serde_json::from_slice(&out.stdout).expect("one JSON line");
    manifest["open"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|m| {
                    (
                        m["kind"].as_str().expect("kind").to_string(),
                        m["message"].as_str().expect("message").to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

const KINDS: [&str; 3] = ["question", "assumed", "parent"];

/// The report says it, first in block B, where a person signing it reads
/// for what could still be wrong.
#[test]
fn the_report_lists_what_the_design_leaves_open_at_the_top_of_block_b() {
    let root = design_root("d/gate.scxml", OPEN);
    let page = report(root.path(), "d/gate.scxml");
    let block_b = page
        .split("B. NEEDING ATTENTION\n")
        .nth(1)
        .expect("a block B")
        .split("\nC. ")
        .next()
        .expect("block B ends at C");
    let lines: Vec<&str> = block_b.lines().collect();
    assert!(lines[0].starts_with("  open question"), "{block_b}");
    assert!(lines[0].contains("timeout-policy"), "{block_b}");
    assert!(lines[1].starts_with("  assumed value"), "{block_b}");
    assert!(lines[1].contains("wait-limit"), "{block_b}");
    assert!(lines[2].starts_with("  needs a parent"), "{block_b}");
    assert!(lines[2].contains("announce"), "{block_b}");
}

/// The control the design turns on: a design that leaves nothing open gets
/// nothing added, so a report that was complete is byte for byte what it was.
#[test]
fn a_finished_design_gets_no_new_line() {
    let root = design_root("d/finished.scxml", FINISHED);
    let page = report(root.path(), "d/finished.scxml");
    for label in [
        "open question",
        "assumed value",
        "needs a parent",
        "host processor",
    ] {
        assert!(
            !page.contains(label),
            "{label} on a finished design:\n{page}"
        );
    }
}

/// The manifest, the report and the record are one list: the same words.
#[test]
fn the_manifest_the_report_and_the_record_say_the_same_words() {
    let root = design_root("d/gate.scxml", OPEN);
    let from_manifest = manifest_open(root.path(), "d/gate.scxml");
    let kinds: Vec<&str> = from_manifest.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kinds, KINDS, "{from_manifest:?}");

    let page = report(root.path(), "d/gate.scxml");
    let record = accepted(root.path(), "d/gate.scxml");
    let kept: Vec<String> = record["open_at_acceptance"]
        .as_array()
        .expect("the record keeps what was open")
        .iter()
        .map(|m| m["message"].as_str().expect("message").to_string())
        .collect();
    let said: Vec<String> = from_manifest.iter().map(|(_, m)| m.clone()).collect();
    assert_eq!(kept, said, "the record's words differ from the manifest's");
    for message in &said {
        assert!(page.contains(message), "the report lacks: {message}");
    }
}

/// A record for a finished design has no new field, so it keeps the bytes
/// it always had.
#[test]
fn a_record_for_a_finished_design_has_no_open_field() {
    let root = design_root("d/finished.scxml", FINISHED);
    let record = accepted(root.path(), "d/finished.scxml");
    assert!(record.get("open_at_acceptance").is_none(), "{record}");
    assert!(manifest_open(root.path(), "d/finished.scxml").is_empty());
}

/// Accepting is not refused for an open question: that is the owner's
/// decision, and the record says they made it.
#[test]
fn accepting_with_a_question_open_is_recorded_not_refused() {
    let root = design_root("d/gate.scxml", OPEN);
    let record = accepted(root.path(), "d/gate.scxml");
    assert_eq!(record["record"], "sce-acceptance-record");
    assert!(!record["open_at_acceptance"]
        .as_array()
        .expect("kept")
        .is_empty());
    // And the record reads back: what this module wrote it can read.
    let held = run(
        &[
            "acceptance-check",
            RECORD,
            "--variant",
            "base",
            "--root",
            &root.path().display().to_string(),
        ],
        root.path(),
    );
    assert!(
        held.status.success(),
        "{}",
        String::from_utf8_lossy(&held.stderr)
    );
}

/// A forge kind sends to no parent and no processor; what it leaves open is
/// its markers, and it says so on the same three surfaces.
#[test]
fn a_forge_kind_says_it_too() {
    let root = design_root("d/scaled.scxml", TRANSFORM_OPEN);
    let open = manifest_open(root.path(), "d/scaled.scxml");
    assert_eq!(open.len(), 1, "{open:?}");
    assert_eq!(open[0].0, "assumed");
    let page = report(root.path(), "d/scaled.scxml");
    assert!(page.contains("assumed value"), "{page}");
    assert!(page.contains(&open[0].1), "{page}");
    let record = accepted(root.path(), "d/scaled.scxml");
    assert_eq!(
        record["open_at_acceptance"][0]["message"].as_str(),
        Some(open[0].1.as_str())
    );
}

/// A run that reads several documents has no single answer: each member has
/// its own, and a union would name a parent one of them may not lack.
///
/// The member that leaves a question open sends to no parent, so the set
/// passes the check — a set with a sender to a parent nothing invokes is
/// refused outright, which is a different answer and is not this one.
#[test]
fn a_document_set_publishes_no_open_list() {
    let question_only = OPEN.replace(
        r##"<onentry><send event="announce" target="#_parent"/></onentry>"##,
        "",
    );
    let root = design_root("d/gate.scxml", &question_only);
    fs::write(root.path().join("d/finished.scxml"), FINISHED).expect("write");
    // The control: alone, the same document does say what it leaves open.
    assert!(
        !manifest_open(root.path(), "d/gate.scxml").is_empty(),
        "the member leaves nothing open, so the set proves nothing"
    );
    let out = ok(
        run(
            &[
                "check",
                "--document",
                "d/gate.scxml",
                "--document",
                "d/finished.scxml",
            ],
            root.path(),
        ),
        "check of a set",
    );
    let manifest: serde_json::Value = serde_json::from_slice(&out.stdout).expect("one JSON line");
    assert!(manifest.get("open").is_none(), "{manifest}");
}
