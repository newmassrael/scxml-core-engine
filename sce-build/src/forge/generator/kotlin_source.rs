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
//! value (docs/adr/0058 §1). A byte string is presented as a value class that
//! names its bound, the abstract type of §2 for Kotlin: a class whose
//! constructor is private and whose one factory, `of`, checks the bound, over
//! bytes whose count cannot change after, so a value of it holds its bound for
//! as long as it lives and the encode over it refuses nothing the bound
//! decides.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::codec_roles::ElementId;
use crate::forge::encode_plan::{
    Absent, CodecPlan, Derive, Element, EncodePlan, Guard, Measured, Narrow, Number, NumberInto,
    Offset, Presence, Value, Write,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

pub(super) mod source_case;

/// The first form of `plan` the Kotlin printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    unlowered_forms(plan).into_iter().next()
}

/// Every form of `plan` the Kotlin printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums
/// and byte strings, each always there; whose every derived element is the
/// length of one byte string, held in a field of its own that the length
/// fits, its `sce:length-arith` never taking it below zero; and whose encode
/// refuses nothing. A CBOR map and a content-line component keep their own
/// templates' encodes.
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// [`unlowered_forms`] of a positional codec.
fn positional_forms(plan: &EncodePlan<'_>) -> Vec<String> {
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
            Value::Bytes(_) => {}
            Value::Text(_) => forms.push(format!("field '{id}': a text")),
            Value::Body { codec } => forms.push(format!("field '{id}': a body of `{codec}`")),
            Value::List { codec, .. } => {
                forms.push(format!("field '{id}': a list of `{codec}`"));
            }
        }
        match element.presence {
            Presence::Always => {}
            Presence::Gated => forms.push(format!("field '{id}': an element a gate decides")),
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
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a length"
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
            Write::Streamed { field, guard, .. } => {
                forms.extend(unread_form(plan, &numbers, field));
                if *guard != Guard::Always {
                    forms.push(format!(
                        "field '{}': a field written where a gate holds",
                        field.id
                    ));
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
/// prints the length of one byte string always there, its `sce:length-arith`
/// never taking the number below zero, into a field the number fits that
/// holds nothing else.
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
            Measured::Encoded { .. } => {
                forms.push(format!("element '{element}': the encoded length of a body"));
            }
            Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                forms.push(format!("element '{element}': a flag a held codec says"));
            }
        },
        _ => forms.push(format!(
            "element '{element}': a number that measures other than one element"
        )),
    }
    if number.measures.iter().any(|m| m.gated) {
        forms.push(format!(
            "element '{element}': a length of content a gate decides"
        ));
    }
    match &number.into {
        NumberInto::Field { narrow, absent, .. } => {
            if *narrow == Narrow::Checked {
                forms.push(format!(
                    "element '{element}': a length that may not fit its field"
                ));
            }
            if *absent != Absent::Never {
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
    /// Before the value's class: the class of each byte string's bound and the
    /// interface a source of the codec implements.
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

/// The Kotlin text of the codec `plan`, whose value's data class is
/// `struct_name`; the plan is one [`unlowered`] answers `None` for.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> KotlinSource {
    let mut before = String::new();
    let mut held = BTreeMap::new();
    // The type the source presents each element as.
    let mut presented: BTreeMap<String, String> = BTreeMap::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let Value::Bytes(bound) = &element.value else {
            presented.insert(
                id.clone(),
                l.type_name(&element.field.sce_type).into_owned(),
            );
            continue;
        };
        let class = refinement_class(struct_name, id, &element.field.sce_type);
        before.push_str(&bounded_bytes_class(struct_name, id, &class, bound));
        presented.insert(id.clone(), class.clone());
        let default = format!("{class}.fewest()");
        let made = format!("{class}.of({}) ?: return null", l.codec_field_id(id));
        held.insert(
            id.clone(),
            Held {
                ty: class,
                default,
                made,
            },
        );
    }
    let inline = !held.is_empty();

    // The source: what the encode reads, each element by the property a value
    // holds it in.
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
        encode: encode_source(plan, l, struct_name),
        held,
        inline,
    }
}

/// The value class a byte string `id` of `struct_name` is held in, `class`,
/// within `bound`: a private constructor, and `of`, which makes a value only
/// of bytes within the bound.
fn bounded_bytes_class(
    struct_name: &str,
    id: &str,
    class: &str,
    bound: &crate::forge::encode_plan::Bound,
) -> String {
    let least = bound.least;
    let held = match bound.most {
        Some(most) => format!("{least} to {most} bytes"),
        None => format!("{least} or more bytes, the protocol stating no most"),
    };
    let most = bound.most.map_or_else(String::new, |most| {
        format!(
            "\n\
             \x20       /// The most bytes a value holds.\n\
             \x20       const val MOST: Int = {most}\n"
        )
    });
    // Only the comparisons the bound makes: a least of zero holds every count
    // of bytes, and no most none above it.
    let test = match (least, bound.most) {
        (0, None) => None,
        (0, Some(_)) => Some("bytes.size <= MOST"),
        (_, None) => Some("bytes.size >= LEAST"),
        (_, Some(_)) => Some("bytes.size in LEAST..MOST"),
    };
    let of = match test {
        Some(test) => format!("if ({test}) {class}(bytes) else null"),
        None => format!("{class}(bytes)"),
    };
    format!(
        "/// The `{id}` element of [{struct_name}], {held}, held to that bound where a\n\
         /// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is\n\
         /// private and [of] makes a value only of bytes within the bound. The count of\n\
         /// the bytes cannot change after, so a value holds its bound for as long as it\n\
         /// lives.\n\
         @JvmInline\n\
         value class {class} private constructor(val bytes: ByteArray) {{\n\
         \x20   companion object {{\n\
         \x20       /// The fewest bytes a value holds.\n\
         \x20       const val LEAST: Int = {least}\n\
         {most}\n\
         \x20       /// `bytes` as a value of the bound, or `null` where they lie outside it.\n\
         \x20       fun of(bytes: ByteArray): {class}? = {of}\n\
         \n\
         \x20       /// The fewest bytes the bound holds, each zero: what a value made by\n\
         \x20       /// default holds.\n\
         \x20       fun fewest(): {class} = {class}(ByteArray(LEAST))\n\
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

/// `value`, the local an element `field` of bytes is read into, as the bytes it
/// is written as.
fn wire_bytes(value: &str) -> String {
    format!("{value}.bytes")
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
/// derived number made from what it measures, then the writes in wire order.
/// A function of the companion object and not a method of the value, so it
/// writes any source, and a property of the value cannot take its name.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> String {
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
    let mut body = String::new();
    for derived in &plan.derivations {
        let (ElementId::Field(id), Derive::Number(number)) = (&derived.element, &derived.by) else {
            unreachable!("`unlowered` prints a derived length only");
        };
        let ([measure], NumberInto::Field { field: into, .. }) =
            (number.measures.as_slice(), &number.into)
        else {
            unreachable!("`unlowered` prints a number of one measure into a field only");
        };
        let Measured::Bytes { field, offset } = &measure.of else {
            unreachable!("`unlowered` prints the length of a byte string only");
        };
        let size = format!("{}.size", wire_bytes(&element(&field.id)));
        let count = match offset {
            Offset::None => size,
            Offset::More { by } => format!("{size} + {by}"),
            Offset::Less { by, .. } => format!("{size} - {by}"),
        };
        body.push_str(&format!(
            "val _d_{} = {}\n",
            l.codec_field_id(id),
            int_as(&count, &into.sce_type)
        ));
    }
    let read = |f: &CodecField| -> String {
        if numbers.contains(&f.id) {
            format!("_d_{}", l.codec_field_id(&f.id))
        } else {
            element(&f.id)
        }
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
                    wire_bytes(&read(field))
                )));
            }
            Write::Streamed { field, .. } => match &field.bit_size {
                BitSize::Fixed { .. } => {
                    for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                        body.push_str(&written(&format!("w.writeU8({byte})")));
                    }
                }
                // The runtime's writer, as the template's encode calls it.
                BitSize::Vle { width_bits } => {
                    let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                    body.push_str(&format!(
                        "{}\n",
                        vle_encode_block(&value, *width_bits, l.lang).trim()
                    ));
                }
                BitSize::Tail | BitSize::LengthRef => {
                    body.push_str(&written(&format!(
                        "w.writeBytes({})",
                        wire_bytes(&read(field))
                    )));
                }
                BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                    unreachable!("`unlowered` prints no body or list")
                }
            },
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
