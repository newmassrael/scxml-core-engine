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
    byte_values, case_inputs, case_tag, entry_rows, enum_variant, gates_label, group_choice,
    integer_value, source_members, variant_choice, HeldCodecs, CALLER_TAG_LABEL, VARIANT_MEMBER,
};
use super::super::*;
use super::group::groups;
use super::{arm_kind, arm_member, BODY};
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
    /// Each element by its id, and each input the value says, with a C
    /// expression over `decoded` and `value` that holds when the two present
    /// the same, or when the value says the input the case hands it.
    pub checks: Vec<(String, String)>,
    /// The arguments of the decode after the value it writes: the flag inputs
    /// the case hands the codec, in the order the codec declares them.
    pub inputs: Vec<String>,
    /// The flag inputs the encode is handed after the sink: those the value
    /// does not say, in the order the codec declares them.
    pub encode_inputs: Vec<String>,
}

impl SourceCase {
    /// A case that sets nothing and hands nothing, which the members of a
    /// value, a body or the entries of a list are written into.
    fn empty() -> Self {
        SourceCase {
            statements: Vec::new(),
            checks: Vec::new(),
            inputs: Vec::new(),
            encode_inputs: Vec::new(),
        }
    }
}

/// The byte case of `source`, a value of the codec `plan`, its parent handing
/// it `inputs` (`{"name": 0 | 1}`, or `null` for a codec that takes none). The
/// case names no symbol of the codec but the function of an input the value
/// says: the harness holds the value in the codec's struct, and each statement
/// sets a member of it. Each input is handed to the decode, one the value does
/// not say to the encode too, and the value is held to saying each other one
/// through the input's own function (`{codec}_input_<name>`, docs/adr/0065 §6
/// R4a), which answers an absent value where the value says nothing of it. A
/// variant with no tag of its own is handed the tag of the arm the source
/// holds (`tag`, [`case_tag`]) after the inputs, and the value made and the
/// value decoded are held to answering it (`{codec}_tag`). A codec that holds
/// a group is one the C11 printer does not print, and its case is refused by
/// name; so is a source that states an element the value does not present or
/// leaves out one it does.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    tag: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    let snake = filters::to_snake_case(plan.model.name.clone());
    let mut args = Vec::new();
    let mut given = Vec::new();
    let mut said = Vec::new();
    for (input, value, to_encode) in case_inputs(plan, inputs)? {
        args.push(format!("{value}u"));
        if to_encode {
            given.push(format!("{value}u"));
            continue;
        }
        let says = format!(
            "{snake}_input_{}({snake}_value_source(), &{VALUE})",
            filters::to_snake_case(input.name.clone())
        );
        let holds = if plan.roles.says_input_partly(&input.name) {
            format!("(!{says}.present || {says}.value == {value}u)")
        } else {
            format!("{says} == {value}u")
        };
        said.push((format!("flag input {}", input.name), holds));
    }
    // The decode of a variant with no tag of its own is handed the tag of the
    // arm the source holds after the flag inputs, and the value made and the
    // value decoded both answer it (`{codec}_tag`).
    let tag = case_tag(plan, source, tag)?;
    if let Some(tag) = tag {
        args.push(format!("{tag}u"));
    }
    let mut case = SourceCase {
        inputs: args,
        encode_inputs: given,
        ..SourceCase::empty()
    };
    members(plan, source, "", "", held, &mut case)?;
    case.checks.extend(said);
    if let Some(tag) = tag {
        case.checks.push((
            CALLER_TAG_LABEL.to_string(),
            format!("{snake}_tag(&{DECODED}) == {tag}u && {snake}_tag(&{VALUE}) == {tag}u"),
        ));
    }
    Ok(case)
}

/// Into `case`, the statements that set each element of `source`, a value of
/// the codec `plan` held at the member path `at` of the case's value (`""`
/// for the value itself, `"body."` or `"body.value."` for a body it holds,
/// `"list[0]."` or `"list.value.data[0]."` for an entry of a list it holds,
/// `"body.arm.<arm>."` for the body of its variant's arm), and the check of
/// each, by its id after `id_at`. A body, each entry of a list and the arm of
/// a variant are set member by member in turn, as the struct holds them.
fn members(
    plan: &EncodePlan<'_>,
    source: &serde_json::Value,
    at: &str,
    id_at: &str,
    held: &mut dyn HeldCodecs,
    case: &mut SourceCase,
) -> Result<(), String> {
    let object = source_members(plan, source)?;
    let l = LangCtx::new(crate::generator::Language::C11, &[]);
    let SourceCase {
        statements, checks, ..
    } = case;
    for element in &plan.elements {
        let declared = &element.field.id;
        let id = format!("{id_at}{declared}");
        let member = format!("{at}{}", l.codec_field_id(declared));
        let json = object
            .get(declared)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        // One a gate decides, and a number the value states while its content
        // is absent, is held in the optional value of what it holds: absent as
        // the zeroed value is, or present with its value set. A decode answers
        // a stated number present exactly where the content is absent, as
        // every case states it.
        let gated = element.presence.optional();
        if json.is_null() {
            if !gated {
                return Err(format!(
                    "`{id}` is always there in a codec the C11 printer prints, and the source \
                     states none"
                ));
            }
            checks.push((id.clone(), format!("!{DECODED}.{member}.present")));
            continue;
        }
        let (data, len, scalar) = if gated {
            statements.push(format!("{VALUE}.{member}.present = true;"));
            (
                format!("{member}.value.data"),
                format!("{member}.value.len"),
                format!("{member}.value"),
            )
        } else {
            (member.clone(), format!("{member}_len"), member.clone())
        };
        let check = |holds: String| {
            if gated {
                format!("{DECODED}.{member}.present && {holds}")
            } else {
                holds
            }
        };
        match &element.value {
            // The bytes into the array that holds them and their count beside
            // it, the length member the C11 struct names `{member}_len`, or
            // the value of the optional one a gate decides; a text is held as
            // the bytes of its UTF-8, as the decode holds it.
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
                        "memcpy({VALUE}.{data}, (const uint8_t[]){{{}}}, {});",
                        listed.join(", "),
                        bytes.len()
                    ));
                }
                statements.push(format!("{VALUE}.{len} = {};", bytes.len()));
                checks.push((
                    id.clone(),
                    check(format!(
                        "{DECODED}.{len} == {VALUE}.{len} && \
                         memcmp({DECODED}.{data}, {VALUE}.{data}, {VALUE}.{len}) == 0"
                    )),
                ));
            }
            // The members of the body the struct holds, as its codec's own
            // case sets them, under the member that holds it.
            Value::Body { codec } => {
                let inner_at = if gated {
                    format!("{member}.value.")
                } else {
                    format!("{member}.")
                };
                let inner_id = format!("{id}.");
                let mut body = SourceCase::empty();
                held.read(codec, &mut |body_plan, _path, held| {
                    members(body_plan, json, &inner_at, &inner_id, held, &mut body)
                        .map(|()| String::new())
                })
                .map_err(|e| format!("`{id}`: {e}"))?;
                if gated {
                    checks.push((id.clone(), format!("{DECODED}.{member}.present")));
                }
                statements.extend(body.statements);
                checks.extend(body.checks);
            }
            // Each entry into the array that holds them, member by member as
            // its codec's own case sets them, and their count beside it, the
            // length member the C11 struct names `{member}_len`, or the value
            // of the optional one a gate decides. A list held to a law is
            // stated with its start, which no C11 list holds yet.
            Value::List { codec, law, .. } => {
                if !law.is_none() {
                    return Err(format!(
                        "`{id}`: a list held to a law is not stated as a C11 value yet: the \
                         C11 printer prints no codec that holds one"
                    ));
                }
                let rows = entry_rows(json).map_err(|e| format!("`{id}`: {e}"))?;
                let mut entries = SourceCase::empty();
                for (i, row) in rows.iter().enumerate() {
                    let inner_at = format!("{data}[{i}].");
                    let inner_id = format!("{id}[{i}].");
                    held.read(codec, &mut |entry_plan, _path, held| {
                        members(entry_plan, row, &inner_at, &inner_id, held, &mut entries)
                            .map(|()| String::new())
                    })
                    .map_err(|e| format!("`{id}`: {e}"))?;
                }
                statements.push(format!("{VALUE}.{len} = {};", rows.len()));
                checks.push((
                    id.clone(),
                    check(format!("{DECODED}.{len} == {VALUE}.{len}")),
                ));
                statements.extend(entries.statements);
                checks.extend(entries.checks);
            }
            _ => {
                let literal = member_literal(&element.value, json, held)
                    .map_err(|e| format!("`{id}`: {e}"))?;
                statements.push(format!("{VALUE}.{scalar} = {literal};"));
                checks.push((
                    id.clone(),
                    check(format!("{DECODED}.{scalar} == {VALUE}.{scalar}")),
                ));
            }
        }
    }
    // Each group, held in the struct's member of it: the arm the source
    // states, its kind and each of its members, or no arm, as the zeroed value
    // holds none (docs/adr/0065 §6 R4).
    let snake = filters::to_snake_case(plan.model.name.clone());
    for g in groups(plan, &snake) {
        let label = gates_label(g.group);
        let id = format!("{id_at}{label}");
        let json = object
            .get(&label)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        let chosen = group_choice(g.group, json).map_err(|e| format!("`{id}`: {e}"))?;
        let mut literals = Vec::new();
        let arm = match chosen {
            None => None,
            Some(chosen) => {
                for member in &chosen.arm.fields {
                    let field = plan
                        .model
                        .fields
                        .iter()
                        .find(|f| &f.id == member)
                        .ok_or_else(|| format!("`{id}`: the codec has no field `{member}`"))?;
                    let stated = chosen
                        .members
                        .get(member)
                        .ok_or_else(|| format!("`{id}`: the arm states no `{member}`"))?;
                    let literal = member_literal(&plan.value(field), stated, held)
                        .map_err(|e| format!("`{id}.{member}`: {e}"))?;
                    literals.push((member.clone(), literal));
                }
                Some(chosen.index)
            }
        };
        let (set, held_to) = g.stated(at, arm, &literals, &id, (VALUE, DECODED));
        statements.extend(set);
        checks.extend(held_to);
    }
    // The arm of the variant, held in the struct's variant: its kind, the tag
    // the catch-all holds, and the members of its body in the union's member
    // of the arm, as its codec's own case sets them.
    if plan.model.variant.is_some() {
        let id = format!("{id_at}{VARIANT_MEMBER}");
        let json = object
            .get(VARIANT_MEMBER)
            .ok_or_else(|| format!("the source states no `{id}`, the variant's arm"))?;
        let (chosen, body) = variant_choice(plan, json).map_err(|e| format!("`{id}`: {e}"))?;
        let named = chosen.default_tag.is_none().then_some(chosen.codec);
        let variant = format!("{at}{BODY}");
        statements.push(format!(
            "{VALUE}.{variant}.kind = {};",
            arm_kind(&plan.model.name, named)
        ));
        checks.push((
            id.clone(),
            format!("{DECODED}.{variant}.kind == {VALUE}.{variant}.kind"),
        ));
        if let Some(tag) = chosen.default_tag {
            statements.push(format!("{VALUE}.{variant}.default_tag = {tag}u;"));
            checks.push((
                format!("{id} tag"),
                format!("{DECODED}.{variant}.default_tag == {VALUE}.{variant}.default_tag"),
            ));
        }
        let inner_at = format!("{variant}.arm.{}.", arm_member(named));
        let inner_id = format!("{id}.");
        let mut arm = SourceCase::empty();
        held.read(chosen.codec, &mut |arm_plan, _path, held| {
            members(arm_plan, body, &inner_at, &inner_id, held, &mut arm).map(|()| String::new())
        })
        .map_err(|e| format!("`{id}`: {e}"))?;
        statements.extend(arm.statements);
        checks.extend(arm.checks);
    }
    Ok(())
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
        Value::Body { .. } => unreachable!("a body is set member by member"),
        Value::List { .. } => unreachable!("a list is set entry by entry"),
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
