// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A C11 value presents its elements to its encode through a table of
//! functions, `{codec}_source_t`, each handed the context pointer the table is
//! passed with: the interface of C, as an operations table is, which a type of
//! any layout joins by a table of its own, and which a build places in
//! read-only memory once for each type, so a source costs an MCU no RAM. The
//! encode is a function over any source, `{codec}_encode_source(src, ctx, w)`:
//! it reads each element once, through its function, and derives every other
//! element of the frame from them as the plan says, never from the value
//! (docs/adr/0058 §1). A value of the codec's struct is one source, by the
//! table `{codec}_value_source()` answers, whose context is the value, and its
//! `encode` is the encode over that source.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::encode_plan::{CodecPlan, Element, EncodePlan, Guard, Presence, Value, Write};

/// The first form of `plan` the C11 printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    unlowered_forms(plan).into_iter().next()
}

/// Every form of `plan` the C11 printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets and
/// enums, each always there, of which the encode derives nothing, that takes
/// no flag input and whose encode refuses nothing: its writes are the fixed
/// bytes of a positional layout and, of a streamed one, whole fixed fields and
/// variable-length integers. A CBOR map and a content-line component keep
/// their own templates' encodes.
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
            // A byte string, which the elements above name.
            Write::Tail { .. } => {}
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, field));
                if *guard != Guard::Always || list.is_some() {
                    forms.push(format!(
                        "field '{}': a streamed field written other than always",
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
                    } => {}
                    BitSize::Vle { width_bits } => forms.push(format!(
                        "field '{}': a variable-length integer of {width_bits} bits",
                        field.id
                    )),
                    // A byte string, a body or a list, which the elements
                    // above name.
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

/// Whether the printer cannot read `field` as a write reads it: the value does
/// not present it.
fn unread_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    element_of(plan, &field.id)
        .is_none()
        .then(|| format!("field '{}': a field the value does not present", field.id))
}

/// The element of `plan` the value presents as `id`, if it presents one.
fn element_of<'p, 'm>(plan: &'p EncodePlan<'m>, id: &str) -> Option<&'p Element<'m>> {
    plan.elements.iter().find(|e| e.field.id == id)
}

/// The C type the source presents `element` as: the type the value's struct
/// holds it in.
fn presented_type(l: &LangCtx, element: &Element<'_>) -> String {
    match &element.value {
        Value::Scalar(ty) => l.type_name(ty).into_owned(),
        _ => unreachable!("`unlowered` prints a scalar element only"),
    }
}

/// The name of the local the encode reads the element `id` into, once.
fn element_local(l: &LangCtx, id: &str) -> String {
    format!("_e_{}", l.codec_field_id(id))
}

/// The C11 text of the codec `plan`, a plan [`unlowered`] answers `None` for,
/// whose symbols are named after `snake` (`{snake}_t` its value's struct): the
/// table of a source, the value's functions and table, and the encode over any
/// source. It stands after the value's struct and before the value's encode,
/// which calls it.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let value = format!("{snake}_t");
    let table = format!("{snake}_source_t");
    let mut out = format!(
        "/* The elements a value of {value} presents, which its encode reads\n \
         * (docs/adr/0081 §2): one function for each, handed the context the\n \
         * table is passed with. A value of {value} is a source by the table\n \
         * {snake}_value_source() answers, its context the value; any other type\n \
         * that presents the elements is one by a table of its own. The encode\n \
         * derives every other element of the frame. */\n\
         typedef struct {{\n"
    );
    if plan.elements.is_empty() {
        out.push_str(
            "    /* A C struct declares at least one member; the codec presents no\n     \
             * element, so the table has no function. */\n    \
             char _reserved;\n",
        );
    }
    for element in &plan.elements {
        out.push_str(&format!(
            "    {} (*{})(const void *ctx);\n",
            presented_type(l, element),
            l.codec_field_id(&element.field.id)
        ));
    }
    out.push_str(&format!("}} {table};\n\n"));

    // The value's functions, each reading its element off the value the
    // context points at.
    for element in &plan.elements {
        let name = l.codec_field_id(&element.field.id);
        out.push_str(&format!(
            "/* The `{name}` of the {value} at `ctx`. */\n\
             static inline {ty} {snake}_value_{name}(const void *ctx) {{\n    \
             return ((const {value} *)ctx)->{name};\n\
             }}\n\n",
            ty = presented_type(l, element),
        ));
    }
    let entries = if plan.elements.is_empty() {
        "0".to_string()
    } else {
        plan.elements
            .iter()
            .map(|element| format!("{snake}_value_{}", l.codec_field_id(&element.field.id)))
            .collect::<Vec<_>>()
            .join(",\n        ")
    };
    out.push_str(&format!(
        "/* The table that presents a value of {value} as a source of it, its\n \
         * context a `const {value} *`. One table for every value, in read-only\n \
         * memory. */\n\
         static inline const {table} *{snake}_value_source(void) {{\n    \
         static const {table} table = {{\n        {entries},\n    }};\n    \
         return &table;\n\
         }}\n\n"
    ));
    out.push_str(&encode_source(plan, l, snake));
    out
}

/// The encode of `{snake}_t` over any source of it: each element read once off
/// the source into a local, then the writes in wire order.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let mut out = format!(
        "/* Write the source `src` presents over `ctx` into `w`, each element read\n \
         * once and written where the codec's encode plan lays it out\n \
         * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or\n \
         * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */\n\
         static inline sce_forge_codec_status_t {snake}_encode_source(const {snake}_source_t *src, \
         const void *ctx, sce_forge_writer_t *w) {{\n"
    );
    if plan.elements.is_empty() {
        out.push_str("    (void)src;\n    (void)ctx;\n");
    }
    if plan.program.is_empty() {
        out.push_str("    (void)w;\n");
    }
    for element in &plan.elements {
        out.push_str(&format!(
            "    const {} {} = src->{}(ctx);\n",
            presented_type(l, element),
            element_local(l, &element.field.id),
            l.codec_field_id(&element.field.id)
        ));
    }
    let read = |field: &CodecField| element_local(l, &field.id);
    let write =
        |byte: &str| format!("    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, {byte}));\n");
    let endian = plan.model.default_endian;
    for step in &plan.program {
        match step {
            Write::Whole(field) => {
                for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                    out.push_str(&write(&byte));
                }
            }
            Write::Byte { pieces, .. } => {
                out.push_str(&write(&encode_byte_read_by(pieces, l, &read)));
            }
            Write::Streamed { field, .. } => match &field.bit_size {
                BitSize::Fixed { .. } => {
                    for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                        out.push_str(&write(&byte));
                    }
                }
                // The runtime's writer, as the template's encode calls it.
                BitSize::Vle { width_bits } => {
                    let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                    out.push_str(&vle_encode_block(&value, *width_bits, l.lang));
                    out.push('\n');
                }
                _ => unreachable!("`unlowered` prints a fixed field or a VLE only"),
            },
            Write::Tail { .. } | Write::Variant => {
                unreachable!("`unlowered` prints no byte string and no variant")
            }
        }
    }
    out.push_str("    return SCE_FORGE_CODEC_OK;\n}\n");
    out
}
