// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the C++ conformance harness (docs/adr/0081 §5): a value a case
//! states as the codec's interface presents it, made by the printed value's
//! constructor, and for each element an expression that holds when a decode of
//! the case's bytes presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! ([`source_members`] and the readers beside it), so one case states one value
//! in every language. It names no bound, law or kind of its
//! own: an element that holds one is made by `make` of the type the printed
//! class's accessor presents it as, read with `decltype`, so the literal holds
//! what the printer declares and has nothing to agree with. A byte string and a
//! list of entries are made over a named `std::vector`, since the views `make`
//! takes do not own what they view; a body, an arm and a group are made by
//! value, as the value holds them. Each `make` is read with `value()`, which
//! throws where the case is outside a bound or a law and stops the harness, as
//! the Rust case's `expect` does. A shape no case spells is refused by name, as
//! the Rust case refuses it.

use super::super::rust_source::{
    aligned_name, all_of, byte_values, case_inputs, chain_optional_in, entry_rows, enum_variant,
    gates_label, group_accessor, group_arm_name, group_choice, group_type, holds_chain,
    integer_value, law_start_bits, lawful_parts, placed_parts, source_members, spelled,
    variant_choice, ChainInArm, ChosenGroupArm, CodecPath as RustCodecPath, HeldCodecs,
    VARIANT_MEMBER,
};
use super::super::*;
use super::{declared_parts, Part};
use crate::forge::codec_roles::GateGroup;
use crate::forge::encode_plan::{Automaton, Element, EncodePlan, Law, Value};

/// Where a codec's C++ types are: the namespace the generator writes it in and
/// the class of its value, both named after the document's name in Pascal case
/// (`::SCE::Generated::X::X`, as `forge_import_identity` names an import).
#[derive(Clone, Debug)]
pub(crate) struct CodecPath {
    /// The document's name in Pascal case.
    pub type_name: String,
}

impl CodecPath {
    /// The codec whose types are named after `type_name`.
    pub(crate) fn new(type_name: String) -> Self {
        CodecPath { type_name }
    }

    /// The codec a held codec's Rust path names: its types are named after the
    /// same document's name in both languages.
    fn of_held(path: &RustCodecPath) -> Self {
        CodecPath::new(path.type_name.clone())
    }

    /// The namespace that holds its class, its `Wire` and its traits.
    fn namespace(&self) -> String {
        format!("::SCE::Generated::{}", self.type_name)
    }

    /// The class of its value.
    fn class(&self) -> String {
        format!("{}::{}", self.namespace(), self.type_name)
    }
}

/// The name the harness gives the value a case makes.
const VALUE: &str = "value";

/// The value the harness's decode of the case's bytes answers, read where it is
/// there.
const DECODED: &str = "(*decoded)";

/// A byte case of the C++ harness (docs/adr/0081 §5).
pub(crate) struct SourceCase {
    /// The declarations the literal reads, in order: each byte string and each
    /// array of entries a view of the literal views.
    pub bindings: Vec<String>,
    /// The value, made by its class's constructor.
    pub literal: String,
    /// Each element by its id, each group by its gates and the variant by
    /// `body`, with a `bool` expression over `(*decoded)` and `value` that
    /// holds when the two present the same; and each flag input the value
    /// says, with one that holds when it says the input the case is handed.
    pub checks: Vec<(String, String)>,
    /// The flag inputs the decode is handed, in the order the codec declares
    /// them, as arguments of its `decode`.
    pub inputs: Vec<String>,
    /// The flag inputs the encode is handed, those the value does not say
    /// (docs/adr/0065 §6 R4a), in the order the codec declares them, as
    /// arguments of `Wire::encode_source` after the sink.
    pub encode_inputs: Vec<String>,
}

/// The byte case of `source`, a value of the codec `plan` at `path`, its parent
/// handing it `inputs` (`{"name": 0 | 1}`, or `null` for a codec that takes
/// none). A shape the literal does not spell yet is refused by name, and so is
/// a source that states an element the value does not present or leaves out
/// one it does, and inputs that are not the codec's.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    let mut bindings = Vec::new();
    let literal = value_literal(plan, path, source, held, &mut bindings)?;
    let Handed { args, given, said } = handed_inputs(plan, path, inputs)?;
    let mut fresh = 0;
    let mut checks: Vec<(String, String)> = plan
        .elements
        .iter()
        .map(|element| {
            let holds = element_equal(element, DECODED, VALUE, held, &mut fresh)?;
            Ok((element.field.id.clone(), holds))
        })
        .collect::<Result<_, String>>()?;
    for group in plan.groups() {
        checks.push((
            gates_label(group),
            group_equal(plan, group, DECODED, VALUE, held, &mut fresh)?,
        ));
    }
    if plan.model.variant.is_some() {
        checks.push((
            VARIANT_MEMBER.to_string(),
            variant_equal(plan, DECODED, VALUE, held, &mut fresh)?,
        ));
    }
    checks.extend(said);
    Ok(SourceCase {
        bindings,
        literal,
        checks,
        inputs: args,
        encode_inputs: given,
    })
}

/// The flag inputs a byte case hands its codec.
struct Handed {
    /// `decode`'s arguments, in the order the codec declares the inputs.
    args: Vec<String>,
    /// `encode_source`'s arguments: each input the value does not say, in the
    /// order the codec declares them.
    given: Vec<String>,
    /// For each input the value says, its label and an expression that holds
    /// when `value` says the one the case hands.
    said: Vec<(String, String)>,
}

/// The flag inputs a case hands the codec at `path`, as [`case_inputs`] reads
/// them: each is handed to the decode, one the value does not say to the
/// encode too, and the value is held to saying each other one through the
/// input's own function (`input_<name>`, docs/adr/0065 §6 R4a), which answers
/// no value where the value says nothing of it.
fn handed_inputs(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    inputs: &serde_json::Value,
) -> Result<Handed, String> {
    let mut args = Vec::new();
    let mut given = Vec::new();
    let mut said = Vec::new();
    for (input, value, to_encode) in case_inputs(plan, inputs)? {
        args.push(value.to_string());
        if to_encode {
            given.push(value.to_string());
            continue;
        }
        let says = format!(
            "{}::input_{}({VALUE})",
            path.namespace(),
            filters::to_snake_case(input.name.clone())
        );
        let holds = if plan.roles.says_input_partly(&input.name) {
            format!(
                "[&](const auto& said) {{ return !said.has_value() || *said == {value}; }}({says})"
            )
        } else {
            format!("{says} == {value}")
        };
        said.push((format!("flag input {}", input.name), holds));
    }
    Ok(Handed { args, given, said })
}

/// The name the printed value reads the element `id` by: its accessor, as the
/// printer names it (`codec_field_id`).
fn accessor(id: &str) -> String {
    LangCtx::new(crate::generator::Language::Cpp, &[]).codec_field_id(id)
}

/// The type the accessor `accessor` of the class `owner` presents its element
/// as, read off the printed class; where `optional`, the type the
/// `std::optional` it answers holds.
fn presented_type(owner: &str, accessor: &str, optional: bool) -> String {
    let read = format!("decltype(std::declval<const {owner}&>().{accessor}())");
    if optional {
        format!("{read}::value_type")
    } else {
        read
    }
}

/// `make` of the bounded type `ty` over `args`, which the case holds within its
/// bounds and its law: `value()` throws where it is not, which stops the
/// harness.
fn made(ty: &str, args: &str) -> String {
    format!("{ty}::make({args}).value()")
}

/// The value of the codec `plan` at `path` that `source` states, made by its
/// class's constructor: each element and group in the order their fields are
/// declared, then the arm of its variant.
fn value_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let object = source_members(plan, source)?;
    let class = path.class();
    let mut args = Vec::new();
    for part in declared_parts(plan) {
        let arg = match part {
            Part::Element(index) => {
                let element = &plan.elements[index];
                let id = &element.field.id;
                let value = object
                    .get(id)
                    .ok_or_else(|| format!("the source states no `{id}`"))?;
                // A gate decides it, or it is a number the value states while
                // its content is absent and says nothing of while some is
                // there (docs/adr/0058 §1): either is a `std::optional`.
                let optional = element.presence.optional();
                if value.is_null() {
                    if !optional {
                        return Err(format!(
                            "`{id}` is always there, and the source states none"
                        ));
                    }
                    "std::nullopt".to_string()
                } else {
                    let ty = presented_type(&class, &accessor(id), optional);
                    member_literal(&element.value, &ty, value, held, bindings)
                        .map_err(|e| format!("`{id}`: {e}"))?
                }
            }
            Part::Group(index) => {
                let group = &plan.groups()[index];
                let label = gates_label(group);
                let value = object
                    .get(&label)
                    .ok_or_else(|| format!("the source states no `{label}`"))?;
                group_literal(plan, path, group, value, held, bindings)
                    .map_err(|e| format!("`{label}`: {e}"))?
            }
        };
        args.push(arg);
    }
    if plan.model.variant.is_some() {
        let value = object
            .get(VARIANT_MEMBER)
            .ok_or_else(|| format!("the source states no `{VARIANT_MEMBER}`, the variant's arm"))?;
        args.push(
            variant_literal(plan, path, value, held, bindings)
                .map_err(|e| format!("`{VARIANT_MEMBER}`: {e}"))?,
        );
    }
    Ok(format!("{class}{{{}}}", args.join(", ")))
}

/// One value of the abstract type `value`, as a C++ expression of `ty`, the
/// type the printed class presents it as: a byte string, a text or a list made
/// by that type's `make`, within its bounds and its law; a body made by its own
/// class's constructor; a number or a flag as a literal.
fn member_literal(
    value: &Value<'_>,
    ty: &str,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    match value {
        Value::Body { codec } => held.read(codec, &mut |plan, path, held| {
            value_literal(plan, &CodecPath::of_held(path), json, held, bindings)
        }),
        Value::List { codec, law, .. } if !law.is_none() => {
            // The runtime's `make` runs the law over the entries.
            let (start, entries) = lawful_parts(json)?;
            let start = law_start_literal(law, ty, start)?;
            let array = entries_binding(codec, entries, held, bindings)?;
            Ok(made(
                ty,
                &format!("{start}, {}", made(&format!("{ty}::list_type"), &array)),
            ))
        }
        Value::List { codec, .. } => {
            let array = entries_binding(codec, json, held, bindings)?;
            Ok(made(ty, &array))
        }
        Value::Scalar(SceType::Bool) => json
            .as_bool()
            .map(|b| b.to_string())
            .ok_or_else(|| "a boolean is `true` or `false`".to_string()),
        Value::Scalar(
            SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Uint64
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32
            | SceType::Int64,
        ) => integer_value(json).map(integer_literal),
        Value::Bytes(_) => {
            let bytes: Vec<String> = byte_values(json)?.iter().map(u8::to_string).collect();
            let name = bind(bindings, "std::uint8_t", &bytes);
            Ok(made(ty, &name))
        }
        Value::Text(_) => {
            let text = json
                .as_str()
                .ok_or_else(|| "text is a JSON string".to_string())?;
            // The length is stated, so a text that holds a zero byte is whole.
            Ok(made(
                ty,
                &format!(
                    "std::string_view(\"{}\", {})",
                    filters::escape_cpp(text.to_string()),
                    text.len()
                ),
            ))
        }
        // The variant the value holds, by the type the C++ tree declares the
        // enum as.
        Value::Scalar(SceType::Enum(enum_ref)) => {
            let (held_enum, variant) = enum_variant(held, &enum_ref.alias, json)?;
            let ty = crate::forge::generator::enum_qualified_type(
                &held_enum.name,
                crate::generator::Language::Cpp,
            );
            Ok(crate::forge::enum_naming::variant_ref(
                crate::generator::Language::Cpp,
                &format!("::{ty}"),
                &held_enum.name,
                variant,
            ))
        }
        Value::Scalar(_) => Err("a float is not spelled as a literal yet".to_string()),
    }
}

/// An integer as a C++ literal of the type it initializes: unsigned where it is
/// not negative, so a value past `long long` is one literal, and the least
/// `std::int64_t` as the expression of it, which no literal spells.
fn integer_literal(n: i128) -> String {
    if n >= 0 {
        format!("{n}u")
    } else if n == i128::from(i64::MIN) {
        "(-9223372036854775807 - 1)".to_string()
    } else {
        n.to_string()
    }
}

/// A name for the next binding of a byte case: a `const std::vector` of
/// `element` holding `items`, which a view the literal makes reads.
fn bind(bindings: &mut Vec<String>, element: &str, items: &[String]) -> String {
    let name = format!("held_{}", bindings.len());
    bindings.push(format!(
        "const std::vector<{element}> {name}{{{}}};",
        items.join(", ")
    ));
    name
}

/// The binding of the entries `json` states, each a value of the codec imported
/// as `codec`: a list's, or a chain's an arm of a group of its kinds holds.
fn entries_binding(
    codec: &str,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let rows = entry_rows(json)?;
    let mut entries = Vec::new();
    let mut class = String::new();
    for (i, row) in rows.iter().enumerate() {
        entries.push(
            held.read(codec, &mut |plan, path, held| {
                let path = CodecPath::of_held(path);
                class = path.class();
                value_literal(plan, &path, row, held, bindings)
            })
            .map_err(|e| format!("entry {i}: {e}"))?,
        );
    }
    if rows.is_empty() {
        held.read(codec, &mut |_, path, _| {
            class = CodecPath::of_held(path).class();
            Ok(String::new())
        })?;
    }
    Ok(bind(bindings, &class, &entries))
}

/// The start of a list's law as the literal of `lawful`'s `start_type`
/// (docs/adr/0065 §6 R8), from the bits [`law_start_bits`] reads:
/// the flag the value gives a Uniform input, and `FixedStart` for an input the
/// entry before decides; each one's, in the law's order, where it runs several.
fn law_start_literal(law: &Law, lawful: &str, json: &serde_json::Value) -> Result<String, String> {
    let parts: Vec<String> = law_start_bits(law, json)?
        .into_iter()
        .map(|bit| {
            bit.map_or_else(
                || "::SCE::Forge::FixedStart{}".to_string(),
                |b| b.to_string(),
            )
        })
        .collect();
    Ok(match parts.as_slice() {
        [one] => one.clone(),
        many => format!("{lawful}::start_type{{{}}}", many.join(", ")),
    })
}

/// The chain as an arm of a group of its kinds holds it (ADR 0081 §5), made by
/// `make` of `ty`, the type the arm presents it as: where the arm holds a bare
/// entry, the others and the entry's place, as a `Placed`; otherwise the
/// entries whole as a `Chain`, `std::nullopt` where the arm may hold none. The
/// runtime's `make` checks the kinds the arm sets and clears.
fn chain_literal(
    plan: &EncodePlan<'_>,
    chain: &ChainInArm<'_>,
    ty: &str,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let Value::List { codec, .. } = plan.value(chain.field) else {
        return Err(format!("`{}` is no chain", chain.field.id));
    };
    if chain.placed() {
        let (rest, place) = placed_parts(json)?;
        let array = entries_binding(codec, rest, held, bindings)?;
        return Ok(made(ty, &format!("{array}, {place}u")));
    }
    if json.is_null() {
        return if chain.optional() {
            Ok("std::nullopt".to_string())
        } else {
            Err("the arm holds the chain always".to_string())
        };
    }
    let array = entries_binding(codec, json, held, bindings)?;
    Ok(made(ty, &array))
}

/// The value of the gate group `group` a source states, as [`group_choice`]
/// reads it: the class of its arm, each member as the arm holds it, in the
/// variant of the arms for a group of several, and `std::nullopt` for a group a
/// value may hold none of. Two lists of one count are one `Aligned` member
/// (docs/adr/0065 §6 R5).
fn group_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    group: &GateGroup,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let Some(ChosenGroupArm {
        index,
        arm,
        members: object,
    }) = group_choice(group, json)?
    else {
        return Ok("std::nullopt".to_string());
    };
    let class = format!(
        "{}::{}",
        path.namespace(),
        group_type(&path.type_name, group)
    );
    let arm_class = if group.is_sum() {
        format!("{class}{}", group_arm_name(group, arm))
    } else {
        class.clone()
    };
    let field_of = |id: &str| {
        plan.model
            .fields
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| format!("the codec has no field `{id}`"))
    };
    let mut spell = |id: &str, ty: &str, bindings: &mut Vec<String>| -> Result<String, String> {
        let field = field_of(id)?;
        let value = object
            .get(id)
            .ok_or_else(|| format!("the arm states no `{id}`"))?;
        if holds_chain(group, field) {
            let chain = ChainInArm { group, arm, field };
            chain_literal(plan, &chain, ty, value, held, bindings)
        } else {
            member_literal(&plan.value(field), ty, value, held, bindings)
        }
        .map_err(|e| format!("`{id}`: {e}"))
    };
    let mut members = Vec::new();
    for id in &arm.fields {
        // Two lists of one count are one member of the arm (docs/adr/0065 §6
        // R5): the first holds the second, each stated by its own id.
        if group.is_second_in(arm, id) {
            continue;
        }
        match group.second_of(arm, id) {
            Some(second) => {
                let pair = presented_type(
                    &arm_class,
                    &aligned_name(&accessor(id), &accessor(second)),
                    false,
                );
                let list = |end: &str| {
                    format!("std::decay_t<decltype(std::declval<const {pair}&>().{end}())>")
                };
                let (first_list, second_list) = (
                    spell(id, &list("first"), bindings)?,
                    spell(second, &list("second"), bindings)?,
                );
                members.push(made(&pair, &format!("{first_list}, {second_list}")));
            }
            None => {
                let field = field_of(id)?;
                let optional = holds_chain(group, field) && chain_optional_in(group, field, arm);
                let ty = presented_type(&arm_class, &accessor(id), optional);
                members.push(spell(id, &ty, bindings)?);
            }
        }
    }
    let arm_value = format!("{arm_class}{{{}}}", members.join(", "));
    Ok(if group.is_sum() {
        format!("{class}{{std::in_place_index<{index}>, {arm_value}}}")
    } else {
        arm_value
    })
}

/// The arm of the variant a source states, as [`variant_choice`] reads it: the
/// variant of the arms holding the arm's value, at the arm's place, and for the
/// default arm the catch-all holding its tag and its body.
fn variant_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let (arm, body) = variant_choice(plan, json)?;
    let value = held.read(arm.codec, &mut |plan, path, held| {
        value_literal(plan, &CodecPath::of_held(path), body, held, bindings)
    })?;
    let (namespace, type_name) = (path.namespace(), &path.type_name);
    let alternative = match arm.default_tag {
        None => value,
        Some(tag) => format!("{namespace}::{type_name}Default{{{tag}u, {value}}}"),
    };
    Ok(format!(
        "{namespace}::{type_name}Variant{{std::in_place_index<{}>, {alternative}}}",
        arm.index
    ))
}

/// A `bool` expression that holds when the `std::optional` values `left` and
/// `right` are both absent, or both present with `same` holding of them, which
/// reads them as `(*x)` and `(*y)`.
fn both_absent_or(x: &str, y: &str, same: &str, left: &str, right: &str) -> String {
    format!(
        "[&](const auto& {x}, const auto& {y}) {{ if ({x}.has_value() != {y}.has_value()) {{ \
         return false; }} return !{x}.has_value() || ({same}); }}({left}, {right})"
    )
}

/// A `bool` expression that holds when `a` and `b`, two values of the codec
/// `plan`, present the same `element`: its value, or for a body each element of
/// it and for a list each entry in order, or both none of it.
fn element_equal(
    element: &Element<'_>,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    let name = accessor(&element.field.id);
    let (left, right) = (format!("{a}.{name}()"), format!("{b}.{name}()"));
    if !matches!(element.value, Value::Body { .. } | Value::List { .. }) {
        return Ok(format!("{left} == {right}"));
    }
    if !element.presence.optional() {
        return value_same(&element.value, &left, &right, held, fresh);
    }
    *fresh += 1;
    let (x, y) = (format!("x{fresh}"), format!("y{fresh}"));
    let same = value_same(
        &element.value,
        &format!("(*{x})"),
        &format!("(*{y})"),
        held,
        fresh,
    )?;
    Ok(both_absent_or(&x, &y, &same, &left, &right))
}

/// A `bool` expression that holds when `x` and `y`, two values of the abstract
/// type `value` as the printed classes present them, are the same: a body's
/// elements each, a list's entries each in order and a lawful list's start,
/// any other value whole.
fn value_same(
    value: &Value<'_>,
    x: &str,
    y: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    match value {
        Value::Body { codec } => held.read(codec, &mut |plan, _, held| {
            body_equal(plan, x, y, held, fresh)
        }),
        Value::List { codec, law, .. } if !law.is_none() => {
            let entries = entries_same(codec, x, y, held, fresh)?;
            // A law whose every input the entry before decides starts from a
            // fixed start: there is none to compare.
            let started = law
                .inputs
                .iter()
                .any(|i| matches!(i.automaton, Automaton::Uniform));
            Ok(if started {
                all_of(&[format!("{x}.start() == {y}.start()"), entries])
            } else {
                entries
            })
        }
        Value::List { codec, .. } => entries_same(codec, x, y, held, fresh),
        _ => Ok(format!("{x} == {y}")),
    }
}

/// A `bool` expression that holds when `x` and `y`, two ranges of entries of
/// the codec imported as `codec`, hold the same entries in order.
fn entries_same(
    codec: &str,
    x: &str,
    y: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    *fresh += 1;
    let n = *fresh;
    let (xs, ys, i, j) = (
        format!("xs{n}"),
        format!("ys{n}"),
        format!("i{n}"),
        format!("j{n}"),
    );
    let entry = {
        let (ea, eb) = (format!("(*{i})"), format!("(*{j})"));
        held.read(codec, &mut |plan, _, held| {
            body_equal(plan, &ea, &eb, held, fresh)
        })?
    };
    Ok(format!(
        "[&](const auto& {xs}, const auto& {ys}) {{ auto {i} = {xs}.begin(); auto {j} = \
         {ys}.begin(); for (; {i} != {xs}.end() && {j} != {ys}.end(); ++{i}, ++{j}) {{ if \
         (!({entry})) {{ return false; }} }} return {i} == {xs}.end() && {j} == {ys}.end(); \
         }}({x}, {y})"
    ))
}

/// A `bool` expression that holds when `a` and `b`, two values of the codec
/// `plan`, present the same elements.
fn body_equal(
    plan: &EncodePlan<'_>,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    spelled(plan)?;
    let mut checks = plan
        .elements
        .iter()
        .map(|element| element_equal(element, a, b, held, fresh))
        .collect::<Result<Vec<_>, _>>()?;
    for group in plan.groups() {
        checks.push(group_equal(plan, group, a, b, held, fresh)?);
    }
    if plan.model.variant.is_some() {
        checks.push(variant_equal(plan, a, b, held, fresh)?);
    }
    Ok(all_of(&checks))
}

/// A `bool` expression that holds when `x` and `y`, the chain as two values
/// present it in its arm, are the same: the place of a bare entry and the
/// others, or the entries whole, or both absent.
fn chain_same(
    plan: &EncodePlan<'_>,
    chain: &ChainInArm<'_>,
    x: &str,
    y: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    let value = plan.value(chain.field);
    if chain.placed() {
        let rest = value_same(
            &value,
            &format!("{x}.rest()"),
            &format!("{y}.rest()"),
            held,
            fresh,
        )?;
        return Ok(all_of(&[format!("{x}.place() == {y}.place()"), rest]));
    }
    if !chain.optional() {
        return value_same(&value, x, y, held, fresh);
    }
    *fresh += 1;
    let (ox, oy) = (format!("x{fresh}"), format!("y{fresh}"));
    let same = value_same(
        &value,
        &format!("(*{ox})"),
        &format!("(*{oy})"),
        held,
        fresh,
    )?;
    Ok(both_absent_or(&ox, &oy, &same, x, y))
}

/// A `bool` expression that holds when `a` and `b`, two values of the codec
/// `plan`, hold the same arm of `group`, or none of it, with the same members:
/// each compared as an element is, so a body or a list the arm holds compares
/// by its elements or entries, and two lists of one count each as a list.
fn group_equal(
    plan: &EncodePlan<'_>,
    group: &GateGroup,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    let field_of = |id: &str| {
        plan.model
            .fields
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| format!("the codec has no field `{id}`"))
    };
    *fresh += 1;
    let n = *fresh;
    let (gx, gy) = (format!("gx{n}"), format!("gy{n}"));
    // The group where it is there.
    let (x, y) = if group.absent.is_some() {
        (format!("(*{gx})"), format!("(*{gy})"))
    } else {
        (gx.clone(), gy.clone())
    };
    let mut arms = Vec::new();
    for (index, arm) in group.arms.iter().enumerate() {
        let (ax, ay) = if group.is_sum() {
            (
                format!("std::get<{index}>({x})"),
                format!("std::get<{index}>({y})"),
            )
        } else {
            (x.clone(), y.clone())
        };
        let mut same = Vec::new();
        for id in &arm.fields {
            // The second of two lists of one count is held by the first (R5).
            if group.is_second_in(arm, id) {
                continue;
            }
            let field = field_of(id)?;
            let member = accessor(id);
            let one = if holds_chain(group, field) {
                let chain = ChainInArm { group, arm, field };
                chain_same(
                    plan,
                    &chain,
                    &format!("{ax}.{member}()"),
                    &format!("{ay}.{member}()"),
                    held,
                    fresh,
                )?
            } else if let Some(second) = group.second_of(arm, id) {
                // Both lists, each compared as a list is.
                let pair = aligned_name(&member, &accessor(second));
                let (px, py) = (format!("{ax}.{pair}()"), format!("{ay}.{pair}()"));
                let first = value_same(
                    &plan.value(field),
                    &format!("{px}.first()"),
                    &format!("{py}.first()"),
                    held,
                    fresh,
                )?;
                let second = value_same(
                    &plan.value(field_of(second)?),
                    &format!("{px}.second()"),
                    &format!("{py}.second()"),
                    held,
                    fresh,
                )?;
                all_of(&[first, second])
            } else {
                value_same(
                    &plan.value(field),
                    &format!("{ax}.{member}()"),
                    &format!("{ay}.{member}()"),
                    held,
                    fresh,
                )?
            };
            // A member that compares nothing, a body of no element, reads
            // neither value.
            if one != "true" {
                same.push(one);
            }
        }
        arms.push(all_of(&same));
    }
    let body = if group.is_sum() {
        let cases: String = arms
            .iter()
            .enumerate()
            .map(|(index, same)| format!("case {index}: return {same}; "))
            .collect();
        format!(
            "if ({x}.index() != {y}.index()) {{ return false; }} switch ({x}.index()) {{ \
             {cases}default: return false; }}"
        )
    } else if group.absent.is_none() && arms[0] == "true" {
        // An arm whose every member compares nothing reads neither value.
        return Ok("true".to_string());
    } else {
        format!("return {};", arms[0])
    };
    let absent = if group.absent.is_some() {
        format!(
            "if ({gx}.has_value() != {gy}.has_value()) {{ return false; }} if \
             (!{gx}.has_value()) {{ return true; }} "
        )
    } else {
        String::new()
    };
    let read = group_accessor(group);
    Ok(format!(
        "[&](const auto& {gx}, const auto& {gy}) {{ {absent}{body} }}({a}.{read}(), \
         {b}.{read}())"
    ))
}

/// A `bool` expression that holds when `a` and `b`, two values of the codec
/// `plan`, hold the same arm of its variant with the same body, and for the
/// default arm the same tag.
fn variant_equal(
    plan: &EncodePlan<'_>,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    *fresh += 1;
    let n = *fresh;
    let (vx, vy) = (format!("vx{n}"), format!("vy{n}"));
    let mut cases = String::new();
    for (index, arm) in plan.arms.iter().enumerate() {
        let (x, y) = (
            format!("std::get<{index}>({vx})"),
            format!("std::get<{index}>({vy})"),
        );
        let same = match arm.tag {
            Some(_) => held.read(arm.codec, &mut |plan, _, held| {
                body_equal(plan, &x, &y, held, fresh)
            })?,
            // The catch-all holds the tag it was read under beside its body.
            None => {
                let (bx, by) = (format!("{x}.body"), format!("{y}.body"));
                let body = held.read(arm.codec, &mut |plan, _, held| {
                    body_equal(plan, &bx, &by, held, fresh)
                })?;
                all_of(&[format!("{x}.tag == {y}.tag"), body])
            }
        };
        cases.push_str(&format!("case {index}: return {same}; "));
    }
    Ok(format!(
        "[&](const auto& {vx}, const auto& {vy}) {{ if ({vx}.index() != {vy}.index()) {{ return \
         false; }} switch ({vx}.index()) {{ {cases}default: return false; }} }}({a}.body(), \
         {b}.body())"
    ))
}
