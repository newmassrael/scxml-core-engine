// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An acceptance report answers for a design of several documents.
//!
//! `requirements A B C --manifest m` measured the documents of one design
//! together, and `acceptance-report` took one document: a requirement claimed
//! in an algorithm the statechart imports read "this document annotates
//! nothing for it" on the page a person signs. The page now takes the set
//! `requirements` takes, read by one reader (`sce_build::design_set`):
//! several documents, or one with `--with-imports` and every document it
//! reaches.
//!
//! What these hold, each by a case that fails when it stops being true:
//!   * a claim made in an imported document, one import down or two, is
//!     credited on the page and by `requirements`
//!   * several documents named are one page, each named, a claim in any of
//!     them the design's
//!   * a standard document the design reaches is no member of it
//!   * one document's page is the page `render` writes, byte for byte

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
fn the_report_of_several_documents_names_each_and_credits_each() {
    let dir = design();
    let page = report(dir.path(), &[ENTRY, "peer.scxml"], false);
    assert!(page.contains("<->  2 documents"), "{page}");
    assert!(page.contains("document: peer.scxml"), "{page}");
    assert!(page.contains("evidence in peer.scxml:"), "{page}");
    assert!(!page.contains("missing        REQ-4"), "{page}");
    assert!(page.contains("missing        REQ-2"), "{page}");
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
