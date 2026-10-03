// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The owner's acceptance flow answers for a design of any kind.
//!
//! # What was wrong
//!
//! `requirements --manifest`, `acceptance-report`, `accept` and
//! `acceptance-check` read their document through the SCXML parser, which
//! refuses every kind but statechart as `validation/wrong-pipeline`. An
//! owner reviewing a lookup, a transform or a codec through the authoring
//! MCP could see its pseudocode and could not accept it, and `diagram`
//! answered a transform with a pipeline error rather than with where the
//! transform is reviewed.
//!
//! These cases hold the forge route on the binary: the classification is
//! the statechart's (one coverage implementation, Requirement-closure RFC
//! §7.1), the report shows each requirement's own review-table line, a
//! kind that carries `sce:req` on no node says so at the top of its page,
//! the record pins what the document imports and lapses when it moves,
//! and a figure is the kind's own picture beside the table of its values.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir.canonicalize().expect("canonical scratch dir")
}

fn run(args: &[&str], cwd: &Path) -> Output {
    Command::new(sce_codegen_bin())
        .args(["--error-format", "json"])
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run sce-codegen")
}

fn stdout(out: &Output) -> String {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout.clone()).expect("utf-8")
}

const LOOKUP: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="severity">
  <datamodel>
    <data id="code" sce:type="uint8" sce:direction="in"/>
    <data id="severity" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="UNKNOWN">
      <sce:entry key="0" value="OK" sce:req="REQ-1"/>
      <sce:entry key="1" value="WARN" sce:req="REQ-2"/>
      <sce:entry key="2" value="FAIL"/>
      <sce:entry key="9" value="UNKNOWN"/>
    </data>
  </datamodel>
</scxml>
"#;

const TRANSFORM: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="volts">
  <datamodel>
    <data id="count" sce:type="uint16" sce:direction="in"/>
    <data id="volts" sce:type="float64" sce:direction="out" expr="count * 3.3 / 4095.0"/>
  </datamodel>
</scxml>
"#;

const MANIFEST: &str = r#"{
  "doc_id": "SPEC-9",
  "rev": "1",
  "extraction": {"ids": "native", "trace": "none",
                 "modality_convention": "english-modal-verbs", "method": "hand"},
  "requirements": [
    {"id": "REQ-1"},
    {"id": "REQ-2"},
    {"id": "REQ-3"}
  ]
}
"#;

fn stage(dir: &Path, name: &str, text: &str) -> String {
    std::fs::write(dir.join(name), text).expect("stage");
    name.to_string()
}

/// The classification a lookup gets is the statechart's: annotated
/// requirements implemented, the unannotated one missing.
#[test]
fn a_lookup_is_classified_by_the_entries_that_claim_requirements() {
    let dir = scratch("any-kind-classify");
    let doc = stage(&dir, "severity.scxml", LOOKUP);
    let manifest = stage(&dir, "spec.manifest.json", MANIFEST);
    let records = stdout(&run(&["requirements", &doc, "--manifest", &manifest], &dir));
    let outcome = |id: &str| {
        records
            .lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("NDJSON"))
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("{id} in {records}"))["outcome"]
            .clone()
    };
    assert_eq!(outcome("REQ-1"), "implemented");
    assert_eq!(outcome("REQ-2"), "implemented");
    assert_eq!(outcome("REQ-3"), "missing");

    // Without a manifest: one record per claiming entry, in the shape a
    // statechart's records take.
    let claims = stdout(&run(&["requirements", &doc], &dir));
    assert_eq!(claims.lines().count(), 2, "{claims}");
    assert!(
        claims.contains("\"requirement_ids\":[\"REQ-1\"]"),
        "{claims}"
    );
}

/// The report shows each requirement's own line of the review table, and
/// a kind whose nodes carry no `sce:req` says so before anything else.
#[test]
fn the_report_shows_what_each_requirement_rests_on() {
    let dir = scratch("any-kind-report");
    let lookup = stage(&dir, "severity.scxml", LOOKUP);
    let transform = stage(&dir, "volts.scxml", TRANSFORM);
    let manifest = stage(&dir, "spec.manifest.json", MANIFEST);

    let page = stdout(&run(
        &[
            "acceptance-report",
            &lookup,
            "--manifest",
            &manifest,
            "--variant",
            "base",
        ],
        &dir,
    ));
    assert!(page.contains("kind: lookup"), "{page}");
    assert!(page.contains("0 -> OK"), "{page}");
    assert!(page.contains("missing        REQ-3"), "{page}");

    let page = stdout(&run(
        &[
            "acceptance-report",
            &transform,
            "--manifest",
            &manifest,
            "--variant",
            "base",
        ],
        &dir,
    ));
    assert!(
        page.contains("SCE reads sce:req on no node of a transform document yet"),
        "{page}"
    );
}

/// A forge design is accepted, and the acceptance lapses when the
/// document moves.
#[test]
fn a_forge_design_is_accepted_and_lapses_when_it_moves() {
    let dir = scratch("any-kind-accept");
    let doc = stage(&dir, "volts.scxml", TRANSFORM);
    let manifest = stage(&dir, "spec.manifest.json", MANIFEST);
    stdout(&run(
        &[
            "accept",
            &doc,
            "--manifest",
            &manifest,
            "--variant",
            "base",
            "--root",
            ".",
            "--out",
            "acceptance.json",
        ],
        &dir,
    ));
    let check = || {
        run(
            &[
                "acceptance-check",
                "acceptance.json",
                "--variant",
                "base",
                "--root",
                ".",
            ],
            &dir,
        )
    };
    assert!(
        check().status.success(),
        "{}",
        String::from_utf8_lossy(&check().stderr)
    );

    std::fs::write(dir.join("volts.scxml"), TRANSFORM.replace("3.3", "5.0")).expect("edit");
    let lapsed = check();
    assert!(!lapsed.status.success(), "a moved design must lapse");
    assert!(
        String::from_utf8_lossy(&lapsed.stderr).contains("cli/acceptance-lapsed"),
        "{}",
        String::from_utf8_lossy(&lapsed.stderr)
    );
}

/// A statechart's record pins the event schema it imports: editing the
/// schema moves what the acceptance rested on. Before the forge route was
/// written, the record did not name the schema at all.
#[test]
fn a_statechart_record_pins_the_schema_it_imports() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/event_schema");
    let dir = scratch("any-kind-import");
    for name in [
        "mesh_receiver_matched.scxml",
        "schema_job_completed_minimal.scxml",
    ] {
        std::fs::copy(fixtures.join(name), dir.join(name)).expect("copy fixture");
    }
    let manifest = stage(&dir, "spec.manifest.json", MANIFEST);
    stdout(&run(
        &[
            "accept",
            "mesh_receiver_matched.scxml",
            "--manifest",
            &manifest,
            "--variant",
            "base",
            "--root",
            ".",
            "--out",
            "acceptance.json",
        ],
        &dir,
    ));
    let record = std::fs::read_to_string(dir.join("acceptance.json")).expect("record");
    assert!(
        record.contains("schema_job_completed_minimal.scxml"),
        "{record}"
    );
}

/// A statechart is boxes and arrows; any other kind is drawn as its own
/// picture and the table of every value it states (a transform: its inputs,
/// its expression, its outputs), and is not refused as the wrong pipeline.
#[test]
fn a_figure_of_a_forge_document_is_its_picture_and_its_table() {
    let dir = scratch("any-kind-diagram");
    let doc = stage(&dir, "volts.scxml", TRANSFORM);
    let out = run(&["diagram", &doc, "-o", "figs"], &dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = written
        .lines()
        .filter_map(|l| std::path::Path::new(l).file_name()?.to_str())
        .collect();
    assert_eq!(names, ["dataflow.svg", "fields-1.svg"], "{written}");
    for name in names {
        let svg = std::fs::read_to_string(dir.join("figs").join(name)).expect("written");
        assert!(svg.starts_with("<svg"), "{name}");
        // Named by the file it came from, as every figure is.
        assert!(svg.contains("volts"), "{name} names its document");
    }
}
