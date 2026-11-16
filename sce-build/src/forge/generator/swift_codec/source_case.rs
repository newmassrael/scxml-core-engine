// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the Swift conformance harness (docs/adr/0081 §5): a value a case states as the
//! codec's interface presents it, made by the printed value's initializer, and for each element an
//! expression that holds when a decode of the case's bytes presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! (`rust_source::source_members` and the readers beside it), so one case states one value in every
//! language. It is written for a codec the Swift printer prints ([`super::source::unlowered`],
//! which the conformance generator asks first), and refuses by name a shape the printer does not
//! lower, as a case of a shape no printer spells is refused by name.

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
    let mut bindings = Vec::new();
    let literal = value_literal(plan, type_name, source, held, &mut bindings)?;
    // The decode is handed every flag input, in the order the codec declares them, by its label;
    // the encode the ones the value does not say, and the value is held to saying each other one
    // as the case hands it, through the input's own function (docs/adr/0065 §6 R4a). A printed
    // codec's variant, which it has none of, takes no tag.
    let mut decode_inputs = Vec::new();
    let mut encode_inputs = Vec::new();
    let mut said = Vec::new();
    for (input, value, given) in case_inputs(plan, inputs)? {
        let argument = format!("{}: {value}", input.name);
        decode_inputs.push(argument.clone());
        if given {
            encode_inputs.push(argument);
        } else {
            said.push((
                format!("flag input {}", input.name),
                format!(
                    "{type_name}.{}({VALUE}) == {value}",
                    super::source::said_reader(&input.name)
                ),
            ));
        }
    }
    if case_tag(plan, source, tag)?.is_some() {
        return Err("a case hands a tag to a codec whose variant takes none".to_string());
    }
    // A decode presents each element as the value holds it, and every value the Swift printer
    // prints is `Equatable`, a body's down into its own elements: one comparison an element.
    let mut checks: Vec<(String, String)> = plan
        .elements
        .iter()
        .map(|element| {
            let id = &element.field.id;
            (id.clone(), format!("{DECODED}.{id} == {VALUE}.{id}"))
        })
        .collect();
    checks.extend(said);
    Ok(SourceCase {
        bindings,
        literal,
        checks,
        inputs: decode_inputs,
        encode_inputs,
    })
}

/// The value of the codec `plan`, whose Swift type is `type_name`, that `source` states, made by
/// its initializer, each element in declaration order: a byte string or a text through its bound's
/// one door, bound first in `bindings`, which the harness requires to answer; a body by the
/// initializer of its own codec's value; a number as a literal. An element a gate leaves out is
/// `nil`. A shape the Swift printer does not print (a group, a list, a variant) is refused by name,
/// and so is a source that states an element the value does not present or leaves out one it does.
fn value_literal(
    plan: &EncodePlan<'_>,
    type_name: &str,
    source: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    if plan.model.variant.is_some() || !plan.groups().is_empty() {
        return Err(format!(
            "the Swift printer does not print `{}`, which holds a variant or a gate group, so no \
             value of it is made from a case",
            plan.model.name
        ));
    }
    let object = source_members(plan, source)?;
    let mut arguments = Vec::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let value = object
            .get(id)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        let spelled = if value.is_null() && element.presence.optional() {
            "nil".to_string()
        } else if let Some(bounded) = super::source::bounded_type(type_name, element) {
            let raw = match &element.value {
                Value::Text(_) => text_literal(value),
                _ => bytes_literal(value),
            }
            .map_err(|e| format!("`{id}`: {e}"))?;
            // Numbered, as a body's elements are bound beside its holder's.
            let local = format!("bound{}", bindings.len());
            bindings.push(format!("let {local} = try #require({bounded}({raw}))"));
            local
        } else if let Value::Body { codec } = &element.value {
            held.read(codec, &mut |plan, _, held| {
                let type_name = filters::to_pascal_case(plan.model.name.clone());
                value_literal(plan, &type_name, value, held, bindings)
            })
            .map_err(|e| format!("`{id}`: {e}"))?
        } else {
            number_literal(&element.value, value, &*held).map_err(|e| format!("`{id}`: {e}"))?
        };
        arguments.push(format!("{id}: {spelled}"));
    }
    Ok(format!("{type_name}({})", arguments.join(", ")))
}

/// The Swift array literal of the bytes a source states, an array of numbers each a byte.
fn bytes_literal(json: &serde_json::Value) -> Result<String, String> {
    let bytes = json
        .as_array()
        .ok_or_else(|| "a byte string is an array of numbers".to_string())?
        .iter()
        .map(|b| {
            b.as_u64()
                .filter(|b| *b <= 0xFF)
                .map(|b| b.to_string())
                .ok_or_else(|| format!("`{b}` is no byte"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("[{}]", bytes.join(", ")))
}

/// The Swift string literal of the text a source states.
fn text_literal(json: &serde_json::Value) -> Result<String, String> {
    let text = json
        .as_str()
        .ok_or_else(|| "a text is a string".to_string())?;
    Ok(format!(
        "\"{}\"",
        crate::filters::escape_swift(text.to_string())
    ))
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
