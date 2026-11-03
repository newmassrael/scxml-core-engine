// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A bound written as an attribute of a declaration rather than in its type
//! (docs/adr/0086, section 1.3).
//!
//! Every bound of a value type is written in the type, at every depth
//! (`list<string[8]>[4]`, `string[merge_text=96]`). Five attributes stated the
//! outermost bound before: `sce:max-size` and `sce:max-count` on an event
//! schema's field, `sce:capacity` on a `sce-static` statechart's variable, and
//! an algorithm's `capacity=` on a local and `returns-max-size=` on its return,
//! each with the `sce:limit` that names it. They are synonyms of the bracket on
//! the outermost type they bound, read so by the parser
//! ([`crate::forge::value_type::WrittenType::with_outer_bound`]), and each is
//! reported here with the rewrite: `validation/bound-attribute-superseded`,
//! whose record does not stop the run unless `--lint` makes it.
//!
//! The attributes that are not bounds of a value type are none of this: a
//! binary codec field's `sce:max-size` and a `<sce:repeat max-count>` describe a
//! foreign frame (docs/adr/0077), a content-line entry's `sce:max-count` belongs
//! to its codec, and `sce:capacity` on `<scxml>` is the depth of the event queue.
//! A position is therefore found by what declares it, not by the attribute's
//! name.
//!
//! The two readers of a document's legacy bounds share one walk: the records
//! `check` and `generate` report ([`superseded`]), and `sce-codegen
//! bound-spelling` ([`rewritten_document`]), which prints the document with every
//! such bound moved into its type and every other byte as written.

use crate::forge::error::{ForgeError, Located, ValidationError, XmlError};
use crate::forge::model::{parse_int_literal, SCE_NAMESPACE};
use crate::forge::parser::located;
use crate::forge::value_type::{Bound, TypeRefusal, WrittenType};

/// A declaration of a value whose bound an attribute may state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Site {
    /// A `<data>` field of an event schema: `sce:max-size` on a text or a
    /// byte string, `sce:max-count` on a list.
    SchemaField,
    /// A `<data>` variable of a `sce-static` statechart: `sce:capacity`.
    Variable,
    /// An algorithm's `<sce:var>` local: `capacity`.
    Local,
    /// An algorithm's signature `<sce:return>`: `returns-max-size`.
    Return,
}

impl Site {
    /// The attribute that carries the type: `(namespace, local name)`.
    fn type_attr(self) -> (Option<&'static str>, &'static str) {
        match self {
            Self::SchemaField | Self::Variable => (Some(SCE_NAMESPACE), "type"),
            Self::Local | Self::Return => (None, "type"),
        }
    }

    /// The attribute that states the outermost bound of `ty`, or `None` when
    /// the position states none for a type of that constructor.
    fn bound_attr(self, ty: &WrittenType) -> Option<(Option<&'static str>, &'static str)> {
        match self {
            Self::SchemaField => match outermost_bounded(ty)? {
                WrittenType::List { .. } => Some((Some(SCE_NAMESPACE), "max-count")),
                _ => Some((Some(SCE_NAMESPACE), "max-size")),
            },
            Self::Variable => Some((Some(SCE_NAMESPACE), "capacity")),
            Self::Local => Some((None, "capacity")),
            Self::Return => Some((None, "returns-max-size")),
        }
    }
}

/// The outermost text, byte string or list of `ty`, seen through every
/// `optional<…>`: what a legacy attribute bounds.
fn outermost_bounded(ty: &WrittenType) -> Option<&WrittenType> {
    match ty {
        WrittenType::Text { .. } | WrittenType::Bytes { .. } | WrittenType::List { .. } => Some(ty),
        WrittenType::Optional { elem } => outermost_bounded(elem),
        WrittenType::Scalar(_) | WrittenType::Record { .. } => None,
    }
}

/// The attribute `(namespace, local)` of `node`.
fn attribute<'a, 'input>(
    node: &roxmltree::Node<'a, 'input>,
    (namespace, local): (Option<&str>, &str),
) -> Option<roxmltree::Attribute<'a, 'input>> {
    node.attributes()
        .find(|a| a.namespace() == namespace && a.name() == local)
}

/// `attr` as the document spells it, its prefix included.
fn spelled(node: &roxmltree::Node, attr: &roxmltree::Attribute) -> String {
    match attr.namespace() {
        Some(ns) => match node.lookup_prefix(ns) {
            Some(prefix) => format!("{prefix}:{}", attr.name()),
            None => attr.name().to_string(),
        },
        None => attr.name().to_string(),
    }
}

/// The element as a record names it: `<data id="tag">`, `<sce:var name="buf">`,
/// `<sce:return>`.
fn element_label(node: &roxmltree::Node) -> String {
    let tag = match node.tag_name().namespace() {
        Some(ns) if ns == SCE_NAMESPACE => {
            let prefix = node.lookup_prefix(ns).unwrap_or("sce");
            format!("{prefix}:{}", node.tag_name().name())
        }
        _ => node.tag_name().name().to_string(),
    };
    match (node.attribute("id"), node.attribute("name")) {
        (Some(id), _) => format!("<{tag} id=\"{id}\">"),
        (None, Some(name)) => format!("<{tag} name=\"{name}\">"),
        (None, None) => format!("<{tag}>"),
    }
}

fn is_sce(node: &roxmltree::Node, name: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(SCE_NAMESPACE)
        && node.tag_name().name() == name
}

fn is_scxml(node: &roxmltree::Node, name: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() != Some(SCE_NAMESPACE)
        && node.tag_name().name() == name
}

/// Every declaration of the document whose bound an attribute may state, in
/// document order, with the position it is.
fn sites<'a, 'input>(
    root: &roxmltree::Node<'a, 'input>,
) -> Vec<(roxmltree::Node<'a, 'input>, Site)> {
    let kind = attribute(root, (Some(SCE_NAMESPACE), "kind")).map(|a| a.value().trim());
    match kind {
        Some("event-schema") => root
            .children()
            .filter(|n| is_scxml(n, "datamodel"))
            .flat_map(|datamodel| datamodel.children().filter(|n| is_scxml(n, "data")))
            .map(|data| (data, Site::SchemaField))
            .collect(),
        Some("algorithm") => root
            .descendants()
            .filter_map(|n| {
                if is_sce(&n, "var") {
                    Some((n, Site::Local))
                } else if is_sce(&n, "return")
                    && n.parent().is_some_and(|p| is_sce(&p, "signature"))
                {
                    Some((n, Site::Return))
                } else {
                    None
                }
            })
            .collect(),
        Some(_) => Vec::new(),
        None if root.attribute("datamodel").map(str::trim) == Some("sce-static") => {
            // Every variable at any depth, and nothing inside a kind declared
            // in place, whose fields are that kind's and no variable.
            let mut out = Vec::new();
            let mut stack: Vec<roxmltree::Node> =
                root.children().filter(|n| n.is_element()).collect();
            stack.reverse();
            while let Some(node) = stack.pop() {
                // A machine an `<invoke>` holds in place has a data model of
                // its own.
                if is_scxml(&node, "scxml") {
                    continue;
                }
                if is_scxml(&node, "data") {
                    if attribute(&node, (Some(SCE_NAMESPACE), "kind")).is_none() {
                        out.push((node, Site::Variable));
                    }
                    continue;
                }
                let mut children: Vec<roxmltree::Node> =
                    node.children().filter(|n| n.is_element()).collect();
                children.reverse();
                stack.extend(children);
            }
            out
        }
        None => Vec::new(),
    }
}

/// One edit of the document's text: the bytes `range` become `text`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    range: std::ops::Range<usize>,
    text: String,
}

/// What one declaration's legacy bound is.
enum Finding {
    /// The bound is an attribute: its record, and the edits that move it into
    /// the type.
    Superseded {
        record: Located<ForgeError>,
        edits: Vec<Edit>,
    },
    /// The type writes the bracket and an attribute bounds it too: refused by
    /// the parser, and not rewritten here.
    BothForms(Located<ForgeError>),
}

/// The removal of `attr` from the text, with the whitespace that set it apart.
fn removal(source: &str, attr: &roxmltree::Attribute) -> Edit {
    let range = attr.range();
    let start = source[..range.start].trim_end().len();
    Edit {
        range: start..range.end,
        text: String::new(),
    }
}

/// `text` as an attribute value spells it.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The legacy bound of one declaration, or `None` when it writes none, or one
/// this reading cannot rewrite: a number that is no bound, a type that is no
/// type, an attribute beside a type with no length. Each of those is the
/// parser's to refuse, in its own words.
fn finding(node: &roxmltree::Node, site: Site, doc_name: &str) -> Option<Finding> {
    let source = node.document().input_text();
    let type_attr = attribute(node, site.type_attr())?;
    let written = WrittenType::read(type_attr.value()).ok()?;
    let bound_attr = attribute(node, site.bound_attr(&written)?)?;
    let max = parse_int_literal(bound_attr.value().trim())
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)?;
    let limit_attr = attribute(node, (Some(SCE_NAMESPACE), "limit"));
    let bound = match &limit_attr {
        Some(limit) => Bound::named(limit.value().trim(), max),
        None => Bound::of(max),
    };
    let element = element_label(node);
    let mut attrs = vec![spelled(node, &bound_attr)];
    attrs.extend(limit_attr.as_ref().map(|a| spelled(node, a)));
    let sce_type = match written.with_outer_bound(bound) {
        Ok(ty) => ty.to_string(),
        Err(refusal @ TypeRefusal::BothForms { .. }) => {
            return Some(Finding::BothForms(located(
                node,
                doc_name,
                ValidationError::IncompatibleAttributes {
                    element,
                    detail: format!("{}: {}", attrs[0], refusal.rule()),
                },
            )));
        }
        Err(_) => return None,
    };
    let mut edits = vec![Edit {
        range: type_attr.range_value(),
        text: escaped(&sce_type),
    }];
    edits.push(removal(source, &bound_attr));
    edits.extend(limit_attr.as_ref().map(|a| removal(source, a)));
    let record = located(
        node,
        doc_name,
        ValidationError::BoundAttributeSuperseded {
            element,
            attrs,
            type_attr: spelled(node, &type_attr),
            sce_type,
        },
    );
    Some(Finding::Superseded { record, edits })
}

/// Every legacy bound of the document, in document order.
fn findings(root: &roxmltree::Node, doc_name: &str) -> Vec<Finding> {
    sites(root)
        .into_iter()
        .filter_map(|(node, site)| finding(&node, site, doc_name))
        .collect()
}

/// The records of every bound the document `content` writes as an attribute,
/// in document order: `validation/bound-attribute-superseded`, each placed at
/// the attribute and carrying the rewrite. Empty for a document that writes
/// none, and for one that does not parse, whose refusal is the parser's. A
/// declaration that writes its bound both ways has no record here: the parser
/// refuses it.
pub fn superseded(content: &str, doc_name: &str) -> Vec<Located<ForgeError>> {
    let Ok(doc) = roxmltree::Document::parse(content) else {
        return Vec::new();
    };
    findings(&doc.root_element(), doc_name)
        .into_iter()
        .filter_map(|f| match f {
            Finding::Superseded { record, .. } => Some(record),
            Finding::BothForms(_) => None,
        })
        .collect()
}

/// The whole document `content` with every bound it writes as an attribute
/// moved into the type of its declaration, for `sce-codegen bound-spelling`:
/// each type rewritten with its outermost bound in the bracket, and the bound
/// attribute and the `sce:limit` beside it removed. Every other byte is the
/// author's, so a document that writes no such bound is printed as it is.
///
/// Refused when a declaration writes its bound both ways, which the rewrite
/// cannot settle, and when the document holds a directive the preprocessor
/// expands ([`crate::parser::reject_unexpanded_directives`]): the declarations
/// it expands to are no text of this document.
pub fn rewritten_document(content: &str, doc_name: &str) -> Result<String, Located<ForgeError>> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| Located::new(XmlError::from_roxmltree(&e).into(), doc_name, None, None))?;
    let root = doc.root_element();
    crate::parser::reject_unexpanded_directives(&root, doc_name)?;
    let mut edits: Vec<Edit> = Vec::new();
    for finding in findings(&root, doc_name) {
        match finding {
            Finding::Superseded { edits: own, .. } => edits.extend(own),
            Finding::BothForms(refusal) => return Err(refusal),
        }
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.range.start));
    let mut text = content.to_string();
    for edit in edits {
        text.replace_range(edit.range, &edit.text);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = r#"xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext""#;

    fn schema(fields: &str) -> String {
        format!(
            "<scxml {NS} sce:kind=\"event-schema\" sce:event-name=\"probe.sent\" name=\"probe\" version=\"1.0\">\n  <datamodel>\n{fields}\n  </datamodel>\n</scxml>\n"
        )
    }

    fn statechart(datamodel: &str, body: &str) -> String {
        format!(
            "<scxml {NS} datamodel=\"{datamodel}\" initial=\"a\" name=\"m\" version=\"1.0\">\n  <datamodel>\n{body}\n  </datamodel>\n  <state id=\"a\"/>\n</scxml>\n"
        )
    }

    fn algorithm(signature: &str, body: &str) -> String {
        format!(
            "<scxml {NS} sce:kind=\"algorithm\" name=\"alg\" version=\"1.0\">\n  <sce:signature>\n{signature}\n  </sce:signature>\n  <sce:body>\n{body}\n  </sce:body>\n</scxml>\n"
        )
    }

    fn fix_of(record: &Located<ForgeError>) -> (String, Vec<String>, String) {
        match &record.error {
            ForgeError::Validation(v) => match &**v {
                ValidationError::BoundAttributeSuperseded {
                    element,
                    attrs,
                    sce_type,
                    ..
                } => (element.clone(), attrs.clone(), sce_type.clone()),
                other => panic!("not a superseded bound: {other:?}"),
            },
            other => panic!("not a validation error: {other:?}"),
        }
    }

    #[test]
    fn a_schema_text_bound_moves_into_its_type() {
        let text = schema(
            r#"    <data id="tag" sce:type="string" sce:max-size="32" sce:direction="in"/>"#,
        );
        let records = superseded(&text, "probe.scxml");
        assert_eq!(records.len(), 1);
        assert_eq!(
            fix_of(&records[0]),
            (
                "<data id=\"tag\">".to_string(),
                vec!["sce:max-size".to_string()],
                "string[32]".to_string()
            )
        );
        assert_eq!(
            rewritten_document(&text, "probe.scxml").unwrap(),
            text.replace(
                r#"sce:type="string" sce:max-size="32""#,
                r#"sce:type="string[32]""#
            )
        );
    }

    #[test]
    fn a_named_bound_keeps_its_name_in_the_bracket() {
        let text = schema(
            r#"    <data id="tag" sce:type="optional&lt;string&gt;" sce:max-size="96" sce:limit="merge_text" sce:direction="in"/>"#,
        );
        let records = superseded(&text, "probe.scxml");
        assert_eq!(
            fix_of(&records[0]),
            (
                "<data id=\"tag\">".to_string(),
                vec!["sce:max-size".to_string(), "sce:limit".to_string()],
                "optional<string[merge_text=96]>".to_string()
            )
        );
        assert_eq!(
            rewritten_document(&text, "probe.scxml").unwrap(),
            text.replace(
                r#"sce:type="optional&lt;string&gt;" sce:max-size="96" sce:limit="merge_text""#,
                r#"sce:type="optional&lt;string[merge_text=96]&gt;""#
            )
        );
    }

    #[test]
    fn a_schema_list_bound_is_its_max_count() {
        let text = schema(
            r#"    <data id="tags" sce:type="list&lt;string[8]&gt;" sce:max-count="4" sce:direction="in"/>"#,
        );
        assert_eq!(
            rewritten_document(&text, "probe.scxml").unwrap(),
            text.replace(
                r#"sce:type="list&lt;string[8]&gt;" sce:max-count="4""#,
                r#"sce:type="list&lt;string[8]&gt;[4]""#
            )
        );
    }

    #[test]
    fn a_static_variable_capacity_moves_into_its_type() {
        let text = statechart(
            "sce-static",
            r#"    <data id="items" sce:type="list&lt;int64&gt;" sce:capacity="16"/>
    <data id="name" sce:type="string" sce:capacity="8" sce:limit="name_len"/>
    <data id="count" sce:type="int64" expr="0"/>"#,
        );
        let records = superseded(&text, "m.scxml");
        let types: Vec<String> = records.iter().map(|r| fix_of(r).2).collect();
        assert_eq!(types, ["list<int64>[16]", "string[name_len=8]"]);
        let rewritten = rewritten_document(&text, "m.scxml").unwrap();
        assert!(rewritten.contains(r#"<data id="items" sce:type="list&lt;int64&gt;[16]"/>"#));
        assert!(rewritten.contains(r#"<data id="name" sce:type="string[name_len=8]"/>"#));
        assert!(rewritten.contains(r#"<data id="count" sce:type="int64" expr="0"/>"#));
    }

    #[test]
    fn a_variable_of_another_data_model_is_no_value_type_of_the_language() {
        let text = statechart(
            "ecmascript",
            r#"    <data id="items" sce:type="list&lt;int64&gt;" sce:capacity="16"/>"#,
        );
        assert!(superseded(&text, "m.scxml").is_empty());
        assert_eq!(rewritten_document(&text, "m.scxml").unwrap(), text);
    }

    #[test]
    fn the_queue_depth_and_a_kind_declared_in_place_are_not_bounds_of_a_variable() {
        let text = statechart(
            "sce-static",
            r#"    <data id="conv" sce:kind="codec">
      <datamodel>
        <data id="raw" sce:type="bytes" sce:max-size="8" sce:direction="in"/>
      </datamodel>
    </data>"#,
        )
        .replacen("name=\"m\"", "name=\"m\" sce:capacity=\"4\"", 1);
        assert!(superseded(&text, "m.scxml").is_empty());
        assert_eq!(rewritten_document(&text, "m.scxml").unwrap(), text);
    }

    #[test]
    fn a_codec_field_bound_describes_a_frame_and_stays() {
        let text = format!(
            "<scxml {NS} sce:kind=\"codec\" sce:encoding=\"cbor\" sce:codec-format=\"2\" name=\"c\" version=\"1.0\">\n  <datamodel>\n    <data id=\"label\" sce:type=\"string\" sce:key=\"0\" sce:max-size=\"4\" sce:direction=\"out\"/>\n  </datamodel>\n</scxml>\n"
        );
        assert!(superseded(&text, "c.scxml").is_empty());
        assert_eq!(rewritten_document(&text, "c.scxml").unwrap(), text);
    }

    #[test]
    fn an_algorithm_local_and_return_bound_move_into_their_types() {
        let text = algorithm(
            r#"    <sce:param name="frame" type="bytes"/>
    <sce:return type="bytes" returns-max-size="64" sce:limit="frame_max"/>"#,
            r#"    <sce:var name="out" type="bytes" capacity="64"/>
    <sce:return expr="out"/>"#,
        );
        let records = superseded(&text, "alg.scxml");
        let found: Vec<(String, Vec<String>, String)> = records.iter().map(fix_of).collect();
        assert_eq!(
            found,
            [
                (
                    "<sce:return>".to_string(),
                    vec!["returns-max-size".to_string(), "sce:limit".to_string()],
                    "bytes[frame_max=64]".to_string()
                ),
                (
                    "<sce:var name=\"out\">".to_string(),
                    vec!["capacity".to_string()],
                    "bytes[64]".to_string()
                ),
            ]
        );
        let rewritten = rewritten_document(&text, "alg.scxml").unwrap();
        assert!(
            rewritten.contains(r#"<sce:return type="bytes[frame_max=64]"/>"#),
            "{rewritten}"
        );
        assert!(
            rewritten.contains(r#"<sce:var name="out" type="bytes[64]"/>"#),
            "{rewritten}"
        );
        // The parameter is a view of its argument and writes no bound.
        assert!(rewritten.contains(r#"<sce:param name="frame" type="bytes"/>"#));
        assert!(rewritten.contains(r#"<sce:return expr="out"/>"#));
    }

    #[test]
    fn a_bound_written_both_ways_is_refused_and_not_rewritten() {
        let text = schema(
            r#"    <data id="tag" sce:type="string[32]" sce:max-size="32" sce:direction="in"/>"#,
        );
        assert!(superseded(&text, "probe.scxml").is_empty());
        let refusal = rewritten_document(&text, "probe.scxml").unwrap_err();
        assert!(
            matches!(
                &refusal.error,
                ForgeError::Validation(v)
                    if matches!(&**v, ValidationError::IncompatibleAttributes { .. })
            ),
            "{refusal:?}"
        );
    }

    #[test]
    fn a_rewritten_document_has_nothing_left_to_rewrite() {
        let text = schema(
            r#"    <data id="a" sce:type="string" sce:max-size="8" sce:direction="in"/>
    <!-- a comment the rewrite keeps -->
    <data   id="b"
            sce:type="bytes"
            sce:max-size="0x10"
            sce:direction="in"/>
    <data id="c" sce:type="uint8" sce:direction="in"/>"#,
        );
        let rewritten = rewritten_document(&text, "probe.scxml").unwrap();
        assert!(rewritten.contains("<!-- a comment the rewrite keeps -->"));
        assert!(rewritten.contains("<data   id=\"b\"\n            sce:type=\"bytes[16]\"\n            sce:direction=\"in\"/>"), "{rewritten}");
        assert!(superseded(&rewritten, "probe.scxml").is_empty());
        assert_eq!(
            rewritten_document(&rewritten, "probe.scxml").unwrap(),
            rewritten
        );
    }

    #[test]
    fn a_bound_this_reading_cannot_rewrite_is_left_to_the_parser() {
        // A number that is no bound, and an attribute beside a type with no length.
        let text = schema(
            r#"    <data id="a" sce:type="string" sce:max-size="0" sce:direction="in"/>
    <data id="b" sce:type="uint32" sce:max-size="8" sce:direction="in"/>
    <data id="c" sce:type="strng" sce:max-size="8" sce:direction="in"/>"#,
        );
        assert!(superseded(&text, "probe.scxml").is_empty());
        assert_eq!(rewritten_document(&text, "probe.scxml").unwrap(), text);
    }

    #[test]
    fn the_record_stands_at_the_bound_attribute() {
        let text = schema(
            "    <data id=\"tag\"\n          sce:type=\"string\"\n          sce:max-size=\"32\"\n          sce:direction=\"in\"/>",
        );
        let records = superseded(&text, "probe.scxml");
        let row = text[..text.find("sce:max-size").unwrap()]
            .matches('\n')
            .count() as u32
            + 1;
        assert_eq!(records[0].location.line, Some(row));
    }
}
