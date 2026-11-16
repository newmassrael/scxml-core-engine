// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the Swift conformance harness (docs/adr/0081 §5): a value a case states as the
//! codec's interface presents it, made by the printed value's initializer, and for each element an
//! expression that holds when a decode of the case's bytes presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! (`rust_source::source_members` and the readers beside it), so one case states one value in every
//! language. It is written for a codec the Swift printer prints ([`super::source::unlowered`]), and
//! refuses any other by the form the printer does not lower, as a case of a shape no printer spells
//! is refused by name.

use super::super::rust_source::{
    case_inputs, case_tag, enum_variant, integer_value, source_members, HeldCodecs,
};
use super::*;
use crate::forge::encode_plan::{EncodePlan, Value};

/// The name the harness gives the value a case makes.
const VALUE: &str = "value";

/// The name the harness gives the value its decode of the case's bytes answers.
const DECODED: &str = "decoded";

/// A byte case of the Swift harness (docs/adr/0081 §5).
pub(crate) struct SourceCase {
    /// The declarations the literal reads, in order.
    pub bindings: Vec<String>,
    /// The value, made by its type's initializer.
    pub literal: String,
    /// Each element by its id, with a `Bool` expression over `decoded` and `value` that holds when
    /// the two present the same.
    pub checks: Vec<(String, String)>,
    /// The arguments of `decode` after the cursor.
    pub inputs: Vec<String>,
    /// The arguments of `encodeSource` after the sink.
    pub encode_inputs: Vec<String>,
}

/// The byte case of `source`, a value of the codec `plan` whose Swift type is `type_name`, its
/// parent handing it `inputs` and its caller `tag` (`null` where it takes none). A codec the Swift
/// printer does not print is refused by the first form it does not lower, and so is a source that
/// states an element the value does not present or leaves out one it does.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    type_name: &str,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    tag: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    if let Some(form) = super::source::unlowered_positional(plan) {
        return Err(format!(
            "the Swift printer does not print {form}, so no value of the codec is made from a case"
        ));
    }
    let object = source_members(plan, source)?;
    // A printed codec takes no flag input and its variant, which it has none of, no tag: the
    // readers refuse a case that hands either.
    if !case_inputs(plan, inputs)?.is_empty() {
        return Err("a case hands flag inputs to a codec that takes none".to_string());
    }
    if case_tag(plan, source, tag)?.is_some() {
        return Err("a case hands a tag to a codec whose variant takes none".to_string());
    }
    let mut arguments = Vec::new();
    let mut checks = Vec::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let value = object
            .get(id)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        let spelled =
            number_literal(&element.value, value, &*held).map_err(|e| format!("`{id}`: {e}"))?;
        arguments.push(format!("{id}: {spelled}"));
        checks.push((id.clone(), format!("{DECODED}.{id} == {VALUE}.{id}")));
    }
    Ok(SourceCase {
        bindings: Vec::new(),
        literal: format!("{type_name}({})", arguments.join(", ")),
        checks,
        inputs: Vec::new(),
        encode_inputs: Vec::new(),
    })
}

/// The Swift literal of a number a source states: an integer as itself, which the parameter's
/// type reads, and a value of an imported enum as the member the enum's unit declares for the
/// variant it names.
fn number_literal(
    value: &Value<'_>,
    json: &serde_json::Value,
    held: &dyn HeldCodecs,
) -> Result<String, String> {
    match value {
        Value::Scalar(SceType::Enum(r)) => {
            let (held_enum, variant) = enum_variant(held, &r.alias, json)?;
            Ok(format!(
                "{}.{}",
                filters::to_pascal_case(held_enum.name.clone()),
                crate::forge::enum_naming::variant_ident(
                    crate::generator::Language::Swift,
                    &held_enum.name,
                    variant
                )
            ))
        }
        Value::Scalar(t) if t.int_bit_width().is_some() => Ok(integer_value(json)?.to_string()),
        _ => Err("a value the Swift printer does not print".to_string()),
    }
}
