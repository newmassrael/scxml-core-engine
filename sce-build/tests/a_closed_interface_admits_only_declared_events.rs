// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `<scxml sce:interface="closed">` — SCE Accepted Subset §2.16.
//!
//! A statechart that declares its interface closed is held to the
//! event-schemas it imports: what it takes from outside and what it sends
//! outside are events a schema declares, and what it sends itself is taken
//! by some transition. The controls fail the other way: the same document
//! with the interface left open takes an event no schema declares (the
//! schemaless fallback stays the default), and the closed document that
//! keeps to its schemas — descriptor prefixes, platform events, its own
//! raises and delayed sends included — builds. A self-send nothing takes
//! is lost whether or not the interface is closed, so an open document is
//! refused it too, by the lint that shares the reading
//! (`scxml/self-send-discarded`).

use std::fs;
use std::path::Path;

use sce_build::forge::error::ForgeError;
use sce_build::generator::Language;
use sce_build::scxml_semantic::{InterfaceCrossing, ScxmlSemanticError};
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

const SCHEMAS: [(&str, &str, &str); 3] = [
    ("schema_coin.scxml", "coin.inserted", "value"),
    ("schema_select.scxml", "product.selected", "product"),
    ("schema_dispense.scxml", "product.dispense", "product"),
];

fn schema(event: &str, field: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="{}" sce:kind="event-schema" sce:event-name="{event}">
  <datamodel><data id="{field}" sce:type="int32" sce:direction="in"/></datamodel>
</scxml>
"#,
        event.replace('.', "_")
    )
}

/// A vending statechart that keeps to its interface: a descriptor prefix
/// (`coin`), a platform event, its own `<raise>` and its own delayed send are
/// all admitted, and its output goes out as a declared event.
const KEEPS_TO_IT: &str = r##"
  <state id="idle">
    <transition event="coin" target="credited"/>
    <transition event="error.execution" target="idle"/>
  </state>
  <state id="credited">
    <onentry><send id="t" event="credit.timeout" delay="30s"/></onentry>
    <onexit><cancel sendid="t"/></onexit>
    <transition event="product.selected" target="dispensing">
      <raise event="credit.spent"/>
    </transition>
    <transition event="credit.timeout" target="idle"/>
  </state>
  <state id="dispensing">
    <onentry><send event="product.dispense" target="#_parent"/></onentry>
    <transition event="credit.spent" target="idle"/>
  </state>
"##;

fn document(interface: Option<&str>, body: &str) -> String {
    let declared = interface
        .map(|v| format!(r#" sce:interface="{v}""#))
        .unwrap_or_default();
    let imports: String = SCHEMAS
        .iter()
        .enumerate()
        .map(|(i, (file, _, _))| {
            format!(
                r#"  <sce:import src="{file}" kind="event-schema" as="Schema{i}"/>{}"#,
                "\n"
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="vending" initial="idle" datamodel="ecmascript"{declared}>
{imports}{body}
</scxml>
"#
    )
}

fn compile(text: &str) -> Result<(), sce_build::forge::error::Located<ForgeError>> {
    let dir = tempfile::TempDir::new().expect("tempdir");
    for (file, event, field) in SCHEMAS {
        fs::write(dir.path().join(file), schema(event, field)).expect("write schema");
    }
    let path = dir.path().join("vending.scxml");
    fs::write(&path, text).expect("write document");
    compile_at(&path)
}

fn compile_at(path: &Path) -> Result<(), sce_build::forge::error::Located<ForgeError>> {
    compile_scxml_lang_typed(
        path.to_str().unwrap(),
        &find_template_dir_for(Language::Rust),
        Language::Rust,
    )
    .map(|_| ())
}

fn crossing(body: &str) -> (InterfaceCrossing, String, String) {
    let err = compile(&document(Some("closed"), body)).expect_err("a closed interface refuses it");
    match &err.error {
        ForgeError::Scxml(semantic) => match semantic.as_ref() {
            ScxmlSemanticError::UndeclaredInterfaceEvent {
                crossing,
                event,
                state,
                ..
            } => (*crossing, event.clone(), state.clone()),
            other => panic!("expected UndeclaredInterfaceEvent, got {other:?}"),
        },
        other => panic!("expected a semantic refusal, got {other:?}"),
    }
}

#[test]
fn a_statechart_that_keeps_to_its_interface_builds() {
    compile(&document(Some("closed"), KEEPS_TO_IT)).expect("every crossing is declared");
}

#[test]
fn an_event_taken_that_no_schema_declares_is_refused() {
    let body = KEEPS_TO_IT.replace(r#"event="coin""#, r#"event="coin.insert""#);
    assert_eq!(
        crossing(&body),
        (
            InterfaceCrossing::Receives,
            "coin.insert".into(),
            "idle".into()
        )
    );
}

#[test]
fn an_event_sent_out_that_no_schema_declares_is_refused() {
    let body = KEEPS_TO_IT.replace(
        r##"event="product.dispense" target="#_parent""##,
        r##"event="product.dispensed" target="#_parent""##,
    );
    assert_eq!(
        crossing(&body),
        (
            InterfaceCrossing::Sends,
            "product.dispensed".into(),
            "dispensing".into()
        )
    );
}

#[test]
fn a_computed_event_name_cannot_be_checked_and_is_refused() {
    let body = KEEPS_TO_IT.replace(
        r##"event="product.dispense" target="#_parent""##,
        r##"eventexpr="'product.' + 'dispense'" target="#_parent""##,
    );
    assert_eq!(crossing(&body).0, InterfaceCrossing::SendsComputed);
}

#[test]
fn an_output_sent_to_itself_that_nothing_takes_is_refused() {
    // The measured shape: an output written as a send with no target, which
    // W3C SCXML 6.2.4 delivers to the session's own queue.
    let body = KEEPS_TO_IT.replace(
        r##"<send event="product.dispense" target="#_parent"/>"##,
        r#"<send event="display.price"/>"#,
    );
    assert_eq!(
        crossing(&body),
        (
            InterfaceCrossing::SendsToItself,
            "display.price".into(),
            "dispensing".into()
        )
    );
}

#[test]
fn the_same_document_with_an_open_interface_is_accepted() {
    // The schemaless fallback is the default and stays it: an undeclared
    // input is accepted when open.
    let body = KEEPS_TO_IT.replace(r#"event="coin""#, r#"event="coin.insert""#);
    compile(&document(None, &body)).expect("an open interface admits undeclared events");
}

#[test]
fn an_open_document_is_refused_the_output_it_sends_itself_by_the_lint() {
    // The same lost output, open: not an interface crossing, since nothing
    // was declared, but a message the machine sends itself and discards.
    let body = KEEPS_TO_IT.replace(
        r##"<send event="product.dispense" target="#_parent"/>"##,
        r#"<send event="display.price"/>"#,
    );
    let err = compile(&document(None, &body)).expect_err("the lint refuses it");
    match &err.error {
        ForgeError::Scxml(semantic) => match semantic.as_ref() {
            ScxmlSemanticError::SelfSendDiscarded { event, state } => {
                assert_eq!(
                    (event.as_str(), state.as_str()),
                    ("display.price", "dispensing")
                );
            }
            other => panic!("expected SelfSendDiscarded, got {other:?}"),
        },
        other => panic!("expected a semantic refusal, got {other:?}"),
    }
}

#[test]
fn only_closed_is_a_value() {
    let err = compile(&document(Some("open"), KEEPS_TO_IT)).expect_err("`open` is the absence");
    assert!(
        err.error.to_string().contains("sce:interface"),
        "{:?}",
        err.error
    );
}

#[test]
fn a_forge_document_does_not_take_the_declaration() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("scaled.scxml");
    fs::write(
        &path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="scaled" sce:kind="transform" sce:interface="closed">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="raw * 2"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("write");
    let refused = sce_build::compile_forge_file(
        &path,
        Language::Rust,
        &[],
        &sce_build::ForgeCompileOptions::default(),
    )
    .map(|_| ())
    .expect_err("a forge root does not read sce:interface");
    assert!(
        refused.error.to_string().contains("interface"),
        "{:?}",
        refused.error
    );
    // The control: without the attribute the same transform builds, so the
    // refusal above is about the declaration and nothing else.
    fs::write(
        &path,
        fs::read_to_string(&path)
            .expect("read")
            .replace(r#" sce:interface="closed""#, ""),
    )
    .expect("write");
    sce_build::compile_forge_file(
        &path,
        Language::Rust,
        &[],
        &sce_build::ForgeCompileOptions::default(),
    )
    .map(|_| ())
    .expect("the transform itself is well formed");
}
