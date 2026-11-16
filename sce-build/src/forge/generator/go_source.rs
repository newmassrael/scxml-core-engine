// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A Go value presents its elements through methods, the methods of an
//! interface `{Codec}Source` that names what a source of the codec is, and
//! holds them in unexported fields, so a value outside its package is made by
//! one constructor, `Make{Codec}`, by its decode, or as its zero value, and is
//! read through its methods. Its encode is a function of the codec's package,
//! `EncodeSource`, over any source: it reads each element the source presents
//! and derives every other element of the frame from them, as the plan says,
//! never from the value (docs/adr/0058 §1).
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::encode_plan::{CodecPlan, Element, EncodePlan, Guard, Presence, Value, Write};

/// The first form of `plan` the Go printer does not print, as the clause that
/// names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    unlowered_forms(plan).into_iter().next()
}

/// Every form of `plan` the Go printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets and
/// enums, each always there, written whole, in bytes it shares with other
/// fields, or as a variable-length integer of 16, 32 or 64 bits; which derives
/// no element and whose encode refuses nothing. A CBOR map and a content-line
/// component keep their own templates' encodes.
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
            Value::Bytes(_) => forms.push(format!("field '{id}': a byte string")),
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
        forms.push(format!(
            "element '{}': an element the encode derives",
            derived.element
        ));
    }
    if !plan.gate_checks.is_empty() || !plan.input_checks.is_empty() || !plan.totality.is_total() {
        forms.push("an encode that can refuse a value".to_string());
    }
    for write in &plan.program {
        match write {
            Write::Whole(field) => forms.extend(unread_form(plan, field)),
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, field));
                }
            }
            Write::Tail { field, .. } => {
                forms.extend(unread_form(plan, field));
                forms.push(format!(
                    "field '{}': a field written to the end of its frame",
                    field.id
                ));
            }
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, field));
                if *guard != Guard::Always {
                    forms.push(format!(
                        "field '{}': a field written where it is there",
                        field.id
                    ));
                }
                // A list or a chain, which the elements above name.
                if list.is_some() {
                    continue;
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
                    } => {}
                    BitSize::Vle { width_bits } => forms.push(format!(
                        "field '{}': a variable-length integer of {width_bits} bits",
                        field.id
                    )),
                    // A byte string, a text, a body or a list, which the
                    // elements above name.
                    BitSize::Tail
                    | BitSize::LengthRef
                    | BitSize::Embed
                    | BitSize::Repeat { .. }
                    | BitSize::TlvChain { .. } => {}
                }
            }
            // The variant above names it.
            Write::Variant => {}
        }
    }
    forms.dedup();
    forms
}

/// Whether the printer cannot read `field` as a write reads it: the value
/// does not present it alone.
fn unread_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    element_of(plan, &field.id).is_none().then(|| {
        format!(
            "field '{}': a field the value does not present alone",
            field.id
        )
    })
}

/// The element of `plan` the value presents as `id`, if it presents one.
fn element_of<'p, 'm>(plan: &'p EncodePlan<'m>, id: &str) -> Option<&'p Element<'m>> {
    plan.elements.iter().find(|e| e.field.id == id)
}

/// The unexported field a Go value printed from its plan holds the element
/// `id` in: the name of its method, the element's Go field name, with its
/// first letter lowered. Lowering the first letter of distinct exported names
/// keeps them distinct, and a Go keyword is refused in the document, so the
/// name needs no escape.
pub(super) fn member(id: &str) -> String {
    let exported = filters::to_pascal_case(id.to_string());
    let mut chars = exported.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => exported,
    }
}

/// The Go text the printer gives a codec's file.
pub(super) struct GoSource {
    /// Before the value's struct: the interface a source of the codec
    /// satisfies.
    pub(super) before: String,
    /// After the value's struct: its constructor, the method that presents
    /// each element, and the encode over any source, `EncodeSource`.
    pub(super) after: String,
}

/// The Go text of the codec `plan`, whose value's struct is `struct_name`; the
/// plan is one [`unlowered`] answers `None` for.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> GoSource {
    // Each element: its method, its field, and the Go type it is presented as.
    let elements: Vec<(String, String, String)> = plan
        .elements
        .iter()
        .map(|element| {
            (
                l.codec_field_id(&element.field.id),
                member(&element.field.id),
                l.type_name(&element.field.sce_type).into_owned(),
            )
        })
        .collect();

    let mut before = format!(
        "// {struct_name}Source is what the encode of {struct_name} reads: one method\n\
         // for each element a value presents (docs/adr/0081 §2). {struct_name} is one,\n\
         // and so is any value with these methods. The encode derives every other\n\
         // element of the frame.\n\
         type {struct_name}Source interface {{\n"
    );
    for (method, _, ty) in &elements {
        before.push_str(&format!("\t{method}() {ty}\n"));
    }
    before.push('}');

    let parameters: Vec<String> = elements
        .iter()
        .map(|(_, field, ty)| format!("{field} {ty}"))
        .collect();
    let fields: Vec<String> = elements
        .iter()
        .map(|(_, field, _)| format!("{field}: {field}"))
        .collect();
    let mut after = format!(
        "// Make{struct_name} returns the {struct_name} that presents each element it\n\
         // is handed, in declaration order (docs/adr/0081 §2). Its fields are\n\
         // unexported: outside this package a value is made here, by\n\
         // Decode{struct_name}, or as its zero value, and is read through its methods.\n\
         func Make{struct_name}({}) {struct_name} {{\n\
         \treturn {struct_name}{{{}}}\n\
         }}\n",
        parameters.join(", "),
        fields.join(", ")
    );
    for (method, field, ty) in &elements {
        after.push_str(&format!(
            "\n// {method} returns the element the value presents as {method}.\n\
             func (s *{struct_name}) {method}() {ty} {{\n\
             \treturn s.{field}\n\
             }}\n"
        ));
    }
    after.push('\n');
    after.push_str(&encode_source(plan, l, struct_name));

    GoSource { before, after }
}

/// One write of `bytes`, the byte expressions of a write, into the sink.
fn write_bytes(bytes: &[String]) -> String {
    format!(
        "\tif err := w.WriteBytes([]byte{{{}}}); err != nil {{\n\t\treturn err\n\t}}\n",
        bytes.join(", ")
    )
}

/// The encode of `struct_name` over any source of it, a function of the
/// codec's package, which is the codec's namespace as a C++ codec's namespace
/// is: the writes in wire order, each element read off the source by its
/// method. A function of the package and not a method of the value, so a
/// source need not be a value of the codec.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> String {
    let mut out = format!(
        "// EncodeSource writes any source of {struct_name} into w, each element read\n\
         // off src and written where the codec's encode plan lays it out\n\
         // (docs/adr/0081). It refuses no value: it returns only the error of w,\n\
         // codec.ErrBufferOverflow from a bounded sink that has no room left.\n\
         func EncodeSource(src {struct_name}Source, w codec.SceSink) error {{\n"
    );
    let read = |f: &CodecField| -> String { format!("src.{}()", l.codec_field_id(&f.id)) };
    let endian = plan.model.default_endian;
    for write in &plan.program {
        match write {
            Write::Whole(field) => {
                out.push_str(&write_bytes(&encode_whole_field_read_as(
                    field,
                    endian,
                    &read(field),
                    l,
                )));
            }
            Write::Byte { pieces, .. } => {
                out.push_str(&write_bytes(&[encode_byte_read_by(pieces, l, &read)]));
            }
            Write::Streamed { field, .. } => match &field.bit_size {
                BitSize::Fixed { .. } => {
                    out.push_str(&write_bytes(&encode_whole_field_read_as(
                        field,
                        endian,
                        &read(field),
                        l,
                    )));
                }
                // The runtime's writer, as the template's encode calls it.
                BitSize::Vle { width_bits } => {
                    let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                    out.push_str(&vle_encode_block(&value, *width_bits, l.lang));
                    out.push('\n');
                }
                _ => unreachable!("`unlowered` prints a fixed field or a VLE only"),
            },
            Write::Tail { .. } => unreachable!("`unlowered` prints no field written to its end"),
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    out.push_str("\treturn nil\n}");
    out
}
