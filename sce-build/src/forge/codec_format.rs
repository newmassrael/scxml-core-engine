// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The format a CBOR or content-line codec document is written in
//! (docs/adr/0055, section 8).
//!
//! A codec entry that may be absent was spelt, until format 2, by leaving
//! `sce:required="true"` off it; since format 2 it is spelt by its type,
//! `optional<T>`, and a bare `T` is an entry that must be present. A document
//! cannot be told from one in the other spelling by its entries alone (a codec
//! of bare types only is a codec of optional entries in one and of required
//! entries in the other), so the spelling is a fact the document states on its
//! root, `sce:codec-format`.
//!
//! Format 2 is the only one read and generated. A document that declares none,
//! or `sce:codec-format="1"`, is refused, never read, and told its rewrite
//! ([`rewrite`]): the rewrite is computed from what format 1 says, entry by
//! entry. The refusal is part of the grammar and has no end; there is no
//! switch, no migration flag and no step that removes it.
//!
//! The two places that need the rewrite share one computation: the refusal of
//! the parser ([`judge`]), whose record carries the type of every entry, and
//! `sce-codegen codec-format` ([`rewritten_document`]), which prints the whole
//! document in format 2.

use serde::Serialize;

use crate::forge::error::{ForgeError, Located, ValidationError, XmlError};
use crate::forge::model::{AlgorithmValueType, SCE_NAMESPACE};
use crate::forge::parser::located;

/// The local name of the attribute that declares the format, on the root.
pub const ATTRIBUTE: &str = "codec-format";

/// The one format that is read and generated.
pub const CURRENT: &str = "2";

/// The format a document that declares none is written in, and the only other
/// one a document may name: refused, with its rewrite.
pub const SUPERSEDED: &str = "1";

/// What format 2 spells for one entry of a document written in format 1: the
/// `id` of the entry and its `sce:type`, as format 2 writes it (`uint8`, a
/// bare type, for an entry that was `sce:required="true"`; `optional<uint8>`
/// for one that was not). The `sce:required` attribute is removed from every
/// entry; no spelling of it is part of format 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryRewrite {
    pub id: String,
    /// The `sce:type` format 2 writes.
    #[serde(rename = "type")]
    pub sce_type: String,
}

/// One edit of the document's text: the bytes `range` become `text`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    range: std::ops::Range<usize>,
    text: String,
}

/// The document's own rewrite into format 2: the type of each entry, and the
/// edits that make them the document's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    /// Every entry of the document, in document order.
    pub entries: Vec<EntryRewrite>,
    edits: Vec<Edit>,
}

impl Rewrite {
    /// `source` with the edits applied: each entry's `sce:type` rewritten, every
    /// `sce:required` removed, and `sce:codec-format="2"` declared beside
    /// `sce:encoding`. Everything else — comments, spacing, the order of
    /// attributes — is the author's, byte for byte.
    ///
    /// `source` is the text the rewrite was read from.
    pub fn apply(&self, source: &str) -> String {
        let mut edits: Vec<&Edit> = self.edits.iter().collect();
        edits.sort_by_key(|e| std::cmp::Reverse(e.range.start));
        let mut text = source.to_string();
        for edit in edits {
            text.replace_range(edit.range.clone(), &edit.text);
        }
        text
    }
}

/// The attribute `sce:<local>` on `node`, read through the channel that
/// records the read.
fn attribute<'a, 'input>(
    node: &roxmltree::Node<'a, 'input>,
    local: &str,
) -> Option<roxmltree::Attribute<'a, 'input>> {
    crate::sce_attr::attribute(node, local)
}

/// The prefix `node`'s document binds to the SCE namespace, as it writes it.
fn sce_prefix<'a>(node: &roxmltree::Node<'a, '_>) -> &'a str {
    node.lookup_prefix(SCE_NAMESPACE).unwrap_or("sce")
}

/// The `<data>` elements of a codec's `<datamodel>` that are entries, in
/// document order: every one but the input frame (`sce:direction="in"`).
fn entries<'a, 'input>(root: &roxmltree::Node<'a, 'input>) -> Vec<roxmltree::Node<'a, 'input>> {
    let Some(datamodel) = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "datamodel")
    else {
        return Vec::new();
    };
    datamodel
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "data")
        .filter(|n| crate::sce_attr::read(n, "direction") != Some("in"))
        .collect()
}

/// The rewrite of the codec document whose root is `root`, from what format 1
/// says of each entry: `sce:required="true"` is its bare type, and no
/// `sce:required`, or `="false"`, is `optional<T>`.
///
/// An entry the rewrite cannot compute from what format 1 says is refused
/// instead, since a rewrite that guessed would be the silent flip the format
/// exists to prevent: a `sce:required` that is neither `true` nor `false`, and
/// an `sce:type` that is already `optional<T>` (format 1 has no such type — the
/// document is written in format 2 and does not say so).
pub fn rewrite(root: &roxmltree::Node, doc_name: &str) -> Result<Rewrite, Located<ForgeError>> {
    let mut rewrite = Rewrite {
        entries: Vec::new(),
        edits: Vec::new(),
    };
    let source = root.document().input_text();
    for node in entries(root) {
        let Some(id) = node.attribute("id") else {
            continue;
        };
        let Some(type_attr) = attribute(&node, "type") else {
            continue;
        };
        let written = type_attr.value().trim();
        if written.starts_with(AlgorithmValueType::OPTIONAL_PREFIX) {
            return Err(located(
                &node,
                doc_name,
                ValidationError::AttributeRuleViolated {
                    element: format!("Codec entry '{id}'"),
                    attr: "sce:type".into(),
                    // As the document spells it (`optional&lt;uint8&gt;`), which
                    // is what a record's `actual` is (SCE_ERROR_CONTRACT.md §2.1).
                    value: source
                        .get(type_attr.range_value())
                        .unwrap_or(type_attr.value())
                        .to_string(),
                    rule: format!(
                        "no optional type in a codec document of format 1: an entry that \
                         may be absent is written without sce:required there. A document \
                         that writes optional<T> is written in format 2, and says so with \
                         {}:{ATTRIBUTE}=\"{CURRENT}\" on its root",
                        sce_prefix(&node)
                    ),
                },
            ));
        }
        let required = attribute(&node, "required");
        let is_required = match required.as_ref().map(|a| a.value()) {
            None | Some("false") => false,
            Some("true") => true,
            Some(other) => {
                return Err(located(
                    &node,
                    doc_name,
                    ValidationError::InvalidAttribute {
                        element: format!("Codec entry '{id}'"),
                        attr: "sce:required".into(),
                        value: other.to_string(),
                        allowed: vec!["true".into(), "false".into()],
                    },
                ));
            }
        };
        rewrite.entries.push(EntryRewrite {
            id: id.to_string(),
            sce_type: if is_required {
                written.to_string()
            } else {
                format!("{}{written}>", AlgorithmValueType::OPTIONAL_PREFIX)
            },
        });
        if !is_required {
            // The value as the source spells it, so a character reference in
            // it is kept; the wrapper is escaped, as an attribute value is.
            let range = type_attr.range_value();
            let raw = source.get(range.clone()).unwrap_or(written);
            rewrite.edits.push(Edit {
                range,
                text: format!("optional&lt;{raw}&gt;"),
            });
        }
        if let Some(required) = required {
            // The attribute goes with the whitespace that set it apart.
            let range = required.range();
            let start = source[..range.start].trim_end().len();
            rewrite.edits.push(Edit {
                range: start..range.end,
                text: String::new(),
            });
        }
    }
    // A declared `1` is the value to change; with none declared, the
    // declaration stands beside `sce:encoding`, which every codec of these two
    // encodings carries.
    match (attribute(root, ATTRIBUTE), attribute(root, "encoding")) {
        (Some(declared), _) => rewrite.edits.push(Edit {
            range: declared.range_value(),
            text: CURRENT.to_string(),
        }),
        (None, Some(encoding)) => {
            let end = encoding.range().end;
            rewrite.edits.push(Edit {
                range: end..end,
                text: format!(" {}:{ATTRIBUTE}=\"{CURRENT}\"", sce_prefix(root)),
            });
        }
        (None, None) => {}
    }
    Ok(rewrite)
}

/// The refusal of a codec document that is not in format 2 (docs/adr/0055,
/// section 8): `Ok` when the root declares `sce:codec-format="2"`.
///
/// No `sce:codec-format`, or `="1"`, is [`ValidationError::CodecFormatSuperseded`],
/// placed on the root and carrying the rewrite of every entry; any other value
/// is `validation/invalid-attribute`, naming `2`.
pub fn judge(root: &roxmltree::Node, doc_name: &str) -> Result<(), Located<ForgeError>> {
    let declared = attribute(root, ATTRIBUTE);
    match declared.as_ref().map(|a| a.value()) {
        Some(CURRENT) => Ok(()),
        None | Some(SUPERSEDED) => {
            let rewrite = rewrite(root, doc_name)?;
            Err(located(
                root,
                doc_name,
                ValidationError::CodecFormatSuperseded {
                    declared: declared.as_ref().map(|a| a.value().to_string()),
                    attribute: format!("{}:{ATTRIBUTE}", sce_prefix(root)),
                    entries: rewrite.entries,
                },
            ))
        }
        Some(other) => Err(located(
            root,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "Codec".into(),
                attr: format!("{}:{ATTRIBUTE}", sce_prefix(root)),
                value: other.to_string(),
                allowed: vec![CURRENT.to_string()],
            },
        )),
    }
}

/// The whole document `content` in codec format 2, for `sce-codegen
/// codec-format`: the text itself with the rewrite applied when it is written in
/// format 1, and unchanged when it already declares format 2. `None` when the
/// document is no CBOR or content-line codec, which has no format to rewrite.
///
/// A document that holds a directive the preprocessor expands is refused
/// ([`crate::parser::reject_unexpanded_directives`]): the entries it expands to
/// are no text of this document, so a rewrite of the text would leave them
/// spelt as they were.
pub fn rewritten_document(
    content: &str,
    doc_name: &str,
) -> Result<Option<String>, Located<ForgeError>> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| Located::new(XmlError::from_roxmltree(&e).into(), doc_name, None, None))?;
    let root = doc.root_element();
    let is_codec = crate::sce_attr::read(&root, "kind") == Some("codec");
    let has_format = matches!(
        crate::sce_attr::read(&root, "encoding"),
        Some("cbor" | "content-line")
    );
    if !is_codec || !has_format {
        return Ok(None);
    }
    crate::parser::reject_unexpanded_directives(&root, doc_name)?;
    match crate::sce_attr::read(&root, ATTRIBUTE) {
        Some(CURRENT) => Ok(Some(content.to_string())),
        None | Some(SUPERSEDED) => Ok(Some(rewrite(&root, doc_name)?.apply(content))),
        // Any other value is no format: the refusal names the one there is.
        Some(_) => judge(&root, doc_name).map(|()| None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="codec" sce:encoding="cbor" name="probe" version="1.0">"#;

    fn doc(body: &str) -> String {
        format!("{HEAD}\n  <datamodel>\n{body}\n  </datamodel>\n</scxml>\n")
    }

    #[test]
    fn a_required_entry_is_its_bare_type_and_any_other_is_optional() {
        let text = doc(
            r#"    <data id="a" sce:type="uint8" sce:key="0" sce:required="true" sce:direction="out"/>
    <data id="b" sce:type="uint8" sce:key="1" sce:required="false" sce:direction="out"/>
    <data id="c" sce:type="string" sce:key="2" sce:max-size="4" sce:direction="out"/>"#,
        );
        let parsed = roxmltree::Document::parse(&text).unwrap();
        let rewrite = rewrite(&parsed.root_element(), "probe.scxml").expect("a rewrite");
        let types: Vec<(&str, &str)> = rewrite
            .entries
            .iter()
            .map(|e| (e.id.as_str(), e.sce_type.as_str()))
            .collect();
        assert_eq!(
            types,
            [
                ("a", "uint8"),
                ("b", "optional<uint8>"),
                ("c", "optional<string>")
            ]
        );
    }

    #[test]
    fn the_rewrite_changes_only_what_format_2_changes() {
        let text = doc(
            r#"    <data id="a" sce:type="uint8" sce:key="0" sce:required="true" sce:direction="out"/>
    <data id="b" sce:type="uint8" sce:key="1" sce:direction="out"/>"#,
        );
        let rewritten = rewritten_document(&text, "probe.scxml")
            .expect("a rewrite")
            .expect("a codec");
        let expected = text
            .replace(
                r#"sce:encoding="cbor""#,
                r#"sce:encoding="cbor" sce:codec-format="2""#,
            )
            .replace(r#" sce:required="true""#, "")
            .replace(
                r#"id="b" sce:type="uint8""#,
                r#"id="b" sce:type="optional&lt;uint8&gt;""#,
            );
        assert_eq!(rewritten, expected);
    }

    #[test]
    fn a_format_1_declaration_is_replaced_and_not_repeated() {
        let text = doc(r#"    <data id="a" sce:type="uint8" sce:key="0" sce:direction="out"/>"#)
            .replace(
                r#"sce:encoding="cbor""#,
                r#"sce:codec-format="1" sce:encoding="cbor""#,
            );
        let rewritten = rewritten_document(&text, "probe.scxml")
            .expect("a rewrite")
            .expect("a codec");
        assert_eq!(rewritten.matches("sce:codec-format").count(), 1);
        assert!(
            rewritten.contains(r#"sce:codec-format="2" sce:encoding="cbor""#),
            "{rewritten}"
        );
    }

    #[test]
    fn a_document_in_format_2_is_printed_as_it_is() {
        let text = doc(r#"    <data id="a" sce:type="uint8" sce:key="0" sce:direction="out"/>"#)
            .replace(
                r#"sce:encoding="cbor""#,
                r#"sce:encoding="cbor" sce:codec-format="2""#,
            );
        assert_eq!(
            rewritten_document(&text, "probe.scxml").expect("a rewrite"),
            Some(text)
        );
    }
}
