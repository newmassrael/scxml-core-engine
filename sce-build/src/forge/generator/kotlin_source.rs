// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A Kotlin value presents its elements as the properties of an interface,
//! `{Codec}Source`, which the codec's data class implements and any other
//! class may, and its encode is a function over any source,
//! `{Codec}.encodeSource` of the data class's companion object beside its
//! `decode`: it reads each element the source presents once and derives every
//! other element of the frame from them, as the plan says, never from the
//! value (docs/adr/0058 §1). A byte string or a text is presented as a value
//! class that names its bound, the abstract type of §2 for Kotlin: a class
//! whose constructor is private and whose one factory, `of`, checks the bound,
//! over bytes whose count or a text that cannot change after, so a value of it
//! holds its bound for as long as it lives and the encode over it refuses
//! nothing the bound decides. An element a gate decides is `null` where it is
//! absent, and the flag that says so is derived from it.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::codec_roles::ElementId;
use crate::forge::encode_plan::{
    Absent, Bound, Clause, CodecPlan, Derive, Element, EncodePlan, Guard, Measured, Narrow, Number,
    NumberInto, Offset, Presence, PresenceFlag, Value, Write,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

pub(super) mod source_case;

/// The first form of `plan` the Kotlin printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the Kotlin printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums,
/// byte strings and texts, each always there or decided by a gate on a flag of
/// the codec's own; whose every derived element is either the length of one
/// byte string or text, held in a field of its own that the length fits and
/// gated beside its content where its content is, its `sce:length-arith` never
/// taking it below zero, or a flag set exactly where the one field it gates is
/// there; and whose encode refuses nothing. It holds a body of a codec it
/// prints that is handed no flag input, and derives the encoded length of one.
/// A CBOR map and a content-line component keep their own templates' encodes.
/// `imports` are the codec's own, which say whether a codec it holds is printed
/// ([`ImportContext::kotlin_printed`]).
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan, imports),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// The import of the codec a body of `plan` holds as `alias`, where the
/// Kotlin backend prints it from its plan.
fn printed_import<'i>(imports: &'i [ImportContext], alias: &str) -> Option<&'i ImportContext> {
    imports
        .iter()
        .find(|import| import.alias == alias && import.kotlin_printed)
}

/// [`unlowered_forms`] of a positional codec.
fn positional_forms(plan: &EncodePlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let m = plan.model;
    let mut forms = Vec::new();
    if m.variant.is_some() {
        forms.push("a variant".to_string());
    }
    if !m.flag_inputs.is_empty() {
        forms.push("a flag input".to_string());
    }
    for element in &plan.elements {
        let id = &element.field.id;
        match &element.value {
            Value::Scalar(t) if t.int_bit_width().is_some() => {}
            Value::Scalar(SceType::Enum(_)) => {}
            Value::Scalar(t) => forms.push(format!("field '{id}': a `{}` field", t.as_attr())),
            Value::Bytes(_) | Value::Text(_) => {}
            // A body is written by its codec's own `encodeSource`, which a
            // codec the Kotlin backend prints has, handed nothing.
            Value::Body { codec } => {
                if printed_import(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a body of `{codec}`, which the Kotlin backend does not print"
                    ));
                }
                if plan.handed.get(*codec).is_some_and(|h| !h.is_empty()) {
                    forms.push(format!("field '{id}': a body handed flag inputs"));
                }
            }
            Value::List { codec, .. } => {
                forms.push(format!("field '{id}': a list of `{codec}`"));
            }
        }
        match element.presence {
            Presence::Always => {}
            Presence::Gated => forms.extend(gate_form(plan, element.field)),
            Presence::Stated => forms.push(format!(
                "field '{id}': a length the value states where its content is absent"
            )),
        }
    }
    if !plan.groups().is_empty() {
        forms.push("a group of fields a gate decides together".to_string());
    }
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Field(_), Derive::Number(number)) => {
                forms.extend(number_form(&derived.element, number));
            }
            (ElementId::Flag { .. }, Derive::Presence(presence)) => {
                forms.extend(presence_form(&derived.element, presence));
            }
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a length \
                 or a presence flag"
            )),
        }
    }
    if !plan.gate_checks.is_empty() || !plan.input_checks.is_empty() || !plan.totality.is_total() {
        forms.push("an encode that can refuse a value".to_string());
    }
    let numbers = derived_numbers(plan);
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
                if list.is_some() {
                    forms.push(format!("field '{}': a walk of a list", field.id));
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

/// What of the derived number of `element` the printer does not print: it
/// prints the length of one byte string or text, its `sce:length-arith` never
/// taking the number below zero, into a field the number fits that holds
/// nothing else, written where the content is (zero where a gate leaves the
/// content out, beside which the field is gated too).
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    match number.measures.as_slice() {
        [one] => match &one.of {
            Measured::Bytes {
                offset: Offset::Less { held: false, .. },
                ..
            } => forms.push(format!(
                "element '{element}': a length its arithmetic may take below zero"
            )),
            Measured::Bytes { .. } => {}
            Measured::Entries { .. } => {
                forms.push(format!("element '{element}': a count of a list's entries"));
            }
            Measured::Encoded { refuses: true, .. } => forms.push(format!(
                "element '{element}': the encoded length of a body whose encode can refuse"
            )),
            Measured::Encoded { .. } => {}
            Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                forms.push(format!("element '{element}': a flag a held codec says"));
            }
        },
        _ => forms.push(format!(
            "element '{element}': a number that measures other than one element"
        )),
    }
    let gated = number.measures.iter().any(|m| m.gated);
    match &number.into {
        NumberInto::Field { narrow, absent, .. } => {
            if *narrow == Narrow::Checked {
                forms.push(format!(
                    "element '{element}': a length that may not fit its field"
                ));
            }
            if !matches!(
                (absent, gated),
                (Absent::Never, false) | (Absent::Zero, true)
            ) {
                forms.push(format!(
                    "element '{element}': a length the value states where its content is absent"
                ));
            }
        }
        NumberInto::Flag => forms.push(format!(
            "element '{element}': a length held in a flag of a carrier"
        )),
    }
    forms
}

/// What of the presence flag `element` the printer does not print: it prints
/// a flag the presence of one field decides, or of fields one group holds.
fn presence_form(element: &ElementId, presence: &PresenceFlag<'_>) -> Option<String> {
    (presence.of.len() != 1 && !presence.together)
        .then(|| format!("element '{element}': a flag the presence of several fields decides"))
}

/// What of the gate of `field`, a field a gate decides, the printer does not
/// print: it tests a flag of the codec's own carriers.
fn gate_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    let gate = field.present_if.as_ref()?;
    plan.clauses(gate)
        .iter()
        .any(|clause| !matches!(clause, Clause::Flag { .. }))
        .then(|| {
            format!(
                "field '{}': a gate on an input or on a chain's entries",
                field.id
            )
        })
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

/// Whether the printer cannot read `field` as a write reads it: the value
/// presents it alone, or the encode derives it as a number.
fn unread_form(
    plan: &EncodePlan<'_>,
    numbers: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    (element_of(plan, &field.id).is_none() && !numbers.contains(&field.id)).then(|| {
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

/// Whether the Kotlin value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no field `id`: the encode derives it (docs/adr/0081 §3).
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id)
}

/// The class the Kotlin value of the codec whose data class is `struct_name`
/// holds its element `id` of type `ty` in, where the element is a byte string
/// or a text, which the value holds within its bound (docs/adr/0065 §6 R1).
/// The one place the name is made: the printer declares the class by it and
/// the conformance harness makes a value of it by it.
pub(crate) fn refinement_class(struct_name: &str, id: &str, ty: &SceType) -> String {
    let pascal = filters::to_pascal_case(id.to_string());
    match ty {
        SceType::String => format!("{struct_name}{pascal}Text"),
        _ => format!("{struct_name}{pascal}Bytes"),
    }
}

/// How the Kotlin value holds an element that is no Kotlin type of its own:
/// the type its property is declared as, the value it holds by default, and
/// what its decode makes it of from the local it read, where it reads one.
pub(super) struct Held {
    /// The type of the property.
    pub(super) ty: String,
    /// The value it holds by default.
    pub(super) default: String,
    /// What the decode hands the constructor for it.
    pub(super) made: String,
}

/// The Kotlin text the printer gives a codec's file.
pub(super) struct KotlinSource {
    /// Before the value's class: the class of each byte string's and text's
    /// bound and the interface a source of the codec implements.
    pub(super) before: String,
    /// The encode over any source, `encodeSource`, a function of the value's
    /// companion object, at its indentation there.
    pub(super) encode: String,
    /// For each element the value holds in a class of the printer's, by field
    /// id, how it holds it.
    pub(super) held: BTreeMap<String, Held>,
    /// Whether the file declares a value class, for which it imports
    /// `JvmInline`.
    pub(super) inline: bool,
}

/// The data class of the codec a printed import names, which the wildcard
/// import of its package (`ImportContext::include_stmt`) brings into scope with
/// its source interface, `{Type}Source`.
fn body_type(import: &ImportContext) -> &str {
    &import.type_name
}

/// The Kotlin text of the codec `plan`, whose value's data class is
/// `struct_name`; the plan is one [`unlowered`] answers `None` for.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> KotlinSource {
    let mut before = String::new();
    let mut held = BTreeMap::new();
    // The type the source presents each element as.
    let mut presented: BTreeMap<String, String> = BTreeMap::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let gated = element.presence == Presence::Gated;
        let optional = |ty: String| {
            if gated {
                l.optional_of(&ty).expect("Kotlin wraps the present type")
            } else {
                ty
            }
        };
        // A body is any source of its codec, the interface its file declares;
        // the value holds its codec's data class, which implements it, so the
        // value's property overrides the interface's with a type of its own.
        if let Value::Body { codec } = &element.value {
            let import = printed_import(imports, codec)
                .expect("`unlowered` prints a body of a printed codec only");
            presented.insert(id.clone(), optional(format!("{}Source", body_type(import))));
            continue;
        }
        let (Value::Bytes(bound) | Value::Text(bound)) = &element.value else {
            presented.insert(
                id.clone(),
                optional(l.type_name(&element.field.sce_type).into_owned()),
            );
            continue;
        };
        let class = refinement_class(struct_name, id, &element.field.sce_type);
        let text = matches!(element.value, Value::Text(_));
        before.push_str(&bounded_class(struct_name, id, &class, bound, text));
        let local = l.codec_field_id(id);
        let (default, made) = if gated {
            (
                "null".to_string(),
                format!("{local}?.let {{ {class}.of(it) ?: return null }}"),
            )
        } else {
            (
                format!("{class}.fewest()"),
                format!("{class}.of({local}) ?: return null"),
            )
        };
        let ty = optional(class);
        presented.insert(id.clone(), ty.clone());
        held.insert(id.clone(), Held { ty, default, made });
    }
    let inline = !held.is_empty();

    // The source: what the encode reads, each element by the property a value
    // holds it in, `null` where a gate leaves it out.
    before.push_str(&format!(
        "/// The elements a value of [{struct_name}] presents, which its encode reads\n\
         /// (docs/adr/0081 §2): any object that presents them is a source of it, which\n\
         /// [{struct_name}.encodeSource] writes. The encode derives every other element\n\
         /// of the frame.\n\
         interface {struct_name}Source"
    ));
    if plan.elements.is_empty() {
        before.push('\n');
    } else {
        before.push_str(" {\n");
        for element in &plan.elements {
            let id = &element.field.id;
            before.push_str(&format!(
                "    val {}: {}\n",
                l.codec_field_id(id),
                presented[id]
            ));
        }
        before.push_str("}\n");
    }

    KotlinSource {
        before: before.trim_end_matches('\n').to_string(),
        encode: encode_source(plan, l, imports, struct_name),
        held,
        inline,
    }
}

/// The value class a byte string or a text (`text`) `id` of `struct_name` is
/// held in, `class`, within `bound`: a private constructor, and `of`, which
/// makes a value only of bytes, or of a text whose UTF-8 bytes, within the
/// bound. A text no UTF-8 holds (one with a lone surrogate) is no value of it.
fn bounded_class(struct_name: &str, id: &str, class: &str, bound: &Bound, text: bool) -> String {
    let least = bound.least;
    let (unit, of_what, holder, size) = if text {
        ("UTF-8 bytes", "a text", "text: String", "size")
    } else {
        ("bytes", "bytes", "bytes: ByteArray", "bytes.size")
    };
    let held = match bound.most {
        Some(most) => format!("{least} to {most} {unit}"),
        None => format!("{least} or more {unit}, the protocol stating no most"),
    };
    let most = bound.most.map_or_else(String::new, |most| {
        format!(
            "\n\
             \x20       /// The most {unit} a value holds.\n\
             \x20       const val MOST: Int = {most}\n"
        )
    });
    // Only the comparisons the bound makes: a least of zero holds every count,
    // and no most none above it.
    let test = match (least, bound.most) {
        (0, None) => None,
        (0, Some(_)) => Some(format!("{size} <= MOST")),
        (_, None) => Some(format!("{size} >= LEAST")),
        (_, Some(_)) => Some(format!("{size} in LEAST..MOST")),
    };
    let argument = if text { "text" } else { "bytes" };
    let of = match (text, test) {
        (false, Some(test)) => format!(" =\n            if ({test}) {class}(bytes) else null"),
        (false, None) => format!(" = {class}(bytes)"),
        // A text's bound is counted in the bytes UTF-8 encodes it to, as every
        // backend counts a text's length on the wire.
        (true, Some(test)) => format!(
            " {{\n\
             \x20           val size = com.sce.forge.runtime.Utf8.size(text) ?: return null\n\
             \x20           return if ({test}) {class}(text) else null\n\
             \x20       }}"
        ),
        (true, None) => format!(
            " =\n            if (com.sce.forge.runtime.Utf8.size(text) != null) {class}(text) else null"
        ),
    };
    let fewest = if text {
        format!("{class}(\"\\u0000\".repeat(LEAST))")
    } else {
        format!("{class}(ByteArray(LEAST))")
    };
    format!(
        "/// The `{id}` element of [{struct_name}], {held}, held to that bound where a\n\
         /// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is\n\
         /// private and [of] makes a value only of {of_what} within the bound. The value it\n\
         /// holds cannot change after, so a value holds its bound for as long as it lives.\n\
         @JvmInline\n\
         value class {class} private constructor(val {holder}) {{\n\
         \x20   companion object {{\n\
         \x20       /// The fewest {unit} a value holds.\n\
         \x20       const val LEAST: Int = {least}\n\
         {most}\n\
         \x20       /// `{argument}` as a value of the bound, or `null` where it lies outside it.\n\
         \x20       fun of({holder}): {class}?{of}\n\
         \n\
         \x20       /// The fewest {unit} the bound holds, each zero: what a value made by\n\
         \x20       /// default holds.\n\
         \x20       fun fewest(): {class} = {fewest}\n\
         \x20   }}\n\
         }}\n\n"
    )
}

/// `count` converted to the Kotlin type of `ty`, the type of the field a
/// derived number goes into; `count` is an `Int`.
fn int_as(count: &str, ty: &SceType) -> String {
    let conversion = match ty {
        SceType::Uint8 => "toUByte",
        SceType::Uint16 => "toUShort",
        SceType::Uint32 => "toUInt",
        SceType::Uint64 => "toULong",
        SceType::Int8 => "toByte",
        SceType::Int16 => "toShort",
        SceType::Int64 => "toLong",
        // Already an `Int`, which a conversion would only repeat.
        _ => return count.to_string(),
    };
    if count.contains(' ') {
        format!("({count}).{conversion}()")
    } else {
        format!("{count}.{conversion}()")
    }
}

/// How the bits of a carrier of type `ty` are worked on: the unsigned type of
/// its width (`UInt`, or `ULong` for 64 bits) and the conversions into it and
/// back, each `None` where the carrier is that type already. Unsigned, so a
/// mask of the top bit is a literal of the type.
struct CarrierBits {
    /// Whether the carrier is worked on as a `ULong`.
    wide: bool,
    /// The conversion into the unsigned type.
    view: Option<&'static str>,
    /// The conversion back to the carrier's type.
    back: Option<&'static str>,
}

impl CarrierBits {
    fn of(ty: &SceType) -> Self {
        let (wide, view, back) = match ty {
            SceType::Uint8 => (false, Some("toUInt"), Some("toUByte")),
            SceType::Uint16 => (false, Some("toUInt"), Some("toUShort")),
            SceType::Uint32 => (false, None, None),
            SceType::Uint64 => (true, None, None),
            SceType::Int8 => (false, Some("toUInt"), Some("toByte")),
            SceType::Int16 => (false, Some("toUInt"), Some("toShort")),
            SceType::Int64 => (true, Some("toULong"), Some("toLong")),
            _ => (false, Some("toUInt"), Some("toInt")),
        };
        CarrierBits { wide, view, back }
    }

    /// `value` as the unsigned type.
    fn viewed(&self, value: &str) -> String {
        match self.view {
            Some(view) => format!("{value}.{view}()"),
            None => value.to_string(),
        }
    }

    /// `n` as a literal of the unsigned type.
    fn literal(&self, n: u64) -> String {
        if self.wide {
            format!("0x{n:X}uL")
        } else {
            format!("0x{n:X}u")
        }
    }
}

/// `value`, the local an element of bytes or of a text (`text`) is read into,
/// as the bytes it is written as.
fn wire_bytes(value: &str, text: bool) -> String {
    if text {
        format!("{value}.text.encodeToByteArray()")
    } else {
        format!("{value}.bytes")
    }
}

/// One write of the encode at the body's indentation, its sink's failure
/// answered at once.
fn written(call: &str) -> String {
    format!("{call}?.let {{ return it }}\n")
}

/// Each line of `body` one level deeper.
fn indented(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_string()
            } else {
                format!("    {line}\n")
            }
        })
        .collect()
}

/// The encode of `struct_name` over any source of it, a function of the value's
/// companion object: each element read once off the source into a local, each
/// derived number made from what it measures and each derived flag folded into
/// its carrier, then the writes in wire order. A function of the companion
/// object and not a method of the value, so it writes any source, and a
/// property of the value cannot take its name.
fn encode_source(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let numbers = derived_numbers(plan);
    let derived: Vec<String> = numbers
        .iter()
        .map(|id| format!("`{}`", l.codec_field_id(id)))
        .collect();
    let derives = if derived.is_empty() {
        String::new()
    } else {
        format!(
            " It derives {} from what each measures, and never reads it off the\n/// source.",
            derived.join(", ")
        )
    };
    // The elements the body reads, which are read off the source once, before
    // the first write.
    let reads: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
    let element = |id: &str| -> String {
        reads.borrow_mut().insert(id.to_string());
        format!("_e_{}", l.codec_field_id(id))
    };
    let is_text = |field: &CodecField| field.sce_type == SceType::String;
    // The data class of the codec whose companion's `encodeSource` writes the
    // body `field`.
    let body_writer = |field: &CodecField| -> String {
        let Some(Value::Body { codec }) = element_of(plan, &field.id).map(|e| &e.value) else {
            unreachable!("a body is an element the value presents")
        };
        let import =
            printed_import(imports, codec).expect("`unlowered` prints a body of a printed codec");
        body_type(import).to_string()
    };
    // The bodies encoded into a buffer first, by field id: those whose length a
    // number measures.
    let mut buffered: BTreeSet<String> = BTreeSet::new();
    let mut body = String::new();
    // Each flag the presence of the fields it gates sets, by its carrier: the
    // flag's mask and the test that sets it.
    let mut folds: BTreeMap<String, Vec<(u64, String)>> = BTreeMap::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Field(id), Derive::Number(number)) => {
                let ([measure], NumberInto::Field { field: into, .. }) =
                    (number.measures.as_slice(), &number.into)
                else {
                    unreachable!("`unlowered` prints a number of one measure into a field only");
                };
                let (content, count) = match &measure.of {
                    Measured::Bytes { field, offset } => {
                        let content = element(&field.id);
                        let size = format!("{}.size", wire_bytes(&content, is_text(field)));
                        let count = match offset {
                            Offset::None => size,
                            Offset::More { by } => format!("{size} + {by}"),
                            Offset::Less { by, .. } => format!("{size} - {by}"),
                        };
                        (content, count)
                    }
                    // The body's frame, encoded once into a buffer the write
                    // of the body then writes; `null` where it is absent, so
                    // the buffer is what the number and the write test.
                    Measured::Encoded { body: held, .. } => {
                        let content = element(&held.id);
                        let name = l.codec_field_id(&held.id);
                        let (list, buffer) = (format!("_l_{name}"), format!("_b_{name}"));
                        let encode = format!(
                            "val {list} = mutableListOf<Byte>()\n{}",
                            written(&format!(
                                "{}.encodeSource({content}, MutableListSink({list}))",
                                body_writer(held)
                            ))
                        );
                        if measure.gated {
                            body.push_str(&format!(
                                "var {buffer}: ByteArray? = null\n\
                                 if ({content} != null) {{\n\
                                 {}    {buffer} = {list}.toByteArray()\n\
                                 }}\n",
                                indented(&encode)
                            ));
                        } else {
                            body.push_str(&format!(
                                "{encode}val {buffer} = {list}.toByteArray()\n"
                            ));
                        }
                        buffered.insert(held.id.clone());
                        let count = format!("{buffer}.size");
                        (buffer, count)
                    }
                    _ => unreachable!("`unlowered` prints a length or an encoded length only"),
                };
                let number = int_as(&count, &into.sce_type);
                // Zero where a gate leaves the content out, beside which the
                // length is gated too and not written.
                let number = if measure.gated {
                    format!(
                        "if ({content} != null) {number} else {}",
                        int_as("0", &into.sce_type)
                    )
                } else {
                    number
                };
                body.push_str(&format!("val _d_{} = {number}\n", l.codec_field_id(id)));
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
                let test = if *when_clear { "==" } else { "!=" };
                folds
                    .entry(field.clone())
                    .or_default()
                    .push((mask, format!("{} {test} null", element(&gated.id))));
            }
            _ => unreachable!("`unlowered` prints no other derivation"),
        }
    }
    // Each carrier as the encode writes it: its flags the value presents, and
    // those the presence of a field sets folded in, in place of the value's.
    for (carrier, carried) in &folds {
        let field = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == *carrier)
            .expect("a folded flag is of a field of the codec");
        let bits = CarrierBits::of(&field.sce_type);
        let zero = bits.literal(0);
        let mut word = bits.viewed(&element(carrier));
        for (mask, test) in carried {
            let mask = bits.literal(*mask);
            word = format!("(({word} and {mask}.inv()) or (if ({test}) {mask} else {zero}))");
        }
        let word = match bits.back {
            Some(back) => format!("{word}.{back}()"),
            None => word,
        };
        body.push_str(&format!("val _c_{} = {word}\n", l.codec_field_id(carrier)));
    }
    let read = |f: &CodecField| -> String {
        let name = l.codec_field_id(&f.id);
        if numbers.contains(&f.id) {
            format!("_d_{name}")
        } else if folds.contains_key(&f.id) {
            format!("_c_{name}")
        } else {
            element(&f.id)
        }
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
                } => {
                    let bits = CarrierBits::of(&carrier.sce_type);
                    format!(
                        "(({} shr {bit}) and {}) {} {}",
                        bits.viewed(&read(carrier)),
                        bits.literal(1),
                        if *negate { "==" } else { "!=" },
                        bits.literal(0)
                    )
                }
                _ => unreachable!("`unlowered` prints a gate on a carrier's flags only"),
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    let endian = plan.model.default_endian;
    for write in &plan.program {
        match write {
            Write::Whole(field) => {
                for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                    body.push_str(&written(&format!("w.writeU8({byte})")));
                }
            }
            Write::Byte { pieces, .. } => {
                body.push_str(&written(&format!(
                    "w.writeU8({})",
                    encode_byte_read_by(pieces, l, &read)
                )));
            }
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    body.push_str(&format!(
                        "while (w.position() < {}) {{\n    {}}}\n",
                        field.byte_offset,
                        written("w.writeU8(0)")
                    ));
                }
                body.push_str(&written(&format!(
                    "w.writeBytes({})",
                    wire_bytes(&read(field), is_text(field))
                )));
            }
            Write::Streamed { field, guard, .. } => {
                let write = match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        encode_whole_field_read_as(field, endian, &read(field), l)
                            .iter()
                            .map(|byte| written(&format!("w.writeU8({byte})")))
                            .collect::<String>()
                    }
                    // The runtime's writer, as the template's encode calls it.
                    BitSize::Vle { width_bits } => {
                        let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                        format!("{}\n", vle_encode_block(&value, *width_bits, l.lang).trim())
                    }
                    BitSize::Tail | BitSize::LengthRef => written(&format!(
                        "w.writeBytes({})",
                        wire_bytes(&read(field), is_text(field))
                    )),
                    // A body: the buffer its length was measured over, or its
                    // codec's encode over the source it is presented as.
                    BitSize::Embed if buffered.contains(&field.id) => {
                        written(&format!("w.writeBytes(_b_{})", l.codec_field_id(&field.id)))
                    }
                    BitSize::Embed => written(&format!(
                        "{}.encodeSource({}, w)",
                        body_writer(field),
                        read(field)
                    )),
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                        unreachable!("`unlowered` prints no list")
                    }
                };
                // What says whether the value holds the field: the local it
                // was read into, or the buffer a body was encoded into, which
                // is `null` where the body is absent.
                let held = if buffered.contains(&field.id) {
                    format!("_b_{}", l.codec_field_id(&field.id))
                } else {
                    read(field)
                };
                body.push_str(&match guard {
                    Guard::Always => write,
                    // Where the value holds it: the local it is tested by is
                    // not `null`, which narrows it for the write.
                    Guard::Present => {
                        format!("if ({held} != null) {{\n{}}}\n", indented(&write))
                    }
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => {
                        format!("if ({}) {{\n{}}}\n", gate_test(field), indented(&write))
                    }
                });
            }
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    body.push_str("return null\n");

    // Each element the body reads, once, in declaration order.
    let reads = reads.into_inner();
    let mut bound = String::new();
    for element in &plan.elements {
        if reads.contains(&element.field.id) {
            let name = l.codec_field_id(&element.field.id);
            bound.push_str(&format!("val _e_{name} = src.{name}\n"));
        }
    }
    // A codec that writes nothing reads no element and touches no sink.
    let suppress = if reads.is_empty() {
        "@Suppress(\"UNUSED_PARAMETER\")\n"
    } else {
        ""
    };
    let text = format!(
        "/// Write any source of [{struct_name}] into `w`: each element read once off\n\
         /// `src` and written where the codec's encode plan lays it out\n\
         /// (docs/adr/0081).{derives} It refuses no value: it answers `null`, or\n\
         /// [CodecError.BufferOverflow] from a bounded sink with no room left.\n\
         {suppress}\
         fun encodeSource(src: {struct_name}Source, w: SceSink): CodecError? {{\n\
         {}\
         }}",
        indented(&format!("{bound}{body}"))
    );
    indented(&indented(&text))
        .trim_end_matches('\n')
        .to_string()
}
