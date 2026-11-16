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
    Absent, Bound, Clause, CodecPlan, Derive, Element, EncodePlan, Guard, Measured, Narrow, Number,
    NumberInto, Offset, Presence, Value, Write,
};
use std::collections::{BTreeMap, BTreeSet};

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
/// byte strings and texts, always there or decided by a gate of a carrier's
/// flags, whose every derived element is the length of one byte string or text
/// in a field of its own that the length fits (zero where a gate leaves the
/// bytes out) or in a flag of a carrier, its `sce:length-arith` never taking it
/// below zero, or a flag set where the one element it gates is there, that
/// takes no flag input and whose encode refuses nothing: its writes are the fixed bytes and
/// the byte strings of a positional layout and, of a streamed one, whole fixed
/// fields, variable-length integers and byte strings. A CBOR map and a
/// content-line component keep their own templates' encodes.
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
            // Presented as the optional value of what it holds.
            Presence::Always | Presence::Gated => {}
            Presence::Stated => forms.push(format!(
                "field '{id}': a length the value states where its content is absent"
            )),
        }
    }
    if !plan.groups().is_empty() {
        forms.push("a group of fields a gate decides together".to_string());
    }
    if !plan.overridable.is_empty() {
        forms.push("a flag a parent sets in place of the value's".to_string());
    }
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Field(_) | ElementId::Flag { .. }, Derive::Number(number)) => {
                forms.extend(number_form(&derived.element, number));
                if let ElementId::Flag { field, .. } = &derived.element {
                    forms.extend(carrier_form(plan, &derived.element, field));
                }
            }
            (ElementId::Flag { field, .. }, Derive::Presence(presence)) => {
                forms.extend(carrier_form(plan, &derived.element, field));
                // Set exactly where the element it gates is there: fields the
                // flag gates apart may disagree, which the encode would refuse.
                let said = match presence.of.as_slice() {
                    [(gated, _)] => element_of(plan, &gated.id),
                    [(gated, _), ..] if presence.together => element_of(plan, &gated.id),
                    _ => None,
                };
                if said.is_none() {
                    forms.push(format!(
                        "element '{}': a presence flag no one element it gates says",
                        derived.element
                    ));
                }
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
            Write::Whole(field) => forms.extend(unread_form(plan, &numbers, field)),
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, &numbers, field));
                }
            }
            Write::Tail { field, .. } => forms.extend(unread_form(plan, &numbers, field)),
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &numbers, field));
                if list.is_some() {
                    forms.push(format!(
                        "field '{}': a list walked as it is written",
                        field.id
                    ));
                }
                // Written where its gate holds over the carriers the encode
                // writes: a gate of a flag input or of a chain's membership
                // reads what this printer does not hold.
                if *guard == Guard::Gate {
                    let gate = field
                        .present_if
                        .as_ref()
                        .expect("a field written where its gate holds is gated");
                    if plan
                        .clauses(gate)
                        .iter()
                        .any(|clause| !matches!(clause, Clause::Flag { .. }))
                    {
                        forms.push(format!(
                            "field '{}': a gate of other than a carrier's flags",
                            field.id
                        ));
                    }
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
                    // Written whole from the view the source presents.
                    BitSize::Tail | BitSize::LengthRef => {}
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
            // Where a gate leaves the content out, a carrier's flag would keep
            // the bits the value presents, which this printer does not fold.
            Measured::Bytes { .. } if one.gated && matches!(number.into, NumberInto::Flag) => {
                forms.push(format!(
                    "element '{element}': a length of content a gate decides, in a flag"
                ));
            }
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
            // Zero where a gate leaves the content out, as the field is then
            // gated beside it.
            match absent {
                Absent::Never | Absent::Zero => {}
                Absent::Group => forms.push(format!(
                    "element '{element}': a length a group holds where its content is absent"
                )),
                Absent::Stated => forms.push(format!(
                    "element '{element}': a length the value states where its content is absent"
                )),
            }
        }
        // Folded into its carrier, as a presence flag is.
        NumberInto::Flag => {}
    }
    forms
}

/// What of the carrier `carrier` of the derived flag `element` the printer
/// does not fold the flag into: it folds a flag into a number the value always
/// presents, whose other bits are the value's.
fn carrier_form(plan: &EncodePlan<'_>, element: &ElementId, carrier: &str) -> Option<String> {
    let folds = element_of(plan, carrier).is_some_and(|e| {
        e.presence == Presence::Always
            && matches!(&e.value, Value::Scalar(t) if t.int_bit_width().is_some())
    });
    (!folds).then(|| {
        format!("element '{element}': a flag of a carrier the value does not always present")
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

/// Whether the C11 value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no member `id`: the encode derives it (docs/adr/0081 §3), and the
/// decode reads it into a local of its own.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id)
}

/// The numbers a codec's C11 value holds no member of, as its decode reads
/// them: the encode printed from its plan derives each (docs/adr/0081 §3), so
/// the decode reads each into a local of its own, [`derived_local`], and reads
/// a length or a count off that local where it would read the member. Empty
/// for a codec on the template's encode, whose value holds every field, and
/// for every language whose decode binds every field to a local anyway.
#[derive(Default)]
pub(crate) struct Derived(BTreeSet<String>);

impl Derived {
    /// The numbers of `plan`, a plan [`unlowered`] answers `None` for.
    pub(super) fn of(plan: &EncodePlan<'_>) -> Self {
        Derived(derived_numbers(plan))
    }

    /// Whether the value holds no member `id`.
    pub(crate) fn contains(&self, id: &str) -> bool {
        self.0.contains(id)
    }
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
///
/// An element a gate decides is presented as the optional value of that type:
/// a number in the type `sce/forge/optional.h` holds it in, which the value's
/// struct holds it in too, and a view in the optional type declared beside it.
fn presented_type(l: &LangCtx, snake: &str, element: &Element<'_>) -> String {
    let gated = element.presence == Presence::Gated;
    match &element.value {
        Value::Scalar(ty) if gated => l.c11_codec_optional(ty, None).type_name,
        Value::Scalar(ty) => l.type_name(ty).into_owned(),
        Value::Bytes(_) | Value::Text(_) if gated => BoundedView::of(l, snake, element).optional(),
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

    /// The optional value of it, which a source presents an element a gate
    /// decides as: `{ present, value }`, made by `{base}_optional_some(view)`
    /// and `{base}_optional_none()`, as `sce/forge/optional.h` makes one.
    fn optional(&self) -> String {
        format!("{}_optional_t", self.base)
    }

    /// The declaration of [`Self::optional`].
    fn declare_optional(&self) -> String {
        format!(
            "/* A view of {} that may be absent: absent where the gate leaves the\n \
             * element out. */\n\
             SCE_OPTIONAL_DECLARE_NAMED({}_optional, {})\n\n",
            self.ty(),
            self.base,
            self.ty()
        )
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
        let mut checks = length_refusal("len", least, self.bound.most);
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

/// The statement that refuses the length `len` outside `least` and `most`
/// bytes, or nothing where every length is within them. The most is asked of
/// the runtime's `sce_forge_length_within_bound` and not compared where it is
/// written: on a target whose `size_t` is narrower than the bound, that
/// comparison is one the type decides, a warning under -Wtype-limits.
fn length_refusal(len: &str, least: u64, most: Option<u64>) -> String {
    match length_outside(len, least, most) {
        Some(outside) => format!(
            "    if ({outside}) {{\n        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n"
        ),
        None => String::new(),
    }
}

/// The condition [`length_refusal`] refuses on, or `None` where it refuses no
/// length.
fn length_outside(len: &str, least: u64, most: Option<u64>) -> Option<String> {
    let mut outside = Vec::new();
    if least > 0 {
        outside.push(format!("{len} < {least}u"));
    }
    if let Some(most) = most {
        outside.push(format!("!sce_forge_length_within_bound({len}, {most})"));
    }
    (!outside.is_empty()).then(|| outside.join(" || "))
}

/// The name of the local the encode reads the element `id` into, once.
fn element_local(l: &LangCtx, id: &str) -> String {
    format!("_e_{}", l.codec_field_id(id))
}

/// The name of the local the encode derives the number `id` into, and the
/// decode reads it into: the value holds no member of it (docs/adr/0081 §3).
/// The member it stands for would be named in snake case, as every C11 member
/// of a codec is (`LangCtx::codec_field_id`).
pub(crate) fn derived_local(id: &str) -> String {
    format!("_d_{}", filters::to_snake_case(id.to_string()))
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
            let view = BoundedView::of(l, snake, element);
            out.push_str(&view.declare(&element.field.id, &value));
            if element.presence == Presence::Gated {
                out.push_str(&view.declare_optional());
            }
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
    // within the bound the value's encode holds the array to. One a gate
    // decides is read off the optional value the struct holds it in: a number
    // as it is, and a view of the bytes and their length where it is present.
    for element in &plan.elements {
        let name = l.codec_field_id(&element.field.id);
        let ty = presented_type(l, snake, element);
        let gated = element.presence == Presence::Gated;
        let body = match &element.value {
            Value::Bytes(_) | Value::Text(_) if gated => {
                let view = BoundedView::of(l, snake, element);
                let (view_ty, base) = (view.ty(), &view.base);
                // The struct holds a text's UTF-8 as bytes when a gate decides it.
                let data = if view.text {
                    format!("(const char *)self->{name}.value.data")
                } else {
                    format!("self->{name}.value.data")
                };
                format!(
                    "    const {value} *self = (const {value} *)ctx;\n    \
                     if (!self->{name}.present) {{\n        \
                     return {base}_optional_none();\n    }}\n    \
                     {view_ty} view;\n    \
                     view.data = {data};\n    \
                     view.len = self->{name}.value.len;\n    \
                     return {base}_optional_some(view);\n"
                )
            }
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
    // What the encode derives (docs/adr/0081 §3): each length from the bytes
    // the view it measures holds, which its type holds within the bound the
    // field fits, less or more its arithmetic, and zero where a gate leaves
    // the bytes out, as the field is then gated beside them; and the flags of
    // a carrier, each folded into it: a presence flag set exactly where the
    // element it gates is there (or is not, for one gated on the flag's clear
    // state), and a length its bit-range holds.
    let mut folds: BTreeMap<&str, Vec<(&str, String)>> = BTreeMap::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let there = format!("{}.present", element_local(l, &gated.id));
                let test = if *when_clear {
                    format!("!{there}")
                } else {
                    there
                };
                folds
                    .entry(field.as_str())
                    .or_default()
                    .push((flag.as_str(), format!("(uint64_t)({test})")));
            }
            (element, Derive::Number(number)) => {
                let [measure] = number.measures.as_slice() else {
                    unreachable!("`unlowered` prints a number of one measure only");
                };
                let Measured::Bytes { field, offset } = &measure.of else {
                    unreachable!("`unlowered` prints the length of a byte string or a text only");
                };
                let local = element_local(l, &field.id);
                let len = if measure.gated {
                    format!("{local}.value.len")
                } else {
                    format!("{local}.len")
                };
                let count = match offset {
                    Offset::None => len,
                    Offset::Less { by, .. } => format!("{len} - {by}u"),
                    Offset::More { by } => format!("{len} + {by}u"),
                };
                match (element, &number.into) {
                    (ElementId::Field(id), NumberInto::Field { field: into, .. }) => {
                        let ty = l.type_name(&into.sce_type);
                        let value = if measure.gated {
                            format!("{local}.present ? ({ty})({count}) : ({ty})0")
                        } else {
                            format!("({ty})({count})")
                        };
                        out.push_str(&format!(
                            "    const {ty} {} = {value};\n",
                            derived_local(id)
                        ));
                    }
                    (ElementId::Flag { field, flag }, NumberInto::Flag) => folds
                        .entry(field.as_str())
                        .or_default()
                        .push((flag.as_str(), format!("(uint64_t)({count})"))),
                    _ => unreachable!("a field holds a number whole, and a flag a bit-range"),
                }
            }
            _ => unreachable!("`unlowered` prints a derived number or a presence flag only"),
        }
    }
    // Each carrier as the source presents it, each derived bit-range replaced
    // in turn, the bits it holds no wider than the grammar bounds them.
    for (carrier_id, flags) in &folds {
        let carrier = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == *carrier_id)
            .expect("a derived flag is a flag of a carrier of the codec");
        let ty = l.type_name(&carrier.sce_type);
        let name = l.codec_field_id(&carrier.id);
        let bits = format!("_f_{name}");
        out.push_str(&format!(
            "    uint64_t {bits} = (uint64_t){};\n",
            element_local(l, &carrier.id)
        ));
        for (flag, value) in flags {
            let def = carrier
                .flags
                .iter()
                .find(|d| d.name == *flag)
                .expect("a derived flag is a flag of its carrier");
            let width_mask = if def.width >= 64 {
                u64::MAX
            } else {
                (1u64 << def.width) - 1
            };
            let mask = width_mask << def.bit;
            let placed = if def.bit == 0 {
                value.clone()
            } else {
                format!("({value} << {})", def.bit)
            };
            out.push_str(&format!(
                "    {bits} = ({bits} & ~0x{mask:X}ull) | {placed};\n"
            ));
        }
        out.push_str(&format!("    const {ty} _c_{name} = ({ty}){bits};\n"));
    }
    // The value a field is written as: the number the encode derived, the
    // carrier with its derived flags, the present value of an element a gate
    // decides (read only where it is tested present), or what the source
    // presents.
    let numbers = derived_numbers(plan);
    let read = |field: &CodecField| {
        if numbers.contains(&field.id) {
            derived_local(&field.id)
        } else if folds.contains_key(field.id.as_str()) {
            format!("_c_{}", l.codec_field_id(&field.id))
        } else if plan.presence(field) == Presence::Gated {
            format!("{}.value", element_local(l, &field.id))
        } else {
            element_local(l, &field.id)
        }
    };
    // Whether the gate of `field` holds over the carriers the encode writes:
    // its clauses, each a bit of a carrier, joined by "or".
    let gate_test = |field: &CodecField| {
        let gate = field
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
                    "(((uint64_t){} & 0x{:X}ull) {} 0)",
                    read(carrier),
                    1u64 << *bit,
                    if *negate { "==" } else { "!=" }
                ),
                _ => unreachable!("`unlowered` prints a gate of a carrier's flags only"),
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    let write =
        |byte: &str| format!("    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, {byte}));\n");
    // A byte string or a text, whole, from the view the source presents: a
    // text's view points at `char`, which the writer takes as its bytes.
    let view_bytes = |field: &CodecField| {
        let local = read(field);
        let data = if field.sce_type == SceType::String {
            format!("(const uint8_t *){local}.data")
        } else {
            format!("{local}.data")
        };
        format!("    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, {data}, {local}.len));\n")
    };
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
            Write::Streamed { field, guard, .. } => {
                let mut body = String::new();
                match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                            body.push_str(&write(&byte));
                        }
                    }
                    // The runtime's writer, as the template's encode calls it.
                    BitSize::Vle { width_bits } => {
                        let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                        body.push_str(&vle_encode_block(&value, *width_bits, l.lang));
                        body.push('\n');
                    }
                    // Its length, where the frame carries one, a field of its
                    // own the program writes before it.
                    BitSize::Tail | BitSize::LengthRef => body.push_str(&view_bytes(field)),
                    _ => unreachable!(
                        "`unlowered` prints a fixed field, a VLE or a byte string only"
                    ),
                }
                out.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => {
                        guarded(&format!("{}.present", element_local(l, &field.id)), &body)
                    }
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => guarded(&gate_test(field), &body),
                });
            }
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
                out.push_str(&view_bytes(field));
            }
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    out.push_str("    return SCE_FORGE_CODEC_OK;\n}\n");
    out
}

/// `body`, statements at the indentation of an encode's own, written where
/// `test` holds.
fn guarded(test: &str, body: &str) -> String {
    let mut out = format!("    if ({test}) {{\n");
    for line in body.lines() {
        if !line.is_empty() {
            out.push_str("    ");
            out.push_str(line);
        }
        out.push('\n');
    }
    out.push_str("    }\n");
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
        // One a gate decides is held in the optional value of its bytes, and
        // held to the bound where it is present.
        let (data, len, present) = if element.presence == Presence::Gated {
            (
                format!("self->{name}.value.data"),
                format!("self->{name}.value.len"),
                Some(format!("self->{name}.present")),
            )
        } else {
            (
                format!("(const uint8_t *)self->{name}"),
                format!("self->{name}_len"),
                None,
            )
        };
        let mut refusals = Vec::new();
        if let Some(outside) = length_outside(&len, view.bound.least, Some(most)) {
            refusals.push((outside, "SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE"));
        }
        if view.text {
            refusals.push((
                format!("!sce_forge_is_valid_utf8({data}, {len})"),
                "SCE_FORGE_CODEC_INVALID_UTF8",
            ));
        }
        for (refused, status) in refusals {
            let refused = match &present {
                Some(present) => format!("{present} && ({refused})"),
                None => refused,
            };
            out.push_str(&format!(
                "    if ({refused}) {{\n        return {status};\n    }}\n"
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
