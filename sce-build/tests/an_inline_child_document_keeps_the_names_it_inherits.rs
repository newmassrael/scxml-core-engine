// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An in-line child document keeps the names it inherits.
//!
//! `<invoke>`'s `<content>` may hold a whole `<scxml>` (W3C SCXML 6.4), and
//! the generator parses it as a machine of its own. It is cut out of the
//! enclosing document, so the namespace bindings its names inherit from there
//! have to come with it. They did not: the SCXML namespace was added to the
//! child only when its text held no `xmlns=` anywhere, and no other binding
//! was added at all. A child whose data carried `<books xmlns="">` arrived in
//! no namespace, and one using a prefix its parent declared named an unbound
//! prefix. Both failed to parse, the generator said so on stderr and went on
//! without the child, and the parent's generated code still included the
//! child's header (measured 2026-09-24).
//!
//! The oracle is the child the generator parsed: it exists, and it is the
//! machine its author wrote.

use sce_build::model::{Invoke, SCXMLModel};
use sce_build::parser::SCXMLParser;

/// A child that declares no namespace, and whose data un-declares the
/// default one inside it.
const UNDECLARED_DEFAULT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s0">
  <state id="s0">
    <invoke id="kid" type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml version="1.0" datamodel="ecmascript" initial="c">
          <datamodel>
            <data id="d"><books xmlns=""><book title="t"/></books></data>
          </datamodel>
          <final id="c"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.kid" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#;

/// A child using a prefix only its parent declares.
const PARENT_PREFIX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:ext="urn:example:ext" version="1.0" datamodel="ecmascript" initial="s0">
  <state id="s0">
    <invoke id="kid" type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml version="1.0" datamodel="ecmascript" initial="c">
          <datamodel>
            <data id="d"><item ext:note="x"/></data>
          </datamodel>
          <final id="c"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.kid" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#;

fn parse(document: &str, label: &str) -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(document, label)
        .unwrap_or_else(|err| panic!("{label} parses: {err}"))
}

/// The child machine the generator parsed for `s0`'s `<invoke>`, and the
/// text it parsed it from.
fn inline_child(model: &SCXMLModel) -> (&SCXMLModel, &str) {
    let state = model.states.get("s0").expect("state s0");
    let info = state
        .invokes
        .iter()
        .find_map(|invoke| match invoke {
            Invoke::Scxml(info) => Some(info),
            _ => None,
        })
        .expect("the <invoke> starts an SCXML session");
    let text = info
        .inline_child_xml
        .as_deref()
        .expect("the in-line child's text is kept");
    let child = info
        .inline_child
        .as_deref()
        .unwrap_or_else(|| panic!("the in-line child did not parse:\n{text}"));
    (child, text)
}

/// The in-line value of `child`'s `<data id="d">`.
fn data_value(child: &SCXMLModel) -> &str {
    &child
        .variables
        .iter()
        .find(|variable| variable.id == "d")
        .expect("the child's <data id=\"d\">")
        .content
}

#[test]
fn a_child_whose_data_undeclares_the_default_namespace_is_parsed() {
    let model = parse(UNDECLARED_DEFAULT, "undeclared_default");
    let (child, _) = inline_child(&model);
    assert!(
        child.states.contains_key("c"),
        "the child is the machine its author wrote"
    );
    assert_eq!(
        data_value(child),
        r#"<books xmlns=""><book title="t"></book></books>"#
    );
}

#[test]
fn a_child_using_a_prefix_its_parent_declares_is_parsed() {
    let model = parse(PARENT_PREFIX, "parent_prefix");
    let (child, _) = inline_child(&model);
    assert!(
        child.states.contains_key("c"),
        "the child is the machine its author wrote"
    );
    assert_eq!(
        data_value(child),
        r#"<item xmlns="http://www.w3.org/2005/07/scxml" xmlns:ext="urn:example:ext" ext:note="x"></item>"#
    );
}

/// The declarations go on the child root's first line and nowhere else, so
/// a position in the child is a position its author can find.
#[test]
fn the_child_keeps_every_line_its_author_wrote() {
    for (document, label) in [
        (UNDECLARED_DEFAULT, "undeclared_default"),
        (PARENT_PREFIX, "parent_prefix"),
    ] {
        let model = parse(document, label);
        let (_, text) = inline_child(&model);
        let start = document.find("<scxml version").expect("the child's root");
        let end = document.find("</scxml>").expect("the child's end") + "</scxml>".len();
        let authored: Vec<&str> = document[start..end].lines().collect();
        let body = text
            .strip_prefix("<?xml version=\"1.0\"?>\n\n")
            .expect("the child document's prologue");
        let written: Vec<&str> = body.lines().collect();
        assert_eq!(
            written.len(),
            authored.len(),
            "{label}: the child has the lines its author wrote"
        );
        assert_eq!(
            written[1..],
            authored[1..],
            "{label}: only the root's first line gains declarations"
        );
        let first = authored[0]
            .strip_prefix("<scxml")
            .expect("the root's first line opens it");
        assert!(
            written[0].starts_with("<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"")
                && written[0].ends_with(first),
            "{label}: the root's first line is the author's, with the inherited \
             bindings declared after its name: {}",
            written[0]
        );
    }
}
