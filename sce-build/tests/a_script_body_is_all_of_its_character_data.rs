// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `<script>` body is all of its character data (W3C SCXML 5.8).
//!
//! The parser read an element's text with `roxmltree::Node::text`, which
//! answers the FIRST child, and only when that child is text. A body that
//! opened with a comment therefore read as empty — and a global script with
//! no body is "neither src nor content", so the whole document was rejected
//! under 5.8 while generation exited 0 — and a comment inside a body dropped
//! everything after it (measured 2026-09-24). The interpreter's parser had
//! the second half of the same defect, which
//! `tests/parsing/ScriptBodyCharacterData_test.cpp` pins.

use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="x" expr="0"/>
    <data id="y" expr="0"/>
    <data id="z" expr="0"/>
  </datamodel>
  <script><!-- set up before anything runs -->z = 7;</script>
  <state id="a">
    <onentry>
      <script>x = 2; <!-- the rest of the body --> y = 3;</script>
      <script>x = 4; <![CDATA[ y = y < 5 ? 5 : y; ]]> z = 8;</script>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

fn model() -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(DOCUMENT, "script_bodies")
        .unwrap_or_else(|err| panic!("the document parses: {err}"))
}

#[test]
fn a_global_script_that_opens_with_a_comment_keeps_its_body() {
    let model = model();
    assert!(
        !model.document_rejected,
        "a script with a body was rejected as having none"
    );
    let bodies: Vec<&str> = model
        .global_scripts
        .iter()
        .map(|script| script.content.as_str())
        .collect();
    assert_eq!(bodies, ["z = 7;"]);
}

/// The comment splits the body into two text nodes, and both are the body.
/// The CDATA section is the control: the reader folds it into the text
/// around it, so that body was whole before and must stay whole.
#[test]
fn a_comment_inside_a_script_does_not_cut_it_off() {
    let model = model();
    let state = model.states.get("a").expect("state a");
    let bodies: Vec<&str> = state
        .on_entry_blocks
        .iter()
        .flatten()
        .filter(|action| action.action_type == "script")
        .map(|action| action.content.as_str())
        .collect();
    assert_eq!(
        bodies,
        ["x = 2;  y = 3;", "x = 4;  y = y < 5 ? 5 : y;  z = 8;"]
    );
}
