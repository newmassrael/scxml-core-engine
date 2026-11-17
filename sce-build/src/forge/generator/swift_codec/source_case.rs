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
    case_inputs, case_tag, enum_variant, gates_label, group_choice, integer_value, source_members,
    variant_choice, ChosenGroupArm, HeldCodecs, CALLER_TAG_LABEL, VARIANT_MEMBER,
};
use super::source::{presented_order, Presented};
use super::*;
use crate::forge::codec_roles::GateGroup;
use crate::forge::encode_plan::{EncodePlan, Law, Value};

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
    // as the case hands it, through the input's own function (docs/adr/0065 §6 R4a).
    let mut decode_inputs = Vec::new();
    let mut encode_inputs = Vec::new();
    let mut said = Vec::new();
    for (input, value, given) in case_inputs(plan, inputs)? {
        let argument = format!("{}: {value}", input.name);
        decode_inputs.push(argument.clone());
        if given {
            encode_inputs.push(argument);
        } else {
            let says = format!(
                "{type_name}.{}({VALUE})",
                super::source::said_reader(&input.name)
            );
            // A value that says nothing of the input contradicts no value of it (docs/adr/0065
            // §6 R4e).
            let holds = if plan.roles.says_input_partly(&input.name) {
                format!("({says}.map {{ $0 == {value} }} ?? true)")
            } else {
                format!("{says} == {value}")
            };
            said.push((format!("flag input {}", input.name), holds));
        }
    }
    // A variant whose caller hands its tag is handed the tag of the arm the source holds, after
    // the flag inputs, and the value made and the value decoded both answer it.
    let caller_tag = case_tag(plan, source, tag)?;
    if let Some(tag) = caller_tag {
        decode_inputs.push(format!("tag: {tag}"));
    }
    // A decode presents each element as the value holds it, and every value the Swift printer
    // prints is `Equatable`, a body's down into its own elements and a variant's into the body of
    // the arm it holds: one comparison an element.
    let mut checks: Vec<(String, String)> = plan
        .elements
        .iter()
        .map(|element| {
            let id = &element.field.id;
            (id.clone(), format!("{DECODED}.{id} == {VALUE}.{id}"))
        })
        .collect();
    // A gate group is `Equatable` down into its members: one comparison a group, by its gates.
    for group in plan.groups() {
        let name = super::source::group_name(group);
        checks.push((
            gates_label(group),
            format!("{DECODED}.{name} == {VALUE}.{name}"),
        ));
    }
    if plan.model.variant.is_some() {
        checks.push((
            VARIANT_MEMBER.to_string(),
            format!("{DECODED}.{VARIANT_MEMBER} == {VALUE}.{VARIANT_MEMBER}"),
        ));
    }
    if let Some(tag) = caller_tag {
        checks.push((
            CALLER_TAG_LABEL.to_string(),
            format!(
                "{DECODED}.{VARIANT_MEMBER}.tag == {tag} && {VALUE}.{VARIANT_MEMBER}.tag == {tag}"
            ),
        ));
    }
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
/// initializer of its own codec's value; a list of entries each made so, through its count bound's
/// door; a number as a literal; a gate group, where its first member stands, as [`group_literal`]
/// makes it; the variant's arm as [`variant_literal`] makes it, last. An element a gate leaves out
/// is `nil`. A shape the Swift printer does not print (a list held to its law) is refused by name,
/// and so is a source that states an element the value does not present or leaves out one it does.
fn value_literal(
    plan: &EncodePlan<'_>,
    type_name: &str,
    source: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let object = source_members(plan, source)?;
    let mut arguments = Vec::new();
    for presented in presented_order(plan) {
        let element = match presented {
            Presented::Element(element) => element,
            Presented::Group(index) => {
                let group = &plan.groups()[index];
                let label = gates_label(group);
                let value = object
                    .get(&label)
                    .ok_or_else(|| format!("the source states no `{label}`"))?;
                let spelled = group_literal(plan, type_name, group, value, held, bindings)
                    .map_err(|e| format!("`{label}`: {e}"))?;
                arguments.push(format!("{}: {spelled}", super::source::group_name(group)));
                continue;
            }
        };
        let id = &element.field.id;
        let value = object
            .get(id)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        let spelled = if value.is_null() && element.presence.optional() {
            "nil".to_string()
        } else if let Some(bounded) = super::source::bounded_type(type_name, element) {
            bounded_literal(&bounded, &element.value, value, bindings)
                .map_err(|e| format!("`{id}`: {e}"))?
        } else if let Value::Body { codec } = &element.value {
            held.read(codec, &mut |plan, _, held| {
                let type_name = filters::to_pascal_case(plan.model.name.clone());
                value_literal(plan, &type_name, value, held, bindings)
            })
            .map_err(|e| format!("`{id}`: {e}"))?
        } else if let Value::List { codec, law, .. } = &element.value {
            list_literal(type_name, id, codec, law, value, held, bindings)
                .map_err(|e| format!("`{id}`: {e}"))?
        } else {
            number_literal(&element.value, value, &*held).map_err(|e| format!("`{id}`: {e}"))?
        };
        arguments.push(format!("{id}: {spelled}"));
    }
    // The variant, last: the arm the source holds, its body made by its own codec's initializer.
    if plan.model.variant.is_some() {
        let stated = object
            .get(VARIANT_MEMBER)
            .ok_or_else(|| format!("the source states no `{VARIANT_MEMBER}`, the variant's arm"))?;
        let arm = variant_literal(plan, type_name, stated, held, bindings)
            .map_err(|e| format!("`{VARIANT_MEMBER}`: {e}"))?;
        arguments.push(format!("{VARIANT_MEMBER}: {arm}"));
    }
    Ok(format!("{type_name}({})", arguments.join(", ")))
}

/// The byte string or text `value` holds as a source states it (`json`), made through the one door
/// of `bounded`, its bounded type (docs/adr/0065 §6 R1), bound first in `bindings`, which the
/// harness requires to answer: the name of the binding.
fn bounded_literal(
    bounded: &str,
    value: &Value<'_>,
    json: &serde_json::Value,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let raw = match value {
        Value::Text(_) => text_literal(json),
        _ => bytes_literal(json),
    }?;
    // Numbered, as a body's elements are bound beside its holder's.
    let local = format!("bound{}", bindings.len());
    bindings.push(format!("let {local} = try #require({bounded}({raw}))"));
    Ok(local)
}

/// The value of the gate group `group` of the codec `plan`, whose Swift type is `type_name`, that
/// `json` states (`rust_source::group_choice`): `nil` where it states none, else the arm it states,
/// the case of the group's enum or its one arm's struct, each member as the arm holds it (docs/adr
/// /0065 §6 R4b, R4c): a number as a literal, a byte string or a text through its bound's door, a
/// body by its own codec's initializer, a list of entries through its count bound's door, as an
/// element is. The group's type is generic over the source of each body and list it holds, which
/// the value the initializer is handed binds to each codec's own value.
fn group_literal(
    plan: &EncodePlan<'_>,
    type_name: &str,
    group: &GateGroup,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let Some(ChosenGroupArm {
        arm,
        members: object,
        ..
    }) = group_choice(group, json)?
    else {
        return Ok("nil".to_string());
    };
    let mut members = Vec::new();
    for id in &arm.fields {
        let field = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == *id)
            .ok_or_else(|| format!("the codec has no field `{id}`"))?;
        let stated = object
            .get(id)
            .ok_or_else(|| format!("the arm states no `{id}`"))?;
        let value = plan.value(field);
        let spelled = match (
            super::source::bounded_field_type(type_name, plan, field),
            &value,
        ) {
            (Some(bounded), _) => bounded_literal(&bounded, &value, stated, bindings),
            (None, Value::Body { codec }) => held.read(codec, &mut |plan, _, held| {
                let type_name = filters::to_pascal_case(plan.model.name.clone());
                value_literal(plan, &type_name, stated, held, bindings)
            }),
            (None, Value::List { codec, law, .. }) => {
                list_literal(type_name, id, codec, law, stated, held, bindings)
            }
            (None, _) => number_literal(&value, stated, &*held),
        }
        .map_err(|e| format!("`{id}`: {e}"))?;
        members.push(format!("{id}: {spelled}"));
    }
    let group_type = super::source::group_type_name(type_name, group);
    Ok(match (group.is_sum(), members.is_empty()) {
        (true, true) => format!("{group_type}.{}", super::source::group_case(group, arm)),
        (true, false) => format!(
            "{group_type}.{}({})",
            super::source::group_case(group, arm),
            members.join(", ")
        ),
        (false, _) => format!("{group_type}({})", members.join(", ")),
    })
}

/// The arm of the variant of the codec `plan`, whose Swift type is `type_name`, that `json` states
/// (`rust_source::variant_choice`): the case of the arm, its body made by its codec's initializer,
/// and for the catch-all the tag it holds, made through its type's one door (docs/adr/0065 §6 R3),
/// bound first in `bindings`, which the harness requires to answer.
fn variant_literal(
    plan: &EncodePlan<'_>,
    type_name: &str,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let (arm, body) = variant_choice(plan, json)?;
    let literal = held.read(arm.codec, &mut |plan, _, held| {
        let type_name = filters::to_pascal_case(plan.model.name.clone());
        value_literal(plan, &type_name, body, held, bindings)
    })?;
    Ok(match arm.default_tag {
        None => format!(
            ".{}({literal})",
            filters::to_camel_case(arm.codec.to_string())
        ),
        Some(tag) => {
            // Numbered, as a byte string's door is bound.
            let local = format!("bound{}", bindings.len());
            bindings.push(format!(
                "let {local} = try #require({type_name}UnknownTag(0x{tag:X}))"
            ));
            format!(".default(tag: {local}, body: {literal})")
        }
    })
}

/// The list or chain `id` of the codec whose Swift type is `type_name`, an element or a member of a
/// gate group, its entries of the codec imported as `codec`, that `json` states as an array of its
/// entries: each made by its own codec's initializer, and the list through its count bound's one
/// door (docs/adr/0065 §6 R2), bound first in `bindings`, which the harness requires to answer. A
/// list held to its law, which a source states with its start, is refused by name.
fn list_literal(
    type_name: &str,
    id: &str,
    codec: &str,
    law: &Law,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    if !law.is_none() {
        return Err(
            "a list held to the law of the inputs its entries say, which the Swift printer does \
             not print"
                .to_string(),
        );
    }
    let stated = json
        .as_array()
        .ok_or_else(|| "a list is an array of its entries".to_string())?;
    let mut entries = Vec::new();
    for (i, entry) in stated.iter().enumerate() {
        let literal = held
            .read(codec, &mut |plan, _, held| {
                let type_name = filters::to_pascal_case(plan.model.name.clone());
                value_literal(plan, &type_name, entry, held, bindings)
            })
            .map_err(|e| format!("entry {i}: {e}"))?;
        entries.push(literal);
    }
    let entry_type = held.read(codec, &mut |plan, _, _| {
        Ok(filters::to_pascal_case(plan.model.name.clone()))
    })?;
    let list = super::source::list_type(type_name, id, &entry_type);
    // Numbered, as a byte string's door is bound.
    let local = format!("bound{}", bindings.len());
    bindings.push(format!(
        "let {local} = try #require({list}([{}]))",
        entries.join(", ")
    ));
    Ok(local)
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
