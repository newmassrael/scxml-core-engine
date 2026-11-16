// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A Python value presents its elements as attributes, the members of a
//! `typing.Protocol` that names what a source of the codec is, and its encode
//! is a function over any source, `encode_source` of the codec's module: it
//! reads each element the source presents and derives every other element of
//! the frame from them, as the plan says, never from the value (docs/adr/0058
//! §1). A byte
//! string is presented as a subclass of the runtime's `BoundedBytes` that names
//! its bound, the abstract type of §2 for Python: a class whose constructor
//! checks, of immutable bytes, so a value of it holds its bound for as long as
//! it lives and the encode over it refuses nothing the bound decides.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::codec_roles::ElementId;
use crate::forge::encode_plan::{
    Absent, CodecPlan, Derive, EncodePlan, Guard, Measured, Narrow, Number, NumberInto, Offset,
    Presence, Value, Write,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) mod source_case;

/// The first form of `plan` the Python printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    unlowered_forms(plan).into_iter().next()
}

/// Every form of `plan` the Python printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums
/// and byte strings, each always there, whose every derived element is the
/// length of one byte string always there, held in a field of its own that the
/// length fits, and whose encode refuses nothing. A CBOR map and a
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
            Value::Bytes(_) => {}
            Value::Text(_) => forms.push(format!("field '{id}': a text")),
            Value::Body { .. } => forms.push(format!("field '{id}': an embedded body")),
            Value::List { .. } => forms.push(format!("field '{id}': a list of entries")),
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
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &numbers, field));
                if *guard != Guard::Always || list.is_some() {
                    forms.push(format!(
                        "field '{}': a write a gate or a walk decides",
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
/// never taking the number below zero, into a field the number fits, which
/// holds nothing else.
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    match number.measures.as_slice() {
        [one] => {
            if one.gated {
                forms.push(format!(
                    "element '{element}': a length of content a gate decides"
                ));
            }
            match &one.of {
                Measured::Bytes {
                    offset: Offset::Less { held: false, .. },
                    ..
                } => forms.push(format!(
                    "element '{element}': a length its arithmetic may take below zero"
                )),
                Measured::Bytes { .. } => {}
                Measured::Entries { .. } => {
                    forms.push(format!("element '{element}': a count of entries"));
                }
                Measured::Encoded { .. } => {
                    forms.push(format!("element '{element}': the encoded length of a body"));
                }
                Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                    forms.push(format!("element '{element}': a flag a held codec says"));
                }
            }
        }
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

/// Whether the printer cannot read `field` as a write reads it: the value
/// presents it alone, or the encode derives it as a number.
fn unread_form(
    plan: &EncodePlan<'_>,
    numbers: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    let presented = plan.elements.iter().any(|e| e.field.id == field.id);
    (!presented && !numbers.contains(&field.id)).then(|| {
        format!(
            "field '{}': a field the value neither presents nor the encode derives as a number",
            field.id
        )
    })
}

/// Whether the Python value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no field `id`: the encode derives it (docs/adr/0081 §3).
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id)
}

/// The class the element `id` of the codec `struct_name` is presented as where
/// it is a byte string: a subclass of the runtime's `BoundedBytes` that names
/// its bound.
pub(super) fn bytes_class(struct_name: &str, id: &str) -> String {
    format!(
        "{struct_name}{}Bytes",
        filters::to_pascal_case(id.to_string())
    )
}

/// The Python text the printer gives a codec's module.
pub(super) struct PythonSource {
    /// Before the value's class: the class of each byte string's bound, the
    /// `Protocol` a source of the codec satisfies, and the encode over any
    /// source, `encode_source`.
    pub(super) before: String,
    /// The value's `__setattr__`, which holds each byte string to its bound
    /// wherever it is set, where the value is made and after; empty where it
    /// presents none.
    pub(super) hold: String,
    /// For each byte string the value presents, by field id, its class and the
    /// default the value's field takes.
    pub(super) bytes: BTreeMap<String, (String, String)>,
    /// Whether the module names the runtime's `BoundedBytes`.
    pub(super) bounded: bool,
}

/// The Python text of the codec `plan`, whose value's class is `struct_name`;
/// the plan is one [`unlowered`] answers `None` for.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> PythonSource {
    let mut before = String::new();
    let mut hold = String::new();
    let mut bytes = BTreeMap::new();
    for element in &plan.elements {
        let Value::Bytes(bound) = &element.value else {
            continue;
        };
        let id = &element.field.id;
        let class = bytes_class(struct_name, id);
        let most = bound
            .most
            .map_or_else(|| "None".to_string(), |most| most.to_string());
        let held = match bound.most {
            Some(most) => format!("{} to {most} of them", bound.least),
            None => format!(
                "{} or more of them, the protocol stating no most",
                bound.least
            ),
        };
        before.push_str(&format!(
            "class {class}(BoundedBytes):\n\
             \x20   \"\"\"The bytes of the ``{id}`` element of :class:`{struct_name}`, {held},\n\
             \x20   held to that bound where a value of it is made (docs/adr/0077 §3).\"\"\"\n\
             \n\
             \x20   LEAST = {least}\n\
             \x20   MOST = {most}\n\n\n",
            least = bound.least
        ));
        let keyword = if hold.is_empty() { "if" } else { "elif" };
        hold.push_str(&format!(
            "        {keyword} name == \"{}\":\n            value = {class}(value)\n",
            l.codec_field_id(id)
        ));
        // The fewest bytes the class holds, zeros, spelled as a literal: the
        // default is evaluated in the class body, where a field named `bytes`
        // declared before it would hide the builtin.
        let default = if bound.least == 0 {
            format!("{class}()")
        } else {
            format!("{class}(b\"\\x00\" * {})", bound.least)
        };
        bytes.insert(id.clone(), (class, default));
    }
    let bounded = !bytes.is_empty();

    // The source: what the encode reads, each element by the attribute a value
    // holds it in.
    before.push_str(&format!(
        "class {struct_name}Source(Protocol):\n\
         \x20   \"\"\"The elements a value of :class:`{struct_name}` presents, which its encode\n\
         \x20   reads (docs/adr/0081 §2): any object that presents them is a source of it.\n\
         \x20   The encode derives every other element of the frame.\"\"\"\n"
    ));
    for element in &plan.elements {
        let id = &element.field.id;
        let ty = match bytes.get(id) {
            Some((class, _)) => class.clone(),
            None => l.type_name(&element.field.sce_type).into_owned(),
        };
        before.push_str(&format!(
            "\n    @property\n    def {}(self) -> {ty}: ...\n",
            l.codec_field_id(id)
        ));
    }

    // A dataclass's constructor sets each field as an attribute, so one
    // `__setattr__` holds a byte string to its bound where the value is made
    // and wherever it is set after.
    let hold = if hold.is_empty() {
        hold
    } else {
        format!(
            "    def __setattr__(self, name: str, value: object) -> None:\n\
             \x20       # Each byte string is held to its bound wherever it is set, where\n\
             \x20       # the value is made and after (docs/adr/0081 §2), whatever bytes\n\
             \x20       # it was handed.\n\
             {hold}\
             \x20       object.__setattr__(self, name, value)\n"
        )
    };

    PythonSource {
        before: format!(
            "{}\n\n\n{}",
            before.trim_end_matches('\n'),
            encode_source(plan, l, struct_name)
        ),
        hold: hold.trim_end_matches('\n').to_string(),
        bytes,
        bounded,
    }
}

/// The encode of `struct_name` over any source of it, a function of the
/// module, which is the codec's namespace as a C++ codec's namespace is: each
/// derived number made from what it measures, then the writes in wire order,
/// each element read off the source. A function of the module and not a
/// method of the value's class, so no field of the value, a class attribute,
/// can take its name.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> String {
    let numbers = derived_numbers(plan);
    let derived: Vec<String> = numbers.iter().map(|id| format!("``{id}``")).collect();
    let derives = if derived.is_empty() {
        String::new()
    } else {
        format!(
            " It derives {} from what each measures, and never reads it off\n    the source.",
            derived.join(", ")
        )
    };
    let mut out = format!(
        "def encode_source(src: {struct_name}Source, w: SceSink) -> None:\n\
         \x20   \"\"\"Write any source of :class:`{struct_name}` into ``w``, each element read\n\
         \x20   off ``src`` and written where the codec's encode plan lays it out\n\
         \x20   (docs/adr/0081).{derives} It refuses no value; a bounded sink raises\n\
         \x20   :class:`BufferOverflow` where it has no room left.\"\"\"\n"
    );
    // Each derived number, from the one byte string it measures.
    for derived in &plan.derivations {
        let (ElementId::Field(id), Derive::Number(number)) = (&derived.element, &derived.by) else {
            unreachable!("`unlowered` prints no derivation but a number into a field");
        };
        let [measure] = number.measures.as_slice() else {
            unreachable!("`unlowered` prints a number of one measure only");
        };
        let Measured::Bytes { field, offset } = &measure.of else {
            unreachable!("`unlowered` prints a length of a byte string only");
        };
        let arith = match offset {
            Offset::None => String::new(),
            Offset::More { by } => format!(" + {by}"),
            Offset::Less { by, .. } => format!(" - {by}"),
        };
        out.push_str(&format!(
            "    _d_{} = len(src.{}){arith}\n",
            l.codec_field_id(id),
            l.codec_field_id(&field.id)
        ));
    }
    let read = |f: &CodecField| -> String {
        let name = l.codec_field_id(&f.id);
        if numbers.contains(&f.id) {
            format!("_d_{name}")
        } else {
            format!("src.{name}")
        }
    };
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
                out.push_str(&format!("    w.write_bytes({})\n", read(field)));
            }
            Write::Streamed { field, .. } => match &field.bit_size {
                BitSize::Fixed { .. } => {
                    for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                        out.push_str(&format!("    w.write_u8({byte})\n"));
                    }
                }
                // The runtime's writer, as the template's encode calls it,
                // at the body's indentation of a function of the module.
                BitSize::Vle { width_bits } => {
                    let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                    out.push_str(&format!(
                        "    {}\n",
                        vle_encode_block(&value, *width_bits, l.lang).trim()
                    ));
                }
                BitSize::Tail | BitSize::LengthRef => {
                    out.push_str(&format!("    w.write_bytes({})\n", read(field)));
                }
                BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                    unreachable!("`unlowered` prints no body and no list")
                }
            },
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    out.trim_end_matches('\n').to_string()
}
