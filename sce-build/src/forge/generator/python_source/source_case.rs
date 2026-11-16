// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the Python conformance harness (docs/adr/0081 §5): a value a
//! case states as the codec's interface presents it, made by the printed
//! dataclass's constructor, and for each element an expression that holds when
//! a decode of the case's bytes presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! ([`source_members`] and the readers beside it), so one case states one value
//! in every language. A byte string is handed to the constructor as `bytes`,
//! which the value holds in the class of its bound where it is made, so the
//! literal names no bound. A shape the Python printer does not print yet is
//! refused by name, as the Rust case refuses what it does not spell.

use super::super::rust_source::{
    byte_values, case_inputs, case_tag, enum_variant, gates_label, group_accessor, group_choice,
    integer_value, source_members, variant_choice, CodecPath as RustCodecPath, HeldCodecs,
    CALLER_TAG_LABEL, VARIANT_MEMBER,
};
use super::super::*;
use crate::forge::encode_plan::{EncodePlan, Value};

/// Where a codec's Python value is: the module the harness imports it as and
/// the dataclass of its value, both named after the document's name.
#[derive(Clone, Debug)]
pub(crate) struct CodecPath {
    /// The module the harness imports, the document's name.
    pub module: String,
    /// The dataclass of the value, the document's name in Pascal case.
    pub type_name: String,
}

impl CodecPath {
    /// The codec the document `name` generates.
    pub(crate) fn new(name: &str) -> Self {
        CodecPath {
            module: name.to_string(),
            type_name: filters::to_pascal_case(name.to_string()),
        }
    }

    /// The dataclass of its value, as the harness reaches it.
    fn class(&self) -> String {
        format!("{}.{}", self.module, self.type_name)
    }
}

/// Where a codec held by the codec at `holder` is, whose Rust path a held
/// codec's reading names `held`: its module, which the printed holder imports
/// whole under `sce_<snake>`, reached through the holder's.
fn held_codec(holder: &CodecPath, held: &RustCodecPath) -> CodecPath {
    let snake = held.module.rsplit("::").next().unwrap_or(&held.module);
    CodecPath {
        module: format!("{}.{}", holder.module, python_module_alias(snake)),
        type_name: held.type_name.clone(),
    }
}

/// The name the harness gives the value a case makes.
const VALUE: &str = "value";

/// The name the harness gives the value its decode of the case's bytes answers.
const DECODED: &str = "decoded";

/// A byte case of the Python harness (docs/adr/0081 §5).
pub(crate) struct SourceCase {
    /// The value, made by its dataclass's constructor.
    pub literal: String,
    /// Each element by its id, with a Python expression over `decoded` and
    /// `value` that holds when the two present the same; and each flag input
    /// the value says, with one that holds when it says the input the case is
    /// handed.
    pub checks: Vec<(String, String)>,
    /// The arguments the decode is handed after its cursor: each flag input,
    /// in the order the codec declares them.
    pub inputs: Vec<String>,
    /// The arguments the encode is handed after its sink: the flag inputs the
    /// value does not say.
    pub encode_inputs: Vec<String>,
}

/// The byte case of `source`, a value of the codec `plan` at `path`, handed
/// the flag inputs `inputs` states: each to the decode, one the value does not
/// say to the encode too, and the value held to saying each other one. A group
/// is the class of the arm the source states, and a variant the arm: a value
/// of the arm's codec, or the default arm of the tag it keeps and its body. A
/// caller's tag (`tag`) is handed to the decode after the inputs, and the
/// value and its decode are held to answering it. A source that states an
/// element the value does not present or leaves out one it does is refused
/// by name.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    tag: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    // Each input as the decode is handed it, in the order the codec declares
    // them, and those the value does not say as the encode is handed them; the
    // value is held to saying each other one through the input's own function
    // (`input_<name>`, docs/adr/0065 §6 R4a), which answers `None` where the
    // value says nothing of it.
    let mut decode_inputs = Vec::new();
    let mut encode_inputs = Vec::new();
    let mut said = Vec::new();
    for (input, value, given) in case_inputs(plan, inputs)? {
        decode_inputs.push(value.to_string());
        if given {
            encode_inputs.push(value.to_string());
            continue;
        }
        let says = format!(
            "{}.input_{}({VALUE})",
            path.module,
            filters::to_snake_case(input.name.clone())
        );
        let holds = if plan.roles.says_input_partly(&input.name) {
            format!("{says} in (None, {value})")
        } else {
            format!("{says} == {value}")
        };
        said.push((format!("flag input {}", input.name), holds));
    }
    let tag = case_tag(plan, source, tag)?;
    let literal = value_literal(plan, path, source, held)?;
    let l = LangCtx::new(crate::generator::Language::Python, &[]);
    // A body is its codec's dataclass, compared whole as a dataclass is, and
    // so is a group: the same arm's class on both sides, or `None`, its
    // members equal.
    let mut checks: Vec<(String, String)> = plan
        .elements
        .iter()
        .map(|element| {
            let attribute = l.codec_field_id(&element.field.id);
            (
                element.field.id.clone(),
                format!("{DECODED}.{attribute} == {VALUE}.{attribute}"),
            )
        })
        .collect();
    for group in plan.groups() {
        let attribute = group_accessor(group);
        checks.push((
            gates_label(group),
            format!("{DECODED}.{attribute} == {VALUE}.{attribute}"),
        ));
    }
    // The arm, the same class on both sides, its body equal, and the default
    // arm's tag too.
    if plan.model.variant.is_some() {
        checks.push((
            VARIANT_MEMBER.to_string(),
            format!("{DECODED}.{VARIANT_MEMBER} == {VALUE}.{VARIANT_MEMBER}"),
        ));
    }
    checks.extend(said);
    if let Some(tag) = tag {
        // The decode takes the tag after the flag inputs, and the value made
        // and the value decoded under it both answer it (`tag`): it is what a
        // caller writes for the arm the value holds.
        decode_inputs.push(tag.to_string());
        checks.push((
            CALLER_TAG_LABEL.to_string(),
            format!(
                "{module}.tag({VALUE}) == {tag} and {module}.tag({DECODED}) == {tag}",
                module = path.module
            ),
        ));
    }
    Ok(SourceCase {
        literal,
        checks,
        inputs: decode_inputs,
        encode_inputs,
    })
}

/// The value of the codec `plan` at `path` that `source` states, made by its
/// dataclass's constructor, each element by its keyword.
fn value_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let object = source_members(plan, source)?;
    let l = LangCtx::new(crate::generator::Language::Python, &[]);
    let mut args = Vec::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let json = object
            .get(id)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        let value = if json.is_null() {
            if !element.presence.optional() {
                return Err(format!(
                    "`{id}` is always there, and the source states none"
                ));
            }
            "None".to_string()
        } else {
            member_literal(&element.value, path, json, held).map_err(|e| format!("`{id}`: {e}"))?
        };
        args.push(format!("{}={value}", l.codec_field_id(id)));
    }
    // A group, keyed by its gates, is the class of the arm it states, made of
    // its members, or `None` (docs/adr/0081 §5).
    for group in plan.groups() {
        let label = gates_label(group);
        let json = object
            .get(&label)
            .ok_or_else(|| format!("the source states no `{label}`"))?;
        let value = match group_choice(group, json).map_err(|e| format!("`{label}`: {e}"))? {
            None => "None".to_string(),
            Some(chosen) => {
                let class = &super::arm_classes(&path.type_name, group)[chosen.index];
                let spell = |id: &str, held: &mut dyn HeldCodecs| -> Result<String, String> {
                    let field = plan
                        .model
                        .fields
                        .iter()
                        .find(|f| f.id == id)
                        .expect("a group's member is a field of the codec");
                    let json = chosen
                        .members
                        .get(id)
                        .filter(|json| !json.is_null())
                        .ok_or_else(|| format!("`{label}`: the arm holds `{id}`"))?;
                    member_literal(&plan.value(field), path, json, held)
                        .map_err(|e| format!("`{label}`: `{id}`: {e}"))
                };
                let mut members = Vec::new();
                for id in &chosen.arm.fields {
                    // Two lists of one count are one member of the arm, an
                    // `Aligned` of the two (docs/adr/0065 §6 R5), each stated
                    // by its own id.
                    if group.is_second_in(chosen.arm, id) {
                        continue;
                    }
                    let (name, member) = match group.second_of(chosen.arm, id) {
                        Some(second) => {
                            let first_list = spell(id, &mut *held)?;
                            let second_list = spell(second, &mut *held)?;
                            (
                                super::aligned_member(&l, id, second),
                                format!("{}.Aligned({first_list}, {second_list})", path.module),
                            )
                        }
                        None => (l.codec_field_id(id), spell(id, &mut *held)?),
                    };
                    members.push(format!("{name}={member}"));
                }
                format!("{}.{class}({})", path.module, members.join(", "))
            }
        };
        args.push(format!("{}={value}", group_accessor(group)));
    }
    // The arm of the variant, a value of the arm's codec, reached as a body
    // is, or the default arm of the tag it keeps and its body.
    if plan.model.variant.is_some() {
        let json = object
            .get(VARIANT_MEMBER)
            .ok_or_else(|| format!("the source states no `{VARIANT_MEMBER}`, the variant's arm"))?;
        let (arm, body) =
            variant_choice(plan, json).map_err(|e| format!("`{VARIANT_MEMBER}`: {e}"))?;
        let literal = held
            .read(arm.codec, &mut |plan, held_path, held| {
                value_literal(plan, &held_codec(path, held_path), body, held)
            })
            .map_err(|e| format!("`{VARIANT_MEMBER}`: {e}"))?;
        let value = match arm.default_tag {
            None => literal,
            Some(tag) => format!(
                "{}.{}(tag={tag}, body={literal})",
                path.module,
                super::default_arm_class(&path.type_name)
            ),
        };
        args.push(format!("{VARIANT_MEMBER}={value}"));
    }
    Ok(format!("{}({})", path.class(), args.join(", ")))
}

/// One value of the abstract type `value` of the codec at `path`, as a Python
/// expression the dataclass's constructor takes: a number as a literal, a byte
/// string as `bytes`, a text as a string literal, and a value of an enum as the
/// variant of the type the codec's module imports it as.
fn member_literal(
    value: &Value<'_>,
    path: &CodecPath,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    match value {
        // The held codec's dataclass, reached through the module the printed
        // codec imports it under, `sce_<snake>`, as its encode reaches it.
        Value::Body { codec } => held.read(codec, &mut |plan, held_path, held| {
            value_literal(plan, &held_codec(path, held_path), json, held)
        }),
        Value::Scalar(
            SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Uint64
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32
            | SceType::Int64,
        ) => integer_value(json).map(|n| n.to_string()),
        Value::Bytes(_) => {
            let bytes: Vec<String> = byte_values(json)?.iter().map(u8::to_string).collect();
            Ok(format!("bytes([{}])", bytes.join(", ")))
        }
        Value::Scalar(SceType::Enum(enum_ref)) => {
            let (held_enum, variant) = enum_variant(held, &enum_ref.alias, json)?;
            let ty = crate::forge::generator::enum_qualified_type(
                &held_enum.name,
                crate::generator::Language::Python,
            );
            Ok(crate::forge::enum_naming::variant_ref(
                crate::generator::Language::Python,
                &format!("{}.{ty}", path.module),
                &held_enum.name,
                variant,
            ))
        }
        // A JSON string is a Python string literal of the same text: its
        // escapes are ones Python reads alike.
        Value::Text(_) => {
            let text = json
                .as_str()
                .ok_or_else(|| "text is a JSON string".to_string())?;
            serde_json::to_string(text).map_err(|e| e.to_string())
        }
        Value::Scalar(_) => Err("a float or a bool is not spelled as a Python literal yet".into()),
        // Each entry its codec's dataclass, reached as a body's is, in a list
        // the value holds within its bound where it is made.
        Value::List { codec, law, .. } if law.is_none() => {
            let entries = json
                .as_array()
                .ok_or_else(|| "a list is an array of its entries".to_string())?;
            let mut items = Vec::new();
            for entry in entries {
                items.push(held.read(codec, &mut |plan, held_path, held| {
                    value_literal(plan, &held_codec(path, held_path), entry, held)
                })?);
            }
            Ok(format!("[{}]", items.join(", ")))
        }
        Value::List { .. } => Err(
            "a list held to a law is not spelled as a Python literal yet: the Python printer \
             prints no codec that holds one"
                .into(),
        ),
    }
}
