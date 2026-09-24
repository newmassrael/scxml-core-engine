// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An element of another vocabulary is not read as SCXML's.
//!
//! A document may carry elements from other namespaces: W3C SCXML 4.10 says
//! the schema "allows elements from arbitrary namespaces inside blocks of
//! executable content", and calls its own `<ccxml:accept>` example "legal on
//! any SCXML interpreter". SCE skips what it does not read, and its parser
//! says so (`scxml_children`: a foreign element whose local name collides
//! with a W3C element "is correctly skipped"). Five places matched the local
//! name alone and broke that (measured 2026-09-24): a top-level `<x:script/>`
//! got the whole document refused under §5.8, `<x:data>` became a variable,
//! `<x:else/>` split an `<if>`, `<x:raise>` raised, and a first child
//! `<x:state>` became the default initial state of a document with no state
//! by its id.

use sce_build::model::{Action, SCXMLModel};
use sce_build::parser::SCXMLParser;

fn parse(document: &str, label: &str) -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(document, label)
        .unwrap_or_else(|err| panic!("{label} parses: {err}"))
}

/// `state`'s `<onentry>` actions, in document order.
fn on_entry<'m>(model: &'m SCXMLModel, state: &str) -> Vec<&'m Action> {
    model
        .states
        .get(state)
        .unwrap_or_else(|| panic!("state {state}"))
        .on_entry_blocks
        .iter()
        .flatten()
        .collect()
}

#[test]
fn a_foreign_script_is_not_a_script() {
    let model = parse(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="urn:example:x"
       version="1.0" datamodel="ecmascript" initial="a">
  <x:script/>
  <state id="a"/>
</scxml>"#,
        "foreign_script",
    );
    assert!(
        !model.document_rejected,
        "an empty foreign element is no SCXML <script> lacking both src and content"
    );
    assert!(model.global_scripts.is_empty());
}

#[test]
fn a_foreign_data_is_not_a_variable() {
    let model = parse(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="urn:example:x"
       version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="real" expr="1"/>
    <x:data id="shadow" expr="2"/>
  </datamodel>
  <x:datamodel>
    <data id="phantom" expr="3"/>
  </x:datamodel>
  <state id="a"/>
</scxml>"#,
        "foreign_data",
    );
    let ids: Vec<&str> = model.variables.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(
        ids,
        ["real"],
        "neither a foreign <data> nor a <data> under a foreign <datamodel>"
    );
}

#[test]
fn a_foreign_else_does_not_split_an_if() {
    let model = parse(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="urn:example:x"
       version="1.0" datamodel="ecmascript" initial="a">
  <state id="a">
    <onentry>
      <if cond="true">
        <x:else/>
        <raise event="then"/>
      </if>
    </onentry>
  </state>
</scxml>"#,
        "foreign_else",
    );
    let actions = on_entry(&model, "a");
    let branch = actions
        .iter()
        .find(|action| action.action_type == "if")
        .expect("the <if>");
    let then: Vec<&str> = branch
        .then_actions
        .iter()
        .map(|a| a.event.as_str())
        .collect();
    assert_eq!(then, ["then"], "the <raise> stays in the <if>'s own branch");
    assert!(branch.else_actions.is_empty());
    assert!(branch.elseif_branches.is_empty());
}

#[test]
fn a_foreign_raise_raises_nothing() {
    let model = parse(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="urn:example:x"
       version="1.0" datamodel="ecmascript" initial="a">
  <state id="a">
    <onentry>
      <x:raise event="bogus"/>
      <x:action/>
      <raise event="real"/>
    </onentry>
  </state>
</scxml>"#,
        "foreign_raise",
    );
    let read: Vec<(&str, &str)> = on_entry(&model, "a")
        .iter()
        .map(|action| (action.action_type.as_str(), action.event.as_str()))
        .collect();
    assert_eq!(
        read,
        [("raise", "real")],
        "only SCXML's <raise> is read — not a foreign <raise>, and a foreign \
         <action> is not SCE's"
    );
    assert!(!model.events.contains("bogus"));
}

#[test]
fn a_foreign_state_is_not_the_default_initial_state() {
    let model = parse(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:x="urn:example:x"
       version="1.0" datamodel="null">
  <x:state id="ghost"/>
  <state id="real">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>"#,
        "foreign_state",
    );
    assert_eq!(
        model.initial, "real",
        "the first child STATE in document order (W3C SCXML 3.6)"
    );
}
