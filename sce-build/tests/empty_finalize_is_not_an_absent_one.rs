// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.5.2: an EMPTY `<finalize>` carries a clause of its own, and the
// parser is where that clause becomes content.
//
// "If no executable content is specified, the SCXML Processor MUST update the
// data model each time an event is received from the child process ... for
// each item in the 'namelist' attribute and each such <param> element, the
// Processor MUST update the corresponding location as if by <assign> with any
// return value that has a name that matches ... Note that the automatic
// update does not take place if the <finalize> element is absent as opposed
// to empty."
//
// "As if by `<assign>`" is what makes this a parser answer: the clause names
// the executable content the empty element stands for, so synthesising it
// here lets every channel's existing `<finalize>` path run it unchanged.
//
// The seven-channel behaviour is witnessed by
// `integration_resources/empty_finalize_updates_the_location/`. This file
// witnesses the parser's half, and it exists as a separate test for a reason
// the mutation harness measured: a mutation to `parser.rs` does not reach a
// generated tree, because `sce_codegen_require` keeps a binary whose
// `verify-generator` check passes and that check is over the TEMPLATE tree.
// A test that links the parser directly is what makes this contract
// falsifiable — see `sce-build/tests/mutations/empty_finalize_is_not_an_absent_one.cases`.

use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

/// A document with one `<invoke>` carrying `invoke_attrs` and `finalize_xml`
/// verbatim, so a test spells only what it is about.
fn model_with_invoke(invoke_attrs: &str, finalize_xml: &str) -> SCXMLModel {
    let source = format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       name="probe" initial="s">
  <datamodel><data id="tally" expr="1"/><data id="other" expr="2"/></datamodel>
  <state id="s">
    <invoke type="scxml" id="inv" {invoke_attrs}>
      <content><scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="f">
        <final id="f"/></scxml></content>
      {finalize_xml}
    </invoke>
  </state>
</scxml>"#
    );
    SCXMLParser::new()
        .parse_string(&source, "probe")
        .unwrap_or_else(|err| panic!("probe document did not parse: {err:?}"))
}

fn finalize_of(model: &SCXMLModel) -> String {
    let invoke = model
        .states
        .get("s")
        .expect("the probe document has state s")
        .invokes
        .first()
        .expect("the probe document has one invoke")
        .clone();
    match &invoke {
        sce_build::model::Invoke::Scxml(info) => info.finalize_content.clone(),
        other => panic!("the probe invoke is not an scxml invoke: {other:?}"),
    }
}

#[test]
fn an_empty_finalize_stands_for_the_automatic_update() {
    let content = finalize_of(&model_with_invoke(r#"namelist="tally""#, "<finalize/>"));
    assert!(
        !content.is_empty(),
        "an empty `<finalize/>` on an invoke with a namelist produced no content — \
         §scxml-6.5.2 makes the empty element mean the automatic update, so the \
         parser has to stand it for the `<assign>` the clause names"
    );
    assert!(
        content.contains("tally = _event.data.tally"),
        "the synthesised content does not write the namelist item's location from \
         the matching return value:\n  {content}"
    );
}

#[test]
fn the_automatic_update_only_writes_when_the_name_is_there() {
    let content = finalize_of(&model_with_invoke(r#"namelist="tally""#, "<finalize/>"));
    assert!(
        content.contains("!== undefined"),
        "the synthesised content writes unconditionally — §scxml-6.5.2 says \"with \
         ANY return value that has a name that matches\", so an event carrying no \
         such name must leave the location alone. An unconditional write blanks the \
         parent's data model on every unrelated answer:\n  {content}"
    );
    assert!(
        !content.contains("=== undefined"),
        "the guard is inverted: the location would be written only when the event \
         does NOT carry the name:\n  {content}"
    );
}

#[test]
fn an_absent_finalize_stands_for_nothing() {
    let content = finalize_of(&model_with_invoke(r#"namelist="tally""#, ""));
    assert!(
        content.is_empty(),
        "an invoke with a namelist and NO `<finalize>` produced finalize content — \
         §scxml-6.5.2's note is a prohibition: \"the automatic update does not take \
         place if the <finalize> element is absent as opposed to empty\". Wiring the \
         update to the namelist rather than to the element is the defect:\n  {content}"
    );
}

#[test]
fn a_param_contributes_only_when_it_carries_a_location() {
    // The clause says "`<param>` children containing 'location' attributes",
    // and the name it matches on is the param's `name` — which may differ from
    // the location it writes. A `<param expr>` has no location to update.
    let with_location = finalize_of(&model_with_invoke(
        "",
        r#"<param name="tally" location="other"/><finalize/>"#,
    ));
    assert!(
        with_location.contains("other = _event.data.tally"),
        "a `<param location>` did not produce an update from its NAME into its \
         LOCATION:\n  {with_location}"
    );

    let expr_only = finalize_of(&model_with_invoke(
        "",
        r#"<param name="tally" expr="1"/><finalize/>"#,
    ));
    assert!(
        expr_only.is_empty(),
        "a `<param expr>` with no `location` produced an update — the clause names \
         only the params that carry a location:\n  {expr_only}"
    );
}
