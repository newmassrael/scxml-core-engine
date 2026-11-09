// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust source interface of a codec (docs/adr/0058): `{Codec}Source`, the
//! trait a value implements to say what it presents, and the one encode that
//! reads it.
//!
//! A codec's grammar is an interface. What a value of it says — its *presented*
//! elements, in [`codec_roles`](crate::forge::codec_roles) — is a method each;
//! what the encoding determines from those (a length, a count, the flag a gated
//! field is announced by) is not a method but computed by the encode, as the
//! encoding rules of ASN.1 compute a length and never take it from the value.
//! `{Codec}::write_source` is that encode, written once against the interface;
//! the builder `{Codec}` implements it, so a value built the way it always was
//! is encoded through it, and so is a value that holds the parts in its own
//! types.
//!
//! Every statement here is the Rust statement the codec's own `encode` writes for
//! the same wire, with its reads made through the interface and its derived
//! elements computed in a prologue, before the first byte: so an encode that
//! refuses a value refuses it with the sink untouched.

use super::*;
use crate::forge::codec_roles::{
    BindSite, CodecRoles, ElementId, GateGroup, GateVar, GroupArm, InputCheck, InputSource,
    InputValue,
};
use crate::forge::codec_totality::{self, child_alias, RefusalKind};
use crate::forge::encode_plan::{
    self, Absent, Against, ArmInput, Automaton, CheckSite, Clause, Derive, Element, EncodePlan,
    EntryId, Guard, HandedFrom, Law, ListWrite, Measured, Narrow, Number, NumberInto, Offset,
    ParentTag, PresenceFlag, Value, Write,
};
use std::collections::BTreeSet;

pub(super) mod cbor;
pub(super) mod content_line;

/// Why a codec has no source interface (yet): the shape it holds is one the
/// interface does not cover. Scaffolding for the order the shapes land in; the
/// interface covers every codec when the last of them has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Unsupported(pub(super) String);

fn no<T>(why: impl Into<String>) -> Result<T, Unsupported> {
    Err(Unsupported(why.into()))
}

/// What the interface of one codec is made from.
pub(super) struct Input<'a> {
    pub(super) m: &'a CodecModel,
    pub(super) imports: &'a [ImportContext],
    pub(super) l: &'a LangCtx,
    /// `to_pascal_case(m.name)`: the builder's name.
    pub(super) struct_name: &'a str,
    /// `<'a>` when the builder carries a lifetime, otherwise nothing.
    pub(super) struct_lifetime: &'a str,
    /// The lifetime of the codec's own view (`<'a>` or nothing), or `None` when
    /// it has none: views were not asked for, or the view is the builder.
    pub(super) view_lifetime: Option<&'a str>,
    /// Whether the codec has an owned form: it holds a borrowed field.
    pub(super) owned: bool,
    /// Whether the functions that write are always inlined into their callers
    /// (`generate --inline-encode`): the nine small functions of a frame become
    /// one, for the instructions and the call overhead they cost, at the price of
    /// text. Off, the output is as it was.
    pub(super) inline_encode: bool,
}

/// What a function of a codec that writes into the sink `sink` returns, with
/// the bound it needs (docs/adr/0065 §4): the sink's error alone for a codec
/// whose encode is total, and `CodecError`, which the sink's error joins, for
/// one that can refuse.
pub(super) fn write_result(may_fail: bool, sink: &str) -> String {
    if may_fail {
        format!("Result<(), CodecError> where CodecError: From<{sink}::Error>")
    } else {
        format!("Result<(), {sink}::Error>")
    }
}

/// `count_source`: the bytes a codec's `write_source` writes for `src`, run into
/// a sink that only counts. A codec whose encode is total counts with nothing to
/// handle; one that can refuse returns the refusal.
///
/// It is a function of its own, and the count is never run inline in a function
/// that writes into the caller's sink `K`, because such a function of a codec
/// that can refuse carries the bound `CodecError: From<K::Error>`, and Rust then
/// selects that bound for the count's `CodecError: From<Infallible>` too, which
/// it cannot satisfy. `params` and `args` are the codec's flag inputs as
/// declared and as passed on.
pub(super) fn count_function(trait_name: &str, params: &str, args: &str, may_fail: bool) -> String {
    let (ret, run, out) = if may_fail {
        (
            "Result<usize, CodecError>",
            format!("Self::write_source(src, &mut _c{args})?;"),
            "Ok(_c.len())",
        )
    } else {
        (
            "usize",
            format!("let Ok(()) = Self::write_source(src, &mut _c{args});"),
            "_c.len()",
        )
    };
    format!(
        "\x20   /// The bytes [`Self::write_source`] writes for `src`, counted with\n\
         \x20   /// nothing written.\n\
         \x20   #[doc(hidden)]\n\
         \x20   pub fn count_source<S: {trait_name} + ?Sized>(src: &S{params}) -> {ret} {{\n\
         \x20       let mut _c = ::sce_forge_runtime::codec::CountSink::new();\n\
         \x20       {run}\n\
         \x20       {out}\n\
         \x20   }}\n"
    )
}

/// The type the default arm of the codec `struct_name` holds its tag in, which
/// admits no tag the encode could not write ([`codec_totality::UnknownTag`]).
/// The codec's template defines it beside the variant, from the context
/// `render_codec` gives it under the same name.
pub(super) fn unknown_tag_type(struct_name: &str) -> String {
    format!("{struct_name}UnknownTag")
}

/// The attribute of a function that writes, for [`Input::inline_encode`].
pub(super) fn write_attribute(inline_encode: bool) -> &'static str {
    if inline_encode {
        "\x20   #[inline(always)]\n"
    } else {
        ""
    }
}

/// Where a codec's Rust types are, as a conformance harness names them: the
/// path of its module and the name its types are named after.
#[derive(Clone, Debug)]
pub(crate) struct CodecPath {
    /// The module, as the harness reaches it (`fixtures::codec_x`).
    pub module: String,
    /// The name of its types (`CodecX`, of `CodecXParts` and `CodecXSource`).
    pub type_name: String,
}

impl CodecPath {
    fn parts(&self) -> String {
        format!("{}::{}Parts", self.module, self.type_name)
    }

    fn source(&self) -> String {
        format!("{}::{}Source", self.module, self.type_name)
    }
}

/// What a held codec is read with: its plan, its path and the codecs it holds in
/// turn, answered with the text of its case or why there is none.
pub(crate) type ReadHeld<'r> =
    dyn FnMut(&EncodePlan<'_>, &CodecPath, &mut dyn HeldCodecs) -> Result<String, String> + 'r;

/// What a byte case needs of the codecs a codec holds: each read as its holder
/// was, with the names the harness gives it.
pub(crate) trait HeldCodecs {
    /// Answers `read` of the plan of the codec imported as `alias`, its path,
    /// and the codecs it holds in turn.
    fn read(&mut self, alias: &str, read: &mut ReadHeld<'_>) -> Result<String, String>;
}

/// A byte case of a Rust harness (docs/adr/0081 §5): the value a case states as
/// the codec's interface presents it, spelled as a literal of the codec's
/// ready-made source, `{Codec}Parts`, after the bindings of the bodies and
/// entries it holds; and, for each element, an expression that holds when a
/// decode of the bytes presents what the literal does.
pub(crate) struct SourceCase {
    /// `let` statements the literal reads, in order.
    pub bindings: Vec<String>,
    /// The `{Codec}Parts` value.
    pub literal: String,
    /// Each element by its id, with a `bool` expression over `decoded` and
    /// `parts` that holds when the two present the same; and each flag input
    /// the value says, with one that holds when it says the input the case
    /// is handed.
    pub checks: Vec<(String, String)>,
    /// The flag inputs the decode is handed, in the order the codec declares
    /// them, as arguments of its `decode`.
    pub inputs: Vec<String>,
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
    let literal = parts_literal(plan, path, source, held, &mut bindings)?;
    let HandedInputs { args: inputs, said } = handed_inputs(plan, path, inputs)?;
    let mut fresh = 0;
    let mut checks: Vec<(String, String)> = plan
        .elements
        .iter()
        .map(|element| {
            let holds = element_equal(element, path, "&decoded", "&parts", held, &mut fresh)?;
            Ok((element.field.id.clone(), holds))
        })
        .collect::<Result<_, String>>()?;
    for group in plan.groups() {
        checks.push((
            gates_label(group),
            group_equal(plan, group, path, "&decoded", "&parts", held, &mut fresh)?,
        ));
    }
    if plan.model.variant.is_some() {
        checks.push((
            VARIANT_MEMBER.to_string(),
            variant_equal(plan, path, "&decoded", "&parts", held, &mut fresh)?,
        ));
    }
    checks.extend(said);
    Ok(SourceCase {
        bindings,
        literal,
        checks,
        inputs,
    })
}

/// The flag inputs a byte case hands its codec.
#[derive(Default)]
struct HandedInputs {
    /// `decode`'s arguments, in the order the codec declares the inputs.
    args: Vec<String>,
    /// For each input the value says, its label and an expression that holds
    /// when `parts` says the one the case hands.
    said: Vec<(String, String)>,
}

/// The flag inputs a case hands the codec at `path` ([`HandedInputs`]), each
/// one the value says (docs/adr/0065 §6 R4a). An input the value does not say
/// is one the encode takes as an argument, which a case does not spell yet.
fn handed_inputs(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    inputs: &serde_json::Value,
) -> Result<HandedInputs, String> {
    if let Some(input) = plan.given.first() {
        return Err(format!(
            "the value does not say the flag input `{}`, which its encode takes, and a case \
             does not spell that yet",
            input.name
        ));
    }
    let declared = &plan.model.flag_inputs;
    let object = match inputs {
        serde_json::Value::Null if declared.is_empty() => return Ok(HandedInputs::default()),
        serde_json::Value::Object(object) => object,
        _ => return Err("a case of a codec that takes flag inputs states `inputs`".to_string()),
    };
    if let Some(extra) = object
        .keys()
        .find(|key| !declared.iter().any(|i| &i.name == *key))
    {
        return Err(format!("`{extra}` is no flag input of the codec"));
    }
    let mut args = Vec::new();
    let mut said = Vec::new();
    for input in declared {
        let value = object
            .get(&input.name)
            .and_then(serde_json::Value::as_u64)
            .filter(|v| *v <= 1)
            .ok_or_else(|| format!("the case hands `{}` as 0 or 1", input.name))?;
        args.push(value.to_string());
        let says = format!(
            "{}::{}Wire::input_{}(&parts)",
            path.module,
            path.type_name,
            filters::to_snake_case(input.name.clone())
        );
        // The bool itself, or its negation: a comparison with a literal `true`
        // or `false` is what clippy's `bool_comparison` refuses in the harness.
        let not = if value == 1 { "" } else { "!" };
        let holds = if plan.roles.says_input_partly(&input.name) {
            format!("{says}.is_none_or(|said| {not}said)")
        } else {
            format!("{not}{says}")
        };
        said.push((format!("flag input {}", input.name), holds));
    }
    Ok(HandedInputs { args, said })
}

/// Refuse what a literal does not spell yet.
fn spelled(plan: &EncodePlan<'_>) -> Result<(), String> {
    if let Some(variant) = &plan.model.variant {
        // A peeked byte's tag is the arm's own first byte, which the arm's value
        // states: the literal is the one of a variant whose tag is derived.
        if variant.tag_field.is_none() {
            return Err(
                "a variant whose tag its caller supplies is not spelled as a literal yet"
                    .to_string(),
            );
        }
        if let Some(arm) = plan.arms.iter().find(|arm| arm.feature.is_some()) {
            return Err(format!(
                "the arm `{}` is compiled under a feature, which a literal does not spell yet",
                arm.codec
            ));
        }
    }
    Ok(())
}

/// The `{Codec}Parts` literal of `source`, binding each body and list it holds
/// first, in `bindings`: a temporary in a call's argument lives only to the end
/// of its statement, so a list of entries is a named array.
fn parts_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    spelled(plan)?;
    let object = source
        .as_object()
        .ok_or_else(|| "a source is a JSON object".to_string())?;
    if let Some(extra) = object.keys().find(|key| {
        !plan.elements.iter().any(|e| &e.field.id == *key)
            && !plan.groups().iter().any(|g| &gates_label(g) == *key)
            && !(plan.model.variant.is_some() && *key == VARIANT_MEMBER)
    }) {
        return Err(format!(
            "the source states `{extra}`, which the value does not present"
        ));
    }
    let mut members = Vec::new();
    for group in plan.groups() {
        let label = gates_label(group);
        let value = object
            .get(&label)
            .ok_or_else(|| format!("the source states no `{label}`"))?;
        let spelled = group_literal(plan, path, group, value, held, bindings)
            .map_err(|e| format!("`{label}`: {e}"))?;
        members.push(format!("{}: {spelled}", group_accessor(group)));
    }
    if plan.model.variant.is_some() {
        let value = object
            .get(VARIANT_MEMBER)
            .ok_or_else(|| format!("the source states no `{VARIANT_MEMBER}`, the variant's arm"))?;
        let spelled = variant_literal(plan, path, value, held, bindings)
            .map_err(|e| format!("`{VARIANT_MEMBER}`: {e}"))?;
        members.push(format!("{VARIANT_MEMBER}: {spelled}"));
    }
    for element in &plan.elements {
        let id = &element.field.id;
        let value = object
            .get(id)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        let spelled = if value.is_null() {
            if !element.presence.optional() {
                return Err(format!(
                    "`{id}` is always there, and the source states none"
                ));
            }
            "None".to_string()
        } else {
            let one = literal_value(&element.value, value, held, bindings)
                .map_err(|e| format!("`{id}`: {e}"))?;
            if element.presence.optional() {
                format!("Some({one})")
            } else {
                one
            }
        };
        members.push(format!("{}: {spelled}", filters::to_snake_case(id.clone())));
    }
    Ok(format!("{} {{ {} }}", path.parts(), members.join(", ")))
}

/// The member a source states a variant's arm by, and the accessor of it
/// (docs/adr/0058 §1, `fn body(&self)`).
const VARIANT_MEMBER: &str = "body";

/// The arm of the variant a source states as `{"arm": alias, "body": ..}` for a
/// known arm or `{"tag": n, "body": ..}` for the default arm (ADR 0081 §5): the
/// `{Codec}PartsBody` arm holding the body, bound first, and for the default arm
/// the tag it holds.
fn variant_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let object = json
        .as_object()
        .ok_or_else(|| "a variant is a JSON object".to_string())?;
    let body = object
        .get("body")
        .ok_or_else(|| "a variant states `body`, the arm's value".to_string())?;
    let enum_type = format!("{}::{}PartsBody", path.module, path.type_name);
    let mut held_body = |alias: &str| -> Result<String, String> {
        let literal = held.read(alias, &mut |plan, path, held| {
            parts_literal(plan, path, body, held, bindings)
        })?;
        Ok(bind(bindings, None, literal))
    };
    match (object.get("arm"), object.get("tag"), object.len()) {
        (Some(alias), None, 2) => {
            let alias = alias
                .as_str()
                .ok_or_else(|| "`arm` is the alias of the arm's codec".to_string())?;
            if !plan
                .arms
                .iter()
                .any(|a| a.codec == alias && a.tag.is_some())
            {
                return Err(format!("no known arm holds `{alias}`"));
            }
            let name = held_body(alias)?;
            Ok(format!(
                "{enum_type}::{}(&{name})",
                filters::to_pascal_case(alias.to_string())
            ))
        }
        (None, Some(tag), 2) => {
            let tag = tag
                .as_u64()
                .ok_or_else(|| "`tag` is the number the default arm holds".to_string())?;
            let arm = plan
                .arms
                .iter()
                .find(|a| a.tag.is_none())
                .ok_or_else(|| "the variant has no default arm".to_string())?;
            let name = held_body(arm.codec)?;
            Ok(format!(
                "{enum_type}::Default {{ tag: {}::{}::new({tag}).expect(\"the case's tag is no \
                 arm's\"), body: &{name} }}",
                path.module,
                unknown_tag_type(&path.type_name)
            ))
        }
        _ => Err("a variant states `arm` or `tag`, and `body`".to_string()),
    }
}

/// A `bool` expression that holds when `a` and `b`, references to two sources
/// of the codec `plan` at `path`, hold the same arm of its variant with the
/// same body, and for the default arm the same tag.
fn variant_equal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    let source = format!("{}::{}VariantSource", path.module, path.type_name);
    let mut arms = Vec::new();
    for arm in &plan.arms {
        *fresh += 1;
        let (x, y) = (format!("a{fresh}"), format!("b{fresh}"));
        let same = {
            let (x, y) = (x.clone(), y.clone());
            held.read(arm.codec, &mut |plan, path, held| {
                body_equal(plan, path, &x, &y, held, fresh)
            })?
        };
        // A body of no element binds nothing it reads.
        let (x, y) = if same == "true" {
            ("_".to_string(), "_".to_string())
        } else {
            (x, y)
        };
        arms.push(match arm.tag {
            Some(_) => {
                let name = filters::to_pascal_case(arm.codec.to_string());
                format!("({source}::{name}({x}), {source}::{name}({y})) => {same}")
            }
            None => format!(
                "({source}::Default {{ tag: ta{fresh}, body: {x} }}, \
                 {source}::Default {{ tag: tb{fresh}, body: {y} }}) => ta{fresh} == tb{fresh} && {same}"
            ),
        });
    }
    if arms.len() > 1 {
        arms.push("_ => false".to_string());
    }
    Ok(format!(
        "match ({src}::{VARIANT_MEMBER}({a}), {src}::{VARIANT_MEMBER}({b})) {{ {} }}",
        arms.join(", "),
        src = path.source()
    ))
}

/// The value of the gate group `group` a source states as `{"arm": i, ..}`, its
/// members by id, or `null` where the value holds no arm (ADR 0081 §5): the
/// group's type, its arm for a group of several, each member as the arm holds
/// it, and `Option` for a group a value may hold none of.
fn group_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    group: &GateGroup,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    if json.is_null() {
        return if group.absent.is_some() {
            Ok("None".to_string())
        } else {
            Err("the value holds an arm of it always".to_string())
        };
    }
    let object = json
        .as_object()
        .ok_or_else(|| "a group is a JSON object or `null`".to_string())?;
    let arm = object
        .get("arm")
        .and_then(serde_json::Value::as_u64)
        .and_then(|i| group.arms.get(usize::try_from(i).ok()?))
        .ok_or_else(|| "a group states `arm`, the index of one of its arms".to_string())?;
    if let Some(extra) = object
        .keys()
        .find(|key| *key != "arm" && !arm.fields.contains(key))
    {
        return Err(format!("the arm holds no `{extra}`"));
    }
    let mut members = Vec::new();
    let mut spell = |id: &str, bindings: &mut Vec<String>| -> Result<String, String> {
        let field = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| format!("the codec has no field `{id}`"))?;
        let value = object
            .get(id)
            .ok_or_else(|| format!("the arm states no `{id}`"))?;
        if holds_chain(group, field) {
            let chain = ChainInArm { group, arm, field };
            chain_literal(plan, &chain, value, held, bindings)
        } else {
            literal_value(&plan.value(field), value, held, bindings)
        }
        .map_err(|e| format!("`{id}`: {e}"))
    };
    for id in &arm.fields {
        // Two lists of one count are one element of the arm (docs/adr/0065 §6
        // R5): the first holds the second, each stated by its own id.
        if group.is_second_in(arm, id) {
            continue;
        }
        let snake = filters::to_snake_case(id.clone());
        match group.second_of(arm, id) {
            Some(second) => {
                let (first_list, second_list) = (spell(id, bindings)?, spell(second, bindings)?);
                members.push(format!(
                    "{}: {RUNTIME_CODEC}::Aligned::new({first_list}, {second_list}).expect(\"the \
                     case's lists are of one count\")",
                    aligned_name(&snake, &filters::to_snake_case(second.to_string()))
                ));
            }
            None => members.push(format!("{snake}: {}", spell(id, bindings)?)),
        }
    }
    let ty = format!("{}::{}", path.module, group_type(&path.type_name, group));
    let value = if group.is_sum() {
        format!(
            "{ty}::{} {{ {} }}",
            group_arm_name(group, arm),
            members.join(", ")
        )
    } else {
        format!("{ty} {{ {} }}", members.join(", "))
    };
    Ok(if group.absent.is_some() {
        format!("Some({value})")
    } else {
        value
    })
}

/// A name for the next binding of a byte case.
fn bind(bindings: &mut Vec<String>, ty: Option<String>, value: String) -> String {
    let name = format!("held_{}", bindings.len());
    bindings.push(match ty {
        Some(ty) => format!("let {name}: {ty} = {value};"),
        None => format!("let {name} = {value};"),
    });
    name
}

/// The binding of an array of the entries `json` states, each a `{Codec}Parts`
/// of the codec imported as `codec`: a list's, or a chain's an arm of a group
/// of its kinds holds.
fn entries_binding(
    codec: &str,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let rows = json
        .as_array()
        .ok_or_else(|| "a list is a JSON array".to_string())?;
    let mut entries = Vec::new();
    let mut parts = String::new();
    for (i, row) in rows.iter().enumerate() {
        entries.push(
            held.read(codec, &mut |plan, path, held| {
                parts = path.parts();
                parts_literal(plan, path, row, held, bindings)
            })
            .map_err(|e| format!("entry {i}: {e}"))?,
        );
    }
    if rows.is_empty() {
        held.read(codec, &mut |_, path, _| {
            parts = path.parts();
            Ok(String::new())
        })?;
    }
    Ok(bind(
        bindings,
        Some(format!("[{parts}; {}]", entries.len())),
        format!("[{}]", entries.join(", ")),
    ))
}

/// A chain as an arm of a group of its kinds holds it (docs/adr/0065 §6 R6).
struct ChainInArm<'g> {
    group: &'g GateGroup,
    arm: &'g GroupArm,
    field: &'g CodecField,
}

impl ChainInArm<'_> {
    /// Whether the arm holds the chain as a bare entry's place among the
    /// others (R6a).
    fn placed(&self) -> bool {
        self.group.member_is_bare() && !self.group.kinds_in(self.arm).0.is_empty()
    }

    /// Whether the arm may hold no chain: it is gated, and the arm holds no
    /// kind.
    fn optional(&self) -> bool {
        chain_optional_in(self.group, self.field, self.arm)
    }
}

/// The chain as an arm of a group of its kinds holds it (ADR 0081 §5): where
/// the arm holds a bare entry, the others and the entry's place, `{"rest":
/// [..], "place": n}`, as a `Placed`; otherwise the entries whole as a
/// `Chain`, `null` where the arm may hold none. The runtime's constructor
/// checks the kinds the arm sets and clears.
fn chain_literal(
    plan: &EncodePlan<'_>,
    chain: &ChainInArm<'_>,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let Value::List { codec, .. } = plan.value(chain.field) else {
        return Err(format!("`{}` is no chain", chain.field.id));
    };
    if chain.placed() {
        let object = json.as_object().filter(|o| o.len() == 2).ok_or_else(|| {
            "a bare entry's chain is `{\"rest\": [..], \"place\": n}`".to_string()
        })?;
        let place = object
            .get("place")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "`place` is the index of the bare entry".to_string())?;
        let rest = object
            .get("rest")
            .ok_or_else(|| "`rest` holds the other entries".to_string())?;
        let name = entries_binding(codec, rest, held, bindings)?;
        return Ok(format!(
            "{RUNTIME_CODEC}::Placed::new(&{name}[..], {place}).expect(\"the case's place is \
             the first of its kind\")"
        ));
    }
    let optional = chain.optional();
    if json.is_null() {
        return if optional {
            Ok("None".to_string())
        } else {
            Err("the arm holds the chain always".to_string())
        };
    }
    let name = entries_binding(codec, json, held, bindings)?;
    let chain = format!(
        "{RUNTIME_CODEC}::Chain::new(&{name}[..]).expect(\"the case's chain holds the arm's kinds\")"
    );
    Ok(if optional {
        format!("Some({chain})")
    } else {
        chain
    })
}

/// A `bool` expression that holds when `x` and `y`, the chain as two sources
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
    let (rx, ry) = (format!("{x}_rest"), format!("{y}_rest"));
    let entries = value_same(&plan.value(chain.field), &rx, &ry, held, fresh)?;
    if chain.placed() {
        return Ok(format!(
            "{{ let (({rx}, {x}_at), ({ry}, {y}_at)) = ({x}.into_parts(), {y}.into_parts()); \
             {x}_at == {y}_at && {entries} }}"
        ));
    }
    let whole = format!("{{ let ({rx}, {ry}) = ({x}.into_inner(), {y}.into_inner()); {entries} }}");
    Ok(if chain.optional() {
        format!("match ({x}, {y}) {{ (Some({x}), Some({y})) => {whole}, (None, None) => true, _ => false }}")
    } else {
        whole
    })
}

/// One value of the abstract type `value`, as a Rust expression of the type the
/// ready-made source holds it in.
fn literal_value(
    value: &Value<'_>,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
    bindings: &mut Vec<String>,
) -> Result<String, String> {
    let bounded = |inner: String| {
        format!("{RUNTIME_CODEC}::Bounded::new({inner}).expect(\"the case is within its bound\")")
    };
    match value {
        Value::Body { codec } => {
            let body = held.read(codec, &mut |plan, path, held| {
                parts_literal(plan, path, json, held, bindings)
            })?;
            Ok(format!("&{}", bind(bindings, None, body)))
        }
        Value::List { codec, law, .. } if !law.is_none() => {
            // A list held to a law (docs/adr/0065 §6 R8) is its start and its
            // entries, `{"start": {input: 0|1}, "entries": [..]}`; the runtime's
            // constructor runs the law over them.
            let object = json.as_object().filter(|o| o.len() == 2).ok_or_else(|| {
                "a list held to a law is `{\"start\": .., \"entries\": ..}`".to_string()
            })?;
            let start = object
                .get("start")
                .ok_or_else(|| "the list states no `start`".to_string())?;
            let entries = object
                .get("entries")
                .ok_or_else(|| "the list states no `entries`".to_string())?;
            let start = law_start_literal(law, start)?;
            let array = entries_binding(codec, entries, held, bindings)?;
            Ok(format!(
                "{RUNTIME_CODEC}::Lawful::new({start}, {}).expect(\"the case's entries follow \
                 their law\")",
                bounded(format!("&{array}[..]"))
            ))
        }
        Value::List { codec, .. } => {
            let array = entries_binding(codec, json, held, bindings)?;
            Ok(bounded(format!("&{array}[..]")))
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
        ) => json
            .as_u64()
            .map(|n| n.to_string())
            .or_else(|| json.as_i64().map(|n| n.to_string()))
            .ok_or_else(|| "an integer is a JSON integer".to_string()),
        Value::Bytes(_) => {
            let bytes = json
                .as_array()
                .ok_or_else(|| "bytes are an array of numbers".to_string())?
                .iter()
                .map(|b| {
                    b.as_u64()
                        .filter(|b| *b <= 0xFF)
                        .map(|b| b.to_string())
                        .ok_or_else(|| "a byte is a number in 0..=255".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(bounded(format!("&[{}][..]", bytes.join(", "))))
        }
        Value::Text(_) => json
            .as_str()
            .map(|s| bounded(format!("{s:?}")))
            .ok_or_else(|| "text is a JSON string".to_string()),
        Value::Scalar(_) => Err("an enum or a float is not spelled as a literal yet".to_string()),
    }
}

/// The start of a list's law as a byte case's literal (docs/adr/0065 §6 R8),
/// from the case's `{input: 0|1}`: for each input in the law's order, the bool
/// the value gives a Uniform input and `()` for an input the entry before
/// decides; a tuple where the law runs more than one. ([`law_start`] is the
/// other direction: the value one input starts from, read off a lawful list.)
fn law_start_literal(law: &Law, json: &serde_json::Value) -> Result<String, String> {
    let object = json.as_object().ok_or_else(|| {
        "a law's start is a JSON object, one bit for each input it gives".to_string()
    })?;
    let given: Vec<&str> = law
        .inputs
        .iter()
        .filter(|i| matches!(i.automaton, Automaton::Uniform))
        .map(|i| i.input.as_str())
        .collect();
    if let Some(extra) = object.keys().find(|k| !given.contains(&k.as_str())) {
        return Err(format!(
            "the start gives `{extra}`, which the law takes from no start"
        ));
    }
    let parts = law
        .inputs
        .iter()
        .map(|i| match i.automaton {
            Automaton::Uniform => object
                .get(&i.input)
                .and_then(serde_json::Value::as_u64)
                .filter(|v| *v <= 1)
                .map(|v| (v == 1).to_string())
                .ok_or_else(|| format!("the start gives `{}` as 0 or 1", i.input)),
            Automaton::AfterKey { .. } => Ok("()".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match parts.as_slice() {
        [one] => one.clone(),
        many => format!("({})", many.join(", ")),
    })
}

/// A `bool` expression that holds when `x` and `y`, two walks of the entries
/// of the codec imported as `codec`, hold the same entries in order.
fn entries_same(
    codec: &str,
    x: &str,
    y: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    let (ea, eb) = (format!("{x}_entry"), format!("{y}_entry"));
    let entry = {
        let (ea, eb) = (format!("&{ea}"), format!("&{eb}"));
        held.read(codec, &mut |plan, path, held| {
            body_equal(plan, path, &ea, &eb, held, fresh)
        })?
    };
    // An entry of no element binds nothing it reads.
    let (ea, eb) = if entry == "true" {
        ("_".to_string(), "_".to_string())
    } else {
        (ea, eb)
    };
    Ok(format!(
        "{{ let (mut {x}, mut {y}) = ({x}, {y}); loop {{ match ({x}.next(), {y}.next()) {{ \
         (Some({ea}), Some({eb})) => {{ let same = {entry}; if !same {{ break false; }} }}, \
         (None, None) => break true, _ => break false }} }} }}"
    ))
}

/// A `bool` expression that holds when `x` and `y`, two values of the abstract
/// type `value` as a source presents them, are the same: a body's elements
/// each, a list's entries each in order and a lawful list's start, any other
/// value whole.
fn value_same(
    value: &Value<'_>,
    x: &str,
    y: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    match value {
        Value::Body { codec } => held.read(codec, &mut |plan, path, held| {
            body_equal(plan, path, x, y, held, fresh)
        }),
        Value::List { codec, law, .. } if !law.is_none() => {
            let (lx, ly) = (format!("{x}_list"), format!("{y}_list"));
            let entries = entries_same(codec, &lx, &ly, held, fresh)?;
            // A law whose every input the entry before decides starts from
            // `()`: there is no start to compare.
            let started = law
                .inputs
                .iter()
                .any(|i| matches!(i.automaton, Automaton::Uniform));
            Ok(if started {
                format!(
                    "{{ let (({x}_start, {lx}), ({y}_start, {ly})) = ({x}.into_parts(), \
                     {y}.into_parts()); {x}_start == {y}_start && {entries} }}"
                )
            } else {
                format!(
                    "{{ let ((_, {lx}), (_, {ly})) = ({x}.into_parts(), {y}.into_parts()); \
                     {entries} }}"
                )
            })
        }
        Value::List { codec, .. } => entries_same(codec, x, y, held, fresh),
        _ => Ok(format!("{x} == {y}")),
    }
}

/// `checks`, each a `bool` expression, as one that holds when every one does.
fn all_of(checks: &[String]) -> String {
    // Parenthesised only where they are joined: alone, a check is a block, a
    // `match` or a comparison that needs none.
    match checks {
        [] => "true".to_string(),
        [one] => one.clone(),
        many => many
            .iter()
            .map(|c| format!("({c})"))
            .collect::<Vec<_>>()
            .join(" && "),
    }
}

/// A `bool` expression that holds when the values `a` and `b` of the codec at
/// `path`, references to two sources, present the same `element`: its value,
/// or for a body each element of it and for a list each entry in order.
fn element_equal(
    element: &Element<'_>,
    path: &CodecPath,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    let accessor = filters::to_snake_case(element.field.id.clone());
    let read = |v: &str| format!("{}::{accessor}({v})", path.source());
    let (left, right) = (read(a), read(b));
    if !matches!(element.value, Value::Body { .. } | Value::List { .. }) {
        return Ok(format!("{left} == {right}"));
    }
    *fresh += 1;
    let (x, y) = (format!("a{fresh}"), format!("b{fresh}"));
    let same = value_same(&element.value, &x, &y, held, fresh)?;
    // A body of no element binds nothing it reads.
    let (x, y) = if same == "true" {
        ("_".to_string(), "_".to_string())
    } else {
        (x, y)
    };
    Ok(if element.presence.optional() {
        format!(
            "match ({left}, {right}) {{ (Some({x}), Some({y})) => {same}, (None, None) => true, _ => false }}"
        )
    } else {
        format!("{{ let ({x}, {y}) = ({left}, {right}); {same} }}")
    })
}

/// A `bool` expression that holds when `a` and `b`, references to two sources
/// of the codec `plan` at `path`, hold the same arm of `group`, or none of it,
/// with the same members: each compared as an element is, so a body or a list
/// the arm holds compares by its elements or entries.
fn group_equal(
    plan: &EncodePlan<'_>,
    group: &GateGroup,
    path: &CodecPath,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    let ty = format!("{}::{}", path.module, group_type(&path.type_name, group));
    let held_in = |pattern: String| {
        if group.absent.is_some() {
            format!("Some({pattern})")
        } else {
            pattern
        }
    };
    let mut arms = Vec::new();
    for arm in &group.arms {
        let (mut xs, mut ys, mut same) = (Vec::new(), Vec::new(), Vec::new());
        let field_of = |id: &str| {
            plan.model
                .fields
                .iter()
                .find(|f| f.id == id)
                .ok_or_else(|| format!("the codec has no field `{id}`"))
        };
        for id in &arm.fields {
            // The second of two lists of one count is held by the first (R5).
            if group.is_second_in(arm, id) {
                continue;
            }
            let field = field_of(id)?;
            *fresh += 1;
            let (x, y) = (format!("a{fresh}"), format!("b{fresh}"));
            let mut member = filters::to_snake_case(id.clone());
            let one = if holds_chain(group, field) {
                let chain = ChainInArm { group, arm, field };
                chain_same(plan, &chain, &x, &y, held, fresh)?
            } else if let Some(second) = group.second_of(arm, id) {
                // Both lists, each compared as a list is.
                member = aligned_name(&member, &filters::to_snake_case(second.to_string()));
                let (xa, xb, ya, yb) = (
                    format!("{x}_a"),
                    format!("{x}_b"),
                    format!("{y}_a"),
                    format!("{y}_b"),
                );
                let first = value_same(&plan.value(field), &xa, &ya, held, fresh)?;
                let second = value_same(&plan.value(field_of(second)?), &xb, &yb, held, fresh)?;
                format!(
                    "{{ let (({xa}, {xb}), ({ya}, {yb})) = ({x}.into_parts(), {y}.into_parts()); {} }}",
                    all_of(&[first, second])
                )
            } else {
                value_same(&plan.value(field), &x, &y, held, fresh)?
            };
            // A body of no element binds nothing it reads.
            if one == "true" {
                xs.push(format!("{member}: _"));
                ys.push(format!("{member}: _"));
            } else {
                xs.push(format!("{member}: {x}"));
                ys.push(format!("{member}: {y}"));
                same.push(one);
            }
        }
        let arm_path = if group.is_sum() {
            format!("{ty}::{}", group_arm_name(group, arm))
        } else {
            ty.clone()
        };
        arms.push(format!(
            "({}, {}) => {}",
            held_in(format!("{arm_path} {{ {} }}", xs.join(", "))),
            held_in(format!("{arm_path} {{ {} }}", ys.join(", "))),
            all_of(&same)
        ));
    }
    if group.absent.is_some() {
        arms.push("(None, None) => true".to_string());
    }
    if arms.len() > 1 {
        arms.push("_ => false".to_string());
    }
    let accessor = group_accessor(group);
    Ok(format!(
        "match ({src}::{accessor}({a}), {src}::{accessor}({b})) {{ {} }}",
        arms.join(", "),
        src = path.source()
    ))
}

/// A `bool` expression that holds when `a` and `b`, references to two values
/// of the codec `plan` at `path`, present the same elements.
fn body_equal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    a: &str,
    b: &str,
    held: &mut dyn HeldCodecs,
    fresh: &mut usize,
) -> Result<String, String> {
    spelled(plan)?;
    let mut checks = plan
        .elements
        .iter()
        .map(|element| element_equal(element, path, a, b, held, fresh))
        .collect::<Result<Vec<_>, _>>()?;
    for group in plan.groups() {
        checks.push(group_equal(plan, group, path, a, b, held, fresh)?);
    }
    if plan.model.variant.is_some() {
        checks.push(variant_equal(plan, path, a, b, held, fresh)?);
    }
    Ok(all_of(&checks))
}

/// The source interface of a codec, as the text a codec file holds, or why the
/// codec has none yet.
pub(super) fn render(input: &Input<'_>) -> Result<String, Unsupported> {
    let plan = Plan::of(input)?;
    let text = plan.text();
    plan.encode.totality.assert_all_written(&input.m.name);
    Ok(text)
}

/// The type a codec named `name` is written through: `{name}Wire`, which holds
/// no value and takes no profile, so its functions compile whatever profiles
/// the codec's fields can be held by (docs/adr/0077 §7).
pub(crate) fn wire_name(name: &str) -> String {
    format!("{name}Wire")
}

/// The link from a codec's `write_source` to the counted-source witness
/// (docs/adr/0061): `{Codec}::measure`, and the impl of `SourceWrite` that lets
/// `Measured::write_into` run the codec's own skeleton into a window. It states
/// no wire rule; the one statement is `write_source`. `lt` is the codec's
/// lifetime parameter (`<'a>` or nothing). A codec that takes a flag from its
/// parent is written only through that parent and has no measure.
///
/// A codec whose encode is total states the sink's error as its own
/// (`type Error<E> = E`), so its measure, whose count cannot refuse, returns the
/// witness with no `Result` (docs/adr/0065 §4); one that can refuse states
/// `CodecError`.
pub(super) fn measure_impl(
    name: &str,
    lt: &str,
    trait_name: &str,
    inline_encode: bool,
    may_fail: bool,
) -> String {
    let generics = match lt.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        Some(lifetime) => format!("<{lifetime}, S: {trait_name} + ?Sized>"),
        None => format!("<S: {trait_name} + ?Sized>"),
    };
    // The window is written through the sink that asks to be inlined when the
    // codec's writes are, so that the whole write can fold into the caller.
    let (inline, sink) = if inline_encode {
        (write_attribute(true), "WindowSink")
    } else {
        ("", "SliceSink")
    };
    // The trait's own `write_window` goes through `SliceSink`; a codec whose
    // writes fold into the caller says the window sink that does the same, by
    // way of the inherent function of that name (which an inherent call
    // resolves to before the trait's).
    let window_choice = if inline_encode {
        format!(
            "\n{inline}\
             \x20   fn write_window(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {{\n\
             \x20       Self::write_window(src, window)\n\
             \x20   }}\n"
        )
    } else {
        String::new()
    };
    let (error, write_error) = if may_fail {
        ("CodecError", "CodecError")
    } else {
        ("E", "K::Error")
    };
    let measure = if may_fail {
        format!(
            "\x20   /// The size of the frame [`Self::encode_source`] writes for `src`, found\n\
             \x20   /// with nothing written, or the refusal that encode would give. The result\n\
             \x20   /// writes the frame into a window of exactly that size and cannot fail.\n\
             {inline}\
             \x20   pub fn measure<S: {trait_name} + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {{\n\
             \x20       ::sce_forge_runtime::codec::measure(src)\n\
             \x20   }}\n\n"
        )
    } else {
        format!(
            "\x20   /// The size of the frame [`Self::encode_source`] writes for `src`, found\n\
             \x20   /// with nothing written: the encode refuses no value, so neither does\n\
             \x20   /// this. The result writes the frame into a window of exactly that size\n\
             \x20   /// and cannot fail.\n\
             {inline}\
             \x20   pub fn measure<S: {trait_name} + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {{\n\
             \x20       let Ok(measured) = ::sce_forge_runtime::codec::measure(src);\n\
             \x20       measured\n\
             \x20   }}\n\n"
        )
    };
    format!(
        "impl{generics} ::sce_forge_runtime::codec::SourceWrite<S> for {name}{lt} {{\n\
         \x20   type Error<E> = {error};\n\n\
         {inline}\
         \x20   fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), {write_error}> where CodecError: From<K::Error> {{\n\
         \x20       Self::write_source(src, w)\n\
         \x20   }}\n\
         {window_choice}\
         }}\n\n\
         #[allow(dead_code)]\n\
         impl{lt} {name}{lt} {{\n\
         {measure}\
         \x20   /// Write `src` into `window` in one pass and return the bytes it is, for a\n\
         \x20   /// window as large as the caller can spare. On an error the window holds a\n\
         \x20   /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].\n\
         {inline}\
         \x20   pub fn write_window<S: {trait_name} + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {{\n\
         \x20       let mut sink = ::sce_forge_runtime::codec::{sink}::new(window);\n\
         \x20       Self::write_source(src, &mut sink)?;\n\
         \x20       Ok(sink.position())\n\
         \x20   }}\n\
         }}\n"
    )
}

/// A representation of a codec's value: the builder a sender fills, the view of
/// a decoded frame, or the owned form that outlives it. Each says what it holds
/// through the same trait.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Builder,
    View,
    Owned,
}

impl Form {
    /// What follows the codec's name in the name of its variant enum.
    fn variant_suffix(self) -> &'static str {
        match self {
            Form::Builder => "Variant",
            Form::View => "ViewVariant",
            Form::Owned => "OwnedVariant",
        }
    }
}

/// One arm of a codec's variant.
struct Arm {
    /// The import alias of the codec that is its body.
    alias: String,
    /// The enum variant that holds it: the body codec's name, or `Default`.
    variant_name: String,
    /// The type parameter of `{Codec}Body` that is the source of its body.
    generic: String,
    child: Child,
    /// The tag that names it; `None` for the default arm.
    value: Option<u64>,
    /// The build feature it is compiled under (docs/adr/0072); `None` for an
    /// arm every build holds, the default arm among them.
    feature: Option<String>,
}

impl Arm {
    /// The line of `#[cfg]` that compiles an item of this arm only where its
    /// feature is on, indented by `indent`; nothing for an arm every build
    /// holds.
    fn cfg_line(&self, indent: &str) -> String {
        match &self.feature {
            Some(feature) => format!("{indent}{}\n", rust_feature_cfg(feature)),
            None => String::new(),
        }
    }

    /// The same `#[cfg]` written in front of an item on its line.
    fn cfg_prefix(&self) -> String {
        match &self.feature {
            Some(feature) => format!("{} ", rust_feature_cfg(feature)),
            None => String::new(),
        }
    }
}

/// One set of the build features a variant's arms name, and the `#[cfg]`
/// predicate that holds in exactly the builds with those features on and the
/// rest off (docs/adr/0072).
struct World {
    /// `#[cfg(..)]`, or `None` when no arm names a feature and every build is
    /// the one world.
    cfg: Option<String>,
    /// The features on in this world.
    on: std::collections::BTreeSet<String>,
}

impl World {
    /// Whether `arm` is compiled in this world.
    fn holds(&self, arm: &Arm) -> bool {
        match &arm.feature {
            Some(feature) => self.on.contains(feature),
            None => true,
        }
    }
}

/// The imported codec a list or an embedded body holds.
struct Child {
    /// The type the codec's wire is written through, `{Codec}Wire`
    /// (`Type::write_source`), by its path.
    type_name: String,
    /// The path of its source trait.
    trait_path: String,
    /// The path of its overrides (`{Codec}Overrides`), which a chain of the
    /// codec's entries sets the end flag through.
    overrides_path: String,
    /// The path of its borrowed source (`{Codec}Parts`) with the lifetime it
    /// takes, as a field of a parent's parts names it.
    parts_type: String,
}

/// The Rust printer of a codec's encode plan (docs/adr/0081 §2): the plan, and
/// what the printer spells it with.
struct Plan<'a> {
    input: &'a Input<'a>,
    /// The encode plan, which decides what every backend's encode shares: the
    /// roles, the totality (every refusal written here must be in it) and the
    /// interface.
    encode: EncodePlan<'a>,
    /// `{Struct}Source`.
    trait_name: String,
}

/// Whether `m` has a source interface, given whether each codec it imports has
/// one. Shared by the codec's own generation and by the enrichment of a parent
/// that imports it, so the two cannot disagree on which codecs the interface
/// covers.
pub(crate) fn supported_with(
    m: &CodecModel,
    mut import_has_source: impl FnMut(&str) -> bool,
) -> bool {
    // The answer for each codec the model holds, asked once and before the
    // check, which cannot take a closure that borrows its caller's walk.
    let known: std::collections::BTreeMap<&str, bool> = child_aliases(m)
        .into_iter()
        .map(|alias| (alias, import_has_source(alias)))
        .collect();
    check(m, &|alias| known.get(alias).copied().unwrap_or(false)).is_ok()
}

// ── groups (docs/adr/0065 §6 R4b, R4c) ───────────────────────────────
// The names a group goes by, read by the source interface here and by the
// forms the generator renders, so the two spell a group alike.

/// The word a chain's membership of the kind `key` goes by in a name
/// (docs/adr/0065 §6 R6a): `has_0x12`.
fn member_word(key: u64) -> String {
    format!("has_0x{key:x}")
}

/// The gates of a group as words of a name: the input, the carrier and its
/// flag, or the chain and the kind it is asked for; several flags of one
/// carrier as the carrier and the flags, each joined by `or`. A group of a
/// chain's memberships is the chain, the kinds, then the other gates
/// (docs/adr/0065 §6 R6b): its gates are decided together, not joined.
fn gate_words(group: &GateGroup) -> String {
    if let Some(chain) = group.chain() {
        let mut words = vec![chain.to_string()];
        words.extend(group.gates.iter().map(|g| match g {
            GateVar::Input(name) => name.clone(),
            GateVar::Flag { carrier, flag } => format!("{carrier}_{flag}"),
            GateVar::Member { key, .. } => member_word(*key),
        }));
        return words.join("_");
    }
    let flag_of = |gate: &GateVar| match gate {
        GateVar::Input(name) => name.clone(),
        GateVar::Flag { flag, .. } => flag.clone(),
        GateVar::Member { key, .. } => member_word(*key),
    };
    let carriers: BTreeSet<&str> = group
        .gates
        .iter()
        .filter_map(|g| match g {
            GateVar::Flag { carrier, .. } => Some(carrier.as_str()),
            GateVar::Input(_) | GateVar::Member { .. } => None,
        })
        .collect();
    let one_carrier = carriers.len() == 1
        && group
            .gates
            .iter()
            .all(|g| matches!(g, GateVar::Flag { .. }));
    if one_carrier {
        let carrier = carriers.into_iter().next().expect("one carrier");
        let flags: Vec<String> = group.gates.iter().map(flag_of).collect();
        format!("{carrier}_{}", flags.join("_or_"))
    } else {
        group
            .gates
            .iter()
            .map(|g| match g {
                GateVar::Input(name) => name.clone(),
                GateVar::Flag { carrier, flag } => format!("{carrier}_{flag}"),
                GateVar::Member { chain, key, .. } => format!("{chain}_{}", member_word(*key)),
            })
            .collect::<Vec<_>>()
            .join("_or_")
    }
}

/// The accessor of a group, and the field each form holds it in.
pub(crate) fn group_accessor(group: &GateGroup) -> String {
    filters::to_snake_case(gate_words(group))
}

/// `words` as a Rust type or variant name, each word in Pascal case: a flag
/// the document spells in capitals (`CURRENT`) is a word (`Current`), not an
/// acronym.
fn pascal_words(words: &str) -> String {
    words
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| filters::to_rust_variant(w.to_string()))
        .collect()
}

/// The type a group of the codec `codec` is held as: an enum of its arms, or a
/// struct of its one arm, named after the gates; a struct of the arm present
/// while one gate does not hold says so in its name.
pub(crate) fn group_type(codec: &str, group: &GateGroup) -> String {
    let pascal = pascal_words(&gate_words(group));
    match group.arms.as_slice() {
        [arm] if arm.values == [false] => format!("{codec}Not{pascal}"),
        _ => format!("{codec}{pascal}"),
    }
}

/// The type that names the kind of entry `key` the chain `chain` of the codec
/// `codec` is asked for (docs/adr/0065 §6 R6a): the codec, the chain and the
/// kind, and `Key`, the name a group of that one membership has with `Key`.
pub(crate) fn key_type(codec: &str, chain: &str, key: u64) -> String {
    format!(
        "{codec}{}Key",
        pascal_words(&format!("{chain}_{}", member_word(key)))
    )
}

/// The inputs the entries of the list `field` say by the list's law
/// (docs/adr/0065 §6 R8), in the order its law type holds them.
pub(crate) fn list_laws<'r>(roles: &'r CodecRoles, field: &str) -> Vec<&'r InputCheck> {
    roles
        .list_laws
        .iter()
        .filter(|law| matches!(&law.binder.site, BindSite::List { field: f } if f == field))
        .collect()
}

/// The type that reads the input `input` off an entry of the list `list` of
/// the codec `codec` (docs/adr/0065 §6 R8): the codec, the list and the input.
pub(crate) fn says_type(codec: &str, list: &str, input: &str) -> String {
    format!("{codec}{}", pascal_words(&format!("{list}_{input}")))
}

/// The name of the law the list `field` of the codec `codec` holds its entries
/// to, or `None` where they say no input it hands them (docs/adr/0065 §6 R8):
/// `{Codec}{List}Law`, which the codec defines as [`law_definition`] says. Every
/// form names the law by it.
pub(crate) fn law_type(roles: &CodecRoles, codec: &str, field: &str) -> Option<String> {
    law_name(codec, field, &encode_plan::list_law(roles, field))
}

/// The name of `law`, the law of the list `field` of the codec `codec`
/// ([`law_type`]).
fn law_name(codec: &str, field: &str, law: &Law) -> Option<String> {
    (!law.is_none()).then(|| format!("{codec}{}Law", pascal_words(field)))
}

/// What the law of the list `field` of the codec `codec` is, or `None` where it
/// has none ([`law_text`]).
pub(crate) fn law_definition(roles: &CodecRoles, codec: &str, field: &str) -> Option<String> {
    law_text(codec, field, &encode_plan::list_law(roles, field))
}

/// `law`, the law of the list `field` of the codec `codec`, as the runtime's
/// automata: one for each input, several run together as a tuple; `None` where
/// it has none.
fn law_text(codec: &str, field: &str, law: &Law) -> Option<String> {
    let laws: Vec<String> = law
        .inputs
        .iter()
        .map(|input| {
            let says = says_type(codec, field, &input.input);
            match input.automaton {
                Automaton::AfterKey { id } => format!(
                    "{RUNTIME_CODEC}::AfterKey<{says}, {}>",
                    key_type(codec, field, id)
                ),
                Automaton::Uniform => format!("{RUNTIME_CODEC}::Uniform<{says}>"),
            }
        })
        .collect();
    match laws.as_slice() {
        [] => None,
        [law] => Some(law.clone()),
        laws => Some(format!("({})", laws.join(", "))),
    }
}

/// `list`, the type a form holds the list `field` of the codec `codec` in,
/// held to the list's law where it has one (docs/adr/0065 §6 R8).
pub(crate) fn lawful_list(roles: &CodecRoles, codec: &str, field: &str, list: String) -> String {
    lawful(codec, field, &encode_plan::list_law(roles, field), list)
}

/// `list` held to `law`, the law of the list `field` of the codec `codec`,
/// where it has one ([`lawful_list`]).
fn lawful(codec: &str, field: &str, law: &Law, list: String) -> String {
    match law_name(codec, field, law) {
        Some(law) => format!("{RUNTIME_CODEC}::Lawful<{list}, {law}>"),
        None => list,
    }
}

/// The value of the input `input` that the list `field`, held to its law as the
/// expression `list`, starts from (docs/adr/0065 §6 R8).
pub(crate) fn law_start(roles: &CodecRoles, field: &str, input: &str, list: &str) -> String {
    let laws = list_laws(roles, field);
    let at = laws
        .iter()
        .position(|law| law.binder.input == input)
        .expect("the list holds the input by its law");
    if laws.len() == 1 {
        format!("{list}.start()")
    } else {
        format!("{list}.start().{at}")
    }
}

/// The start a decode gives the list `field`, from `value`, the expression
/// for each input the list hands its entries (docs/adr/0065 §6 R8): `()` for
/// an input a chain supplies from the entry before, whose start is fixed.
pub(crate) fn law_start_of(
    roles: &CodecRoles,
    field: &str,
    value: impl Fn(&InputCheck) -> String,
) -> String {
    let starts: Vec<String> = list_laws(roles, field)
        .iter()
        .map(|law| match law.against {
            InputValue::PreviousEntry { .. } => "()".to_string(),
            _ => value(law),
        })
        .collect();
    match starts.as_slice() {
        [start] => start.clone(),
        starts => format!("({},)", starts.join(", ")),
    }
}

/// The name an arm holds two lists of one count under (docs/adr/0065 §6 R5): the
/// two lists' names.
pub(crate) fn aligned_name(first: &str, second: &str) -> String {
    format!("{first}_{second}")
}

/// Whether `field` is the chain whose memberships are gates of `group`.
pub(crate) fn holds_chain(group: &GateGroup, field: &CodecField) -> bool {
    group.chain() == Some(field.id.as_str())
}

/// Whether the chain `field` of a group of its memberships may be absent in
/// `arm`: the chain is itself gated, and the arm holds no kind of entry, which
/// an absent chain holds none of either.
pub(crate) fn chain_optional_in(group: &GateGroup, field: &CodecField, arm: &GroupArm) -> bool {
    field.present_if.is_some() && group.kinds_in(arm).0.is_empty()
}

/// The type the chain `field` of a group of its memberships has in `arm`, held
/// as `list` (docs/adr/0065 §6 R6a, R6b): the chain whole with an entry of each
/// kind the arm sets and of none it clears, as an `Option` where it may be
/// absent; where the one gate on a bare entry holds, the others and the named
/// entry's place. Every arm holds the one list. `codec` is the type name of the
/// codec that holds the chain, and `fields` its field list.
pub(crate) fn chain_arm_type(
    codec: &str,
    fields: &[CodecField],
    group: &GateGroup,
    field: &CodecField,
    arm: &GroupArm,
    list: &str,
) -> String {
    let (held, not) = group.kinds_in(arm);
    if group.member_is_bare() && !held.is_empty() {
        return format!(
            "{RUNTIME_CODEC}::Placed<{list}, {}, {}>",
            key_type(codec, &field.id, held[0]),
            super::rust_bound_most(codec_totality::list_bound(field, fields))
        );
    }
    let kinds = |keys: &[u64]| match keys {
        [] => "()".to_string(),
        [key] => format!("({},)", key_type(codec, &field.id, *key)),
        keys => format!(
            "({})",
            keys.iter()
                .map(|key| key_type(codec, &field.id, *key))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let chain = format!(
        "{RUNTIME_CODEC}::Chain<{list}, {}, {}, {}>",
        kinds(&held),
        kinds(&not),
        super::rust_list_bound_args(field, fields)
    );
    if chain_optional_in(group, field, arm) {
        format!("Option<{chain}>")
    } else {
        chain
    }
}

/// The name of an arm of a group: for one gate, the flag (`Not` before it for
/// the arm present while it does not hold); for several, the flags the arm
/// holds set, or `Neither` where it holds none. The type names the carrier
/// already, so an arm names the flags alone.
pub(crate) fn group_arm_name(group: &GateGroup, arm: &GroupArm) -> String {
    let flag_of = |gate: &GateVar| match gate {
        GateVar::Input(name) => pascal_words(name),
        GateVar::Flag { flag, .. } => pascal_words(flag),
        GateVar::Member { key, .. } => pascal_words(&member_word(*key)),
    };
    if let ([gate], [value]) = (group.gates.as_slice(), arm.values.as_slice()) {
        return if *value {
            flag_of(gate)
        } else {
            format!("Not{}", flag_of(gate))
        };
    }
    let set: String = group
        .gates
        .iter()
        .zip(&arm.values)
        .filter(|(_, v)| **v)
        .map(|(g, _)| flag_of(g))
        .collect();
    if set.is_empty() {
        "Neither".to_string()
    } else {
        set
    }
}

/// The gates of a group as the document spells their disjunction.
fn gates_label(group: &GateGroup) -> String {
    group
        .gates
        .iter()
        .map(GateVar::to_string)
        .collect::<Vec<_>>()
        .join(" || ")
}

/// When the value holds `arm`, in words: each gate and whether it holds.
fn arm_condition(group: &GateGroup, values: &[bool]) -> String {
    group
        .gates
        .iter()
        .zip(values)
        .map(|(gate, value)| {
            if *value {
                format!("`{gate}` holds")
            } else {
                format!("`{gate}` does not hold")
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Whether `field`'s type differs between the forms of a value, so that a
/// group holding it takes it as a type parameter.
pub(crate) fn member_varies(field: &CodecField) -> bool {
    field.is_repeat()
        || field.is_tlv_chain()
        || field.is_embed()
        || matches!(field.sce_type, SceType::Bytes | SceType::String)
}

/// Whether the borrowed source of a codec (`{Codec}Parts`) carries a lifetime:
/// it holds text, bytes, a list, an embedded body or a variant, each of which it
/// borrows from the caller. A codec of scalars alone holds everything by value.
pub(crate) fn parts_has_lifetime(m: &CodecModel) -> bool {
    m.variant.is_some()
        || m.fields.iter().any(|f| {
            f.is_repeat()
                || f.is_tlv_chain()
                || f.is_embed()
                || matches!(f.sce_type, SceType::Bytes | SceType::String)
        })
}

/// The shapes the interface covers.
fn check(m: &CodecModel, import_has_source: &dyn Fn(&str) -> bool) -> Result<(), Unsupported> {
    if m.encoding != CodecEncoding::Positional {
        return no("a codec of another encoding writes its own encode");
    }
    for alias in child_aliases(m) {
        if !import_has_source(alias) {
            return no(format!(
                "the codec `{alias}` it holds has no source interface"
            ));
        }
    }
    Ok(())
}

/// Every import alias of a codec a model holds: in a list, an embedded body or
/// an arm of its variant.
fn child_aliases(m: &CodecModel) -> Vec<&str> {
    let mut aliases: Vec<&str> = m.fields.iter().filter_map(child_alias).collect();
    if let Some(variant) = &m.variant {
        aliases.extend(
            variant
                .arms
                .iter()
                .chain(variant.default_arm.iter())
                .map(|arm| arm.body_alias.as_str()),
        );
    }
    aliases
}

impl<'a> Plan<'a> {
    fn of(input: &'a Input<'a>) -> Result<Self, Unsupported> {
        let has_source = |alias: &str| {
            input
                .imports
                .iter()
                .any(|i| i.alias == alias && i.kind == "codec" && i.codec_has_source)
        };
        check(input.m, &has_source)?;
        Ok(Plan {
            input,
            encode: EncodePlan::of(input.m, input.imports),
            trait_name: format!("{}Source", input.struct_name),
        })
    }

    /// Whether the encode can refuse a value (docs/adr/0065): it then returns
    /// `CodecError`, which the sink's error joins, and otherwise the sink's
    /// error alone.
    fn may_fail(&self) -> bool {
        !self.encode.totality.is_total()
    }

    /// The error the encode returns where it refuses at `element` with `kind`,
    /// which the totality analysis must name ([`codec_totality::Totality::require`]).
    fn refuse(&self, kind: RefusalKind, element: &str) -> String {
        self.encode.totality.require(kind, element)
    }

    /// Whether the codec imported as `alias` can refuse a value.
    fn child_may_fail(&self, alias: &str) -> bool {
        self.input
            .imports
            .iter()
            .any(|i| i.alias == alias && i.kind == "codec" && i.codec_source_can_refuse)
    }

    /// The `?` that passes on what a codec written at `element` (the import
    /// `alias`) returns: its refusal, recorded as written, when it can refuse,
    /// and its sink's error otherwise.
    fn child_write(&self, alias: &str, element: &str) -> &'static str {
        if self.child_may_fail(alias) {
            self.refuse(RefusalKind::Child, element);
        }
        "?"
    }

    /// What a function that writes into the sink `sink` returns, with the bound
    /// it needs: the sink's error alone for a total encode, and `CodecError`,
    /// which the sink's error joins, for one that can refuse.
    fn write_result(&self, sink: &str) -> String {
        write_result(self.may_fail(), sink)
    }

    /// The imported codec a list or an embedded body holds, as the type it is
    /// written through and the trait it is read through.
    fn child(&self, field: &CodecField) -> Option<Child> {
        self.child_of(child_alias(field)?)
    }

    /// The imported codec named by `alias`.
    fn child_of(&self, alias: &str) -> Option<Child> {
        let import = self.input.imports.iter().find(|i| i.alias == alias)?;
        Some(Child {
            type_name: format!(
                "super::{}::{}",
                import.namespace,
                wire_name(&import.type_name)
            ),
            trait_path: format!("super::{}::{}Source", import.namespace, import.type_name),
            overrides_path: format!("super::{}::{}Overrides", import.namespace, import.type_name),
            parts_type: format!(
                "super::{}::{}Parts{}",
                import.namespace,
                import.type_name,
                if import.codec_parts_lifetime {
                    "<'a>"
                } else {
                    ""
                }
            ),
        })
    }

    /// The flags of this codec a parent may set in place of the ones a value
    /// presents: a one-bit flag of a fixed carrier that the codec presents. A
    /// chain derives the flag that says an entry is not the last, and the entry
    /// it writes cannot present that flag, as it cannot know where it stands.
    fn overridable(&self) -> Vec<(&CodecField, &FlagDef)> {
        self.encode.overridable.clone()
    }

    /// The member of the entry codec's overrides that sets the flag a chain ends
    /// on, for a chain that ends on its entries' flag.
    fn chain_end_flag(&self, list: &ListWrite) -> Option<String> {
        list.ends
            .as_ref()
            .map(|(carrier, flag)| self.override_member_of(carrier, flag))
    }

    /// The name of the member of `{Codec}Overrides` that sets `flag` of `carrier`.
    fn override_member(&self, carrier: &CodecField, flag: &str) -> String {
        self.override_member_of(&carrier.id, flag)
    }

    /// The name of the member of the overrides of a codec that sets `flag` of its
    /// carrier `carrier`.
    fn override_member_of(&self, carrier: &str, flag: &str) -> String {
        format!(
            "{}_{}",
            self.l().codec_field_id(carrier),
            filters::to_snake_case(flag.to_string())
        )
    }

    fn overrides_name(&self) -> String {
        format!("{}Overrides", self.name())
    }

    fn overrides_definition(&self) -> String {
        let overridable = self.overridable();
        if overridable.is_empty() {
            return String::new();
        }
        let members: String = overridable
            .iter()
            .map(|(carrier, def)| {
                format!(
                    "    /// The `{flag}` flag of `{carrier}`, when a parent sets it.\n    pub {member}: Option<bool>,\n",
                    flag = def.name,
                    carrier = carrier.id,
                    member = self.override_member(carrier, &def.name),
                )
            })
            .collect();
        // One constructor for each flag: the chain sets one and leaves the rest,
        // which a struct literal says only with an update that clippy refuses
        // for a struct of one member.
        let setters: String = overridable
            .iter()
            .map(|(carrier, def)| {
                let member = self.override_member(carrier, &def.name);
                format!(
                    "    /// Sets `{flag}` of `{carrier}` and nothing else.\n    #[allow(clippy::needless_update)]\n    pub fn {member}(value: bool) -> Self {{\n        Self {{ {member}: Some(value), ..Self::default() }}\n    }}\n",
                    flag = def.name,
                    carrier = carrier.id,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "/// Flags of `{name}` a parent sets in place of the ones a value presents:\n\
             /// the end flag of a chain whose entries are `{name}`, which the chain\n\
             /// derives from where an entry stands.\n\
             #[doc(hidden)]\n\
             #[derive(Debug, Default, Clone, Copy)]\n\
             pub struct {overrides} {{\n{members}}}\n\n\
             #[allow(dead_code)]\n\
             impl {overrides} {{\n{setters}}}\n\n",
            name = self.name(),
            overrides = self.overrides_name(),
        )
    }

    /// Whether a codec the value holds (in a list, an embedded body or a
    /// variant) can refuse it: a refusal that comes from inside one has to be
    /// found before the first byte is written. A codec whose children cannot
    /// refuse has nothing to find, and skips the pass that looks. That is all
    /// the refusal analysis decides: what a caller sees is a `Result` for every
    /// codec.
    fn children_can_refuse(&self) -> bool {
        child_aliases(self.m()).into_iter().any(|alias| {
            self.input
                .imports
                .iter()
                .any(|i| i.alias == alias && i.codec_source_can_refuse)
        })
    }

    // ── flag-inputs ──────────────────────────────────────────────────

    /// The parameter a flag-input is passed as.
    fn input_name(input: &FlagInput) -> String {
        filters::to_snake_case(input.name.clone())
    }

    /// The flag-inputs the encode is handed: those the value does not say
    /// (docs/adr/0065 §6 R4a).
    fn given_inputs(&self) -> impl Iterator<Item = &FlagInput> {
        self.encode.given.iter().copied()
    }

    /// `, n: u8` for each flag-input the encode takes from its parent: a bit of
    /// the parent's own header that gates a field here, which the value does
    /// not say.
    fn input_params(&self) -> String {
        self.given_inputs()
            .map(|input| format!(", {}: u8", Self::input_name(input)))
            .collect()
    }

    /// `, n` for each flag-input the encode takes, passed on.
    fn input_args(&self) -> String {
        self.given_inputs()
            .map(|input| format!(", {}", Self::input_name(input)))
            .collect()
    }

    /// The truth of the flag-input `name` the value says, as an expression
    /// over `value` (a reference to a source of this codec).
    fn said_input(&self, name: &str, value: &str) -> String {
        match &self.encode.roles.inputs[name] {
            InputSource::Gate { field, when_clear } => {
                let field = self
                    .m()
                    .fields
                    .iter()
                    .find(|f| &f.id == field)
                    .expect("an input is said by a field of the codec");
                let present = format!(
                    "{}::{}({value}).is_some()",
                    self.trait_name,
                    self.accessor(field)
                );
                if *when_clear {
                    format!("!{present}")
                } else {
                    present
                }
            }
            InputSource::Forward {
                embed,
                alias,
                input,
                partly: _,
            } => {
                let field = self
                    .m()
                    .fields
                    .iter()
                    .find(|f| &f.id == embed)
                    .expect("an input is said by a body of the codec");
                let child = self
                    .child_of(alias)
                    .expect("`check` found the codec a body holds");
                format!(
                    "{}::input_{}({}::{}({value}))",
                    child.type_name,
                    filters::to_snake_case(input.clone()),
                    self.trait_name,
                    self.accessor(field)
                )
            }
            InputSource::Group => {
                let group = self
                    .encode
                    .roles
                    .group_of_gate(&GateVar::Input(name.to_string()))
                    .expect("an input a group says has its group");
                let held = format!(
                    "{}::{}({value})",
                    self.trait_name,
                    self.group_accessor(group)
                );
                self.gate_value(group, 0, &held)
            }
            InputSource::EveryArm { arms } => {
                let said: Vec<String> = self
                    .arms()
                    .iter()
                    .map(|arm| {
                        format!(
                            "{}{} => {}::input_{}(_b),",
                            arm.cfg_prefix(),
                            self.arm_pattern(arm),
                            arm.child.type_name,
                            filters::to_snake_case(arms[&arm.alias].clone())
                        )
                    })
                    .collect();
                format!(
                    "match {}::body({value}) {{ {} }}",
                    self.trait_name,
                    said.join(" ")
                )
            }
            InputSource::SomeArms { arms } => {
                let all_arms = self.arms();
                let mut said: Vec<String> = all_arms
                    .iter()
                    .filter(|arm| arms.contains_key(&arm.alias))
                    .map(|arm| {
                        let input = &arms[&arm.alias];
                        let read = format!(
                            "{}::input_{}(_b)",
                            arm.child.type_name,
                            filters::to_snake_case(input.clone())
                        );
                        let said = if self.child_says_input_partly(&arm.alias, input) {
                            read
                        } else {
                            format!("Some({read})")
                        };
                        format!("{}{} => {said},", arm.cfg_prefix(), self.arm_pattern(arm))
                    })
                    .collect();
                if said.len() < all_arms.len() {
                    said.push("_ => None,".to_string());
                }
                format!(
                    "match {}::body({value}) {{ {} }}",
                    self.trait_name,
                    said.join(" ")
                )
            }
            InputSource::List {
                list,
                input,
                partly,
                ..
            } => {
                let field = self
                    .m()
                    .fields
                    .iter()
                    .find(|f| &f.id == list)
                    .expect("an input is said by a list of the codec");
                let read = format!("{}::{}({value})", self.trait_name, self.accessor(field));
                if *partly {
                    format!(
                        "{read}.map(|_l| {})",
                        law_start(&self.encode.roles, list, input, "_l")
                    )
                } else {
                    law_start(&self.encode.roles, list, input, &read)
                }
            }
        }
    }

    /// Whether the list `field` is held to the law of its entries' inputs
    /// (docs/adr/0065 §6 R8).
    fn lawful(&self, field: &CodecField) -> bool {
        !list_laws(&self.encode.roles, &field.id).is_empty()
    }

    /// The call a source makes on the list `field` it holds for the walk it
    /// returns: `walk()` for a list held to its law, which keeps the law, and
    /// `iter()` for any other; their `list_` forms on a profile's list (`owned`).
    fn walk_of(&self, field: &CodecField, owned: bool) -> &'static str {
        match (self.lawful(field), owned) {
            (true, true) => "list_walk()",
            (true, false) => "walk()",
            (false, true) => "list_iter()",
            (false, false) => "iter()",
        }
    }

    /// The law of each list held to one, by its name, and the types that read
    /// each input the entries of the list say off an entry (docs/adr/0065 §6
    /// R8): one for each list and input, over every form of the entry, through
    /// its source.
    fn law_definitions(&self) -> String {
        let mut out = String::new();
        let mut named: BTreeSet<&str> = BTreeSet::new();
        for law in &self.encode.roles.list_laws {
            let BindSite::List { field } = &law.binder.site else {
                continue;
            };
            if !named.insert(field) {
                continue;
            }
            let (Some(name), Some(definition)) = (
                law_type(&self.encode.roles, self.name(), field),
                law_definition(&self.encode.roles, self.name(), field),
            ) else {
                continue;
            };
            out.push_str(&format!(
                "/// The law the entries of `{field}` follow in the inputs they say, which\n\
                 /// the list holds them to (docs/adr/0065 §6 R8).\n\
                 #[allow(dead_code)]\n\
                 pub type {name} = {definition};\n\n"
            ));
            let list = self
                .m()
                .fields
                .iter()
                .find(|f| &f.id == field)
                .expect("a list law names a list of the codec");
            out.push_str(
                &super::rust_lawful_list_definition(
                    &self.encode.roles,
                    self.m(),
                    list,
                    self.input.imports,
                    self.l(),
                )
                .expect("the owned form resolved the list's entry")
                .expect("a list with a law has a type at a profile"),
            );
        }
        let mut keys: BTreeSet<String> = BTreeSet::new();
        for law in &self.encode.roles.list_laws {
            let BindSite::List { field } = &law.binder.site else {
                continue;
            };
            let child = self
                .child_of(&law.binder.alias)
                .expect("`check` found the codec a list holds");
            let says = says_type(self.name(), field, &law.binder.input);
            let given = match &law.against {
                InputValue::Carrier { carrier, flag } => format!("`{carrier}.{flag}`"),
                InputValue::Input { name } => format!("the input `{name}`"),
                InputValue::PreviousEntry { id } => {
                    format!("whether the entry before has the identifier 0x{id:X}")
                }
            };
            // An entry that says the input in some of its values only answers
            // `None` from the others, which disagrees with no state (R8b).
            let read = format!(
                "{}::input_{}(entry)",
                child.type_name,
                filters::to_snake_case(law.binder.input.clone())
            );
            let said = if self.child_says_input_partly(&law.binder.alias, &law.binder.input) {
                read
            } else {
                format!("Some({read})")
            };
            out.push_str(&format!(
                "/// The input `{input}` each entry of `{field}` says, which the list hands\n\
                 /// every entry as {given} and holds to its law (docs/adr/0065 §6 R8).\n\
                 #[allow(dead_code)]\n\
                 #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n\
                 pub struct {says};\n\n\
                 impl<E: {trait_path} + ?Sized> {RUNTIME_CODEC}::Says<E> for {says} {{\n\
                 \x20   fn says(entry: &E) -> Option<bool> {{\n\
                 \x20       {said}\n\
                 \x20   }}\n\
                 }}\n\n",
                input = law.binder.input,
                trait_path = child.trait_path,
            ));
            // One kind for each identifier the chain's binds wait for.
            if let InputValue::PreviousEntry { id } = &law.against {
                if keys.insert(key_type(self.name(), field, *id)) {
                    out.push_str(&self.law_key(field, &child.trait_path, *id));
                }
            }
        }
        out
    }

    /// The kind of entry an input a chain supplies from the entry before waits
    /// for (docs/adr/0065 §6 R8b): an entry whose identifier is `id`, read off
    /// every form of the entry through its source, as a gate's kind is (R6).
    fn law_key(&self, chain_id: &str, entry_trait: &str, id: u64) -> String {
        let chain = self
            .m()
            .fields
            .iter()
            .find(|f| f.id == chain_id)
            .expect("a list law names a chain of the codec");
        let key_ty = key_type(self.name(), chain_id, id);
        let test = format!("{} == {id}u64", self.entry_identifier(chain, "entry"));
        format!(
            "/// The kind of entry the entry after which `{chain_id}` supplies its input\n\
             /// set: an entry whose identifier is 0x{id:X} (docs/adr/0065 §6 R8b).\n\
             #[allow(dead_code)]\n\
             #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n\
             pub struct {key_ty};\n\n\
             impl<E: {entry_trait} + ?Sized> {RUNTIME_CODEC}::Key<E> for {key_ty} {{\n\
             \x20   fn matches(entry: &E) -> bool {{\n\
             \x20       {test}\n\
             \x20   }}\n\
             }}\n\n"
        )
    }

    /// The pattern that binds `_b` to the body of `arm` in the value of the
    /// variant.
    fn arm_pattern(&self, arm: &Arm) -> String {
        let enum_name = self.body_enum();
        if arm.value.is_some() {
            format!("{enum_name}::{}(_b)", arm.variant_name)
        } else {
            format!("{enum_name}::Default {{ body: _b, .. }}")
        }
    }

    /// `input_{i}`: each flag-input the value says, read off a source
    /// (docs/adr/0065 §6 R4a). A parent derives a flag it binds to the input
    /// from it, and holds a value it hands the input from elsewhere to it.
    fn input_functions(&self) -> String {
        let mut out = String::new();
        for said in &self.encode.said {
            let input = said.input;
            let by = match &self.encode.roles.inputs[&input.name] {
                InputSource::Gate {
                    field,
                    when_clear: false,
                } => format!("set exactly when `{field}` is present"),
                InputSource::Gate {
                    field,
                    when_clear: true,
                } => format!("set exactly when `{field}` is absent"),
                InputSource::Forward { embed, input, .. } => {
                    format!("the input `{input}` that the body `{embed}` says")
                }
                InputSource::Group => {
                    format!(
                        "set exactly when the value holds the side of `{}` present while it holds",
                        input.name
                    )
                }
                InputSource::EveryArm { .. } => "the input the arm it holds says".to_string(),
                InputSource::SomeArms { .. } => {
                    "the input the arm it holds says, and `None` from an arm that does not read \
                     it"
                    .to_string()
                }
                InputSource::List { list, partly, .. } => {
                    let start = format!("the start of `{list}`, which every entry of it says");
                    if *partly {
                        format!("{start}, and `None` while the list is absent")
                    } else {
                        start
                    }
                }
            };
            let answer = if self.encode.roles.says_input_partly(&input.name) {
                "Option<bool>"
            } else {
                "bool"
            };
            out.push_str(&format!(
                "\x20   /// The flag-input `{name}` that `src` says: {by}.\n\
                 \x20   /// A parent hands the decode this bit and derives it from here for the\n\
                 \x20   /// encode.\n\
                 \x20   pub fn input_{snake}<S: {trait_name} + ?Sized>(src: &S) -> {answer} {{\n\
                 \x20       {expr}\n\
                 \x20   }}\n\n",
                name = input.name,
                snake = Self::input_name(input),
                trait_name = self.trait_name,
                expr = self.said_input(&input.name, "src"),
            ));
        }
        out
    }

    /// Hold the input a held codec says to the value the parent hands it from
    /// elsewhere ([`codec_roles::InputCheck`]), in the prologue, so a refusal
    /// leaves the sink untouched.
    fn input_check(
        &self,
        check: &encode_plan::InputCheck<'_>,
        derived: &Derived,
        lines: &mut Vec<String>,
    ) {
        let child = self.child_of_plan(&check.codec);
        // Whether `entry` says the input otherwise than `value`. A value that says
        // nothing of it disagrees with nothing (docs/adr/0065 §6 R4e).
        let says_otherwise = |entry: &str, value: &str| {
            let read = format!(
                "{}::input_{}({entry})",
                child.type_name,
                filters::to_snake_case(check.input.clone())
            );
            if check.partly {
                format!("{read}.is_some_and(|_s| _s != {value})")
            } else {
                format!("{read} != {value}")
            }
        };
        let refuse = self.refuse(RefusalKind::PresentIfMismatch, &check.at);
        let handed = match &check.against {
            Against::Carrier { carrier, bit } => {
                format!(
                    "({} & 0x{:X}) != 0",
                    self.read(carrier, derived),
                    1u64 << bit
                )
            }
            Against::Input { name } => format!("{} != 0", filters::to_snake_case(name.clone())),
            Against::PreviousEntry { id } => {
                let CheckSite::List { list: chain } = &check.site else {
                    unreachable!("a chain supplies an input from the entry before");
                };
                let walk = [
                    format!("    if {} {{", says_otherwise("&_e", "_prev")),
                    format!("        return Err({refuse});"),
                    "    }".to_string(),
                    format!(
                        "    _prev = {} == {id}u64;",
                        self.entry_identifier(chain, "&_e")
                    ),
                    "}".to_string(),
                ];
                lines.push("{".to_string());
                lines.push("    let mut _prev: bool = false;".to_string());
                let read = self.read_src(chain);
                let entries = if chain.present_if.is_some() {
                    format!("{read}.into_iter().flatten()")
                } else {
                    read
                };
                lines.push(format!("    for _e in {entries} {{"));
                lines.extend(walk.iter().map(|line| format!("    {line}")));
                lines.push("}".to_string());
                return;
            }
        };
        let differs = match &check.site {
            CheckSite::List { list } => {
                let read = self.read_src(list);
                let differs = says_otherwise("&_e", "_h");
                if list.present_if.is_some() {
                    format!("{read}.is_some_and(|mut _l| _l.any(|_e| {differs}))")
                } else {
                    format!("{read}.any(|_e| {differs})")
                }
            }
            CheckSite::Embed { body } => {
                let read = self.read_src(body);
                if body.present_if.is_some() {
                    format!("{read}.is_some_and(|_b| {})", says_otherwise("_b", "_h"))
                } else {
                    says_otherwise(&read, "_h")
                }
            }
            CheckSite::Arm { arms: held } => {
                let all = self.arms();
                let mut arms: Vec<String> = held
                    .iter()
                    .map(|&index| {
                        let arm = &all[index];
                        format!(
                            "{}{} => {},",
                            arm.cfg_prefix(),
                            self.arm_pattern(arm),
                            says_otherwise("_b", "_h")
                        )
                    })
                    .collect();
                arms.push("_ => false,".to_string());
                format!("match src.body() {{ {} }}", arms.join(" "))
            }
        };
        lines.push(format!(
            "{{\n    let _h: bool = {handed};\n    if {differs} {{\n        return Err({refuse});\n    }}\n}}"
        ));
    }

    /// Whether the import `alias` says its input `input` in some of its values
    /// only, so that reading it answers an `Option`.
    fn child_says_input_partly(&self, alias: &str, input: &str) -> bool {
        self.input
            .imports
            .iter()
            .any(|i| i.alias == alias && i.codec_partly_says_inputs.iter().any(|s| s == input))
    }

    /// The arguments a parent passes the codec `alias`, one for each flag-input
    /// it takes, in the order it declares them: the bits of the parent's own
    /// carrier that the import binds, or the parent's own input passed on.
    fn thread_args(&self, alias: &str, derived: &Derived) -> String {
        self.thread_args_supplying(alias, derived, &|_| None)
    }

    /// [`Self::thread_args`] for a caller that supplies some inputs itself: a
    /// chain passes the entry it writes an input of its own making, in place of
    /// the one the import binds.
    fn thread_args_supplying(
        &self,
        alias: &str,
        derived: &Derived,
        supplied: &dyn Fn(&str) -> Option<String>,
    ) -> String {
        let Some(handed) = self.encode.handed.get(alias) else {
            return String::new();
        };
        let mut out = String::new();
        for handed in handed {
            if let Some(arg) = supplied(&handed.input) {
                out.push_str(&format!(", {arg}"));
                continue;
            }
            match &handed.from {
                // Nothing the parent binds: a caller supplies it, or no one does.
                None => continue,
                Some(HandedFrom::Carrier {
                    carrier,
                    bit,
                    width,
                }) => {
                    let mask = (1u64 << width) - 1;
                    let read = self.read(carrier, derived);
                    let bits = format!("{} & 0x{mask:X}", shr(&read, *bit));
                    if carrier.fixed_bits().is_some_and(|b| b <= 8) {
                        out.push_str(&format!(", {bits}"));
                    } else {
                        out.push_str(&format!(", ({bits}) as u8"));
                    }
                }
                Some(HandedFrom::Input { name }) => {
                    out.push_str(&format!(", {}", filters::to_snake_case(name.clone())));
                }
            }
        }
        out
    }

    // ── gates that are checked ───────────────────────────────────────

    /// The test a gate states, over what the source presents and what the
    /// codec is passed: each clause reads a bit, and the clauses are joined
    /// as the author joined them.
    fn gate_test(&self, gate: &PresentIfPredicate, derived: &Derived) -> String {
        let bit = |read: String, mask: u64, negate: bool| {
            let op = if negate { "==" } else { "!=" };
            format!("({read} & 0x{mask:X}) {op} 0")
        };
        self.encode
            .clauses(gate)
            .iter()
            .map(|clause| match clause {
                // The chain holds an entry of the identifier, as the entries the
                // source lists say.
                Clause::Member {
                    chain,
                    id,
                    negate,
                    held,
                } => {
                    let has = self.chain_has(chain, *id, *held);
                    if *negate {
                        format!("!{has}")
                    } else {
                        has
                    }
                }
                Clause::Input { name, negate } => {
                    bit(filters::to_snake_case(name.clone()), 1, *negate)
                }
                Clause::Flag {
                    carrier,
                    bit: at,
                    negate,
                } => bit(self.read(carrier, derived), 1u64 << at, *negate),
            })
            .collect::<Vec<_>>()
            .join(" || ")
    }

    /// The identifier of the entry `entry` (an expression that is a reference
    /// to the entry's source) of the chain `chain`, as a `u64`: the flag or
    /// field the chain's `entry-id` names, read through the entry codec's trait,
    /// less the flags it leaves out.
    fn entry_identifier(&self, chain: &CodecField, entry: &str) -> String {
        let identifier = self
            .encode
            .entry_ids
            .get(&chain.id)
            .expect("a chain-membership clause names a chain that declares entry-id");
        let entry_trait = self
            .child(chain)
            .expect("a chain imports its entry codec")
            .trait_path;
        let whole = |field: &str| {
            format!(
                "u64::from({entry_trait}::{}({entry}))",
                self.l().codec_field_id(field)
            )
        };
        match identifier {
            EntryId::Flag { carrier, bit, mask } => {
                format!("({} & 0x{mask:X})", shr(&whole(carrier), *bit))
            }
            EntryId::Whole {
                field,
                mask: Some(mask),
            } => format!("({} & 0x{mask:X})", whole(field)),
            EntryId::Whole { field, mask: None } => whole(field),
        }
    }

    /// "The chain `chain` holds an entry of the kind `value`", where `held` is
    /// the group that holds its memberships and the member the kind is.
    fn chain_has(
        &self,
        chain: &CodecField,
        value: u64,
        held: Option<(usize, Option<usize>)>,
    ) -> String {
        // A chain whose memberships a group holds: the gate on one of the
        // group's kinds is the arm the value holds, and any other kind is
        // looked for in its walk (docs/adr/0065 §6 R6). A bare entry's chain is
        // walked as slots, whose named entry has the group's key; one with
        // content is walked as the chain it is, its entries read as any chain's.
        let membership = held.map(|(group, _)| &self.encode.groups()[group]);
        if let (Some(group), Some((_, Some(index)))) = (membership, held) {
            let held = format!("src.{}()", self.group_accessor(group));
            return self.gate_value(group, index, &held);
        }
        let found = if membership.is_some_and(GateGroup::member_is_bare) {
            format!(
                "match &_e {{ {RUNTIME_CODEC}::Slot::Entry(_n) => {} == {value}u64, {RUNTIME_CODEC}::Slot::Named => false }}",
                self.entry_identifier(chain, "_n")
            )
        } else {
            format!("{} == {value}u64", self.entry_identifier(chain, "&_e"))
        };
        let read = self.read_src(chain);
        let held = membership.is_some();
        // A chain that is gated off holds no entry, and one a group holds is
        // read from the arm, where it may be absent.
        if chain.present_if.is_some() || held {
            format!("{read}.is_some_and(|mut _l| _l.any(|_e| {found}))")
        } else {
            format!("{read}.any(|_e| {found})")
        }
    }

    /// Each gate the codec cannot derive the presence from, held to the source:
    /// the field is present exactly when the gate says, or the value is not one
    /// frame can carry. Made in the prologue, so nothing is written first.
    fn gate_checks(&self, derived: &Derived, lines: &mut Vec<String>) {
        for check in &self.encode.input_checks {
            self.input_check(check, derived, lines);
        }
        for field in &self.encode.gate_checks {
            let Some(gate) = &field.present_if else {
                continue;
            };
            lines.push(format!(
                "if ({}) != {}.is_some() {{\n    return Err({});\n}}",
                self.gate_test(gate, derived),
                self.read_src(field),
                self.refuse(RefusalKind::PresentIfMismatch, &field.id),
            ));
        }
    }

    fn m(&self) -> &CodecModel {
        self.input.m
    }

    fn l(&self) -> &LangCtx {
        self.input.l
    }

    fn name(&self) -> &str {
        self.input.struct_name
    }

    fn lt(&self) -> &str {
        self.input.struct_lifetime
    }

    /// The accessor a field is read through.
    fn accessor(&self, field: &CodecField) -> String {
        self.l().codec_field_id(&field.id)
    }

    /// What the encode reads `field` as: the source's accessor, or, for an
    /// element of a group (docs/adr/0065 §6 R4b), the local the encode bound
    /// it to from the group, which is `None` on the side the value does not
    /// hold. A list is an iterator, so each read takes a copy of it.
    fn read_src(&self, field: &CodecField) -> String {
        if self.encode.roles.group_of(&field.id).is_some() {
            let local = self.member_local(field);
            if field.is_repeat() || field.is_tlv_chain() {
                format!("{local}.clone()")
            } else {
                local
            }
        } else {
            format!("src.{}()", self.accessor(field))
        }
    }

    /// The local a group's element is bound to in the encode.
    fn member_local(&self, field: &CodecField) -> String {
        format!("_m_{}", self.accessor(field))
    }

    /// The accessor and the field a form holds a group under.
    fn group_accessor(&self, group: &GateGroup) -> String {
        group_accessor(group)
    }

    fn group_type(&self, group: &GateGroup) -> String {
        group_type(self.name(), group)
    }

    /// The path an arm of a group is built and matched by: the enum's variant,
    /// or the struct of a group of one arm.
    fn arm_path(&self, group: &GateGroup, arm: &GroupArm) -> String {
        let ty = self.group_type(group);
        if group.is_sum() {
            format!("{ty}::{}", group_arm_name(group, arm))
        } else {
            ty
        }
    }

    /// `pattern` as what a form holds: inside `Some` for a group a value may
    /// hold no arm of.
    fn held(group: &GateGroup, pattern: &str) -> String {
        if group.absent.is_some() {
            format!("Some({pattern})")
        } else {
            pattern.to_string()
        }
    }

    fn group_fields<'s>(&'s self, ids: &'s [String]) -> impl Iterator<Item = &'s CodecField> {
        ids.iter()
            .filter_map(|id| self.m().fields.iter().find(|f| &f.id == id))
    }

    /// The list `field` shares its count with in `arm`, when the arm holds the
    /// two as one (docs/adr/0065 §6 R5).
    fn second_of<'s>(
        &'s self,
        group: &GateGroup,
        arm: &GroupArm,
        field: &CodecField,
    ) -> Option<&'s CodecField> {
        let second = group.second_of(arm, &field.id)?;
        self.m().fields.iter().find(|f| f.id == second)
    }

    /// The elements of `arm` as a form holds them: its fields, less a list held
    /// with the list it shares its count with.
    fn arm_held<'s>(
        &'s self,
        group: &'s GateGroup,
        arm: &'s GroupArm,
    ) -> impl Iterator<Item = &'s CodecField> {
        self.group_fields(&arm.fields)
            .filter(|f| !group.is_second_in(arm, &f.id))
    }

    /// The name `field`, an element of `arm` ([`Self::arm_held`]), goes by there.
    fn held_name(&self, group: &GateGroup, arm: &GroupArm, field: &CodecField) -> String {
        match self.second_of(group, arm, field) {
            Some(second) => aligned_name(&self.accessor(field), &self.accessor(second)),
            None => self.accessor(field),
        }
    }

    /// The group's elements, each once.
    fn group_members(&self, group: &GateGroup) -> Vec<&CodecField> {
        group
            .fields()
            .into_iter()
            .filter_map(|id| self.m().fields.iter().find(|f| f.id == id))
            .collect()
    }

    /// The value of the gate `index` of `group` while the value holds what
    /// `held` (an expression of what the source presents of the group) holds:
    /// the value the arm gives it, or the one it has with no arm.
    fn gate_value(&self, group: &GateGroup, index: usize, held: &str) -> String {
        let absent = group.absent.as_ref().map(|a| a[index]);
        let arms_with = |value: bool| -> Vec<&GroupArm> {
            group
                .arms
                .iter()
                .filter(|a| a.values[index] == value)
                .collect()
        };
        let pattern = |arms: &[&GroupArm]| {
            arms.iter()
                .map(|a| format!("{} {{ .. }}", self.arm_path(group, a)))
                .collect::<Vec<_>>()
                .join(" | ")
        };
        // Name the arms the gate holds in, or, where it holds with no arm too,
        // the ones it does not.
        let (arms, negate) = if absent == Some(true) {
            (arms_with(false), true)
        } else {
            (arms_with(true), false)
        };
        if arms.is_empty() {
            return negate.to_string();
        }
        let test = match (group.is_sum(), group.absent.is_some()) {
            (true, true) => format!("matches!({held}, Some({}))", pattern(&arms)),
            (true, false) => format!("matches!({held}, {})", pattern(&arms)),
            // A group of one arm, which the value holds or not.
            (false, _) if negate => return format!("{held}.is_none()"),
            (false, _) => return format!("{held}.is_some()"),
        };
        if negate {
            format!("!{test}")
        } else {
            test
        }
    }

    fn member_varies(field: &CodecField) -> bool {
        member_varies(field)
    }

    /// The type parameter of a group that is `field`'s type.
    fn member_param(field: &CodecField) -> String {
        filters::to_pascal_case(field.id.clone())
    }

    /// `group_type` with each varying element's type as `ty` says it, the
    /// group held in one form.
    fn group_type_in(&self, group: &GateGroup, ty: &dyn Fn(&CodecField) -> String) -> String {
        let args: Vec<String> = self
            .group_members(group)
            .into_iter()
            .filter(|f| Self::member_varies(f))
            .map(ty)
            .collect();
        if args.is_empty() {
            self.group_type(group)
        } else {
            format!("{}<{}>", self.group_type(group), args.join(", "))
        }
    }

    /// `group_type_in` wrapped as a form holds it: as it is, or as an `Option`
    /// for a group a value may hold no arm of.
    fn group_held_in(&self, group: &GateGroup, ty: &dyn Fn(&CodecField) -> String) -> String {
        let inner = self.group_type_in(group, ty);
        if group.absent.is_some() {
            format!("Option<{inner}>")
        } else {
            inner
        }
    }

    /// The type of an element of a group while it is there: what the source
    /// presents (`value_type`), a stated length as its number.
    fn member_value_type(&self, field: &CodecField) -> String {
        self.value_type(field)
    }

    /// The definition of each group's type.
    fn group_definitions(&self) -> String {
        let mut out = String::new();
        for group in &self.encode.roles.groups {
            let members: Vec<&CodecField> = self.group_members(group);
            let params: Vec<String> = members
                .iter()
                .filter(|f| Self::member_varies(f))
                .map(|f| Self::member_param(f))
                .collect();
            let generics = if params.is_empty() {
                String::new()
            } else {
                format!("<{}>", params.join(", "))
            };
            // The chain of the memberships is the one list in every arm, refined
            // as the arm says (docs/adr/0065 §6 R6a).
            let member_ty = |f: &CodecField, arm: &GroupArm| {
                if holds_chain(group, f) {
                    chain_arm_type(
                        self.name(),
                        &self.m().fields,
                        group,
                        f,
                        arm,
                        &Self::member_param(f),
                    )
                } else if Self::member_varies(f) {
                    Self::member_param(f)
                } else {
                    self.l().type_name(&f.sce_type).to_string()
                }
            };
            let fields_of = |arm: &GroupArm, indent: &str| -> String {
                self.arm_held(group, arm)
                    .map(|f| {
                        // Two lists of one count are one element of the arm
                        // (docs/adr/0065 §6 R5).
                        let (what, ty) = match self.second_of(group, arm, f) {
                            Some(second) => (
                                format!("`{}` and `{}` lists, of one count", f.id, second.id),
                                format!(
                                    "{RUNTIME_CODEC}::Aligned<{}, {}>",
                                    member_ty(f, arm),
                                    member_ty(second, arm)
                                ),
                            ),
                            None => (format!("`{}` field", f.id), member_ty(f, arm)),
                        };
                        format!(
                            "{indent}/// The {what}.\n{indent}{vis}{acc}: {ty},\n",
                            vis = if indent.len() == 4 { "pub " } else { "" },
                            acc = self.held_name(group, arm, f),
                        )
                    })
                    .collect()
            };
            let name = self.group_type(group);
            let gates = group
                .gates
                .iter()
                .map(|g| format!("`{g}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let none = match &group.absent {
                Some(values) => format!(
                    " A value holds no arm while {}.",
                    arm_condition(group, values)
                ),
                None => String::new(),
            };
            if group.is_sum() {
                let mut arms = String::new();
                for arm in &group.arms {
                    arms.push_str(&format!(
                        "    /// While {}.\n    {} {{\n{}    }},\n",
                        arm_condition(group, &arm.values),
                        group_arm_name(group, arm),
                        fields_of(arm, "        "),
                    ));
                }
                out.push_str(&format!(
                    "/// The fields {gates} decide (docs/adr/0065 §6 R4b, R4c): a value holds the\n\
                     /// ones of one arm, and `{codec}` derives the gates from which.{none}\n\
                     #[allow(dead_code)]\n\
                     #[derive(Debug, Clone, Copy, PartialEq)]\n\
                     pub enum {name}{generics} {{\n{arms}}}\n\n",
                    codec = self.name(),
                ));
                // A form holds a group a value always holds an arm of as it is,
                // and starts it at the arm where every gate is clear.
                let always_present_chain = group
                    .chain()
                    .and_then(|chain| self.m().fields.iter().find(|f| f.id == chain))
                    .filter(|chain| chain.present_if.is_none());
                if group.absent.is_none() {
                    out.push_str(&match always_present_chain {
                        // A chain that is always present holds one entry or
                        // more, and its default the default entries, whose
                        // kinds say the arm (docs/adr/0065 §6 R6b).
                        Some(chain) if !group.member_is_bare() => {
                            self.chain_default(group, chain, &name, &generics, &params)
                        }
                        // The one gate on a bare entry starts where it holds,
                        // with the named entry alone (R6a).
                        Some(chain) => self.arm_default(
                            group,
                            &group.arms[0],
                            &name,
                            &generics,
                            &params,
                            Some(chain),
                        ),
                        None => self.arm_default(
                            group,
                            group
                                .arms
                                .iter()
                                .find(|a| a.values.iter().all(|v| !v))
                                .unwrap_or(&group.arms[0]),
                            &name,
                            &generics,
                            &params,
                            None,
                        ),
                    });
                }
                if let Some(chain) = group.chain() {
                    for (_, key) in group.members() {
                        out.push_str(&self.membership_key(group, chain, key));
                    }
                }
            } else {
                let arm = &group.arms[0];
                out.push_str(&format!(
                    "/// The fields {gates} decide (docs/adr/0065 §6 R4b), present together\n\
                     /// while {condition}: a value holds all of them or none, and `{codec}`\n\
                     /// derives the gate from which.\n\
                     #[allow(dead_code)]\n\
                     #[derive(Debug, Clone, Copy, PartialEq)]\n\
                     pub struct {name}{generics} {{\n{fields}}}\n\n",
                    condition = arm_condition(group, &arm.values),
                    codec = self.name(),
                    fields = fields_of(arm, "    "),
                ));
            }
        }
        out
    }

    /// The `Default` of the group `group`, the type `name` with `generics`
    /// (the parameters `params`): the arm `arm`, each field at its type's
    /// default. `bare_chain` is the chain the one gate on a bare entry holds
    /// where it holds, whose default is the named entry alone (docs/adr/0065 §6
    /// R6a).
    fn arm_default(
        &self,
        group: &GateGroup,
        arm: &GroupArm,
        name: &str,
        generics: &str,
        params: &[String],
        bare_chain: Option<&CodecField>,
    ) -> String {
        let default_params = params
            .iter()
            .map(|p| match bare_chain {
                Some(chain) if *p == Self::member_param(chain) => {
                    format!("{p}: Default + {RUNTIME_CODEC}::Entries")
                }
                _ => format!("{p}: Default"),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let default_fields: String = self
            .arm_held(group, arm)
            .map(|f| format!("{}: Default::default(), ", self.held_name(group, arm, f)))
            .collect();
        format!(
            "impl{default_generics} Default for {name}{generics} {{\n\
             \x20   /// The arm held while {condition}, each field at its type's default.\n\
             \x20   fn default() -> Self {{\n\
             \x20       Self::{arm_name} {{ {default_fields}}}\n\
             \x20   }}\n\
             }}\n\n",
            default_generics = if params.is_empty() {
                String::new()
            } else {
                format!("<{default_params}>")
            },
            condition = arm_condition(group, &arm.values),
            arm_name = group_arm_name(group, arm),
        )
    }

    /// The `Default` of the group `group` of the memberships of `chain`, which
    /// is always present (docs/adr/0065 §6 R6b), the type `name` with
    /// `generics` (the parameters `params`): the chain's own default, one entry
    /// or more, in the arm its entries' kinds select among those where every
    /// other gate is clear, each field at its type's default. The kinds of the
    /// default entries are the entry codec's to say (a variant's default arm
    /// can set the identifier), so they are asked of the entries as a decode
    /// asks them, and the arm found is the one whose refinement they satisfy.
    fn chain_default(
        &self,
        group: &GateGroup,
        chain: &CodecField,
        name: &str,
        generics: &str,
        params: &[String],
    ) -> String {
        let list = Self::member_param(chain);
        let bounded = format!(
            "{RUNTIME_CODEC}::Bounded<{list}, {}>",
            super::rust_list_bound_args(chain, &self.m().fields)
        );
        let keys: Vec<String> = group
            .members()
            .map(|(_, key)| key_type(self.name(), &chain.id, key))
            .collect();
        let default_params = params
            .iter()
            .map(|p| {
                if *p == list {
                    p.clone()
                } else {
                    format!("{p}: Default")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let of_kinds: String = keys
            .iter()
            .map(|k| format!(" + {RUNTIME_CODEC}::OfKind<{k}>"))
            .collect();
        let tests = keys
            .iter()
            .map(|k| {
                format!("<{list} as {RUNTIME_CODEC}::OfKind<{k}>>::first_of_kind(&_list).is_some()")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let others_clear = |arm: &&GroupArm| {
            group
                .gates
                .iter()
                .zip(&arm.values)
                .all(|(gate, value)| matches!(gate, GateVar::Member { .. }) || !value)
        };
        let cases: String = group
            .arms
            .iter()
            .filter(others_clear)
            .map(|arm| {
                let values = group
                    .members()
                    .map(|(index, _)| arm.values[index].to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let fields = self
                    .group_fields(&arm.fields)
                    .map(|f| {
                        if holds_chain(group, f) {
                            format!(
                                "{}: {RUNTIME_CODEC}::Chain::new(_list).expect(\"the default entries are of the kinds found\")",
                                self.accessor(f)
                            )
                        } else {
                            format!("{}: Default::default()", self.accessor(f))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "            ({values},) => Self::{} {{ {fields} }},\n",
                    group_arm_name(group, arm)
                )
            })
            .collect();
        format!(
            "impl<{default_params}> Default for {name}{generics}\n\
             where\n\
             \x20   {bounded}: Default,\n\
             \x20   {list}: {RUNTIME_CODEC}::Entries{of_kinds},\n\
             {{\n\
             \x20   /// The chain's default, in the arm its entries' kinds select with every\n\
             \x20   /// other gate clear, each field at its type's default.\n\
             \x20   fn default() -> Self {{\n\
             \x20       let _list = <{bounded} as Default>::default().into_inner();\n\
             \x20       match ({tests},) {{\n{cases}        }}\n\
             \x20   }}\n\
             }}\n\n"
        )
    }

    /// The type that names the kind of entry the membership `group` of the
    /// chain `chain_id` asks for (`key`), with the entry of the kind, whose
    /// identifier is all of its value (docs/adr/0065 §6 R6a): the kind is read
    /// off every form of the entry through its source, as a gate reads it, and
    /// the entry is the decoded entry that holds the identifier and the arm it
    /// selects, which presents nothing.
    fn membership_key(&self, group: &GateGroup, chain_id: &str, key: u64) -> String {
        if !group.member_is_bare() {
            return self.membership_kind(group, chain_id, key);
        }
        let chain = self
            .m()
            .fields
            .iter()
            .find(|f| f.id == chain_id)
            .expect("a membership names a chain of the codec");
        let import = self
            .input
            .imports
            .iter()
            .find(|i| Some(i.alias.as_str()) == child_alias(chain))
            .expect("a chain imports its entry codec");
        let key_ty = key_type(self.name(), chain_id, key);
        let entry_ty = format!("super::{}::{}", import.namespace, import.type_name);
        let lifetime = if import.codec_is_borrowed {
            "<'static>"
        } else {
            ""
        };
        // A bare entry is its carrier and, where it has one, the arm of its
        // variant the identifier's tag bits select (`named_entry_is_bare`).
        let carrier = import
            .codec_fields
            .first()
            .expect("a bare entry is its carrier");
        let body = match &import.codec_variant {
            None => String::new(),
            Some(variant) => {
                let flag = carrier
                    .flags
                    .iter()
                    .find(|f| variant.tag_flag.as_deref() == Some(f.name.as_str()))
                    .expect("a bare entry's tag is a flag of its carrier");
                let mask = if flag.width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << flag.width) - 1
                };
                let tag = key.checked_shr(flag.bit).unwrap_or(0) & mask;
                let arm = variant
                    .arms
                    .iter()
                    .find(|a| a.value == tag)
                    .expect("a bare entry's tag selects an arm");
                format!(
                    ", body: super::{}::{}Variant::{}(Default::default())",
                    import.namespace,
                    import.type_name,
                    filters::to_pascal_case(arm.body_alias.clone())
                )
            }
        };
        format!(
            "{}#[allow(dead_code)]\n\
             impl {key_ty} {{\n\
             \x20   /// The entry of the kind: its identifier, and nothing else to say.\n\
             \x20   pub fn named() -> {entry_ty}{lifetime} {{\n\
             \x20       {entry_ty} {{ {carrier}: 0x{key:X}{body} }}\n\
             \x20   }}\n\
             }}\n\n",
            self.membership_kind(group, chain_id, key),
            carrier = self.l().codec_field_id(&carrier.id),
        )
    }

    /// The type that names the kind of entry the membership `group` of the
    /// chain `chain_id` asks for (`key`), read off every form of the entry
    /// through its source as a gate reads it (docs/adr/0065 §6 R6).
    fn membership_kind(&self, group: &GateGroup, chain_id: &str, key: u64) -> String {
        let chain = self
            .m()
            .fields
            .iter()
            .find(|f| f.id == chain_id)
            .expect("a membership names a chain of the codec");
        let import = self
            .input
            .imports
            .iter()
            .find(|i| Some(i.alias.as_str()) == child_alias(chain))
            .expect("a chain imports its entry codec");
        let key_ty = key_type(self.name(), chain_id, key);
        let entry_trait = format!("super::{}::{}Source", import.namespace, import.type_name);
        let test = format!("{} == {key}u64", self.entry_identifier(chain, "entry"));
        let what = if group.member_is_bare() {
            "which is all of its value (docs/adr/0065 §6 R6a)"
        } else {
            "which a value holds where it stands (docs/adr/0065 §6 R6b)"
        };
        format!(
            "/// The kind of entry `{chain_id}.has(0x{key:X})` names: an entry whose identifier is\n\
             /// 0x{key:X}, {what}.\n\
             #[allow(dead_code)]\n\
             #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n\
             pub struct {key_ty};\n\n\
             impl<E: {entry_trait}> {RUNTIME_CODEC}::Key<E> for {key_ty} {{\n\
             \x20   fn matches(entry: &E) -> bool {{\n\
             \x20       {test}\n\
             \x20   }}\n\
             }}\n\n"
        )
    }

    /// How the form `form` (`None` for the parts) reads the chain of the group
    /// `group` of its memberships in `arm`, `r` being a reference to it: the
    /// refined chain as its walk, the refinement kept (docs/adr/0065 §6 R6a).
    fn chain_member_read(
        &self,
        form: Option<Form>,
        group: &GateGroup,
        field: &CodecField,
        arm: &GroupArm,
        r: &str,
    ) -> String {
        let walk = if form == Some(Form::Owned) {
            "list_iter()"
        } else {
            "iter()"
        };
        if !chain_optional_in(group, field, arm) {
            format!("{r}.{walk}")
        } else {
            format!("{r}.as_ref().map(|_e| _e.{walk})")
        }
    }

    /// How the form `form` (`None` for the parts) reads an element of a group
    /// it holds, `r` being a reference to the element, as the source presents
    /// it: a list's entries with their count, a body lent, text and bytes as
    /// slices, a scalar copied.
    fn member_read(&self, form: Option<Form>, field: &CodecField, r: &str) -> String {
        let storage = RUNTIME_CODEC;
        if field.is_embed() {
            return match form {
                None => format!("*{r}"),
                Some(_) => r.to_string(),
            };
        }
        if field.is_repeat() || field.is_tlv_chain() {
            let lawful = self.lawful(field);
            return match form {
                Some(Form::Builder) if codec_totality::list_least(field) == 0 && !lawful => {
                    format!("{storage}::BoundedIter::from({r})")
                }
                Some(Form::Owned) => format!("{r}.{}", self.walk_of(field, true)),
                _ => format!("{r}.{}", self.walk_of(field, false)),
            };
        }
        match (form, &field.sce_type) {
            (Some(Form::Owned), SceType::Bytes | SceType::String) => self.owned_text_read(field, r),
            _ => format!("*{r}"),
        }
    }

    /// How the owned form reads its `bytes` or `string` field `field` held at
    /// `place`: through the `Bounded` around its container, which lends the
    /// bytes with their bounds and checks nothing (docs/adr/0077 §3), a field
    /// with no bound at `{ usize::MAX }` (§7).
    fn owned_text_read(&self, field: &CodecField, place: &str) -> String {
        if field.is_string() {
            format!("{place}.as_text()")
        } else {
            format!("{place}.as_bytes()")
        }
    }

    /// [`Self::owned_text_read`] as the path of a function of a reference to
    /// the container, for an `Option` the owned form holds a gated field in.
    fn owned_text_read_fn(&self, field: &CodecField) -> String {
        let storage = RUNTIME_CODEC;
        if field.is_string() {
            format!("{storage}::Bounded::as_text")
        } else {
            format!("{storage}::Bounded::as_bytes")
        }
    }

    /// The body of a form's accessor of `group`: the group it holds, each
    /// element read as the source presents it. A group of scalars alone is
    /// the same in every form and is copied whole.
    fn group_read(&self, group: &GateGroup, form: Option<Form>) -> String {
        let accessor = self.group_accessor(group);
        if !self
            .group_members(group)
            .iter()
            .any(|f| Self::member_varies(f))
        {
            return format!("self.{accessor}");
        }
        // Each arm as the pattern that takes it apart and the arm built from
        // its elements read.
        let arms: Vec<(String, String)> = group
            .arms
            .iter()
            .map(|arm| {
                let fields: Vec<&CodecField> = self.arm_held(group, arm).collect();
                let names = fields
                    .iter()
                    .map(|f| self.held_name(group, arm, f))
                    .collect::<Vec<_>>()
                    .join(", ");
                let reads = fields
                    .iter()
                    .map(|f| {
                        let accessor = self.held_name(group, arm, f);
                        let read = if holds_chain(group, f) {
                            self.chain_member_read(form, group, f, arm, &accessor)
                        } else if let Some(second) = self.second_of(group, arm, f) {
                            // Each list read as the form reads a list, the two
                            // walks of the one count they were held to.
                            format!(
                                "{RUNTIME_CODEC}::Aligned::new({}, {}).expect(\"a walk keeps its list's count\")",
                                self.member_read(form, f, &format!("{accessor}.first()")),
                                self.member_read(form, second, &format!("{accessor}.second()")),
                            )
                        } else {
                            self.member_read(form, f, &accessor)
                        };
                        shorthand(&accessor, &read)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let path = self.arm_path(group, arm);
                (
                    format!("{path} {{ {names} }}"),
                    format!("{path} {{ {reads} }}"),
                )
            })
            .collect();
        if !group.is_sum() {
            let (pattern, value) = &arms[0];
            return format!("self.{accessor}.as_ref().map(|{pattern}| {value})");
        }
        let arms: String = arms
            .iter()
            .map(|(pattern, value)| format!("            {pattern} => {value},\n"))
            .collect();
        if group.absent.is_some() {
            format!("self.{accessor}.as_ref().map(|_g| match _g {{\n{arms}        }})")
        } else {
            format!("match &self.{accessor} {{\n{arms}        }}")
        }
    }

    /// The accessor of each group in the source trait, the group with each
    /// element as the source presents it.
    fn group_accessor_type(&self, group: &GateGroup) -> String {
        self.group_held_in(group, &|f| {
            // The chain of the memberships as the one walk every arm refines
            // (docs/adr/0065 §6 R6a).
            match (holds_chain(group, f), self.child(f)) {
                (true, Some(child)) => format!(
                    "{RUNTIME_CODEC}::BoundedIter<impl Iterator<Item = impl {}> + Clone, {}>",
                    child.trait_path,
                    super::rust_bound_most(codec_totality::list_bound(f, &self.m().fields))
                ),
                _ => self.member_value_type(f),
            }
        })
    }

    /// The statement that binds each element of the groups to its local, as
    /// the side the value holds says: `Some` on that side, `None` on the
    /// other.
    fn group_bindings(&self, lines: &mut Vec<String>) {
        for group in &self.encode.roles.groups {
            let accessor = self.group_accessor(group);
            let all: Vec<&CodecField> = self.group_members(group);
            // A group whose every arm holds every element (a disjunction's)
            // binds each element where any arm holds it.
            let uniform = group
                .arms
                .iter()
                .all(|arm| all.iter().all(|f| arm.fields.contains(&f.id)));
            let names = |arm: &GroupArm| {
                self.arm_held(group, arm)
                    .map(|f| self.held_name(group, arm, f))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            // A chain's memberships hold the chain in each arm with the arm's
            // refinement, so their arms bind apart (docs/adr/0065 §6 R6a).
            if uniform && all.len() == 1 && group.chain().is_none() {
                let member = self.accessor(all[0]);
                let patterns = group
                    .arms
                    .iter()
                    .map(|arm| format!("{} {{ {member} }}", self.arm_path(group, arm)))
                    .collect::<Vec<_>>()
                    .join(" | ");
                let local = self.member_local(all[0]);
                lines.push(if group.absent.is_some() {
                    format!(
                        "let {local} = src.{accessor}().map(|_g| match _g {{\n    {patterns} => {member},\n}});"
                    )
                } else {
                    format!("let {local} = Some(match src.{accessor}() {{\n    {patterns} => {member},\n}});")
                });
                continue;
            }
            let locals = all
                .iter()
                .map(|f| self.member_local(f))
                .collect::<Vec<_>>()
                .join(", ");
            // The chain of a membership is bound as its walk, each entry and
            // the named one at its place, whichever arm holds it.
            let tuple = |arm: &GroupArm| {
                all.iter()
                    .map(|f| {
                        let acc = self.accessor(f);
                        if !arm.fields.contains(&f.id) {
                            "None".to_string()
                        } else if !holds_chain(group, f) {
                            format!("Some({acc})")
                        } else {
                            // A bare entry's chain is walked as slots, the named
                            // entry at its place; one with content is walked as
                            // the chain it is (R6b).
                            let walk = if group.member_is_bare() {
                                "slots()"
                            } else {
                                "into_inner()"
                            };
                            if !chain_optional_in(group, f, arm) {
                                format!("Some({acc}.{walk})")
                            } else {
                                format!("{acc}.map(|_e| _e.{walk})")
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let mut arms: Vec<String> = group
                .arms
                .iter()
                .map(|arm| {
                    let pattern = format!("{} {{ {} }}", self.arm_path(group, arm), names(arm));
                    // Two lists of one count are taken apart here, each written
                    // as a list is (docs/adr/0065 §6 R5).
                    let parts = self.arm_held(group, arm).find_map(|f| {
                        let second = self.second_of(group, arm, f)?;
                        Some(format!(
                            "let ({}, {}) = {}.into_parts(); ",
                            self.accessor(f),
                            self.accessor(second),
                            self.held_name(group, arm, f)
                        ))
                    });
                    match parts {
                        Some(parts) => format!(
                            "    {} => {{ {parts}({}) }}",
                            Self::held(group, &pattern),
                            tuple(arm)
                        ),
                        None => format!("    {} => ({}),", Self::held(group, &pattern), tuple(arm)),
                    }
                })
                .collect();
            if group.absent.is_some() {
                arms.push(format!(
                    "    None => ({}),",
                    vec!["None"; all.len()].join(", ")
                ));
            }
            lines.push(format!(
                "let ({locals}) = match src.{accessor}() {{\n{}\n}};",
                arms.join("\n")
            ));
        }
    }

    /// A flag of a disjunction that decides a group (docs/adr/0065 §6 R4c): the
    /// value the arm the value holds gives it, or the one it has with no arm.
    fn derive_arm_flag(
        &self,
        id: &ElementId,
        group: usize,
        index: usize,
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let ElementId::Flag { field, flag } = id else {
            unreachable!("a disjunction's gate is a flag of a carrier");
        };
        let group = &self.encode.groups()[group];
        let local = self.local("_d_", id);
        let held = format!("src.{}()", self.group_accessor(group));
        lines.push(format!(
            "let {local}: bool = {};",
            self.gate_value(group, index, &held)
        ));
        derived
            .flags
            .entry(field.clone())
            .or_default()
            .push(DerivedFlag {
                flag: flag.clone(),
                value: local,
                ty: "bool".to_string(),
                kind: FlagValue::Bool,
            });
    }

    /// The local that holds the value of a derived element, named after it.
    fn local(&self, prefix: &str, id: &ElementId) -> String {
        let name = match id {
            ElementId::Field(field) => self.l().codec_field_id(field),
            ElementId::Flag { field, flag } => format!(
                "{}_{}",
                self.l().codec_field_id(field),
                filters::to_snake_case(flag.clone())
            ),
            ElementId::Variant => "variant".to_string(),
        };
        format!("{prefix}{name}")
    }

    /// The local that holds a carrier with its derived flags folded in.
    fn carrier_local(&self, field: &CodecField) -> String {
        format!("_c_{}", self.l().codec_field_id(&field.id))
    }

    /// The fields whose size the element `id` is derived from, when it is a
    /// length or a count ([`codec_totality::measured`]).
    fn measured(&self, id: &ElementId) -> Vec<&CodecField> {
        codec_totality::measured(self.m(), &self.encode.roles, id)
    }

    /// Whether a source states `field` when its content is absent: derived only
    /// while the fields it measures are there ([`codec_totality::stated`]).
    fn stated(&self, field: &CodecField) -> bool {
        codec_totality::stated(self.m(), &self.encode.roles, field)
    }

    /// The condition, in a form that holds its fields as members, under which all
    /// the content `field` measures is absent.
    fn absent_content(&self, field: &CodecField) -> String {
        self.measured(&ElementId::Field(field.id.clone()))
            .into_iter()
            .map(|f| format!("self.{}.is_none()", self.accessor(f)))
            .collect::<Vec<_>>()
            .join(" && ")
    }

    /// The type an accessor returns for the value of `field` when it is
    /// present: a scalar by value, bytes and text by reference.
    fn value_type(&self, field: &CodecField) -> String {
        match self.encode.value(field) {
            // An embedded body is read by reference.
            Value::Body { codec } => format!("&impl {}", self.child_of_plan(codec).trait_path),
            // A list as the entries it holds in order, with their count, within
            // its bound (docs/adr/0065 §6 R2): any iterator over values that say
            // an entry, held to its law where its entries say an input it hands
            // them (§6 R8).
            Value::List { codec, bound, law } => lawful(
                self.name(),
                &field.id,
                &law,
                format!(
                    "{RUNTIME_CODEC}::BoundedIter<impl Iterator<Item = impl {}> + Clone, {}>",
                    self.child_of_plan(codec).trait_path,
                    super::rust_bound_args(bound)
                ),
            ),
            // Within the field's bound (docs/adr/0077 §3).
            Value::Bytes(bound) => super::rust_bounded_text_type(false, bound, "'_"),
            Value::Text(bound) => super::rust_bounded_text_type(true, bound, "'_"),
            Value::Scalar(ty) => self.l().type_name(ty).to_string(),
        }
    }

    /// The imported codec the plan names by `alias`, which the codec holds: the
    /// check of [`Plan::of`] found every codec a list or body names.
    fn child_of_plan(&self, alias: &str) -> Child {
        self.child_of(alias)
            .expect("the plan names a codec the codec imports")
    }

    fn accessor_type(&self, field: &CodecField) -> String {
        let value = self.value_type(field);
        // A gated field may be absent; an element stated only while its content
        // is absent is absent the rest of the time.
        if self.encode.presence(field).optional() {
            format!("Option<{value}>")
        } else {
            value
        }
    }

    // ── the interface ────────────────────────────────────────────────

    // ── the variant ──────────────────────────────────────────────────

    /// `{Codec}VariantSource`: the arm a value holds, as the interface says it.
    fn body_enum(&self) -> String {
        format!("{}VariantSource", self.name())
    }

    /// The arms of the variant: those it enumerates, then the default.
    fn arms(&self) -> Vec<Arm> {
        self.encode
            .arms
            .iter()
            .enumerate()
            .map(|(i, arm)| {
                let known = arm.tag.is_some();
                Arm {
                    alias: arm.codec.to_string(),
                    variant_name: if known {
                        filters::to_pascal_case(arm.codec.to_string())
                    } else {
                        "Default".to_string()
                    },
                    generic: if known {
                        format!("A{i}")
                    } else {
                        "D".to_string()
                    },
                    child: self.child_of_plan(arm.codec),
                    value: arm.tag,
                    feature: arm.feature.map(str::to_string),
                }
            })
            .collect()
    }

    /// The builds the variant's arms tell apart (docs/adr/0072): one for each
    /// set of the features its arms name, or the single build when they name
    /// none. An item whose text names the arms a build holds — the type of the
    /// accessor, whose type parameters are one per arm — is written once per
    /// world, under that world's `#[cfg]`.
    fn worlds(&self) -> Vec<World> {
        let features: Vec<String> = self
            .arms()
            .into_iter()
            .filter_map(|arm| arm.feature)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if features.is_empty() {
            return vec![World {
                cfg: None,
                on: std::collections::BTreeSet::new(),
            }];
        }
        (0..1usize << features.len())
            .map(|mask| {
                let on: std::collections::BTreeSet<String> = features
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| (mask >> i) & 1 == 1)
                    .map(|(_, f)| f.clone())
                    .collect();
                let terms: Vec<String> = features
                    .iter()
                    .map(|f| {
                        if on.contains(f) {
                            format!("feature = \"{f}\"")
                        } else {
                            format!("not(feature = \"{f}\")")
                        }
                    })
                    .collect();
                let predicate = match terms.as_slice() {
                    [one] => one.clone(),
                    _ => format!("all({})", terms.join(", ")),
                };
                World {
                    cfg: Some(format!("#[cfg({predicate})]")),
                    on,
                }
            })
            .collect()
    }

    /// `render(ty)` for the accessor type of the variant in each world, each
    /// under its `#[cfg]` line indented by `indent`.
    fn per_world(&self, indent: &str, render: impl Fn(&str) -> String) -> String {
        let mut out = String::new();
        for world in self.worlds() {
            if let Some(cfg) = &world.cfg {
                out.push_str(&format!("{indent}{cfg}\n"));
            }
            out.push_str(&render(&self.variant_type(&world)));
        }
        out
    }

    /// The type a tag takes: its carrier's, or the narrowest unsigned type that
    /// holds the bit-range it is read from.
    fn tag_type(&self) -> SceType {
        codec_totality::tag_type(self.m())
    }

    fn unknown_tag_type(&self) -> String {
        unknown_tag_type(self.name())
    }

    /// What the accessor of the variant returns in `world`: the enum, with the
    /// source of the body of each arm the world holds as a type of its own.
    fn variant_type(&self, world: &World) -> String {
        let args: Vec<String> = self
            .arms()
            .iter()
            .filter(|arm| world.holds(arm))
            .map(|arm| format!("impl {}", arm.child.trait_path))
            .collect();
        format!("{}<'_, {}>", self.body_enum(), args.join(", "))
    }

    fn body_enum_definition(&self) -> String {
        let arms = self.arms();
        if arms.is_empty() {
            return String::new();
        }
        // A type parameter of an arm under a build feature is declared under
        // it too, as an unused one is an error (E0392).
        let generics: Vec<String> = arms
            .iter()
            .map(|arm| format!("{}{}: ?Sized", arm.cfg_prefix(), arm.generic))
            .collect();
        let tag = self.unknown_tag_type();
        let mut variants = String::new();
        for arm in &arms {
            variants.push_str(&arm.cfg_line("    "));
            if arm.value.is_some() {
                variants.push_str(&format!("    {}(&'s {}),\n", arm.variant_name, arm.generic));
            } else {
                variants.push_str(&format!(
                    "    Default {{ tag: {tag}, body: &'s {} }},\n",
                    arm.generic
                ));
            }
        }
        // The tag of each arm: the one table the encode writes from and a caller
        // reads a value's tag from. A type's arguments take no `#[cfg]`, so the
        // impl is written once for each build the arms tell apart, with the
        // arms that build holds (docs/adr/0072).
        let ty = self.l().type_name(&self.tag_type()).to_string();
        let mut tag_impls = String::new();
        for world in self.worlds() {
            let held: Vec<&Arm> = arms.iter().filter(|arm| world.holds(arm)).collect();
            let mut tags = String::new();
            for arm in &held {
                // Each item of a gated arm carries its feature, inside a build's
                // impl as everywhere else (docs/adr/0072).
                tags.push_str(&arm.cfg_line("            "));
                match arm.value {
                    Some(value) => tags.push_str(&format!(
                        "            Self::{}(_) => {value},\n",
                        arm.variant_name
                    )),
                    None => tags.push_str("            Self::Default { tag, .. } => tag.get(),\n"),
                }
            }
            let params: Vec<String> = held
                .iter()
                .map(|arm| format!("{}: ?Sized", arm.generic))
                .collect();
            let args: Vec<&str> = held.iter().map(|arm| arm.generic.as_str()).collect();
            if let Some(cfg) = &world.cfg {
                tag_impls.push_str(&format!("{cfg}\n"));
            }
            tag_impls.push_str(&format!(
                "impl<'s, {params}> {enum_name}<'s, {args}> {{\n\
                 \x20   /// The tag of the arm this is: the value the document gives a known\n\
                 \x20   /// arm, and the tag a default arm holds. It is the tag the encode\n\
                 \x20   /// writes, or the one a caller hands the decode for a variant whose\n\
                 \x20   /// wire form carries none.\n\
                 \x20   pub fn tag(&self) -> {ty} {{\n\
                 \x20       match self {{\n{tags}\x20       }}\n\
                 \x20   }}\n\
                 }}\n\n",
                params = params.join(", "),
                args = args.join(", "),
                enum_name = self.body_enum(),
            ));
        }
        format!(
            "/// The arm of `{name}`'s variant a value holds, each arm's body as the\n\
             /// source it says. The tag is not among them: `{name}` derives it from\n\
             /// the arm, as it derives a length, and a default arm presents the tag\n\
             /// it could not name.\n\
             // A default arm's tag is read by the codec that embeds this one and\n\
             // derives its own carrier bits from the arm, which not every build uses.\n\
             #[allow(dead_code)]\n\
             pub enum {enum_name}<'s, {generics}> {{\n{variants}}}\n\n\
             {tag_impls}",
            name = self.name(),
            enum_name = self.body_enum(),
            generics = generics.join(", "),
        )
    }

    /// `tag_of`: the tag of the arm a value holds, read through the interface,
    /// so every form of the value (the builder, the parts, the owned form, the
    /// view) answers from the one table.
    fn tag_of_function(&self) -> String {
        if self.arms().is_empty() {
            return String::new();
        }
        format!(
            "\x20   /// The tag of the arm `src` holds ([`{body}::tag`]).\n\
             \x20   pub fn tag_of<S: {trait_name} + ?Sized>(src: &S) -> {ty} {{\n\
             \x20       src.body().tag()\n\
             \x20   }}\n\n",
            body = self.body_enum(),
            trait_name = self.trait_name,
            ty = self.l().type_name(&self.tag_type()),
        )
    }

    // ── the borrowed source ──────────────────────────────────────────

    fn parts_name(&self) -> String {
        format!("{}Parts", self.name())
    }

    fn parts_body_name(&self) -> String {
        format!("{}PartsBody", self.name())
    }

    /// `<'a>` when the parts borrow from the caller, otherwise nothing.
    fn parts_lt(&self) -> &'static str {
        if parts_has_lifetime(self.m()) {
            "<'a>"
        } else {
            ""
        }
    }

    /// The type a field of the parts holds: what the caller already has in hand
    /// for the value the accessor says, borrowed for the life of the parts.
    fn parts_value_type(&self, field: &CodecField) -> String {
        if let Some(child) = self.child(field) {
            // A list is a slice no longer than its bound (docs/adr/0065 §6 R2):
            // `Bounded::new` checks a slice once, and an array within the bound
            // converts without a check.
            return if field.is_embed() {
                format!("&'a {}", child.parts_type)
            } else {
                lawful_list(
                    &self.encode.roles,
                    self.name(),
                    &field.id,
                    format!(
                        "{RUNTIME_CODEC}::Bounded<&'a [{}], {}>",
                        child.parts_type,
                        super::rust_list_bound_args(field, &self.m().fields)
                    ),
                )
            };
        }
        match &field.sce_type {
            // Within the field's bound: `new` checks a slice once, and an
            // array whose length lies in it converts without a check
            // (docs/adr/0077 §3).
            SceType::Bytes | SceType::String => {
                super::rust_borrowed_text_type(field, &self.m().fields, "'a")
            }
            other => self.l().type_name(other).to_string(),
        }
    }

    fn parts_field_type(&self, field: &CodecField) -> String {
        let value = self.parts_value_type(field);
        if field.present_if.is_some() || self.stated(field) {
            format!("Option<{value}>")
        } else {
            value
        }
    }

    /// `{Codec}Parts`: a source that holds what a value says in the types a
    /// caller already has, with its lists as slices of the entries' parts and its
    /// bodies as references. Nothing is copied into it and it reserves no room
    /// for a list's capacity, so the stack an encode needs is the entries the
    /// caller holds (ADR 0058 §4).
    fn parts_definition(&self) -> String {
        let mut members = String::new();
        for field in &self.m().fields {
            if !self.has_accessor(field) {
                continue;
            }
            members.push_str(&format!(
                "    /// The `{id}` field.\n    pub {accessor}: {ty},\n",
                id = field.id,
                accessor = self.accessor(field),
                ty = self.parts_field_type(field),
            ));
        }
        for group in &self.encode.roles.groups {
            members.push_str(&format!(
                "    /// The fields `{gate}` decides, held together (docs/adr/0065 §6 R4).\n    pub {accessor}: {ty},\n",
                gate = gates_label(group),
                accessor = self.group_accessor(group),
                ty = self.group_held_in(group, &|f| {
                    // The chain of the memberships as the slice every arm
                    // refines.
                    match (holds_chain(group, f), self.child(f)) {
                        (true, Some(child)) => format!("&'a [{}]", child.parts_type),
                        _ => self.parts_value_type(f),
                    }
                }),
            ));
        }
        let mut out = String::new();
        if self.m().variant.is_some() {
            members.push_str(&format!(
                "    /// The arm of the variant this value holds.\n    pub body: {}{},\n",
                self.parts_body_name(),
                self.parts_lt()
            ));
            let mut arms = String::new();
            let tag = self.unknown_tag_type();
            for arm in self.arms() {
                arms.push_str(&arm.cfg_line("    "));
                if arm.value.is_some() {
                    arms.push_str(&format!(
                        "    {}(&'a {}),\n",
                        arm.variant_name, arm.child.parts_type
                    ));
                } else {
                    arms.push_str(&format!(
                        "    /// The tag the grammar could not name, and the body it holds.\n    Default {{ tag: {tag}, body: &'a {} }},\n",
                        arm.child.parts_type
                    ));
                }
            }
            out.push_str(&format!(
                "/// The arm of `{name}`'s variant that {parts} holds.\n\
                 #[allow(dead_code)]\n\
                 #[derive(Debug, Clone, Copy)]\n\
                 pub enum {body}<'a> {{\n{arms}}}\n\n",
                name = self.name(),
                parts = self.parts_name(),
                body = self.parts_body_name(),
            ));
        }
        out.push_str(&format!(
            "/// A value of `{name}` held in the types a caller already has: a source\n\
             /// ([`{trait_name}`]) whose lists are slices within their bounds and whose\n\
             /// bodies are references.\n\
             #[allow(dead_code)]\n\
             #[derive(Debug, Clone, Copy)]\n\
             pub struct {parts}{lt} {{\n{members}}}\n\n",
            name = self.name(),
            trait_name = self.trait_name,
            parts = self.parts_name(),
            lt = self.parts_lt(),
        ));
        out
    }

    fn parts_impl(&self) -> String {
        let mut items = String::new();
        for group in &self.encode.roles.groups {
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        {read}\n    }}\n",
                accessor = self.group_accessor(group),
                ty = self.group_accessor_type(group),
                read = self.group_read(group, None),
            ));
        }
        for field in &self.m().fields {
            if !self.has_accessor(field) {
                continue;
            }
            let accessor = self.accessor(field);
            let read = if self.child(field).is_some() && !field.is_embed() {
                let walk = self.walk_of(field, false);
                if field.present_if.is_some() {
                    format!("self.{accessor}.as_ref().map(|_l| _l.{walk})")
                } else {
                    format!("self.{accessor}.{walk}")
                }
            } else {
                format!("self.{accessor}")
            };
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        {read}\n    }}\n",
                ty = self.accessor_type(field),
            ));
        }
        if self.m().variant.is_some() {
            let enum_name = self.body_enum();
            let parts_body = self.parts_body_name();
            let mut arms = String::new();
            for arm in self.arms() {
                arms.push_str(&arm.cfg_line("            "));
                if arm.value.is_some() {
                    arms.push_str(&format!(
                        "            {parts_body}::{name}(_b) => {enum_name}::{name}(*_b),\n",
                        name = arm.variant_name,
                    ));
                } else {
                    arms.push_str(&format!(
                        "            {parts_body}::Default {{ tag, body }} => {enum_name}::Default {{ tag: *tag, body: *body }},\n",
                    ));
                }
            }
            items.push_str(&self.per_world("    ", |ty| {
                format!(
                    "    fn body(&self) -> {ty} {{\n        match &self.body {{\n{arms}        }}\n    }}\n"
                )
            }));
        }
        let lt = self.parts_lt();
        format!(
            "impl{lt} {trait_name} for {parts}{lt} {{\n{items}}}\n\n",
            trait_name = self.trait_name,
            parts = self.parts_name(),
        )
    }

    fn text(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.law_definitions());
        out.push_str(&self.group_definitions());
        out.push_str(&self.body_enum_definition());
        out.push_str(&self.overrides_definition());
        out.push_str(&self.trait_definition());
        out.push_str(&self.reference_impl());
        out.push_str(&self.parts_definition());
        out.push_str(&self.parts_impl());
        out.push_str(&self.impls());
        out.push_str(&self.encode_impl());
        out
    }

    /// Whether the source presents `field` by an accessor of its own: it is
    /// presented or stated, and no group holds it.
    fn has_accessor(&self, field: &CodecField) -> bool {
        self.encode.presents_alone(field)
    }

    fn trait_definition(&self) -> String {
        let mut items = String::new();
        for element in &self.encode.elements {
            let field = element.field;
            items.push_str(&format!(
                "    /// The `{id}` field.\n    fn {accessor}(&self) -> {ty};\n",
                id = field.id,
                accessor = self.accessor(field),
                ty = self.accessor_type(field),
            ));
        }
        for group in &self.encode.roles.groups {
            items.push_str(&format!(
                "    /// The fields `{gate}` decides, held together (docs/adr/0065 §6 R4).\n    fn {accessor}(&self) -> {ty};\n",
                gate = gates_label(group),
                accessor = self.group_accessor(group),
                ty = self.group_accessor_type(group),
            ));
        }
        if self.m().variant.is_some() {
            items.push_str(&self.per_world("    ", |ty| {
                format!("    /// The arm of the variant this value holds.\n    fn body(&self) -> {ty};\n")
            }));
        }
        format!(
            "// ── Source (ADR 0058) ────────────────────────────────────────────────\n\
             /// What a value of `{name}` says: its presented fields. A length, a count\n\
             /// or the flag a gated field is announced by is not among them; the\n\
             /// encode computes those from what is ([`{name}Wire::encode_source`]).\n\
             pub trait {trait_name} {{\n{items}}}\n\n",
            name = self.name(),
            trait_name = self.trait_name,
        )
    }

    /// `impl<T: {Trait} + ?Sized> {Trait} for &T`, which lets a list of
    /// references, or a reference to a value, stand for the value.
    fn reference_impl(&self) -> String {
        let mut items = String::new();
        for field in &self.m().fields {
            if !self.has_accessor(field) {
                continue;
            }
            let accessor = self.accessor(field);
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        (**self).{accessor}()\n    }}\n",
                ty = self.accessor_type(field),
            ));
        }
        for group in &self.encode.roles.groups {
            let accessor = self.group_accessor(group);
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        (**self).{accessor}()\n    }}\n",
                ty = self.group_accessor_type(group),
            ));
        }
        if self.m().variant.is_some() {
            items.push_str(&self.per_world("    ", |ty| {
                format!("    fn body(&self) -> {ty} {{\n        (**self).body()\n    }}\n")
            }));
        }
        format!(
            "impl<T: {trait_name} + ?Sized> {trait_name} for &T {{\n{items}}}\n\n",
            trait_name = self.trait_name,
        )
    }

    /// Every form of the codec that says what it holds: the value and its view.
    /// A codec that has a storage profile holds its value at any of them, which
    /// one impl answers for, the eager form among them; any other holds it
    /// plainly. The one `encode_source` writes them all.
    fn impls(&self) -> String {
        let mut out = self.impl_for(if self.input.owned {
            Form::Owned
        } else {
            Form::Builder
        });
        if self.input.view_lifetime.is_some() {
            out.push_str(&self.impl_for(Form::View));
        }
        out
    }

    /// The `impl` of the source trait for one form of the codec.
    fn impl_for(&self, form: Form) -> String {
        let storage = "::sce_forge_runtime::codec";
        let mut items = String::new();
        for group in &self.encode.roles.groups {
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        {read}\n    }}\n",
                accessor = self.group_accessor(group),
                ty = self.group_accessor_type(group),
                read = self.group_read(group, Some(form)),
            ));
        }
        for field in &self.m().fields {
            if !self.has_accessor(field) {
                continue;
            }
            let accessor = self.accessor(field);
            let gated = field.present_if.is_some();
            // A list or an embedded body is lent, a scalar copied or reborrowed.
            // An owned form holds its text, bytes and lists in the storage
            // profile's containers, which it reads through their traits.
            let read = if self.stated(field) {
                // The number the wire holds beside content that is not there.
                format!(
                    "if {} {{ Some(self.{accessor}) }} else {{ None }}",
                    self.absent_content(field)
                )
            } else if field.is_embed() {
                if gated {
                    format!("self.{accessor}.as_ref()")
                } else {
                    format!("&self.{accessor}")
                }
            } else if self.child(field).is_some() {
                // A list's entries with their count (docs/adr/0065 §6 R2). The
                // builder's `heapless::Vec` is sized to the bound, which its type
                // proves; the view and the owned form hold a `Bounded`, the owned
                // one around its profile's list, read as a slice. A builder's
                // list of one entry or more is a `Bounded` too (R7).
                // A list held to its law is walked through it (docs/adr/0065 §6 R8).
                let builder_holds_bounded =
                    codec_totality::list_least(field) > 0 || self.lawful(field);
                let owned = self.walk_of(field, true);
                let walk = self.walk_of(field, false);
                match (form, gated) {
                    (Form::Builder, true) if !builder_holds_bounded => {
                        format!("self.{accessor}.as_ref().map({storage}::BoundedIter::from)")
                    }
                    (Form::Builder, false) if !builder_holds_bounded => {
                        format!("{storage}::BoundedIter::from(&self.{accessor})")
                    }
                    (Form::Owned, true) => {
                        format!("self.{accessor}.as_ref().map(|_l| _l.{owned})")
                    }
                    (Form::Owned, false) => format!("self.{accessor}.{owned}"),
                    (_, true) => format!("self.{accessor}.as_ref().map(|_l| _l.{walk})"),
                    (_, false) => format!("self.{accessor}.{walk}"),
                }
            } else if form == Form::Owned
                && matches!(field.sce_type, SceType::Bytes | SceType::String)
            {
                if gated {
                    format!(
                        "self.{accessor}.as_ref().map({})",
                        self.owned_text_read_fn(field)
                    )
                } else {
                    self.owned_text_read(field, &format!("self.{accessor}"))
                }
            } else {
                format!("self.{accessor}")
            };
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        {read}\n    }}\n",
                ty = self.accessor_type(field),
            ));
        }
        if self.m().variant.is_some() {
            // The arm the value holds, each body lent.
            let enum_name = self.body_enum();
            let variant = format!("{}{}", self.name(), form.variant_suffix());
            let mut arms = String::new();
            for arm in self.arms() {
                arms.push_str(&arm.cfg_line("            "));
                if arm.value.is_some() {
                    arms.push_str(&format!(
                        "            {variant}::{name}(_b) => {enum_name}::{name}(_b),\n",
                        name = arm.variant_name,
                    ));
                } else {
                    arms.push_str(&format!(
                        "            {variant}::Default {{ tag, body }} => {enum_name}::Default {{ tag: *tag, body }},\n",
                    ));
                }
            }
            items.push_str(&self.per_world("    ", |ty| {
                format!(
                    "    fn body(&self) -> {ty} {{\n        match &self.body {{\n{arms}        }}\n    }}\n"
                )
            }));
        }
        let head = match form {
            Form::Builder => format!(
                "impl{lt} {} for {}{lt}",
                self.trait_name,
                self.name(),
                lt = self.lt()
            ),
            Form::View => {
                let lt = self.input.view_lifetime.unwrap_or_default();
                format!("impl{lt} {} for {}View{lt}", self.trait_name, self.name())
            }
            Form::Owned => format!(
                "impl<S: {name}Storage> {} for {name}Owned<S>",
                self.trait_name,
                name = self.name()
            ),
        };
        format!("{head} {{\n{items}}}\n\n")
    }

    // ── the encode ───────────────────────────────────────────────────

    fn encode_impl(&self) -> String {
        let write = self.write_functions(&self.write_body());
        // A list or an embedded body is written by the codec it holds, which
        // can refuse a value after this codec has written bytes. The same write
        // into a sink that only counts finds that refusal first, so the
        // caller's sink is untouched when the encode refuses.
        let params = self.input_params();
        let args = self.input_args();
        let guard = if self.children_can_refuse() {
            format!("\x20       Self::count_source(src{args})?;\n")
        } else {
            String::new()
        };
        let count = count_function(&self.trait_name, &params, &args, self.may_fail());
        let measure = if self.given_inputs().next().is_none() {
            format!(
                "\n{}",
                measure_impl(
                    &wire_name(self.name()),
                    "",
                    &self.trait_name,
                    self.input.inline_encode,
                    self.may_fail(),
                )
            )
        } else {
            String::new()
        };
        let refused = if self.may_fail() {
            "\x20   /// `src` presents, and a value the encoding cannot carry is refused\n\
             \x20   /// before the first byte is written.\n"
        } else {
            "\x20   /// `src` presents. Every value `src` can present has a frame, so the\n\
             \x20   /// only error is the sink's (docs/adr/0065).\n"
        };
        format!(
            "/// The codec `{name}` itself: what writes a value read through\n\
             /// [`{trait_name}`], holding no value and naming no storage profile. Every\n\
             /// form of the value, a parent and a view write through it, so a profile\n\
             /// that cannot hold some field is never asked for here (docs/adr/0077 §7).\n\
             #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n\
             pub struct {wire};\n\n\
             #[allow(dead_code)]\n\
             impl {wire} {{\n\
             \x20   /// Encode what `src` says into the caller-owned sink: the bytes\n\
             \x20   /// `{name}::encode` writes for the value it holds, read through\n\
             \x20   /// [`{trait_name}`]. A length, a count or a gate flag is computed from what\n\
             {refused}\
             \x20   pub fn encode_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> {result} {{\n\
             {guard}\
             \x20       Self::write_source(src, w{args})\n\
             \x20   }}\n\n\
             {inputs}\
             {tag_of}\
             {write}\n\
             {count}\
             }}\n\
             {measure}",
            wire = wire_name(self.name()),
            name = self.name(),
            trait_name = self.trait_name,
            result = self.write_result("K"),
            inputs = self.input_functions(),
            tag_of = self.tag_of_function(),
        )
    }

    /// `write_source`: the wire without the guard a parent has already made. A
    /// codec with flags a parent may set also has `write_source_in`, which
    /// takes them, and `write_source` is that with none set.
    fn write_functions(&self, body: &str) -> String {
        let trait_name = &self.trait_name;
        let params = self.input_params();
        let args = self.input_args();
        let inline = write_attribute(self.input.inline_encode);
        let result = self.write_result("K");
        if self.overridable().is_empty() {
            return format!(
                "\x20   /// [`Self::encode_source`] without the guard a parent has already made.\n\
                 \x20   #[doc(hidden)]\n\
                 {inline}\
                 \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> {result} {{\n\
                 {body}\
                 \x20   }}\n"
            );
        }
        let overrides = self.overrides_name();
        format!(
            "\x20   /// [`Self::encode_source`] without the guard a parent has already made.\n\
             \x20   #[doc(hidden)]\n\
             {inline}\
             \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> {result} {{\n\
             \x20       Self::write_source_in(src, w{args}, &{overrides}::default())\n\
             \x20   }}\n\n\
             \x20   /// [`Self::write_source`] with the flags a parent sets in place of the\n\
             \x20   /// ones `src` presents.\n\
             \x20   #[doc(hidden)]\n\
             {inline}\
             \x20   pub fn write_source_in<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}, over: &{overrides}) -> {result} {{\n\
             {body}\
             \x20   }}\n"
        )
    }

    /// The statements of `write_source`: the derived values, then the wire.
    fn write_body(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        // Each group's elements, as the side the value holds says them
        // (docs/adr/0065 §6 R4b).
        self.group_bindings(&mut lines);
        // An input the value says is read off it, under the name a gate reads
        // it by; a codec that does not test it has nothing to read
        // (docs/adr/0065 §6 R4a).
        for said in self.encode.said.iter().filter(|said| said.read) {
            lines.push(format!(
                "let {}: u8 = u8::from({});",
                Self::input_name(said.input),
                self.said_input(&said.input.name, "src")
            ));
        }
        let derived = self.derivations(&mut lines);
        self.gate_checks(&derived, &mut lines);
        for input in self.given_inputs() {
            // A codec that does not test its input still takes it.
            lines.push(format!("let _ = {};", Self::input_name(input)));
        }
        if self.m().fields.is_empty() && self.m().variant.is_none() {
            // An empty body: the surrounding wire-protocol header alone marks the
            // codec, and nothing is read or written.
            lines.push("let _ = (src, w);".to_string());
            lines.push("Ok(())".to_string());
            return indent(&lines, 8);
        }
        for write in &self.encode.program {
            match write {
                Write::Byte { pieces, .. } => self.byte(pieces, &derived, &mut lines),
                Write::Whole(field) => self.whole(field, &derived, &mut lines),
                Write::Tail { field, pad } => {
                    if *pad {
                        lines.push(format!(
                            "while w.position() < {} {{\n    w.write_u8(0)?;\n}}",
                            field.byte_offset
                        ));
                    }
                    let value = self.read(field, &derived);
                    lines.push(format!("w.write_bytes({})?;", self.bytes_of(field, &value)));
                }
                Write::Streamed { field, guard, list } => {
                    self.streamed(field, *guard, list.as_ref(), &derived, &mut lines)
                }
                Write::Variant => self.write_variant(&derived, &mut lines),
            }
        }
        lines.push("Ok(())".to_string());
        indent(&lines, 8)
    }

    /// The suffix: the body of the arm the value holds.
    fn write_variant(&self, derived: &Derived, lines: &mut Vec<String>) {
        lines.push("match src.body() {".to_string());
        for arm in self.arms() {
            let pattern = self.arm_pattern(&arm);
            // An arm under a build feature is written in every rendering, its
            // refusal recorded as any arm's (docs/adr/0065), and compiled where
            // the feature is on (docs/adr/0072).
            if let Some(feature) = &arm.feature {
                lines.push(format!("    {}", rust_feature_cfg(feature)));
            }
            lines.push(format!(
                "    {pattern} => {}::write_source(_b, w{}){},",
                arm.child.type_name,
                self.thread_args(&arm.alias, derived),
                self.child_write(&arm.alias, &arm.alias),
            ));
        }
        lines.push("}".to_string());
    }

    /// The bits of a carrier that an embedded variant's active arm fills: the
    /// arm's value, read from the embedded body's source. A gated body that is
    /// absent fills none.
    fn derive_parent_tag(
        &self,
        id: &ElementId,
        planned: &ParentTag<'_>,
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let ElementId::Flag { field, flag } = id else {
            unreachable!("a parent tag fills a flag of a carrier");
        };
        let ty = self.l().type_name(&planned.carrier.sce_type).to_string();
        let local = self.local("_d_", id);
        let mut terms: Vec<String> = Vec::new();
        for term in &planned.terms {
            let import = self
                .input
                .imports
                .iter()
                .find(|i| i.alias == term.codec)
                .expect("the plan names an embedded codec the codec imports");
            let embed = term.body;
            let mask = term.mask;
            let enum_path = format!(
                "super::{}::{}VariantSource",
                import.namespace, import.type_name
            );
            // An arm the embedded codec compiles under a build feature has no
            // variant in a build without it, so its arm here is compiled under
            // the same feature (docs/adr/0072).
            let mut arms: Vec<String> = term
                .arms
                .iter()
                .map(|arm| {
                    format!(
                        "{}{enum_path}::{}(_) => {}",
                        arm.feature
                            .as_deref()
                            .map(|f| format!("{} ", rust_feature_cfg(f)))
                            .unwrap_or_default(),
                        filters::to_pascal_case(arm.codec.clone()),
                        arm.tag
                    )
                })
                .collect();
            // A catch-all arm presents the tag it could not name, and that is
            // what the carrier holds.
            if term.catch_all {
                arms.push(format!(
                    "{enum_path}::Default {{ tag, .. }} => {ty}::from(tag.get()) & 0x{mask:X}"
                ));
            }
            let read = self.read_src(embed);
            let arms = arms.join(", ");
            // The embedded body's source is read through its trait by path: the
            // trait is not imported here.
            let body_of = format!(
                "super::{}::{}Source::body",
                import.namespace, import.type_name
            );
            terms.push(if embed.present_if.is_some() {
                format!(
                    "match {read} {{ Some(_x) => match {body_of}(_x) {{ {arms} }}, None => 0 }}"
                )
            } else {
                format!("match {body_of}({read}) {{ {arms} }}")
            });
        }
        lines.push(format!("let {local}: {ty} = {};", terms.join(" | ")));
        derived
            .flags
            .entry(field.clone())
            .or_default()
            .push(DerivedFlag {
                flag: flag.clone(),
                value: local,
                ty,
                kind: FlagValue::Number,
            });
    }

    /// The tag the active arm names, which the encode writes where the codec
    /// holds it. A default arm names none, so it presents the tag it holds.
    fn derive_variant_tag(&self, id: &ElementId, lines: &mut Vec<String>, derived: &mut Derived) {
        let ty = self.l().type_name(&self.tag_type()).to_string();
        let local = self.local("_d_", id);
        lines.push(format!("let {local}: {ty} = src.body().tag();"));
        match id {
            ElementId::Field(name) => {
                derived.fields.insert(name.clone(), local);
            }
            ElementId::Flag { field, flag } => {
                derived
                    .flags
                    .entry(field.clone())
                    .or_default()
                    .push(DerivedFlag {
                        flag: flag.clone(),
                        value: local,
                        ty,
                        kind: FlagValue::Number,
                    });
            }
            ElementId::Variant => unreachable!("a tag is not the variant"),
        }
    }

    // ── derived values ───────────────────────────────────────────────

    /// The prologue: each derived element's value, computed and checked before
    /// anything is written. Returns the expression each element is read as.
    fn derivations(&self, lines: &mut Vec<String>) -> Derived {
        let mut derived = Derived::default();
        // A carrier a parent may set a flag of is read as a local, folded below.
        for (carrier, _) in self.overridable() {
            derived.flags.entry(carrier.id.clone()).or_default();
        }
        self.count_lists(lines);
        for planned in &self.encode.derivations {
            let id = &planned.element;
            match &planned.by {
                Derive::Number(number) => self.derive_number(id, number, lines, &mut derived),
                Derive::VariantTag => self.derive_variant_tag(id, lines, &mut derived),
                Derive::ParentTag(tag) => self.derive_parent_tag(id, tag, lines, &mut derived),
                Derive::Presence(presence) => {
                    self.derive_presence(id, presence, lines, &mut derived)
                }
                Derive::ArmFlag { group, gate } => {
                    self.derive_arm_flag(id, *group, *gate, lines, &mut derived)
                }
                // The end flag of a chain is the entries' to set, as each is
                // written (`streamed`), and no value of this codec's own.
                Derive::ChainContinuation => {}
            }
        }
        derived.close(self, lines);
        derived
    }

    /// The expression for the number of bytes `field` holds, 0 when it is
    /// absent.
    fn byte_count(&self, field: &CodecField) -> String {
        let read = self.read_src(field);
        if field.present_if.is_some() {
            format!("{read}.map_or(0, |_v| _v.len())")
        } else {
            format!("{read}.len()")
        }
    }

    /// The local that holds `measure` of the number held in `local`, computed
    /// into `lines` (0 while the content is absent), with the test that its
    /// content is there where it may not be.
    fn measure(
        &self,
        measure: &encode_plan::Measure<'_>,
        local: &str,
        derived: &Derived,
        lines: &mut Vec<String>,
    ) -> Measure {
        let number = format!("_n{local}_{}", measure.at);
        match &measure.of {
            Measured::Bytes { field, offset } => self.measure_bytes(field, *offset, number, lines),
            Measured::Entries { list } => Measure {
                number: self.count_local(list),
                present: self.presence_test(list),
            },
            Measured::Encoded {
                body,
                codec,
                refuses,
            } => self.measure_encoded(body, codec, *refuses, derived, lines),
            Measured::Says { body, codec, input } => {
                let read = self.read_src(body);
                if body.present_if.is_some() {
                    lines.push(format!(
                        "let {number}: usize = {read}.map_or(0, |_b| {});",
                        self.says(codec, input, "_b")
                    ));
                    Measure {
                        number,
                        present: Some(format!("{read}.is_some()")),
                    }
                } else {
                    lines.push(format!(
                        "let {number}: usize = {};",
                        self.says(codec, input, &read)
                    ));
                    Measure {
                        number,
                        present: None,
                    }
                }
            }
            // A list hands every entry the flag, which its law holds them to:
            // the flag is the list's start, empty or not (docs/adr/0065 §6 R8).
            Measured::Start { list, input } => {
                let read = self.read_src(list);
                let start = |l: &str| {
                    format!(
                        "usize::from({})",
                        law_start(&self.encode.roles, &list.id, input, l)
                    )
                };
                if list.present_if.is_some() {
                    lines.push(format!(
                        "let {number}: usize = {read}.map_or(0, |_l| {});",
                        start("_l")
                    ));
                    Measure {
                        number,
                        present: Some(format!("{read}.is_some()")),
                    }
                } else {
                    lines.push(format!("let {number}: usize = {};", start(&read)));
                    Measure {
                        number,
                        present: None,
                    }
                }
            }
            Measured::ArmSays { arms } => self.measure_arms(arms, measure.gated, local, lines),
        }
    }

    /// `usize::from` of the input `input` that `body`, a value of the codec
    /// imported as `codec`, says (docs/adr/0065 §6 R4a).
    fn says(&self, codec: &str, input: &str, body: &str) -> String {
        format!(
            "usize::from({}::input_{}({body}))",
            self.child_of_plan(codec).type_name,
            filters::to_snake_case(input.to_string())
        )
    }

    /// The input the arm of the variant the value holds says, where it holds one
    /// of `said` (docs/adr/0065 §6 R4a): one match says both whether an arm that
    /// says it is there and what it says. An arm under a build feature carries
    /// its `#[cfg]`, which an alternative of `matches!` cannot.
    fn measure_arms(
        &self,
        said: &[ArmInput],
        gated: bool,
        local: &str,
        lines: &mut Vec<String>,
    ) -> Measure {
        let arms = self.arms();
        let arm_lines = |wrap: &dyn Fn(String) -> String| -> Vec<String> {
            said.iter()
                .map(|s| {
                    let arm = &arms[s.arm];
                    format!(
                        "    {}{} => {},",
                        arm.cfg_prefix(),
                        self.arm_pattern(arm),
                        wrap(self.says(&s.codec, &s.input, "_b"))
                    )
                })
                .collect()
        };
        if !gated {
            // Every arm says it: the value always does.
            let number = format!("_n{local}_arm");
            lines.push(format!(
                "let {number}: usize = match src.body() {{\n{}\n}};",
                arm_lines(&|v| v).join("\n")
            ));
            Measure {
                number,
                present: None,
            }
        } else {
            let said = format!("_o{local}_arm");
            lines.push(format!(
                "let {said}: Option<usize> = match src.body() {{\n{}\n    _ => None,\n}};",
                arm_lines(&|v| format!("Some({v})")).join("\n")
            ));
            Measure {
                number: format!("{said}.unwrap_or(0)"),
                present: Some(format!("{said}.is_some()")),
            }
        }
    }

    /// How many bytes `field` holds, as the wire's length of it (`offset`), in
    /// the local `number`.
    fn measure_bytes(
        &self,
        field: &CodecField,
        offset: Offset,
        number: String,
        lines: &mut Vec<String>,
    ) -> Measure {
        // A length plus a constant cannot pass `usize`, since a slice is at most
        // `isize::MAX` bytes.
        let measure = |count: &str| match offset {
            Offset::None => count.to_string(),
            Offset::Less { by, held: true } => format!("{count} - {by}"),
            Offset::Less { by, held: false } => format!(
                "{count}.checked_sub({by}).ok_or({})?",
                self.refuse(RefusalKind::LengthOutOfRange, &field.id)
            ),
            Offset::More { by } => format!("{count} + {by}"),
        };
        // A field gated off has no content to measure, so its length is the
        // wire's zero and not the arithmetic of an empty field.
        let value = if field.present_if.is_some() {
            format!(
                "match {} {{\n    Some(_v) => {},\n    None => 0,\n}}",
                self.read_src(field),
                measure("_v.len()")
            )
        } else {
            measure(&self.byte_count(field))
        };
        lines.push(format!("let {number}: usize = {value};"));
        Measure {
            number,
            present: self.presence_test(field),
        }
    }

    /// The test that `field` is there, for a field that may not be.
    fn presence_test(&self, field: &CodecField) -> Option<String> {
        field
            .present_if
            .is_some()
            .then(|| format!("{}.is_some()", self.read_src(field)))
    }

    /// The local that holds how many entries the list `field` holds.
    fn count_local(&self, field: &CodecField) -> String {
        format!("_k_{}", self.accessor(field))
    }

    /// Each list's entry count, as its source counted it. The source's type holds
    /// no more entries than the list's bound (docs/adr/0065 §6 R2), so the count
    /// is not checked here and no list is refused.
    fn count_lists(&self, lines: &mut Vec<String>) {
        for field in &self.m().fields {
            if !(field.is_repeat() || field.is_tlv_chain()) {
                continue;
            }
            let read = self.read_src(field);
            // A group's element is read from the arm the value holds, where it
            // may be absent whether or not it is itself gated (`read_src`).
            let optional =
                field.present_if.is_some() || self.encode.roles.group_of(&field.id).is_some();
            let count = if optional {
                format!("{read}.map_or(0, |_l| _l.len())")
            } else {
                format!("{read}.len()")
            };
            let local = self.count_local(field);
            lines.push(format!("let {local}: usize = {count};"));
        }
    }

    /// How many bytes the embedded `body`, of the codec imported as `codec`,
    /// encodes to: the body written into a sink that counts, which is the length
    /// the body's own encode gives it and cannot differ from it. The length of a
    /// body that is not there is the wire's zero.
    fn measure_encoded(
        &self,
        body: &CodecField,
        codec: &str,
        refuses: bool,
        derived: &Derived,
        lines: &mut Vec<String>,
    ) -> Measure {
        let child = self.child_of_plan(codec);
        let size = format!("_s_{}", self.accessor(body));
        let args = self.thread_args(codec, derived);
        // The body's own count ([`count_function`]): with nothing to handle for
        // a body that cannot refuse, and its refusal passed on for one that can.
        let count = format!(
            "{}::count_source(_v{args}){}",
            child.type_name,
            if refuses {
                self.child_write(codec, &body.id)
            } else {
                ""
            }
        );
        let read = self.read_src(body);
        lines.push(if body.present_if.is_some() {
            format!(
                "let {size}: usize = match {read} {{\n    Some(_v) => {count},\n    None => 0,\n}};"
            )
        } else {
            format!("let {size}: usize = {{\n    let _v = {read};\n    {count}\n}};")
        });
        Measure {
            number: size,
            present: None,
        }
    }

    /// The derived number `n`, a `usize`, as the type `ty` of the field
    /// `element` it is written into, as the plan says (docs/adr/0065 §5).
    fn narrow_number(&self, element: &str, ty: &SceType, narrow: Narrow, n: &str) -> String {
        let rust = self.l().type_name(ty);
        match narrow {
            Narrow::Checked => format!(
                "{rust}::try_from({n}).map_err(|_| {})?",
                self.refuse(RefusalKind::LengthOutOfRange, element)
            ),
            Narrow::Widened => format!("::sce_forge_runtime::codec::widen_usize({n})"),
            Narrow::Cast => format!("{n} as {rust}"),
        }
    }

    /// The element `id` holds the derived number `planned` (docs/adr/0081 §1):
    /// what is always there fixes it, and anything that is there beside it says
    /// the same or there is no frame that says both. When all that is measured
    /// may be absent, the number is derived while some of it is there, and the
    /// plan says what the element holds when none is.
    fn derive_number(
        &self,
        id: &ElementId,
        planned: &Number<'_>,
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let local = self.local("_d_", id);
        let measures: Vec<Measure> = planned
            .measures
            .iter()
            .map(|measure| self.measure(measure, &local, derived, lines))
            .collect();
        self.combine(id, &local, measures, &planned.into, lines, derived);
    }

    /// The element `id`, held in `local`, holds the number the `measures` say,
    /// which goes `into` it: the first measure always there fixes it, and every
    /// other one there says the same; where all may be absent, it is the first
    /// one there.
    fn combine(
        &self,
        id: &ElementId,
        local: &str,
        measures: Vec<Measure>,
        into: &NumberInto<'_>,
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let element = id.to_string();
        let disagree = || {
            format!(
                "{{\n    return Err({});\n}}",
                self.refuse(RefusalKind::InconsistentSource, &element)
            )
        };
        let always: Vec<&Measure> = measures.iter().filter(|m| m.present.is_none()).collect();
        let gated: Vec<&Measure> = measures.iter().filter(|m| m.present.is_some()).collect();
        // The number, as an expression of a `usize`, or the local that holds an
        // `Option<usize>` of it when there may be none to measure.
        let mut free: Option<String> = None;
        let mut number = "0usize".to_string();
        if let Some(anchor) = always.first() {
            for other in &always[1..] {
                lines.push(format!(
                    "if {} != {} {}",
                    other.number,
                    anchor.number,
                    disagree()
                ));
            }
            for g in &gated {
                lines.push(format!(
                    "if {} && {} != {} {}",
                    g.present.as_deref().unwrap_or("false"),
                    g.number,
                    anchor.number,
                    disagree()
                ));
            }
            number = anchor.number.clone();
        } else if !gated.is_empty() {
            let one = format!("_f{local}");
            let mut chain = String::new();
            for g in &gated {
                chain.push_str(&format!(
                    "if {} {{ Some({}) }} else ",
                    g.present.as_deref().unwrap_or("false"),
                    g.number
                ));
            }
            lines.push(format!("let {one}: Option<usize> = {chain}{{ None }};"));
            for g in &gated[1..] {
                lines.push(format!(
                    "if {} && Some({}) != {one} {}",
                    g.present.as_deref().unwrap_or("false"),
                    g.number,
                    disagree()
                ));
            }
            free = Some(one);
        }
        match (into, id) {
            (
                NumberInto::Field {
                    field,
                    narrow,
                    absent,
                },
                _,
            ) => {
                let ty = self.l().type_name(&field.sce_type);
                let narrow = |n: &str| self.narrow_number(&element, &field.sce_type, *narrow, n);
                match (&free, absent) {
                    (None, _) => lines.push(format!("let {local}: {ty} = {};", narrow(&number))),
                    (Some(_), Absent::Never) => {
                        unreachable!("the plan says what a free number's field holds")
                    }
                    // Gated beside its content, it is written only where there
                    // is content, and then the content says it.
                    (Some(one), Absent::Zero) => lines.push(format!(
                        "let {local}: {ty} = {};",
                        narrow(&format!("{one}.unwrap_or(0)"))
                    )),
                    // Derived while there is content, and what the group holds
                    // on the side where there is not (docs/adr/0065 §6 R4b):
                    // the two sides are apart, so it is never said twice.
                    (Some(one), Absent::Group) => {
                        lines.push(format!(
                            "let {local}: {ty} = match {one} {{\n    Some(_n) => {},\n    None => {}.unwrap_or_default(),\n}};",
                            narrow("_n"),
                            self.read_src(field),
                        ));
                    }
                    // Derived while there is content, and what the source states
                    // while there is not; a source that states a number beside
                    // content says it twice and must say the same.
                    (Some(one), Absent::Stated) => {
                        let read = self.read_src(field);
                        lines.push(format!(
                            "let {local}: {ty} = match {one} {{\n    Some(_n) => {{\n        let _d = {};\n        if {read}.is_some_and(|_s| _s != _d) {{\n            return Err({});\n        }}\n        _d\n    }}\n    None => {read}.unwrap_or_default(),\n}};",
                            narrow("_n"),
                            self.refuse(RefusalKind::InconsistentSource, &element),
                        ));
                    }
                }
                derived.fields.insert(field.id.clone(), local.to_string());
            }
            (NumberInto::Flag, ElementId::Flag { field, flag }) => {
                // With none of it there, the carrier keeps the bits the source
                // presents.
                let (value, kind) = match free {
                    Some(one) => (one, FlagValue::Optional),
                    None => (number, FlagValue::Number),
                };
                derived
                    .flags
                    .entry(field.clone())
                    .or_default()
                    .push(DerivedFlag {
                        flag: flag.clone(),
                        value,
                        ty: "usize".to_string(),
                        kind,
                    });
            }
            (NumberInto::Flag, _) => unreachable!("a number goes into a flag it names"),
        }
    }

    fn derive_presence(
        &self,
        id: &ElementId,
        planned: &PresenceFlag<'_>,
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let ElementId::Flag { field, flag } = id else {
            unreachable!("a gate flag is a flag of a carrier");
        };
        let mut presences: Vec<String> = Vec::new();
        for (index, (gated_field, when_clear)) in planned.of.iter().enumerate() {
            let present = format!("{}.is_some()", self.read_src(gated_field));
            // The flag is set exactly when the field is present, or exactly when
            // it is absent for a field gated on `!flag`.
            let local = format!("{}_{index}", self.local("_p_", id));
            lines.push(format!(
                "let {local}: bool = {};",
                if *when_clear {
                    format!("!{present}")
                } else {
                    present
                }
            ));
            presences.push(local);
        }
        // Fields one group holds are present together (docs/adr/0065 §6 R4b):
        // the first says the flag and the rest cannot disagree with it.
        if planned.together {
            presences.truncate(1);
        }
        for pair in presences.windows(2) {
            lines.push(format!(
                "if {} != {} {{\n    return Err({});\n}}",
                pair[0],
                pair[1],
                self.refuse(RefusalKind::InconsistentSource, &id.to_string()),
            ));
        }
        derived
            .flags
            .entry(field.clone())
            .or_default()
            .push(DerivedFlag {
                flag: flag.clone(),
                value: presences
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "false".to_string()),
                ty: "bool".to_string(),
                kind: FlagValue::Bool,
            });
    }

    // ── reading a field ──────────────────────────────────────────────

    /// The expression a field is read as: what the source presents, or what the
    /// encode derived for it.
    fn read(&self, field: &CodecField, derived: &Derived) -> String {
        if let Some(local) = derived.fields.get(&field.id) {
            return local.clone();
        }
        if derived.flags.contains_key(&field.id) {
            return self.carrier_local(field);
        }
        self.read_src(field)
    }

    /// `expr` as the carrier the wire holds: an enum goes out as its
    /// underlying value.
    fn carrier(&self, field: &CodecField, expr: &str) -> String {
        self.l().codec_carrier_expr(&field.sce_type, expr)
    }

    // ── the wire: positional layout ──────────────────────────────────

    /// One byte of a positional frame: the or of each field's piece in it, a
    /// zero where no field has one (docs/adr/0095 §2). A piece of a field wider
    /// than a byte is narrowed to one before it moves within the byte.
    fn byte(
        &self,
        pieces: &[(&CodecField, crate::forge::bit_layout::Piece)],
        derived: &Derived,
        lines: &mut Vec<String>,
    ) {
        let parts: Vec<String> = pieces
            .iter()
            .map(|(field, piece)| {
                let value = self.carrier(field, &self.read(field, derived));
                let narrow = self.l().codec_carrier_bits(&field.sce_type) == Some(8);
                // Drop the `>> 0` and `<< 0` no-ops (clippy::identity_op).
                let down = if piece.from == 0 {
                    value
                } else {
                    format!("({value} >> {})", piece.from)
                };
                let masked = format!("{down} & 0x{:02X}", piece.mask);
                let bits = if narrow {
                    masked
                } else {
                    format!("({masked}) as u8")
                };
                (bits, piece.to)
            })
            .map(|(bits, to)| match (pieces.len(), to) {
                (1, 0) => bits,
                (1, to) => format!("({bits}) << {to}"),
                (_, 0) => format!("({bits})"),
                (_, to) => format!("(({bits}) << {to})"),
            })
            .collect();
        if parts.is_empty() {
            lines.push("w.write_u8(0)?;".to_string());
        } else {
            lines.push(format!("w.write_u8({})?;", parts.join(" | ")));
        }
    }

    /// A whole field that shares none of its bytes: its value, as its bytes in
    /// its byte order.
    fn whole(&self, field: &CodecField, derived: &Derived, lines: &mut Vec<String>) {
        let value = self.carrier(field, &self.read(field, derived));
        let span = crate::forge::bit_layout::BitSpan::of(field, self.m().default_endian)
            .expect("a whole write is of a fixed field");
        if span.bytes == 1 {
            lines.push(format!("w.write_u8({value})?;"));
            return;
        }
        for e in encode_multibyte_unified(
            &value,
            span.bytes,
            span.endian,
            crate::generator::Language::Rust,
        ) {
            lines.push(format!("w.write_u8({e})?;"));
        }
    }

    // ── the wire: streamed layout ────────────────────────────────────

    fn streamed(
        &self,
        field: &CodecField,
        guard: Guard,
        list: Option<&ListWrite>,
        derived: &Derived,
        lines: &mut Vec<String>,
    ) {
        // A field derived whole whose gate is its own is written where it is
        // present: the gate flag is derived from the field it gates, and a length
        // field that is itself gated is announced by the same flag.
        let value = if guard == Guard::Present {
            "_v".to_string()
        } else {
            self.carrier_read(field, derived)
        };
        let mut body: Vec<String> = Vec::new();
        match &field.bit_size {
            BitSize::Fixed { bits } => {
                let n = bits.div_ceil(8);
                let endian = field.effective_endian(self.m().default_endian);
                let source = self.carrier(field, &value);
                if n == 1 {
                    body.push(format!("w.write_u8({source})?;"));
                } else {
                    for e in encode_multibyte_unified(
                        &source,
                        n,
                        endian,
                        crate::generator::Language::Rust,
                    ) {
                        body.push(format!("w.write_u8({e})?;"));
                    }
                }
            }
            BitSize::Tail | BitSize::LengthRef => {
                body.push(format!("w.write_bytes({})?;", self.bytes_of(field, &value)));
            }
            BitSize::Vle { width_bits } => {
                body.push(format!("w.write_vle_u{width_bits}({value})?;"));
            }
            BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                let list = list.expect("the plan walks a list it writes");
                let child = self
                    .child(field)
                    .expect("`check` found the codec a list holds");
                // The inputs of the entry codec that the chain supplies: one
                // value for each entry, 1 when the entry just written has the
                // identifier the bind waits for, and 0 for the first.
                let binds: Vec<(String, String, u64)> = list
                    .previous
                    .iter()
                    .map(|(input, previous)| {
                        (
                            input.clone(),
                            format!("_prev_{}", filters::to_snake_case(input.clone())),
                            *previous,
                        )
                    })
                    .collect();
                for (_, local, _) in &binds {
                    body.push(format!("let mut {local}: u8 = 0;"));
                }
                let supplied = |name: &str| {
                    binds
                        .iter()
                        .find(|(input, _, _)| input == name)
                        .map(|(_, local, _)| local.clone())
                };
                let args = child_alias(field)
                    .map(|alias| self.thread_args_supplying(alias, derived, &supplied))
                    .unwrap_or_default();
                let pass = self.child_write(child_alias(field).unwrap_or_default(), &field.id);
                let updates: Vec<String> = binds
                    .iter()
                    .map(|(_, local, previous)| {
                        format!(
                            "    {local} = u8::from({} == {previous}u64);",
                            self.entry_identifier(field, "&_e")
                        )
                    })
                    .collect();
                // A chain whose membership a group holds is walked as slots:
                // each entry written as it is, and the named one, whose
                // identifier is all of it, written from the kind's entry. The
                // identifier the named entry has is the key, which a bind of
                // the entry after it reads.
                if let Some(key) = list.slots {
                    let named = format!("{}::named()", key_type(self.name(), &field.id, key));
                    let named_updates: Vec<String> = binds
                        .iter()
                        .map(|(_, local, previous)| {
                            format!("{local} = u8::from({key}u64 == {previous}u64);")
                        })
                        .collect();
                    let (head, overrides) = match self.chain_end_flag(list) {
                        Some(member) => (
                            format!("for (_i, _s) in {value}.enumerate() {{"),
                            format!(
                                ", &{}::{member}(_i + 1 < {})",
                                child.overrides_path,
                                self.count_local(field)
                            ),
                        ),
                        None => (format!("for _s in {value} {{"), String::new()),
                    };
                    let write = if overrides.is_empty() {
                        "write_source"
                    } else {
                        "write_source_in"
                    };
                    body.push(head);
                    body.push("    match _s {".to_string());
                    body.push(format!("        {RUNTIME_CODEC}::Slot::Entry(_e) => {{"));
                    body.push(format!(
                        "            {}::{write}(&_e, w{args}{overrides}){pass};",
                        child.type_name
                    ));
                    body.extend(updates.iter().map(|u| format!("        {u}")));
                    body.push("        }".to_string());
                    body.push(format!("        {RUNTIME_CODEC}::Slot::Named => {{"));
                    body.push(format!(
                        "            {}::{write}(&{named}, w{args}{overrides}){pass};",
                        child.type_name
                    ));
                    body.extend(named_updates.iter().map(|u| format!("            {u}")));
                    body.push("        }".to_string());
                    body.push("    }".to_string());
                    body.push("}".to_string());
                } else {
                    match self.chain_end_flag(list) {
                        // Every entry but the last says another follows: the flag is
                        // derived from where the entry stands, and set as it is
                        // written.
                        Some(member) => {
                            let count = self.count_local(field);
                            // A list held to its law is walked through it, which
                            // holds each entry to the law again (docs/adr/0065 §6 R8).
                            let walk = if list.lawful {
                                format!("{value}.into_iter()")
                            } else {
                                value.clone()
                            };
                            body.push(format!("for (_i, _e) in {walk}.enumerate() {{"));
                            body.push(format!(
                            "    {}::write_source_in(&_e, w{args}, &{}::{member}(_i + 1 < {count})){pass};",
                            child.type_name, child.overrides_path
                        ));
                        }
                        None => {
                            body.push(format!("for _e in {value} {{"));
                            body.push(format!(
                                "    {}::write_source(&_e, w{args}){pass};",
                                child.type_name
                            ));
                        }
                    }
                    body.extend(updates);
                    body.push("}".to_string());
                }
            }
            BitSize::Embed => {
                let child = self
                    .child(field)
                    .expect("`check` found the codec a body holds");
                let args = child_alias(field)
                    .map(|alias| self.thread_args(alias, derived))
                    .unwrap_or_default();
                body.push(format!(
                    "{}::write_source({value}, w{args}){};",
                    child.type_name,
                    self.child_write(child_alias(field).unwrap_or_default(), &field.id)
                ));
            }
        }
        match guard {
            Guard::Present => {
                lines.push(format!("if let Some(_v) = {} {{", self.read_src(field)));
                lines.extend(body.into_iter().map(|line| format!("    {line}")));
                lines.push("}".to_string());
            }
            // Derived whole and gated: written where the field it gates is
            // present is the gate's business, and this field's gate is one flag
            // of a carrier, read as the codec reads it.
            Guard::Gate => lines.extend(self.guarded_by_flag(field, body, derived)),
            Guard::Always => lines.extend(body),
        }
    }

    /// A derived field that is itself gated: written when its gate flag is set,
    /// as the gate flag the codec computes says.
    fn guarded_by_flag(
        &self,
        field: &CodecField,
        body: Vec<String>,
        derived: &Derived,
    ) -> Vec<String> {
        let gate = field.present_if.as_ref().expect("a gated field has a gate");
        let mut out = vec![format!("if {} {{", self.gate_test(gate, derived))];
        out.extend(body.into_iter().map(|line| format!("    {line}")));
        out.push("}".to_string());
        out
    }

    fn carrier_read(&self, field: &CodecField, derived: &Derived) -> String {
        self.read(field, derived)
    }

    /// The `&[u8]` a bytes or text value is written as: the slice or `str` a
    /// field's `Bytes` or `Text` holds (docs/adr/0077 §3), its bounds having
    /// been the source's to keep.
    fn bytes_of(&self, field: &CodecField, value: &str) -> String {
        let value = format!("{value}.into_inner()");
        if field.is_string() {
            format!("{value}.as_bytes()")
        } else {
            value
        }
    }
}

/// What the prologue computed: the value of each derived element.
#[derive(Default)]
struct Derived {
    /// Fields derived whole, by id: the local that holds the value.
    fields: std::collections::BTreeMap<String, String>,
    /// Flags of a carrier that are derived, by carrier id. The carrier's value
    /// is the one its source presents, with those bit-ranges replaced.
    flags: std::collections::BTreeMap<String, Vec<DerivedFlag>>,
}

struct DerivedFlag {
    flag: String,
    /// A local that holds the number (`Number`) or the truth (`Bool`).
    value: String,
    /// The Rust type of that local, so that a conversion to the carrier's type
    /// is written only where the two differ.
    ty: String,
    kind: FlagValue,
}

enum FlagValue {
    Number,
    Bool,
    /// A number there may be none of: the carrier keeps the bits the source
    /// presents when there is not.
    Optional,
}

/// What a derived number is measured from: the local that holds the measure (0
/// while the content is absent), and, for content that may be absent, the test
/// that it is there.
struct Measure {
    number: String,
    present: Option<String>,
}

impl Derived {
    /// Fold the derived flags of each carrier into the carrier's value, which
    /// the wire then reads as `_c_<carrier>`. Emitted after the flags' values
    /// exist and before the first write.
    fn close(&self, plan: &Plan<'_>, lines: &mut Vec<String>) {
        for (carrier_id, flags) in &self.flags {
            let carrier = plan
                .m()
                .fields
                .iter()
                .find(|f| &f.id == carrier_id)
                .expect("a derived flag belongs to a carrier of the codec");
            let ty = plan.l().type_name(&carrier.sce_type);
            // The carrier as the source presents it, each derived bit-range
            // replaced in turn. `compound` is whether `value` needs parentheses
            // to be the left operand of the next `&`.
            let mut value = format!("src.{}()", plan.accessor(carrier));
            let mut compound = false;
            // Flags that are a number only while there is content to measure.
            let mut optional: Vec<(&DerivedFlag, &FlagDef)> = Vec::new();
            for derived in flags {
                let def = carrier
                    .flags
                    .iter()
                    .find(|d| d.name == derived.flag)
                    .expect("a derived flag is a flag of its carrier");
                let width_mask = if def.width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << def.width) - 1
                };
                let mask = width_mask << def.bit;
                let element = format!("{carrier_id}.{}", derived.flag);
                let bits = match derived.kind {
                    FlagValue::Optional => {
                        optional.push((derived, def));
                        continue;
                    }
                    FlagValue::Bool => format!("{ty}::from({})", derived.value),
                    FlagValue::Number => {
                        // A number the flag's bit-range cannot hold is not a
                        // frame. Where the grammar bounds it within the bits (a
                        // known arm's tag, a parent's tag masked to them, a
                        // count held to its list's bound), there is nothing to
                        // check.
                        if plan
                            .encode
                            .totality
                            .refuses(RefusalKind::LengthOutOfRange, &element)
                        {
                            lines.push(format!(
                                "if {} > 0x{width_mask:X} {{\n    return Err({});\n}}",
                                derived.value,
                                plan.refuse(RefusalKind::LengthOutOfRange, &element)
                            ));
                        }
                        if derived.ty == ty {
                            // Already the carrier's type: a conversion to it
                            // is the identity.
                            derived.value.clone()
                        } else {
                            // Within the bit-range, so within the carrier's
                            // type: the conversion keeps the value.
                            let narrowed = format!(
                                "_v_{}_{}",
                                plan.l().codec_field_id(&carrier.id),
                                filters::to_snake_case(derived.flag.clone())
                            );
                            lines
                                .push(format!("let {narrowed}: {ty} = {} as {ty};", derived.value));
                            narrowed
                        }
                    }
                };
                // Drop the `<< 0` no-op for a flag at bit 0.
                let placed = if def.bit == 0 {
                    bits
                } else {
                    format!("({bits} << {})", def.bit)
                };
                let kept = if compound {
                    format!("({value})")
                } else {
                    value
                };
                value = format!("({kept} & !0x{mask:X}) | {placed}");
                compound = true;
            }
            // The flags a parent sets replace the bits the source presents.
            let sets: Vec<&FlagDef> = plan
                .overridable()
                .into_iter()
                .filter(|(c, _)| c.id == carrier.id)
                .map(|(_, def)| def)
                .collect();
            let local = plan.carrier_local(carrier);
            let binding = if sets.is_empty() && optional.is_empty() {
                "let"
            } else {
                "let mut"
            };
            lines.push(format!("{binding} {local}: {ty} = {value};"));
            for (derived, def) in optional {
                let width_mask = if def.width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << def.width) - 1
                };
                let element = format!("{carrier_id}.{}", derived.flag);
                // Within the bit-range, so within the carrier's type.
                let placed = shl(&format!("(_n as {ty})"), def.bit);
                let check = if plan
                    .encode
                    .totality
                    .refuses(RefusalKind::LengthOutOfRange, &element)
                {
                    format!(
                        "    if _n > 0x{width_mask:X} {{\n        return Err({});\n    }}\n",
                        plan.refuse(RefusalKind::LengthOutOfRange, &element)
                    )
                } else {
                    String::new()
                };
                lines.push(format!(
                    "if let Some(_n) = {} {{\n{check}    {local} = ({local} & !0x{mask:X}) | {placed};\n}}",
                    derived.value,
                    mask = width_mask << def.bit,
                ));
            }
            for def in sets {
                lines.push(format!(
                    "if let Some(_o) = over.{member} {{\n    {local} = ({local} & !0x{mask:X}) | {placed};\n}}",
                    member = plan.override_member(carrier, &def.name),
                    mask = 1u64 << def.bit,
                    placed = shl(&format!("{ty}::from(_o)"), def.bit),
                ));
            }
        }
    }
}

/// `value` shifted right by `bit`, with no shift at bit 0 (a shift by nothing is
/// an operation clippy refuses in generated code).
fn shr(value: &str, bit: u32) -> String {
    if bit == 0 {
        value.to_string()
    } else {
        format!("({value} >> {bit})")
    }
}

/// `value` shifted left by `bit`, with no shift at bit 0.
fn shl(value: &str, bit: u32) -> String {
    if bit == 0 {
        value.to_string()
    } else {
        format!("({value} << {bit})")
    }
}
