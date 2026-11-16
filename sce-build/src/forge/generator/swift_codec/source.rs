// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift printer of a codec's encode plan (docs/adr/0081 step 5).
//!
//! A printed codec presents its elements through a protocol, `{Codec}Source`, each element a
//! read-only property of the type the value holds it as, `nil` where a gate leaves it out, and its
//! encode is `{Codec}.encodeSource(_:into:)` over any type that conforms: the host's own type is
//! encoded with no copy into the generated value, as a Rust value is through its source trait
//! (docs/adr/0058 §1). The generated value conforms, and its `encode(into:)` is `encodeSource` of
//! itself. Every decision of the encode is the plan's; this module spells it and decides none.
//!
//! The printer lowers the plan's forms one at a time (docs/adr/0081 §6). A codec whose plan uses a
//! form it does not lower yet keeps the encode of its template, and [`unlowered`] names that form,
//! so no codec has two encodes. The forms printed are a positional frame laid out by byte or
//! written in turn (a fixed number's bytes, a variable-length integer's chain), of numbers (an
//! integer, a flag set presented whole, a value of an imported enum), each always there or decided
//! by a gate, whose flag the encode derives from the presence of what it gates; and byte strings and
//! texts, each held in the runtime's `BoundedBytes` or `BoundedText` over a bound type the codec
//! declares (docs/adr/0065 §6 R1), whose length the encode derives into the field or the flag that
//! holds it, which the value then no longer holds (docs/adr/0081 §3). A flag input the
//! value does not say is a parameter of the encode, as it is of the decode, and a gate on it, which
//! the encode cannot derive, is a check the value is held to before the first byte
//! (`presentIfMismatch`); fields that each say one flag must say the same (`inconsistentSource`).
//! A one-bit flag a parent may set in place of the one the value presents
//! (`EncodePlan::overridable`) is a member of the codec's `Overrides`, which the encode is handed
//! and folds into its carrier.

use super::*;
use crate::forge::codec_roles::ElementId;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Bound, Clause, CodecPlan, Derive, Element, EncodePlan, Guard, Measured, Narrow, Number,
    NumberInto, Offset, Presence, PresenceFlag, Value, Write,
};

/// The first form of `plan` the Swift printer does not lower yet, described for a reader; `None`
/// where it prints the codec whole.
pub(crate) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    match plan {
        CodecPlan::Positional(plan) => unlowered_positional(plan),
        CodecPlan::Cbor(_) => Some("a CBOR map".to_string()),
        CodecPlan::ContentLine(_) => Some("a component of content lines".to_string()),
    }
}

/// [`unlowered`] of a positional codec's plan.
pub(crate) fn unlowered_positional(plan: &EncodePlan<'_>) -> Option<String> {
    if plan.model.variant.is_some() {
        return Some("a `<sce:variant>`".to_string());
    }
    if let Some(group) = plan.groups().first() {
        return Some(format!(
            "the gate group `{}`",
            super::super::rust_source::gates_label(group)
        ));
    }
    for element in &plan.elements {
        let id = &element.field.id;
        let what = match &element.value {
            Value::Scalar(t) if is_number(t) => None,
            Value::Scalar(t) => Some(format!("a `{}` value", t.as_attr())),
            Value::Bytes(_) | Value::Text(_) => None,
            Value::Body { .. } => Some("an embedded body".to_string()),
            Value::List { .. } => Some("a list of entries".to_string()),
        }
        .or_else(|| match element.presence {
            Presence::Always | Presence::Gated => None,
            Presence::Stated => {
                Some("a number stated where what it measures is absent".to_string())
            }
        });
        if let Some(what) = what {
            return Some(format!("field '{id}': {what}"));
        }
    }
    // A presence flag is read off the fields it gates that say it: the first, or every one where
    // they may disagree.
    let presented = |f: &CodecField| plan.elements.iter().any(|e| e.field.id == f.id);
    let readable = |presence: &PresenceFlag<'_>| {
        let (first, rest) = match presence.of.split_first() {
            Some(split) => split,
            None => return false,
        };
        presented(first.0) && (presence.together || rest.iter().all(|&(f, _)| presented(f)))
    };
    if let Some(derived) = plan.derivations.iter().find(|derived| {
        let presence = presence_flag(&derived.element, &derived.by)
            .is_some_and(|(_, _, presence)| readable(presence));
        !presence && byte_length(&derived.element, &derived.by).is_none()
    }) {
        return Some(format!("`{}`, which the encode derives", derived.element));
    }
    if let Some(said) = plan.said.first() {
        return Some(format!(
            "the flag input `{}`, which the value says",
            said.input.name
        ));
    }
    if let Some(check) = plan.input_checks.first() {
        return Some(format!(
            "the input `{}` the held codec `{}` says",
            check.input, check.codec
        ));
    }
    if let Some(field) = plan.gate_checks.iter().find(|field| {
        field.present_if.as_ref().is_some_and(|gate| {
            plan.clauses(gate)
                .iter()
                .any(|clause| matches!(clause, Clause::Member { .. }))
        })
    }) {
        return Some(format!(
            "field '{}': a gate on the entries of a chain",
            field.id
        ));
    }
    if let Some(refusal) = plan
        .totality
        .refusals()
        .iter()
        .find(|refusal| !writes_refusal(plan, refusal.kind, &refusal.element))
    {
        return Some(format!(
            "an encode that refuses a value at `{}` ({})",
            refusal.element, refusal.kind
        ));
    }
    let on_a_chain = |field: &CodecField| {
        field.present_if.as_ref().is_some_and(|gate| {
            plan.clauses(gate)
                .iter()
                .any(|clause| matches!(clause, Clause::Member { .. }))
        })
    };
    plan.program.iter().find_map(|write| match write {
        Write::Whole(_) | Write::Byte { .. } => None,
        Write::Streamed {
            field,
            guard: Guard::Gate,
            ..
        } if on_a_chain(field) => Some(format!(
            "field '{}': a gate on the entries of a chain",
            field.id
        )),
        // A number or a byte string or text in turn, always there or where the value holds it, and a
        // number the encode derives whole where its gate holds: the bytes of its carrier, its chain,
        // or its bytes.
        Write::Streamed {
            field, list: None, ..
        } if matches!(
            field.bit_size,
            BitSize::Fixed { .. } | BitSize::Vle { .. } | BitSize::Tail | BitSize::LengthRef
        ) =>
        {
            None
        }
        // The bytes or text after a frame laid out by byte, never gated (a gate makes a frame one
        // written in turn).
        Write::Tail { .. } => None,
        Write::Streamed { field, .. } => {
            Some(format!("field '{}': a field written in turn", field.id))
        }
        Write::Variant => Some("a variant's arm".to_string()),
    })
}

/// The number `element` is, where `by` derives it as one the printer writes: the length of the
/// byte strings and texts it measures, into a field of its own (zero where a gate leaves every one
/// of them out, beside which the field is gated too) or into a flag of a carrier.
fn byte_length<'d, 'm>(element: &'d ElementId, by: &'d Derive<'m>) -> Option<&'d Number<'m>> {
    let Derive::Number(number) = by else {
        return None;
    };
    let measures_bytes = number
        .measures
        .iter()
        .all(|measure| matches!(measure.of, Measured::Bytes { .. }));
    let into = match (element, &number.into) {
        (ElementId::Field(_), NumberInto::Field { absent, .. }) => {
            matches!(absent, Absent::Never | Absent::Zero)
        }
        (ElementId::Flag { .. }, NumberInto::Flag) => true,
        _ => false,
    };
    (measures_bytes && into).then_some(number)
}

/// The presence flag `element` is, where `by` derives it as one: a flag of a carrier set exactly
/// where what it gates is present.
fn presence_flag<'d, 'm>(
    element: &'d ElementId,
    by: &'d Derive<'m>,
) -> Option<(&'d str, &'d str, &'d PresenceFlag<'m>)> {
    match (element, by) {
        (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
            Some((field.as_str(), flag.as_str(), presence))
        }
        _ => None,
    }
}

/// Whether the printer writes the refusal of `kind` at `element`: the presence of a field held to a
/// gate the encode cannot derive; fields that each say one flag, or byte strings and texts that each
/// say one length, and may disagree; a length past the field that holds it where the plan checks
/// it, past the bits of the flag that holds it, or below what its arithmetic takes off.
fn writes_refusal(plan: &EncodePlan<'_>, kind: RefusalKind, element: &str) -> bool {
    let derived_at =
        |derived: &&crate::forge::encode_plan::Derived<'_>| derived.element.to_string() == element;
    match kind {
        RefusalKind::PresentIfMismatch => plan.gate_checks.iter().any(|f| f.id == element),
        RefusalKind::InconsistentSource => {
            plan.derivations.iter().filter(derived_at).any(|derived| {
                presence_flag(&derived.element, &derived.by).is_some()
                    || byte_length(&derived.element, &derived.by)
                        .is_some_and(|number| number.measures.len() > 1)
            })
        }
        RefusalKind::LengthOutOfRange => plan.derivations.iter().any(|derived| {
            let Some(number) = byte_length(&derived.element, &derived.by) else {
                return false;
            };
            let past_its_field = derived_at(&derived)
                && match &number.into {
                    NumberInto::Field { narrow, .. } => *narrow == Narrow::Checked,
                    NumberInto::Flag => true,
                };
            let below_its_arithmetic = number.measures.iter().any(|measure| {
                matches!(
                    measure.of,
                    Measured::Bytes {
                        field,
                        offset: Offset::Less { held: false, .. },
                    } if field.id == element
                )
            });
            past_its_field || below_its_arithmetic
        }),
        _ => false,
    }
}

/// Whether the printer writes a value of `t`: an integer of a native width, or a value of an
/// imported enum, written as its carrier.
fn is_number(t: &SceType) -> bool {
    t.int_bit_width().is_some() || matches!(t, SceType::Enum(_))
}

/// The protocol a value of the codec `codec` presents its elements through.
pub(crate) fn source_protocol(codec: &str) -> String {
    format!("{}Source", filters::to_pascal_case(codec.to_string()))
}

/// The name a printed encode gives the source it writes.
const SOURCE: &str = "source";

/// The name a printed encode gives what a gated element holds where it is present.
const HELD: &str = "held";

/// The statement that throws the refusal of `kind` at `element`, which the totality analysis names
/// (docs/adr/0065 §2) and records as written: the runtime's `CodecError` case of the variant it
/// names, in lower camel case.
fn refusal(plan: &EncodePlan<'_>, kind: RefusalKind, element: &str) -> String {
    let variant = plan.totality.require(kind, element);
    let variant = variant.trim_start_matches("CodecError::");
    let mut chars = variant.chars();
    let case = match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect::<String>(),
        None => unreachable!("an encode's own refusal names a variant of CodecError"),
    };
    format!("throw CodecError.{case}")
}

/// The bound type of the byte string or text `field` of a printed codec: a type the codec's value
/// declares, which a protocol of the host's module names through it.
fn bound_name(field: &str) -> String {
    format!("{}Bound", filters::to_pascal_case(field.to_string()))
}

/// The Swift type a printed codec named `struct_name` holds the byte string or text `element` as,
/// within the bound its own type declares (docs/adr/0065 §6 R1); `None` for another element.
pub(super) fn bounded_type(struct_name: &str, element: &Element<'_>) -> Option<String> {
    let runtime = match element.value {
        Value::Bytes(_) => "BoundedBytes",
        Value::Text(_) => "BoundedText",
        _ => return None,
    };
    Some(format!(
        "{runtime}<{struct_name}.{}>",
        bound_name(&element.field.id)
    ))
}

/// The Swift type the value of `plan` presents `element` as: its number, or its bounded byte
/// string or text, optional where a gate decides it.
fn presented_type(l: &LangCtx, plan: &EncodePlan<'_>, element: &Element<'_>) -> String {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    let ty = bounded_type(&struct_name, element)
        .unwrap_or_else(|| l.type_name(&element.field.sce_type).into_owned());
    if element.presence.optional() {
        format!("{ty}?")
    } else {
        ty
    }
}

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, an `Int`, which is read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
    /// Whether the plan says the measure may be absent (`Measure::gated`).
    gated: bool,
}

/// The local the encode holds the derived `element` in: a field's `<field>_derived`, a flag's
/// `<carrier>_<flag>_derived`. A field is read through the source and never bare, so the local
/// hides no field.
fn derived_local(element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => format!("{id}_derived"),
        ElementId::Flag { field, flag } => format!("{field}_{flag}_derived"),
        ElementId::Variant => unreachable!("a variant's tag is no number of bytes"),
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

/// The number the derived `named` holds, made from what it measures as the Rust encode's `combine`
/// makes it (docs/adr/0081 §1), and, where every measure may be absent, the test that one is there;
/// the statements it needs are pushed onto `body`. The first measure always there fixes it, and every
/// other one there must say the same, or the value is refused as `inconsistentSource`; where every
/// measure may be absent, it is the first one there, held in `<local>_first`.
fn combine(
    plan: &EncodePlan<'_>,
    named: &str,
    local: &str,
    measures: &[MeasureText],
    body: &mut Vec<String>,
) -> (String, Option<String>) {
    match measures {
        // Nothing measured: zero, as the Rust encode's `combine` holds it.
        [] => ("0".to_string(), None),
        [one] => (one.size.clone(), one.present.clone()),
        _ => {
            let disagree = || refusal(plan, RefusalKind::InconsistentSource, named);
            let there = |m: &MeasureText| m.present.clone().unwrap_or_else(|| "false".to_string());
            let always: Vec<&MeasureText> = measures.iter().filter(|m| !m.gated).collect();
            let gated: Vec<&MeasureText> = measures.iter().filter(|m| m.gated).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    body.push(format!(
                        "if {} != {} {{ {} }}",
                        other.size,
                        anchor.size,
                        disagree()
                    ));
                }
                for g in &gated {
                    body.push(format!(
                        "if {} && {} != {} {{ {} }}",
                        there(g),
                        g.size,
                        anchor.size,
                        disagree()
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let first = format!("{local}_first");
                body.push(format!("var {first}: Int? = nil"));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    body.push(format!("{keyword} {} {{", there(g)));
                    body.push(format!("    {first} = {}", g.size));
                }
                body.push("}".to_string());
                for g in gated.iter().skip(1) {
                    body.push(format!(
                        "if {} && {} != {first} {{ {} }}",
                        there(g),
                        g.size,
                        disagree()
                    ));
                }
                (format!("({first} ?? 0)"), Some(format!("{first} != nil")))
            }
        }
    }
}

/// What the template writes for the codec of `plan`, which [`unlowered`] has found nothing to
/// refuse in: the protocol and its requirements, one read-only property per element in
/// declaration order, the flag inputs the encode is handed, and the statements of `encodeSource`:
/// each carrier with the flags the encode derives or a parent sets folded in, the checks the value
/// is held to, and the writes of the plan's program, one after another, into a frame written in
/// one write.
pub(super) fn context(
    l: &LangCtx,
    plan: &EncodePlan<'_>,
    carriers: &std::collections::HashMap<String, Carrier>,
) -> serde_json::Value {
    let requirements: Vec<serde_json::Value> = plan
        .elements
        .iter()
        .map(|element| {
            serde_json::json!({
                "name": element.field.id,
                "type": presented_type(l, plan, element),
            })
        })
        .collect();
    // The bound each byte string and text is held to, a type of the codec's own (docs/adr/0065 §6
    // R1): the value's type of the element names it, so a value made for one is no value of another.
    let bounds: Vec<serde_json::Value> = plan
        .elements
        .iter()
        .filter_map(|element| {
            let (Value::Bytes(bound) | Value::Text(bound)) = &element.value else {
                return None;
            };
            let Bound { least, most } = *bound;
            Some(serde_json::json!({
                "name": bound_name(&element.field.id),
                "field": element.field.id,
                "least": least,
                "most": most.map_or_else(|| "nil".to_string(), |most| most.to_string()),
            }))
        })
        .collect();
    let carrier_of = |f: &CodecField| {
        carriers
            .get(&f.id)
            .expect("every field the printer writes holds a number")
    };
    let field_of = |id: &str| {
        plan.model
            .fields
            .iter()
            .find(|f| f.id == id)
            .expect("a derived flag is a flag of a field of the codec")
    };
    let mut body: Vec<String> = Vec::new();
    // Each carrier the encode writes otherwise than the value presents it, folded into a local
    // first: the flags its presence derives, then those a parent sets in place of the value's own.
    let mut folded: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut fold = |carrier: &str, body: &mut Vec<String>| {
        let local = folded_carrier(carrier);
        if folded.insert(carrier.to_string()) {
            body.push(format!("var {local} = {SOURCE}.{carrier}"));
        }
        local
    };
    // Each length a byte string or text says, which a field holds or a flag of a carrier, made
    // from what it measures (docs/adr/0058 §1).
    let mut below: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut derived_fields: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for derived in &plan.derivations {
        let Some(number) = byte_length(&derived.element, &derived.by) else {
            continue;
        };
        let named = derived.element.to_string();
        let measures: Vec<MeasureText> = number
            .measures
            .iter()
            .map(|measure| {
                let Measured::Bytes { field, offset } = &measure.of else {
                    unreachable!("`byte_length` holds a number of bytes and texts alone")
                };
                let content = format!("{SOURCE}.{}", field.id);
                let (raw, present) = if measure.gated {
                    (
                        format!("({content}?.byteCount ?? 0)"),
                        Some(format!("{content} != nil")),
                    )
                } else {
                    (format!("{content}.byteCount"), None)
                };
                // A wire that holds the count less what its arithmetic takes off holds no count
                // below that (docs/adr/0065 §5).
                if let Offset::Less { by, held: false } = offset {
                    if plan
                        .totality
                        .refuses(RefusalKind::LengthOutOfRange, &field.id)
                        && below.insert(field.id.clone())
                    {
                        let test = match &present {
                            Some(there) => format!("{there} && {raw} < {by}"),
                            None => format!("{raw} < {by}"),
                        };
                        body.push(format!(
                            "if {test} {{ {} }}",
                            refusal(plan, RefusalKind::LengthOutOfRange, &field.id)
                        ));
                    }
                }
                let size = match offset {
                    Offset::None => raw,
                    Offset::Less { by, .. } => format!("({raw} - {by})"),
                    Offset::More { by } => format!("({raw} + {by})"),
                };
                MeasureText {
                    size,
                    present,
                    gated: measure.gated,
                }
            })
            .collect();
        let local = derived_local(&derived.element);
        let (size, present) = combine(plan, &named, &local, &measures, &mut body);
        match (&derived.element, &number.into) {
            (ElementId::Field(id), NumberInto::Field { field, narrow, .. }) => {
                let ty = l.type_name(&field.sce_type).into_owned();
                // A number past what the field holds is no frame (docs/adr/0065 §5).
                if *narrow == Narrow::Checked {
                    let most = largest_of(&field.sce_type)
                        .expect("a length that may not fit its field is held in an integer field");
                    let past = format!("{size} > 0x{most:X}");
                    let test = match &present {
                        Some(there) => format!("{there} && {past}"),
                        None => past,
                    };
                    body.push(format!(
                        "if {test} {{ {} }}",
                        refusal(plan, RefusalKind::LengthOutOfRange, &named)
                    ));
                }
                // Where nothing it measures is there, the field is gated beside its content and
                // holds zero, which no write reads.
                let value = match &present {
                    None => format!("{ty}({size})"),
                    Some(there) => format!("{there} ? {ty}({size}) : 0"),
                };
                body.push(format!("let {local}: {ty} = {value}"));
                derived_fields.insert(id.clone());
            }
            (ElementId::Flag { field, flag }, NumberInto::Flag) => {
                let def = field_of(field)
                    .flags
                    .iter()
                    .find(|def| def.name == *flag)
                    .expect("the parser found the flag a length names");
                let width = def.width.max(1);
                let bits = if width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << width) - 1
                };
                if plan.totality.refuses(RefusalKind::LengthOutOfRange, &named) {
                    let past = format!("{size} > 0x{bits:X}");
                    let test = match &present {
                        Some(there) => format!("{there} && {past}"),
                        None => past,
                    };
                    body.push(format!(
                        "if {test} {{ {} }}",
                        refusal(plan, RefusalKind::LengthOutOfRange, &named)
                    ));
                }
                let carrier_type = &carrier_of(field_of(field)).integer;
                let carrier = fold(field, &mut body);
                let mask = bits << def.bit;
                let set = format!(
                    "{carrier} = ({carrier} & ~0x{mask:X}) | (({carrier_type}(truncatingIfNeeded: {size}) & 0x{bits:X}) << {})",
                    def.bit
                );
                // Where every measure may be absent and none is there, the flag keeps the bits
                // the value presents.
                body.push(match &present {
                    Some(there) => format!("if {there} {{ {set} }}"),
                    None => set,
                });
            }
            _ => unreachable!("`byte_length` holds a number into its own kind of element"),
        }
    }
    // A flag set exactly where what it gates is present (clear, for a field gated on its clear
    // state). Fields that each say it must say the same, unless one group holds them all.
    for derived in &plan.derivations {
        let Some((carrier, flag, presence)) = presence_flag(&derived.element, &derived.by) else {
            continue;
        };
        let def = field_of(carrier)
            .flags
            .iter()
            .find(|def| def.name == flag)
            .expect("the parser found the flag a gate reads");
        let says = |(gated, when_clear): &(&CodecField, bool)| {
            let test = if *when_clear { "==" } else { "!=" };
            format!("({SOURCE}.{} {test} nil)", gated.id)
        };
        let (first, rest) = presence
            .of
            .split_first()
            .expect("a presence flag gates a field");
        if !presence.together {
            for other in rest {
                body.push(format!(
                    "if {} != {} {{ {} }}",
                    says(other),
                    says(first),
                    refusal(
                        plan,
                        RefusalKind::InconsistentSource,
                        &derived.element.to_string()
                    )
                ));
            }
        }
        let width = def.width.max(1);
        let mask = if width >= 64 {
            u64::MAX
        } else {
            ((1u64 << width) - 1) << def.bit
        };
        let local = fold(carrier, &mut body);
        body.push(format!(
            "{local} = ({local} & ~0x{mask:X}) | ({} ? 0x{mask:X} : 0)",
            says(first)
        ));
    }
    // The one-bit flags a parent may set in place of the ones the value presents (the flag a chain's
    // entries end on), each a member of `Overrides`: what the parent says is what the wire holds.
    let mut overrides = Vec::new();
    for &(carrier, flag) in &plan.overridable {
        let member = override_member(&carrier.id, &flag.name);
        let local = fold(&carrier.id, &mut body);
        let mask = format!("0x{:X}", 1u64 << flag.bit);
        body.push(format!(
            "if let set = overrides.{member} {{ {local} = set ? ({local} | {mask}) : ({local} & ~{mask}) }}"
        ));
        overrides.push(serde_json::json!({
            "member": member,
            "flag": flag.name,
            "carrier": carrier.id,
        }));
    }
    // A field as the encode reads it: the number it derives, the carrier the flags are folded into,
    // else the source's.
    let read = |f: &CodecField| {
        if derived_fields.contains(&f.id) {
            derived_local(&ElementId::Field(f.id.clone()))
        } else if folded.contains(&f.id) {
            folded_carrier(&f.id)
        } else {
            format!("{SOURCE}.{}", f.id)
        }
    };
    // A gate, as the encode writes the carriers it reads and is handed the inputs it reads.
    let gate_test = |f: &CodecField| {
        let gate = f
            .present_if
            .as_ref()
            .expect("a field held to its gate is gated");
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
                Clause::Input { name, negate } => {
                    format!("{name} {} 0", if *negate { "==" } else { "!=" })
                }
                Clause::Member { .. } => {
                    unreachable!("`unlowered` refuses a gate on a chain's entries")
                }
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    // A field whose gate the encode cannot derive (it reads a flag input the caller hands) is
    // there exactly where its gate holds, or the value has no frame.
    for f in &plan.gate_checks {
        body.push(format!(
            "if ({}) != ({SOURCE}.{} != nil) {{ {} }}",
            gate_test(f),
            f.id,
            refusal(plan, RefusalKind::PresentIfMismatch, &f.id)
        ));
    }
    // A frame laid out by byte is written into zeros as long as its fixed part, and the bytes or
    // text after it appended; one written in turn grows as each field is appended.
    let streamed = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Streamed { .. }));
    let fixed_end = plan
        .program
        .iter()
        .filter_map(|write| match write {
            Write::Whole(f) => BitSpan::of(f, plan.model.default_endian).map(|span| span.end()),
            Write::Byte { at, .. } => Some(at + 1),
            _ => None,
        })
        .max();
    let tails = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Tail { .. }));
    body.push(match (streamed, fixed_end, tails) {
        (true, _, _) | (false, None, true) => "var frame: [UInt8] = []".to_string(),
        (false, Some(end), true) => format!("var frame = [UInt8](repeating: 0, count: {end})"),
        (false, Some(_), false) => {
            "var frame = [UInt8](repeating: 0, count: Self.frameLength)".to_string()
        }
        (false, None, false) => {
            "let frame = [UInt8](repeating: 0, count: Self.frameLength)".to_string()
        }
    });
    for write in &plan.program {
        match *write {
            Write::Whole(f) => body.extend(whole_write(
                l,
                f,
                carrier_of(f),
                &read(f),
                plan.model.default_endian,
            )),
            Write::Byte { at, ref pieces } => {
                for &(f, piece) in pieces {
                    body.push(piece_write(l, carrier_of(f), &read(f), at, piece));
                }
            }
            Write::Streamed {
                field: f, guard, ..
            } => {
                // A byte string or a text appends its bytes; a number, those of its carrier or its
                // chain.
                let append = |value: &str| {
                    if matches!(f.bit_size, BitSize::Tail | BitSize::LengthRef) {
                        format!("{value}.append(to: &frame)")
                    } else {
                        super::streamed_append(
                            l,
                            f,
                            carrier_of(f),
                            value,
                            plan.model.default_endian,
                            None,
                        )
                    }
                };
                match guard {
                    Guard::Always => body.push(append(&read(f))),
                    // Where the value holds it.
                    Guard::Present => {
                        body.push(format!("if let {HELD} = {SOURCE}.{} {{", f.id));
                        body.push(format!("    {}", append(HELD)));
                        body.push("}".to_string());
                    }
                    // Derived whole and gated: where its gate holds, as the carriers the encode
                    // writes say.
                    Guard::Gate => {
                        body.push(format!("if {} {{", gate_test(f)));
                        body.push(format!("    {}", append(&read(f))));
                        body.push("}".to_string());
                    }
                }
            }
            // The bytes or text after a frame laid out by byte, after zeros up to the field's
            // offset where the layout aligns its fields.
            Write::Tail { field: f, pad } => {
                if pad {
                    body.push(format!(
                        "while frame.count < {} {{ frame.append(0) }}",
                        f.byte_offset
                    ));
                }
                body.push(format!("{SOURCE}.{}.append(to: &frame)", f.id));
            }
            Write::Variant => {
                unreachable!("`unlowered` refuses every other write before the printer runs")
            }
        }
    }
    plan.totality.assert_all_written(&plan.model.name);
    serde_json::json!({
        "protocol": source_protocol(&plan.model.name),
        "requirements": requirements,
        "given": plan.given.iter().map(|input| input.name.clone()).collect::<Vec<_>>(),
        "refuses": !plan.totality.is_total(),
        "bounds": bounds,
        "body": body,
        "overrides": overrides,
    })
}

/// The member of a printed codec's `Overrides` that sets the flag `flag` of the carrier `carrier`:
/// the two names in lower camel case, so two carriers' flags of one name are two members.
pub(crate) fn override_member(carrier: &str, flag: &str) -> String {
    filters::to_camel_case(format!("{carrier}_{flag}"))
}

/// The local a printed encode folds the flags it derives or a parent sets into the carrier
/// `carrier` in. A field is read through the source and never bare, so the local hides no field.
fn folded_carrier(carrier: &str) -> String {
    format!("{carrier}_folded")
}
