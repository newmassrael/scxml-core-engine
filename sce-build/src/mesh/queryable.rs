// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `machines.<m>.queryables` — the queries a machine serves (SCE
//! Protocol-Synthesis RFC §synth-5-R, Queryables a machine serves; ADR 0041).
//!
//! A machine that answers queries states, in its deploy, the key of each one
//! and the shape of the reply. Everything a node knows before it runs is then
//! in one place and is generated from it: the matching from a query key to the
//! queryables that answer it, the writer of each reply into a buffer the
//! caller owns, and the size of the largest reply, which a host sizes its
//! response buffers from.
//!
//! What stays out of the declaration is what the node learns at run time: the
//! node's own key prefix (a zenoh id the host supplies) and the values the
//! documents carry. The declared key is the part after that prefix.
//!
//! ```yaml
//! queryables:
//!   - name: status_connect
//!     key: status/connect
//!     reply:
//!       encoding: application/json
//!       document:
//!         - { name: endpoints, type: array, max_items: 8, items: { type: string, max_len: 64 } }
//!         - { name: last_write, type: u64, nullable: true }
//!         - name: permissions
//!           type: object
//!           fields:
//!             - { name: write, type: bool }
//!   - name: ping
//!     key: ping
//!     reply: { encoding: text/plain, body: pong }
//! ```
//!
//! [`resolve_queryables`] is the one reader of that block. It refuses what no
//! generated code could honour, so a deploy that passes it has a closed set of
//! legs, each with a bounded reply.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::mesh::error::{
    DeployError, QueryableDocumentFault, QueryableKeyFault, QueryableNameFault, QueryableReplyFault,
};
use crate::scxml_identifier::is_code_identifier;

/// The most chunks a declared key may have; the matcher's state is one bit
/// per position (`sce_link_runtime::keyexpr::MAX_DECLARED_CHUNKS`).
pub const MAX_KEY_CHUNKS: usize = 32;

/// The deepest nesting of objects and arrays in one document.
pub const MAX_DOCUMENT_DEPTH: usize = 8;

/// The only encoding a `document:` reply is written in.
pub const JSON_ENCODING: &str = "application/json";

/// One entry of `machines.<m>.queryables`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryableConfig {
    /// The queryable's name in the generated code: a variant of `Leg`, the
    /// prefix of its document types and constants. A code identifier, unique
    /// in the machine.
    pub name: String,
    /// The key the machine answers, without the node's own prefix. Chunks
    /// separated by `/`; a chunk is a literal, `*` (exactly one chunk) or `**`
    /// (any number, including none). The empty string is the node prefix
    /// itself.
    pub key: String,
    pub reply: QueryableReplyConfig,
}

/// The reply of one queryable.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryableReplyConfig {
    /// The MIME type of the body, written as the reply's encoding. A
    /// `document:` is JSON, so it requires `application/json`.
    pub encoding: String,
    /// A body that never changes, written as is. Exactly one of `body` and
    /// `document` is stated.
    #[serde(default)]
    pub body: Option<String>,
    /// A JSON object whose fields, in this order, are the reply. The values
    /// are the host's, at run time; the fields, their types and their bounds
    /// are the deploy's.
    #[serde(default)]
    pub document: Option<Vec<DocNodeConfig>>,
}

/// One field of a document, or the element of an array.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocNodeConfig {
    /// The JSON member name. Required for a field of an object, and not
    /// allowed on the `items:` of an array, which has none.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub ty: DocType,
    /// The value may be absent at run time and is then written as `null`.
    #[serde(default)]
    pub nullable: bool,
    /// Bytes of the value, before escaping. Required for `string`.
    #[serde(default)]
    pub max_len: Option<u32>,
    /// Elements. Required for `array`.
    #[serde(default)]
    pub max_items: Option<u32>,
    /// The element of an `array`.
    #[serde(default)]
    pub items: Option<Box<DocNodeConfig>>,
    /// The fields of an `object`, in order.
    #[serde(default)]
    pub fields: Option<Vec<DocNodeConfig>>,
}

/// The type of a document value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    String,
    Array,
    Object,
}

impl DocType {
    fn as_str(self) -> &'static str {
        match self {
            DocType::Bool => "bool",
            DocType::U8 => "u8",
            DocType::U16 => "u16",
            DocType::U32 => "u32",
            DocType::U64 => "u64",
            DocType::I8 => "i8",
            DocType::I16 => "i16",
            DocType::I32 => "i32",
            DocType::I64 => "i64",
            DocType::String => "string",
            DocType::Array => "array",
            DocType::Object => "object",
        }
    }
}

/// A queryable with everything the generated code needs resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryableLeg {
    /// The deploy's name, a code identifier.
    pub name: String,
    /// `name` in PascalCase: the `Leg` variant and the type-name prefix.
    pub pascal: String,
    pub key: String,
    /// The key has no `*`, so an exact query is one comparison.
    pub key_is_literal: bool,
    pub encoding: String,
    pub reply: LegReply,
    /// The most bytes the reply body can occupy, the figure a host sizes its
    /// response buffer from. For a document it is a bound the writer
    /// guarantees (a value outside the declared bounds is refused, never
    /// written); for a static body it is the body's length.
    pub max_body_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegReply {
    Static { body: String },
    Document { root: DocObject },
}

/// An object: its fields in the order they are written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocObject {
    pub fields: Vec<DocField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocField {
    pub name: String,
    pub node: DocNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocNode {
    pub nullable: bool,
    pub kind: DocKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocKind {
    Bool,
    /// An unsigned integer of this many bits.
    Unsigned(u8),
    /// A signed integer of this many bits.
    Signed(u8),
    String {
        max_len: u32,
    },
    Array {
        max_items: u32,
        item: Box<DocNode>,
    },
    Object(DocObject),
}

/// Read a machine's `queryables:` into legs, or refuse it.
pub fn resolve_queryables(
    machine: &str,
    queryables: &[QueryableConfig],
) -> Result<Vec<QueryableLeg>, DeployError> {
    let mut legs: Vec<QueryableLeg> = Vec::with_capacity(queryables.len());
    let mut type_names: BTreeSet<String> = BTreeSet::new();
    for config in queryables {
        let name_fault = |fault| DeployError::QueryableNameInvalid {
            machine: machine.to_string(),
            name: config.name.clone(),
            fault,
        };
        if !is_code_identifier(&config.name) {
            return Err(name_fault(QueryableNameFault::NotIdentifier));
        }
        if crate::filters::RUST_KEYWORDS.contains(&config.name.as_str()) {
            return Err(name_fault(QueryableNameFault::Reserved));
        }
        let pascal = pascal_case(&config.name);
        if legs.iter().any(|l| l.pascal == pascal) {
            return Err(name_fault(QueryableNameFault::Duplicate));
        }
        check_key(&config.key).map_err(|fault| DeployError::QueryableKeyInvalid {
            machine: machine.to_string(),
            name: config.name.clone(),
            key: config.key.clone(),
            fault,
        })?;
        if let Some(first) = legs.iter().find(|l| l.key == config.key) {
            return Err(DeployError::QueryableKeyDuplicate {
                machine: machine.to_string(),
                key: config.key.clone(),
                first: first.name.clone(),
                second: config.name.clone(),
            });
        }
        let reply_fault = |fault| DeployError::QueryableReplyInvalid {
            machine: machine.to_string(),
            name: config.name.clone(),
            fault,
        };
        check_encoding(&config.reply.encoding).map_err(reply_fault)?;
        let (reply, max_body_bytes) = match (&config.reply.body, &config.reply.document) {
            (Some(_), Some(_)) => return Err(reply_fault(QueryableReplyFault::BodyAndDocument)),
            (None, None) => return Err(reply_fault(QueryableReplyFault::NeitherBodyNorDocument)),
            (Some(body), None) => {
                if config.reply.encoding == JSON_ENCODING
                    && serde_json::from_str::<serde_json::Value>(body).is_err()
                {
                    return Err(reply_fault(QueryableReplyFault::BodyNotJson));
                }
                let len = u32::try_from(body.len())
                    .map_err(|_| reply_fault(QueryableReplyFault::BodyTooLarge))?;
                (LegReply::Static { body: body.clone() }, len)
            }
            (None, Some(fields)) => {
                if config.reply.encoding != JSON_ENCODING {
                    return Err(reply_fault(QueryableReplyFault::DocumentNeedsJson));
                }
                let document_fault = |path: &str, fault: QueryableDocumentFault| {
                    DeployError::QueryableDocumentInvalid {
                        machine: machine.to_string(),
                        name: config.name.clone(),
                        path: path.to_string(),
                        fault,
                    }
                };
                let root = resolve_object(fields, "", 1, &document_fault)?;
                collect_type_names(&pascal, &root, &mut type_names).map_err(|clash| {
                    document_fault(&clash, QueryableDocumentFault::TypeNameCollision)
                })?;
                let max = object_max_bytes(&root)
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| document_fault("", QueryableDocumentFault::TooLarge))?;
                (LegReply::Document { root }, max)
            }
        };
        legs.push(QueryableLeg {
            name: config.name.clone(),
            pascal,
            key_is_literal: !config.key.contains('*'),
            key: config.key.clone(),
            encoding: config.reply.encoding.clone(),
            reply,
            max_body_bytes,
        });
    }
    Ok(legs)
}

/// The grammar of a declared key (see [`QueryableConfig::key`]).
fn check_key(key: &str) -> Result<(), QueryableKeyFault> {
    if key.is_empty() {
        return Ok(());
    }
    if let Some(c) = key.chars().find(|c| matches!(c, '?' | '#')) {
        return Err(QueryableKeyFault::ForbiddenCharacter(c));
    }
    if let Some(c) = key.chars().find(|c| c.is_control() || c.is_whitespace()) {
        return Err(QueryableKeyFault::ForbiddenCharacter(c));
    }
    if key.contains('$') {
        return Err(QueryableKeyFault::SubChunkWildcard);
    }
    let chunks: Vec<&str> = key.split('/').collect();
    if chunks.len() > MAX_KEY_CHUNKS {
        return Err(QueryableKeyFault::TooManyChunks {
            max: MAX_KEY_CHUNKS,
        });
    }
    let mut previous: Option<&str> = None;
    for chunk in chunks {
        if chunk.is_empty() {
            return Err(QueryableKeyFault::EmptyChunk);
        }
        if chunk.contains('*') && chunk != "*" && chunk != "**" {
            return Err(QueryableKeyFault::WildcardInsideChunk);
        }
        // `**/**` is `**`, and `**/*` is `*/**`: the canonical forms say one
        // thing one way, so a key is not written in the other.
        if previous == Some("**") && (chunk == "**" || chunk == "*") {
            return Err(QueryableKeyFault::NotCanonical);
        }
        previous = Some(chunk);
    }
    Ok(())
}

/// A MIME type with no parameters: `type/subtype`, both tokens.
fn check_encoding(encoding: &str) -> Result<(), QueryableReplyFault> {
    let token = |s: &str| {
        !s.is_empty()
            && s.bytes().all(|b| {
                b.is_ascii_alphanumeric()
                    || matches!(
                        b,
                        b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                    )
            })
    };
    match encoding.split_once('/') {
        Some((kind, subtype)) if token(kind) && token(subtype) => Ok(()),
        _ => Err(QueryableReplyFault::EncodingInvalid),
    }
}

type DocFaultBuilder<'a> = dyn Fn(&str, QueryableDocumentFault) -> DeployError + 'a;

fn join_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}.{name}")
    }
}

fn resolve_object(
    fields: &[DocNodeConfig],
    path: &str,
    depth: usize,
    fail: &DocFaultBuilder<'_>,
) -> Result<DocObject, DeployError> {
    if depth > MAX_DOCUMENT_DEPTH {
        return Err(fail(
            path,
            QueryableDocumentFault::TooDeep {
                max: MAX_DOCUMENT_DEPTH,
            },
        ));
    }
    if fields.is_empty() {
        return Err(fail(path, QueryableDocumentFault::ObjectEmpty));
    }
    let mut out: Vec<DocField> = Vec::with_capacity(fields.len());
    for field in fields {
        let Some(name) = field.name.as_deref() else {
            return Err(fail(path, QueryableDocumentFault::NameMissing));
        };
        let field_path = join_path(path, name);
        if !is_code_identifier(name) {
            return Err(fail(&field_path, QueryableDocumentFault::NameNotIdentifier));
        }
        if crate::filters::RUST_KEYWORDS.contains(&name) {
            return Err(fail(&field_path, QueryableDocumentFault::NameReserved));
        }
        if out.iter().any(|f| f.name == name) {
            return Err(fail(&field_path, QueryableDocumentFault::NameDuplicate));
        }
        let node = resolve_node(field, &field_path, depth, fail)?;
        out.push(DocField {
            name: name.to_string(),
            node,
        });
    }
    Ok(DocObject { fields: out })
}

fn resolve_node(
    config: &DocNodeConfig,
    path: &str,
    depth: usize,
    fail: &DocFaultBuilder<'_>,
) -> Result<DocNode, DeployError> {
    let not_applicable = |attr: &'static str| {
        fail(
            path,
            QueryableDocumentFault::AttributeNotApplicable {
                attr,
                ty: config.ty.as_str(),
            },
        )
    };
    let bound = |attr: &'static str, value: Option<u32>| -> Result<u32, DeployError> {
        match value {
            None => Err(fail(path, QueryableDocumentFault::BoundMissing { attr })),
            Some(0) => Err(fail(path, QueryableDocumentFault::BoundZero { attr })),
            Some(n) => Ok(n),
        }
    };
    if config.ty != DocType::String && config.max_len.is_some() {
        return Err(not_applicable("max_len"));
    }
    if config.ty != DocType::Array && config.max_items.is_some() {
        return Err(not_applicable("max_items"));
    }
    if config.ty != DocType::Array && config.items.is_some() {
        return Err(not_applicable("items"));
    }
    if config.ty != DocType::Object && config.fields.is_some() {
        return Err(not_applicable("fields"));
    }
    let kind = match config.ty {
        DocType::Bool => DocKind::Bool,
        DocType::U8 => DocKind::Unsigned(8),
        DocType::U16 => DocKind::Unsigned(16),
        DocType::U32 => DocKind::Unsigned(32),
        DocType::U64 => DocKind::Unsigned(64),
        DocType::I8 => DocKind::Signed(8),
        DocType::I16 => DocKind::Signed(16),
        DocType::I32 => DocKind::Signed(32),
        DocType::I64 => DocKind::Signed(64),
        DocType::String => DocKind::String {
            max_len: bound("max_len", config.max_len)?,
        },
        DocType::Array => {
            let max_items = bound("max_items", config.max_items)?;
            let Some(items) = config.items.as_deref() else {
                return Err(fail(path, QueryableDocumentFault::ArrayItemsMissing));
            };
            if items.name.is_some() {
                return Err(fail(path, QueryableDocumentFault::NameNotAllowed));
            }
            if depth + 1 > MAX_DOCUMENT_DEPTH {
                return Err(fail(
                    path,
                    QueryableDocumentFault::TooDeep {
                        max: MAX_DOCUMENT_DEPTH,
                    },
                ));
            }
            let item_path = format!("{path}[]");
            DocKind::Array {
                max_items,
                item: Box::new(resolve_node(items, &item_path, depth + 1, fail)?),
            }
        }
        DocType::Object => {
            let Some(fields) = config.fields.as_deref() else {
                return Err(fail(path, QueryableDocumentFault::ObjectEmpty));
            };
            DocKind::Object(resolve_object(fields, path, depth + 1, fail)?)
        }
    };
    Ok(DocNode {
        nullable: config.nullable,
        kind,
    })
}

/// Digits of the longest decimal spelling of an integer of `bits` bits.
fn integer_max_bytes(bits: u8, signed: bool) -> u64 {
    match (bits, signed) {
        (8, false) => 3,
        (16, false) => 5,
        (32, false) => 10,
        (64, false) => 20,
        (8, true) => 4,
        (16, true) => 6,
        (32, true) => 11,
        _ => 20,
    }
}

fn node_max_bytes(node: &DocNode) -> Option<u64> {
    let value = match &node.kind {
        DocKind::Bool => 5,
        DocKind::Unsigned(bits) => integer_max_bytes(*bits, false),
        DocKind::Signed(bits) => integer_max_bytes(*bits, true),
        // Two quotation marks, and six bytes for every byte at worst: a
        // control character is written as `\u00XX`.
        DocKind::String { max_len } => 2 + 6 * u64::from(*max_len),
        DocKind::Array { max_items, item } => {
            let n = u64::from(*max_items);
            // Brackets, the elements, and a comma between neighbours.
            2u64.checked_add(n.checked_mul(node_max_bytes(item)?)?)?
                .checked_add(n - 1)?
        }
        DocKind::Object(object) => object_max_bytes(object)?,
    };
    // `null` is four bytes and a value is at least one, so a nullable node is
    // bounded by the larger.
    Some(if node.nullable { value.max(4) } else { value })
}

fn object_max_bytes(object: &DocObject) -> Option<u64> {
    // Braces, then per field `"name":value`, and a comma between neighbours.
    let mut total: u64 = 2;
    for (i, field) in object.fields.iter().enumerate() {
        let name = u64::try_from(field.name.len()).ok()?;
        total = total
            .checked_add(name + 3)?
            .checked_add(node_max_bytes(&field.node)?)?;
        if i > 0 {
            total = total.checked_add(1)?;
        }
    }
    Some(total)
}

/// Record the Rust type name of every object in a document, and report the
/// first one that is already taken.
fn collect_type_names(
    pascal: &str,
    root: &DocObject,
    taken: &mut BTreeSet<String>,
) -> Result<(), String> {
    fn object(prefix: &str, o: &DocObject, taken: &mut BTreeSet<String>) -> Result<(), String> {
        if !taken.insert(prefix.to_string()) {
            return Err(prefix.to_string());
        }
        for field in &o.fields {
            node(
                &format!("{prefix}{}", pascal_case(&field.name)),
                &field.node,
                taken,
            )?;
        }
        Ok(())
    }
    fn node(prefix: &str, n: &DocNode, taken: &mut BTreeSet<String>) -> Result<(), String> {
        match &n.kind {
            DocKind::Object(o) => object(prefix, o, taken),
            DocKind::Array { item, .. } => node(&format!("{prefix}Item"), item, taken),
            _ => Ok(()),
        }
    }
    object(&format!("{pascal}Doc"), root, taken)
}

/// `status_connect` as `StatusConnect`.
pub fn pascal_case(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Vec<QueryableConfig> {
        serde_yaml_ng::from_str(yaml).expect("queryables parse")
    }

    fn resolve(yaml: &str) -> Result<Vec<QueryableLeg>, DeployError> {
        resolve_queryables("admin", &parse(yaml))
    }

    #[test]
    fn a_document_is_bounded_by_what_its_fields_can_hold() {
        let legs = resolve(
            r#"
- name: status_connect
  key: status/connect
  reply:
    encoding: application/json
    document:
      - { name: endpoints, type: array, max_items: 2, items: { type: string, max_len: 4 } }
      - { name: last_write, type: u64, nullable: true }
      - name: permissions
        type: object
        fields:
          - { name: write, type: bool }
"#,
        )
        .expect("resolves");
        // {"endpoints":["....","...."],"last_write":u64,"permissions":{"write":false}}
        //  braces 2
        //  "endpoints": 12 + array (2 + 2*(2+24) + 1 = 55)
        //  "last_write": 13 + 20
        //  "permissions": 14 + object (2 + "write": 8 + 5 = 15)
        //  commas 2
        assert_eq!(legs[0].max_body_bytes, 2 + 12 + 55 + 13 + 20 + 14 + 15 + 2);
        assert!(legs[0].key_is_literal);
        assert_eq!(legs[0].pascal, "StatusConnect");
    }

    #[test]
    fn a_static_body_is_its_own_length() {
        let legs = resolve(
            r#"
- name: ping
  key: ping
  reply: { encoding: text/plain, body: pong }
"#,
        )
        .expect("resolves");
        assert_eq!(legs[0].max_body_bytes, 4);
    }

    #[test]
    fn integer_widths_have_their_longest_spelling() {
        assert_eq!(integer_max_bytes(8, false), "255".len() as u64);
        assert_eq!(integer_max_bytes(16, false), "65535".len() as u64);
        assert_eq!(integer_max_bytes(32, false), "4294967295".len() as u64);
        assert_eq!(
            integer_max_bytes(64, false),
            u64::MAX.to_string().len() as u64
        );
        assert_eq!(integer_max_bytes(8, true), "-128".len() as u64);
        assert_eq!(integer_max_bytes(16, true), "-32768".len() as u64);
        assert_eq!(integer_max_bytes(32, true), "-2147483648".len() as u64);
        assert_eq!(
            integer_max_bytes(64, true),
            i64::MIN.to_string().len() as u64
        );
    }

    fn key_fault(key: &str) -> QueryableKeyFault {
        match check_key(key) {
            Err(fault) => fault,
            Ok(()) => panic!("`{key}` was accepted"),
        }
    }

    #[test]
    fn a_key_is_literal_chunks_and_whole_chunk_wildcards() {
        for ok in [
            "",
            "config",
            "status/connect",
            "peer/*",
            "peer/**",
            "**",
            "*",
            "a/**/z",
            "*/**",
        ] {
            assert_eq!(check_key(ok), Ok(()), "{ok}");
        }
        assert_eq!(key_fault("a//b"), QueryableKeyFault::EmptyChunk);
        assert_eq!(key_fault("/a"), QueryableKeyFault::EmptyChunk);
        assert_eq!(key_fault("a/"), QueryableKeyFault::EmptyChunk);
        assert_eq!(key_fault("a$*"), QueryableKeyFault::SubChunkWildcard);
        assert_eq!(key_fault("a*"), QueryableKeyFault::WildcardInsideChunk);
        assert_eq!(key_fault("***"), QueryableKeyFault::WildcardInsideChunk);
        assert_eq!(key_fault("**/**"), QueryableKeyFault::NotCanonical);
        assert_eq!(key_fault("**/*"), QueryableKeyFault::NotCanonical);
        assert_eq!(key_fault("a?b"), QueryableKeyFault::ForbiddenCharacter('?'));
        assert_eq!(key_fault("a#b"), QueryableKeyFault::ForbiddenCharacter('#'));
        assert_eq!(key_fault("a b"), QueryableKeyFault::ForbiddenCharacter(' '));
        let long = vec!["a"; MAX_KEY_CHUNKS + 1].join("/");
        assert_eq!(
            key_fault(&long),
            QueryableKeyFault::TooManyChunks {
                max: MAX_KEY_CHUNKS
            }
        );
        assert_eq!(check_key(&vec!["a"; MAX_KEY_CHUNKS].join("/")), Ok(()));
    }

    #[test]
    fn the_same_key_twice_is_refused_and_overlapping_keys_are_not() {
        let err = resolve(
            r#"
- { name: a, key: config, reply: { encoding: text/plain, body: x } }
- { name: b, key: config, reply: { encoding: text/plain, body: y } }
"#,
        )
        .expect_err("refused");
        assert!(
            matches!(err, DeployError::QueryableKeyDuplicate { .. }),
            "{err:?}"
        );
        resolve(
            r#"
- { name: a, key: "peer/**", reply: { encoding: text/plain, body: x } }
- { name: b, key: "peer/x", reply: { encoding: text/plain, body: y } }
"#,
        )
        .expect("overlap is allowed: both answer a query that reaches both");
    }

    #[test]
    fn a_name_is_a_code_identifier_and_unique_after_casing() {
        let fault = |yaml: &str| match resolve(yaml).expect_err("refused") {
            DeployError::QueryableNameInvalid { fault, .. } => fault,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            fault("- { name: \"a-b\", key: x, reply: { encoding: text/plain, body: x } }"),
            QueryableNameFault::NotIdentifier
        );
        assert_eq!(
            fault("- { name: match, key: x, reply: { encoding: text/plain, body: x } }"),
            QueryableNameFault::Reserved
        );
        assert_eq!(
            fault(
                "- { name: a_b, key: x, reply: { encoding: text/plain, body: x } }\n\
                 - { name: a__b, key: y, reply: { encoding: text/plain, body: x } }"
            ),
            QueryableNameFault::Duplicate
        );
    }

    #[test]
    fn a_reply_states_exactly_one_of_body_and_document() {
        let fault = |yaml: &str| match resolve(yaml).expect_err("refused") {
            DeployError::QueryableReplyInvalid { fault, .. } => fault,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            fault("- { name: a, key: x, reply: { encoding: text/plain } }"),
            QueryableReplyFault::NeitherBodyNorDocument
        );
        assert_eq!(
            fault(
                "- { name: a, key: x, reply: { encoding: application/json, body: '{}', \
                 document: [ { name: f, type: bool } ] } }"
            ),
            QueryableReplyFault::BodyAndDocument
        );
        assert_eq!(
            fault("- { name: a, key: x, reply: { encoding: text/plain, document: [ { name: f, type: bool } ] } }"),
            QueryableReplyFault::DocumentNeedsJson
        );
        assert_eq!(
            fault("- { name: a, key: x, reply: { encoding: application/json, body: '{' } }"),
            QueryableReplyFault::BodyNotJson
        );
        assert_eq!(
            fault("- { name: a, key: x, reply: { encoding: json, body: x } }"),
            QueryableReplyFault::EncodingInvalid
        );
        assert_eq!(
            fault(
                "- { name: a, key: x, reply: { encoding: 'text/plain; charset=utf-8', body: x } }"
            ),
            QueryableReplyFault::EncodingInvalid
        );
    }

    fn document_fault(document: &str) -> (String, QueryableDocumentFault) {
        let yaml = format!(
            "- {{ name: a, key: x, reply: {{ encoding: application/json, document: {document} }} }}"
        );
        match resolve(&yaml).expect_err("refused") {
            DeployError::QueryableDocumentInvalid { path, fault, .. } => (path, fault),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_document_field_names_its_type_and_its_bounds() {
        assert_eq!(
            document_fault("[ { name: s, type: string } ]"),
            (
                "s".to_string(),
                QueryableDocumentFault::BoundMissing { attr: "max_len" }
            )
        );
        assert_eq!(
            document_fault("[ { name: s, type: string, max_len: 0 } ]"),
            (
                "s".to_string(),
                QueryableDocumentFault::BoundZero { attr: "max_len" }
            )
        );
        assert_eq!(
            document_fault("[ { name: n, type: u8, max_len: 4 } ]"),
            (
                "n".to_string(),
                QueryableDocumentFault::AttributeNotApplicable {
                    attr: "max_len",
                    ty: "u8"
                }
            )
        );
        assert_eq!(
            document_fault("[ { name: l, type: array, max_items: 2 } ]"),
            ("l".to_string(), QueryableDocumentFault::ArrayItemsMissing)
        );
        assert_eq!(
            document_fault(
                "[ { name: l, type: array, max_items: 2, items: { name: x, type: bool } } ]"
            ),
            ("l".to_string(), QueryableDocumentFault::NameNotAllowed)
        );
        assert_eq!(
            document_fault("[ { name: o, type: object, fields: [] } ]"),
            ("o".to_string(), QueryableDocumentFault::ObjectEmpty)
        );
        assert_eq!(
            document_fault("[ { type: bool } ]"),
            (String::new(), QueryableDocumentFault::NameMissing)
        );
        assert_eq!(
            document_fault("[ { name: a, type: bool }, { name: a, type: u8 } ]"),
            ("a".to_string(), QueryableDocumentFault::NameDuplicate)
        );
        assert_eq!(
            document_fault("[ { name: loop, type: bool } ]"),
            ("loop".to_string(), QueryableDocumentFault::NameReserved)
        );
        assert_eq!(
            document_fault("[ { name: \"a-b\", type: bool } ]"),
            ("a-b".to_string(), QueryableDocumentFault::NameNotIdentifier)
        );
    }

    #[test]
    fn a_document_that_nests_past_the_limit_is_refused() {
        let mut document = String::from("[ { name: leaf, type: bool } ]");
        for _ in 0..MAX_DOCUMENT_DEPTH {
            document = format!("[ {{ name: o, type: object, fields: {document} }} ]");
        }
        let (_, fault) = document_fault(&document);
        assert_eq!(
            fault,
            QueryableDocumentFault::TooDeep {
                max: MAX_DOCUMENT_DEPTH
            }
        );
    }

    #[test]
    fn two_objects_cannot_share_a_generated_type_name() {
        // `a` holding `b` and a top-level `a_b` both want `...DocAB`.
        let (_, fault) = document_fault(
            "[ { name: a, type: object, fields: [ { name: b, type: object, fields: \
             [ { name: x, type: bool } ] } ] }, { name: a_b, type: object, fields: \
             [ { name: x, type: bool } ] } ]",
        );
        assert_eq!(fault, QueryableDocumentFault::TypeNameCollision);
    }

    #[test]
    fn a_nullable_value_is_bounded_by_the_larger_of_it_and_null() {
        let legs = resolve(
            r#"
- name: a
  key: x
  reply:
    encoding: application/json
    document:
      - { name: f, type: bool, nullable: true }
      - { name: n, type: u8, nullable: true }
"#,
        )
        .expect("resolves");
        // {"f":false,"n":255}
        assert_eq!(legs[0].max_body_bytes, 2 + (4 + 5) + (4 + 4) + 1);
    }

    #[test]
    fn a_document_larger_than_four_gibibytes_is_refused() {
        let (_, fault) = document_fault("[ { name: s, type: string, max_len: 4294967295 } ]");
        assert_eq!(fault, QueryableDocumentFault::TooLarge);
    }
}
