// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use sce_build::forge::model::{CapacitySource, ForgeDocument};
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
