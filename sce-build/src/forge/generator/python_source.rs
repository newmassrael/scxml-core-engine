// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A Python value presents its elements as attributes, the members of a
//! `typing.Protocol` that names what a source of the codec is, and its encode
//! is a function over any source, `encode_source` of the codec's module: it
//! reads each element the source presents and derives every other element of
//! the frame from them, as the plan says, never from the value (docs/adr/0058
//! §1). A byte string or a text is presented as a subclass of the runtime's
//! `BoundedBytes` or `BoundedText` that names its bound, the abstract type of
//! §2 for Python: a class whose constructor checks, of an immutable value, so a
//! value of it holds its bound for as long as it lives and the encode over it
//! refuses nothing the bound decides. An element a gate decides is `None` where
//! it is absent, and the flag that says so is derived from it. What the encode
//! does refuse, it raises before the first byte, as the runtime's exception
//! class of the refusal's name.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::codec_roles::{ElementId, GateGroup, GateVar, InputSource};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Against, CheckSite, Clause, CodecPlan, Derive, Element, EncodePlan, Guard, HandedFrom,
    InputCheck, Measure, Measured, Narrow, Number, NumberInto, Offset, Presence, PresenceFlag,
    Value, Write,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) mod source_case;

/// The first form of `plan` the Python printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the Python printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums,
/// byte strings, texts and bodies of codecs it prints that are handed no flag
/// input, each always there or decided by a gate on a flag of the codec's own;
/// whose every derived element is either a length, a count or an encoded
/// length of the elements it measures, held in a field of its own and gated
/// beside its content where its content is, a flag set exactly where the one
/// field it gates is there, or a flag a body says; whose every flag input the
/// value says is said by the field gated on it alone or by a body it is handed
/// on to or by the arm of its variant; whose variant's arms are codecs it
/// prints, no two named ones of one class; and whose encode refuses only what
/// it raises itself, before the first
/// byte: a gate it cannot derive, an input a body or the entries of a list say
/// otherwise than they are handed, measures that disagree, a number past its
/// field, or what a codec it holds refuses. A CBOR
/// map and a content-line component keep their own templates'
/// encodes. `imports` are the codec's own, which say whether a codec it holds
/// is printed ([`ImportContext::python_printed`]).
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan, imports),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// The import of the codec a body of `plan` holds as `alias`, where the
/// Python backend prints it from its plan.
fn printed_import<'i>(imports: &'i [ImportContext], alias: &str) -> Option<&'i ImportContext> {
    imports
        .iter()
        .find(|import| import.alias == alias && import.python_printed)
}

/// [`unlowered_forms`] of a positional codec.
fn positional_forms(plan: &EncodePlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let m = plan.model;
    let mut forms = Vec::new();
    // A variant is presented as the arm the value holds, a value of the arm's
    // codec's class, the default arm a `{Codec}Default` of the tag it keeps and
    // its body; the class of the arm says which arm it is, so two arms whose
    // codecs are one class could not be told apart. Each arm is written by its
    // codec's own `encode_source`, which a codec the Python backend prints has.
    let mut classes: BTreeMap<(String, String), &str> = BTreeMap::new();
    for arm in &plan.arms {
        let Some(import) = printed_import(imports, arm.codec) else {
            forms.push(format!(
                "the arm `{}` is a codec the Python backend does not print",
                arm.codec
            ));
            continue;
        };
        if arm.tag.is_none() {
            continue;
        }
        let class = (import.namespace.clone(), import.type_name.clone());
        if let Some(other) = classes.insert(class, arm.codec) {
            forms.push(format!(
                "the arms `{other}` and `{}` are values of one Python class, which a value \
                 cannot tell apart",
                arm.codec
            ));
        }
    }
    // An input the value says (docs/adr/0065 §6 R4a) is read off it, by a
    // function of the module a parent calls, `input_<name>`, and by the encode
    // where a gate or a check reads it; one it does not say is a parameter of
    // the encode. It is said by the field gated on it alone, by the group
    // gated on it, by the body, always there, it is handed on to, or by the
    // arm of the variant the value holds, through that body's or arm's codec's
    // own function, which a codec the Python backend prints has.
    for said in &plan.said {
        let by = match &plan.roles.inputs[&said.input.name] {
            InputSource::Gate { .. }
            | InputSource::Forward { .. }
            | InputSource::Group
            | InputSource::EveryArm { .. }
            | InputSource::SomeArms { .. } => continue,
            InputSource::List { .. } => "the start of a list held to a law",
        };
        forms.push(format!(
            "input '{}': a flag input {by} says",
            said.input.name
        ));
    }
    for element in &plan.elements {
        forms.extend(value_forms(imports, &element.field.id, &element.value));
    }
    // A group (docs/adr/0065 §6 R4b, R4c) is presented as the dataclass of
    // its one arm's members, or the `Union` of a dataclass for each of its
    // arms, held optionally where the gates allow no arm; its gates are flags
    // of the codec's own or flag inputs, derived from the arm the value holds.
    // Its members are held as an element alone is, and two lists of one count
    // (R5) as one `Aligned` of the two.
    for group in plan.groups() {
        let label = super::rust_source::gates_label(group);
        if group.chain().is_some() {
            forms.push(format!("group '{label}': a group of a chain's memberships"));
        }
        for id in group.fields() {
            let member = m
                .fields
                .iter()
                .find(|f| f.id == id)
                .expect("a group's member is a field of the codec");
            forms.extend(value_forms(imports, id, &plan.value(member)));
        }
    }
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            // A number into a field, or into a flag of a carrier.
            (_, Derive::Number(number)) => {
                forms.extend(number_form(&derived.element, number));
            }
            (ElementId::Flag { .. }, Derive::Presence(presence)) => {
                forms.extend(presence_form(&derived.element, presence));
            }
            // A flag of a disjunction, what the arm the value holds gives it
            // (docs/adr/0065 §6 R4c).
            (ElementId::Flag { .. }, Derive::ArmFlag { .. }) => {}
            // The tag of the arm of the variant the value holds, into a field
            // or a flag of a carrier, and the bits of a carrier the arm of
            // each embedded body's variant fills, each read through that
            // body's codec's own `tag`.
            (ElementId::Field(_) | ElementId::Flag { .. }, Derive::VariantTag) => {}
            (ElementId::Flag { .. }, Derive::ParentTag(_)) => {}
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a length, \
                 a presence flag, a group's flag or a variant's tag"
            )),
        }
    }
    // A gate the encode cannot derive the presence from is a check the value
    // is held to before the first byte, as the encode writes the carriers it
    // reads.
    for field in &plan.gate_checks {
        forms.extend(gate_form(plan, field));
    }
    // An input a body or the entries of a list say, held before the first
    // byte to what this codec hands them from a carrier or an input of its
    // own; a value that says nothing of it disagrees with nothing
    // (docs/adr/0065 §6 R4e).
    for check in &plan.input_checks {
        match (&check.site, &check.against) {
            (_, Against::PreviousEntry { .. }) => forms.push(format!(
                "input '{}': an input a chain's entry says, held to the entry before",
                check.input
            )),
            (CheckSite::List { .. } | CheckSite::Embed { .. } | CheckSite::Arm { .. }, _) => {}
        }
    }
    let numbers = derived_whole(plan);
    for write in &plan.program {
        match write {
            Write::Whole(field) | Write::Tail { field, .. } => {
                forms.extend(unread_form(plan, &numbers, field));
            }
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, &numbers, field));
                }
            }
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &numbers, field));
                // A walk of each entry in turn; a chain that sets its entries'
                // flag, hands an entry what the one before it was, holds a
                // bare entry's place or is held to a law is not printed yet.
                if list.as_ref().is_some_and(|walk| {
                    !walk.previous.is_empty()
                        || walk.slots.is_some()
                        || walk.ends.is_some()
                        || walk.lawful
                }) {
                    forms.push(format!(
                        "field '{}': a chain whose walk sets or hands its entries a flag",
                        field.id
                    ));
                }
                if *guard == Guard::Gate {
                    forms.extend(gate_form(plan, field));
                }
                match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        let whole =
                            BitSpan::of(field, m.default_endian).is_some_and(|span| span.whole());
                        if !whole {
                            forms.push(format!(
                                "field '{}': a streamed field of part of its bytes",
                                field.id
                            ));
                        }
                    }
                    BitSize::Vle {
                        width_bits: 16 | 32 | 64,
                    }
                    | BitSize::Tail
                    | BitSize::LengthRef => {}
                    BitSize::Vle { width_bits } => forms.push(format!(
                        "field '{}': a variable-length integer of {width_bits} bits",
                        field.id
                    )),
                    // A body or a list, which the elements above name.
                    BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {}
                }
            }
            // The variant above names it.
            Write::Variant => {}
        }
    }
    forms.dedup();
    forms
}

/// What of `id`, an element of the abstract type `value` the value presents
/// alone or a group of it holds, the printer does not print: it prints an
/// integer, a flag set, an enum, a byte string, a text, and a body or a list
/// of a codec the Python backend prints, held to no law.
fn value_forms(imports: &[ImportContext], id: &str, value: &Value<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    match value {
        Value::Scalar(t) if t.int_bit_width().is_some() => {}
        Value::Scalar(SceType::Enum(_)) => {}
        Value::Scalar(t) => forms.push(format!("field '{id}': a `{}` field", t.as_attr())),
        Value::Bytes(_) | Value::Text(_) => {}
        // A body is written by its codec's own `encode_source`, which a codec
        // the Python backend prints has, handed each input it takes and does
        // not say from a carrier or an input of this codec's.
        Value::Body { codec } => {
            if printed_import(imports, codec).is_none() {
                forms.push(format!(
                    "field '{id}': a body of `{codec}`, which the Python backend does not print"
                ));
            }
        }
        // A list is walked, each entry written by its codec's own
        // `encode_source`, which a codec the Python backend prints has, handed
        // as a body is.
        Value::List { codec, law, .. } => {
            if printed_import(imports, codec).is_none() {
                forms.push(format!(
                    "field '{id}': a list of `{codec}`, which the Python backend does not print"
                ));
            }
            if !law.is_none() {
                forms.push(format!("field '{id}': a list held to a law"));
            }
        }
    }
    forms
}

/// What of the derived number of `element` the printer does not print: it
/// prints a length, a count or an encoded length of the elements it measures,
/// or the input a body says,
/// each of which that is there says the same number or the value is refused,
/// into a field that holds nothing else, refused where it may not fit; where
/// nothing it measures is there the field holds zero, gated beside its
/// content.
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    for measure in &number.measures {
        match &measure.of {
            // The input a body or the arm the value holds says, read through
            // its codec's own function.
            Measured::Bytes { .. }
            | Measured::Entries { .. }
            | Measured::Encoded { .. }
            | Measured::Says { .. }
            | Measured::ArmSays { .. } => {}
            Measured::Start { .. } => {
                forms.push(format!("element '{element}': a flag a list's start says"));
            }
        }
    }
    // A number into a field holds, where nothing it measures is there, zero,
    // what the value states, or what the arm of a group that holds it on the
    // side without its content does (docs/adr/0065 §6 R4b); one into a flag is
    // folded into its carrier, as a presence flag is.
    forms
}

/// What of the presence flag `element` the printer does not print: it prints
/// a flag the presence of one field decides, or of fields one group holds.
fn presence_form(element: &ElementId, presence: &PresenceFlag<'_>) -> Option<String> {
    (presence.of.len() != 1 && !presence.together)
        .then(|| format!("element '{element}': a flag the presence of several fields decides"))
}

/// What of the gate of `field`, a field written where its gate holds, the
/// printer does not print: it tests a flag of the codec's own carriers or an
/// input the encode is handed.
fn gate_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    let gate = field.present_if.as_ref()?;
    plan.clauses(gate)
        .iter()
        .any(|clause| matches!(clause, Clause::Member { .. }))
        .then(|| format!("field '{}': a gate on a chain's entries", field.id))
}

/// The fields of `plan` whose value the encode derives as a number.
fn derived_numbers(plan: &EncodePlan<'_>) -> BTreeSet<String> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(derived.by, Derive::Number(_)))
        .filter_map(|derived| match &derived.element {
            ElementId::Field(id) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// The fields of `plan` the encode derives whole, each written as its local
/// `_d_<field>`: a number, or the tag of the arm of the variant the value
/// holds.
fn derived_whole(plan: &EncodePlan<'_>) -> BTreeSet<String> {
    let mut whole = derived_numbers(plan);
    for derived in &plan.derivations {
        if let (ElementId::Field(id), Derive::VariantTag) = (&derived.element, &derived.by) {
            whole.insert(id.clone());
        }
    }
    whole
}

/// Whether the printer cannot read `field` as a write reads it: the value
/// presents it alone or as a member of a group, or the encode derives it
/// whole.
fn unread_form(
    plan: &EncodePlan<'_>,
    numbers: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    (element_of(plan, &field.id).is_none()
        && plan.roles.group_of(&field.id).is_none()
        && !numbers.contains(&field.id))
    .then(|| {
        format!(
            "field '{}': a field the value neither presents nor the encode derives as a number",
            field.id
        )
    })
}

/// The element of `plan` the value presents as `id`, if it presents one.
fn element_of<'p, 'm>(plan: &'p EncodePlan<'m>, id: &str) -> Option<&'p Element<'m>> {
    plan.elements.iter().find(|e| e.field.id == id)
}

/// Whether the Python value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no field `id` of its own: the encode derives it (docs/adr/0081 §3)
/// and the value does not state it while its content is absent, or a group
/// holds it (docs/adr/0065 §6 R4), which is the value's field.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    (derived_whole(plan).contains(id) || plan.roles.group_of(id).is_some())
        && element_of(plan, id).is_none()
}

/// The expression the encode reads the element `id` by: an attribute of the
/// source, or, for a member of a group, the local the encode binds it to off
/// the arm the value holds, `None` where that arm does not hold it.
fn element_read(plan: &EncodePlan<'_>, l: &LangCtx, id: &str) -> String {
    let name = l.codec_field_id(id);
    if plan.roles.group_of(id).is_some() {
        format!("_e_{name}")
    } else {
        format!("src.{name}")
    }
}

/// The fields the number the value states as `id` measures, each gated: the
/// value states the number only while none of them is there.
fn stated_contents<'m>(plan: &EncodePlan<'m>, id: &str) -> Vec<&'m CodecField> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(&derived.element, ElementId::Field(field) if field == id))
        .filter_map(|derived| match &derived.by {
            Derive::Number(number) => Some(number),
            _ => None,
        })
        .flat_map(|number| number.measures.iter())
        .map(|measure| match &measure.of {
            Measured::Bytes { field, .. } => *field,
            Measured::Entries { list } => *list,
            Measured::Encoded { body, .. } => *body,
            _ => unreachable!("a number the value states is a length or a count of gated content"),
        })
        .collect()
}

/// The Python text the printer gives a codec's module.
pub(super) struct PythonSource {
    /// Before the value's class: the class of each byte string's and text's
    /// bound, the `Protocol` a source of the codec satisfies, and the encode
    /// over any source, `encode_source`.
    pub(super) before: String,
    /// The value's `__setattr__`, which holds each byte string and text to its
    /// bound wherever it is set, where the value is made and after; empty
    /// where it presents none.
    pub(super) hold: String,
    /// For each byte string, text and body the value presents, by field id,
    /// the type the value's field is declared as and the default it takes.
    pub(super) declared: BTreeMap<String, (String, String)>,
    /// The runtime's classes the module names, joined for its import; empty
    /// where it names none.
    pub(super) runtime: String,
    /// The modules of the printed codecs the value holds a body of, each
    /// imported whole under its alias, one statement a line; empty where it
    /// holds none.
    pub(super) modules: String,
    /// Whether a default of the value's fields is made by a factory, a body's
    /// class, which the module imports `field` of `dataclasses` for.
    pub(super) factory: bool,
    /// For each element the decode makes other than as the local it read, by
    /// field id, the expression it makes it of.
    pub(super) decoded: BTreeMap<String, String>,
    /// Each gate group the value holds as one field (docs/adr/0065 §6 R4), in
    /// the order of the plan's groups; its members are no fields of their own.
    pub(super) groups: Vec<GroupField>,
    /// Whether the module names `typing.Union`, the type of a group of several
    /// arms and of a variant.
    pub(super) union: bool,
    /// The variant the value holds in its `body`, where the codec has one.
    pub(super) variant: Option<VariantField>,
}

/// The variant as the value holds it: its `body`, of the type
/// `{Codec}Variant`, the `Union` of a class for each arm.
pub(super) struct VariantField {
    /// The default `body` takes: a factory of the arm the document marks as
    /// the one a value starts in, or else of the first.
    pub(super) default: String,
    /// The class of the default arm, which the decode makes of the tag it
    /// read and the body, where the variant has one.
    pub(super) default_arm: Option<String>,
}

/// The attribute a value holds its variant's arm in, as every printer names
/// it (`VARIANT_MEMBER`).
const VARIANT: &str = super::rust_source::VARIANT_MEMBER;

impl PythonSource {
    /// What the codec's template reads of the printed text, as its
    /// `python_source`: the template names each part, and the printer says
    /// what each holds.
    pub(super) fn context(&self) -> serde_json::Value {
        serde_json::json!({
            "before": self.before,
            "hold": self.hold,
            "runtime": self.runtime,
            "modules": self.modules,
            "factory": self.factory,
            "union": self.union,
            "groups": self
                .groups
                .iter()
                .map(|group| serde_json::json!({
                    "name": group.name,
                    "ty": group.ty,
                    "default": group.default,
                    "decoded": group.decoded,
                }))
                .collect::<Vec<_>>(),
            "variant": self.variant.as_ref().map(|variant| serde_json::json!({
                "default": variant.default,
                "default_arm": variant.default_arm,
            })),
        })
    }
}

/// A gate group as the value holds it: one field of the value's class.
pub(super) struct GroupField {
    /// The field's name, the group's accessor.
    pub(super) name: String,
    /// The type it is declared as.
    pub(super) ty: String,
    /// The default it takes.
    pub(super) default: String,
    /// The expression the decode makes it of, over the locals it read each
    /// member and carrier into and the inputs it is handed.
    pub(super) decoded: String,
}

/// The module alias a printed codec's module is imported under where a
/// printed parent writes a body of it (`python_module_alias`), and the class
/// of its value.
fn body_module(import: &ImportContext) -> (String, &str) {
    (python_module_alias(&import.namespace), &import.type_name)
}

/// One arm of the codec's variant as the printed module reads it off the arm
/// a value holds: by the class of the arm, a value of the arm's codec's class
/// or, for the default arm, the `{Codec}Default` that keeps its tag beside its
/// body.
struct ArmRead {
    /// The class `isinstance` tests the arm a value holds against.
    class: String,
    /// The alias the module imports the arm's codec's module under, whose
    /// `encode_source` and `input_<name>` read the arm's body.
    module: String,
    /// The import alias of the arm's codec, which the plan names it by.
    alias: String,
    /// The tag the arm names, `None` for the default arm, which keeps its own.
    tag: Option<u64>,
}

impl ArmRead {
    /// The arm's body, a source of its codec, read off `arm`, what the value
    /// holds where it holds this arm: the arm itself, or the default arm's
    /// body.
    fn body(&self, arm: &str) -> String {
        match self.tag {
            Some(_) => arm.to_string(),
            None => format!("{arm}.body"),
        }
    }

    /// The tag of the arm, read off `arm` for the default arm.
    fn tag(&self, arm: &str) -> String {
        match self.tag {
            Some(tag) => tag.to_string(),
            None => format!("{arm}.tag"),
        }
    }
}

/// The class of the default arm of the variant of the codec `struct_name`.
fn default_arm_class(struct_name: &str) -> String {
    format!("{struct_name}Default")
}

/// The local the encode reads the arm the value holds by.
const ARM: &str = "_v_body";

/// The arms of the variant of `plan`, in the plan's order (those it names,
/// then the default), each a codec the Python backend prints; empty where the
/// codec has no variant.
fn variant_arms(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    struct_name: &str,
) -> Vec<ArmRead> {
    plan.arms
        .iter()
        .map(|arm| {
            let import = printed_import(imports, arm.codec)
                .expect("`unlowered` prints a variant whose every arm is printed");
            let (module, class) = body_module(import);
            ArmRead {
                class: match arm.tag {
                    Some(_) => format!("{module}.{class}"),
                    None => default_arm_class(struct_name),
                },
                module,
                alias: arm.codec.to_string(),
                tag: arm.tag,
            }
        })
        .collect()
}

/// A Python expression that is `each(arm)` of the arm `held` holds among
/// `arms`, each tested by its class but the last, which is what `held` holds
/// where it holds none of the others; `otherwise` where `held` may hold an arm
/// outside `arms`, which then ends the chain.
fn arm_chain(
    arms: &[&ArmRead],
    held: &str,
    otherwise: Option<&str>,
    each: &dyn Fn(&ArmRead) -> String,
) -> String {
    let mut chain = String::new();
    for (i, arm) in arms.iter().enumerate() {
        if i + 1 == arms.len() && otherwise.is_none() {
            chain.push_str(&each(arm));
        } else {
            chain.push_str(&format!(
                "{} if isinstance({held}, {}) else ",
                each(arm),
                arm.class
            ));
        }
    }
    if let Some(otherwise) = otherwise {
        chain.push_str(otherwise);
    }
    format!("({chain})")
}

/// The class a byte string, a text or a list of `struct_name` is held in, the
/// runtime class it is a subclass of, what it counts, and a literal of its
/// fewest `least` units, which a list has none of: a list of a least past zero
/// is a chain closed by its entries' flag, which the printer does not print.
fn bounded_class(
    struct_name: &str,
    id: &str,
    value: &Value<'_>,
    least: u64,
) -> (String, &'static str, &'static str, String) {
    let pascal = filters::to_pascal_case(id.to_string());
    // The fewest units the class holds, zeros, spelled as a literal: the
    // default is evaluated in the class body, where a field named `bytes`
    // declared before it would hide the builtin.
    match value {
        Value::Text(_) => (
            format!("{struct_name}{pascal}Text"),
            "BoundedText",
            "UTF-8 bytes",
            format!("\"\\x00\" * {least}"),
        ),
        Value::List { .. } => (
            format!("{struct_name}{pascal}List"),
            "BoundedTuple",
            "entries",
            "()".to_string(),
        ),
        _ => (
            format!("{struct_name}{pascal}Bytes"),
            "BoundedBytes",
            "bytes",
            format!("b\"\\x00\" * {least}"),
        ),
    }
}

/// The Python text of the codec `plan`, whose value's class is `struct_name`;
/// the plan is one [`unlowered`] answers `None` for.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> PythonSource {
    let mut gathered = Gathered::default();
    let mut hold = String::new();
    let mut declared = BTreeMap::new();
    let mut decoded = BTreeMap::new();
    // The type the source presents each element as.
    let mut presented: BTreeMap<String, String> = BTreeMap::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let gated = element.presence == Presence::Gated;
        // A number the value states only while the content it measures is
        // absent (docs/adr/0058 §1): `None` while it is there, which the
        // encode then derives from it, and the decode gives back `None` where
        // it read any of it.
        if element.presence == Presence::Stated {
            let optional = format!("Optional[{}]", l.type_name(&element.field.sce_type));
            presented.insert(id.clone(), optional.clone());
            declared.insert(id.clone(), (optional, "None".to_string()));
            let contents: Vec<String> = stated_contents(plan, id)
                .iter()
                .map(|content| format!("{} is None", l.codec_field_decode_local(&content.id)))
                .collect();
            decoded.insert(
                id.clone(),
                format!(
                    "{} if {} else None",
                    l.codec_field_decode_local(id),
                    contents.join(" and ")
                ),
            );
            continue;
        }
        let held = held_as(
            &mut gathered,
            l,
            imports,
            struct_name,
            element.field,
            &element.value,
            gated,
        );
        presented.insert(id.clone(), held.presented);
        if let Some(line) = held.hold {
            hold.push_str(&hold_line(&hold, &l.codec_field_id(id), &line));
        }
        // An integer, a flag set or an enum takes the default the template
        // gives every field of its type.
        if !matches!(element.value, Value::Scalar(_)) {
            declared.insert(id.clone(), (held.ty, held.default));
        }
    }
    // Each group, its arms' classes before the source that presents it.
    let mut groups = Vec::new();
    let mut union = false;
    for group in plan.groups() {
        let (field, presents) = group_part(
            &mut gathered,
            plan,
            l,
            imports,
            struct_name,
            group,
            &mut union,
        );
        presented.insert(field.name.clone(), presents);
        groups.push(field);
    }
    // The variant, its classes after the groups'.
    let variant = variant_part(&mut gathered, plan, imports, struct_name);
    if variant.is_some() {
        union = true;
        presented.insert(VARIANT.to_string(), format!("{struct_name}Variant"));
    }
    let Gathered {
        classes: mut before,
        runtime,
        modules,
        factory,
    } = gathered;

    // The source: what the encode reads, each element by the attribute a value
    // holds it in, `None` where a gate leaves it out.
    before.push_str(&format!(
        "class {struct_name}Source(Protocol):\n\
         \x20   \"\"\"The elements a value of :class:`{struct_name}` presents, which its encode\n\
         \x20   reads (docs/adr/0081 §2): any object that presents them is a source of it.\n\
         \x20   The encode derives every other element of the frame.\"\"\"\n"
    ));
    let names = plan
        .elements
        .iter()
        .map(|element| (l.codec_field_id(&element.field.id), &element.field.id))
        .chain(groups.iter().map(|group| (group.name.clone(), &group.name)));
    let variant_key = VARIANT.to_string();
    let names = names.chain(
        variant
            .is_some()
            .then(|| (variant_key.clone(), &variant_key)),
    );
    for (name, key) in names {
        before.push_str(&format!(
            "\n    @property\n    def {name}(self) -> {}: ...\n",
            presented[key]
        ));
    }

    // A dataclass's constructor sets each field as an attribute, so one
    // `__setattr__` holds a byte string or a text to its bound where the value
    // is made and wherever it is set after.
    let hold = setattr(&hold);

    PythonSource {
        before: format!(
            "{}\n\n\n{}{}{}",
            before.trim_end_matches('\n'),
            encode_source(plan, l, imports, struct_name),
            input_functions(plan, l, imports, struct_name),
            tag_function(plan, imports, struct_name)
        ),
        variant,
        hold: hold.trim_end_matches('\n').to_string(),
        declared,
        runtime: runtime.into_iter().collect::<Vec<_>>().join(", "),
        modules: modules.into_iter().collect::<Vec<_>>().join("\n"),
        factory,
        decoded,
        groups,
        union,
    }
}

/// What the declarations of a module's elements gather as they are printed:
/// the classes before the source (each bound's, each group arm's), the
/// runtime's classes they name, the modules of the codecs they hold, and
/// whether a default is made by a factory.
#[derive(Default)]
struct Gathered {
    /// The classes, each followed by two blank lines.
    classes: String,
    /// The runtime's classes named.
    runtime: BTreeSet<&'static str>,
    /// The `from . import` statements of the held codecs' modules.
    modules: BTreeSet<String>,
    /// Whether a default is made by `field(default_factory=..)`.
    factory: bool,
}

/// How a class of the module holds one element where it is there.
struct Held {
    /// The type a source presents it as.
    presented: String,
    /// The type the class declares it as.
    ty: String,
    /// The default the class gives it.
    default: String,
    /// Where a class holds its bound, what `__setattr__` makes of `value`.
    hold: Option<String>,
    /// The class of its bound, where it has one.
    class: Option<String>,
}

/// How a class holds `field`, of the abstract type `value`, `Optional` where
/// `gated`: a body as any source of its codec, held as its codec's value, made
/// by default by its class; a byte string, a text or a list as the class of
/// its bound, which `__setattr__` makes wherever it is set; anything else as
/// its declared type. The classes and modules it names go into `gathered`.
fn held_as(
    gathered: &mut Gathered,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
    field: &CodecField,
    value: &Value<'_>,
    gated: bool,
) -> Held {
    let optional = |ty: &str| {
        if gated {
            format!("Optional[{ty}]")
        } else {
            ty.to_string()
        }
    };
    let id = &field.id;
    let bound = match value {
        // A body is any source of its codec, which that codec's module names;
        // a class holds its codec's value, made by default by its class.
        Value::Body { codec } => {
            let import = printed_import(imports, codec)
                .expect("`unlowered` prints a body of a printed codec only");
            let (alias, class) = body_module(import);
            gathered
                .modules
                .insert(format!("from . import {} as {alias}", import.namespace));
            let default = if gated {
                "None".to_string()
            } else {
                gathered.factory = true;
                format!("field(default_factory={class})")
            };
            return Held {
                presented: optional(&format!("{alias}.{class}Source")),
                ty: optional(class),
                default,
                hold: None,
                class: None,
            };
        }
        Value::Scalar(ty) => {
            let ty = l.type_name(ty);
            return Held {
                presented: optional(&ty),
                ty: optional(&ty),
                default: if gated {
                    "None".to_string()
                } else {
                    l.default_expr(&field.sce_type)
                },
                hold: None,
                class: None,
            };
        }
        Value::Bytes(bound) | Value::Text(bound) => bound,
        // A list's entries are written by their codec's module, imported whole
        // as a body's is.
        Value::List { codec, bound, .. } => {
            let import = printed_import(imports, codec)
                .expect("`unlowered` prints a list of a printed codec only");
            gathered.modules.insert(format!(
                "from . import {} as {}",
                import.namespace,
                body_module(import).0
            ));
            bound
        }
    };
    let (class, base, unit, fewest) = bounded_class(struct_name, id, value, bound.least);
    gathered.runtime.insert(base);
    let held = match bound.most {
        Some(most) => format!("{} to {most} {unit}", bound.least),
        None => format!(
            "{} or more {unit}, the protocol stating no most",
            bound.least
        ),
    };
    let most = bound
        .most
        .map_or_else(|| "None".to_string(), |most| most.to_string());
    gathered.classes.push_str(&format!(
        "class {class}({base}):\n\
         \x20   \"\"\"The ``{id}`` element of :class:`{struct_name}`, {held},\n\
         \x20   held to that bound where a value of it is made (docs/adr/0077 §3).\"\"\"\n\
         \n\
         \x20   LEAST = {least}\n\
         \x20   MOST = {most}\n\n\n",
        least = bound.least
    ));
    let made = if gated {
        format!("None if value is None else {class}(value)")
    } else {
        format!("{class}(value)")
    };
    let default = if gated {
        "None".to_string()
    } else if bound.least == 0 {
        format!("{class}()")
    } else {
        format!("{class}({fewest})")
    };
    Held {
        presented: optional(&class),
        ty: optional(&class),
        default,
        hold: Some(made),
        class: Some(class),
    }
}

/// The branch of a `__setattr__` whose branches so far are `so_far` that makes
/// `made` of the value set as the attribute `name`.
fn hold_line(so_far: &str, name: &str, made: &str) -> String {
    let keyword = if so_far.is_empty() { "if" } else { "elif" };
    format!("        {keyword} name == \"{name}\":\n            value = {made}\n")
}

/// The attribute of an arm that holds the two lists `first` and `second` of
/// one count (docs/adr/0065 §6 R5) as one `Aligned`, named as every printer
/// names it (`aligned_name`).
fn aligned_member(l: &LangCtx, first: &str, second: &str) -> String {
    super::rust_source::aligned_name(&l.codec_field_id(first), &l.codec_field_id(second))
}

/// The `__setattr__` of a class whose branches are `hold`, which holds each
/// byte string, text and list to its bound wherever it is set, where a value
/// is made and after; empty where it has none.
fn setattr(hold: &str) -> String {
    if hold.is_empty() {
        return String::new();
    }
    format!(
        "    def __setattr__(self, name: str, value: object) -> None:\n\
         \x20       # Each byte string and text is held to its bound wherever it is\n\
         \x20       # set, where the value is made and after (docs/adr/0081 §2),\n\
         \x20       # whatever it was handed.\n\
         {hold}\
         \x20       object.__setattr__(self, name, value)\n"
    )
}

/// The class of each arm of `group`, a group of the codec whose value's class
/// is `struct_name`, in the plan's order: one for each arm of a sum, named
/// after the group's type and the arm, and the group's type itself for a group
/// of one arm.
fn arm_classes(struct_name: &str, group: &GateGroup) -> Vec<String> {
    let base = super::rust_source::group_type(struct_name, group);
    if group.is_sum() {
        group
            .arms
            .iter()
            .map(|arm| format!("{base}{}", super::rust_source::group_arm_name(group, arm)))
            .collect()
    } else {
        vec![base]
    }
}

/// The classes of the gate group `group` (docs/adr/0065 §6 R4b, R4c), into
/// `gathered`, and the field the value holds it in with the type a source
/// presents it as. Each arm is a dataclass of the members present while the
/// gates hold the arm's values, each held as an element alone is but always
/// there; a group of several arms is the `Union` of their classes (`union`
/// then set), which a value holds one of, or `None` where the gates allow no
/// arm. The decode makes the arm the gates it read select, from the locals it
/// read each member into.
fn group_part(
    gathered: &mut Gathered,
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
    group: &GateGroup,
    union: &mut bool,
) -> (GroupField, String) {
    let name = super::rust_source::group_accessor(group);
    let base = super::rust_source::group_type(struct_name, group);
    let classes = arm_classes(struct_name, group);
    // How each member is held, once, though several arms may hold it.
    let mut helds: BTreeMap<&str, Held> = BTreeMap::new();
    for id in group.fields() {
        let field = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == id)
            .expect("a group's member is a field of the codec");
        let held = held_as(
            gathered,
            l,
            imports,
            struct_name,
            field,
            &plan.value(field),
            false,
        );
        helds.insert(id, held);
    }
    let mut made = Vec::new();
    for (arm, class) in group.arms.iter().zip(&classes) {
        let when: Vec<String> = group
            .gates
            .iter()
            .zip(&arm.values)
            .map(|(gate, value)| {
                if *value {
                    format!("``{gate}`` holds")
                } else {
                    format!("``{gate}`` does not hold")
                }
            })
            .collect();
        let mut members = String::new();
        let mut hold = String::new();
        let mut args = Vec::new();
        for id in &arm.fields {
            // Two lists of one count are one member of an arm that holds both
            // (docs/adr/0065 §6 R5), where the first is: an `Aligned` of the
            // two, each held in the class of its bound.
            if group.is_second_in(arm, id) {
                continue;
            }
            if let Some(second) = group.second_of(arm, id) {
                let (one, two) = (&helds[id.as_str()], &helds[second]);
                let bound_of = |held: &Held| {
                    held.class
                        .clone()
                        .expect("two lists of one count are each held in the class of its bound")
                };
                let (first_class, second_class) = (bound_of(one), bound_of(two));
                let pair = aligned_member(l, id, second);
                gathered.runtime.insert("Aligned");
                members.push_str(&format!(
                    "    {pair}: Aligned[{first_class}, {second_class}] = Aligned({}, {})\n",
                    one.default, two.default
                ));
                hold.push_str(&hold_line(
                    &hold,
                    &pair,
                    &format!("Aligned({first_class}(value.first), {second_class}(value.second))"),
                ));
                args.push(format!(
                    "{pair}=Aligned({}, {})",
                    l.codec_field_decode_local(id),
                    l.codec_field_decode_local(second)
                ));
                continue;
            }
            let held = &helds[id.as_str()];
            let member = l.codec_field_id(id);
            members.push_str(&format!("    {member}: {} = {}\n", held.ty, held.default));
            if let Some(line) = &held.hold {
                hold.push_str(&hold_line(&hold, &member, line));
            }
            args.push(format!("{member}={}", l.codec_field_decode_local(id)));
        }
        let hold = setattr(&hold);
        gathered.classes.push_str(&format!(
            "@dataclass\n\
             class {class}:\n\
             \x20   \"\"\"The ``{name}`` group of :class:`{struct_name}` where {}\n\
             \x20   (docs/adr/0065 §6 R4): the members present then.\"\"\"\n\
             \n\
             {members}{}{hold}\n\n",
            when.join(" and "),
            if hold.is_empty() { "" } else { "\n" },
        ));
        made.push((
            format!("{class}({})", args.join(", ")),
            decoded_arm_test(plan, l, group, &arm.values),
        ));
    }
    if group.is_sum() {
        *union = true;
        gathered.classes.push_str(&format!(
            "{base} = Union[{}]\n\
             \"\"\"The ``{name}`` group of :class:`{struct_name}`: the arm the value holds.\"\"\"\n\n\n",
            classes.join(", ")
        ));
    }
    // The arms the gates select, in turn; where the gates may select none,
    // the value holds no arm.
    let mut decoded = String::new();
    let last = made.len() - 1;
    for (i, (arm, test)) in made.into_iter().enumerate() {
        if i == last && group.absent.is_none() {
            decoded.push_str(&arm);
        } else {
            decoded.push_str(&format!("{arm} if {test} else "));
        }
    }
    let (ty, default) = match &group.absent {
        Some(_) => {
            decoded.push_str("None");
            (format!("Optional[{base}]"), "None".to_string())
        }
        None => {
            gathered.factory = true;
            (base, format!("field(default_factory={})", classes[0]))
        }
    };
    (
        GroupField {
            name,
            ty: ty.clone(),
            default,
            decoded,
        },
        ty,
    )
}

/// Whether the gates of `group`, as the decode read them, hold `values`: each
/// flag tested on the local its carrier was read into, each input as the
/// decode is handed it.
fn decoded_arm_test(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    group: &GateGroup,
    values: &[bool],
) -> String {
    group
        .gates
        .iter()
        .zip(values)
        .map(|(gate, value)| {
            let holds = match gate {
                GateVar::Flag { carrier, flag } => {
                    let bit = plan
                        .model
                        .fields
                        .iter()
                        .find(|f| &f.id == carrier)
                        .and_then(|f| f.flags.iter().find(|d| &d.name == flag))
                        .map(|d| d.bit)
                        .expect("the parser found the flag a gate reads");
                    format!(
                        "(({} >> {bit}) & 1) != 0",
                        l.codec_field_decode_local(carrier)
                    )
                }
                GateVar::Input(name) => format!("{} != 0", input_name(name)),
                GateVar::Member { .. } => {
                    unreachable!("`unlowered` prints no group of a chain's memberships")
                }
            };
            if *value {
                format!("({holds})")
            } else {
                format!("not ({holds})")
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Whether the gate at `gate` among those of `group` holds where the value
/// holds `held`, the group as it holds it: it holds one of the arms that set
/// the gate, or no arm where the gates the value then has set it.
fn gate_holds(struct_name: &str, group: &GateGroup, gate: usize, held: &str) -> String {
    let set: Vec<String> = arm_classes(struct_name, group)
        .into_iter()
        .zip(&group.arms)
        .filter(|(_, arm)| arm.values[gate])
        .map(|(class, _)| class)
        .collect();
    let some = match set.as_slice() {
        [] => None,
        [one] => Some(format!("isinstance({held}, {one})")),
        many => Some(format!("isinstance({held}, ({}))", many.join(", "))),
    };
    let none = group.absent.as_ref().is_some_and(|absent| absent[gate]);
    match (some, none) {
        (None, false) => "False".to_string(),
        (None, true) => format!("{held} is None"),
        (Some(some), false) => some,
        (Some(some), true) => format!("({held} is None or {some})"),
    }
}

/// The classes of the variant of `plan`, into `gathered`, and how the value
/// holds it, where the codec has one: the arm a value holds is a value of the
/// arm's codec's class, imported whole as a body's is, or a `{Codec}Default`
/// of the tag the default arm keeps, held to the tags no arm names
/// (`{Codec}UnknownTag`, docs/adr/0065 §6 R3), and its body; the variant is the
/// `Union` of the arms' classes, `{Codec}Variant`.
fn variant_part(
    gathered: &mut Gathered,
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    struct_name: &str,
) -> Option<VariantField> {
    let variant = plan.model.variant.as_ref()?;
    let arms = variant_arms(plan, imports, struct_name);
    let mut default_arm = None;
    for arm in &arms {
        let import = printed_import(imports, &arm.alias)
            .expect("`unlowered` prints a variant whose every arm is printed");
        gathered.modules.insert(format!(
            "from . import {} as {}",
            import.namespace, arm.module
        ));
        if arm.tag.is_some() {
            continue;
        }
        let unknown = crate::forge::codec_totality::unknown_tag(plan.model)
            .expect("a variant with a default arm holds its tag as an unknown tag");
        let tag_class = format!("{struct_name}UnknownTag");
        let mut known = unknown.known.clone();
        known.sort_unstable();
        known.dedup();
        let known = if known.is_empty() {
            "frozenset()".to_string()
        } else {
            format!(
                "frozenset({{{}}})",
                known
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let least = (0..=unknown.max()).find(|tag| unknown.admits(*tag)).expect(
            "a default arm is reached by a tag no arm names (codec/variant-arm-unreachable)",
        );
        let body = format!("{}.{}", arm.module, import.type_name);
        let class = default_arm_class(struct_name);
        gathered.runtime.insert("UnknownTag");
        gathered.factory = true;
        gathered.classes.push_str(&format!(
            "class {tag_class}(UnknownTag):\n\
             \x20   \"\"\"The tag the default arm of :class:`{struct_name}`'s variant keeps: at\n\
             \x20   most {most}, and none an arm names (docs/adr/0065 §6 R3).\"\"\"\n\
             \n\
             \x20   MOST = {most}\n\
             \x20   KNOWN = {known}\n\n\n\
             @dataclass\n\
             class {class}:\n\
             \x20   \"\"\"The default arm of :class:`{struct_name}`'s variant: the tag no arm\n\
             \x20   names, which it keeps, and its body.\"\"\"\n\
             \n\
             \x20   tag: {tag_class} = {tag_class}({least})\n\
             \x20   body: {body} = field(default_factory={body})\n\
             \n\
             \x20   def __setattr__(self, name: str, value: object) -> None:\n\
             \x20       # The tag is held to the tags no arm names wherever it is set,\n\
             \x20       # where the arm is made and after (docs/adr/0065 §6 R3).\n\
             \x20       if name == \"tag\":\n\
             \x20           value = {tag_class}(value)\n\
             \x20       object.__setattr__(self, name, value)\n\n\n",
            most = unknown.max(),
        ));
        default_arm = Some(class);
    }
    let classes: Vec<&str> = arms.iter().map(|arm| arm.class.as_str()).collect();
    gathered.classes.push_str(&format!(
        "{struct_name}Variant = Union[{}]\n\
         \"\"\"The arm a value of :class:`{struct_name}` holds: a value of the arm's codec,\n\
         or the default arm.\"\"\"\n\n\n",
        classes.join(", ")
    ));
    // The arm a value starts in: the one the document marks, or the first.
    let start = variant
        .arms
        .iter()
        .position(|arm| arm.is_default)
        .unwrap_or(0);
    gathered.factory = true;
    Some(VariantField {
        default: format!("field(default_factory={})", classes[start]),
        default_arm,
    })
}

/// The module's `tag`, over any source of the codec of `plan`, where its
/// variant takes its tag from its caller: the tag of the arm the source holds,
/// the one the arm names or the one the default arm keeps. It is what a parent
/// that holds the codec writes where the arm says the bits of its carrier, and
/// the tag a caller hands the decode. A variant with a tag of its own writes
/// it itself, and has none, as in every printer. It follows the text before
/// it, two blank lines apart; empty where the codec has no such variant.
fn tag_function(plan: &EncodePlan<'_>, imports: &[ImportContext], struct_name: &str) -> String {
    if !plan
        .model
        .variant
        .as_ref()
        .is_some_and(|variant| variant.tag_field.is_none())
    {
        return String::new();
    }
    let arms = variant_arms(plan, imports, struct_name);
    let mut cases = String::new();
    for arm in &arms {
        cases.push_str(&format!(
            "    if isinstance(_v, {}):\n        return {}\n",
            arm.class,
            arm.tag("_v")
        ));
    }
    format!(
        "\n\n\ndef tag(src: {struct_name}Source) -> int:\n\
         \x20   \"\"\"The tag of the arm a source of :class:`{struct_name}` holds: the one the\n\
         \x20   arm names, or the one the default arm keeps. It is what a parent writes\n\
         \x20   where the arm says its bits, and the tag a caller hands the decode.\n\
         \x20   Raises :class:`TypeError` for a source whose ``{VARIANT}`` is no arm.\"\"\"\n\
         \x20   _v = src.{VARIANT}\n\
         {cases}\
         \x20   raise TypeError(f\"{{type(_v).__name__}} is no arm of {struct_name}'s variant\")"
    )
}

/// `value`, the bytes or text `field` holds, as the bytes it is written as.
fn wire_bytes(field: &CodecField, value: &str) -> String {
    if field.sce_type == SceType::String {
        format!("{value}.encode(\"utf-8\")")
    } else {
        value.to_string()
    }
}

/// Each line of `body` one level deeper.
fn indented(body: &str) -> String {
    body.lines().map(|line| format!("    {line}\n")).collect()
}

/// The encode of `struct_name` over any source of it, a function of the
/// module, which is the codec's namespace as a C++ codec's namespace is: each
/// derived number made from what it measures and each derived flag folded into
/// its carrier, then the writes in wire order, each element read off the
/// source. A function of the module and not a method of the value's class, so
/// no field of the value, a class attribute, can take its name.
///
/// A body whose encoded length the codec carries is encoded once, into a
/// buffer whose length is the number, and the buffer is what is written where
/// the body is; any other body is encoded straight into the sink.
fn encode_source(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let numbers = derived_numbers(plan);
    let (stated, derived): (Vec<&String>, Vec<&String>) = numbers
        .iter()
        .partition(|id| element_of(plan, id).is_some() || plan.roles.group_of(id).is_some());
    let named = |ids: &[&String]| {
        ids.iter()
            .map(|id| format!("``{id}``"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut derives = String::new();
    if !derived.is_empty() {
        derives.push_str(&format!(
            " It derives {} from what each measures, and never reads it off\n    the source.",
            named(&derived)
        ));
    }
    if !stated.is_empty() {
        derives.push_str(&format!(
            " It derives {} where what it measures is there, and reads\n    what the source \
             states where none is.",
            named(&stated)
        ));
    }
    // What the encode refuses, each refusal the runtime's exception class:
    // nothing, or what the plan's totality names, before the first byte.
    let refuses = if plan.totality.is_total() {
        " It refuses no value;".to_string()
    } else {
        let mut raised: Vec<String> = plan
            .totality
            .refusals()
            .iter()
            .map(|r| match r.kind.error_variant() {
                Some(variant) => format!(":class:`{variant}`"),
                None => "what a codec it holds raises".to_string(),
            })
            .collect();
        raised.sort();
        raised.dedup();
        format!(
            " It raises\n    {} for a value no frame carries, before\n    it writes a byte to ``w``;",
            raised.join(", ")
        )
    };
    // The flag inputs the encode is handed, those the value does not say,
    // after the sink, as the decode is handed them after the cursor.
    let given: String = plan
        .given
        .iter()
        .map(|input| format!(", {}: int", input_name(&input.name)))
        .collect();
    let mut out = format!(
        "def encode_source(src: {struct_name}Source, w: SceSink{given}) -> None:\n\
         \x20   \"\"\"Write any source of :class:`{struct_name}` into ``w``, each element read\n\
         \x20   off ``src`` and written where the codec's encode plan lays it out\n\
         \x20   (docs/adr/0081).{derives}{refuses} a bounded sink raises\n\
         \x20   :class:`BufferOverflow` where it has no room left.\"\"\"\n"
    );
    let source = |id: &str| element_read(plan, l, id);
    // Each group read once, as `_g_<group>`, and each of its members bound to
    // `_e_<member>` off the arm the value holds where that arm holds it, and
    // `None` where it does not, so each statement that reads a member reads
    // it as it reads an element a gate decides.
    for group in plan.groups() {
        let name = super::rust_source::group_accessor(group);
        let held = format!("_g_{name}");
        out.push_str(&format!("    {held} = src.{name}\n"));
        let classes = arm_classes(struct_name, group);
        for id in group.fields() {
            // How each arm that holds the member reads it off the arm: as its
            // attribute, or as one of the two lists of one count it holds as
            // one `Aligned` (docs/adr/0065 §6 R5); the arms that read it alike
            // are tested together.
            let mut reads: Vec<(String, Vec<&str>)> = Vec::new();
            for (class, arm) in classes.iter().zip(&group.arms) {
                if !arm.fields.iter().any(|f| f == id) {
                    continue;
                }
                let read = if let Some(second) = group.second_of(arm, id) {
                    format!("{held}.{}.first", aligned_member(l, id, second))
                } else if group.is_second_in(arm, id) {
                    let (first, _) = group
                        .aligned
                        .as_ref()
                        .expect("the second of two lists of one count is aligned");
                    format!("{held}.{}.second", aligned_member(l, first, id))
                } else {
                    format!("{held}.{}", l.codec_field_id(id))
                };
                match reads.iter_mut().find(|(r, _)| *r == read) {
                    Some((_, arms)) => arms.push(class.as_str()),
                    None => reads.push((read, vec![class.as_str()])),
                }
            }
            let every = reads.len() == 1 && reads[0].1.len() == group.arms.len();
            let bound = if every && group.absent.is_none() {
                reads.remove(0).0
            } else if every {
                format!("None if {held} is None else {}", reads[0].0)
            } else {
                let mut bound = String::new();
                for (read, arms) in &reads {
                    let tested = match arms.as_slice() {
                        [one] => one.to_string(),
                        many => format!("({})", many.join(", ")),
                    };
                    bound.push_str(&format!("{read} if isinstance({held}, {tested}) else "));
                }
                bound.push_str("None");
                bound
            };
            out.push_str(&format!("    {} = {bound}\n", source(id)));
        }
    }
    // The arm of the variant the value holds, read once and held to being one
    // of the arms, so each statement after it that tests which arm it is
    // takes the last arm it tests for as the one held where it is none of
    // the others.
    let arms = variant_arms(plan, imports, struct_name);
    if !arms.is_empty() {
        let classes: Vec<&str> = arms.iter().map(|arm| arm.class.as_str()).collect();
        out.push_str(&format!(
            "    {ARM} = src.{VARIANT}\n\
             \x20   if not isinstance({ARM}, ({},)):\n\
             \x20       raise TypeError(f\"{{type({ARM}).__name__}} is no arm of {struct_name}'s variant\")\n",
            classes.join(", ")
        ));
    }
    // Each input the value says that a gate or a check of the codec reads,
    // read off the source under the name the gate reads it by, as a parent
    // reads it through `input_<name>`; one no gate reads is left to the parent
    // that asks for it.
    for said in plan.said.iter().filter(|said| said.read) {
        if plan.roles.says_input_partly(&said.input.name) {
            unreachable!("`{}`: {}", said.input.name, FORWARD_READ_BY_NO_GATE);
        }
        out.push_str(&format!(
            "    {} = {}\n",
            input_name(&said.input.name),
            said_input(plan, l, imports, struct_name, &said.input.name)
        ));
    }
    // The `encode_source` of the module that writes the body `field`, or each
    // entry of the list `field`.
    let body_writer = |field: &CodecField| -> String {
        let import = printed_import(imports, held_alias(plan, field))
            .expect("`unlowered` prints a codec it holds printed");
        format!("{}.encode_source", body_module(import).0)
    };
    // The bodies encoded into a buffer first, by field id: those whose length
    // a number measures.
    let mut buffered: BTreeSet<String> = BTreeSet::new();
    // Each flag the encode derives, by its carrier: the statements that fold
    // it into the carrier's local, `_c_<carrier>`, in place of the bits the
    // value presents.
    let mut folds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // The numbers that measure a body's frame, made once the carriers its
    // bodies may be handed are as the encode writes them.
    let mut encoded: Vec<(&ElementId, &Number<'_>)> = Vec::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (element, Derive::Number(number))
                if number
                    .measures
                    .iter()
                    .any(|m| matches!(m.of, Measured::Encoded { .. })) =>
            {
                encoded.push((element, number));
            }
            (element, Derive::Number(number)) => {
                let measured: Vec<MeasureText> = number
                    .measures
                    .iter()
                    .map(|measure| plain_measure(plan, l, imports, &arms, measure))
                    .collect();
                if let Some((carrier, statement)) =
                    derive_number(plan, l, element, &number.into, &measured, &mut out)
                {
                    folds.entry(carrier).or_default().push(statement);
                }
            }
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let def = plan
                    .model
                    .fields
                    .iter()
                    .find(|f| f.id == *field)
                    .and_then(|carrier| carrier.flags.iter().find(|d| d.name == *flag))
                    .expect("the parser found the flag a gate reads");
                let mask = ((1u64 << def.width) - 1) << def.bit;
                let test = if *when_clear {
                    "is None"
                } else {
                    "is not None"
                };
                let c = format!("_c_{}", l.codec_field_id(field));
                folds.entry(field.clone()).or_default().push(format!(
                    "{c} = ({c} & ~0x{mask:X}) | (0x{mask:X} if {} {test} else 0)\n",
                    source(&gated.id)
                ));
            }
            // A flag of a disjunction that decides a group (docs/adr/0065 §6
            // R4c): set where the value holds an arm that sets it, or no arm
            // where the gates then set it.
            (ElementId::Flag { field, flag }, Derive::ArmFlag { group, gate }) => {
                let group = &plan.groups()[*group];
                let def = plan
                    .model
                    .fields
                    .iter()
                    .find(|f| f.id == *field)
                    .and_then(|carrier| carrier.flags.iter().find(|d| d.name == *flag))
                    .expect("the parser found the flag a gate reads");
                let mask = ((1u64 << def.width) - 1) << def.bit;
                let held = format!("_g_{}", super::rust_source::group_accessor(group));
                let c = format!("_c_{}", l.codec_field_id(field));
                folds.entry(field.clone()).or_default().push(format!(
                    "{c} = ({c} & ~0x{mask:X}) | (0x{mask:X} if {} else 0)\n",
                    gate_holds(struct_name, group, *gate, &held)
                ));
            }
            // The tag of the arm the value holds: the one the arm names, or
            // the one the default arm keeps, into the field that holds it or
            // masked to the bits of its flag.
            (element, Derive::VariantTag) => {
                let every: Vec<&ArmRead> = arms.iter().collect();
                let tag_of = |arm: &ArmRead| arm.tag(ARM);
                out.push_str(&format!(
                    "    _t_body = {}\n",
                    arm_chain(&every, ARM, None, &tag_of)
                ));
                match element {
                    ElementId::Field(id) => {
                        out.push_str(&format!("    _d_{} = _t_body\n", l.codec_field_id(id)));
                    }
                    ElementId::Flag { field, flag } => {
                        let (mask, width_mask, bit) = flag_masks(plan, field, flag);
                        let c = format!("_c_{}", l.codec_field_id(field));
                        folds.entry(field.clone()).or_default().push(format!(
                            "{c} = ({c} & ~0x{mask:X}) | ((_t_body & 0x{width_mask:X}) << {bit})\n"
                        ));
                    }
                    ElementId::Variant => unreachable!(
                        "a variant whose caller hands its tag derives none: the caller writes it"
                    ),
                }
            }
            // The bits of a carrier the arm of each embedded body's variant
            // fills: the tag of the arm the body holds, read through its
            // codec's own `tag`, which a variant whose caller hands its tag
            // has, masked to the bits; nothing where a gate leaves the body
            // out.
            (ElementId::Flag { field, flag }, Derive::ParentTag(tag)) => {
                let local = format!("_p_{}", derived_local(l, &derived.element));
                out.push_str(&format!("    {local} = 0\n"));
                for term in &tag.terms {
                    let import = printed_import(imports, &term.codec)
                        .expect("`unlowered` prints a body of a printed codec only");
                    let body = element_read(plan, l, &term.body.id);
                    let fill = format!(
                        "{local} |= {}.tag({body}) & 0x{:X}\n",
                        body_module(import).0,
                        term.mask
                    );
                    if term.body.present_if.is_some()
                        || plan.roles.group_of(&term.body.id).is_some()
                    {
                        out.push_str(&indented(&format!(
                            "if {body} is not None:\n{}",
                            indented(&fill)
                        )));
                    } else {
                        out.push_str(&indented(&fill));
                    }
                }
                let (mask, width_mask, bit) = flag_masks(plan, field, flag);
                let c = format!("_c_{}", l.codec_field_id(field));
                folds.entry(field.clone()).or_default().push(format!(
                    "{c} = ({c} & ~0x{mask:X}) | (({local} & 0x{width_mask:X}) << {bit})\n"
                ));
            }
            _ => unreachable!("`unlowered` prints no other derivation"),
        }
    }
    // Each carrier as the encode writes it: its flags the value presents, and
    // those the encode derives folded in, in place of the value's.
    for (carrier, carried) in &folds {
        let name = l.codec_field_id(carrier);
        out.push_str(&format!("    _c_{name} = {}\n", source(carrier)));
        for statement in carried {
            out.push_str(&indented(statement));
        }
    }
    let whole = derived_whole(plan);
    let read = |f: &CodecField| -> String {
        let name = l.codec_field_id(&f.id);
        if whole.contains(&f.id) {
            format!("_d_{name}")
        } else if folds.contains_key(&f.id) {
            format!("_c_{name}")
        } else {
            source(&f.id)
        }
    };
    // The arguments after the sink the codec imported as `alias` is handed:
    // for each input it takes and does not say, the bits of a carrier as the
    // encode writes it, or an input of this codec's own passed on.
    let handed = |alias: &str| -> String {
        plan.handed
            .get(alias)
            .map(|inputs| {
                inputs
                    .iter()
                    .map(|h| match &h.from {
                        Some(HandedFrom::Carrier {
                            carrier,
                            bit,
                            width,
                        }) => {
                            let mask = if *width >= 64 {
                                u64::MAX
                            } else {
                                (1u64 << width) - 1
                            };
                            format!(", ({} >> {bit}) & 0x{mask:X}", read(carrier))
                        }
                        Some(HandedFrom::Input { name }) => format!(", {}", input_name(name)),
                        None => unreachable!(
                            "`unlowered` prints no codec handed an input from the entry before"
                        ),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    // A gate, tested on the carriers as the encode writes them.
    let gate_test = |f: &CodecField| -> String {
        let gate = f
            .present_if
            .as_ref()
            .expect("a field written where its gate holds is gated");
        plan.clauses(gate)
            .iter()
            .map(|clause| match clause {
                Clause::Flag {
                    carrier,
                    bit,
                    negate,
                } => format!(
                    "(({} >> {bit}) & 1) {} 0",
                    read(carrier),
                    if *negate { "==" } else { "!=" }
                ),
                // An input the encode is handed.
                Clause::Input { name, negate } => format!(
                    "{} {} 0",
                    input_name(name),
                    if *negate { "==" } else { "!=" }
                ),
                Clause::Member { .. } => {
                    unreachable!("`unlowered` prints no gate on a chain's entries")
                }
            })
            .collect::<Vec<_>>()
            .join(" or ")
    };
    // What a value is held to before the first byte (docs/adr/0065 §2), so a
    // refusal leaves the sink untouched: each input a held codec says, to what
    // this codec hands it from a carrier as the encode writes it or from an
    // input of its own, and each gate the encode cannot derive the presence
    // from, to the field it gates.
    for check in &plan.input_checks {
        out.push_str(&indented(&input_check(
            plan, l, imports, &arms, check, &read,
        )));
    }
    for field in &plan.gate_checks {
        out.push_str(&format!(
            "    if ({}) != ({} is not None):\n        {}\n",
            gate_test(field),
            source(&field.id),
            raise(plan, RefusalKind::PresentIfMismatch, &field.id)
        ));
    }
    // Each number that measures a body's frame: the body encoded once into a
    // buffer the write of the body then writes, `None` where it is absent. A
    // body whose encode refuses raises here, before the first byte of this
    // frame, and after every check of this codec's own.
    for (element, number) in encoded {
        let mut measured: Vec<MeasureText> = Vec::new();
        for measure in &number.measures {
            let Measured::Encoded { body, .. } = &measure.of else {
                unreachable!("{}", ENCODED_ALONE)
            };
            let content = source(&body.id);
            let buffer = format!("_b_{}", l.codec_field_id(&body.id));
            if buffered.insert(body.id.clone()) {
                let encode = format!(
                    "{buffer} = bytearray()\n{}({content}, BytearraySink({buffer}){})\n",
                    body_writer(body),
                    handed(held_alias(plan, body))
                );
                if measure.gated {
                    out.push_str(&format!("    {buffer} = None\n"));
                    out.push_str(&format!(
                        "    if {content} is not None:\n{}",
                        indented(&indented(&encode))
                    ));
                } else {
                    out.push_str(&indented(&encode));
                }
            }
            measured.push(MeasureText {
                size: format!("len({buffer})"),
                present: measure.gated.then(|| format!("{buffer} is not None")),
            });
        }
        if derive_number(plan, l, element, &number.into, &measured, &mut out).is_some() {
            unreachable!("{}", ENCODED_INTO_FIELD);
        }
    }
    // A codec it holds can refuse the value as it is written, so the frame is
    // written whole first and handed to the caller's sink only once it is, as
    // a count over a sink that keeps nothing precedes the write in the other
    // printers (docs/adr/0081 §1): a refusal leaves the caller's sink as it
    // was.
    let mut staged = false;
    for r in plan.totality.refusals() {
        if r.kind == RefusalKind::Child {
            plan.totality.require(RefusalKind::Child, &r.element);
            staged = true;
        }
    }
    if staged {
        out.push_str("    _w = w\n    _frame = bytearray()\n    w = BytearraySink(_frame)\n");
    }
    let endian = plan.model.default_endian;
    for write in &plan.program {
        match write {
            Write::Whole(field) => {
                for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                    out.push_str(&format!("    w.write_u8({byte})\n"));
                }
            }
            Write::Byte { pieces, .. } => {
                out.push_str(&format!(
                    "    w.write_u8({})\n",
                    encode_byte_read_by(pieces, l, &read)
                ));
            }
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    out.push_str(&format!(
                        "    while w.position() < {}:\n        w.write_u8(0)\n",
                        field.byte_offset
                    ));
                }
                out.push_str(&format!(
                    "    w.write_bytes({})\n",
                    wire_bytes(field, &read(field))
                ));
            }
            Write::Streamed { field, guard, .. } => {
                let body = match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        encode_whole_field_read_as(field, endian, &read(field), l)
                            .iter()
                            .map(|byte| format!("    w.write_u8({byte})\n"))
                            .collect::<String>()
                    }
                    // The runtime's writer, as the template's encode calls it,
                    // at the body's indentation of a function of the module.
                    BitSize::Vle { width_bits } => {
                        let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                        format!(
                            "    {}\n",
                            vle_encode_block(&value, *width_bits, l.lang).trim()
                        )
                    }
                    BitSize::Tail | BitSize::LengthRef => {
                        format!("    w.write_bytes({})\n", wire_bytes(field, &read(field)))
                    }
                    // A body: the buffer its length was measured over, or its
                    // codec's encode over the source it is presented as.
                    BitSize::Embed if buffered.contains(&field.id) => {
                        format!("    w.write_bytes(_b_{})\n", l.codec_field_id(&field.id))
                    }
                    BitSize::Embed => {
                        format!(
                            "    {}({}, w{})\n",
                            body_writer(field),
                            read(field),
                            handed(held_alias(plan, field))
                        )
                    }
                    // A list or a chain walked entry by entry, each written by
                    // its codec's encode over the source the list holds it as.
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => format!(
                        "    for _entry in {}:\n        {}(_entry, w{})\n",
                        read(field),
                        body_writer(field),
                        handed(held_alias(plan, field))
                    ),
                };
                out.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => {
                        format!("    if {} is not None:\n{}", read(field), indented(&body))
                    }
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => format!("    if {}:\n{}", gate_test(field), indented(&body)),
                });
            }
            // The arm the value holds, written by its codec's encode over its
            // body, each handed what its codec takes and does not say.
            Write::Variant => {
                for (i, arm) in arms.iter().enumerate() {
                    let keyword = match i {
                        0 => format!("if isinstance({ARM}, {}):", arm.class),
                        _ if i + 1 == arms.len() => "else:".to_string(),
                        _ => format!("elif isinstance({ARM}, {}):", arm.class),
                    };
                    let write = format!(
                        "{}.encode_source({}, w{})\n",
                        arm.module,
                        arm.body(ARM),
                        handed(&arm.alias)
                    );
                    if arms.len() == 1 {
                        out.push_str(&indented(&write));
                    } else {
                        out.push_str(&format!("    {keyword}\n{}", indented(&indented(&write))));
                    }
                }
            }
        }
    }
    if staged {
        out.push_str("    _w.write_bytes(_frame)\n");
    }
    // Every refusal the totality analysis names is one the encode raises
    // (docs/adr/0065 §2).
    plan.totality.assert_all_written(struct_name);
    out.trim_end_matches('\n').to_string()
}

/// Each flag input the value says (docs/adr/0065 §6 R4a), read off any source
/// of it by a function of the module, `input_<name>`, which its encode is not
/// handed: what a parent that holds this codec derives the flag it binds to the
/// input from, or holds the flag to. Each follows the text before it, two blank
/// lines apart; empty where the value says none.
fn input_functions(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let mut out = String::new();
    for said in &plan.said {
        let name = &said.input.name;
        // A value that can say nothing of the input answers `None` then
        // (docs/adr/0065 §6 R4e).
        let (answer, nothing) = if plan.roles.says_input_partly(name) {
            (
                "Optional[int]",
                ",\n    or ``None`` where the value says nothing of it",
            )
        } else {
            ("int", "")
        };
        out.push_str(&format!(
            "\n\n\ndef input_{snake}(src: {struct_name}Source) -> {answer}:\n\
             \x20   \"\"\"The flag input ``{name}`` a source of :class:`{struct_name}` says, which\n\
             \x20   its encode is not handed (docs/adr/0065 §6 R4a){nothing}.\"\"\"\n\
             \x20   return {}\n",
            said_input(plan, l, imports, struct_name, name),
            snake = input_name(name),
        ));
    }
    out.trim_end_matches('\n').to_string()
}

/// The value of the flag input `input` the value says (docs/adr/0065 §6 R4a),
/// as an expression over `src`, any source of the codec: set exactly when the
/// field gated on it alone is there (clear, for a field gated on its clear
/// state), what the arm of the group gated on it gives it, or what the body it
/// is handed on to says, read through that body's codec's own `input_<name>`,
/// `None` where that body can say nothing of it. `struct_name` is the class of
/// the codec's value, which names its groups' arms. [`unlowered`] refuses every
/// other way a value says one.
fn said_input(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
    input: &str,
) -> String {
    match &plan.roles.inputs[input] {
        InputSource::Gate { field, when_clear } => {
            let (set, clear) = if *when_clear { (0, 1) } else { (1, 0) };
            format!(
                "({set} if src.{} is not None else {clear})",
                l.codec_field_id(field)
            )
        }
        InputSource::Forward {
            embed,
            alias,
            input,
            ..
        } => {
            let import = printed_import(imports, alias)
                .expect("`unlowered` prints a body of a printed codec only");
            format!(
                "{}.input_{}(src.{})",
                body_module(import).0,
                input_name(input),
                l.codec_field_id(embed)
            )
        }
        // Set where the value holds an arm of the group gated on it that sets
        // it, or no arm where the gates then set it (docs/adr/0065 §6 R4b).
        InputSource::Group => {
            let gate = GateVar::Input(input.to_string());
            let group = plan
                .groups()
                .iter()
                .find(|g| g.gates.contains(&gate))
                .expect("an input a group says has its group");
            let at = group
                .gates
                .iter()
                .position(|g| *g == gate)
                .expect("filtered to the group gated on it");
            let held = format!("src.{}", super::rust_source::group_accessor(group));
            format!(
                "(1 if {} else 0)",
                gate_holds(struct_name, group, at, &held)
            )
        }
        // What the arm the value holds says, read through that arm's codec
        // off its body (docs/adr/0065 §6 R4d); an arm whose codec does not
        // read the input has no bit it could change, and says nothing of it
        // (R4e).
        InputSource::EveryArm { arms: bound } | InputSource::SomeArms { arms: bound } => {
            let arms = variant_arms(plan, imports, struct_name);
            let saying: Vec<&ArmRead> = arms
                .iter()
                .filter(|arm| bound.contains_key(&arm.alias))
                .collect();
            let nothing = if plan.roles.says_input_partly(input) {
                "None"
            } else {
                "0"
            };
            let held = format!("src.{VARIANT}");
            arm_chain(&saying, &held, Some(nothing), &|arm: &ArmRead| {
                format!(
                    "{}.input_{}({})",
                    arm.module,
                    input_name(&bound[&arm.alias]),
                    arm.body(&held)
                )
            })
        }
        InputSource::List { .. } => {
            unreachable!("`unlowered` prints no other way a value says an input")
        }
    }
}

/// The statements, at the encode's top level less one indentation, that hold
/// the codec `check` names to what this codec hands it for its input
/// (docs/adr/0065 §6 R4a): `_h`, the bit of a carrier as the encode writes it
/// (`read`) or an input of this codec's own, against what the body or each
/// entry of the list, or the arm of the variant the value holds where it is
/// one of the arms checked, says through its codec's own `input_<name>`. A
/// value that says nothing of the input disagrees with nothing (R4e), and a
/// body or a list a gate leaves out holds nothing that could disagree.
fn input_check(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    arms: &[ArmRead],
    check: &InputCheck<'_>,
    read: &dyn Fn(&CodecField) -> String,
) -> String {
    let import = printed_import(imports, &check.codec)
        .expect("`unlowered` prints a codec whose input is checked only where it is printed");
    let says = |entry: &str| {
        format!(
            "{}.input_{}({entry})",
            body_module(import).0,
            input_name(&check.input)
        )
    };
    let fail = raise(plan, RefusalKind::PresentIfMismatch, &check.at);
    // Whether `entry` says the input otherwise than `_h`.
    let test = |entry: &str| {
        if check.partly {
            format!(
                "_s = {}\nif _s is not None and (_s != 0) != _h:\n    {fail}\n",
                says(entry)
            )
        } else {
            format!("if ({} != 0) != _h:\n    {fail}\n", says(entry))
        }
    };
    let handed = match &check.against {
        Against::Carrier { carrier, bit } => format!("(({} >> {bit}) & 1) != 0", read(carrier)),
        Against::Input { name } => format!("{} != 0", input_name(name)),
        Against::PreviousEntry { .. } => {
            unreachable!("`unlowered` prints no input held to the entry before")
        }
    };
    let (held, statements) = match &check.site {
        CheckSite::List { list } => {
            let entries = element_read(plan, l, &list.id);
            let walk = format!("for _x in {entries}:\n{}", indented(&test("_x")));
            (list, walk)
        }
        CheckSite::Embed { body } => (body, test(&element_read(plan, l, &body.id))),
        // The arm the value holds, where it is one the check names; every
        // arm is there, so nothing gates it.
        CheckSite::Arm { arms: checked } => {
            let mut statements = String::new();
            for (n, &i) in checked.iter().enumerate() {
                let arm = &arms[i];
                let keyword = if n == 0 { "if" } else { "elif" };
                statements.push_str(&format!(
                    "{keyword} isinstance({ARM}, {}):\n{}",
                    arm.class,
                    indented(&test(&arm.body(ARM)))
                ));
            }
            return format!("_h = {handed}\n{statements}");
        }
    };
    // A gated element, or a member of a group the arm the value holds may not
    // hold, is read where it is there.
    let statements = if held.present_if.is_some() || plan.roles.group_of(&held.id).is_some() {
        format!(
            "if {} is not None:\n{}",
            element_read(plan, l, &held.id),
            indented(&statements)
        )
    } else {
        statements
    };
    format!("_h = {handed}\n{statements}")
}

/// Why the encode never reads an input a value may say nothing of: the roles
/// forward an input only where no gate of the codec names it and one bind alone
/// hands it on (`codec_roles::said_inputs`), and the body that forwards it is
/// derived from it rather than checked against it, so neither a gate nor a
/// check reads it (`Said::read`); only a parent asks for it.
const FORWARD_READ_BY_NO_GATE: &str = "no gate or check of the codec reads an input a body \
     forwards (codec_roles::said_inputs), so only a parent asks for it";

/// Why a flag derived from the input a body says is read off a body that says
/// it in every value: the roles hold a body that says the input in some of its
/// values only to the flag by a check, and derive the flag only from one that
/// says it in every value (`codec_roles::classify`).
const SAYS_IN_EVERY_VALUE: &str = "the roles derive a flag from a body only where it says the \
     input in every value, and check the others (codec_roles::classify)";

/// The import alias of the codec the body `field`, or each entry of the list
/// `field`, of `plan` is a value of.
fn held_alias<'f>(plan: &EncodePlan<'_>, field: &'f CodecField) -> &'f str {
    match plan.value(field) {
        Value::Body { codec } | Value::List { codec, .. } => codec,
        _ => unreachable!("`{}` is a body or a list the value presents", field.id),
    }
}

/// The Python name of the flag input `name`: the parameter the encode and the
/// decode are handed it by.
fn input_name(name: &str) -> String {
    filters::to_snake_case(name.to_string())
}

/// A measure of a derived number that reads the source alone: the bytes of a
/// byte string or a text, less or more its `sce:length-arith`, the entries of
/// a list, or the input a body says (docs/adr/0065 §6 R4a), read through the
/// body's codec's own `input_<name>`; each tested present where a gate may
/// leave it out.
fn plain_measure(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    arms: &[ArmRead],
    measure: &Measure<'_>,
) -> MeasureText {
    let (content, size) = match &measure.of {
        Measured::Bytes { field, offset } => {
            let arith = match offset {
                Offset::None => String::new(),
                Offset::More { by } => format!(" + {by}"),
                Offset::Less { by, held: true } => format!(" - {by}"),
                Offset::Less { held: false, .. } => {
                    unreachable!("`{}`: {}", field.id, LESS_IS_HELD)
                }
            };
            let content = element_read(plan, l, &field.id);
            let size = format!("len({}){arith}", wire_bytes(field, &content));
            (content, size)
        }
        // How many entries the list holds, within its bound.
        Measured::Entries { list } => {
            let content = element_read(plan, l, &list.id);
            let size = format!("len({content})");
            (content, size)
        }
        // The input the body says, 0 or 1; where a gate leaves the body out,
        // it says nothing, and a field beside it holds zero and a flag keeps
        // the bits the value presents.
        Measured::Says { body, codec, input } => {
            let import = printed_import(imports, codec)
                .expect("`unlowered` prints a body of a printed codec only");
            if import.codec_partly_says_inputs.contains(input) {
                unreachable!("`{}`: {}", body.id, SAYS_IN_EVERY_VALUE);
            }
            let content = element_read(plan, l, &body.id);
            let size = format!(
                "{}.input_{}({content})",
                body_module(import).0,
                input_name(input)
            );
            (content, size)
        }
        Measured::Encoded { .. } => unreachable!("a number of encoded lengths is counted apart"),
        // The input the arm the value holds says, 0 or 1, read through the
        // arm's codec off its body, where the value holds one of the arms
        // that say it; where it holds another, a field beside it holds zero
        // and a flag keeps the bits the value presents.
        Measured::ArmSays { arms: said } => {
            let mut saying: Vec<&ArmRead> = Vec::new();
            for input in said {
                let import = printed_import(imports, &input.codec)
                    .expect("`unlowered` prints a variant whose every arm is printed");
                if import.codec_partly_says_inputs.contains(&input.input) {
                    unreachable!("`{}`: {}", input.codec, SAYS_IN_EVERY_VALUE);
                }
                saying.push(&arms[input.arm]);
            }
            let size = arm_chain(&saying, ARM, Some("0"), &|arm: &ArmRead| {
                let input = said
                    .iter()
                    .find(|one| arms[one.arm].alias == arm.alias)
                    .expect("each arm that says the input names it");
                format!(
                    "{}.input_{}({})",
                    arm.module,
                    input_name(&input.input),
                    arm.body(ARM)
                )
            });
            let classes: Vec<&str> = saying.iter().map(|arm| arm.class.as_str()).collect();
            return MeasureText {
                size,
                present: measure
                    .gated
                    .then(|| format!("isinstance({ARM}, ({},))", classes.join(", "))),
            };
        }
        Measured::Start { .. } => {
            unreachable!("`unlowered` prints no flag a list's start says")
        }
    };
    MeasureText {
        size,
        present: measure.gated.then(|| format!("{content} is not None")),
    }
}

/// Why a number that measures a body's frame measures nothing else: the roles
/// refuse declarations of two kinds that name one element as ambiguous.
const ENCODED_ALONE: &str = "a number of encoded lengths measures nothing else \
     (codec_roles::classify)";

/// Why an encoded length is held in a field: `length-from` names a field
/// declared before the embed, never a flag (`validate_codec_embed_length_from`).
const ENCODED_INTO_FIELD: &str = "length-from names a field declared before the embed, never a \
     flag (validate_codec_embed_length_from)";

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, which is read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
}

/// Why a length the wire holds less a constant is never past its content: a
/// length-ref field's bound takes its fewest bytes from that constant, so the
/// class of its bytes holds at least what the wire takes off
/// (`codec_totality::held_to_offset`, docs/adr/0077 §3), and the analysis
/// names no refusal at the content.
const LESS_IS_HELD: &str = "a length-ref's bound holds at least the bytes its length-arith takes \
     off (LengthBound::of), so the length is never negative";

/// The statement that raises the refusal of `kind` at `at`, the runtime's
/// exception class of the refusal's name, which the totality analysis names
/// (`Totality::require`, docs/adr/0065 §2).
fn raise(plan: &EncodePlan<'_>, kind: RefusalKind, at: &str) -> String {
    let error = plan.totality.require(kind, at);
    let class = error.trim_start_matches("CodecError::");
    format!("raise {class}(\"{at}\")")
}

/// The largest value the integer type `ty` holds, or `None` for another type.
fn largest_of(ty: &SceType) -> Option<u64> {
    let bits = ty.int_bit_width()?;
    let bits = if ty.is_signed() { bits - 1 } else { bits };
    Some(if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    })
}

/// The bits of the flag `flag` of the carrier `carrier`: the mask of them in
/// place, the mask of their width, and the lowest bit.
fn flag_masks(plan: &EncodePlan<'_>, carrier: &str, flag: &str) -> (u64, u64, u32) {
    let def = plan
        .model
        .fields
        .iter()
        .find(|f| f.id == carrier)
        .and_then(|c| c.flags.iter().find(|d| d.name == flag))
        .expect("a derived flag is a flag of a carrier of the codec");
    let width_mask = if def.width >= 64 {
        u64::MAX
    } else {
        (1u64 << def.width) - 1
    };
    (width_mask << def.bit, width_mask, def.bit)
}

/// The name the encode's locals of the derived `element` are named by.
fn derived_local(l: &LangCtx, element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => l.codec_field_id(id),
        ElementId::Flag { field, flag } => format!(
            "{}_{}",
            l.codec_field_id(field),
            filters::to_snake_case(flag.clone())
        ),
        ElementId::Variant => unreachable!("the plan derives nothing into the variant"),
    }
}

/// The statements, written into `out`, that make the derived number of
/// `element` from what it measures, as the Rust encode's `combine` makes it
/// (docs/adr/0081 §1): the first measure always there fixes it, and every
/// other one there must say the same, or the value is refused as
/// `InconsistentSource`; where every measure may be absent, it is the first
/// one there. A field holds it as `_d_<field>`, refused as `LengthOutOfRange`
/// where it may not fit; where nothing it measures is there, the field holds
/// zero or what the value states. A number held in a flag is answered as the
/// carrier and the statement that folds it into the carrier's local.
fn derive_number(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    element: &ElementId,
    into: &NumberInto<'_>,
    measures: &[MeasureText],
    out: &mut String,
) -> Option<(String, String)> {
    let named = element.to_string();
    let disagree = || raise(plan, RefusalKind::InconsistentSource, &named);
    let there = |m: &MeasureText| {
        m.present
            .clone()
            .expect("a measure that may be absent has a test that it is there")
    };
    // The number, and where it may not be there, the test that it is.
    let (size, present) = match measures {
        // Nothing measured: zero, as the Rust encode's `combine` holds it.
        [] => ("0".to_string(), None),
        [one] => (one.size.clone(), one.present.clone()),
        _ => {
            let always: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_none()).collect();
            let gated: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_some()).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    out.push_str(&format!(
                        "    if {} != {}:\n        {}\n",
                        other.size,
                        anchor.size,
                        disagree()
                    ));
                }
                for g in &gated {
                    out.push_str(&format!(
                        "    if {} and {} != {}:\n        {}\n",
                        there(g),
                        g.size,
                        anchor.size,
                        disagree()
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let first = format!("_f_{}", derived_local(l, element));
                out.push_str(&format!("    {first} = None\n"));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "elif" };
                    out.push_str(&format!(
                        "    {keyword} {}:\n        {first} = {}\n",
                        there(g),
                        g.size
                    ));
                }
                for g in gated.iter().skip(1) {
                    out.push_str(&format!(
                        "    if {} and {} != {first}:\n        {}\n",
                        there(g),
                        g.size,
                        disagree()
                    ));
                }
                let found = format!("{first} is not None");
                (first, Some(found))
            }
        }
    };
    let (field, narrow, absent) = match (element, into) {
        (
            ElementId::Field(_),
            NumberInto::Field {
                field,
                narrow,
                absent,
            },
        ) => (field, narrow, absent),
        // Folded into its carrier in place of the bits the value presents,
        // where it is there; where it is free and nothing it measures is
        // there, the carrier keeps the value's bits. A number the flag's bits
        // cannot hold is no frame, where the grammar does not bound it within
        // them.
        (ElementId::Flag { field, flag }, NumberInto::Flag) => {
            let def = plan
                .model
                .fields
                .iter()
                .find(|f| f.id == *field)
                .and_then(|carrier| carrier.flags.iter().find(|d| d.name == *flag))
                .expect("a number held in a flag is held in a flag of its carrier");
            let width_mask = if def.width >= 64 {
                u64::MAX
            } else {
                (1u64 << def.width) - 1
            };
            let mask = width_mask << def.bit;
            let c = format!("_c_{}", l.codec_field_id(field));
            let mut set = String::new();
            if plan.totality.refuses(RefusalKind::LengthOutOfRange, &named) {
                set.push_str(&format!(
                    "if {size} > 0x{width_mask:X}:\n    {}\n",
                    raise(plan, RefusalKind::LengthOutOfRange, &named)
                ));
            }
            set.push_str(&format!(
                "{c} = ({c} & ~0x{mask:X}) | (({size}) << {})\n",
                def.bit
            ));
            let statement = match &present {
                None => set,
                Some(there) => format!("if {there}:\n{}", indented(&set)),
            };
            return Some((field.clone(), statement));
        }
        _ => unreachable!("a number goes into the field or the flag it names"),
    };
    let name = l.codec_field_id(&field.id);
    // A number past what the field holds is no frame (docs/adr/0065 §5).
    if *narrow == Narrow::Checked {
        let most = largest_of(&field.sce_type)
            .expect("a length that may not fit its field is held in an integer field");
        let past = format!("{size} > 0x{most:X}");
        let test = match &present {
            Some(there) => format!("{there} and {past}"),
            None => past,
        };
        out.push_str(&format!(
            "    if {test}:\n        {}\n",
            raise(plan, RefusalKind::LengthOutOfRange, &named)
        ));
    }
    // Where the content is not, zero, the field gated beside it and not
    // written, or what the value states: alone, or in the arm of the group
    // that holds it on the side without its content (docs/adr/0065 §6 R4b),
    // which the arms keep apart from the content, so the two never both say
    // it.
    let stated = element_read(plan, l, &field.id);
    let otherwise = match absent {
        Absent::Never | Absent::Zero => "0".to_string(),
        Absent::Stated | Absent::Group => format!("(0 if {stated} is None else {stated})"),
    };
    let value = match &present {
        None => size,
        Some(there) => format!("({size}) if {there} else {otherwise}"),
    };
    out.push_str(&format!("    _d_{name} = {value}\n"));
    // A value that states the number beside content says it twice, and must
    // say what the content does.
    if *absent == Absent::Stated {
        let there = present
            .as_deref()
            .expect("a number the value states is free: every measure may be absent");
        out.push_str(&format!(
            "    if {there} and {stated} is not None and {stated} != _d_{name}:\n        {}\n",
            disagree()
        ));
    }
    None
}
