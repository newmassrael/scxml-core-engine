// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! NL→IR Mapping Roadmap Item 7 — `sce:provenance` parse integration.
//!
//! The type, the compact-URI parser, and the four model fields landed
//! with Items 1-6; nothing read the attribute. These tests are what
//! makes that reading observable, and they are driven from committed
//! fixtures rather than inline strings for a reason: the same three
//! documents are swept by `diagnostic_corpus_schema.rs`, which runs
//! `sce-codegen check` over every tracked fixture and validates each
//! record against `schemas/sce-diagnostic.v1.schema.json`. A fixture
//! here therefore pins the model shape AND the wire shape of the
//! rejection, with no second copy of the document to drift.
//!
//! What each fixture is for is written in its own leading comment.

use std::path::PathBuf;
use std::process::Command;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::model::{Invoke, SCXMLModel};
use sce_build::parser::SCXMLParser;
use sce_build::provenance::SpecProvenance;

const FIXTURES_DIR: &str = "tests/fixtures/provenance";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent (workspace root)")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_DIR)
        .join(name)
}

fn parse(name: &str) -> Result<SCXMLModel, Located<ForgeError>> {
    let path = fixture(name);
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture must be readable at {}: {e}", path.display()));
    SCXMLParser::new().parse_string(&body, name)
}

/// `(doc_id, rev, section, page)` in declaration order — the shape the
/// assertions read, so a failure prints the whole anchor rather than
/// the one field that differed.
///
/// The last element stays a page number rather than a
/// [`sce_build::provenance::Position`] because every anchor in this
/// file's fixtures is written in the paginated shape; the variant's
/// other arms are exercised where they are authored, in
/// `provenance.rs`'s own cases and in the locator suite.
type Anchor<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Option<u32>);

fn anchors(list: &[SpecProvenance]) -> Vec<Anchor<'_>> {
    list.iter()
        .map(|p| {
            (
                p.doc_id.as_str(),
                p.rev.as_deref(),
                p.section.as_deref(),
                match &p.at {
                    Some(sce_build::provenance::Position::Page(n)) => Some(*n),
                    _ => None,
                },
            )
        })
        .collect()
}

fn validation_error(err: &Located<ForgeError>) -> &ValidationError {
    match &err.error {
        ForgeError::Validation(v) => v.as_ref(),
        other => panic!("expected a validation error, got: {other:?}"),
    }
}

#[test]
fn attribute_and_element_forms_are_additive_in_document_order() {
    let model = parse("positive_all_attachment_sites.scxml").expect("fixture parses");
    let s0 = &model.states["s0"];
    assert_eq!(
        anchors(&s0.provenance),
        vec![
            // The attribute's anchor first, then the element children
            // in document order. `:112` is the page slot of the
            // compact form; `11.2.1` carries no revision.
            ("OEM-DIAG-SPEC", Some("D"), Some("3.4.2"), Some(112)),
            ("OEM-TIMING-REQ", Some("B"), Some("7.1"), Some(44)),
            ("ISO-14229-1", None, Some("11.2.1"), None),
        ],
    );
    // Orthogonal to `sce:req`: neither family consumes the other's
    // attribute, which a shared-parsing regression would break.
    let req: Vec<&str> = s0.req.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(req, vec!["REQ_STATE_S0"]);
}

#[test]
fn every_attachment_site_reads_the_annotation() {
    let model = parse("positive_all_attachment_sites.scxml").expect("fixture parses");

    // `<parallel>` and `<final>` are separate parse sites from
    // `<state>`, writing the same model field. A site that stopped
    // reading would be invisible without all three.
    assert_eq!(
        anchors(&model.states["fan"].provenance),
        vec![("OEM-DIAG-SPEC", Some("D"), Some("5.1"), None)],
    );
    assert_eq!(
        anchors(&model.states["done"].provenance),
        vec![("OEM-DIAG-SPEC", Some("D"), Some("9.9"), None)],
    );

    let s0 = &model.states["s0"];
    assert_eq!(
        anchors(&s0.transitions[0].provenance),
        vec![
            ("OEM-DIAG-SPEC", Some("D"), Some("3.4.4"), None),
            ("OEM-TIMING-REQ", Some("B"), Some("7.2"), None),
        ],
    );

    let invoke_base = match &s0.invokes[0] {
        Invoke::Scxml(info) => &info.common.base,
        other => panic!("expected an Scxml invoke, got: {other:?}"),
    };
    assert_eq!(
        anchors(&invoke_base.provenance),
        vec![("OEM-INVOKE-SPEC", Some("A"), Some("2.1"), Some(7))],
    );
}

#[test]
fn block_level_anchor_inherits_onto_every_action_without_overwriting() {
    let model = parse("positive_all_attachment_sites.scxml").expect("fixture parses");
    let s0 = &model.states["s0"];

    // `<onentry>` has no model node, so its anchor has nowhere to live
    // but the actions inside: the leaf's own anchor stays first, the
    // block's is appended.
    assert_eq!(
        anchors(&s0.on_entry_blocks[0][0].provenance),
        vec![
            ("OEM-DIAG-SPEC", Some("D"), Some("3.4.3"), None),
            ("OEM-BLOCK-SPEC", Some("A"), Some("1.1"), None),
        ],
    );

    // The inherit is matched on `doc_id`, not on the whole anchor, so
    // the block's rev C does NOT join the leaf's rev D for the same
    // document. Were it matched on equality the leaf would end up
    // carrying two revisions of one document — precisely what
    // `validation/provenance-duplicate` exists to refuse.
    assert_eq!(
        anchors(&s0.on_exit_blocks[0][0].provenance),
        vec![("OEM-DIAG-SPEC", Some("D"), Some("3.4.9"), None)],
    );
}

#[test]
fn absent_annotation_leaves_every_field_empty() {
    // The byte-identity claim in RFC §6.3, at the model layer: a
    // document with no `sce:provenance` must be indistinguishable from
    // one parsed before Item 7 existed. `sce_annotations.scxml` is the
    // sharp version of that — it carries `sce:req` and
    // `sce:unresolved` on every attachment site and no provenance at
    // all, so a parser that populated the field from the wrong
    // attribute fails here rather than on a bare document.
    let model = parse("../codegen_smoke/sce_annotations.scxml")
        .expect("the sce:req / sce:unresolved fixture parses");
    for state in model.states.values() {
        assert!(
            state.provenance.is_empty(),
            "state {} gained an anchor from a document that declares none",
            state.id,
        );
        for transition in &state.transitions {
            assert!(transition.provenance.is_empty());
        }
        for block in state.on_entry_blocks.iter().chain(&state.on_exit_blocks) {
            for action in block {
                assert!(action.provenance.is_empty());
            }
        }
    }
}

#[test]
fn compact_uri_naming_no_document_rejects() {
    let err = parse("negative_malformed_compact_uri.scxml")
        .expect_err("an anchor with an empty doc_id must reject");
    match validation_error(&err) {
        ValidationError::MalformedProvenance { element, value } => {
            assert_eq!(value, "@23", "the offending text is carried verbatim");
            assert!(
                element.contains("<state id=\"s0\">"),
                "the rejection names the annotated element: {element}",
            );
        }
        other => panic!("expected MalformedProvenance, got: {other:?}"),
    }
}

#[test]
fn element_form_without_doc_id_rejects_as_the_same_code() {
    // The element form decomposes the compact grammar, so a missing
    // `doc-id` is the same defect as an empty `doc_id` and must not
    // need a second code for a consumer to branch on.
    let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          version="1.0" initial="s0">
        <state id="s0">
          <sce:provenance rev="D" section="3.4.2"/>
        </state>
    </scxml>"#;
    let err = SCXMLParser::new()
        .parse_string(scxml, "provenance_element_no_doc_id")
        .expect_err("an element-form anchor without doc-id must reject");
    assert!(
        matches!(
            validation_error(&err),
            ValidationError::MalformedProvenance { .. }
        ),
        "expected MalformedProvenance, got: {:?}",
        validation_error(&err),
    );
}

#[test]
fn one_doc_id_anchored_twice_on_a_node_rejects() {
    let err = parse("negative_duplicate_doc_id.scxml")
        .expect_err("one document anchored twice on a node must reject");
    match validation_error(&err) {
        ValidationError::DuplicateProvenanceDocId { element, doc_id } => {
            assert_eq!(doc_id, "OEM-DIAG-SPEC");
            assert!(
                element.contains("<state id=\"s0\">"),
                "the rejection names the annotated element: {element}",
            );
        }
        other => panic!("expected DuplicateProvenanceDocId, got: {other:?}"),
    }
}

/// A rejection raised about an anchored node carries that node's
/// anchors onto the diagnostic — the wire field's first producer.
///
/// `spec_provenance` has been declared on the diagnostic schema since
/// Item 6 and empty in every record SCE ever emitted, which is the
/// shape of defect this whole item exists to close: a declaration
/// that reads as landed and fires nothing. This is the assertion that
/// it fires.
#[test]
fn a_rejection_on_an_anchored_node_carries_its_anchors() {
    // A duplicate `sce:req` on a node that also declares two anchors.
    // The anchors are read first precisely so this rejection can name
    // them; parsed in the other order they would not exist yet.
    let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          version="1.0" initial="s0">
        <state id="s0" sce:req="REQ_A REQ_A"
               sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
          <sce:provenance doc-id="ISO-14229-1" section="11.2.1"/>
        </state>
    </scxml>"#;
    let err = SCXMLParser::new()
        .parse_string(scxml, "provenance_anchored_rejection")
        .expect_err("duplicate sce:req must reject");

    assert!(
        matches!(
            validation_error(&err),
            ValidationError::DuplicateRequirementId { .. }
        ),
        "expected the duplicate-req rejection, got: {:?}",
        validation_error(&err),
    );
    assert_eq!(
        anchors(&err.spec_provenance),
        vec![
            ("OEM-DIAG-SPEC", Some("D"), Some("3.4.2"), Some(112)),
            ("ISO-14229-1", None, Some("11.2.1"), None),
        ],
        "the rejection must carry the node's anchors, in document order",
    );

    // And they survive the hop onto the wire record, which is the
    // half a consumer actually reads.
    let diagnostics = err.to_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        anchors(&diagnostics[0].spec_provenance),
        anchors(&err.spec_provenance),
        "the diagnostic must carry what the located error carried",
    );
}

/// The same claim through the CLI, on stderr, as JSON — the only
/// surface an external consumer actually sees.
///
/// A unit test can hold the field populated on a struct while the
/// serialiser, the flag plumbing, or the stream discipline drops it;
/// `spec_provenance` is `skip_serializing_if = "Vec::is_empty"`, so a
/// producer that regressed to empty would still serialise cleanly and
/// silently. Reading the bytes is the only way to tell.
#[test]
fn the_strict_build_puts_the_anchors_on_the_json_wire() {
    let fixture = fixture("negative_anchored_unresolved.scxml");
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("provenance_wire");
    std::fs::create_dir_all(&out_dir).expect("create scratch");

    let output = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .env("SCE_WORKSPACE_ROOT", workspace_root())
        .args(["generate", "-l", "rust", "--strict-unresolved"])
        .arg("--error-format")
        .arg("json")
        .arg("-o")
        .arg(&out_dir)
        .arg(&fixture)
        .output()
        .expect("spawn sce-codegen");

    assert!(
        !output.status.success(),
        "--strict-unresolved must refuse the fixture; it exited {:?}",
        output.status.code(),
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    let record: serde_json::Value = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .find_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .unwrap_or_else(|| panic!("no JSON diagnostic on stderr:\n{stderr}"));

    assert_eq!(
        record["code"], "validation/unresolved-placeholder",
        "unexpected record: {record}",
    );
    let anchors_on_wire = record["spec_provenance"]
        .as_array()
        .unwrap_or_else(|| panic!("spec_provenance absent or not an array: {record}"));
    assert_eq!(
        anchors_on_wire.len(),
        2,
        "expected both of the node's anchors on the wire: {record}",
    );
    assert_eq!(anchors_on_wire[0]["doc_id"], "OEM-DIAG-SPEC");
    assert_eq!(anchors_on_wire[0]["rev"], "D");
    // The position rides the wire as the variant spells it, so a
    // non-paginated source's coordinate survives serialisation too.
    assert_eq!(anchors_on_wire[0]["at"]["page"], 112);
    assert_eq!(anchors_on_wire[1]["doc_id"], "ISO-14229-1");
    assert_eq!(anchors_on_wire[1]["section"], "11.2.1");
}

#[test]
fn the_duplicate_rejection_points_at_the_second_occurrence() {
    // The author deletes the second anchor, not the first, so that is
    // the line the diagnostic must name. `negative_duplicate_doc_id`
    // declares the attribute form on line 22 and the element form on
    // line 23.
    let err = parse("negative_duplicate_doc_id.scxml").expect_err("must reject");
    let declared_at = std::fs::read_to_string(fixture("negative_duplicate_doc_id.scxml"))
        .expect("fixture readable")
        .lines()
        .position(|l| l.contains("<sce:provenance doc-id=\"OEM-DIAG-SPEC\""))
        .expect("fixture still declares the element-form duplicate")
        + 1;
    assert_eq!(
        err.location.line,
        Some(declared_at as u32),
        "expected the rejection at the duplicate's own line ({declared_at}), got {:?}",
        err.location.line,
    );
}
