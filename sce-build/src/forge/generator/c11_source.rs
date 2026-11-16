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
use crate::forge::codec_roles::ElementId;
use crate::forge::encode_plan::{
    Absent, Bound, CodecPlan, Derive, Element, EncodePlan, Guard, Measured, Narrow, Number,
    NumberInto, Offset, Presence, Value, Write,
};
use std::collections::BTreeSet;

pub(super) mod source_case;

/// The first form of `plan` the C11 printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    unlowered_forms(plan).into_iter().next()
}

/// Every form of `plan` the C11 printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums,
/// byte strings and texts, each always there, whose every derived element is
/// the length of one byte string or text in a field of its own that the length
/// fits, its `sce:length-arith` never taking it below zero, that takes no flag
/// input and whose encode refuses nothing: its writes are the fixed bytes and
/// the byte strings of a positional layout and, of a streamed layout of
/// numbers alone, whole fixed fields and variable-length integers. A CBOR map
/// and a content-line component keep their own templates' encodes.
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
            // Held in the struct's array beside the length member the C11
            // struct names `{id}_len`, which no element of its own may name.
            Value::Bytes(_) | Value::Text(_) => {
                let member = format!("{id}_len");
                if plan.elements.iter().any(|other| other.field.id == member) {
                    forms.push(format!(
                        "field '{id}': a byte string whose length member another element names"
                    ));
                }
            }
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
    let streamed = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Streamed { .. }));
    // A streamed decode reads each field through its own statement, which
    // reads a length off the struct it decodes into; a length the encode
    // derives is no member of it.
    if streamed && !numbers.is_empty() {
        forms.push("a length a streamed layout carries".to_string());
    }
    for write in &plan.program {
        match write {
            Write::Whole(field) => forms.extend(unread_form(plan, &numbers, field)),
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, &numbers, field));
                }
            }
            Write::Tail { field, .. } => forms.extend(unread_form(plan, &numbers, field)),
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &numbers, field));
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
                    BitSize::Tail | BitSize::LengthRef => forms.push(format!(
                        "field '{}': a byte string of a streamed layout",
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
/// prints the length of one byte string or text always there, its
/// `sce:length-arith` never taking the number below zero, into a field of its
/// own that the number fits.
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
            Measured::Bytes { .. } if one.gated => forms.push(format!(
                "element '{element}': a length of content a gate decides"
            )),
            Measured::Bytes { .. } => {}
            Measured::Entries { .. } => {
                forms.push(format!("element '{element}': a count of a list"));
            }
            Measured::Encoded { .. } => {
                forms.push(format!("element '{element}': the encoded length of a body"))
            }
            Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                forms.push(format!("element '{element}': a flag a held codec says"));
            }
        },
        _ => forms.push(format!(
            "element '{element}': a number that measures other than one element"
        )),
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

/// Whether the C11 value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no member `id`: the encode derives it (docs/adr/0081 §3), and the
/// decode reads it into a local of its own.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id)
}

/// Whether the printer cannot read `field` as a write reads it: the value does
/// not present it, and the encode does not derive it as a number.
fn unread_form(
    plan: &EncodePlan<'_>,
    numbers: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    (element_of(plan, &field.id).is_none() && !numbers.contains(&field.id)).then(|| {
        format!(
            "field '{}': a field the value neither presents nor the encode derives",
            field.id
        )
    })
}

/// The element of `plan` the value presents as `id`, if it presents one.
fn element_of<'p, 'm>(plan: &'p EncodePlan<'m>, id: &str) -> Option<&'p Element<'m>> {
    plan.elements.iter().find(|e| e.field.id == id)
}

/// The C type the source presents `element` as: the type the value's struct
/// holds a number in, or the view of a byte string or a text that holds its
/// bound ([`BoundedView`]).
fn presented_type(l: &LangCtx, snake: &str, element: &Element<'_>) -> String {
    match &element.value {
        Value::Scalar(ty) => l.type_name(ty).into_owned(),
        Value::Bytes(_) | Value::Text(_) => BoundedView::of(l, snake, element).ty(),
        Value::Body { .. } | Value::List { .. } => {
            unreachable!("`unlowered` prints no body and no list")
        }
    }
}

/// The refinement of a byte string or a text (docs/adr/0065 §6 R1) in C11: a
/// view, `{ data, len }`, of a type of its own for each element, made by one
/// function that checks the bound and answers a status (docs/adr/0081 §2).
struct BoundedView {
    /// The prefix of its type and its maker, `{snake}_{element}_bytes` or
    /// `{snake}_{element}_text`.
    base: String,
    /// Whether it is a text, held to UTF-8 as well as to its bound.
    text: bool,
    /// The bound the maker holds it to, in bytes.
    bound: Bound,
}

impl BoundedView {
    /// The view of `element`, a byte string or a text of the codec `snake`.
    fn of(l: &LangCtx, snake: &str, element: &Element<'_>) -> Self {
        let name = l.codec_field_id(&element.field.id);
        match &element.value {
            Value::Bytes(bound) => BoundedView {
                base: format!("{snake}_{name}_bytes"),
                text: false,
                bound: *bound,
            },
            Value::Text(bound) => BoundedView {
                base: format!("{snake}_{name}_text"),
                text: true,
                bound: *bound,
            },
            _ => unreachable!("a view is of a byte string or a text"),
        }
    }

    /// Its type.
    fn ty(&self) -> String {
        format!("{}_t", self.base)
    }

    /// The type its data points at: a text's is `char`, as the struct holds it.
    fn data(&self) -> &'static str {
        if self.text {
            "char"
        } else {
            "uint8_t"
        }
    }

    /// Its type and its maker, for the element `id` of `value`.
    fn declare(&self, id: &str, value: &str) -> String {
        let (ty, data, base) = (self.ty(), self.data(), &self.base);
        let (what, unit) = if self.text {
            ("text", "bytes of UTF-8")
        } else {
            ("byte string", "bytes")
        };
        let least = self.bound.least;
        let held = match self.bound.most {
            Some(most) => format!("{least} to {most} {unit}"),
            None => format!("{least} or more {unit}, the protocol stating no most"),
        };
        let mut outside = Vec::new();
        if least > 0 {
            outside.push(format!("len < {least}u"));
        }
        if let Some(most) = self.bound.most {
            outside.push(format!("len > {most}u"));
        }
        let mut checks = String::new();
        if !outside.is_empty() {
            checks.push_str(&format!(
                "    if ({}) {{\n        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n",
                outside.join(" || ")
            ));
        }
        if self.text {
            checks.push_str(
                "    if (!sce_forge_is_valid_utf8((const uint8_t *)data, len)) {\n        \
                 return SCE_FORGE_CODEC_INVALID_UTF8;\n    }\n",
            );
        }
        format!(
            "/* The `{id}` element of {value}: a {what} of {held}, viewed where it\n \
             * is held, which outlives the view. A view is made by {base}_make, which\n \
             * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)\n \
             * and the encode over a source writes it whole, refusing nothing of it. */\n\
             typedef struct {{\n    const {data} *data;\n    size_t len;\n}} {ty};\n\n\
             /* Make `*out` a view of the `len` {unit} at `data`: SCE_FORGE_CODEC_OK, or\n \
             * the refusal of a {what} outside its bound, `*out` then as it was. */\n\
             static inline sce_forge_codec_status_t {base}_make({ty} *out, const {data} *data, \
             size_t len) {{\n\
             {checks}    out->data = data;\n    out->len = len;\n    return SCE_FORGE_CODEC_OK;\n\
             }}\n\n"
        )
    }
}

/// The name of the local the encode reads the element `id` into, once.
fn element_local(l: &LangCtx, id: &str) -> String {
    format!("_e_{}", l.codec_field_id(id))
}

/// The name of the local the encode derives the number `id` into, and the
/// decode reads it into: the value holds no member of it (docs/adr/0081 §3).
pub(super) fn derived_local(l: &LangCtx, id: &str) -> String {
    format!("_d_{}", l.codec_field_id(id))
}

/// The C11 text the printer gives a codec's header.
pub(super) struct C11Source {
    /// After the value's struct and its decode, before the value's encode: the
    /// view of each byte string and text, the table of a source, the value's
    /// functions and table, and the encode over any source.
    pub(super) before_encode: String,
    /// The body of the value's encode, `{snake}_encode`: the bounds of the
    /// arrays the value holds its byte strings and texts in, which C cannot
    /// hold in a type, then the encode over the value's table.
    pub(super) encode_body: String,
}

/// The C11 text of the codec `plan`, a plan [`unlowered`] answers `None` for,
/// whose symbols are named after `snake` (`{snake}_t` its value's struct).
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> C11Source {
    C11Source {
        before_encode: before_encode(plan, l, snake),
        encode_body: encode_body(plan, l, snake),
    }
}

/// [`C11Source::before_encode`].
fn before_encode(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let value = format!("{snake}_t");
    let table = format!("{snake}_source_t");
    let mut out = String::new();
    for element in &plan.elements {
        if matches!(element.value, Value::Bytes(_) | Value::Text(_)) {
            out.push_str(&BoundedView::of(l, snake, element).declare(&element.field.id, &value));
        }
    }
    out.push_str(&format!(
        "/* The elements a value of {value} presents, which its encode reads\n \
         * (docs/adr/0081 §2): one function for each, handed the context the\n \
         * table is passed with. A value of {value} is a source by the table\n \
         * {snake}_value_source() answers, its context the value; any other type\n \
         * that presents the elements is one by a table of its own. The encode\n \
         * derives every other element of the frame. */\n\
         typedef struct {{\n"
    ));
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
            presented_type(l, snake, element),
            l.codec_field_id(&element.field.id)
        ));
    }
    out.push_str(&format!("}} {table};\n\n"));

    // The value's functions, each reading its element off the value the
    // context points at: a number as the struct holds it, a byte string or a
    // text as a view of the array that holds it and the length beside it,
    // within the bound the value's encode holds the array to.
    for element in &plan.elements {
        let name = l.codec_field_id(&element.field.id);
        let ty = presented_type(l, snake, element);
        let body = match &element.value {
            Value::Bytes(_) | Value::Text(_) => format!(
                "    const {value} *self = (const {value} *)ctx;\n    \
                 {ty} view;\n    \
                 view.data = self->{name};\n    \
                 view.len = self->{name}_len;\n    \
                 return view;\n"
            ),
            _ => format!("    return ((const {value} *)ctx)->{name};\n"),
        };
        out.push_str(&format!(
            "/* The `{name}` of the {value} at `ctx`. */\n\
             static inline {ty} {snake}_value_{name}(const void *ctx) {{\n\
             {body}\
             }}\n\n"
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
            presented_type(l, snake, element),
            element_local(l, &element.field.id),
            l.codec_field_id(&element.field.id)
        ));
    }
    // Each length from the bytes the view it measures holds, which its type
    // holds within the bound the field fits, less or more its arithmetic.
    for derived in &plan.derivations {
        let (ElementId::Field(id), Derive::Number(number)) = (&derived.element, &derived.by) else {
            unreachable!("`unlowered` prints a derived number only");
        };
        let ([measure], NumberInto::Field { field: into, .. }) =
            (number.measures.as_slice(), &number.into)
        else {
            unreachable!("`unlowered` prints a number of one measure into a field only");
        };
        let Measured::Bytes { field, offset } = &measure.of else {
            unreachable!("`unlowered` prints the length of a byte string or a text only");
        };
        let len = format!("{}.len", element_local(l, &field.id));
        let count = match offset {
            Offset::None => len,
            Offset::Less { by, .. } => format!("{len} - {by}u"),
            Offset::More { by } => format!("{len} + {by}u"),
        };
        let ty = l.type_name(&into.sce_type);
        out.push_str(&format!(
            "    const {ty} {} = ({ty})({count});\n",
            derived_local(l, id)
        ));
    }
    let numbers = derived_numbers(plan);
    let read = |field: &CodecField| {
        if numbers.contains(&field.id) {
            derived_local(l, &field.id)
        } else {
            element_local(l, &field.id)
        }
    };
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
            // A byte string or a text of a positional layout, whole, after
            // zeros up to its offset where the layout aligns it.
            Write::Tail { field, pad } => {
                if *pad {
                    out.push_str(&format!(
                        "    while (sce_forge_writer_position(w) < {}) {{\n        \
                         SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, 0));\n    }}\n",
                        field.byte_offset
                    ));
                }
                let local = read(field);
                let data = if field.sce_type == SceType::String {
                    format!("(const uint8_t *){local}.data")
                } else {
                    format!("{local}.data")
                };
                out.push_str(&format!(
                    "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, {data}, {local}.len));\n"
                ));
            }
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    out.push_str("    return SCE_FORGE_CODEC_OK;\n}\n");
    out
}

/// [`C11Source::encode_body`]. The value holds each byte string and text in an
/// array beside its length, as C holds a value with no allocator, and C
/// cannot keep that length within the bound of the element's view; a value
/// whose length is outside it, or whose text no UTF-8 holds, is no value of
/// the codec, and is refused before the first byte, as a decode would refuse
/// its frame. Every other value is encoded over the value's table.
fn encode_body(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let mut out = String::new();
    for element in &plan.elements {
        if !matches!(element.value, Value::Bytes(_) | Value::Text(_)) {
            continue;
        }
        let view = BoundedView::of(l, snake, element);
        let name = l.codec_field_id(&element.field.id);
        // The array's own bound, where it is below the element's.
        let held = u64::from(c11_held_bytes(element.field, &plan.model.fields));
        let most = view.bound.most.map_or(held, |most| most.min(held));
        let mut outside = Vec::new();
        if view.bound.least > 0 {
            outside.push(format!("self->{name}_len < {}u", view.bound.least));
        }
        outside.push(format!(
            "!sce_forge_length_within_bound(self->{name}_len, {most})"
        ));
        out.push_str(&format!(
            "    if ({}) {{\n        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n",
            outside.join(" || ")
        ));
        if view.text {
            out.push_str(&format!(
                "    if (!sce_forge_is_valid_utf8((const uint8_t *)self->{name}, self->{name}_len)) {{\n        \
                 return SCE_FORGE_CODEC_INVALID_UTF8;\n    }}\n"
            ));
        }
    }
    if !out.is_empty() {
        out.insert_str(
            0,
            "    /* Each array within the bound of the element's view, and each text\n     \
             * UTF-8, before the first byte (docs/adr/0081 §2). */\n",
        );
    }
    out.push_str(&format!(
        "    return {snake}_encode_source({snake}_value_source(), self, w);\n"
    ));
    out
}
