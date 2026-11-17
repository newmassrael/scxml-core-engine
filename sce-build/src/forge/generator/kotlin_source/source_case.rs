// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the Kotlin conformance harness (docs/adr/0081 §5): a value a
//! case states as the codec's interface presents it, made by the printed data
//! class's constructor, and for each element an expression that holds when a
//! decode of the case's bytes presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! ([`source_members`] and the readers beside it), so one case states one value
//! in every language. A byte string or a text is made by the factory of the
//! value class of its bound, which the literal reaches by the name the printer
//! gives it ([`refinement_class`]). Every name is written whole, by the package
//! of the document that declares it, so the harness imports nothing for a
//! case. A shape the Kotlin printer does not print yet is refused by name, as
//! the Rust case refuses what it does not spell.

use super::super::rust_source::{
    byte_values, case_inputs, case_tag, chain_optional_in, entry_rows, enum_variant, gates_label,
    group_choice, holds_chain, integer_value, law_start_bits, lawful_parts, placed_parts,
    source_members, variant_choice, ChainInArm, CodecPath as RustCodecPath, HeldCodecs,
    CALLER_TAG_LABEL, VARIANT_MEMBER,
};
use super::super::*;
use super::group::{
    aligned_property, chain_class, group_arm_class, group_property, placed_class, placed_in,
    ALIGNED,
};
use super::{
    arm_class, lawful_class, list_class, refinement_class, start_property, unknown_tag_class,
};
use crate::forge::codec_roles::{GateGroup, GroupArm};
use crate::forge::encode_plan::{Automaton, EncodePlan, Value};

/// Where a codec's Kotlin value is: the package of the document that declares
/// it and the data class of its value, both named after the document's name.
#[derive(Clone, Debug)]
pub(crate) struct CodecPath {
    /// The package of the document, `com.sce.generated.<snake>`.
    pub package: String,
    /// The data class of the value, the document's name in Pascal case.
    pub type_name: String,
}

impl CodecPath {
    /// The codec the document `name` generates.
    pub(crate) fn new(name: &str) -> Self {
        CodecPath {
            package: kotlin_forge_package(&filters::to_snake_case(name.to_string())),
            type_name: filters::to_pascal_case(name.to_string()),
        }
    }

    /// `name`, a declaration of the codec's file, written whole.
    fn qualified(&self, name: &str) -> String {
        format!("{}.{name}", self.package)
    }
}

/// Where a codec held by a codec is, whose Rust path a held codec's reading
/// names `held`: the package of its own document, which every Kotlin unit is
/// generated under whoever imports it.
fn held_codec(held: &RustCodecPath) -> CodecPath {
    let snake = held.module.rsplit("::").next().unwrap_or(&held.module);
    CodecPath {
        package: kotlin_forge_package(snake),
        type_name: held.type_name.clone(),
    }
}

/// The name the harness gives the value a case makes.
const VALUE: &str = "value";

/// The name the harness gives the value its decode of the case's bytes answers.
const DECODED: &str = "decoded";

/// A byte case of the Kotlin harness (docs/adr/0081 §5).
pub(crate) struct SourceCase {
    /// The value, made by its data class's constructor.
    pub literal: String,
    /// Each element by its id, with a Kotlin `Boolean` expression over
    /// `decoded` and `value` that holds when the two present the same.
    pub checks: Vec<(String, String)>,
    /// The arguments the decode is handed after its cursor: each flag input,
    /// in the order the codec declares them, as a `UByte` literal.
    pub inputs: Vec<String>,
    /// The arguments the encode is handed after its sink: the flag inputs the
    /// value does not say.
    pub encode_inputs: Vec<String>,
}

/// The byte case of `source`, a value of the codec `plan` at `path`, handed
/// the flag inputs `inputs` states, each to the decode and those the value does
/// not say to the encode, and the decode the tag its caller hands it (`tag`)
/// where its variant has no tag of its own. A group is the class of the arm the
/// source states, compared arm and member alike. A source that states an
/// element the value does not present or leaves out one it does is refused by
/// name, and so is a tag that is not the one of the arm the source holds.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    tag: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    // Each input as the decode is handed it, in the order the codec declares
    // them, and those the value does not say as the encode is handed them.
    let mut decode_inputs = Vec::new();
    let mut encode_inputs = Vec::new();
    for (_, value, given) in case_inputs(plan, inputs)? {
        decode_inputs.push(format!("{value}u"));
        if given {
            encode_inputs.push(format!("{value}u"));
        }
    }
    let literal = value_literal(plan, path, source, held)?;
    let mut checks = Vec::new();
    for element in &plan.elements {
        if let Some(holds) = element_holds(element, DECODED, VALUE, 0, held)? {
            checks.push((element.field.id.clone(), holds));
        }
    }
    for group in plan.groups() {
        checks.push((
            gates_label(group),
            group_holds(plan, group, DECODED, VALUE, 0, held)?,
        ));
    }
    if plan.model.variant.is_some() {
        checks.push((
            VARIANT_MEMBER.to_string(),
            variant_holds(plan, DECODED, VALUE, 0, held)?,
        ));
    }
    if let Some(tag) = case_tag(plan, source, tag)? {
        // The decode takes the tag after the flag inputs, and the value made
        // and the value decoded under it both answer it (`tag`): it is what a
        // caller writes for the arm the value holds.
        decode_inputs.push(format!("{tag}u"));
        checks.push((
            CALLER_TAG_LABEL.to_string(),
            format!("{VALUE}.tag.toUInt() == {tag}u && {DECODED}.tag.toUInt() == {tag}u"),
        ));
    }
    Ok(SourceCase {
        literal,
        checks,
        inputs: decode_inputs,
        encode_inputs,
    })
}

/// A Kotlin `Boolean` expression that holds when `decoded` and `value`, two
/// values of the codec that presents `element`, present the same of it, or
/// `None` where there is nothing to compare: a body always there whose codec
/// presents no element. `depth` names the values a body is compared through,
/// so a body held in a body names its own.
///
/// A value class over an array compares the array it holds by identity, and a
/// data class compares such a value class so, so a byte string is compared by
/// its content and a body element by element, down into the codecs it holds.
fn element_holds(
    element: &crate::forge::encode_plan::Element<'_>,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<Option<String>, String> {
    member_holds(
        element.field,
        &element.value,
        element.presence.optional(),
        decoded,
        value,
        depth,
        held,
    )
}

/// [`element_holds`] of `field`, of the abstract type `value`, an element a
/// value presents alone or a member of an arm of a group, `optional` where the
/// property may be `null`.
fn member_holds(
    field: &CodecField,
    value_type: &Value<'_>,
    optional: bool,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<Option<String>, String> {
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
    let property = l.codec_field_id(&field.id);
    let (decoded, value) = (
        format!("{decoded}.{property}"),
        format!("{value}.{property}"),
    );
    Ok(Some(match value_type {
        Value::Bytes(_) if optional => {
            format!("{decoded}?.bytes.contentEquals({value}?.bytes)")
        }
        Value::Bytes(_) => format!("{decoded}.bytes.contentEquals({value}.bytes)"),
        Value::Body { codec } => {
            let (d, v) = (format!("d{depth}"), format!("v{depth}"));
            let inner = held.read(codec, &mut |plan, _, held| {
                Ok(value_holds(plan, &d, &v, depth + 1, held)?.join(" && "))
            })?;
            match (inner.is_empty(), optional) {
                // The body presents nothing: only whether it is there.
                (true, false) => return Ok(None),
                (true, true) => format!("({decoded} == null) == ({value} == null)"),
                (false, false) => {
                    format!("{decoded}.let {{ {d} -> {value}.let {{ {v} -> {inner} }} }}")
                }
                (false, true) => format!(
                    "{decoded}.let {{ {d} -> {value}.let {{ {v} -> \
                     if ({d} == null || {v} == null) {d} == null && {v} == null else {inner} }} }}"
                ),
            }
        }
        // A list by its entries, each compared as a body is, and a list held
        // to a law by the start of its law too (docs/adr/0065 §6 R8).
        Value::List { codec, law, .. } => {
            let (d, v) = (format!("d{depth}"), format!("v{depth}"));
            let starts = |x: &str, y: &str| -> String {
                law.inputs
                    .iter()
                    .filter(|input| matches!(input.automaton, Automaton::Uniform))
                    .map(|input| {
                        let p = start_property(&input.input);
                        format!(" && {x}.{p} == {y}.{p}")
                    })
                    .collect()
            };
            if optional {
                let entries = list_holds(
                    codec,
                    &format!("{d}.entries"),
                    &format!("{v}.entries"),
                    depth,
                    held,
                )?;
                format!(
                    "{decoded}.let {{ {d} -> {value}.let {{ {v} -> \
                     if ({d} == null || {v} == null) {d} == null && {v} == null else \
                     {entries}{} }} }}",
                    starts(&d, &v)
                )
            } else {
                let entries = list_holds(codec, &d, &v, depth, held)?;
                format!(
                    "{decoded}.entries.let {{ {d} -> {value}.entries.let {{ {v} -> {entries} }} }}{}",
                    starts(&decoded, &value)
                )
            }
        }
        _ => format!("{decoded} == {value}"),
    }))
}

/// Kotlin `Boolean` expressions that each hold when `decoded` and `value`, two
/// sources of the codec `plan`, present the same of one element, or hold the
/// same arm of its variant; empty where the codec presents nothing to compare.
/// `depth` names the values a body is compared through.
fn value_holds(
    plan: &EncodePlan<'_>,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<Vec<String>, String> {
    let mut holds = Vec::new();
    for element in &plan.elements {
        holds.extend(element_holds(element, decoded, value, depth, held)?);
    }
    for group in plan.groups() {
        holds.push(group_holds(plan, group, decoded, value, depth, held)?);
    }
    if plan.model.variant.is_some() {
        holds.push(variant_holds(plan, decoded, value, depth, held)?);
    }
    Ok(holds)
}

/// A Kotlin `Boolean` expression that holds when `decoded` and `value`, two
/// sources of the codec `plan`, hold the same arm of `group`, or none, with
/// each member the same, compared as an element is. The `when` is exhaustive
/// over the sealed class the codec's file declares, `null` among its cases
/// where the value may hold no arm.
fn group_holds(
    plan: &EncodePlan<'_>,
    group: &GateGroup,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let path = CodecPath::new(&plan.model.name);
    let property = group_property(group);
    let (x, y) = (format!("gx{depth}"), format!("gy{depth}"));
    // Each member of `arm` the same on both sides, `true` where it holds none.
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
    let members = |arm: &GroupArm, held: &mut dyn HeldCodecs| -> Result<String, String> {
        let mut same = Vec::new();
        for id in &arm.fields {
            // Two lists of one count, each compared by its entries out of the
            // `Aligned` the arm holds them in (docs/adr/0065 §6 R5).
            if group.is_second_in(arm, id) {
                continue;
            }
            if let Some(second) = group.second_of(arm, id) {
                let pair = aligned_property(&l, id, second);
                for (part, list) in [("first", id.as_str()), ("second", second)] {
                    let Value::List { codec, .. } = plan.value(member_field(plan, list)) else {
                        unreachable!("two lists of one count are lists")
                    };
                    same.push(list_holds(
                        codec,
                        &format!("{x}.{pair}.{part}.entries"),
                        &format!("{y}.{pair}.{part}.entries"),
                        depth + 1,
                        held,
                    )?);
                }
                continue;
            }
            // The chain whose memberships are the group's gates is compared
            // by its entries, both sides absent alike where the arm may hold
            // none of it (docs/adr/0065 §6 R6b), and where it holds a bare
            // entry's place, by the place and the others (R6a).
            let field = member_field(plan, id);
            if holds_chain(group, field) && placed_in(group, arm) {
                let Value::List { codec, .. } = plan.value(field) else {
                    unreachable!("the chain whose memberships are gates is a list")
                };
                let property = l.codec_field_id(id);
                same.push(format!("{x}.{property}.place == {y}.{property}.place"));
                same.push(list_holds(
                    codec,
                    &format!("{x}.{property}.rest"),
                    &format!("{y}.{property}.rest"),
                    depth + 1,
                    held,
                )?);
                continue;
            }
            same.extend(member_holds(
                field,
                &plan.value(field),
                holds_chain(group, field) && chain_optional_in(group, field, arm),
                &x,
                &y,
                depth + 1,
                held,
            )?);
        }
        Ok(if same.is_empty() {
            "true".to_string()
        } else {
            same.join(" && ")
        })
    };
    let compared = if group.is_sum() {
        let mut cases = Vec::new();
        for arm in &group.arms {
            let class = path.qualified(&group_arm_class(&path.type_name, group, arm));
            cases.push(format!(
                "is {class} -> {y} is {class} && {}",
                members(arm, &mut *held)?
            ));
        }
        if group.absent.is_some() {
            cases.push(format!("null -> {y} == null"));
        }
        format!("when ({x}) {{ {} }}", cases.join("; "))
    } else {
        let same = members(&group.arms[0], &mut *held)?;
        if group.absent.is_some() {
            format!("if ({x} == null || {y} == null) {x} == null && {y} == null else {same}")
        } else {
            same
        }
    };
    Ok(format!(
        "{decoded}.{property}.let {{ {x} -> {value}.{property}.let {{ {y} -> {compared} }} }}"
    ))
}

/// The field `id` of `plan`, a member of one of its groups.
fn member_field<'m>(plan: &EncodePlan<'m>, id: &str) -> &'m CodecField {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == id)
        .expect("a group's member is a field of the codec")
}

/// The value of `group`, a group of the codec `plan` at `path`, that `json`
/// states, as [`group_choice`] reads it: the class of the arm it states, made
/// of each member as the arm holds it, or `null`.
fn group_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    group: &GateGroup,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let Some(chosen) = group_choice(group, json)? else {
        return Ok("null".to_string());
    };
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
    let class = path.qualified(&group_arm_class(&path.type_name, group, chosen.arm));
    let spell = |id: &str, held: &mut dyn HeldCodecs| -> Result<String, String> {
        let field = member_field(plan, id);
        // The chain whose memberships are the group's gates, in the class of
        // the kinds the arm says of it (docs/adr/0065 §6 R6b).
        if holds_chain(group, field) {
            let chain = ChainInArm {
                group,
                arm: chosen.arm,
                field,
            };
            let json = chosen
                .members
                .get(id)
                .ok_or_else(|| format!("the arm holds `{id}`, and the source states none"))?;
            return chain_literal(plan, path, &chain, json, held)
                .map_err(|e| format!("`{id}`: {e}"));
        }
        let member = chosen
            .members
            .get(id)
            .filter(|json| !json.is_null())
            .ok_or_else(|| format!("the arm holds `{id}`, and the source states none"))?;
        member_literal(field, &plan.value(field), path, member, held)
            .map_err(|e| format!("`{id}`: {e}"))
    };
    let mut args = Vec::new();
    for id in &chosen.arm.fields {
        // Two lists of one count are one member of the arm, an `Aligned` of
        // the two (docs/adr/0065 §6 R5), each stated by its own id; the case
        // states them of one count.
        if group.is_second_in(chosen.arm, id) {
            continue;
        }
        if let Some(second) = group.second_of(chosen.arm, id) {
            let (first_list, second_list) = (spell(id, &mut *held)?, spell(second, &mut *held)?);
            args.push(format!(
                "{} = {ALIGNED}.of({first_list}, {second_list})!!",
                aligned_property(&l, id, second)
            ));
            continue;
        }
        args.push(format!(
            "{} = {}",
            l.codec_field_id(id),
            spell(id, &mut *held)?
        ));
    }
    Ok(if args.is_empty() {
        class
    } else {
        format!("{class}({})", args.join(", "))
    })
}

/// A Kotlin `Boolean` expression that holds when `decoded` and `value`, two
/// sources of the codec `plan`, hold the same arm of its variant with the same
/// body, compared element by element, and for the catch-all the same tag. The
/// `when` is exhaustive over the sealed class the codec's file declares.
fn variant_holds(
    plan: &EncodePlan<'_>,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let (x, y) = (format!("vx{depth}"), format!("vy{depth}"));
    let path = CodecPath::new(&plan.model.name);
    let mut cases = Vec::new();
    for arm in &plan.arms {
        let class = path.qualified(&arm_class(&path.type_name, arm.tag.map(|_| arm.codec)));
        let (bx, by) = (format!("{x}.body"), format!("{y}.body"));
        let mut same = vec![format!("{y} is {class}")];
        if arm.tag.is_none() {
            same.push(format!("{x}.tag.tag == {y}.tag.tag"));
        }
        let body = held.read(arm.codec, &mut |plan, _, held| {
            Ok(value_holds(plan, &bx, &by, depth + 1, held)?.join(" && "))
        })?;
        if !body.is_empty() {
            same.push(body);
        }
        cases.push(format!("is {class} -> {}", same.join(" && ")));
    }
    Ok(format!(
        "{decoded}.{VARIANT_MEMBER}.let {{ {x} -> {value}.{VARIANT_MEMBER}.let {{ {y} -> \
         when ({x}) {{ {} }} }} }}",
        cases.join("; ")
    ))
}

/// The value of the codec `plan` at `path` that `source` states, made by its
/// data class's constructor, each element by its name, and the arm of its
/// variant as `body`.
fn value_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let object = source_members(plan, source)?;
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
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
            "null".to_string()
        } else {
            member_literal(element.field, &element.value, path, json, held)
                .map_err(|e| format!("`{id}`: {e}"))?
        };
        args.push(format!("{} = {value}", l.codec_field_id(id)));
    }
    // A group, keyed by its gates, is the class of the arm it states, made of
    // its members, or `null` (docs/adr/0081 §5).
    for group in plan.groups() {
        let label = gates_label(group);
        let json = object
            .get(&label)
            .ok_or_else(|| format!("the source states no `{label}`"))?;
        let value =
            group_literal(plan, path, group, json, held).map_err(|e| format!("`{label}`: {e}"))?;
        args.push(format!("{} = {value}", group_property(group)));
    }
    if plan.model.variant.is_some() {
        let json = object
            .get(VARIANT_MEMBER)
            .ok_or_else(|| format!("the source states no `{VARIANT_MEMBER}`, the variant's arm"))?;
        let arm = variant_literal(plan, path, json, held)
            .map_err(|e| format!("`{VARIANT_MEMBER}`: {e}"))?;
        args.push(format!("{VARIANT_MEMBER} = {arm}"));
    }
    Ok(format!(
        "{}({})",
        path.qualified(&path.type_name),
        args.join(", ")
    ))
}

/// The arm of the variant a source states, as [`variant_choice`] reads it: the
/// class of the sealed class that holds the arm, made of the arm's codec's
/// data class, and for the catch-all the tag it holds, made by the factory of
/// the class of the tags no arm names.
fn variant_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let (chosen, body) = variant_choice(plan, json)?;
    let value = held.read(chosen.codec, &mut |plan, held_path, held| {
        value_literal(plan, &held_codec(held_path), body, held)
    })?;
    let arm = plan.arms[chosen.index].tag.map(|_| chosen.codec);
    let class = path.qualified(&arm_class(&path.type_name, arm));
    Ok(match chosen.default_tag {
        None => format!("{class}({value})"),
        Some(tag) => {
            let ty = crate::forge::codec_totality::tag_type(plan.model);
            format!(
                "{class}(tag = {}.of({})!!, body = {value})",
                path.qualified(&unknown_tag_class(&path.type_name)),
                integer_literal(i128::from(tag), &ty)?
            )
        }
    })
}

/// One value of the abstract type `value` of the element `field` of the codec
/// at `path`, as a Kotlin expression the data class's constructor takes: a
/// number as a literal of the element's type, a byte string as a value of the
/// class of its bound, and a value of an enum as the variant of the enum's
/// class.
fn member_literal(
    field: &CodecField,
    value: &Value<'_>,
    path: &CodecPath,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    match value {
        Value::Scalar(ty) if ty.int_bit_width().is_some() => {
            integer_literal(integer_value(json)?, ty)
        }
        // A byte as the signed number of its bits, which `byteArrayOf` takes.
        Value::Bytes(_) => {
            let bytes: Vec<String> = byte_values(json)?
                .iter()
                .map(|b| (*b as i8).to_string())
                .collect();
            Ok(format!(
                "{}.of(byteArrayOf({}))!!",
                path.qualified(&refinement_class(
                    &path.type_name,
                    &field.id,
                    &field.sce_type
                )),
                bytes.join(", ")
            ))
        }
        Value::Scalar(SceType::Enum(enum_ref)) => {
            let (held_enum, variant) = enum_variant(held, &enum_ref.alias, json)?;
            let ty = kotlin_qualified(
                &held_enum.name,
                &filters::to_pascal_case(held_enum.name.clone()),
            );
            Ok(crate::forge::enum_naming::variant_ref(
                crate::generator::Language::Kotlin,
                &ty,
                &held_enum.name,
                variant,
            ))
        }
        Value::Scalar(_) => Err("a float or a bool is not spelled as a Kotlin literal yet".into()),
        Value::Text(_) => {
            let text = json
                .as_str()
                .ok_or_else(|| "text is a JSON string".to_string())?;
            Ok(format!(
                "{}.of({})!!",
                path.qualified(&refinement_class(
                    &path.type_name,
                    &field.id,
                    &field.sce_type
                )),
                string_literal(text)
            ))
        }
        // The held codec's data class, reached by its document's package, as
        // the printed codec's encode reaches it.
        Value::Body { codec } => held.read(codec, &mut |plan, held_path, held| {
            value_literal(plan, &held_codec(held_path), json, held)
        }),
        // Each entry its codec's data class, reached as a body's is, in the
        // value class of the list's bound, made by its factory.
        Value::List { codec, law, .. } if law.is_none() => Ok(format!(
            "{}.of(listOf({}))!!",
            path.qualified(&list_class(&path.type_name, &field.id)),
            entry_literals(codec, json, held)?.join(", ")
        )),
        // A list held to the law its entries say their inputs by
        // (docs/adr/0065 §6 R8), `{"start": {input: 0|1}, "entries": [..]}`:
        // the class of the law made of the list and each start the value
        // gives, by its factory, which runs the law over the entries.
        Value::List { codec, law, .. } => {
            let (start, entries) = lawful_parts(json)?;
            let starts: String = law
                .inputs
                .iter()
                .zip(law_start_bits(law, start)?)
                .filter_map(|(input, bit)| {
                    bit.map(|bit| format!(", {} = {bit}", start_property(&input.input)))
                })
                .collect();
            Ok(format!(
                "{}.of({}.of(listOf({}))!!{starts})!!",
                path.qualified(&lawful_class(&path.type_name, &field.id)),
                path.qualified(&list_class(&path.type_name, &field.id)),
                entry_literals(codec, entries, held)?.join(", ")
            ))
        }
    }
}

/// Each entry of the list or chain `json` states, of the codec imported as
/// `codec`, as that codec's data class, reached by its document's package.
fn entry_literals(
    codec: &str,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<Vec<String>, String> {
    let mut items = Vec::new();
    for entry in entry_rows(json)? {
        items.push(held.read(codec, &mut |plan, held_path, held| {
            value_literal(plan, &held_codec(held_path), entry, held)
        })?);
    }
    Ok(items)
}

/// The chain an arm of a group of its kinds holds (docs/adr/0081 §5): its
/// entries whole, in the class of the kinds the arm says of it, made by that
/// class's factory, which holds them to those kinds; `null` where the arm may
/// hold none of it.
fn chain_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    chain: &ChainInArm<'_>,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let Value::List { codec, .. } = plan.value(chain.field) else {
        return Err(format!("`{}` is no chain", chain.field.id));
    };
    // A bare entry's arm holds the others and the named entry's place,
    // `{"rest": [..], "place": n}` (docs/adr/0065 §6 R6a), made by its class's
    // factory, which holds the place to the first of its kind.
    if chain.placed() {
        let (rest, place) = placed_parts(json)?;
        let (holds, _) = chain.group.kinds_in(chain.arm);
        let key = holds
            .first()
            .ok_or_else(|| "a bare entry's arm holds its kind".to_string())?;
        return Ok(format!(
            "{}.of(listOf({}), {place})!!",
            path.qualified(&placed_class(&path.type_name, &chain.field.id, *key)),
            entry_literals(codec, rest, held)?.join(", ")
        ));
    }
    if json.is_null() {
        return if chain.optional() {
            Ok("null".to_string())
        } else {
            Err("the arm holds the chain always".to_string())
        };
    }
    let (holds, lacks) = chain.group.kinds_in(chain.arm);
    Ok(format!(
        "{}.of(listOf({}))!!",
        path.qualified(&chain_class(
            &path.type_name,
            &chain.field.id,
            &holds,
            &lacks
        )),
        entry_literals(codec, json, held)?.join(", ")
    ))
}

/// A Kotlin `Boolean` expression that holds when the lists `decoded` and
/// `value`, of sources of the codec imported as `codec`, hold as many entries
/// and each the same as the other's at its place, compared element by element
/// through the names `depth` gives them.
fn list_holds(
    codec: &str,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let (i, d, v) = (
        format!("i{depth}"),
        format!("d{}", depth + 1),
        format!("v{}", depth + 1),
    );
    let inner = held.read(codec, &mut |plan, _, held| {
        Ok(value_holds(plan, &d, &v, depth + 2, held)?.join(" && "))
    })?;
    let sizes = format!("{decoded}.size == {value}.size");
    Ok(if inner.is_empty() {
        sizes
    } else {
        format!(
            "{sizes} && {decoded}.indices.all {{ {i} -> {decoded}[{i}].let {{ {d} -> \
             {value}[{i}].let {{ {v} -> {inner} }} }} }}"
        )
    })
}

/// How the value of a printed CBOR map or content-line component holds one
/// element a case states, which its literal and its comparison are spelled by.
pub(super) enum Holds<'t> {
    /// A boolean.
    Bool,
    /// An integer of the type.
    Int(&'t SceType),
    /// A text held in a bare `String`, as a content line holds it.
    Text,
    /// A text or a byte string in the value class of its bound, named by the
    /// codec's file, made by its factory.
    Classed(String),
}

/// The case of a CBOR map or of a content-line component made of `elements`
/// (`source_notation`), each beside how the value holds it, a value of the
/// codec at `path`: its data class made by its constructor, each element by
/// its name, `null` where the case states none and a `MutableList` of the
/// values of a repeated property. A byte string is compared by its content, as
/// a value class over an array compares the array by identity, and any other
/// element whole, a list value by value. A map and a component take no flag
/// input and no caller's tag.
pub(super) fn map_or_line_case(
    path: &CodecPath,
    elements: Vec<(super::super::source_notation::StatedElement, Holds<'_>)>,
) -> Result<SourceCase, String> {
    use super::super::source_notation::{Scalar, Stated};
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
    let mut args = Vec::new();
    let mut checks = Vec::new();
    for (element, holds) in elements {
        let property = l.codec_field_id(&element.id);
        let (decoded, value) = (
            format!("{DECODED}.{property}"),
            format!("{VALUE}.{property}"),
        );
        let literal = |scalar: &Scalar| -> Result<String, String> {
            match (scalar, &holds) {
                (Scalar::Bool(b), Holds::Bool) => Ok(b.to_string()),
                (Scalar::Int(n), Holds::Int(ty)) => integer_literal(*n, ty),
                (Scalar::Text(text), Holds::Text) => Ok(string_literal(text)),
                (Scalar::Text(text), Holds::Classed(class)) => Ok(format!(
                    "{}.of({})!!",
                    path.qualified(class),
                    string_literal(text)
                )),
                // A byte as the signed number of its bits, which `byteArrayOf`
                // takes.
                (Scalar::Bytes(bytes), Holds::Classed(class)) => Ok(format!(
                    "{}.of(byteArrayOf({}))!!",
                    path.qualified(class),
                    bytes
                        .iter()
                        .map(|b| (*b as i8).to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                _ => Err(format!(
                    "`{}`: the case states a value the element does not hold",
                    element.id
                )),
            }
        };
        let (argument, check) = match &element.stated {
            Stated::Absent => ("null".to_string(), format!("{decoded} == {value}")),
            Stated::One(scalar) => {
                let check = match (scalar, element.optional) {
                    (Scalar::Bytes(_), true) => {
                        format!("{decoded}?.bytes.contentEquals({value}?.bytes)")
                    }
                    (Scalar::Bytes(_), false) => {
                        format!("{decoded}.bytes.contentEquals({value}.bytes)")
                    }
                    _ => format!("{decoded} == {value}"),
                };
                (literal(scalar)?, check)
            }
            // The values of a repeated property, in order, in the list the
            // value holds them in, compared as a list is, value by value.
            Stated::List(items) => (
                format!(
                    "mutableListOf({})",
                    items
                        .iter()
                        .map(literal)
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ")
                ),
                format!("{decoded} == {value}"),
            ),
        };
        args.push(format!("{property} = {argument}"));
        checks.push((element.id.clone(), check));
    }
    Ok(SourceCase {
        literal: format!("{}({})", path.qualified(&path.type_name), args.join(", ")),
        checks,
        inputs: Vec::new(),
        encode_inputs: Vec::new(),
    })
}

/// `text` as a Kotlin string literal of the same text: a quote, a backslash and
/// a `$`, which would start a template, escaped, and every control character
/// written as its UTF-16 unit, as Kotlin has no escape for most of them.
fn string_literal(text: &str) -> String {
    let mut literal = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '$' => literal.push_str("\\$"),
            c if c.is_control() => literal.push_str(&format!("\\u{:04X}", c as u32)),
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

/// `n` as a Kotlin literal of the integer type `ty`, which the constructor's
/// parameter gives the literal: an unsigned literal for an unsigned type, and
/// a conversion of a signed one for a type no literal of its own spells.
fn integer_literal(n: i128, ty: &SceType) -> Result<String, String> {
    let fits = |least: i128, most: i128| {
        if (least..=most).contains(&n) {
            Ok(())
        } else {
            Err(format!("{n} is no value of a `{}`", ty.as_attr()))
        }
    };
    Ok(match ty {
        SceType::Uint8 => {
            fits(0, u8::MAX.into())?;
            format!("{n}u")
        }
        SceType::Uint16 => {
            fits(0, u16::MAX.into())?;
            format!("{n}u")
        }
        SceType::Uint32 => {
            fits(0, u32::MAX.into())?;
            format!("{n}u")
        }
        SceType::Uint64 => {
            fits(0, u64::MAX.into())?;
            format!("{n}uL")
        }
        SceType::Int8 => {
            fits(i8::MIN.into(), i8::MAX.into())?;
            format!("({n}).toByte()")
        }
        SceType::Int16 => {
            fits(i16::MIN.into(), i16::MAX.into())?;
            format!("({n}).toShort()")
        }
        SceType::Int32 => {
            fits(i32::MIN.into(), i32::MAX.into())?;
            n.to_string()
        }
        SceType::Int64 => {
            fits(i64::MIN.into(), i64::MAX.into())?;
            format!("{n}L")
        }
        _ => return Err(format!("a `{}` is no integer", ty.as_attr())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text reaches the harness as a Kotlin literal of the same text: what
    /// would end the literal, escape, or start a template is escaped, and a
    /// control character is written as its unit.
    ///
    /// Turns red where `string_literal` stops escaping a `$`, a quote or a
    /// backslash, or writes a control character raw.
    #[test]
    fn a_text_is_spelled_as_a_kotlin_literal_of_the_same_text() {
        assert_eq!(string_literal("abc"), "\"abc\"");
        assert_eq!(string_literal("a\"b\\c$d"), "\"a\\\"b\\\\c\\$d\"");
        assert_eq!(string_literal("\u{0c}\n"), "\"\\u000C\\u000A\"");
        assert_eq!(string_literal("h\u{e9}llo"), "\"h\u{e9}llo\"");
    }

    /// A number is a literal of the element's type, which the parameter gives
    /// it, and one its type does not hold is refused by name.
    ///
    /// Turns red where `integer_literal` spells a 64-bit unsigned number without
    /// its `uL` suffix, a signed byte without its conversion, or lets a number
    /// past its type through.
    #[test]
    fn a_number_is_a_literal_of_its_elements_type() {
        assert_eq!(integer_literal(255, &SceType::Uint8).as_deref(), Ok("255u"));
        assert_eq!(
            integer_literal(18_446_744_073_709_551_615, &SceType::Uint64).as_deref(),
            Ok("18446744073709551615uL")
        );
        assert_eq!(
            integer_literal(-1, &SceType::Int8).as_deref(),
            Ok("(-1).toByte()")
        );
        assert_eq!(integer_literal(-7, &SceType::Int64).as_deref(), Ok("-7L"));
        assert!(integer_literal(256, &SceType::Uint8).is_err());
        assert!(integer_literal(-1, &SceType::Uint32).is_err());
    }
}
