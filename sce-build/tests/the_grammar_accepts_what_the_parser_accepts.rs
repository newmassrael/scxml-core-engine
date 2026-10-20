// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A grammar is a statement about what the PARSER accepts.
//!
//! `schemas/sce-forge-ext.xsd` runs before every parse
//! (`SCXMLParser::parse_string` → `xsd_validator::validate_or_skip`), so
//! a declaration narrower than its parser does not merely under-describe
//! the language — it REMOVES the form from it, and says so in libxml2's
//! voice rather than in the one the parser wrote for that mistake.
//!
//! ⚠ Three declarations were derived from the documents in the checkout
//! rather than from their parsers, and the checkout is a SAMPLE of what
//! a parser accepts, never its definition:
//!
//! * `<sce:capacity>` was declared with `const` alone, `use="required"`,
//!   while the parser takes `source="deploy" key="..."` as the other of
//!   two forms. Six contract tests went red and stayed red for four
//!   days, because the lane that runs them is `ci_only`.
//! * `<sce:helper args>` was declared `use="required"`, while the parser
//!   reads it with `unwrap_or("")` — an absent `args` is a helper of no
//!   arguments. No test exercised one, so nothing went red.
//! * `<sce:provenance page/row>` were declared as integer types, while
//!   the parser drops a value that does not parse, deliberately, so that
//!   the element form and the compact URI form accept the same thing.
//!
//! ⚠⚠ The discriminator, because a grammar MAY legitimately be stricter
//! than its parser: does the extra strictness remove a form that MEANS
//! something? A bare `<sce:reset-on/>` says exactly what omitting the
//! element says, so requiring its attribute removes nothing and stays.
//! A helper of no arguments is a helper you cannot otherwise write.
//!
//! Each case below is a document the parser accepts, so each one fails
//! if its declaration narrows again.
//!
//! ⚠ A grammar is also a statement about what the GENERATOR reads: an
//! algorithm's `enum:<alias>` parameter, return and local were lowered on
//! every backend and tested below the parser, while `algorithmValueType`
//! named `sceType` alone, so the committed documents that hand an algorithm a
//! value of an enum were refused in libxml2's voice by every `check`. The
//! last cases parse those documents through the entry points `check` and
//! `generate` use, the grammar included.

use std::path::{Path, PathBuf};

use sce_build::forge::model::{
    AlgorithmStmt, AlgorithmValueType, CapacitySource, ForgeDocument, SceType,
};
use sce_build::forge::parser::parse_forge;
use sce_build::parser::SCXMLParser;
use sce_build::DocumentLabel;

fn label(name: &'static str) -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: ".scxml-fixture",
    }
}

/// `<sce:capacity source="deploy" key="..."/>` — the form whose
/// declaration cost six tests.
///
/// The parser's two forms are mutually exclusive and it owns the
/// refusal that says so, which is why all three attributes are optional
/// in the grammar. This asserts the accepted one, not the refusal.
#[test]
fn a_deploy_key_capacity_is_a_document_the_grammar_accepts() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity source="deploy" key="machines.mcu_node.limits.local_subscriptions"/>
</scxml>"##;
    let doc = parse_forge(xml, label("local_sub_table"))
        .expect("a deploy-key capacity must reach the parser")
        .expect("the forge entry point must claim a bounded-collection");
    match doc {
        ForgeDocument::BoundedCollection(c) => assert!(
            matches!(&c.capacity, CapacitySource::DeployKey { key }
                     if key == "machines.mcu_node.limits.local_subscriptions"),
            "the bound must come from the deployment key: {:?}",
            c.capacity,
        ),
        other => panic!("expected a bounded-collection, got {:?}", other.kind()),
    }
}

/// `<sce:helper name="..." returns="..."/>` with no `args` — a helper
/// that takes no arguments.
///
/// `parse_procedure_helper` splits `args` on commas after
/// `unwrap_or("")`, so an absent attribute and an empty one are one
/// thing: zero parameters. Writing `args=""` to satisfy a grammar would
/// be a workaround for the grammar, not a statement about the helper.
#[test]
fn a_helper_of_no_arguments_is_a_document_the_grammar_accepts() {
    let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="s0" version="1.0">
  <datamodel>
    <data id="out" sce:type="uint32" sce:direction="internal"/>
    <sce:helper name="now" returns="uint32"/>
  </datamodel>
  <state id="s0">
    <transition event="ok" target="done"/>
  </state>
  <final id="done"/>
</scxml>"##;
    let doc = parse_forge(xml, label("helper_no_args"))
        .expect("a helper of no arguments must reach the parser")
        .expect("the forge entry point must claim a procedure");
    match doc {
        ForgeDocument::Procedure(p) => {
            let helper = p
                .helpers
                .iter()
                .find(|h| h.name == "now")
                .expect("the helper must reach the IR");
            assert!(
                helper.args.is_empty(),
                "an absent `args` is zero parameters, got {:?}",
                helper.args,
            );
        }
        other => panic!("expected a procedure, got {:?}", other.kind()),
    }
}

/// `<sce:provenance page="draft"/>` — a position the parser drops.
///
/// The compact URI form reads `#4.4.2:draft` as a section with no
/// position rather than refusing it, and the element form is the same
/// grammar decomposed. An integer type on `page` would refuse here what
/// the compact form takes, which is the disagreement the parser's own
/// comment rules out.
#[test]
fn a_position_the_parser_drops_is_a_document_the_grammar_accepts() {
    let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s0">
        <state id="s0">
          <sce:provenance doc-id="OEM-DIAG-SPEC" section="3.4.2" page="draft"/>
        </state>
    </scxml>"#;
    let model = SCXMLParser::new()
        .parse_string(xml, "provenance_unparsable_page")
        .expect("an unparsable position must reach the parser, not libxml2");
    let anchors = &model.states["s0"].provenance;
    assert_eq!(anchors.len(), 1, "one anchor: {anchors:?}");
    assert_eq!(anchors[0].doc_id, "OEM-DIAG-SPEC");
    assert!(
        anchors[0].at.is_none(),
        "a position that does not parse is dropped, not kept: {:?}",
        anchors[0].at,
    );
}

/// Whether `ty` is a value of the enum the document imports as `Status`.
fn is_status(ty: &AlgorithmValueType) -> bool {
    matches!(ty, AlgorithmValueType::Scalar(SceType::Enum(r)) if r.alias == "Status")
}

/// An algorithm document that takes, holds and returns a value of the enum it
/// imports as `Status` and declares `extra` as one more parameter.
fn status_algorithm(extra: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="pick_status" version="1.0">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:signature>
    <sce:param name="status" type="enum:Status"/>
    <sce:param name="keep" type="bool"/>
    {extra}
    <sce:return type="enum:Status"/>
  </sce:signature>
  <sce:body>
    <sce:var name="found" type="enum:Status" init="Status.open"/>
    <sce:if cond="keep">
      <sce:assign target="found" expr="status"/>
    </sce:if>
    <sce:return expr="found"/>
  </sce:body>
</scxml>"##
    )
}

/// `<sce:param type="enum:Status"/>`, `<sce:return type="enum:Status"/>` and
/// `<sce:var type="enum:Status"/>` — a value of an enum an algorithm takes,
/// holds and gives back (SCE_FORGE.md §4.12), read into the IR as the enum the
/// document imports under that alias.
#[test]
fn an_enum_slot_of_an_algorithm_is_a_document_the_grammar_accepts() {
    let doc = parse_forge(&status_algorithm(""), label("pick_status"))
        .expect("an algorithm's enum slots must reach the parser, not stop in libxml2")
        .expect("the forge entry point must claim an algorithm");
    let m = match doc {
        ForgeDocument::Algorithm(m) => m,
        other => panic!("expected an algorithm, got {:?}", other.kind()),
    };
    let status = m
        .signature
        .params
        .iter()
        .find(|p| p.name == "status")
        .expect("the parameter must reach the IR");
    assert!(
        is_status(&status.sce_type),
        "the parameter is a value of `Status`: {:?}",
        status.sce_type
    );
    assert!(
        m.signature.return_type.as_ref().is_some_and(is_status),
        "the return is a value of `Status`: {:?}",
        m.signature.return_type
    );
    assert!(
        m.body.iter().any(|s| matches!(
            s,
            AlgorithmStmt::Var { name, sce_type, .. } if name == "found" && is_status(sce_type)
        )),
        "the local is a value of `Status`"
    );
}

/// The grammar admits `enum:<alias>` lexically, as it does on a field: an
/// alias the document does not import passes it and is refused by the parser,
/// which offers the enums the document does import.
#[test]
fn an_enum_slot_naming_no_import_is_refused_by_the_parser() {
    let err = parse_forge(
        &status_algorithm(r#"<sce:param name="other" type="enum:Nope"/>"#),
        label("pick_status"),
    )
    .expect_err("an alias the document does not import is refused");
    let message = err.to_string();
    assert!(
        message.contains("enum:Nope") && message.contains("enum:Status"),
        "the parser names the alias and offers the import: {message}"
    );
}

fn static_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The committed algorithm documents whose slots are a value of an enum, each
/// parsed as `check` and `generate` parse a forge document, the grammar first.
#[test]
fn the_committed_enum_algorithms_are_documents_the_grammar_accepts() {
    for (file, name) in [
        ("algorithm_item_status_set.scxml", "item_status_set"),
        ("algorithm_item_status_count.scxml", "item_status_count"),
        ("algorithm_item_status_of.scxml", "item_status_of"),
    ] {
        let text = std::fs::read_to_string(static_fixtures().join(file)).expect("read the fixture");
        let doc = parse_forge(&text, label(name))
            .unwrap_or_else(|err| panic!("{file} must parse, grammar included: {err}"))
            .unwrap_or_else(|| panic!("{file} must be claimed as a forge document"));
        let m = match doc {
            ForgeDocument::Algorithm(m) => m,
            other => panic!("{file}: expected an algorithm, got {:?}", other.kind()),
        };
        let slots = m
            .signature
            .params
            .iter()
            .map(|p| &p.sce_type)
            .chain(m.signature.return_type.as_ref());
        let held = slots.filter(|t| is_status(t)).count()
            + m.body
                .iter()
                .filter(|s| matches!(s, AlgorithmStmt::Var { sce_type, .. } if is_status(sce_type)))
                .count();
        assert!(
            held > 0,
            "{file} holds a value of `Status` in a slot or a local"
        );
    }
}

/// The committed statechart that hands those algorithms a value of an enum,
/// parsed from its file as `check` and `generate` read it: its own grammar,
/// then each import it resolves, the algorithms' grammar among them.
#[test]
fn the_committed_machine_that_hands_algorithms_an_enum_parses_from_its_file() {
    let path = static_fixtures().join("static_state_algorithm_enum.scxml");
    SCXMLParser::new()
        .parse_file(path.to_str().expect("a utf-8 path"))
        .unwrap_or_else(|err| panic!("the machine must parse with its imports: {err}"));
}
