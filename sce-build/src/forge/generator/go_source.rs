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
//! A byte string or a text is presented as the runtime's `codec.Bytes` or
//! `codec.Text` of a bound type the codec names for the element,
//! `{Codec}{Element}Bound`: the abstract type of §2 for Go, a struct with an
//! unexported field made only by `codec.MakeBytes` or `codec.MakeText`, which
//! check the bound where the value is made, so the encode over it refuses
//! nothing the bound decides.
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
use std::collections::{BTreeMap, BTreeSet};

/// The first form of `plan` the Go printer does not print, as the clause that
/// names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    unlowered_forms(plan).into_iter().next()
}

/// Every form of `plan` the Go printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets,
/// enums, byte strings and texts, each always there, the integers written
/// whole, in bytes they share with other fields, or as a variable-length
/// integer of 16, 32 or 64 bits, and the byte strings and texts written to the
/// end of the frame or by a length; whose every derived element is the length
/// of one byte string or text, held in a field of its own that the length
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
            // Written whole, to the end of the frame or by its length.
            Value::Bytes(_) | Value::Text(_) => {
                if !matches!(element.field.bit_size, BitSize::Tail | BitSize::LengthRef) {
                    forms.push(format!(
                        "field '{id}': a byte string or a text of a fixed size"
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
/// prints the length of one byte string or text always there, its
/// `sce:length-arith` never taking the number below zero, into a field the
/// number fits that holds nothing else.
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    match number.measures.as_slice() {
        [one] => {
            match &one.of {
                Measured::Bytes {
                    offset: Offset::Less { held: false, .. },
                    ..
                } => forms.push(format!(
                    "element '{element}': a length its arithmetic may take below zero"
                )),
                Measured::Bytes { .. } => {}
                Measured::Entries { .. } => {
                    forms.push(format!("element '{element}': a count of a list"));
                }
                Measured::Encoded { .. } => {
                    forms.push(format!("element '{element}': the encoded length of a body"));
                }
                Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                    forms.push(format!("element '{element}': a flag a held codec says"));
                }
            }
            if one.gated {
                forms.push(format!(
                    "element '{element}': a length of content a gate decides"
                ));
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

/// Whether the Go value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no field `id`: the encode derives it (docs/adr/0081 §3).
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id)
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

/// What a read of an element a printed Go codec presents as `call` (a call of
/// its method) is as a Go value of the element's type `ty`: a byte string's
/// bytes and a text's string, which the runtime's `codec.Bytes` and
/// `codec.Text` hold, and any other element as the method returns it. A reader
/// of a printed codec's element in an expression, a procedure, a validator or
/// an algorithm reads it through this, so it is spelled in one place.
pub(crate) fn element_read(call: &str, ty: &SceType) -> String {
    match ty {
        SceType::Bytes => format!("{call}.Bytes()"),
        SceType::String => format!("{call}.String()"),
        _ => call.to_string(),
    }
}

/// What a reader in another package knows of a codec the Go backend prints:
/// the elements its value presents, in the order `Make{Codec}` takes them, and
/// the elements its encode derives, each from what it measures. A procedure
/// and a validator that hold a value of the codec read and make it through
/// this ([`GoPrinted::read`], [`GoPrinted::rebuilt`]), so neither spells a
/// printed codec's interface of its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GoPrinted {
    /// Each element the value presents, in declaration order.
    presented: Vec<GoPresented>,
    /// Each element the encode derives as a number.
    derived: Vec<GoDerived>,
}

/// One element a printed Go value presents.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GoPresented {
    /// The field id.
    id: String,
    /// For a byte string or a text, the runtime constructor that makes its
    /// bounded value (`MakeBytes` or `MakeText`) and the bound type the
    /// codec's package names for it; `None` for any other element.
    bounded: Option<(&'static str, String)>,
}

/// One element the encode of a printed Go codec derives, which the value
/// holds no field for: the length of the byte string or text `content`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GoDerived {
    /// The field id of the length.
    id: String,
    /// The Go type of the length's field.
    ty: String,
    /// The field id of the byte string or text it measures.
    content: String,
    /// The `sce:length-arith` the wire adds to the count of bytes, spelled as
    /// a Go term (` + 2`, ` - 1`), empty where it adds nothing.
    arith: String,
}

/// A rebuild of a printed Go codec's value with one element assigned: the
/// expression that makes the value again, and, for a byte string or a text,
/// the call that makes the bounded value of what is assigned, which can fail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GoRebuild {
    /// The call of `Make{Codec}`.
    pub(crate) value: String,
    /// The fallible call that makes the assigned element's bounded value,
    /// which `value` reads as [`GO_REBUILT_ELEMENT`]; `None` where the element
    /// is handed as it is.
    pub(crate) made: Option<String>,
}

/// The local a rebuild reads a bounded element it made through
/// [`GoRebuild::made`] from.
pub(crate) const GO_REBUILT_ELEMENT: &str = "sceMade";

impl GoPrinted {
    /// What a reader outside its package knows of the codec `plan`, whose
    /// value's struct is `struct_name`; the plan is one [`unlowered`]
    /// answers `None` for.
    pub(crate) fn of(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> Self {
        let presented = plan
            .elements
            .iter()
            .map(|element| {
                let id = element.field.id.clone();
                let bounded = match &element.value {
                    Value::Bytes(_) => Some(("MakeBytes", bound_type(struct_name, &id))),
                    Value::Text(_) => Some(("MakeText", bound_type(struct_name, &id))),
                    _ => None,
                };
                GoPresented { id, bounded }
            })
            .collect();
        let derived = plan
            .derivations
            .iter()
            .filter_map(|derived| {
                let (ElementId::Field(id), Derive::Number(number)) =
                    (&derived.element, &derived.by)
                else {
                    return None;
                };
                let ([measure], NumberInto::Field { field, .. }) =
                    (number.measures.as_slice(), &number.into)
                else {
                    return None;
                };
                let Measured::Bytes {
                    field: content,
                    offset,
                } = &measure.of
                else {
                    return None;
                };
                Some(GoDerived {
                    id: id.clone(),
                    ty: l.type_name(&field.sce_type).into_owned(),
                    content: content.id.clone(),
                    arith: length_arith(offset),
                })
            })
            .collect();
        GoPrinted { presented, derived }
    }

    /// The read of the element `id`, of the type `ty`, of the value `held`:
    /// an element the value presents by its method, a byte string or a text
    /// as the slice or the string it holds ([`element_read`]); an element the
    /// encode derives as the encode derives it, from what it measures, since
    /// the value holds no field for it (docs/adr/0081 §3).
    pub(crate) fn read(&self, held: &str, id: &str, ty: &SceType) -> String {
        match self.derived.iter().find(|d| d.id == id) {
            Some(derived) => format!(
                "{}({held}.{}().Len(){})",
                derived.ty,
                filters::to_pascal_case(derived.content.clone()),
                derived.arith
            ),
            None => element_read(
                &format!("{held}.{}()", filters::to_pascal_case(id.to_string())),
                ty,
            ),
        }
    }

    /// The rebuild of the value `held`, made by `make` (the qualified
    /// `Make{Codec}`) in the package `package` names, with the element
    /// `element` assigned `value` and every other element the value presents
    /// read by its method; `None` where the value presents no `element`, as
    /// for a length the encode derives, which no value holds.
    ///
    /// A byte string or a text is assigned as the slice or the string the
    /// expression gives, made into its bounded value by the runtime's
    /// constructor, which refuses one outside the bound or a text that is not
    /// UTF-8 ([`GoRebuild::made`]).
    pub(crate) fn rebuilt(
        &self,
        held: &str,
        make: &str,
        package: Option<&str>,
        element: &str,
        value: &str,
    ) -> Option<GoRebuild> {
        let assigned = self.presented.iter().find(|p| p.id == element)?;
        let made = assigned.bounded.as_ref().map(|(constructor, bound)| {
            let bound = match package {
                Some(package) => format!("{package}.{bound}"),
                None => bound.clone(),
            };
            format!("codec.{constructor}[{bound}]({value})")
        });
        let arguments: Vec<String> = self
            .presented
            .iter()
            .map(|p| {
                if p.id != element {
                    format!("{held}.{}()", filters::to_pascal_case(p.id.clone()))
                } else if made.is_some() {
                    GO_REBUILT_ELEMENT.to_string()
                } else {
                    value.to_string()
                }
            })
            .collect();
        Some(GoRebuild {
            value: format!("{make}({})", arguments.join(", ")),
            made,
        })
    }
}

/// The `sce:length-arith` of a derived length as a Go term the count of bytes
/// is followed by: what the wire adds to it, or takes off.
fn length_arith(offset: &Offset) -> String {
    match offset {
        Offset::None => String::new(),
        Offset::More { by } => format!(" + {by}"),
        Offset::Less { by, .. } => format!(" - {by}"),
    }
}

/// The type a codec `struct_name` names for the bound of its byte string or
/// text `id`.
fn bound_type(struct_name: &str, id: &str) -> String {
    format!(
        "{struct_name}{}Bound",
        filters::to_pascal_case(id.to_string())
    )
}

/// The Go text the printer gives a codec's file.
pub(super) struct GoSource {
    /// Before the value's struct: the bound type of each byte string and text,
    /// and the interface a source of the codec satisfies.
    pub(super) before: String,
    /// After the value's struct: its constructor, the method that presents
    /// each element, and the encode over any source, `EncodeSource`.
    pub(super) after: String,
    /// For each byte string and text the value presents, by field id, the Go
    /// type its field is declared as: `codec.Bytes` or `codec.Text` of its
    /// bound.
    pub(super) held: BTreeMap<String, String>,
    /// For each byte string and text, by field id, the local the decode makes
    /// the bounded value of it in, which the value's literal holds.
    pub(super) made: BTreeMap<String, String>,
    /// The statements the decode runs before the value's literal: each byte
    /// string and text it read made into its bounded value, refused as the
    /// runtime refuses it; empty where the value presents none.
    pub(super) decoded: String,
}

/// The Go text of the codec `plan`, whose value's struct is `struct_name`; the
/// plan is one [`unlowered`] answers `None` for.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> GoSource {
    let mut before = String::new();
    let mut held = BTreeMap::new();
    let mut made = BTreeMap::new();
    let mut decoded = String::new();
    // Each element: its method, its field, and the Go type it is presented as.
    let mut elements: Vec<(String, String, String)> = Vec::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let method = l.codec_field_id(id);
        let (Value::Bytes(bound) | Value::Text(bound)) = &element.value else {
            elements.push((
                method,
                member(id),
                l.type_name(&element.field.sce_type).into_owned(),
            ));
            continue;
        };
        let text = matches!(element.value, Value::Text(_));
        let (kind, unit) = if text {
            ("Text", "UTF-8 bytes")
        } else {
            ("Bytes", "bytes")
        };
        let bound_name = bound_type(struct_name, id);
        let (held_as, most) = match bound.most {
            Some(most) => (
                format!("{} to {most} {unit}", bound.least),
                most.to_string(),
            ),
            None => (
                format!(
                    "{} or more {unit}, the protocol stating no most",
                    bound.least
                ),
                "codec.Unbounded".to_string(),
            ),
        };
        before.push_str(&format!(
            "// {bound_name} is the bound of the {id} element of {struct_name}: {held_as}\n\
             // (docs/adr/0077 §3). A {kind} of it is made only within it.\n\
             type {bound_name} struct{{}}\n\
             \n\
             // Least is the fewest {unit} the element holds.\n\
             func ({bound_name}) Least() int {{ return {least} }}\n\
             \n\
             // Most is the most {unit} the element holds.\n\
             func ({bound_name}) Most() int {{ return {most} }}\n\n",
            least = bound.least
        ));
        let ty = format!("codec.{kind}[{bound_name}]");
        // The decode read the element as a slice or a string; the bounded
        // value is made of it, a slice made a string for a text, and refused
        // as the runtime refuses it.
        let local = format!("_b{method}");
        let from = if text {
            format!("string({method})")
        } else {
            method.clone()
        };
        decoded.push_str(&format!(
            "\t{local}, err := codec.Make{kind}[{bound_name}]({from})\n\
             \tif err != nil {{\n\
             \t\treturn nil, err\n\
             \t}}\n"
        ));
        made.insert(id.clone(), local);
        held.insert(id.clone(), ty.clone());
        elements.push((method, member(id), ty));
    }

    before.push_str(&format!(
        "// {struct_name}Source is what the encode of {struct_name} reads: one method\n\
         // for each element a value presents (docs/adr/0081 §2). {struct_name} is one,\n\
         // and so is any value with these methods. The encode derives every other\n\
         // element of the frame.\n\
         type {struct_name}Source interface {{\n"
    ));
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

    GoSource {
        before,
        after,
        held,
        made,
        decoded: decoded.trim_end_matches('\n').to_string(),
    }
}

/// One write of `bytes`, the byte expressions of a write, into the sink.
fn write_bytes(bytes: &[String]) -> String {
    format!(
        "\tif err := w.WriteBytes([]byte{{{}}}); err != nil {{\n\t\treturn err\n\t}}\n",
        bytes.join(", ")
    )
}

/// One write of the slice `slice` into the sink.
fn write_slice(slice: &str) -> String {
    format!("\tif err := w.WriteBytes({slice}); err != nil {{\n\t\treturn err\n\t}}\n")
}

/// The encode of `struct_name` over any source of it, a function of the
/// codec's package, which is the codec's namespace as a C++ codec's namespace
/// is: each derived number made from what it measures, then the writes in
/// wire order, each element read off the source by its method. A function of
/// the package and not a method of the value, so a source need not be a value
/// of the codec.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> String {
    let numbers = derived_numbers(plan);
    let derives = if numbers.is_empty() {
        String::new()
    } else {
        format!(
            " It derives {} from\n// what each measures, and never reads it off the source.",
            numbers.iter().cloned().collect::<Vec<_>>().join(", ")
        )
    };
    let mut out = format!(
        "// EncodeSource writes any source of {struct_name} into w, each element read\n\
         // off src and written where the codec's encode plan lays it out\n\
         // (docs/adr/0081).{derives} It refuses no value: it returns only the\n\
         // error of w, codec.ErrBufferOverflow from a bounded sink that has no room\n\
         // left.\n\
         func EncodeSource(src {struct_name}Source, w codec.SceSink) error {{\n"
    );
    let source = |id: &str| format!("src.{}()", l.codec_field_id(id));
    // Each derived number, as a value of its field's type: the bytes the
    // bounded value holds, less or more its arithmetic.
    for derived in &plan.derivations {
        let (ElementId::Field(id), Derive::Number(number)) = (&derived.element, &derived.by) else {
            unreachable!("`unlowered` prints a derived length only");
        };
        let ([measure], NumberInto::Field { field, .. }) =
            (number.measures.as_slice(), &number.into)
        else {
            unreachable!("`unlowered` prints a length of one element into a field");
        };
        let Measured::Bytes {
            field: content,
            offset,
        } = &measure.of
        else {
            unreachable!("`unlowered` prints the length of a byte string or a text only");
        };
        let arith = length_arith(offset);
        out.push_str(&format!(
            "\t_d{} := {}({}.Len(){arith})\n",
            l.codec_field_id(id),
            l.type_name(&field.sce_type),
            source(&content.id)
        ));
    }
    let read = |f: &CodecField| -> String {
        if numbers.contains(&f.id) {
            format!("_d{}", l.codec_field_id(&f.id))
        } else {
            source(&f.id)
        }
    };
    // A byte string's bytes or a text's, as the slice the sink writes.
    let slice = |f: &CodecField| -> String {
        if f.sce_type == SceType::String {
            format!("[]byte({}.String())", read(f))
        } else {
            format!("{}.Bytes()", read(f))
        }
    };
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
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    out.push_str(&format!(
                        "\tfor w.Position() < {} {{\n\
                         \t\tif err := w.WriteBytes([]byte{{0}}); err != nil {{\n\
                         \t\t\treturn err\n\
                         \t\t}}\n\
                         \t}}\n",
                        field.byte_offset
                    ));
                }
                out.push_str(&write_slice(&slice(field)));
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
                BitSize::Tail | BitSize::LengthRef => out.push_str(&write_slice(&slice(field))),
                _ => unreachable!("`unlowered` prints no body or list"),
            },
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    out.push_str("\treturn nil\n}");
    out
}
