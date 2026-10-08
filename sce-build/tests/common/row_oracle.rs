// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What the tests of the review table's rows share: a population of attributes
//! read from a document, a way to move one of them, and a way to line a row up
//! with the parsed model it came from.
//!
//! The tests that use it (`an_attribute_that_changes_an_action_changes_its_row`,
//! `an_attribute_that_changes_a_transition_changes_its_row`) hold one rule: if
//! the parsed model of a node moved, the node's row moved. Neither lists "the
//! attributes that matter", because a list is the defect they guard against
//! written a second time: a cell is short when somebody chose what to print.
//! The population is whatever attributes the fixture document carries, read
//! with a real XML parser, so an attribute added to the fixture is mutated
//! without anyone remembering to ask.
#![allow(dead_code)]

use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

pub const SCXML_NS: &str = "http://www.w3.org/2005/07/scxml";
pub const SCE_NS: &str = "http://sce.dev/ext";

pub fn parse(text: &str) -> Option<SCXMLModel> {
    SCXMLParser::new().parse_string(text, "cells").ok()
}

/// The fixture itself, which has to parse; says why when it does not.
pub fn parse_fixture(text: &str) -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(text, "cells")
        .unwrap_or_else(|error| panic!("the fixture does not parse: {error:?}"))
}

/// One attribute to mutate: its byte range in the document and what it names.
pub struct Site {
    pub range: std::ops::Range<usize>,
    pub what: String,
}

/// Every attribute, outside the `sce:` namespace, that `keep` accepts, given
/// its element and its name.
///
/// `sce:` annotations are left out: they reach the table through `source`, which
/// other files hold.
pub fn sites(doc: &str, keep: impl Fn(&roxmltree::Node, &str) -> bool) -> Vec<Site> {
    let parsed = roxmltree::Document::parse(doc).expect("the fixture is well-formed XML");
    let mut out = Vec::new();
    for node in parsed.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            if attr.namespace() == Some(SCE_NS) || !keep(&node, attr.name()) {
                continue;
            }
            out.push(Site {
                range: attr.range_value(),
                what: format!("<{} {}>", node.tag_name().name(), attr.name()),
            });
        }
    }
    out
}

/// The value of a JSON tree at a `node_path` such as
/// `states.s.on_entry_blocks[0][3].then_actions[0]`.
///
/// Walk paths name the serialised model's own fields, which is what lets
/// a row be lined up with the model it came from.
pub fn at<'a>(tree: &'a serde_json::Value, node_path: &str) -> Option<&'a serde_json::Value> {
    let mut cursor = tree;
    for segment in node_path.split('.') {
        let (key, indices) = match segment.find('[') {
            Some(open) => (&segment[..open], &segment[open..]),
            None => (segment, ""),
        };
        if !key.is_empty() {
            cursor = cursor.get(key)?;
        }
        for index in indices.split(['[', ']']).filter(|s| !s.is_empty()) {
            cursor = cursor.get(index.parse::<usize>().ok()?)?;
        }
    }
    Some(cursor)
}
