// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The attributes the SCXML Recommendation defines for each element, and the
//! refusal of one it does not.
//!
//! # Why
//!
//! `schemas/sce-forge.xsd` admits every W3C element laxly and leaves their
//! structure to the parser, and the parsers read the attributes they know by
//! name: an attribute the Recommendation does not define for an element is
//! therefore never asked for, and nothing said so. Measured 2026-10-11, on a
//! document an AI wrote for the platform: `<state id="main" type="parallel">`
//! built with exit 0 (`check` and `generate`) and meant an ordinary compound
//! state, so only the first of its regions was ever entered and six of the
//! eight test cases failed. `<parallel>` is the element for concurrent
//! regions; `<state>` takes `id` and `initial` and nothing else.
//!
//! The same silence was in the repository: a `<send expr="_event.data"/>`
//! (a `<send>` has no `expr`, so the data it was meant to carry was never
//! sent), and three W3C conformance fixtures whose converter had written a
//! condition as `expr` on a `<transition>`.
//!
//! # What this is, and is not
//!
//! §scxml-A-1: a conforming document "MUST conform to all the syntactic
//! constraints defined in this specification, including those contained in
//! the schema". The unqualified attributes of an element are the ones its
//! attribute table lists, so an unqualified attribute outside the table is a
//! document that does not conform. [`unknown_scxml_attributes`] answers with
//! every such attribute; [`refuse_unknown_scxml_attributes`] refuses the
//! document.
//!
//! * **Unqualified attributes only.** A qualified one belongs to its own
//!   namespace: `sce:` has its own check (`crate::read_ledger`), `xml:` is
//!   the XML Recommendation's, and the W3C conformance suite's `conf:` is
//!   rewritten before a fixture is built.
//! * **A list, unlike `sce:`.** What an element takes of `sce:` is whatever
//!   the parsers read, so no list is kept beside them. For these the
//!   Recommendation IS the list, independent of what the parsers happen to
//!   read, and `tests/an_scxml_attribute_outside_the_standard_is_refused.rs`
//!   holds the table below to the specification ledger table by table.
//! * **Inline payload is not judged.** A `<data>` or `<content>` carries a
//!   value as written (an XML datamodel's tree, an inline child machine that
//!   its own parse judges), so nothing under one is read here.

use roxmltree::Node;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::near_miss::near_misses;

const SCXML_NAMESPACE: &str = "http://www.w3.org/2005/07/scxml";

/// The unqualified attributes the Recommendation defines for each element,
/// in the order its attribute table lists them. `xmlns` is a namespace
/// declaration, not an attribute a parser sees, so it is not listed.
///
/// An element with no entry is not an SCXML element this table knows and is
/// left alone, which is what lets an extension element in the SCXML
/// namespace be refused by the parser that reads it rather than here.
pub const SCXML_ATTRIBUTES: &[(&str, &[&str])] = &[
    (
        "scxml",
        &["initial", "name", "version", "datamodel", "binding"],
    ),
    ("state", &["id", "initial"]),
    ("parallel", &["id"]),
    ("transition", &["event", "cond", "target", "type"]),
    ("initial", &[]),
    ("final", &["id"]),
    ("onentry", &[]),
    ("onexit", &[]),
    ("history", &["id", "type"]),
    ("raise", &["event"]),
    ("if", &["cond"]),
    ("elseif", &["cond"]),
    ("else", &[]),
    ("foreach", &["array", "item", "index"]),
    ("log", &["label", "expr"]),
    ("datamodel", &[]),
    ("data", &["id", "src", "expr"]),
    ("assign", &["location", "expr"]),
    ("donedata", &[]),
    ("content", &["expr"]),
    ("param", &["name", "expr", "location"]),
    ("script", &["src"]),
    (
        "send",
        &[
            "event",
            "eventexpr",
            "target",
            "targetexpr",
            "type",
            "typeexpr",
            "id",
            "idlocation",
            "delay",
            "delayexpr",
            "namelist",
        ],
    ),
    ("cancel", &["sendid", "sendidexpr"]),
    (
        "invoke",
        &[
            "type",
            "typeexpr",
            "src",
            "srcexpr",
            "id",
            "idlocation",
            "namelist",
            "autoforward",
        ],
    ),
    ("finalize", &[]),
];

/// An unqualified attribute an SCXML element carries that the Recommendation
/// does not define for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAnScxmlAttribute {
    /// The element's local name, as the author wrote it.
    pub element: String,
    /// The attribute's name as the author wrote it — the record's `actual`.
    pub attribute: String,
    /// 1-based row and column of the attribute.
    pub row: u32,
    pub col: u32,
    /// Every attribute the Recommendation defines for the element.
    pub takes: Vec<String>,
    /// The nearest of them to the one written, at most
    /// [`crate::near_miss::SUGGESTION_LIMIT`].
    pub near: Vec<String>,
    /// What to write instead, for a mistake whose shape is known.
    pub hint: Option<&'static str>,
}

/// What an author most likely meant by a mistake whose shape is known.
fn hint_for(element: &str, attribute: &str, value: &str) -> Option<&'static str> {
    match (element, attribute) {
        ("state", "type") if value.trim() == "parallel" => Some(
            "concurrent regions are written <parallel id=\"…\">, not <state type=\"parallel\">; \
             read as an ordinary state it would enter only its first child",
        ),
        ("state", "type") => Some(
            "a <state> has no type; <parallel> holds concurrent regions and <final> an end state",
        ),
        ("send", "expr") => Some(
            "a <send> carries its data in <param> or <content>, or by namelist; it has no expr",
        ),
        _ => None,
    }
}

fn takes(element: &str) -> Option<&'static [&'static str]> {
    SCXML_ATTRIBUTES
        .iter()
        .find(|(name, _)| *name == element)
        .map(|(_, attributes)| *attributes)
}

/// The unqualified attributes of the SCXML elements at and below `root` that
/// the Recommendation does not define for them, in document order.
pub fn unknown_scxml_attributes(root: &Node) -> Vec<NotAnScxmlAttribute> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out
}

fn collect(node: &Node, out: &mut Vec<NotAnScxmlAttribute>) {
    if !node.is_element() {
        return;
    }
    let is_scxml = node.tag_name().namespace() == Some(SCXML_NAMESPACE);
    let name = node.tag_name().name();
    if is_scxml {
        if let Some(takes) = takes(name) {
            for attribute in node.attributes().filter(|a| a.namespace().is_none()) {
                if takes.contains(&attribute.name()) {
                    continue;
                }
                let at = node.document().text_pos_at(attribute.range().start);
                out.push(NotAnScxmlAttribute {
                    element: name.to_string(),
                    attribute: attribute.name().to_string(),
                    row: at.row,
                    col: at.col,
                    takes: takes.iter().map(|t| t.to_string()).collect(),
                    near: near_misses(attribute.name(), takes.iter().copied()),
                    hint: hint_for(name, attribute.name(), attribute.value()),
                });
            }
        }
        // A value carried as written is its own reader's to judge.
        if matches!(name, "data" | "content") {
            return;
        }
    }
    for child in node.children() {
        collect(&child, out);
    }
}

/// Refuse the document when any SCXML element carries an attribute the
/// Recommendation does not define for it. One refusal for the whole list:
/// each fans out to a diagnostic of its own, at the attribute.
pub(crate) fn refuse_unknown_scxml_attributes(
    root: &Node,
    doc_name: &str,
) -> Result<(), Located<ForgeError>> {
    let unknown = unknown_scxml_attributes(root);
    let Some(first) = unknown.first() else {
        return Ok(());
    };
    let (row, col) = (first.row, first.col);
    Err(Located::new(
        ValidationError::UnknownScxmlAttributes(unknown).into(),
        doc_name,
        Some(row),
        Some(col),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = "http://www.w3.org/2005/07/scxml";

    fn unknown(body: &str) -> Vec<NotAnScxmlAttribute> {
        let text = format!(
            r#"<scxml xmlns="{NS}" xmlns:sce="http://sce.dev/ext" version="1.0">{body}</scxml>"#
        );
        let document = roxmltree::Document::parse(&text).unwrap();
        unknown_scxml_attributes(&document.root_element())
    }

    #[test]
    fn a_state_of_type_parallel_is_refused_with_the_element_that_means_it() {
        let found = unknown(r#"<state id="main" type="parallel"><state id="a"/></state>"#);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].element, "state");
        assert_eq!(found[0].attribute, "type");
        assert_eq!(found[0].takes, vec!["id", "initial"]);
        assert!(
            found[0].hint.is_some_and(|h| h.contains("<parallel")),
            "{:?}",
            found[0]
        );
    }

    #[test]
    fn a_misspelt_attribute_names_the_nearest_the_element_takes() {
        let found = unknown(r#"<state id="a"><transition evnt="go" target="a"/></state>"#);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].near, vec!["event"]);
    }

    #[test]
    fn what_the_recommendation_defines_is_not_refused() {
        assert!(unknown(
            r#"<state id="a" initial="b" sce:req="R1"><state id="b"/>
                 <transition event="e" cond="true" target="b" type="internal"/>
               </state>
               <parallel id="p"><state id="x"/></parallel>"#
        )
        .is_empty());
    }

    #[test]
    fn a_qualified_attribute_belongs_to_its_own_namespace() {
        assert!(unknown(r#"<state id="a" sce:anything="x" xml:lang="en"/>"#).is_empty());
    }

    #[test]
    fn a_value_carried_as_written_is_not_judged() {
        assert!(unknown(
            r#"<datamodel><data id="d"><item bogus="1"/></data></datamodel>
               <state id="a"><onentry><send event="e"><content><a type="x"/></content></send></onentry></state>"#
        )
        .is_empty());
    }

    #[test]
    fn each_unknown_attribute_is_found_at_its_own_position() {
        let found = unknown(
            "<state id=\"a\"\n  type=\"x\"><onentry><send event=\"e\"\n    expr=\"1\"/></onentry></state>",
        );
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(
            (found[0].element.as_str(), found[0].attribute.as_str()),
            ("state", "type")
        );
        assert_eq!(
            (found[1].element.as_str(), found[1].attribute.as_str()),
            ("send", "expr")
        );
        assert!(found[0].row < found[1].row);
    }
}
