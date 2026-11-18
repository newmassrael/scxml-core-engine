// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a byte case of a CBOR map or of a content-line component states
//! (docs/adr/0081 §5), read once from its `source` against the codec's plan, so
//! the Rust, C++ and Python case printers spell one reading of one value.
//!
//! A CBOR map and a content-line component derive nothing a value says: every
//! entry is presented (`CborPlan`, `ContentLinePlan`), so a case states each
//! one, keyed by its id, as the plan's interface presents it. A boolean is
//! `true` or `false`, an integer a JSON integer within its type, a text a JSON
//! string within its bound in UTF-8 bytes, a byte string an array of numbers of
//! a length its entry admits, an entry that may be absent `null` where it is,
//! and the values of a repeated property an array in order, `[]` for none. The
//! notation is the one the catalog's `decoded` objects already write for these
//! codecs, so a case and a `decoded` object of one value are one object.
//!
//! The elements are read in the order the value presents them: a map's entries
//! in declaration order, and a component's properties in declaration order,
//! each followed by the parameters declared for it. A printer whose value is
//! made by a constructor takes them in that order.
//!
//! A shape no printer spells yet is refused by name, never left out: a value of
//! an enum, a list of records, a line that holds a list of values, and the lines
//! of a repeated property that are records.

use crate::forge::encode_plan::{
    CborPlan, CborValue, ContentLinePlan, LineShape, LineValue, ParamValue,
};
use crate::forge::model::SceType;

/// One value a case states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Scalar {
    /// A boolean.
    Bool(bool),
    /// An integer, within the type of its element.
    Int(i128),
    /// A text, within its bound.
    Text(String),
    /// A byte string, of a length its element admits.
    Bytes(Vec<u8>),
}

/// What a case states of one element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Stated {
    /// None of it: `null`, of an element a value may lack.
    Absent,
    /// One value.
    One(Scalar),
    /// The values of a repeated property, in order.
    List(Vec<Scalar>),
}

/// One element of a value as a case states it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StatedElement {
    /// The element's id, as its document names it.
    pub(super) id: String,
    /// Whether a value may lack it, so that a value holding it holds it as an
    /// optional does; false for a repeated property, whose absence is no value.
    pub(super) optional: bool,
    /// What the case states of it.
    pub(super) stated: Stated,
}

/// What one value of an element is, as the notation reads it.
#[derive(Clone, Copy)]
enum Kind<'t> {
    Bool,
    /// An integer of the type `ty`.
    Int(&'t SceType),
    /// A text of at most `most` UTF-8 bytes, where the document states a bound.
    Text {
        most: Option<u32>,
    },
    /// A byte string of `least` bytes or more, and at most `most`.
    Bytes {
        least: u32,
        most: Option<u32>,
    },
}

/// Whether `len` is past `bound`.
fn past(len: usize, bound: u32) -> bool {
    match u32::try_from(len) {
        Ok(len) => len > bound,
        Err(_) => true,
    }
}

/// Whether `len` is short of `least`.
fn short_of(len: usize, least: u32) -> bool {
    match u32::try_from(len) {
        Ok(len) => len < least,
        Err(_) => false,
    }
}

/// One value of `kind` that `json` states.
fn scalar(kind: Kind<'_>, json: &serde_json::Value) -> Result<Scalar, String> {
    match kind {
        Kind::Bool => json
            .as_bool()
            .map(Scalar::Bool)
            .ok_or_else(|| "a boolean is `true` or `false`".to_string()),
        Kind::Int(ty) => {
            let n = json
                .as_u64()
                .map(i128::from)
                .or_else(|| json.as_i64().map(i128::from))
                .ok_or_else(|| "an integer is a JSON integer".to_string())?;
            let (least, most) = ty
                .int_value_range()
                .ok_or_else(|| format!("{ty:?} is no integer type a case states"))?;
            if n < least || n > most {
                return Err(format!(
                    "{n} is outside the type of the element, {least}..={most}"
                ));
            }
            Ok(Scalar::Int(n))
        }
        Kind::Text { most } => {
            let text = json
                .as_str()
                .ok_or_else(|| "a text is a JSON string".to_string())?;
            if let Some(most) = most.filter(|most| past(text.len(), *most)) {
                return Err(format!(
                    "the text is {} bytes of UTF-8, past its bound of {most}",
                    text.len()
                ));
            }
            Ok(Scalar::Text(text.to_string()))
        }
        Kind::Bytes { least, most } => {
            let bytes = json
                .as_array()
                .ok_or_else(|| "bytes are an array of numbers".to_string())?
                .iter()
                .map(|b| {
                    b.as_u64()
                        .and_then(|b| u8::try_from(b).ok())
                        .ok_or_else(|| "a byte is a number in 0..=255".to_string())
                })
                .collect::<Result<Vec<u8>, String>>()?;
            if short_of(bytes.len(), least) || most.is_some_and(|most| past(bytes.len(), most)) {
                return Err(format!(
                    "{} bytes, outside the {least} or more and the {} at most the element admits",
                    bytes.len(),
                    most.map_or_else(|| "any number".to_string(), |most| most.to_string())
                ));
            }
            Ok(Scalar::Bytes(bytes))
        }
    }
}

/// The members of `source`, a JSON object whose every key is one of `ids`.
fn members<'s>(
    source: &'s serde_json::Value,
    ids: &[&str],
) -> Result<&'s serde_json::Map<String, serde_json::Value>, String> {
    let object = source
        .as_object()
        .ok_or_else(|| "a source is a JSON object".to_string())?;
    if let Some(extra) = object.keys().find(|key| !ids.contains(&key.as_str())) {
        return Err(format!(
            "the source states `{extra}`, which the value does not present"
        ));
    }
    Ok(object)
}

/// What `object` states of the element `id`, one value of `kind`, `null`
/// where the value may lack it.
fn one(
    object: &serde_json::Map<String, serde_json::Value>,
    id: &str,
    optional: bool,
    kind: Kind<'_>,
) -> Result<Stated, String> {
    let json = object
        .get(id)
        .ok_or_else(|| format!("the source states no `{id}`"))?;
    if json.is_null() {
        return if optional {
            Ok(Stated::Absent)
        } else {
            Err(format!(
                "`{id}` is always there, and the source states none"
            ))
        };
    }
    scalar(kind, json)
        .map(Stated::One)
        .map_err(|e| format!("`{id}`: {e}"))
}

/// What `object` states of the repeated element `id`: an array of values of
/// `kind`, in order.
fn list(
    object: &serde_json::Map<String, serde_json::Value>,
    id: &str,
    kind: Kind<'_>,
) -> Result<Stated, String> {
    let json = object
        .get(id)
        .ok_or_else(|| format!("the source states no `{id}`"))?;
    json.as_array()
        .ok_or_else(|| format!("`{id}` is repeated, an array of its values"))?
        .iter()
        .map(|item| scalar(kind, item))
        .collect::<Result<Vec<Scalar>, String>>()
        .map(Stated::List)
        .map_err(|e| format!("`{id}`: {e}"))
}

/// Each entry of the CBOR map `plan` that `source` states, in declaration
/// order: the order its value presents them and its constructor takes them.
pub(super) fn cbor_elements(
    plan: &CborPlan<'_>,
    source: &serde_json::Value,
) -> Result<Vec<StatedElement>, String> {
    let ids: Vec<&str> = plan.entries.iter().map(|i| i.entry.id.as_str()).collect();
    let object = members(source, &ids)?;
    plan.entries
        .iter()
        .map(|item| {
            let e = item.entry;
            let kind = match item.value {
                CborValue::Bool => Kind::Bool,
                CborValue::Uint { .. } => Kind::Int(
                    e.leaf_type()
                        .expect("an unsigned integer entry is a leaf entry"),
                ),
                CborValue::Text => Kind::Text {
                    most: e.size_bound(),
                },
                CborValue::Bytes { exact: true } => {
                    let length = e.length.expect("an exact byte string states its length");
                    Kind::Bytes {
                        least: length,
                        most: Some(length),
                    }
                }
                CborValue::Bytes { exact: false } => Kind::Bytes {
                    least: 0,
                    most: e.size_bound(),
                },
                CborValue::Enum { .. } => {
                    return Err(format!(
                        "`{}`: a case does not state a value of an enum entry yet",
                        e.id
                    ))
                }
                CborValue::RecordList { .. } => {
                    return Err(format!(
                        "`{}`: a case does not state a list of records yet",
                        e.id
                    ))
                }
                CborValue::Unlowered => {
                    return Err(format!(
                        "`{}`: a case does not state a value of `{}` yet",
                        e.id, e.sce_type
                    ))
                }
            };
            let optional = !item.required;
            Ok(StatedElement {
                id: e.id.clone(),
                optional,
                stated: one(object, &e.id, optional, kind)?,
            })
        })
        .collect()
}

/// Each element of the content-line component `plan` that `source` states:
/// each property in declaration order, followed by the parameters declared for
/// it, the order its value presents them and its constructor takes them.
pub(super) fn content_line_elements(
    plan: &ContentLinePlan<'_>,
    source: &serde_json::Value,
) -> Result<Vec<StatedElement>, String> {
    // The shapes no printer spells, refused before any member is read: the
    // parameters of a line that is a record are members of the record and not
    // of the value, so the members are asked of a component that has none.
    for p in &plan.properties {
        let e = p.entry;
        match p.shape {
            LineShape::Records { .. } => {
                return Err(format!(
                    "`{}`: a case does not state the lines of a property that are records yet",
                    e.id
                ))
            }
            LineShape::One { values: true, .. } => {
                return Err(format!(
                    "`{}`: a case does not state a line that holds a list of values yet",
                    e.id
                ))
            }
            LineShape::Repeated { .. } | LineShape::One { .. } => {}
        }
        if matches!(p.value, LineValue::Enum { .. })
            || p.params
                .iter()
                .any(|param| matches!(param.value, ParamValue::Enum { .. }))
        {
            return Err(format!(
                "`{}`: a case does not state a value of an enum entry yet",
                e.id
            ));
        }
    }
    let ids: Vec<&str> = plan
        .properties
        .iter()
        .flat_map(|p| {
            std::iter::once(p.entry.id.as_str())
                .chain(p.params.iter().map(|param| param.entry.id.as_str()))
        })
        .collect();
    let object = members(source, &ids)?;
    let mut out = Vec::new();
    for p in &plan.properties {
        let e = p.entry;
        let kind = match &p.value {
            LineValue::Text { max_size, .. } => Kind::Text { most: *max_size },
            LineValue::Bool => Kind::Bool,
            LineValue::Uint { .. } | LineValue::Int { .. } => Kind::Int(e.present_type()),
            LineValue::Values { .. } | LineValue::Enum { .. } => {
                unreachable!("the shapes no printer spells were refused above")
            }
        };
        let (optional, stated) = match p.shape {
            LineShape::Repeated { .. } => (false, list(object, &e.id, kind)?),
            LineShape::One { required, .. } => (!required, one(object, &e.id, !required, kind)?),
            LineShape::Records { .. } => {
                unreachable!("the shapes no printer spells were refused above")
            }
        };
        out.push(StatedElement {
            id: e.id.clone(),
            optional,
            stated,
        });
        for param in &p.params {
            let ParamValue::Text { max_size } = &param.value else {
                unreachable!("the shapes no printer spells were refused above")
            };
            let optional = !param.required;
            out.push(StatedElement {
                id: param.entry.id.clone(),
                optional,
                stated: one(
                    object,
                    &param.entry.id,
                    optional,
                    Kind::Text { most: *max_size },
                )?,
            });
        }
    }
    Ok(out)
}
