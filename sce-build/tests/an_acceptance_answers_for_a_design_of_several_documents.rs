// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An acceptance answers for a design of several documents.
//!
//! `requirements A B C --manifest m` measured the documents of one design
//! together, and `acceptance-report` and `accept` took one document: a
//! requirement claimed in an algorithm the statechart imports read "this
//! document annotates nothing for it" on the page a person signs, while
//! `accept` pinned that algorithm's bytes as an input of the design. The page
//! and the record now take the set `requirements` takes, read by one reader
//! (`sce_build::design_set`): several documents, or one with `--with-imports`
//! and every document it reaches.
//!
//! What these hold, each by a case that fails when it stops being true:
//!   * a claim made in an imported document, one import down or two, is
//!     credited on the page, by `requirements` and in the record
//!   * a change to any input of the set, an import two documents down among
//!     them, lapses the acceptance, and a document of the set that is gone does
//!   * several documents named are one acceptance, and a claim in any of them is
//!     the design's
//!   * a standard document the design reaches is no member of it
//!   * `acceptance-delta` reads a record of a set as a set, and refuses a draft
//!     of one document offered against it
//!   * one entry document named alone answers exactly as it did: the same page,
//!     the same record bytes, no claim credited from what it imports

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::acceptance_record::AcceptanceRecord;
use sce_build::design_set::DesignSet;

/// A statechart that imports a local algorithm (which imports another), an
/// event schema, an enum, and two standard documents; it claims REQ-1 itself.
const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="sce-static" name="closure_machine">
  <sce:import kind="event-schema" src="schema_pick.scxml" as="Pick"/>
  <sce:import kind="enum" src="enum_level.scxml" as="Level"/>
  <sce:import kind="algorithm" src="algorithm_outer.scxml" as="OuterPick"/>
  <sce:import kind="algorithm" src="sce:std/time/days_from_civil.scxml" as="DaysFromCivil"/>
  <sce:import kind="enum" src="sce:std/mesh/rpc_status.scxml" as="Status"/>
  <datamodel>
    <data id="shownLevel" sce:type="enum:Level" expr="Level.low" sce:direction="out"/>
    <data id="outcome" sce:type="enum:Status" expr="Status.ok" sce:direction="out"/>
    <data id="picked" sce:type="uint8" expr="0" sce:direction="out"/>
    <data id="civilYear" sce:type="int32" expr="2026" sce:direction="out"/>
    <data id="civilMonth" sce:type="uint8" expr="9" sce:direction="out"/>
    <data id="civilDay" sce:type="uint8" expr="23" sce:direction="out"/>
    <data id="epochDays" sce:type="int64" expr="0" sce:direction="out"/>
    <data id="refusals" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="pick" type="internal" sce:req="REQ-1">
      <assign location="picked" expr="OuterPick(_event.data.count)"/>
      <assign location="shownLevel" expr="_event.data.level"/>
    </transition>
    <transition event="civil" type="internal">
      <assign location="epochDays" expr="DaysFromCivil(civilYear, civilMonth, civilDay)"/>
      <assign location="outcome" expr="Status.notFound"/>
    </transition>
    <transition event="error.execution" type="internal">
      <assign location="refusals" expr="refusals + 1"/>
    </transition>
  </state>
</scxml>
"#;

const SCHEMA_PICK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="schema_pick" sce:event-name="pick">
  <sce:import src="enum_level.scxml" kind="enum" as="Level"/>
  <datamodel>
    <data id="count" sce:type="uint8" sce:direction="in"/>
    <data id="level" sce:type="enum:Level" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

const ENUM_LEVEL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="enum_level" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="1"/>
      <sce:variant name="high" value="2"/>
    </data>
  </datamodel>
</scxml>
"#;

/// Imported by the statechart, and claims REQ-2 on its root.
const ALGORITHM_OUTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="outer_pick" version="1.0" sce:req="REQ-2">
  <sce:import kind="algorithm" src="algorithm_inner.scxml" as="even"/>
  <sce:signature>
    <sce:param name="n" type="uint8"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:if cond="even(n)">
      <sce:return expr="1"/>
    </sce:if>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;

/// Imported by the outer algorithm, two documents down from the statechart,
/// and claims REQ-3 on its root.
const ALGORITHM_INNER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="inner_even" version="1.0" sce:req="REQ-3">
  <sce:signature>
    <sce:param name="n" type="uint8"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="n % 2 === 0"/>
  </sce:body>
</scxml>
"#;

/// A second statechart of the same specification, imported by nothing, which
/// claims REQ-4.
const PEER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="null">
  <state id="idle" sce:req="REQ-4">
    <transition event="go" target="busy"/>
  </state>
  <state id="busy"/>
</scxml>
"#;

/// Nothing claims REQ-5.
const MANIFEST: &str = r#"{
  "doc_id": "SPEC-SET",
  "rev": "1",
  "extraction": {"ids": "native", "trace": "none",
                 "modality_convention": "english-modal-verbs", "method": "hand"},
  "requirements": [
    {"id": "REQ-1"},
    {"id": "REQ-2"},
    {"id": "REQ-3"},
    {"id": "REQ-4"},
    {"id": "REQ-5"}
  ]
}
"#;

const ENTRY: &str = "closure_machine.scxml";

/// A design directory holding every document above and the manifest.
fn design() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, text) in [
        (ENTRY, MACHINE),
        ("schema_pick.scxml", SCHEMA_PICK),
        ("enum_level.scxml", ENUM_LEVEL),
        ("algorithm_outer.scxml", ALGORITHM_OUTER),
        ("algorithm_inner.scxml", ALGORITHM_INNER),
        ("peer.scxml", PEER),
        ("spec.manifest.json", MANIFEST),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    dir
}

fn run(args: &[&str], cwd: &Path) -> Output {
    Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
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

fn report(dir: &Path, documents: &[&str], with_imports: bool) -> String {
    let mut args = vec!["acceptance-report"];
    args.extend_from_slice(documents);
    if with_imports {
        args.push("--with-imports");
    }
    args.extend_from_slice(&["--manifest", "spec.manifest.json", "--variant", "base"]);
    stdout(&run(&args, dir))
}

fn requirement_outcome(dir: &Path, documents: &[&str], with_imports: bool, id: &str) -> String {
    let mut args = vec!["requirements"];
    args.extend_from_slice(documents);
    if with_imports {
        args.push("--with-imports");
    }
    args.extend_from_slice(&["--manifest", "spec.manifest.json"]);
    let records = stdout(&run(&args, dir));
    records
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("NDJSON"))
        .find(|record| record["kind"] == "requirement" && record["id"] == id)
        .unwrap_or_else(|| panic!("no record for {id} in {records}"))["outcome"]
        .as_str()
        .expect("an outcome is a word")
        .to_string()
}

fn accept(dir: &Path, documents: &[&str], with_imports: bool) -> serde_json::Value {
    let mut args = vec!["accept"];
    args.extend_from_slice(documents);
    if with_imports {
        args.push("--with-imports");
    }
    args.extend_from_slice(&[
        "--manifest",
        "spec.manifest.json",
        "--variant",
        "base",
        "--root",
        ".",
        "--out",
        "acceptance.json",
    ]);
    stdout(&run(&args, dir));
    let text = std::fs::read_to_string(dir.join("acceptance.json")).expect("the record");
    serde_json::from_str(&text).expect("the record is JSON")
}

fn check(dir: &Path) -> Output {
    run(
        &[
            "acceptance-check",
            "acceptance.json",
            "--variant",
            "base",
            "--root",
            ".",
        ],
        dir,
    )
}

/// The requirement ids the record keeps evidence for.
fn evidenced(record: &serde_json::Value) -> Vec<String> {
    record["evidence"]
        .as_object()
        .map(|evidence| evidence.keys().cloned().collect())
        .unwrap_or_default()
}

/// The paths the record pins as inputs.
fn pinned(record: &serde_json::Value) -> Vec<String> {
    record["inputs"]
        .as_array()
        .expect("inputs is a list")
        .iter()
        .map(|pin| pin["path"].as_str().expect("a path").to_string())
        .collect()
}

/// Append a comment after the root, so the document says the same and its
/// bytes do not.
fn touch(dir: &Path, name: &str) {
    let path = dir.join(name);
    let mut text = std::fs::read_to_string(&path).expect("read");
    text.push_str("<!-- edited -->\n");
    std::fs::write(&path, text).expect("write");
}

#[test]
fn a_claim_made_in_an_imported_document_is_credited_on_the_page() {
    let dir = design();
    let page = report(dir.path(), &[ENTRY], true);
    for id in ["REQ-2", "REQ-3"] {
        assert!(
            !page.contains(&format!("missing        {id}")),
            "{id} is claimed in an imported algorithm: {page}"
        );
    }
    assert!(
        page.contains("evidence in algorithm_outer.scxml:"),
        "{page}"
    );
    assert!(
        page.contains("evidence in algorithm_inner.scxml:"),
        "two imports down: {page}"
    );
    assert!(
        page.contains("evidence in closure_machine.scxml:"),
        "{page}"
    );
    // Nothing in the design claims REQ-5, nor REQ-4, which the peer claims
    // and the peer is no import.
    assert!(page.contains("missing        REQ-5"), "{page}");
    assert!(page.contains("missing        REQ-4"), "{page}");
    // The standard documents the statechart imports are not the owner's.
    assert!(!page.contains("sce:std"), "{page}");
    assert!(!page.contains("days_from_civil"), "{page}");
}

#[test]
fn requirements_credit_the_claims_of_what_the_entry_imports() {
    let dir = design();
    assert_eq!(
        requirement_outcome(dir.path(), &[ENTRY], true, "REQ-2"),
        "implemented"
    );
    assert_eq!(
        requirement_outcome(dir.path(), &[ENTRY], true, "REQ-3"),
        "implemented"
    );
    assert_eq!(
        requirement_outcome(dir.path(), &[ENTRY], true, "REQ-5"),
        "missing"
    );
}

#[test]
fn one_entry_document_named_alone_answers_as_it_did() {
    let dir = design();
    // The page: REQ-2 is claimed only in the algorithm, which a document read
    // alone does not take in.
    let page = report(dir.path(), &[ENTRY], false);
    assert!(page.contains("missing        REQ-2"), "{page}");
    assert!(
        page.contains("evidence: (this document annotates nothing for it)"),
        "{page}"
    );
    assert!(!page.contains("evidence in "), "{page}");
    assert_eq!(
        requirement_outcome(dir.path(), &[ENTRY], false, "REQ-2"),
        "missing"
    );

    // The record: one document, no set, the imports pinned as they always
    // were, and no claim of theirs credited.
    let record = accept(dir.path(), &[ENTRY], false);
    let object = record.as_object().expect("a record is an object");
    assert!(!object.contains_key("documents"), "{record}");
    assert!(!object.contains_key("with_imports"), "{record}");
    assert_eq!(record["document"], ENTRY);
    assert_eq!(evidenced(&record), ["REQ-1"]);
    assert!(
        pinned(&record).contains(&"algorithm_outer.scxml".to_string()),
        "{record}"
    );
    assert!(check(dir.path()).status.success());
}

#[test]
fn an_acceptance_of_the_entry_and_what_it_imports_credits_their_claims() {
    let dir = design();
    let record = accept(dir.path(), &[ENTRY], true);
    assert_eq!(record["with_imports"], serde_json::json!(true), "{record}");
    assert!(
        record.get("documents").is_none(),
        "one document was named: {record}"
    );
    assert_eq!(evidenced(&record), ["REQ-1", "REQ-2", "REQ-3"]);
    let inputs = pinned(&record);
    for name in [
        ENTRY,
        "schema_pick.scxml",
        "enum_level.scxml",
        "algorithm_outer.scxml",
        "algorithm_inner.scxml",
    ] {
        assert!(inputs.contains(&name.to_string()), "{name}: {record}");
    }
    assert!(
        inputs.iter().all(|path| !path.starts_with("sce:std")),
        "{record}"
    );
    let held = check(dir.path());
    assert!(
        held.status.success(),
        "{}",
        String::from_utf8_lossy(&held.stderr)
    );
}

#[test]
fn a_change_to_any_input_of_the_set_lapses_the_acceptance() {
    for moved in [
        ENTRY,
        "algorithm_outer.scxml",
        "algorithm_inner.scxml",
        "enum_level.scxml",
    ] {
        let dir = design();
        accept(dir.path(), &[ENTRY], true);
        touch(dir.path(), moved);
        let lapsed = check(dir.path());
        assert!(!lapsed.status.success(), "{moved} moved and the check held");
        let stderr = String::from_utf8_lossy(&lapsed.stderr);
        assert!(
            stderr.contains("cli/acceptance-lapsed"),
            "{moved}: {stderr}"
        );
        assert!(stderr.contains(moved), "{moved}: {stderr}");
    }
}

#[test]
fn several_documents_named_are_one_acceptance() {
    let dir = design();
    let record = accept(dir.path(), &[ENTRY, "peer.scxml"], false);
    assert_eq!(
        record["documents"],
        serde_json::json!([ENTRY, "peer.scxml"]),
        "{record}"
    );
    assert!(record.get("with_imports").is_none(), "{record}");
    // The peer's claim is the design's; the algorithm's is not, since the set
    // was not asked to take in what the documents import.
    assert_eq!(evidenced(&record), ["REQ-1", "REQ-4"]);
    assert!(check(dir.path()).status.success());

    touch(dir.path(), "peer.scxml");
    let lapsed = check(dir.path());
    assert!(
        !lapsed.status.success(),
        "the peer moved and the check held"
    );
    assert!(String::from_utf8_lossy(&lapsed.stderr).contains("peer.scxml"));

    std::fs::remove_file(dir.path().join("peer.scxml")).expect("remove");
    let gone = check(dir.path());
    assert!(
        !gone.status.success(),
        "the peer is gone and the check held"
    );
    assert!(
        String::from_utf8_lossy(&gone.stderr).contains("no longer there"),
        "{}",
        String::from_utf8_lossy(&gone.stderr)
    );
}

#[test]
fn the_report_of_several_documents_names_each_and_credits_each() {
    let dir = design();
    let page = report(dir.path(), &[ENTRY, "peer.scxml"], false);
    assert!(page.contains("<->  2 documents"), "{page}");
    assert!(page.contains("document: peer.scxml"), "{page}");
    assert!(page.contains("evidence in peer.scxml:"), "{page}");
    assert!(!page.contains("missing        REQ-4"), "{page}");
    assert!(page.contains("missing        REQ-2"), "{page}");
}

#[test]
fn a_record_of_a_set_is_compared_as_a_set_and_a_draft_of_one_document_is_refused() {
    let dir = design();
    accept(dir.path(), &[ENTRY], true);
    let delta = stdout(&run(
        &["acceptance-delta", "acceptance.json", "--root", "."],
        dir.path(),
    ));
    let summary: serde_json::Value = serde_json::from_str(
        delta
            .lines()
            .last()
            .expect("a summary line ends the report"),
    )
    .expect("JSON");
    assert_eq!(summary["kind"], "acceptance-delta-summary", "{delta}");
    assert_eq!(summary["requirements"], 3, "{delta}");
    assert_eq!(summary["unchanged"], 3, "{delta}");

    let refused = run(
        &[
            "acceptance-delta",
            "acceptance.json",
            "--root",
            ".",
            "--design",
            ENTRY,
        ],
        dir.path(),
    );
    assert!(
        !refused.status.success(),
        "a draft of one document compared"
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("cli/closure-input-unusable"), "{stderr}");
    assert!(stderr.contains("draft of one document"), "{stderr}");
}

/// The library's own door: a design of one document is the record `take`
/// writes, byte for byte, and the same document read with its imports is
/// another record, one that credits what they claim.
#[test]
fn a_design_of_one_document_is_the_record_take_writes() {
    let dir = design();
    let root = dir.path();
    let alone = AcceptanceRecord::take(
        root,
        &root.join(ENTRY),
        &root.join("spec.manifest.json"),
        "base",
    )
    .expect("taken");
    let one = AcceptanceRecord::take_design(
        root,
        &DesignSet::one(root.join(ENTRY).display().to_string()),
        &root.join("spec.manifest.json"),
        "base",
        &[],
    )
    .expect("taken");
    assert_eq!(one.to_json(), alone.to_json());
    assert!(!one.is_of_a_set());

    let closure = AcceptanceRecord::take_design(
        root,
        &DesignSet::new(vec![root.join(ENTRY).display().to_string()], true).expect("a design"),
        &root.join("spec.manifest.json"),
        "base",
        &[],
    )
    .expect("taken");
    assert!(closure.is_of_a_set());
    assert!(
        closure.evidence.contains_key("REQ-2"),
        "{}",
        closure.to_json()
    );
    assert!(
        closure.recheck(root, "base").expect("rechecked").is_empty(),
        "{}",
        closure.to_json()
    );
}

/// The page of one statechart is `render`'s page, byte for byte.
#[test]
fn a_page_of_one_document_is_the_page_render_writes() {
    use sce_build::acceptance_report::{render, render_design, ReportDocument, ReportMember};
    use sce_build::requirement_manifest::RequirementManifest;

    let dir = design();
    let manifest = RequirementManifest::load(&dir.path().join("spec.manifest.json"))
        .expect("the requirement set loads");
    let peer = dir.path().join("peer.scxml");
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(peer.to_str().expect("a utf-8 path"))
        .expect("the peer parses");
    let alone = render(&model, &manifest, None, "base");
    let one = render_design(
        &[ReportMember {
            label: "peer.scxml",
            document: ReportDocument::Statechart(&model),
        }],
        &manifest,
        None,
        "base",
    );
    assert_eq!(one, alone);
    assert!(alone.contains("REQ-4"), "{alone}");
}
