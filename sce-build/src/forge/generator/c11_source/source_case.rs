// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the C11 conformance harness (docs/adr/0081 §5): a value a case
//! states as the codec's interface presents it, held in the codec's struct, and
//! for each element an expression that holds when a decode of the case's bytes
//! presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! ([`source_members`] and the readers beside it), so one case states one value
//! in every language. A C value is made by setting its members, so the case is a
//! statement for each element the value presents, on a value the harness has
//! zeroed first; a struct that holds an array of megabytes is then never a
//! literal on a stack. A shape the C11 printer does not print yet is refused by
//! name, as the Rust case refuses what it does not spell.

use super::super::rust_source::{
    byte_values, case_inputs, case_tag, enum_variant, integer_value, source_members, HeldCodecs,
};
use super::super::*;
use crate::forge::encode_plan::{EncodePlan, Value};

/// The name the harness gives the value a case makes.
const VALUE: &str = "value";

/// The name the harness gives the value its decode of the case's bytes writes.
const DECODED: &str = "decoded";

/// A byte case of the C11 harness (docs/adr/0081 §5).
pub(crate) struct SourceCase {
    /// The statements that set each element the value presents, in order, on
    /// a value the harness has zeroed.
    pub statements: Vec<String>,
    /// Each element by its id, with a C expression over `decoded` and `value`
    /// that holds when the two present the same.
    pub checks: Vec<(String, String)>,
}

/// The byte case of `source`, a value of the codec `plan`. The case names no
/// symbol of the codec: the harness holds the value in the codec's struct, and
/// each statement sets a member of it. A codec whose case hands it flag inputs
/// (`inputs`) or a caller's tag (`tag`), or that holds a group or a variant, is
/// one the C11 printer does not print, and its case is refused by name; so is a
/// source that states an element the value does not present or leaves out one
/// it does.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    tag: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    if !case_inputs(plan, inputs)?.is_empty() {
        return Err("a C11 case hands no flag input yet: the C11 printer \
                    prints no codec that takes one"
            .to_string());
    }
    if case_tag(plan, source, tag)?.is_some() || plan.model.variant.is_some() {
        return Err("a C11 case states no variant yet: the C11 printer \
                    prints no codec that holds one"
            .to_string());
    }
    if !plan.groups().is_empty() {
        return Err("a C11 case states no group yet: the C11 printer \
                    prints no codec that holds one"
            .to_string());
    }
    let object = source_members(plan, source)?;
    let l = LangCtx::new(crate::generator::Language::C11, &[]);
    let mut statements = Vec::new();
    let mut checks = Vec::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let member = l.codec_field_id(id);
        let json = object
            .get(id)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        if json.is_null() {
            return Err(format!(
                "`{id}` is always there in a codec the C11 printer prints, and the source \
                 states none"
            ));
        }
        match &element.value {
            // The bytes into the array that holds them and their count beside
            // it, the length member the C11 struct names `{member}_len`; a
            // text is held as the bytes of its UTF-8, as the decode holds it.
            Value::Bytes(_) | Value::Text(_) => {
                let bytes = match &element.value {
                    Value::Text(_) => json
                        .as_str()
                        .ok_or_else(|| format!("`{id}`: text is a JSON string"))?
                        .as_bytes()
                        .to_vec(),
                    _ => byte_values(json).map_err(|e| format!("`{id}`: {e}"))?,
                };
                if !bytes.is_empty() {
                    let listed: Vec<String> = bytes.iter().map(|b| format!("0x{b:02X}")).collect();
                    statements.push(format!(
                        "memcpy({VALUE}.{member}, (const uint8_t[]){{{}}}, {});",
                        listed.join(", "),
                        bytes.len()
                    ));
                }
                statements.push(format!("{VALUE}.{member}_len = {};", bytes.len()));
                checks.push((
                    id.clone(),
                    format!(
                        "{DECODED}.{member}_len == {VALUE}.{member}_len && \
                         memcmp({DECODED}.{member}, {VALUE}.{member}, {VALUE}.{member}_len) == 0"
                    ),
                ));
            }
            _ => {
                let literal = member_literal(&element.value, json, held)
                    .map_err(|e| format!("`{id}`: {e}"))?;
                statements.push(format!("{VALUE}.{member} = {literal};"));
                checks.push((
                    id.clone(),
                    format!("{DECODED}.{member} == {VALUE}.{member}"),
                ));
            }
        }
    }
    Ok(SourceCase { statements, checks })
}

/// One value of the abstract type `value`, as a C expression a member of the
/// struct is set to: a number as a literal of its type's signedness, and a
/// value of an enum as the constant its header declares.
fn member_literal(
    value: &Value<'_>,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    match value {
        Value::Scalar(SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64) => {
            integer_value(json).map(|n| format!("{n}u"))
        }
        Value::Scalar(SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64) => {
            integer_value(json).map(signed_literal)
        }
        Value::Scalar(SceType::Enum(enum_ref)) => {
            let (held_enum, variant) = enum_variant(held, &enum_ref.alias, json)?;
            // A C enum's constant carries its type's name, so nothing
            // qualifies it.
            Ok(crate::forge::enum_naming::variant_ref(
                crate::generator::Language::C11,
                "",
                &held_enum.name,
                variant,
            ))
        }
        Value::Scalar(_) => Err("a float or a bool is not spelled as a C11 literal yet".into()),
        Value::Bytes(_) | Value::Text(_) => {
            unreachable!("a byte string or a text is set by statements of its own")
        }
        Value::Body { .. } | Value::List { .. } => Err(
            "a body or a list is not spelled as a C11 literal yet: the C11 printer prints no \
             codec that holds one"
                .into(),
        ),
    }
}

/// A signed integer as a C literal: the least `int64_t` is no literal C writes,
/// as its magnitude is past the type, so it is the next one less one.
fn signed_literal(n: i128) -> String {
    if n == i128::from(i64::MIN) {
        format!("({} - 1)", n + 1)
    } else {
        n.to_string()
    }
}
