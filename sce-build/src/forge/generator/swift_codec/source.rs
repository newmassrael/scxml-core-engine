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
//! by a gate, whose flag the encode derives from the presence of what it gates. A flag input the
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
    Clause, CodecPlan, Derive, EncodePlan, Guard, Presence, PresenceFlag, Value, Write,
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
            Value::Bytes(_) => Some("a byte string".to_string()),
            Value::Text(_) => Some("a text".to_string()),
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
    if let Some(derived) = plan
        .derivations
        .iter()
        .find(|derived| presence_flag(&derived.element, &derived.by).is_none())
    {
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
    plan.program.iter().find_map(|write| match write {
        Write::Whole(_) | Write::Byte { .. } => None,
        // A number in turn, always there or where the value holds it: the bytes of its carrier or
        // its chain.
        Write::Streamed {
            field,
            guard: Guard::Always | Guard::Present,
            list: None,
        } if matches!(field.bit_size, BitSize::Fixed { .. } | BitSize::Vle { .. }) => None,
        Write::Tail { field, .. } | Write::Streamed { field, .. } => {
            Some(format!("field '{}': a field written in turn", field.id))
        }
        Write::Variant => Some("a variant's arm".to_string()),
    })
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
/// gate the encode cannot derive, and fields that each say one flag and may disagree.
fn writes_refusal(plan: &EncodePlan<'_>, kind: RefusalKind, element: &str) -> bool {
    match kind {
        RefusalKind::PresentIfMismatch => plan.gate_checks.iter().any(|f| f.id == element),
        RefusalKind::InconsistentSource => plan.derivations.iter().any(|derived| {
            presence_flag(&derived.element, &derived.by).is_some()
                && derived.element.to_string() == element
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
            let ty = l.type_name(&element.field.sce_type).into_owned();
            let optional = if element.presence.optional() { "?" } else { "" };
            serde_json::json!({
                "name": element.field.id,
                "type": format!("{ty}{optional}"),
            })
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
    // A flag set exactly where what it gates is present (clear, for a field gated on its clear
    // state). Fields that each say it must say the same, unless one group holds them all.
    for derived in &plan.derivations {
        let Some((carrier, flag, presence)) = presence_flag(&derived.element, &derived.by) else {
            unreachable!("`unlowered` refuses every other derivation before the printer runs")
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
    // A field as the encode reads it: the carrier the flags are folded into, else the source's.
    let read = |f: &CodecField| {
        if folded.contains(&f.id) {
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
    // A frame laid out by byte is written into zeros of its length; one written in turn grows as
    // each field is appended.
    let streamed = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Streamed { .. } | Write::Tail { .. }));
    let fixed_writes = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Whole(_) | Write::Byte { .. }));
    body.push(
        if streamed {
            "var frame: [UInt8] = []"
        } else if fixed_writes {
            "var frame = [UInt8](repeating: 0, count: Self.frameLength)"
        } else {
            "let frame = [UInt8](repeating: 0, count: Self.frameLength)"
        }
        .to_string(),
    );
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
                let append = |value: &str| {
                    super::streamed_append(
                        l,
                        f,
                        carrier_of(f),
                        value,
                        plan.model.default_endian,
                        None,
                    )
                };
                match guard {
                    Guard::Always => body.push(append(&read(f))),
                    // Where the value holds it.
                    Guard::Present => {
                        body.push(format!("if let {HELD} = {SOURCE}.{} {{", f.id));
                        body.push(format!("    {}", append(HELD)));
                        body.push("}".to_string());
                    }
                    Guard::Gate => {
                        unreachable!("`unlowered` refuses a field derived whole and gated")
                    }
                }
            }
            Write::Tail { .. } | Write::Variant => {
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
