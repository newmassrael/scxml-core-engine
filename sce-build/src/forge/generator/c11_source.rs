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
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_failure::EncodeFailure;
use crate::forge::encode_plan::{
    Absent, Bound, Clause, CodecPlan, Derive, Element, EncodePlan, Guard, Measure, Measured,
    Narrow, Number, NumberInto, Offset, Presence, Value, Write,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) mod source_case;

/// The first form of `plan` the C11 printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the C11 printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums,
/// byte strings, texts, and bodies and lists of codecs it prints that are
/// handed no flag input and held to no law, always there, decided by a gate of
/// a carrier's flags, or a number the value states while the content it
/// measures is absent; whose every derived element is a number measured off
/// byte strings, texts, lists and bodies, in a field or in a flag of a
/// carrier, or a flag set where the elements it gates are there; and that
/// takes no flag input. Its encode refuses, before its first byte, a number
/// past what holds it, measures of one number that disagree, a number the
/// value states beside content that says another, and what a body or an entry
/// it holds refuses. Its writes are the fixed bytes and the byte strings of a
/// positional layout and, of a streamed one, whole fixed fields,
/// variable-length integers, byte strings, bodies, and lists and chains that
/// end with their scope, walked entry by entry. A CBOR map and a
/// content-line component keep their own templates' encodes. `imports` are the
/// codec's own, which say whether a codec it holds is printed
/// ([`ImportContext::c11_printed`]).
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan, imports),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// The C11 prefix of the symbols of the codec a body of `plan` holds as
/// `alias`, where the C11 backend prints it from its plan: its header then
/// declares `{prefix}_encode_source`, `{prefix}_value_source` and
/// `{prefix}_source_ref_t`. The prefix is the one the template names the
/// body's encode by (`resolve_variant_arm_encoder`).
fn printed_body(imports: &[ImportContext], alias: &str) -> Option<String> {
    imports
        .iter()
        .any(|import| import.alias == alias && import.c11_printed)
        .then(|| body_prefix(alias))
}

/// The prefix [`printed_body`] answers, of the codec imported as `alias`.
fn body_prefix(alias: &str) -> String {
    filters::to_snake_case(alias.to_string())
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
            // Presented as the table and the context of its codec's source,
            // and written by that codec's own encode over it, which a codec
            // the C11 backend prints has, handed nothing.
            Value::Body { codec } => {
                if printed_body(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a body of `{codec}`, which the C11 backend does not print"
                    ));
                }
                if plan.handed.get(*codec).is_some_and(|h| !h.is_empty()) {
                    forms.push(format!("field '{id}': a body handed flag inputs"));
                }
            }
            // Presented as a walk of its entries, each a source of its codec,
            // and written entry by entry by that codec's own encode, which a
            // codec the C11 backend prints has. The value holds the entries in
            // an array beside the length member `{id}_len`, which its walk
            // `{codec}_value_{id}_begin` and `_next` read, names no element
            // of its own may take.
            Value::List { codec, law, .. } => {
                if printed_body(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a list of `{codec}`, which the C11 backend does not print"
                    ));
                }
                if plan.handed.get(*codec).is_some_and(|h| !h.is_empty()) {
                    forms.push(format!(
                        "field '{id}': a list whose entries are handed flag inputs"
                    ));
                }
                if !law.is_none() {
                    forms.push(format!("field '{id}': a list held to a law"));
                }
                let taken = [
                    format!("{id}_len"),
                    format!("{id}_begin"),
                    format!("{id}_next"),
                ];
                if plan
                    .elements
                    .iter()
                    .any(|other| taken.contains(&other.field.id))
                {
                    forms.push(format!(
                        "field '{id}': a list whose length member or walk another element names"
                    ));
                }
            }
        }
        // An element a gate decides, and a number the value states while its
        // content is absent, are presented as the optional value of what they
        // hold.
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
                // Set where the elements it gates are there, each read off
                // whether the source holds it; elements the flag gates apart
                // that disagree are refused. A group presents the fields it
                // holds together.
                if presence
                    .of
                    .iter()
                    .any(|(gated, _)| element_of(plan, &gated.id).is_none())
                {
                    forms.push(format!(
                        "element '{}': a presence flag of fields a group holds",
                        derived.element
                    ));
                }
            }
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a number \
                 or a presence flag"
            )),
        }
    }
    // The encode refuses what the derivations above refuse, and what a body
    // it holds refuses, which a count of its writes finds first; a gate it
    // cannot derive and an input a held codec says are checks of their own.
    if !plan.gate_checks.is_empty() {
        forms.push("a gate whose field the encode holds to it rather than derives".to_string());
    }
    if !plan.input_checks.is_empty() {
        forms.push("an input a held codec says that the encode hands it as well".to_string());
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
/// prints a number measured off byte strings, texts, lists and the encoded
/// lengths of bodies, in a field or in a flag of a carrier, held to what its
/// measures say together ([`derive_number`]).
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    for measure in &number.measures {
        match &measure.of {
            // The bytes the view holds, less or more its arithmetic, refused
            // where the arithmetic takes them below zero.
            Measured::Bytes { .. } => {}
            // Counted by its codec's encode into a counting sink, which
            // passes on what the body refuses.
            Measured::Encoded { .. } => {}
            // The entries the list's maker counted as it walked them.
            Measured::Entries { .. } => {}
            Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                forms.push(format!("element '{element}': a flag a held codec says"));
            }
        }
    }
    // Zero where a gate leaves the content out, as the field is then gated
    // beside it, and what the value states where it states one; a group
    // presents what the field holds on the side where its content is absent.
    if let NumberInto::Field {
        absent: Absent::Group,
        ..
    } = &number.into
    {
        forms.push(format!(
            "element '{element}': a length a group holds where its content is absent"
        ));
    }
    forms.dedup();
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
/// holds no member `id`: the encode derives it (docs/adr/0081 §3), the value
/// never states it, and the decode reads it into a local of its own. A number
/// the value states while its content is absent is a member, the optional
/// value of its type.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id) && element_of(plan, id).is_none()
}

/// The numbers a codec's C11 decode reads into a local of its own,
/// [`derived_local`], and reads a length or a count off where it would read
/// the member: the encode printed from its plan derives each (docs/adr/0081
/// §3), so the value holds no member of it, or, for a number the value states
/// while its content is absent, a member the decode sets from the local once
/// it knows whether the content is there ([`C11Source::decoded`]). Empty for a
/// codec on the template's encode, whose value holds every field, and for
/// every language whose decode binds every field to a local anyway.
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
/// holds a number in, the view of a byte string or a text that holds its bound
/// ([`BoundedView`]), the pair a body's source is passed as, or the walk of a
/// list that holds its bound ([`BoundedList`]).
///
/// An element a gate decides, or a number the value states while its content
/// is absent, is presented as the optional value of that type: a number in the
/// type `sce/forge/optional.h` holds it in, which the value's struct holds it
/// in too, and a view, a pair or a list in the optional type declared beside
/// it.
fn presented_type(l: &LangCtx, snake: &str, element: &Element<'_>) -> String {
    let gated = element.presence.optional();
    match &element.value {
        Value::Scalar(ty) if gated => l.c11_codec_optional(ty, None).type_name,
        Value::Scalar(ty) => l.type_name(ty).into_owned(),
        Value::Bytes(_) | Value::Text(_) if gated => BoundedView::of(l, snake, element).optional(),
        Value::Bytes(_) | Value::Text(_) => BoundedView::of(l, snake, element).ty(),
        Value::Body { codec } if gated => format!("{}_source_ref_optional_t", body_prefix(codec)),
        Value::Body { codec } => format!("{}_source_ref_t", body_prefix(codec)),
        Value::List { .. } if gated => BoundedList::of(l, snake, element).optional(),
        Value::List { .. } => BoundedList::of(l, snake, element).ty(),
    }
}

/// The refinement of a list or a chain (docs/adr/0065 §6 R2, R7) in C11: a
/// walk of its entries over the context it is presented with, the table each
/// entry is a source of its codec by, and the count of its entries, of a type
/// of its own for each element, made by one function that walks the list once,
/// counts it and checks the bound, and answers a status (docs/adr/0081 §2).
/// The walk is the runtime's `sce_forge_walk_t`, a begin and next pair, so an
/// array, a linked list or any store of a consumer's own presents its entries
/// where it holds them.
struct BoundedList {
    /// The prefix of its type and its maker, `{snake}_{element}_list`.
    base: String,
    /// The prefix of the symbols of the entries' codec ([`body_prefix`]).
    entry: String,
    /// How many entries the maker holds it to.
    bound: Bound,
}

impl BoundedList {
    /// The list of `element`, a list or a chain of the codec `snake`.
    fn of(l: &LangCtx, snake: &str, element: &Element<'_>) -> Self {
        let Value::List { codec, bound, .. } = &element.value else {
            unreachable!("a list's refinement is of a list or a chain");
        };
        BoundedList {
            base: format!("{snake}_{}_list", l.codec_field_id(&element.field.id)),
            entry: body_prefix(codec),
            bound: *bound,
        }
    }

    /// Its type.
    fn ty(&self) -> String {
        format!("{}_t", self.base)
    }

    /// The optional value of it, which a source presents a list a gate
    /// decides as, made as `sce/forge/optional.h` makes one.
    fn optional(&self) -> String {
        format!("{}_optional_t", self.base)
    }

    /// The declaration of [`Self::optional`].
    fn declare_optional(&self) -> String {
        format!(
            "/* A list of {} that may be absent: absent where the gate leaves the\n \
             * element out. */\n\
             SCE_OPTIONAL_DECLARE_NAMED({}_optional, {})\n\n",
            self.ty(),
            self.base,
            self.ty()
        )
    }

    /// Its type and its maker, for the element `id` of `value`.
    fn declare(&self, id: &str, value: &str) -> String {
        let (ty, base, entry) = (self.ty(), &self.base, &self.entry);
        let least = self.bound.least;
        let held = match self.bound.most {
            Some(most) => format!("{least} to {most} entries"),
            None => format!("{least} or more entries, the protocol stating no most"),
        };
        // An entry past the most is refused as it is met, so a walk that
        // never ends is refused too where the list has a most.
        let past = match self.bound.most {
            Some(most) => format!(
                "        if (!sce_forge_length_within_bound(count, {most})) {{\n            \
                 return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;\n        }}\n"
            ),
            None => String::new(),
        };
        let fewest = if least > 0 {
            format!(
                "    if (count < {least}u) {{\n        \
                 return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n"
            )
        } else {
            String::new()
        };
        format!(
            "/* The `{id}` element of {value}: a list of {held} of {entry}_t,\n \
             * walked where it is held, which outlives the list. A list is made by\n \
             * {base}_make, which walks it once, counts its entries and refuses one\n \
             * outside the bound, so every list holds it (docs/adr/0081 §2) and the\n \
             * encode over a source writes its `count` entries, refusing nothing of\n \
             * the count. */\n\
             typedef struct {{\n    const sce_forge_walk_t *walk;\n    const void *ctx;\n    \
             const {entry}_source_t *entry;\n    size_t count;\n}} {ty};\n\n\
             /* Make `*out` the list `walk` presents over `ctx`, each entry a source of\n \
             * {entry}_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list\n \
             * outside its bound, `*out` then as it was. */\n\
             static inline sce_forge_codec_status_t {base}_make({ty} *out, \
             const sce_forge_walk_t *walk, const void *ctx, const {entry}_source_t *entry) {{\n    \
             size_t count = 0;\n    \
             for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {{\n        \
             ++count;\n\
             {past}    }}\n\
             {fewest}    out->walk = walk;\n    out->ctx = ctx;\n    out->entry = entry;\n    \
             out->count = count;\n    return SCE_FORGE_CODEC_OK;\n\
             }}\n\n"
        )
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
    /// view of each byte string and text, the list of each list and chain and
    /// the walk of the value's array of it, the table of a source and the pair a
    /// parent presents a body as, the value's functions and table, the encode
    /// over any source, and the check of a value's arrays.
    pub(super) before_encode: String,
    /// The body of the value's encode, `{snake}_encode`: the check of the
    /// arrays the value and the bodies it holds keep their byte strings and
    /// texts in, which C cannot hold in a type, then the encode over the
    /// value's table.
    pub(super) encode_body: String,
    /// At the end of the streamed decode, once every field is read: each
    /// number the value states while its content is absent set from the local
    /// the decode read it into, present where none of that content was there
    /// and absent where some was, as the frame then says it of the content.
    /// Empty where the value states none.
    pub(super) decoded: String,
}

/// The C11 text of the codec `plan`, a plan [`unlowered`] answers `None` for,
/// whose symbols are named after `snake` (`{snake}_t` its value's struct).
///
/// # Panics
///
/// Where the encode it prints writes no refusal the plan's totality names, or
/// one it does not (docs/adr/0065 §2): a defect of the printer.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> C11Source {
    let printed = C11Source {
        before_encode: before_encode(plan, l, snake),
        encode_body: encode_body(snake),
        decoded: decoded(plan, l),
    };
    // Every refusal the totality analysis names is one the encode writes.
    plan.totality.assert_all_written(&plan.model.name);
    printed
}

/// [`C11Source::decoded`]. A number the value states measures content a gate
/// decides, so a codec that states one has gated fields, and its decode is
/// the streamed one (`fields_lay_out_by_byte`).
fn decoded(plan: &EncodePlan<'_>, l: &LangCtx) -> String {
    let mut out = Vec::new();
    for element in &plan.elements {
        if element.presence != Presence::Stated {
            continue;
        }
        let id = &element.field.id;
        let absent: Vec<String> = stated_contents(plan, id)
            .iter()
            .map(|content| format!("!out->{}.present", l.codec_field_id(&content.id)))
            .collect();
        let held = l.c11_codec_optional(&element.field.sce_type, None);
        let local = derived_local(id);
        out.push(format!(
            "/* `{id}` is the value's where none of the content it measures is there\n     \
             * (docs/adr/0058 §1). */\n    \
             out->{} = ({}) ? {} : {};",
            l.codec_field_id(id),
            absent.join(" && "),
            held.some(&local),
            held.none()
        ));
    }
    out.join("\n    ")
}

/// The content the number `id` the value states measures: the gated byte
/// strings, texts and bodies its derivation measures
/// (`codec_totality::stated`).
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

/// [`C11Source::before_encode`].
fn before_encode(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let value = format!("{snake}_t");
    let table = format!("{snake}_source_t");
    let mut out = String::new();
    for element in &plan.elements {
        let gated = element.presence == Presence::Gated;
        match &element.value {
            Value::Bytes(_) | Value::Text(_) => {
                let view = BoundedView::of(l, snake, element);
                out.push_str(&view.declare(&element.field.id, &value));
                if gated {
                    out.push_str(&view.declare_optional());
                }
            }
            Value::List { .. } => {
                let list = BoundedList::of(l, snake, element);
                out.push_str(&list.declare(&element.field.id, &value));
                if gated {
                    out.push_str(&list.declare_optional());
                }
            }
            _ => {}
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
    // How a parent presents a body of this codec: the table of its source and
    // the context the table is handed, two pointers, so a body is presented
    // where it is held and with no copy; and the optional value of the pair,
    // which a parent presents a body a gate decides as.
    out.push_str(&format!(
        "/* A source of {value}: the table of its elements and the context the\n \
         * table is handed, as a codec that holds one presents it to its encode. */\n\
         typedef struct {{\n    const {table} *table;\n    const void *ctx;\n}} {snake}_source_ref_t;\n\n\
         /* A source of {value} that may be absent. */\n\
         SCE_OPTIONAL_DECLARE_NAMED({snake}_source_ref_optional, {snake}_source_ref_t)\n\n"
    ));

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
            // The body the struct holds, as a source of its codec by the table
            // that presents a value of it.
            Value::Body { codec } => {
                let prefix = body_prefix(codec);
                let (held, absent) = if gated {
                    (
                        format!("self->{name}.value"),
                        format!(
                            "    if (!self->{name}.present) {{\n        \
                             return {prefix}_source_ref_optional_none();\n    }}\n"
                        ),
                    )
                } else {
                    (format!("self->{name}"), String::new())
                };
                let ret = if gated {
                    format!("{prefix}_source_ref_optional_some(body)")
                } else {
                    "body".to_string()
                };
                format!(
                    "    const {value} *self = (const {value} *)ctx;\n\
                     {absent}    {prefix}_source_ref_t body;\n    \
                     body.table = {prefix}_value_source();\n    \
                     body.ctx = &{held};\n    \
                     return {ret};\n"
                )
            }
            // The entries the struct holds in its array, walked by the value's
            // own begin and next over it, each a source of its codec by the
            // table that presents a value of it, and counted by the length
            // beside the array, within the bound the value's check holds it
            // to.
            Value::List { .. } => {
                let list = BoundedList::of(l, snake, element);
                let (list_ty, base, entry) = (list.ty(), &list.base, &list.entry);
                let (data, len) = if gated {
                    (
                        format!("self->{name}.value.data"),
                        format!("self->{name}.value.len"),
                    )
                } else {
                    (format!("self->{name}"), format!("self->{name}_len"))
                };
                out.push_str(&format!(
                    "/* The first entry of the `{name}` of the {value} at `ctx`, or NULL where\n \
                     * it holds none. */\n\
                     static inline const void *{snake}_value_{name}_begin(const void *ctx) {{\n    \
                     const {value} *self = (const {value} *)ctx;\n    \
                     return {len} > 0u ? (const void *)&{data}[0] : NULL;\n\
                     }}\n\n\
                     /* The entry after `entry` of the `{name}` of the {value} at `ctx`, or\n \
                     * NULL where `entry` is its last. */\n\
                     static inline const void *{snake}_value_{name}_next(const void *ctx, \
                     const void *entry) {{\n    \
                     const {value} *self = (const {value} *)ctx;\n    \
                     const {entry}_t *at = (const {entry}_t *)entry + 1;\n    \
                     return at < {data} + {len} ? (const void *)at : NULL;\n\
                     }}\n\n"
                ));
                let (absent, ret) = if gated {
                    (
                        format!(
                            "    if (!self->{name}.present) {{\n        \
                             return {base}_optional_none();\n    }}\n"
                        ),
                        format!("{base}_optional_some(list)"),
                    )
                } else {
                    (String::new(), "list".to_string())
                };
                format!(
                    "    static const sce_forge_walk_t walk = {{\n        \
                     {snake}_value_{name}_begin,\n        \
                     {snake}_value_{name}_next,\n    }};\n    \
                     const {value} *self = (const {value} *)ctx;\n\
                     {absent}    {list_ty} list;\n    \
                     list.walk = &walk;\n    \
                     list.ctx = ctx;\n    \
                     list.entry = {entry}_value_source();\n    \
                     list.count = {len};\n    \
                     return {ret};\n"
                )
            }
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
    out.push('\n');
    out.push_str(&check_value(plan, l, snake));
    out
}

/// The encode of `{snake}_t` over any source of it: each element read once off
/// the source into a local, what the encode derives computed from them and
/// any value the encoding cannot carry refused, then the writes in wire order.
///
/// Where a body the value holds can refuse, a refusal of the body's own is
/// met only as the body is written, so the encode writes the frame over a
/// counting sink first (docs/adr/0081 §1, the program), which finds it before
/// the first byte: `{snake}_encode_source` counts by `{snake}_write_source`,
/// the writes alone, and then writes by it.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let refuses = if plan.totality.is_total() {
        "It refuses no value: it returns SCE_FORGE_CODEC_OK, or\n \
         * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left."
    } else {
        "A value the encoding cannot carry is refused by the\n \
         * status that names why before the first byte is written (docs/adr/0065\n \
         * §2); any other returns SCE_FORGE_CODEC_OK, or\n \
         * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left."
    };
    let children_refuse = plan
        .totality
        .refusals()
        .iter()
        .any(|r| r.kind == RefusalKind::Child);
    let signature = |name: &str| {
        format!(
            "static inline sce_forge_codec_status_t {snake}_{name}(const {snake}_source_t *src, \
             const void *ctx, sce_forge_writer_t *w) {{\n"
        )
    };
    let mut out = String::new();
    if children_refuse {
        // A body's own refusal passes on through each write of it, which the
        // count before the writes finds first.
        for r in plan.totality.refusals() {
            if r.kind == RefusalKind::Child {
                plan.totality.require(RefusalKind::Child, &r.element);
            }
        }
        out.push_str(&format!(
            "/* {snake}_encode_source without the count that finds a refusal of a body\n \
             * the source holds before the first byte is written. */\n\
             {}",
            signature("write_source")
        ));
    } else {
        out.push_str(&format!(
            "/* Write the source `src` presents over `ctx` into `w`, each element read\n \
             * once and written where the codec's encode plan lays it out\n \
             * (docs/adr/0081). {refuses} */\n\
             {}",
            signature("encode_source")
        ));
    }
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
    // What the encode derives (docs/adr/0081 §3), each refusal of it before
    // the first byte: a number from what it measures ([`derive_number`]),
    // held whole in a field as the local `_d_<field>`; and the flags of a
    // carrier, each folded into it: a presence flag set exactly where the
    // elements it gates are there (or are not, for one gated on the flag's
    // clear state), refused where two of them disagree, and a number its
    // bit-range holds.
    let mut folds: BTreeMap<&str, Vec<Fold>> = BTreeMap::new();
    let mut counted = BTreeSet::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let tests: Vec<String> = presence
                    .of
                    .iter()
                    .map(|(gated, when_clear)| {
                        let there = format!("{}.present", element_local(l, &gated.id));
                        if *when_clear {
                            format!("!{there}")
                        } else {
                            there
                        }
                    })
                    .collect();
                let (first, others) = tests.split_first().expect("a presence flag gates a field");
                // Elements the flag gates apart that disagree have no frame;
                // a group's fields are there together and cannot.
                if !presence.together {
                    for other in others {
                        out.push_str(&refuse_if(
                            &format!("({first}) != ({other})"),
                            &refusal(
                                plan,
                                RefusalKind::InconsistentSource,
                                &derived.element.to_string(),
                            ),
                        ));
                    }
                }
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    named: derived.element.to_string(),
                    bits: format!("(uint64_t)({first})"),
                    count: None,
                    present: None,
                });
            }
            (element, Derive::Number(number)) => {
                let measures: Vec<MeasureText> = number
                    .measures
                    .iter()
                    .map(|measure| measure_text(plan, l, measure, &mut counted, &mut out))
                    .collect();
                if let Some((carrier, fold)) =
                    derive_number(plan, l, element, &number.into, &measures, &mut out)
                {
                    folds.entry(carrier).or_default().push(fold);
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
        for fold in flags {
            let def = carrier
                .flags
                .iter()
                .find(|d| d.name == fold.flag)
                .expect("a derived flag is a flag of its carrier");
            let width_mask = if def.width >= 64 {
                u64::MAX
            } else {
                (1u64 << def.width) - 1
            };
            let mask = width_mask << def.bit;
            let placed = if def.bit == 0 {
                fold.bits.clone()
            } else {
                format!("({} << {})", fold.bits, def.bit)
            };
            // A number past what its bits hold has no frame (docs/adr/0065
            // §5), refused where the analysis says it may be.
            let mut set = String::new();
            if let Some(count) = &fold.count {
                if plan
                    .totality
                    .refuses(RefusalKind::LengthOutOfRange, &fold.named)
                {
                    set.push_str(&refuse_if(
                        &format!("!sce_forge_length_within_bound((size_t)({count}), {width_mask})"),
                        &refusal(plan, RefusalKind::LengthOutOfRange, &fold.named),
                    ));
                }
            }
            set.push_str(&format!(
                "    {bits} = ({bits} & ~0x{mask:X}ull) | {placed};\n"
            ));
            // Where nothing it measures is there, the carrier keeps the bits
            // the value presents.
            out.push_str(&match &fold.present {
                None => set,
                Some(there) => guarded(there, &set),
            });
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
                    // A body, by its codec's encode over the source the value
                    // presents it as.
                    BitSize::Embed => {
                        let Some(Value::Body { codec }) =
                            element_of(plan, &field.id).map(|e| &e.value)
                        else {
                            unreachable!("an embedded field is a body the value presents");
                        };
                        let held = read(field);
                        body.push_str(&format!(
                            "    SCE_FORGE_TRY_WRITE({}_encode_source({held}.table, {held}.ctx, w));\n",
                            body_prefix(codec)
                        ));
                    }
                    // A list or a chain that ends with its scope, its entries
                    // walked from the first, each written by its codec's
                    // encode over the source the list presents it as; no
                    // more than the count the list's maker found, which a
                    // count before it says.
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                        let Some(Value::List { codec, .. }) =
                            element_of(plan, &field.id).map(|e| &e.value)
                        else {
                            unreachable!("a list or a chain is an element the value presents");
                        };
                        let held = read(field);
                        body.push_str(&format!(
                            "    {{\n        \
                             const void *_at = {held}.walk->begin({held}.ctx);\n        \
                             for (size_t _i = 0; _i < {held}.count && _at != NULL; ++_i) {{\n            \
                             SCE_FORGE_TRY_WRITE({}_encode_source({held}.entry, _at, w));\n            \
                             _at = {held}.walk->next({held}.ctx, _at);\n        \
                             }}\n    \
                             }}\n",
                            body_prefix(codec)
                        ));
                    }
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
    if children_refuse {
        out.push_str(&format!(
            "\n/* Write the source `src` presents over `ctx` into `w`, each element read\n \
             * once and written where the codec's encode plan lays it out\n \
             * (docs/adr/0081). {refuses} The frame is written over a\n \
             * counting sink first, which stores nothing, so a refusal of a body the\n \
             * source holds is met before the first byte. */\n\
             {}    {{\n        \
             sce_forge_writer_t _count = sce_forge_writer_init_counting();\n        \
             SCE_FORGE_TRY_WRITE({snake}_write_source(src, ctx, &_count));\n    \
             }}\n    \
             return {snake}_write_source(src, ctx, w);\n\
             }}\n",
            signature("encode_source")
        ));
    }
    out
}

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, a `size_t`, read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
    /// Whether the plan says the measure may be absent (`Measure::gated`).
    gated: bool,
}

/// A flag of a carrier the encode derives, folded into the carrier.
struct Fold {
    /// The flag.
    flag: String,
    /// The element it is, as the totality analysis names it (`carrier.flag`).
    named: String,
    /// What its bits hold, as a `uint64_t`.
    bits: String,
    /// Where it holds a number, the number as a `size_t`, which a refusal of
    /// a number past its bits reads.
    count: Option<String>,
    /// Where it holds a number of content a gate may leave out, the test that
    /// the content is there; the carrier keeps the bits the value presents
    /// where it is not.
    present: Option<String>,
}

/// The statement that returns the refusal of `kind` at `at`, which the
/// totality analysis names (`Totality::require`, docs/adr/0065 §2), as the C
/// runtime's status (`EncodeFailure::symbol`).
fn refusal(plan: &EncodePlan<'_>, kind: RefusalKind, at: &str) -> String {
    plan.totality.require(kind, at);
    let status = EncodeFailure::ALL
        .into_iter()
        .find(|failure| failure.refusal_kind() == kind)
        .and_then(|failure| failure.symbol(crate::generator::Language::C11))
        .expect("a positional encode refuses with a status the C runtime names");
    format!("return {status};")
}

/// `refused`, a statement at the indentation of an encode's own, where `test`
/// holds.
fn refuse_if(test: &str, refused: &str) -> String {
    format!("    if ({test}) {{\n        {refused}\n    }}\n")
}

/// `measure` as the encode reads it, off the local the source's element was
/// read into: the bytes a view holds, less or more its `sce:length-arith`, a
/// length less a number the bytes may be shorter than refused; the entries a
/// list's maker counted; or the bytes a
/// body encodes to, counted by its codec's encode over the source the value
/// presents it as into a counting sink, which stores nothing (docs/adr/0058
/// §3) and passes on what the body refuses. A body is counted once into
/// `_k_<body>`, which `counted` keeps, and only where it is there. The
/// statements that count or refuse go into `out`.
fn measure_text(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    measure: &Measure<'_>,
    counted: &mut BTreeSet<String>,
    out: &mut String,
) -> MeasureText {
    match &measure.of {
        Measured::Bytes { field, offset } => {
            let local = element_local(l, &field.id);
            let (len, present) = if measure.gated {
                (
                    format!("{local}.value.len"),
                    Some(format!("{local}.present")),
                )
            } else {
                (format!("{local}.len"), None)
            };
            if let Offset::Less { by, held: false } = offset {
                let short = format!("{len} < {by}u");
                let test = match &present {
                    Some(there) => format!("{there} && {short}"),
                    None => short,
                };
                out.push_str(&refuse_if(
                    &test,
                    &refusal(plan, RefusalKind::LengthOutOfRange, &field.id),
                ));
            }
            let size = match offset {
                Offset::None => len,
                Offset::Less { by, .. } => format!("{len} - {by}u"),
                Offset::More { by } => format!("{len} + {by}u"),
            };
            MeasureText {
                size,
                present,
                gated: measure.gated,
            }
        }
        Measured::Encoded { body, codec, .. } => {
            let local = element_local(l, &body.id);
            let sink = format!("_k_{}", l.codec_field_id(&body.id));
            let held = if measure.gated {
                format!("{local}.value")
            } else {
                local.clone()
            };
            let present = measure.gated.then(|| format!("{local}.present"));
            if counted.insert(body.id.clone()) {
                let count = format!(
                    "    SCE_FORGE_TRY_WRITE({}_encode_source({held}.table, {held}.ctx, &{sink}));\n",
                    body_prefix(codec)
                );
                out.push_str(&format!(
                    "    sce_forge_writer_t {sink} = sce_forge_writer_init_counting();\n"
                ));
                out.push_str(&match &present {
                    Some(there) => guarded(there, &count),
                    None => count,
                });
            }
            MeasureText {
                size: format!("sce_forge_writer_position(&{sink})"),
                present,
                gated: measure.gated,
            }
        }
        // The entries the list's maker counted, within its bound.
        Measured::Entries { list } => {
            let local = element_local(l, &list.id);
            if measure.gated {
                MeasureText {
                    size: format!("{local}.value.count"),
                    present: Some(format!("{local}.present")),
                    gated: true,
                }
            } else {
                MeasureText {
                    size: format!("{local}.count"),
                    present: None,
                    gated: false,
                }
            }
        }
        _ => unreachable!(
            "`unlowered` prints a number of byte strings, texts, lists and bodies only"
        ),
    }
}

/// The name of the locals the encode makes the derived number of `element`
/// in: its field's, or its carrier's and its flag's.
fn number_name(l: &LangCtx, element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => l.codec_field_id(id),
        ElementId::Flag { field, flag } => format!(
            "{}_{}",
            l.codec_field_id(field),
            filters::to_snake_case(flag.clone())
        ),
        ElementId::Variant => unreachable!("a number is derived into a field or a flag"),
    }
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

/// The statements, written into `out`, that make the derived number of
/// `element` from `measures`, as the Rust encode's `combine` makes it
/// (docs/adr/0081 §1): the first measure always there fixes it, and every
/// other one there must say the same, or the value is refused as
/// `INCONSISTENT_SOURCE`; where every measure may be absent, it is the first
/// one there. A field holds it as `_d_<field>`, refused as `LENGTH_OUT_OF_RANGE`
/// where it may not fit; where nothing it measures is there, the field holds
/// zero, or what the value states, which must then be what the content says
/// where there is content. A flag's number is answered as the fold of it into
/// its carrier, by the carrier's id.
fn derive_number<'e>(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    element: &'e ElementId,
    into: &NumberInto<'_>,
    measures: &[MeasureText],
    out: &mut String,
) -> Option<(&'e str, Fold)> {
    let named = element.to_string();
    let disagree = || refusal(plan, RefusalKind::InconsistentSource, &named);
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
            let always: Vec<&MeasureText> = measures.iter().filter(|m| !m.gated).collect();
            let gated: Vec<&MeasureText> = measures.iter().filter(|m| m.gated).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    out.push_str(&refuse_if(
                        &format!("(size_t)({}) != (size_t)({})", other.size, anchor.size),
                        &disagree(),
                    ));
                }
                for g in &gated {
                    out.push_str(&refuse_if(
                        &format!(
                            "{} && (size_t)({}) != (size_t)({})",
                            there(g),
                            g.size,
                            anchor.size
                        ),
                        &disagree(),
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let first = format!("_m_{}", number_name(l, element));
                let first_there = format!("{first}_there");
                out.push_str(&format!(
                    "    size_t {first} = 0;\n    bool {first_there} = false;\n"
                ));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    out.push_str(&format!(
                        "    {keyword} ({}) {{\n        {first} = (size_t)({});\n        \
                         {first_there} = true;\n",
                        there(g),
                        g.size
                    ));
                }
                out.push_str("    }\n");
                for g in gated.iter().skip(1) {
                    out.push_str(&refuse_if(
                        &format!("{} && (size_t)({}) != {first}", there(g), g.size),
                        &disagree(),
                    ));
                }
                (first, Some(first_there))
            }
        }
    };
    match (element, into) {
        (
            ElementId::Field(_),
            NumberInto::Field {
                field,
                narrow,
                absent,
            },
        ) => {
            let ty = l.type_name(&field.sce_type);
            let local = derived_local(&field.id);
            // A number past what the field holds is no frame (docs/adr/0065
            // §5), asked of the runtime as a bound is.
            if *narrow == Narrow::Checked {
                let most = largest_of(&field.sce_type)
                    .expect("a length that may not fit its field is held in an integer field");
                let past = format!("!sce_forge_length_within_bound((size_t)({size}), {most})");
                let test = match &present {
                    Some(there) => format!("{there} && {past}"),
                    None => past,
                };
                out.push_str(&refuse_if(
                    &test,
                    &refusal(plan, RefusalKind::LengthOutOfRange, &named),
                ));
            }
            // Where nothing it measures is there: zero, as the field is then
            // gated beside its content, or what the value states.
            let stated = element_local(l, &field.id);
            let otherwise = match absent {
                Absent::Never | Absent::Zero => format!("({ty})0"),
                Absent::Stated => format!("({stated}.present ? {stated}.value : ({ty})0)"),
                Absent::Group => unreachable!("`unlowered` prints no length a group holds"),
            };
            let value = match &present {
                None => format!("({ty})({size})"),
                Some(there) => format!("{there} ? ({ty})({size}) : {otherwise}"),
            };
            out.push_str(&format!("    const {ty} {local} = {value};\n"));
            // A value that states the number beside content says it twice,
            // and must say what the content does.
            if *absent == Absent::Stated {
                let there = present
                    .as_deref()
                    .expect("a number the value states is free: every measure may be absent");
                out.push_str(&refuse_if(
                    &format!("{there} && {stated}.present && {stated}.value != {local}"),
                    &disagree(),
                ));
            }
            None
        }
        (ElementId::Flag { field, flag }, NumberInto::Flag) => Some((
            field.as_str(),
            Fold {
                flag: flag.clone(),
                named,
                bits: format!("(uint64_t)({size})"),
                count: Some(size),
                present,
            },
        )),
        _ => unreachable!(
            "a number derived into a field goes into the field, and into a flag into the flag"
        ),
    }
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

/// [`C11Source::encode_body`]: the value checked by [`check_value`] before the
/// first byte, then encoded over the value's table.
fn encode_body(snake: &str) -> String {
    format!(
        "    /* The value within its bounds before the first byte (docs/adr/0081 §2). */\n    \
         SCE_FORGE_TRY_WRITE({snake}_check_value(self));\n    \
         return {snake}_encode_source({snake}_value_source(), self, w);\n"
    )
}

/// `{snake}_check_value`, which answers whether a value of the codec's struct
/// is a value of the codec. The value holds each byte string and text in an
/// array beside its length, as C holds a value with no allocator, and C cannot
/// keep that length within the bound of the element's view; a value whose
/// length is outside it, or whose text no UTF-8 holds, is no value of the
/// codec, and is refused before the first byte, as a decode would refuse its
/// frame. A list's entries are held in an array beside their count in the same
/// way, held to the most the list holds. A body or an entry it holds is
/// checked by its own codec's check, as the value's table presents the body's
/// and the entry's arrays through that codec's views.
fn check_value(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let mut out = String::new();
    for element in &plan.elements {
        // The array of a list within the most its list holds, and each entry
        // it holds a value of its codec, by that codec's own check; where a
        // gate decides the list, only where it is present.
        if let Value::List { codec, .. } = &element.value {
            let list = BoundedList::of(l, snake, element);
            let name = l.codec_field_id(&element.field.id);
            let held = u64::from(c11_held_entries(element.field, &plan.model.fields));
            let most = list.bound.most.map_or(held, |most| most.min(held));
            let (data, len) = if element.presence == Presence::Gated {
                (
                    format!("self->{name}.value.data"),
                    format!("self->{name}.value.len"),
                )
            } else {
                (format!("self->{name}"), format!("self->{name}_len"))
            };
            let mut checks = format!(
                "    if (!sce_forge_length_within_bound({len}, {most})) {{\n        \
                 return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;\n    }}\n"
            );
            if list.bound.least > 0 {
                checks.push_str(&format!(
                    "    if ({len} < {}u) {{\n        \
                     return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n",
                    list.bound.least
                ));
            }
            checks.push_str(&format!(
                "    for (size_t _i = 0; _i < {len}; ++_i) {{\n        \
                 SCE_FORGE_TRY_WRITE({}_check_value(&{data}[_i]));\n    }}\n",
                body_prefix(codec)
            ));
            out.push_str(&if element.presence == Presence::Gated {
                guarded(&format!("self->{name}.present"), &checks)
            } else {
                checks
            });
            continue;
        }
        if let Value::Body { codec } = &element.value {
            let name = l.codec_field_id(&element.field.id);
            let check = format!("{}_check_value", body_prefix(codec));
            out.push_str(&if element.presence == Presence::Gated {
                format!(
                    "    if (self->{name}.present) {{\n        \
                     SCE_FORGE_TRY_WRITE({check}(&self->{name}.value));\n    }}\n"
                )
            } else {
                format!("    SCE_FORGE_TRY_WRITE({check}(&self->{name}));\n")
            });
            continue;
        }
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
    if out.is_empty() {
        out.push_str("    (void)self;\n");
    }
    format!(
        "/* Whether `self` is a value of {snake}_t: SCE_FORGE_CODEC_OK, or the\n \
         * refusal of an array past the bound of its element's view or list or of a\n \
         * text no UTF-8 holds, in it or in a body or an entry it holds\n \
         * (docs/adr/0081 §2). */\n\
         static inline sce_forge_codec_status_t {snake}_check_value(const {snake}_t *self) {{\n\
         {out}    return SCE_FORGE_CODEC_OK;\n\
         }}\n"
    )
}
