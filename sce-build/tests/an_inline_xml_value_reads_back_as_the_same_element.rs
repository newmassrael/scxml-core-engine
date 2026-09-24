// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An in-line XML value reads back as the element its author wrote.
//!
//! `<data>` and `<assign>` take element children as the value (W3C SCXML
//! 5.4, 5.9.3), and `<send>`'s `<content>` takes them as the payload (5.10);
//! the generator writes them out as text that each engine hands its XML
//! reader at run time. That text was written from the DECODED values:
//! `kind="x&quot;y"` came out `kind="x"y"` and `1 &lt; 2` came out `1 < 2` —
//! no XML reader accepts either. Every name lost its prefix, so `xml:lang`
//! became an attribute in no namespace and `<ext:item>` became
//! `<item xmlns="urn:example:ext">`; the declarations an author wrote inside
//! the value were dropped; and in a `<send>` payload an element holding only
//! whitespace was written empty (measured 2026-09-24). The C++ interpreter
//! prints the same value through pugixml, which escapes it and keeps every
//! name as written.
//!
//! The oracle reads the written text back twice, as the two kinds of reader
//! that meet it do. A namespace-aware reader must find the same elements,
//! attributes and text as in the source. A reader that does not resolve
//! prefixes — the DOM readers the backends ship mirror pugixml's default
//! mode — must find the same qualified names, and each namespace declaration
//! the author wrote on the element it was written on. A value is cut out of
//! its document, so a binding it inherits has to be declared inside it: a
//! declaration the source element does not carry is allowed only if it
//! binds what the source had in scope there.
//!
//! The fixture binds prefixes where the value cuts them off — on `<scxml>`
//! — and also where the default namespace shares a prefix's URI, where the
//! value rebinds a prefix, and where an element declares a binding only a
//! descendant uses: a new prefix, a rebound one, and a default whose URI a
//! prefix in scope already names. One branch declares again the very
//! binding the rest of the value inherits, so a declaration counts only for
//! the names beneath it. It writes a tab, a line feed and a carriage return
//! as references, which a reader keeps as those characters and would fold
//! into a space or a line feed if they reached it raw.

use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:ext="urn:example:ext"
       version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="doc"><root kind="x&quot;y" ext:note="a &amp; b" xml:lang="en">1 &lt; 2 &amp;&amp; 3 &gt; 2
        <blank tab="a&#x9;b" feed="a&#xA;b" return="a&#xD;b">   </blank>
        <plain xmlns="">bare&#xD;</plain>
        <ext:item ext:flag="on">x</ext:item>
        <twin xmlns="urn:example:ext" ext:mark="m"/>
        <scoped xmlns:ext="urn:example:other" ext:note="inner"/>
        <outer xmlns:deep="urn:example:deep"><deep:leaf/></outer>
        <relay xmlns:ext="urn:example:other"><ext:x/><back xmlns:ext="urn:example:ext" ext:y="1"/></relay>
        <ext:hub xmlns="urn:example:ext"><spoke/></ext:hub>
      </root></data>
  </datamodel>
  <state id="a">
    <onentry>
      <send event="report">
        <content><payload ext:tag="&lt;t&gt;">q &amp; a<gap> </gap><ext:mark/></payload></content>
      </send>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

fn model() -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(DOCUMENT, "inline_xml")
        .unwrap_or_else(|err| panic!("the document parses: {err}"))
}

/// Each attribute as (namespace, local name, value), in a fixed order.
fn attributes(node: roxmltree::Node<'_, '_>) -> Vec<(Option<String>, String, String)> {
    let mut all: Vec<_> = node
        .attributes()
        .map(|a| {
            (
                a.namespace().map(str::to_string),
                a.name().to_string(),
                a.value().to_string(),
            )
        })
        .collect();
    all.sort();
    all
}

/// The element's name as its text spells it — prefix included — which is
/// the name a reader that does not resolve prefixes knows it by.
fn qualified_name<'i>(node: roxmltree::Node<'_, 'i>) -> &'i str {
    let text = node.document().input_text();
    let start = &text[node.range().start + 1..];
    let end = start
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
        .unwrap_or(start.len());
    &start[..end]
}

/// Each attribute as (qualified name as spelled, value), in a fixed order.
fn qualified_attributes(node: roxmltree::Node<'_, '_>) -> Vec<(String, String)> {
    let text = node.document().input_text();
    let mut all: Vec<_> = node
        .attributes()
        .map(|a| (text[a.range_qname()].to_string(), a.value().to_string()))
        .collect();
    all.sort();
    all
}

/// The namespace declarations on the element itself, as (prefix, URI) with
/// `None` for the default: the bindings its parent did not have.
fn declarations(node: roxmltree::Node<'_, '_>) -> Vec<(Option<String>, String)> {
    let parent = node.parent_element();
    let mut all: Vec<_> = node
        .namespaces()
        .filter(|binding| binding.name() != Some("xml"))
        .filter(|binding| {
            !parent.is_some_and(|parent| {
                parent
                    .namespaces()
                    .any(|held| held.name() == binding.name() && held.uri() == binding.uri())
            })
        })
        .map(|binding| {
            (
                binding.name().map(str::to_string),
                binding.uri().to_string(),
            )
        })
        .collect();
    all.sort();
    all
}

/// The namespace `prefix` names at `node` — empty where it names none.
fn in_scope(node: roxmltree::Node<'_, '_>, prefix: Option<&str>) -> String {
    node.namespaces()
        .find(|binding| binding.name() == prefix)
        .map_or(String::new(), |binding| binding.uri().to_string())
}

/// The element's own character data.
fn text(node: roxmltree::Node<'_, '_>) -> String {
    node.children()
        .filter(|c| c.is_text())
        .filter_map(|c| c.text())
        .collect()
}

fn element_children<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    node.children().filter(|c| c.is_element()).collect()
}

/// Why `written` is not the element `expected` is, if it is not.
fn same_element(
    expected: roxmltree::Node<'_, '_>,
    written: roxmltree::Node<'_, '_>,
) -> Result<(), String> {
    let name = qualified_name(expected);
    if expected.tag_name() != written.tag_name() {
        return Err(format!(
            "<{name}> resolves to {:?} but was written as {:?}",
            expected.tag_name(),
            written.tag_name()
        ));
    }
    if name != qualified_name(written) {
        return Err(format!(
            "<{name}> was written as <{}>",
            qualified_name(written)
        ));
    }
    if attributes(expected) != attributes(written) {
        return Err(format!(
            "<{name}>'s attributes {:?} were written as {:?}",
            attributes(expected),
            attributes(written)
        ));
    }
    if qualified_attributes(expected) != qualified_attributes(written) {
        return Err(format!(
            "<{name}>'s attributes are spelled {:?} but were written {:?}",
            qualified_attributes(expected),
            qualified_attributes(written)
        ));
    }
    let (authored, carried) = (declarations(expected), declarations(written));
    if let Some(dropped) = authored.iter().find(|d| !carried.contains(d)) {
        return Err(format!(
            "<{name}> declares {dropped:?} but was written with {carried:?}"
        ));
    }
    if let Some((prefix, uri)) = carried
        .iter()
        .find(|(prefix, uri)| in_scope(expected, prefix.as_deref()) != *uri)
    {
        return Err(format!(
            "<{name}> was written declaring {prefix:?} as {uri:?}, which is not \
             what that binding is where the author wrote the element"
        ));
    }
    if text(expected) != text(written) {
        return Err(format!(
            "<{name}>'s text {:?} was written as {:?}",
            text(expected),
            text(written)
        ));
    }
    let (expected_children, written_children) =
        (element_children(expected), element_children(written));
    if expected_children.len() != written_children.len() {
        return Err(format!("<{name}>'s children were not all written"));
    }
    for (e, w) in expected_children.into_iter().zip(written_children) {
        same_element(e, w)?;
    }
    Ok(())
}

/// The value as written must parse, and read back as `source` — the first
/// element child of the element with `local` name in the document.
fn assert_reads_back(value: &str, local: &str) {
    let source = roxmltree::Document::parse(DOCUMENT).expect("the fixture parses");
    let expected = source
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == local)
        .and_then(|n| n.children().find(|c| c.is_element()))
        .unwrap_or_else(|| panic!("the fixture has a <{local}> holding an element"));
    let written = roxmltree::Document::parse(value)
        .unwrap_or_else(|err| panic!("the value written is not XML ({err}): {value}"));
    if let Err(why) = same_element(expected, written.root_element()) {
        panic!("{why}\n  written: {value}");
    }
}

#[test]
fn a_data_value_reads_back_as_the_element_written() {
    let model = model();
    let doc = model
        .variables
        .iter()
        .find(|v| v.id == "doc")
        .expect("the <data> is a variable");
    assert_reads_back(&doc.content, "data");
}

#[test]
fn a_send_payload_reads_back_as_the_element_written() {
    let model = model();
    let state = model.states.get("a").expect("state a");
    let send = state
        .on_entry_blocks
        .iter()
        .flatten()
        .find(|action| action.action_type == "send")
        .expect("the <send>");
    assert_reads_back(&send.content, "content");
}
