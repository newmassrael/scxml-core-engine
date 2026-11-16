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
//! so no codec has two encodes. The forms printed are a positional frame laid out by byte, of
//! numbers each always there (an integer, a flag set presented whole, a value of an imported enum),
//! none derived, with no flag input, no flag a parent sets, and an encode that refuses nothing.

use super::*;
use crate::forge::encode_plan::{CodecPlan, EncodePlan, Presence, Value, Write};

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
    if let Some((carrier, flag)) = plan.overridable.first() {
        return Some(format!(
            "the flag `{}` of `{}`, which a parent may set",
            flag.name, carrier.id
        ));
    }
    if !plan.totality.is_total() {
        return Some("an encode that refuses a value".to_string());
    }
    plan.program.iter().find_map(|write| match write {
        Write::Whole(_) | Write::Byte { .. } => None,
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
    let mut writes = Vec::new();
    for write in &plan.program {
        match *write {
            Write::Whole(f) => writes.extend(whole_write(
                l,
                f,
                carrier_of(f),
                SOURCE,
                plan.model.default_endian,
            )),
            Write::Byte { at, ref pieces } => {
                for &(f, piece) in pieces {
                    writes.push(piece_write(l, f, carrier_of(f), SOURCE, at, piece));
                }
            }
            Write::Tail { .. } | Write::Streamed { .. } | Write::Variant => {
                unreachable!("`unlowered` refuses every other write before the printer runs")
            }
        }
    }
    serde_json::json!({
        "protocol": source_protocol(&plan.model.name),
        "requirements": requirements,
        "writes": writes,
    })
}
