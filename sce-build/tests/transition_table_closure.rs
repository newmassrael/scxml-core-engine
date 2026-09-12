//! Requirement-closure RFC ② — Atomic B.
//!
//! `sce-codegen transition-table <doc.scxml>` emits the table whose
//! `source` column makes it a trace table. Two readings, and this file
//! demonstrates each with a non-empty example:
//!
//!   (a) rows whose `source` is `(none)` — behaviour the specification
//!       never asked for, which is the fifth case of RFC §5.3 and the
//!       one no id-based check can see;
//!   (b) requirements the manifest declares that no row claims — the
//!       same `missing` Atomic A reports, derived the other way.
//!
//! # What makes (b) worth asserting rather than assuming
//!
//! RFC §6.2 closes by claiming ① and ② "stop being separate features
//! and become two readings of one export". That is a claim about
//! agreement, and a claim about agreement is exactly the kind that
//! stays true in prose while drifting in code. So this file does not
//! merely check that the table can name missing requirements — it
//! checks that the set it names is **the same set** the classifier
//! calls `missing`. If those two ever diverge the sentence in the RFC
//! becomes false, and one of the two answers is wrong without anyone
//! being told which.
//!
//! ⚠ Fixtures are synthetic. ATOMIC-D is where a real standard arrives.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::{classify, Modality, Outcome, RequirementManifest};
use sce_build::transition_table::{
    requirements_without_a_row, transition_table, EMPTY_CELL, NO_SOURCE,
};

/// Two claimed transitions, one unclaimed transition, a claimed entry
/// action, and a manifest requirement nothing implements.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                            xmlns:sce="http://sce.dev/ext"
                            version="1.0" initial="idle" datamodel="null">
  <state id="idle" sce:req="REQ-001">
    <transition event="btn_press" target="emergency" sce:req="REQ-002"/>
    <transition event="diag_req" target="diagnostic"/>
  </state>
  <state id="emergency">
    <onentry sce:req="REQ-003">
      <send event="drive_off" delay="3s"/>
    </onentry>
    <!-- `In()` rather than a bare expression: W3C SCXML B.1.2
         withholds a boolean expression language from datamodel="null",
         and the parser enforces it. The guard column only needs a
         non-empty `cond` to be exercised. -->
    <transition event="reset" cond="In('diagnostic')" target="idle"/>
  </state>
  <state id="diagnostic"/>
</scxml>"#;

const MANIFEST: &str = r#"{
  "doc_id": "car-body-spec",
  "rev": "D3",
  "extraction": {
    "ids": "native", "trace": "none",
    "modality_convention": "english-modal-verbs", "method": "hand"
  },
  "sections": [{ "id": "3.1", "title": "Start procedure" }],
  "requirements": [
    { "id": "REQ-001", "section": "3.1", "page": 12 },
    { "id": "REQ-002", "section": "3.1", "page": 41 },
    { "id": "REQ-003", "section": "3.1", "page": 42 },
    { "id": "REQ-004", "section": "3.1", "page": 55 }
  ]
}"#;

fn parse(scxml: &str, label: &str) -> sce_build::model::SCXMLModel {
    SCXMLParser::new()
        .parse_string(scxml, label)
        .unwrap_or_else(|e| panic!("fixture {label} must parse: {:?}", e.error))
}

fn manifest(raw: &str, label: &str) -> RequirementManifest {
    RequirementManifest::from_json(raw, label)
        .unwrap_or_else(|e| panic!("fixture manifest {label} must load: {e}"))
}

#[test]
fn the_table_carries_the_seven_columns_of_the_trace_table() {
    let rows = transition_table(&parse(DOC, "table_doc"));

    let btn = rows
        .iter()
        .find(|row| row.event == "btn_press")
        .expect("the claimed transition is in the table");
    assert_eq!(btn.source, "REQ-002");
    assert_eq!(btn.from, "idle");
    assert_eq!(btn.to, "emergency");
    assert_eq!(btn.guard, "-");
    assert_eq!(btn.node_path, "states.idle.transitions[0]");

    let guarded = rows
        .iter()
        .find(|row| row.event == "reset")
        .expect("the guarded transition is in the table");
    assert_eq!(
        guarded.guard, "In('diagnostic')",
        "the guard column carries `cond` verbatim",
    );

    // `after` rides the action that actually holds the delay. The IR
    // has no delay on a transition, so synthesising one there would be
    // inventing a column value the document never wrote.
    let entry = rows
        .iter()
        .find(|row| row.event == "(entry)")
        .expect("the entry action is in the table");
    assert_eq!(entry.source, "REQ-003");
    assert_eq!(entry.from, "emergency");
    assert_eq!(entry.after, "3s", "verbatim, not resolved to milliseconds");
    assert_eq!(entry.action, "send");
}

/// (a) The `(none)` block is non-empty.
#[test]
fn behaviour_the_specification_never_asked_for_collects_under_none() {
    let rows = transition_table(&parse(DOC, "table_doc"));
    let unclaimed: Vec<&str> = rows
        .iter()
        .filter(|row| row.is_unclaimed())
        .map(|row| row.event.as_str())
        .collect();

    assert!(
        unclaimed.contains(&"diag_req"),
        "a transition carrying no `sce:req` must appear under `{NO_SOURCE}` — \
         it is behaviour the specification never asked for, and it is \
         invisible to every id-based check. got: {unclaimed:?}",
    );
    assert!(
        !unclaimed.is_empty(),
        "the `{NO_SOURCE}` block is empty, so this reading demonstrates nothing",
    );
}

/// (b) A manifest requirement with no row is named, and it is the same
/// set the classifier calls `missing`.
#[test]
fn a_requirement_with_no_row_is_the_same_missing_the_classifier_reports() {
    let model = parse(DOC, "table_doc");
    let declared = manifest(MANIFEST, "table_manifest");
    let rows = transition_table(&model);

    let from_table: BTreeSet<String> = requirements_without_a_row(
        &rows,
        declared.requirements.iter().map(|entry| entry.id.as_str()),
    )
    .into_iter()
    .collect();
    assert_eq!(
        from_table,
        BTreeSet::from(["REQ-004".to_string()]),
        "the table must name the declared requirement nothing implements",
    );

    let from_classifier: BTreeSet<String> = classify(&model, &declared)
        .outcomes
        .iter()
        .filter(|outcome| outcome.outcome == Outcome::Missing)
        .map(|outcome| outcome.id.clone())
        .collect();

    // RFC §6.2's closing claim, asserted rather than trusted.
    assert_eq!(
        from_table, from_classifier,
        "the table and the classifier disagree about `missing`. They read \
         one walk, so a disagreement means one of them is answering about \
         a document the other did not see",
    );
    assert!(
        !from_table.is_empty(),
        "both agree on the empty set, which is agreement about nothing",
    );
}

/// Every node the classifier can call `implemented` has a row.
///
/// This is the property that makes the (b) reading sound, and it is
/// why the table covers states and invokes rather than transitions
/// alone: a requirement carried on a `<state>` would otherwise have no
/// row, and "no row" would report it `missing` while the classifier
/// called it `implemented`. RFC §6.2's example table shows only
/// transitions and one entry row, which is where that gap came from.
#[test]
fn a_requirement_carried_on_a_state_still_has_a_row() {
    let rows = transition_table(&parse(DOC, "table_doc"));
    let state_row = rows
        .iter()
        .find(|row| row.node_path == "states.idle")
        .expect("the annotated state has a row");
    assert_eq!(state_row.source, "REQ-001");
    assert_eq!(state_row.event, "(state)");
}

#[test]
fn the_command_emits_the_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = dir.path().join("doc.scxml");
    std::fs::write(&doc, DOC).expect("write fixture document");

    let output = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("transition-table")
        .arg(&doc)
        .output()
        .expect("sce-codegen runs");
    assert!(
        output.status.success(),
        "sce-codegen transition-table failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(
        stdout.contains("\"source\":\"REQ-002\"") && stdout.contains("\"source\":\"(none)\""),
        "the command must emit both a claimed and an unclaimed row. stdout:\n{stdout}",
    );
    // ⚠ Presence of the KEY is not presence of the column. `"guard":`
    // stays in the output when every row's guard is `-`, so this loop
    // used to pass over a table that had stopped reporting a whole
    // dimension. Each column is now required to carry a value on some
    // row: delete the dashed cells, and the column must still be there.
    for column in ["from", "event", "guard", "after", "to", "action"] {
        let key = format!("\"{column}\":\"");
        assert!(
            stdout.contains(&key),
            "column `{column}` is absent from the emitted table. stdout:\n{stdout}",
        );
        let remaining = stdout
            .replace(&format!("\"{column}\":\"{EMPTY_CELL}\""), "")
            .replace(&format!("\"{column}\":\"{NO_SOURCE}\""), "");
        assert!(
            remaining.contains(&key),
            "every emitted `{column}` cell is `{EMPTY_CELL}`; the key is \
             on the wire but the column carries nothing. stdout:\n{stdout}",
        );
    }
}

/// The closing count: what was examined, printed, with a floor.
///
/// ⚠ Without the floor this file passes having examined nothing — an
/// empty table and a clean one print the same green, which is the trap
/// this repository has been caught by before.
#[test]
fn the_sweep_reports_what_it_examined_and_asserts_a_floor() {
    let corpus: Vec<(&str, &str, Option<&str>)> = vec![
        ("table_doc", DOC, Some(MANIFEST)),
        (
            "second_doc",
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                      xmlns:sce="http://sce.dev/ext"
                      version="1.0" initial="s0" datamodel="null">
                 <state id="s0">
                   <transition event="go" target="s1" sce:req="REQ-A"/>
                   <transition event="stop" target="s0"/>
                 </state>
                 <state id="s1">
                   <onexit><raise event="left"/></onexit>
                 </state>
               </scxml>"#,
            Some(
                r#"{ "doc_id": "other-spec", "rev": "A",
                     "extraction": { "ids": "native", "trace": "none",
                                     "modality_convention": "english-modal-verbs",
                                     "method": "hand" },
                     "requirements": [
                       { "id": "REQ-A" }, { "id": "REQ-GONE" } ] }"#,
            ),
        ),
    ];

    let mut documents = 0usize;
    let mut rows_examined = 0usize;
    let mut unclaimed = 0usize;
    let mut without_a_row = 0usize;

    for (label, scxml, manifest_json) in &corpus {
        let model = parse(scxml, label);
        let rows = transition_table(&model);
        documents += 1;
        rows_examined += rows.len();
        unclaimed += rows.iter().filter(|row| row.is_unclaimed()).count();
        if let Some(raw) = manifest_json {
            let declared = manifest(raw, label);
            // `shall` entries only — "no row means missing" is the
            // presence question, and a `shall_not` entry is exactly the
            // one it may not be asked of. Both fixtures here are
            // `shall`, so the filter changes nothing today; it is
            // written out because the call is the contract's example
            // and an example that drops the filter teaches the wrong
            // call to whoever copies it next.
            without_a_row += requirements_without_a_row(
                &rows,
                declared
                    .requirements
                    .iter()
                    .filter(|entry| entry.modality == Modality::Shall)
                    .map(|entry| entry.id.as_str()),
            )
            .len();
        }
    }

    println!(
        "transition table: examined {rows_examined} row(s) across {documents} \
         document(s); {unclaimed} unclaimed, {without_a_row} declared \
         requirement(s) with no row"
    );

    assert!(
        documents >= 2,
        "examined {documents} document(s); one document cannot show the \
         table works on anything but the fixture it was shaped around",
    );
    assert!(
        rows_examined >= 10,
        "examined {rows_examined} row(s), too few to mean anything — the \
         corpus stopped producing them",
    );
    assert!(
        unclaimed > 0,
        "no `{NO_SOURCE}` row anywhere in the corpus; reading (a) has no \
         non-empty example",
    );
    assert!(
        without_a_row > 0,
        "no declared requirement lacked a row anywhere in the corpus; \
         reading (b) has no non-empty example",
    );
}
