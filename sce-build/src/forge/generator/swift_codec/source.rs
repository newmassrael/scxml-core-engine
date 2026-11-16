// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift printer of a codec's encode plan (docs/adr/0081 step 5).
//!
//! A printed codec presents its elements through a protocol, `{Codec}Source`, each element a
//! read-only property of the type the value holds it as, and its encode is
//! `{Codec}.encodeSource(_:into:)` over any type that conforms: the host's own type is encoded with
//! no copy into the generated value, as a Rust value is through its source trait (docs/adr/0058
//! §1). The generated value conforms, and its `encode(into:)` is `encodeSource` of itself. Every
//! decision of the encode is the plan's; this module spells it and decides none.
//!
//! The printer lowers the plan's forms one at a time (docs/adr/0081 §6). A codec whose plan uses a
//! form it does not lower yet keeps the encode of its template, and [`unlowered`] names that form,
//! so no codec has two encodes. The forms printed are a positional frame laid out by byte or
//! written in turn (a fixed number's bytes, a variable-length integer's chain), of numbers each
//! always there (an integer, a flag set presented whole, a value of an imported enum),
//! none derived, with no flag input and an encode that refuses nothing; a one-bit flag a parent may
//! set in place of the one the value presents (`EncodePlan::overridable`) is a member of the
//! codec's `Overrides`, which the encode is handed and folds into its carrier.

use super::*;
use crate::forge::encode_plan::{CodecPlan, EncodePlan, Guard, Presence, Value, Write};

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
            Presence::Always => None,
            Presence::Gated => Some("an element a gate decides".to_string()),
            Presence::Stated => {
                Some("a number stated where what it measures is absent".to_string())
            }
        });
        if let Some(what) = what {
            return Some(format!("field '{id}': {what}"));
        }
    }
    if let Some(derived) = plan.derivations.first() {
        return Some(format!("`{}`, which the encode derives", derived.element));
    }
    if let Some(input) = plan
        .given
        .first()
        .copied()
        .or_else(|| plan.said.first().map(|said| said.input))
    {
        return Some(format!("the flag input `{}`", input.name));
    }
    if !plan.totality.is_total() {
        return Some("an encode that refuses a value".to_string());
    }
    plan.program.iter().find_map(|write| match write {
        Write::Whole(_) | Write::Byte { .. } => None,
        // A number in turn, always there: the bytes of its carrier or its chain.
        Write::Streamed {
            field,
            guard: Guard::Always,
            list: None,
        } if matches!(field.bit_size, BitSize::Fixed { .. } | BitSize::Vle { .. }) => None,
        Write::Tail { field, .. } | Write::Streamed { field, .. } => {
            Some(format!("field '{}': a field written in turn", field.id))
        }
        Write::Variant => Some("a variant's arm".to_string()),
    })
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

/// What the template writes for the codec of `plan`, which [`unlowered`] has found nothing to
/// refuse in: the protocol and its requirements, one read-only property per element in
/// declaration order, and the statements of `encodeSource` that lay the frame out, one write of the
/// plan's program after another.
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
                "type": l.type_name(&element.field.sce_type).into_owned(),
            })
        })
        .collect();
    let carrier_of = |f: &CodecField| {
        carriers
            .get(&f.id)
            .expect("every field the printer writes holds a number")
    };
    // The one-bit flags a parent may set in place of the ones the value presents (the flag a chain's
    // entries end on), each a member of `Overrides`, and the carriers they are folded into before a
    // byte is written: what the parent says is what the wire holds.
    let mut overrides = Vec::new();
    let mut folds: Vec<String> = Vec::new();
    let mut folded: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for &(carrier, flag) in &plan.overridable {
        let member = override_member(&carrier.id, &flag.name);
        let local = folded_carrier(&carrier.id);
        if folded.insert(carrier.id.as_str()) {
            folds.push(format!("var {local} = {SOURCE}.{}", carrier.id));
        }
        let mask = format!("0x{:X}", 1u64 << flag.bit);
        folds.push(format!(
            "if let set = overrides.{member} {{ {local} = set ? ({local} | {mask}) : ({local} & ~{mask}) }}"
        ));
        overrides.push(serde_json::json!({
            "member": member,
            "flag": flag.name,
            "carrier": carrier.id,
        }));
    }
    // A field as the encode reads it: the carrier the overrides are folded into, else the source's.
    let read = |f: &CodecField| {
        if folded.contains(f.id.as_str()) {
            folded_carrier(&f.id)
        } else {
            format!("{SOURCE}.{}", f.id)
        }
    };
    let mut writes = folds;
    for write in &plan.program {
        match *write {
            Write::Whole(f) => writes.extend(whole_write(
                l,
                f,
                carrier_of(f),
                &read(f),
                plan.model.default_endian,
            )),
            Write::Byte { at, ref pieces } => {
                for &(f, piece) in pieces {
                    writes.push(piece_write(l, carrier_of(f), &read(f), at, piece));
                }
            }
            Write::Streamed { field: f, .. } => writes.push(super::streamed_append(
                l,
                f,
                carrier_of(f),
                &read(f),
                plan.model.default_endian,
                None,
            )),
            Write::Tail { .. } | Write::Variant => {
                unreachable!("`unlowered` refuses every other write before the printer runs")
            }
        }
    }
    // A frame laid out by byte is written into zeros of its length; one written in turn grows as
    // each field is appended.
    let streamed = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Streamed { .. } | Write::Tail { .. }));
    let frame = if streamed {
        "var frame: [UInt8] = []"
    } else if writes.is_empty() {
        "let frame = [UInt8](repeating: 0, count: Self.frameLength)"
    } else {
        "var frame = [UInt8](repeating: 0, count: Self.frameLength)"
    };
    serde_json::json!({
        "protocol": source_protocol(&plan.model.name),
        "requirements": requirements,
        "frame": frame,
        "writes": writes,
        "overrides": overrides,
    })
}

/// The member of a printed codec's `Overrides` that sets the flag `flag` of the carrier `carrier`:
/// the two names in lower camel case, so two carriers' flags of one name are two members.
pub(crate) fn override_member(carrier: &str, flag: &str) -> String {
    filters::to_camel_case(format!("{carrier}_{flag}"))
}

/// The local a printed encode folds the flags a parent sets into the carrier `carrier` in. A field is
/// read through the source and never bare, so the local hides no field.
fn folded_carrier(carrier: &str) -> String {
    format!("{carrier}_folded")
}
