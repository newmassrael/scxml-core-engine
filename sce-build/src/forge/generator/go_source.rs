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
//! An element a gate decides, and a length the value states only while its
//! content is absent (docs/adr/0058 §1), is presented as the runtime's
//! `optional.Of` of what it holds. The encode derives each flag such an
//! element's presence says, and folds it into its carrier in place of the
//! bits the value presents.
//!
//! An encode that can refuse returns the refusal the plan's totality names,
//! the runtime's sentinel of its name (`codec.ErrPresentIfMismatch`,
//! `codec.ErrInconsistentSource`, `codec.ErrLengthOutOfRange`), before it
//! writes the first byte; one that cannot returns only the sink's error.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::codec_roles::ElementId;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Clause, CodecPlan, Derive, Element, EncodePlan, Guard, Measure, Measured, Narrow,
    Number, NumberInto, Offset, Presence, PresenceFlag, Value, Write,
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
/// enums, byte strings and texts, each always there or decided by a gate on a
/// flag of the codec's own carriers or on an input the encode is handed, the
/// integers written whole, in bytes they share with other fields, or as a
/// variable-length integer of 16, 32 or 64 bits, and the byte strings and
/// texts written to the end of the frame or by a length; whose every derived
/// element is a length of byte strings or texts, held in a field or in a flag
/// of a carrier the value presents, its `sce:length-arith` never taking it
/// below zero, or a flag the presence of one element says; and whose encode
/// refuses what its totality names of these. A CBOR map and a content-line
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
    // An input the value says is read off it by a parent; one it does not say
    // is a parameter of the encode.
    for said in &plan.said {
        forms.push(format!(
            "input '{}': a flag input the value says",
            said.input.name
        ));
    }
    // An element a gate decides, and a length the value states while its
    // content is absent, are presented as the optional type of what they
    // hold, whatever they hold.
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
    }
    if !plan.groups().is_empty() {
        forms.push("a group of fields a gate decides together".to_string());
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
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a length \
                 or a presence flag"
            )),
        }
    }
    // A flag the encode derives is folded into its carrier as the value
    // presents it, which the value then presents whole.
    for carrier in folded_carriers(plan) {
        if element_of(plan, carrier).map(|e| e.presence) != Some(Presence::Always) {
            forms.push(format!(
                "field '{carrier}': a carrier of a derived flag the value does not present whole"
            ));
        }
    }
    // A gate the encode cannot derive the presence from is a check the value
    // is held to before the first byte, as the encode writes the carriers it
    // reads.
    for field in &plan.gate_checks {
        forms.extend(gate_form(plan, field));
    }
    if !plan.input_checks.is_empty() {
        forms.push("an input a held codec says, held to what the codec is handed".to_string());
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
                if *guard == Guard::Gate {
                    forms.extend(gate_form(plan, field));
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
/// prints a length of the byte strings and texts it measures, each of which
/// that is there says the same number or the value is refused, its
/// `sce:length-arith` never taking the number below zero, into a field that
/// holds nothing else, refused where it may not fit, or into a flag of a
/// carrier; where nothing it measures is there the field holds zero, gated
/// beside its content, or what the value states.
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    for measure in &number.measures {
        match &measure.of {
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
    }
    if let NumberInto::Field {
        absent: Absent::Group,
        ..
    } = &number.into
    {
        forms.push(format!(
            "element '{element}': a length a group holds on the side without its content"
        ));
    }
    forms
}

/// What of the presence flag `element` the printer does not print: it prints
/// a flag the presence of one element decides.
fn presence_form(element: &ElementId, presence: &PresenceFlag<'_>) -> Option<String> {
    (presence.of.len() != 1)
        .then(|| format!("element '{element}': a flag the presence of several fields decides"))
}

/// What of the gate of `field`, a field written where its gate holds or held
/// to it, the printer does not print: it tests a flag of the codec's own
/// carriers or an input the encode is handed.
fn gate_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    let gate = field.present_if.as_ref()?;
    plan.clauses(gate)
        .iter()
        .any(|clause| matches!(clause, Clause::Member { .. }))
        .then(|| format!("field '{}': a gate on a chain's entries", field.id))
}

/// The carriers the encode folds a flag it derives into, by field id: a flag
/// the presence of an element says, or one that holds a length.
fn folded_carriers<'p>(plan: &'p EncodePlan<'_>) -> BTreeSet<&'p str> {
    plan.derivations
        .iter()
        .filter_map(|derived| match (&derived.element, &derived.by) {
            (ElementId::Flag { field, .. }, Derive::Presence(_) | Derive::Number(_)) => {
                Some(field.as_str())
            }
            _ => None,
        })
        .collect()
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
/// holds no field `id`: the encode derives it (docs/adr/0081 §3) and the
/// value does not state it while its content is absent.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id) && element_of(plan, id).is_none()
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
/// the numbers its encode derives into a field, which the value answers by
/// the method [`wire_method`] names. A procedure, a validator and an
/// algorithm that hold a value of the codec read and make it through this
/// ([`GoPrinted::read`], [`GoPrinted::rebuilt`]), so none spells a printed
/// codec's interface of its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GoPrinted {
    /// Each element the value presents, in declaration order.
    presented: Vec<GoPresented>,
    /// The field id of each number the encode derives into a field.
    derived: Vec<String>,
}

/// One element a printed Go value presents.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GoPresented {
    /// The field id.
    id: String,
    /// For a byte string or a text, the runtime's bounded type of it (`Bytes`
    /// or `Text`) and the bound type the codec's package names for it; `None`
    /// for any other element.
    bounded: Option<(&'static str, String)>,
    /// Whether the value presents it as the optional type of what it holds: a
    /// gate decides it, or the value states it only while its content is
    /// absent.
    optional: bool,
}

/// A rebuild of a printed Go codec's value with one element assigned: the
/// expression that makes the value again, and, for a byte string or a text,
/// the call that makes the bounded value of what is assigned, which can fail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GoRebuild {
    /// The call of `Make{Codec}`.
    pub(crate) value: String,
    /// The fallible call that makes the assigned element's bounded value,
    /// which `value` reads as [`GO_REBUILT_ELEMENT`], and the value assigned,
    /// which is held as [`GO_REBUILT_VALUE`] first and which the call reads
    /// there; `None` where the element is handed as it is.
    pub(crate) made: Option<(String, String)>,
}

/// The local a rebuild reads a bounded element it made through
/// [`GoRebuild::made`] from.
pub(crate) const GO_REBUILT_ELEMENT: &str = "sceMade";

/// The local a rebuild holds the value assigned to a byte string or a text
/// in, which the call of [`GoRebuild::made`] makes the bounded value of.
pub(crate) const GO_REBUILT_VALUE: &str = "sceValue";

impl GoPrinted {
    /// What a reader outside its package knows of the codec `plan`, whose
    /// value's struct is `struct_name`; the plan is one [`unlowered`]
    /// answers `None` for.
    pub(crate) fn of(plan: &EncodePlan<'_>, struct_name: &str) -> Self {
        let presented = plan
            .elements
            .iter()
            .map(|element| {
                let id = element.field.id.clone();
                let bounded = match &element.value {
                    Value::Bytes(_) => Some(("Bytes", bound_type(struct_name, &id))),
                    Value::Text(_) => Some(("Text", bound_type(struct_name, &id))),
                    _ => None,
                };
                GoPresented {
                    id,
                    bounded,
                    optional: element.presence.optional(),
                }
            })
            .collect();
        let derived = wire_numbers(plan)
            .into_iter()
            .map(|(id, _)| id.to_string())
            .collect();
        GoPrinted { presented, derived }
    }

    /// The read of the element `id`, of the type `ty`, of the value `held`:
    /// an element the value presents by its method, a byte string or a text
    /// as the slice or the string it holds ([`element_read`]); a number the
    /// encode derives into a field by the method that derives it as the
    /// encode does ([`wire_method`]), since the value holds no field for it
    /// (docs/adr/0081 §3), and a length the value states while its content is
    /// absent as the number the frame carries, which that method answers too.
    /// A byte string or a text a gate decides is read as the optional type of
    /// the slice or the string, as the template's value held it.
    pub(crate) fn read(&self, held: &str, id: &str, ty: &SceType) -> String {
        if self.derived.iter().any(|derived| derived == id) {
            return format!("{held}.{}()", wire_method(id));
        }
        let call = format!("{held}.{}()", filters::to_pascal_case(id.to_string()));
        match self.presented.iter().find(|p| p.id == id) {
            Some(GoPresented {
                bounded: Some((kind, _)),
                optional: true,
                ..
            }) => format!("codec.Optional{kind}({call})"),
            _ => element_read(&call, ty),
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
    /// UTF-8 ([`GoRebuild::made`]); one a gate decides, as the optional type
    /// of them, by the constructor over that type. Any other element is
    /// handed as the expression gives it, which is the type the template's
    /// value held it as, the optional type where a gate decides it.
    pub(crate) fn rebuilt(
        &self,
        held: &str,
        make: &str,
        package: Option<&str>,
        element: &str,
        value: &str,
    ) -> Option<GoRebuild> {
        let assigned = self.presented.iter().find(|p| p.id == element)?;
        let made = assigned.bounded.as_ref().map(|(kind, bound)| {
            let bound = match package {
                Some(package) => format!("{package}.{bound}"),
                None => bound.clone(),
            };
            let optional = if assigned.optional { "Optional" } else { "" };
            (
                format!("codec.Make{optional}{kind}[{bound}]({GO_REBUILT_VALUE})"),
                value.to_string(),
            )
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

/// Each number the encode of `plan` derives into a field, by the field's id,
/// in the order of the derivations.
fn wire_numbers<'p, 'm>(plan: &'p EncodePlan<'m>) -> Vec<(&'p str, &'p Number<'m>)> {
    plan.derivations
        .iter()
        .filter_map(|derived| match (&derived.element, &derived.by) {
            (ElementId::Field(id), Derive::Number(number))
                if matches!(number.into, NumberInto::Field { .. }) =>
            {
                Some((id.as_str(), number))
            }
            _ => None,
        })
        .collect()
}

/// The method of a printed Go value that answers the number its encode
/// derives into the field `id`, as the frame carries it: `Wire` and the
/// field's Go name. The value holds no field for a derived number
/// (docs/adr/0081 §3), and a length it states while its content is absent is
/// presented by the method of the field's own name, so the number the frame
/// carries is a method of another name.
pub(crate) fn wire_method(id: &str) -> String {
    format!("Wire{}", filters::to_pascal_case(id.to_string()))
}

/// The methods of the value `struct_name` that answer each number its encode
/// derives into a field as the frame carries it ([`wire_method`]), read off
/// the elements the value holds as the encode reads them off a source: the
/// first measure always there; where every measure may be absent, the first
/// one there, and where none is, zero, what the value states, or, for a
/// length gated beside its content, no number, as the field is then not on
/// the wire. A value whose measures disagree has no frame, and its encode
/// refuses it; the method answers its first measure.
fn wire_methods(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> String {
    let mut out = String::new();
    let mut seen = BTreeSet::new();
    for (id, number) in wire_numbers(plan) {
        if !seen.insert(id) {
            continue;
        }
        let NumberInto::Field { field, absent, .. } = &number.into else {
            unreachable!("`wire_numbers` keeps a number into a field");
        };
        let ty = l.type_name(&field.sce_type).into_owned();
        let method = wire_method(id);
        let optional = *absent == Absent::Zero;
        let returned = if optional {
            format!("optional.Of[{ty}]")
        } else {
            ty.clone()
        };
        let wrap = |size: &str| {
            if optional {
                format!("optional.Some({ty}({size}))")
            } else {
                format!("{ty}({size})")
            }
        };
        let mut body = String::new();
        match number.measures.iter().find(|measure| !measure.gated) {
            Some(anchor) => {
                let size = held_measure(anchor, &held_element(anchor));
                body.push_str(&format!("\treturn {}\n", wrap(&size)));
            }
            None => {
                for measure in &number.measures {
                    let size = held_measure(measure, "_v");
                    body.push_str(&format!(
                        "\tif _v, ok := {}.Get(); ok {{\n\t\treturn {}\n\t}}\n",
                        held_element(measure),
                        wrap(&size)
                    ));
                }
                body.push_str(&match absent {
                    Absent::Zero => format!("\treturn optional.None[{ty}]()\n"),
                    Absent::Stated => format!(
                        "\tif _s, ok := s.{}.Get(); ok {{\n\t\treturn _s\n\t}}\n\treturn 0\n",
                        member(id)
                    ),
                    Absent::Never => "\treturn 0\n".to_string(),
                    Absent::Group => unreachable!("`unlowered` prints no length a group holds"),
                });
            }
        }
        out.push_str(&format!(
            "\n// {method} returns the {id} the frame of the value carries, which the\n\
             // encode derives from what it measures (docs/adr/0081 §3).\n\
             func (s *{struct_name}) {method}() {returned} {{\n{body}}}\n"
        ));
    }
    out
}

/// The field of the value that holds the content `measure` measures.
fn held_element(measure: &Measure<'_>) -> String {
    let content = match &measure.of {
        Measured::Bytes { field, .. } => field,
        _ => unreachable!("`unlowered` prints a length of byte strings or texts only"),
    };
    format!("s.{}", member(&content.id))
}

/// The number `measure` gives of the content `held` holds, an `int`: the
/// bytes of a byte string or a text, less or more its `sce:length-arith`.
fn held_measure(measure: &Measure<'_>, held: &str) -> String {
    match &measure.of {
        Measured::Bytes { offset, .. } => format!("{held}.Len(){}", length_arith(offset)),
        _ => unreachable!("`unlowered` prints a length of byte strings or texts only"),
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
    /// For each element the value presents, by field id, the Go type its
    /// field is declared as: `codec.Bytes` or `codec.Text` of its bound for a
    /// byte string or a text, the `optional.Of` of what it holds where it may
    /// be absent.
    pub(super) held: BTreeMap<String, String>,
    /// For each byte string and text, by field id, the local the decode makes
    /// the bounded value of it in, and for each length the value states, the
    /// optional value of the number the decode read: what the value's literal
    /// holds in place of what the decode read.
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
        let optional = element.presence.optional();
        let (Value::Bytes(bound) | Value::Text(bound)) = &element.value else {
            let scalar = l.type_name(&element.field.sce_type).into_owned();
            let ty = if optional {
                format!("optional.Of[{scalar}]")
            } else {
                scalar
            };
            // A length the value states while its content is absent is on the
            // wire whether or not the content is there: the decode read it
            // as a number, which the value states.
            if element.presence == Presence::Stated {
                made.insert(id.clone(), format!("optional.Some({method})"));
            }
            held.insert(id.clone(), ty.clone());
            elements.push((method, member(id), ty));
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
        let bounded = format!("codec.{kind}[{bound_name}]");
        // The decode read the element as a slice or a string, or as the
        // optional type of one where a gate decides it; the bounded value is
        // made of it, a slice made a string for a text, and refused as the
        // runtime refuses it.
        let local = format!("_b{method}");
        let (ty, make) = if optional {
            (
                format!("optional.Of[{bounded}]"),
                format!("codec.MakeOptional{kind}[{bound_name}]({method})"),
            )
        } else if text {
            (
                bounded,
                format!("codec.Make{kind}[{bound_name}](string({method}))"),
            )
        } else {
            (bounded, format!("codec.Make{kind}[{bound_name}]({method})"))
        };
        decoded.push_str(&format!(
            "\t{local}, err := {make}\n\
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
    after.push_str(&wire_methods(plan, l, struct_name));
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

/// Each line of `body`, Go statements, one tab further in.
fn indented(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_string()
            } else {
                format!("\t{line}\n")
            }
        })
        .collect()
}

/// How the encode reads an element off its source.
struct SourceReads<'p, 'm> {
    /// The plan of the codec.
    plan: &'p EncodePlan<'m>,
    /// The Go spelling of the codec's names.
    l: &'p LangCtx,
}

impl SourceReads<'_, '_> {
    /// The call of the method that presents the element `id`.
    fn call(&self, id: &str) -> String {
        format!("src.{}()", self.l.codec_field_id(id))
    }

    /// The element `id` as the encode reads it where it is there: the call,
    /// or what the optional type the call returns holds, read where a test
    /// has proven it present.
    fn value(&self, id: &str) -> String {
        if element_of(self.plan, id).is_some_and(|e| e.presence.optional()) {
            format!("{}.Proven()", self.call(id))
        } else {
            self.call(id)
        }
    }

    /// The test that the element `id`, which may be absent, is there.
    fn present(&self, id: &str) -> String {
        format!("{}.IsPresent()", self.call(id))
    }
}

/// The Go name of the flag input `name`: the parameter the encode and the
/// decode are handed it by, as the template names it.
fn input_name(name: &str) -> String {
    filters::to_camel_case(name.to_string())
}

/// The local the encode holds the carrier `id` in as it writes it, its flags
/// the encode derives folded in.
fn carrier_local(l: &LangCtx, id: &str) -> String {
    format!("_c{}", l.codec_field_id(id))
}

/// The flag `flag` of the carrier `field` of `plan`.
fn flag_def<'m>(plan: &EncodePlan<'m>, field: &str, flag: &str) -> &'m FlagDef {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == field)
        .and_then(|carrier| carrier.flags.iter().find(|d| d.name == flag))
        .expect("the parser found the flag a derivation names")
}

/// The bits of `def`, unshifted.
fn flag_width_mask(def: &FlagDef) -> u64 {
    if def.width >= 64 {
        u64::MAX
    } else {
        (1u64 << def.width) - 1
    }
}

/// The statement that returns the refusal of `kind` at `at`, the runtime's
/// sentinel of the refusal's name, which the totality analysis names
/// (`Totality::require`, docs/adr/0065 §2).
fn refusal(plan: &EncodePlan<'_>, kind: RefusalKind, at: &str) -> String {
    plan.totality.require(kind, at);
    let variant = kind
        .error_variant()
        .expect("a refusal the printer writes is one of the codec's own");
    format!("return codec.Err{variant}")
}

/// The encode of `struct_name` over any source of it, a function of the
/// codec's package, which is the codec's namespace as a C++ codec's namespace
/// is: each refusal the plan's totality names checked before the first byte,
/// each derived number made from what it measures, each derived flag folded
/// into its carrier, then the writes in wire order, each element read off the
/// source by its method. A function of the package and not a method of the
/// value, so a source need not be a value of the codec.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> String {
    let numbers = derived_numbers(plan);
    let (stated, derived): (Vec<&String>, Vec<&String>) = numbers
        .iter()
        .partition(|id| element_of(plan, id).is_some());
    let named = |ids: &[&String]| {
        ids.iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut derives = String::new();
    if !derived.is_empty() {
        derives.push_str(&format!(
            " It derives {} from\n// what each measures, and never reads it off the source.",
            named(&derived)
        ));
    }
    if !stated.is_empty() {
        derives.push_str(&format!(
            " It derives {} where\n// what it measures is there, and reads what the source states \
             where none is.",
            named(&stated)
        ));
    }
    // What the encode refuses: nothing, or what the plan's totality names,
    // each the runtime's sentinel of its name, before the first byte.
    let refuses = if plan.totality.is_total() {
        " It refuses no value: it returns only the\n// error of w".to_string()
    } else {
        let mut returned: Vec<String> = plan
            .totality
            .refusals()
            .iter()
            .filter_map(|r| r.kind.error_variant())
            .map(|variant| format!("codec.Err{variant}"))
            .collect();
        returned.sort();
        returned.dedup();
        format!(
            " It returns {} for\n// a value no frame carries, before it writes a byte to w, and \
             otherwise only the\n// error of w",
            returned.join(", ")
        )
    };
    // The flag inputs the encode is handed, those the value does not say,
    // after the sink, as the decode is handed them after the cursor.
    let given: String = plan
        .given
        .iter()
        .map(|input| format!(", {} byte", input_name(&input.name)))
        .collect();
    let mut out = format!(
        "// EncodeSource writes any source of {struct_name} into w, each element read\n\
         // off src and written where the codec's encode plan lays it out\n\
         // (docs/adr/0081).{derives}{refuses}, codec.ErrBufferOverflow from a bounded sink\n\
         // that has no room left.\n\
         func EncodeSource(src {struct_name}Source, w codec.SceSink{given}) error {{\n"
    );
    let reads = SourceReads { plan, l };
    // Each flag the encode derives, by its carrier: the statements that fold
    // it into the carrier's local in place of the bits the value presents.
    let mut folds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (element, Derive::Number(number)) => {
                let measured: Vec<MeasureText> = number
                    .measures
                    .iter()
                    .map(|measure| plain_measure(&reads, measure))
                    .collect();
                if let Some((carrier, statement)) =
                    derive_number(plan, l, &reads, element, &number.into, &measured, &mut out)
                {
                    folds.entry(carrier).or_default().push(statement);
                }
            }
            // Set exactly where the element it gates is there, or clear where
            // the element is there on the flag's clear state.
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let def = flag_def(plan, field, flag);
                let mask = flag_width_mask(def) << def.bit;
                let c = carrier_local(l, field);
                let test = if *when_clear {
                    format!("!{}", reads.present(&gated.id))
                } else {
                    reads.present(&gated.id)
                };
                folds.entry(field.clone()).or_default().push(format!(
                    "\t{c} &^= 0x{mask:X}\n\
                     \tif {test} {{\n\
                     \t\t{c} |= 0x{mask:X}\n\
                     \t}}\n"
                ));
            }
            _ => unreachable!("`unlowered` prints no other derivation"),
        }
    }
    // Each carrier as the encode writes it: its flags the value presents, and
    // those the encode derives folded in, in place of the value's.
    for (carrier, carried) in &folds {
        out.push_str(&format!(
            "\t{} := {}\n",
            carrier_local(l, carrier),
            reads.call(carrier)
        ));
        for statement in carried {
            out.push_str(statement);
        }
    }
    let read = |f: &CodecField| -> String {
        if numbers.contains(&f.id) {
            format!("_d{}", l.codec_field_id(&f.id))
        } else if folds.contains_key(&f.id) {
            carrier_local(l, &f.id)
        } else {
            reads.value(&f.id)
        }
    };
    // A gate, tested on the carriers as the encode writes them and on the
    // inputs it is handed.
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
            .join(" || ")
    };
    // Each gate the encode cannot derive the presence from holds the field it
    // gates, before the first byte (docs/adr/0065 §2).
    for field in &plan.gate_checks {
        out.push_str(&format!(
            "\tif ({}) != {} {{\n\t\t{}\n\t}}\n",
            gate_test(field),
            reads.present(&field.id),
            refusal(plan, RefusalKind::PresentIfMismatch, &field.id)
        ));
    }
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
            Write::Streamed { field, guard, .. } => {
                let body = match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        write_bytes(&encode_whole_field_read_as(field, endian, &read(field), l))
                    }
                    // The runtime's writer, as the template's encode calls it.
                    BitSize::Vle { width_bits } => {
                        let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                        format!("{}\n", vle_encode_block(&value, *width_bits, l.lang))
                    }
                    BitSize::Tail | BitSize::LengthRef => write_slice(&slice(field)),
                    _ => unreachable!("`unlowered` prints no body or list"),
                };
                out.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => format!(
                        "\tif {} {{\n{}\t}}\n",
                        reads.present(&field.id),
                        indented(&body)
                    ),
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => {
                        format!("\tif {} {{\n{}\t}}\n", gate_test(field), indented(&body))
                    }
                });
            }
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    out.push_str("\treturn nil\n}");
    // Every refusal the totality analysis names is one the encode returns
    // (docs/adr/0065 §2).
    plan.totality.assert_all_written(struct_name);
    out
}

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, an `int`, which is read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
}

/// A measure of a derived number, read off the source: the bytes of a byte
/// string or a text, less or more its `sce:length-arith`, tested present
/// where a gate may leave it out.
fn plain_measure(reads: &SourceReads<'_, '_>, measure: &Measure<'_>) -> MeasureText {
    let Measured::Bytes { field, offset } = &measure.of else {
        unreachable!("`unlowered` prints a length of byte strings or texts only");
    };
    MeasureText {
        size: format!("{}.Len(){}", reads.value(&field.id), length_arith(offset)),
        present: measure.gated.then(|| reads.present(&field.id)),
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

/// The name the encode's locals of the derived `element` are named by.
fn derived_local(l: &LangCtx, element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => l.codec_field_id(id),
        ElementId::Flag { field, flag } => format!(
            "{}{}",
            l.codec_field_id(field),
            filters::to_pascal_case(flag.clone())
        ),
        ElementId::Variant => unreachable!("the plan derives nothing into the variant"),
    }
}

/// The statements, written into `out`, that make the derived number of
/// `element` from what it measures, as the Rust encode's `combine` makes it
/// (docs/adr/0081 §1): the first measure always there fixes it, and every
/// other one there must say the same, or the value is refused with
/// `codec.ErrInconsistentSource`; where every measure may be absent, it is the
/// first one there. A field holds it as `_d<Field>`, refused with
/// `codec.ErrLengthOutOfRange` where it may not fit; where nothing it
/// measures is there, the field holds zero or what the value states. A number
/// held in a flag is answered as the carrier and the statement that folds it
/// into the carrier's local.
fn derive_number(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    reads: &SourceReads<'_, '_>,
    element: &ElementId,
    into: &NumberInto<'_>,
    measures: &[MeasureText],
    out: &mut String,
) -> Option<(String, String)> {
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
            let always: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_none()).collect();
            let gated: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_some()).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    out.push_str(&format!(
                        "\tif {} != {} {{\n\t\t{}\n\t}}\n",
                        other.size,
                        anchor.size,
                        disagree()
                    ));
                }
                for g in &gated {
                    out.push_str(&format!(
                        "\tif {} && {} != {} {{\n\t\t{}\n\t}}\n",
                        there(g),
                        g.size,
                        anchor.size,
                        disagree()
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let local = derived_local(l, element);
                let (first, found) = (format!("_f{local}"), format!("_h{local}"));
                out.push_str(&format!("\t{first}, {found} := 0, false\n"));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "\tif" } else { " else if" };
                    out.push_str(&format!(
                        "{keyword} {} {{\n\t\t{first}, {found} = {}, true\n\t}}",
                        there(g),
                        g.size
                    ));
                }
                out.push('\n');
                for g in gated.iter().skip(1) {
                    out.push_str(&format!(
                        "\tif {} && {} != {first} {{\n\t\t{}\n\t}}\n",
                        there(g),
                        g.size,
                        disagree()
                    ));
                }
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
            let def = flag_def(plan, field, flag);
            let width_mask = flag_width_mask(def);
            let mask = width_mask << def.bit;
            let c = carrier_local(l, field);
            let carrier_type = plan
                .model
                .fields
                .iter()
                .find(|f| f.id == *field)
                .map(|carrier| l.type_name(&carrier.sce_type).into_owned())
                .expect("a number held in a flag is held in a flag of its carrier");
            let mut set = String::new();
            if plan.totality.refuses(RefusalKind::LengthOutOfRange, &named) {
                set.push_str(&format!(
                    "\tif {size} > 0x{width_mask:X} {{\n\t\t{}\n\t}}\n",
                    refusal(plan, RefusalKind::LengthOutOfRange, &named)
                ));
            }
            set.push_str(&format!(
                "\t{c} = ({c} &^ 0x{mask:X}) | ({carrier_type}({size}) << {})\n",
                def.bit
            ));
            let statement = match &present {
                None => set,
                Some(there) => format!("\tif {there} {{\n{}\t}}\n", indented(&set)),
            };
            return Some((field.clone(), statement));
        }
        _ => unreachable!("a number goes into the field or the flag it names"),
    };
    let name = l.codec_field_id(&field.id);
    let ty = l.type_name(&field.sce_type);
    // A number past what the field holds is no frame (docs/adr/0065 §5).
    if *narrow == Narrow::Checked {
        let most = largest_of(&field.sce_type)
            .expect("a length that may not fit its field is held in an integer field");
        let past = format!("{size} > 0x{most:X}");
        let test = match &present {
            Some(there) => format!("{there} && {past}"),
            None => past,
        };
        out.push_str(&format!(
            "\tif {test} {{\n\t\t{}\n\t}}\n",
            refusal(plan, RefusalKind::LengthOutOfRange, &named)
        ));
    }
    let d = format!("_d{name}");
    match &present {
        None => out.push_str(&format!("\t{d} := {ty}({size})\n")),
        // Where the content is not, zero, the field gated beside it and not
        // written, or what the value states.
        Some(there) => {
            out.push_str(&format!(
                "\tvar {d} {ty}\n\tif {there} {{\n\t\t{d} = {ty}({size})\n\t}}"
            ));
            if *absent == Absent::Stated {
                out.push_str(&format!(
                    " else if _s, ok := {}.Get(); ok {{\n\t\t{d} = _s\n\t}}",
                    reads.call(&field.id)
                ));
            }
            out.push('\n');
        }
    }
    // A value that states the number beside content says it twice, and must
    // say what the content does.
    if *absent == Absent::Stated {
        let there = present
            .as_deref()
            .expect("a number the value states is free: every measure may be absent");
        out.push_str(&format!(
            "\tif _s, ok := {}.Get(); ok && {there} && _s != {d} {{\n\t\t{}\n\t}}\n",
            reads.call(&field.id),
            disagree()
        ));
    }
    None
}
