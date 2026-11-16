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
//! A flag input the value says by the field gated on it alone, or by the body it hands it on to
//! (docs/adr/0065 §6 R4a), is no parameter of the encode: the encode reads it off the value, and so
//! does a parent or a log through the codec's `input{Name}(_:)`; the decode is still handed it.
//! A one-bit flag a parent may set in place of the one the value presents
//! (`EncodePlan::overridable`) is a member of the codec's `Overrides`, which the encode is handed
//! and folds into its carrier.
//!
//! An embedded body of a codec the Swift printer prints is presented as a source of that codec, an
//! associated type of the protocol the generated value binds to the body's own value, and is
//! encoded into bytes of its own once, before the frame is written: its encoded length, where a
//! field holds it, is the count of those bytes (docs/adr/0058 §3, measured once), and a flag bound
//! to an input the body says is read off the body through its codec's `input{Name}(_:)`. The body
//! is handed each input it does not say from the carrier as the encode writes it or from an input
//! of the parent's own, and an input it says that the parent hands it from elsewhere is a check
//! (`presentIfMismatch`).
//!
//! A list or a chain of entries of a codec the Swift printer prints is held in the runtime's
//! `BoundedList` over a count bound the codec declares (docs/adr/0065 §6 R2), each entry a source of
//! the entry codec, an associated type of the protocol as a body's is. Its count, where a field or
//! a flag holds it, is derived from the entries (docs/adr/0081 §3). The walk writes each entry by
//! the entry codec's encode, hands each the inputs a chain supplies from the entry before, and on
//! a chain that ends on its entries' flag sets that flag on every entry but the last through the
//! entry codec's `Overrides` (§6 R7), whatever the entry presents; an input every entry says that
//! the parent hands them from elsewhere is held to it entry by entry before the first byte.
//!
//! A variant every arm of which is a codec the Swift printer prints is presented as the generic
//! enum `{Codec}Arms` over a source of each arm's codec, an associated type of the protocol each,
//! which the generated value binds to the arm codecs' own values. The enum's `tag` is the one table
//! of the arms' tags: the encode derives the tag field or flag from it, and a parent the bits a
//! body's arm fills. The arm is written by its codec's encode, handed what its import binds; an
//! input every arm says, or some arm, is read off the arm the value holds (docs/adr/0065 §6 R4d,
//! R4e), and one the parent hands an arm that says it is held to it before the first byte.

use super::*;
use crate::forge::codec_roles::{ElementId, InputSource};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Against, Bound, CheckSite, Clause, CodecPlan, Derive, Derived, Element, EncodePlan,
    EntryId, Guard, HandedFrom, InputCheck, ListWrite, Measured, Narrow, Number, NumberInto,
    Offset, Presence, PresenceFlag, Value, Write,
};

/// The Swift struct of each codec a codec holds as a body, by the id of the field that holds it.
pub(crate) type HeldTypes = std::collections::HashMap<String, String>;

/// The first form of `plan` the Swift printer does not lower yet, described for a reader; `None`
/// where it prints the codec whole. `imports` says which codec each import is and whether the
/// Swift printer prints it.
pub(crate) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    match plan {
        CodecPlan::Positional(plan) => unlowered_positional(plan, imports),
        CodecPlan::Cbor(_) => Some("a CBOR map".to_string()),
        CodecPlan::ContentLine(_) => Some("a component of content lines".to_string()),
    }
}

/// The import `alias` of a codec.
fn import_of<'i>(imports: &'i [ImportContext], alias: &str) -> Option<&'i ImportContext> {
    imports.iter().find(|i| i.alias == alias)
}

/// Whether the codec imported as `alias` says its input `input` in some of its values only.
fn says_partly(imports: &[ImportContext], alias: &str, input: &str) -> bool {
    import_of(imports, alias).is_some_and(|i| i.codec_partly_says_inputs.iter().any(|s| s == input))
}

/// How the encode walks the list or chain `id`, where the program writes one.
fn walk_of<'p>(plan: &'p EncodePlan<'_>, id: &str) -> Option<&'p ListWrite> {
    plan.program.iter().find_map(|write| match write {
        Write::Streamed {
            field,
            list: Some(walk),
            ..
        } if field.id == id => Some(walk),
        _ => None,
    })
}

/// Whether every element of the codec that holds the codec imported as `alias` is a chain that
/// supplies its entries the input `input` from the entry before (`ListWrite::previous`), which the
/// parent then binds nothing to.
fn chain_supplies(plan: &EncodePlan<'_>, alias: &str, input: &str) -> bool {
    let mut holders = plan
        .elements
        .iter()
        .filter(|element| match &element.value {
            Value::Body { codec } | Value::List { codec, .. } => *codec == alias,
            _ => false,
        })
        .peekable();
    holders.peek().is_some()
        && holders.all(|element| {
            walk_of(plan, &element.field.id)
                .is_some_and(|walk| walk.previous.iter().any(|(bound, _)| bound == input))
        })
}

/// The identifier of `entry`, an entry of the chain `chain` of the codec of `plan`, as a `UInt64`
/// (`EncodePlan::entry_ids`): the bits of its carrier, or its field less the flags the chain leaves
/// out, read off the source the chain holds it as.
fn entry_identifier(plan: &EncodePlan<'_>, chain: &str, entry: &str) -> String {
    match plan
        .entry_ids
        .get(chain)
        .expect("a chain whose entries a gate, a bind or a check tells apart declares entry-id")
    {
        EntryId::Flag { carrier, bit, mask } => {
            format!("((chainIdentifier({entry}.{carrier}) >> {bit}) & 0x{mask:X})")
        }
        EntryId::Whole {
            field,
            mask: Some(mask),
        } => format!("(chainIdentifier({entry}.{field}) & 0x{mask:X})"),
        EntryId::Whole { field, mask: None } => format!("chainIdentifier({entry}.{field})"),
    }
}

/// Whether the codec of `plan` hands its flag input `name`, which its value says, to a codec it
/// holds, or holds a codec it holds to it, so that the encode reads it off the value.
fn handed_on(plan: &EncodePlan<'_>, name: &str) -> bool {
    plan.handed.values().flatten().any(
        |handed| matches!(&handed.from, Some(HandedFrom::Input { name: from }) if from == name),
    ) || plan
        .input_checks
        .iter()
        .any(|check| matches!(&check.against, Against::Input { name: from } if from == name))
}

/// [`unlowered`] of a positional codec's plan.
pub(crate) fn unlowered_positional(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
) -> Option<String> {
    // An arm is written by its own codec's printed encode, so the Swift printer prints the codec of
    // every arm first.
    if let Some(arm) = plan
        .arms
        .iter()
        .find(|arm| !import_of(imports, arm.codec).is_some_and(|import| import.swift_printed))
    {
        return Some(format!(
            "the arm `{}` of the variant, which the Swift printer does not print",
            arm.codec
        ));
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
            // A body is written by its own codec's printed encode, so the Swift printer prints the
            // codec it holds first.
            Value::Body { codec } => match import_of(imports, codec) {
                Some(import) if import.swift_printed => None,
                _ => Some(format!(
                    "an embedded body of `{codec}`, which the Swift printer does not print"
                )),
            },
            // A list's entries are written by their own codec's printed encode, so the Swift
            // printer prints the codec they are first. A list held to its law (docs/adr/0065 §6
            // R8) waits for the form that walks it through its law.
            Value::List { codec, law, .. } => match import_of(imports, codec) {
                Some(import) if import.swift_printed && law.is_none() => None,
                Some(import) if import.swift_printed => {
                    Some("a list held to the law of the inputs its entries say".to_string())
                }
                _ => Some(format!(
                    "a list of `{codec}`, which the Swift printer does not print"
                )),
            },
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
    // A body or an entry is handed each input it does not say from the parent's carrier or its
    // own input, and an entry of a chain, besides, an input the chain supplies from the entry
    // before (`ListWrite::previous`).
    if let Some((alias, handed)) = plan.handed.iter().find_map(|(alias, handed)| {
        Some((
            alias,
            handed
                .iter()
                .find(|h| h.from.is_none() && !chain_supplies(plan, alias, &h.input))?,
        ))
    }) {
        return Some(format!(
            "the input `{}` of `{alias}`, which no declaration binds",
            handed.input
        ));
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
    // The flag a chain's entries end on is set by the walk on every entry but the last, through
    // the entry codec's `Overrides` (docs/adr/0065 §6 R7).
    let continued = |derived: &&Derived<'_>| match (&derived.element, &derived.by) {
        (ElementId::Flag { field, .. }, Derive::ChainContinuation) => {
            walk_of(plan, field).is_some_and(|walk| walk.ends.is_some())
        }
        _ => false,
    };
    // The tag of the arm the value holds, and the bits of a carrier the arm of an embedded body's
    // variant fills, are read off the arm through the arms enum's `tag`.
    let tagged = |derived: &&Derived<'_>| {
        matches!(
            (&derived.element, &derived.by),
            (
                ElementId::Field(_) | ElementId::Flag { .. },
                Derive::VariantTag
            ) | (ElementId::Flag { .. }, Derive::ParentTag(_))
        )
    };
    if let Some(derived) = plan.derivations.iter().find(|derived| {
        let presence = presence_flag(&derived.element, &derived.by)
            .is_some_and(|(_, _, presence)| readable(presence));
        !presence
            && printed_number(&derived.element, &derived.by).is_none()
            && !continued(derived)
            && !tagged(derived)
    }) {
        return Some(format!("`{}`, which the encode derives", derived.element));
    }
    // A body that says an input in some of its values only says nothing in the others.
    if let Some(derived) = plan.derivations.iter().find(|derived| {
        printed_number(&derived.element, &derived.by).is_some_and(|number| {
            number.measures.iter().any(|measure| match &measure.of {
                Measured::Says { codec, input, .. } => says_partly(imports, codec, input),
                Measured::ArmSays { arms } => arms
                    .iter()
                    .any(|arm| says_partly(imports, &arm.codec, &arm.input)),
                _ => false,
            })
        })
    }) {
        return Some(format!(
            "`{}`, bound to an input its body says in some of its values only",
            derived.element
        ));
    }
    // An input the value says is read off the field gated on it alone, off the body always there
    // that it hands it on to and that says it in every value, or off the arm the value holds; one
    // a group or a list says waits for the form that holds it. One some arms say only is said in
    // some values only, and no encode reads it (docs/adr/0065 §6 R4e): only a parent asks for it.
    if let Some(said) = plan
        .said
        .iter()
        .find(|said| match &plan.roles.inputs[&said.input.name] {
            InputSource::Gate { .. }
            | InputSource::Forward { partly: false, .. }
            | InputSource::EveryArm { .. } => false,
            InputSource::SomeArms { .. } => said.read || handed_on(plan, &said.input.name),
            _ => true,
        })
    {
        return Some(format!(
            "the flag input `{}`, which the value says by what it holds",
            said.input.name
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
        // A number or a byte string or text in turn, always there or where the value holds it, a
        // number the encode derives whole where its gate holds, and a body where the value holds
        // it: the bytes of its carrier, its chain, its bytes, or the body's own frame.
        Write::Streamed {
            field, list: None, ..
        } if matches!(
            field.bit_size,
            BitSize::Fixed { .. }
                | BitSize::Vle { .. }
                | BitSize::Tail
                | BitSize::LengthRef
                | BitSize::Embed
        ) =>
        {
            None
        }
        // A list or a chain where the value holds it, each entry written in turn by its codec's
        // printed encode, handed what the entry before supplies and, on a chain that ends on its
        // entries' flag, the flag set on every entry but the last. A walk through a law or of
        // slots waits for the group that holds it.
        Write::Streamed {
            list: Some(walk),
            guard: Guard::Always | Guard::Present,
            ..
        } if !walk.lawful && walk.slots.is_none() => None,
        // The bytes or text after a frame laid out by byte, never gated (a gate makes a frame one
        // written in turn).
        Write::Tail { .. } => None,
        Write::Streamed { field, .. } => {
            Some(format!("field '{}': a field written in turn", field.id))
        }
        // The arm the value holds, written by its codec's printed encode.
        Write::Variant => None,
    })
}

/// The number `element` is, where `by` derives it as one the printer writes: a number of the byte
/// strings and texts it measures, of the entries of the lists it counts, of the inputs the bodies
/// it measures say, or of the bytes the bodies it measures encode to, into a field of its own
/// (zero where a gate leaves every one of them out, beside which the field is gated too) or, but
/// for an encoded length, into a flag of a carrier.
fn printed_number<'d, 'm>(element: &'d ElementId, by: &'d Derive<'m>) -> Option<&'d Number<'m>> {
    let Derive::Number(number) = by else {
        return None;
    };
    let encoded = |measured: &Measured<'_>| matches!(measured, Measured::Encoded { .. });
    let measures = number.measures.iter().all(|measure| {
        matches!(
            measure.of,
            Measured::Bytes { .. }
                | Measured::Entries { .. }
                | Measured::Says { .. }
                | Measured::ArmSays { .. }
                | Measured::Encoded { .. }
        )
    });
    // An encoded length is counted once the bodies are encoded, after every carrier is folded, so
    // it goes into a field and measures nothing else.
    let counted = !number.measures.iter().any(|m| encoded(&m.of))
        || (matches!(element, ElementId::Field(_))
            && number.measures.iter().all(|m| encoded(&m.of)));
    let into = match (element, &number.into) {
        (ElementId::Field(_), NumberInto::Field { absent, .. }) => {
            matches!(absent, Absent::Never | Absent::Zero)
        }
        (ElementId::Flag { .. }, NumberInto::Flag) => true,
        _ => false,
    };
    (measures && counted && into).then_some(number)
}

/// Whether `number` is the encoded length of bodies, counted once they are encoded.
fn is_encoded_length(number: &Number<'_>) -> bool {
    number
        .measures
        .iter()
        .any(|measure| matches!(measure.of, Measured::Encoded { .. }))
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
/// gate the encode cannot derive, or an input a body or the entries of a list say held to what the
/// parent hands them; fields that each say one flag, or measures that each say one number, and may
/// disagree; a length past the field that holds it where the plan checks it, past the bits of the
/// flag that holds it, or below what its arithmetic takes off; and the refusal of a body or an
/// entry whose codec refuses, which its encode throws.
fn writes_refusal(plan: &EncodePlan<'_>, kind: RefusalKind, element: &str) -> bool {
    let derived_at =
        |derived: &&crate::forge::encode_plan::Derived<'_>| derived.element.to_string() == element;
    match kind {
        RefusalKind::PresentIfMismatch => {
            plan.gate_checks.iter().any(|f| f.id == element)
                || plan.input_checks.iter().any(|check| check.at == element)
        }
        RefusalKind::InconsistentSource => {
            plan.derivations.iter().filter(derived_at).any(|derived| {
                presence_flag(&derived.element, &derived.by).is_some()
                    || printed_number(&derived.element, &derived.by)
                        .is_some_and(|number| number.measures.len() > 1)
            })
        }
        RefusalKind::LengthOutOfRange => plan.derivations.iter().any(|derived| {
            let Some(number) = printed_number(&derived.element, &derived.by) else {
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
        RefusalKind::Child => {
            plan.elements.iter().any(|e| {
                e.field.id == element && matches!(e.value, Value::Body { .. } | Value::List { .. })
            }) || plan.arms.iter().any(|arm| arm.codec == element)
        }
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

/// The static function of a printed codec that reads the flag input `input` off a source of it
/// (docs/adr/0065 §6 R4a): `input` and the input's name in upper camel case.
pub(crate) fn said_reader(input: &str) -> String {
    format!("input{}", filters::to_pascal_case(input.to_string()))
}

/// The associated type of a printed codec's protocol that the body `field` is a source of, which
/// the generated value binds to the body's own value: the field's name in upper camel case and
/// `_Source`. The name of a codec's struct or protocol is in upper camel case and holds no `_`, so
/// the associated type hides none of them, the body's own protocol among them.
fn body_source_type(field: &str) -> String {
    format!("{}_Source", filters::to_pascal_case(field.to_string()))
}

/// The `UInt8` the flag input `input` that the value says holds, read off `source`, a source of
/// the codec of `plan`: set exactly where the field gated on it alone is present (absent, for a
/// field gated on its clear state), or what the body always there that it hands it on to says,
/// read through the body's codec. [`unlowered`] refuses every other way a value says one.
fn said_input(plan: &EncodePlan<'_>, held: &HeldTypes, input: &str, source: &str) -> String {
    match &plan.roles.inputs[input] {
        InputSource::Gate { field, when_clear } => {
            let test = if *when_clear { "==" } else { "!=" };
            format!("{source}.{field} {test} nil ? 1 : 0")
        }
        InputSource::Forward {
            embed,
            input,
            partly: false,
            ..
        } => {
            let child = held
                .get(embed)
                .expect("an input forwarded from a body is said by a body the codec holds");
            format!("{child}.{}({source}.{embed})", said_reader(input))
        }
        // What the arm the value holds says, read through the codec's own reader of it, which
        // [`said_lines`] writes as a switch over the arms.
        InputSource::EveryArm { .. } => format!("Self.{}({source})", said_reader(input)),
        InputSource::Forward { partly: true, .. }
        | InputSource::Group
        | InputSource::SomeArms { .. }
        | InputSource::List { .. } => {
            unreachable!("`unlowered` refuses an input a group or a list says, or one some arms say that the encode reads")
        }
    }
}

/// The statements of the function that reads the flag input `input` the value says off a source
/// of the codec of `plan` (docs/adr/0065 §6 R4a): what the arm the value holds says, read through
/// the arm's codec, and, where some arms do not read it, nothing for those (§6 R4e); or the one
/// expression [`said_input`] reads it by. `arms` are the variant's arms as the printer spells them.
fn said_lines(
    plan: &EncodePlan<'_>,
    held: &HeldTypes,
    arms: &[ArmText],
    input: &str,
) -> Vec<String> {
    let (bound, every) = match &plan.roles.inputs[input] {
        InputSource::EveryArm { arms } => (arms, true),
        InputSource::SomeArms { arms } => (arms, false),
        _ => return vec![said_input(plan, held, input, SOURCE)],
    };
    let mut lines = vec![format!("switch {SOURCE}.{} {{", super::VARIANT_FIELD)];
    for (arm, text) in plan.arms.iter().zip(arms) {
        let line = match bound.get(arm.codec) {
            Some(said) => format!(
                "case {}: return {}.{}(arm)",
                text.pattern("arm"),
                text.child,
                said_reader(said)
            ),
            None if every => unreachable!("every arm says an input every arm says"),
            None => format!("case {}: return nil", text.bare()),
        };
        lines.extend(text.guarded(vec![line]));
    }
    lines.push("}".to_string());
    lines
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

/// The associated type of a printed codec's protocol that each entry of the list or chain `field`
/// is a source of, which the generated value binds to the entry codec's own value: the field's name
/// in upper camel case and `_Entry`, which hides no codec's type, as [`body_source_type`] says.
fn entry_source_type(field: &str) -> String {
    format!("{}_Entry", filters::to_pascal_case(field.to_string()))
}

/// The Swift type a printed codec named `struct_name` holds the list or chain `element` as, each
/// entry an `entry`, within the count bound its own type declares (docs/adr/0065 §6 R2, R7);
/// `None` for another element.
pub(super) fn list_type(struct_name: &str, element: &Element<'_>, entry: &str) -> Option<String> {
    matches!(element.value, Value::List { .. }).then(|| {
        format!(
            "BoundedList<{entry}, {struct_name}.{}>",
            bound_name(&element.field.id)
        )
    })
}

/// The Swift type the protocol of `plan` presents `element` as: its number, its bounded byte string
/// or text, the associated type its body is a source of, or its list of the associated type each
/// entry is a source of, optional where a gate decides it.
fn presented_type(l: &LangCtx, plan: &EncodePlan<'_>, element: &Element<'_>) -> String {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    let ty = match &element.value {
        Value::Body { .. } => body_source_type(&element.field.id),
        Value::List { .. } => {
            list_type(&struct_name, element, &entry_source_type(&element.field.id))
                .expect("a list is held in a list type")
        }
        _ => bounded_type(&struct_name, element)
            .unwrap_or_else(|| l.type_name(&element.field.sce_type).into_owned()),
    };
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

/// The local the encode holds the bytes the body `field` encodes to in, `nil` where a gate leaves
/// the body out. A field is read through the source and never bare, so the local hides no field.
fn body_bytes(field: &str) -> String {
    format!("{field}_bytes")
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

/// The derived number `size` (there where `present` holds, if it may not be), held in the local of
/// the field `id` it goes `into`, of that field's type: refused where it may not fit (docs/adr/0065
/// §5), and zero where nothing it measures is there, beside which the field is gated and no write
/// reads it.
fn field_number(
    l: &LangCtx,
    plan: &EncodePlan<'_>,
    id: &str,
    into: &NumberInto<'_>,
    size: &str,
    present: Option<&str>,
    body: &mut Vec<String>,
) {
    let NumberInto::Field { field, narrow, .. } = into else {
        unreachable!("a number held in a field goes into that field")
    };
    let ty = l.type_name(&field.sce_type).into_owned();
    if *narrow == Narrow::Checked {
        let most = largest_of(&field.sce_type)
            .expect("a length that may not fit its field is held in an integer field");
        let past = format!("{size} > 0x{most:X}");
        let test = match present {
            Some(there) => format!("{there} && {past}"),
            None => past,
        };
        body.push(format!(
            "if {test} {{ {} }}",
            refusal(plan, RefusalKind::LengthOutOfRange, id)
        ));
    }
    let value = match present {
        None => format!("{ty}({size})"),
        Some(there) => format!("{there} ? {ty}({size}) : 0"),
    };
    body.push(format!(
        "let {}: {ty} = {value}",
        derived_local(&ElementId::Field(id.to_string()))
    ));
}

/// What the template writes for the codec of `plan`, which [`unlowered`] has found nothing to
/// refuse in: the protocol and its requirements, one read-only property per element in
/// declaration order, the associated type each body is a source of, the flag inputs the encode is
/// handed, and the statements of `encodeSource`: each carrier with the flags the encode derives or
/// a parent sets folded in, each body encoded into bytes of its own, the checks the value is held
/// to, and the writes of the plan's program, one after another, into a frame written in one write.
/// `held` is the Swift struct of each body by its field, and `arm_types` that of each arm of the
/// variant, in the plan's order.
pub(super) fn context(
    l: &LangCtx,
    plan: &EncodePlan<'_>,
    carriers: &std::collections::HashMap<String, Carrier>,
    held: &HeldTypes,
    arm_types: &[String],
) -> serde_json::Value {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    let arms = arm_texts(plan, arm_types);
    let mut requirements: Vec<serde_json::Value> = plan
        .elements
        .iter()
        .map(|element| {
            serde_json::json!({
                "name": element.field.id,
                "type": presented_type(l, plan, element),
            })
        })
        .collect();
    // The variant, last of the elements, as the arms enum of the sources each arm's body is.
    if !arms.is_empty() {
        requirements.push(serde_json::json!({
            "name": super::VARIANT_FIELD,
            "type": format!(
                "{}<{}>",
                arms_enum(&struct_name),
                arms.iter().map(|a| a.assoc.as_str()).collect::<Vec<_>>().join(", ")
            ),
        }));
    }
    let held_of = |f: &CodecField| {
        held.get(&f.id)
            .expect("every body the printer writes holds a codec of the module")
    };
    // The source each body is presented as: an associated type the protocol requires to be a
    // source of the body's codec, which the generated value binds to the body's own value.
    let bodies: Vec<serde_json::Value> = plan
        .elements
        .iter()
        .filter(|element| matches!(element.value, Value::Body { .. }))
        .map(|element| {
            let child = held_of(element.field);
            serde_json::json!({
                "name": body_source_type(&element.field.id),
                "field": element.field.id,
                "protocol": format!("{child}Source"),
                "type": child,
            })
        })
        .collect();
    // The source each entry of a list or a chain is presented as, as a body is.
    let entries: Vec<serde_json::Value> = plan
        .elements
        .iter()
        .filter(|element| matches!(element.value, Value::List { .. }))
        .map(|element| {
            let child = held_of(element.field);
            serde_json::json!({
                "name": entry_source_type(&element.field.id),
                "field": element.field.id,
                "protocol": format!("{child}Source"),
                "type": child,
            })
        })
        .collect();
    // The bound each byte string and text is held to, a type of the codec's own (docs/adr/0065 §6
    // R1), and the count bound each list or chain is held to (R2, R7): the value's type of the
    // element names it, so a value made for one is no value of another, and a byte bound is no
    // count bound.
    let bounds: Vec<serde_json::Value> = plan
        .elements
        .iter()
        .filter_map(|element| {
            let (kind, bound) = match &element.value {
                Value::Bytes(bound) | Value::Text(bound) => ("SceByteBound", bound),
                Value::List { bound, .. } => ("SceCountBound", bound),
                _ => return None,
            };
            let Bound { least, most } = *bound;
            Some(serde_json::json!({
                "name": bound_name(&element.field.id),
                "field": element.field.id,
                "protocol": kind,
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
    // Each input the value says (docs/adr/0065 §6 R4a) that a gate of the codec reads, or that the
    // codec hands a body or holds a body to, read off what the value holds under the name the gate
    // reads it by; one nothing reads is left to the parent that asks for it.
    for said in plan
        .said
        .iter()
        .filter(|said| said.read || handed_on(plan, &said.input.name))
    {
        body.push(format!(
            "let {}: UInt8 = {}",
            said.input.name,
            said_input(plan, held, &said.input.name, SOURCE)
        ));
    }
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
    // Each number the value's elements say, which a field holds or a flag of a carrier, made from
    // what it measures (docs/adr/0058 §1): the length of a byte string or a text, and the input a
    // body says. The encoded length of a body is counted once the body is encoded, below.
    let mut below: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut derived_fields: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for derived in &plan.derivations {
        let Some(number) = printed_number(&derived.element, &derived.by) else {
            continue;
        };
        if is_encoded_length(number) {
            continue;
        }
        let named = derived.element.to_string();
        let measures: Vec<MeasureText> = number
            .measures
            .iter()
            .map(|measure| match &measure.of {
                Measured::Bytes { field, offset } => {
                    let content = format!("{SOURCE}.{}", field.id);
                    let (raw, present) = if measure.gated {
                        (
                            format!("({content}?.byteCount ?? 0)"),
                            Some(format!("{content} != nil")),
                        )
                    } else {
                        (format!("{content}.byteCount"), None)
                    };
                    // A wire that holds the count less what its arithmetic takes off holds no
                    // count below that (docs/adr/0065 §5).
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
                }
                // How many entries a list or a chain holds, as the value presents it within its
                // bound (docs/adr/0065 §6 R2); where a gate leaves the list out, it counts none.
                Measured::Entries { list } => {
                    let content = format!("{SOURCE}.{}", list.id);
                    if measure.gated {
                        MeasureText {
                            size: format!("({content}?.count ?? 0)"),
                            present: Some(format!("{content} != nil")),
                            gated: true,
                        }
                    } else {
                        MeasureText {
                            size: format!("{content}.count"),
                            present: None,
                            gated: false,
                        }
                    }
                }
                // The input a body says, read off the source the value presents it by through its
                // codec (docs/adr/0065 §6 R4a); where a gate leaves the body out, it says nothing.
                Measured::Says {
                    body: holder,
                    input,
                    ..
                } => {
                    let child = held_of(holder);
                    let reader = said_reader(input);
                    let content = format!("{SOURCE}.{}", holder.id);
                    if measure.gated {
                        MeasureText {
                            size: format!("({content}.map {{ Int({child}.{reader}($0)) }} ?? 0)"),
                            present: Some(format!("{content} != nil")),
                            gated: true,
                        }
                    } else {
                        MeasureText {
                            size: format!("Int({child}.{reader}({content}))"),
                            present: None,
                            gated: false,
                        }
                    }
                }
                // The input the arm the value holds says, read through the arm's codec off the
                // source the arm presents its body as; nothing where the value holds an arm that
                // does not say it (docs/adr/0065 §6 R4a).
                Measured::ArmSays { arms: saying } => {
                    let said = format!("{}_said{}", derived_local(&derived.element), measure.at);
                    body.push(format!("var {said}: Int? = nil"));
                    for arm_input in saying {
                        let text = &arms[arm_input.arm];
                        body.extend(text.guarded(vec![format!(
                            "if case {} = {SOURCE}.{} {{ {said} = Int({}.{}(arm)) }}",
                            text.pattern("arm"),
                            super::VARIANT_FIELD,
                            text.child,
                            said_reader(&arm_input.input)
                        )]));
                    }
                    MeasureText {
                        size: format!("({said} ?? 0)"),
                        present: measure.gated.then(|| format!("{said} != nil")),
                        gated: measure.gated,
                    }
                }
                _ => unreachable!(
                    "`printed_number` holds bytes, entries, inputs and encoded lengths alone"
                ),
            })
            .collect();
        let local = derived_local(&derived.element);
        let (size, present) = combine(plan, &named, &local, &measures, &mut body);
        match (&derived.element, &number.into) {
            (ElementId::Field(id), into @ NumberInto::Field { .. }) => {
                field_number(l, plan, id, into, &size, present.as_deref(), &mut body);
                derived_fields.insert(id.clone());
            }
            (ElementId::Flag { field, flag }, NumberInto::Flag) => {
                let def = field_of(field)
                    .flags
                    .iter()
                    .find(|def| def.name == *flag)
                    .expect("the parser found the flag a number names");
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
            _ => unreachable!("`printed_number` holds a number into its own kind of element"),
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
    // The statement that sets the bits of the flag `flag` of the carrier `carrier`, folded into
    // `local`, to the number `value`, keeping the carrier's other bits.
    let flag_set = |carrier: &str, flag: &str, local: &str, value: &str| {
        let def = field_of(carrier)
            .flags
            .iter()
            .find(|def| def.name == flag)
            .expect("the parser found the flag a tag fills");
        let width = def.width.max(1);
        let bits = if width >= 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        let carrier_type = &carrier_of(field_of(carrier)).integer;
        format!(
            "{local} = ({local} & ~0x{:X}) | (({carrier_type}(truncatingIfNeeded: {value}) & 0x{bits:X}) << {})",
            bits << def.bit,
            def.bit
        )
    };
    // The tag of the arm the value holds, read off it through the arms enum's `tag`: into the
    // field the tag is, or into the bits of the flag it is (docs/adr/0081 §3).
    for derived in &plan.derivations {
        if !matches!(derived.by, Derive::VariantTag) {
            continue;
        }
        let tag = format!("{SOURCE}.{}.tag", super::VARIANT_FIELD);
        match &derived.element {
            ElementId::Field(id) => {
                let ty = l.type_name(&field_of(id).sce_type).into_owned();
                body.push(format!(
                    "let {}: {ty} = {ty}(truncatingIfNeeded: {tag})",
                    derived_local(&derived.element)
                ));
                derived_fields.insert(id.clone());
            }
            ElementId::Flag { field, flag } => {
                let carrier = fold(field, &mut body);
                body.push(flag_set(field, flag, &carrier, &tag));
            }
            ElementId::Variant => {
                unreachable!(
                    "a variant whose caller hands its tag derives none: the caller writes it"
                )
            }
        }
    }
    // The bits of a carrier the arm of each embedded body's variant fills: the tag of the arm the
    // body holds, masked to the bits, joined bit by bit; nothing where a gate leaves the body out.
    for derived in &plan.derivations {
        let (ElementId::Flag { field, flag }, Derive::ParentTag(parent)) =
            (&derived.element, &derived.by)
        else {
            continue;
        };
        let local = format!("{field}_{flag}_filled");
        body.push(format!("var {local}: UInt64 = 0"));
        for term in &parent.terms {
            let id = &term.body.id;
            let gated = plan
                .elements
                .iter()
                .find(|e| e.field.id == *id)
                .expect("a parent tag is filled by a body the value presents")
                .presence
                .optional();
            let fill = |body: &str| {
                format!(
                    "{local} |= UInt64({body}.{}.tag) & 0x{:X}",
                    super::VARIANT_FIELD,
                    term.mask
                )
            };
            if gated {
                body.push(format!(
                    "if let {HELD} = {SOURCE}.{id} {{ {} }}",
                    fill(HELD)
                ));
            } else {
                body.push(fill(&format!("{SOURCE}.{id}")));
            }
        }
        let carrier = fold(field, &mut body);
        body.push(flag_set(field, flag, &carrier, &local));
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
    // A carrier as the encode writes it: the local its flags are folded into, else the source's.
    let carrier_read = |f: &CodecField| {
        if folded.contains(&f.id) {
            folded_carrier(&f.id)
        } else {
            format!("{SOURCE}.{}", f.id)
        }
    };
    // Each input a body or the entries of a list say that the parent hands them from elsewhere,
    // held to what the parent hands them, before the first byte (docs/adr/0065 §6 R4a): a value
    // whose body or entry says otherwise has no frame.
    for check in &plan.input_checks {
        body.extend(input_check(plan, held, &arms, check, &carrier_read));
    }
    // The arguments the codec imported as `codec` is handed after the sink, one for each input it
    // takes and does not say, in the order it declares them: the bits of a carrier as the encode
    // writes it, an input of the parent's own, or, for an entry of a chain, the local the chain
    // keeps what the entry before supplies in (`supplied`).
    let handed_arguments = |codec: &str, supplied: &dyn Fn(&str) -> Option<String>| -> String {
        plan.handed
            .get(codec)
            .into_iter()
            .flatten()
            .map(|handed| {
                let value = match &handed.from {
                    Some(HandedFrom::Carrier {
                        carrier,
                        bit,
                        width,
                    }) => {
                        let mask = if *width >= 64 {
                            u64::MAX
                        } else {
                            (1u64 << width) - 1
                        };
                        format!(
                            "UInt8(truncatingIfNeeded: ({} >> {bit}) & 0x{mask:X})",
                            carrier_read(carrier)
                        )
                    }
                    Some(HandedFrom::Input { name }) => name.clone(),
                    None => supplied(&handed.input).unwrap_or_else(|| {
                        unreachable!("`unlowered` refuses an input no declaration binds")
                    }),
                };
                format!(", {}: {value}", handed.input)
            })
            .collect()
    };
    // Each body, encoded into bytes of its own by its codec, handed the inputs it does not say: its
    // encoded length is the count of those bytes, measured once (docs/adr/0058 §3).
    for element in &plan.elements {
        let Value::Body { codec } = element.value else {
            continue;
        };
        let id = &element.field.id;
        let child = held_of(element.field);
        let arguments = handed_arguments(codec, &|_| None);
        // A body whose codec refuses a value throws its refusal, which this encode passes on.
        if plan.totality.refuses(RefusalKind::Child, id) {
            plan.totality.require(RefusalKind::Child, id);
        }
        let bytes = body_bytes(id);
        let encode = |value: &str, piece: &str| {
            format!("try {child}.encodeSource({value}, into: &{piece}{arguments})")
        };
        if element.presence.optional() {
            body.push(format!("var {bytes}: [UInt8]? = nil"));
            body.push(format!("if let {HELD} = {SOURCE}.{id} {{"));
            body.push("    var piece = BytesSink()".to_string());
            body.push(format!("    {}", encode(HELD, "piece")));
            body.push(format!("    {bytes} = piece.bytes"));
            body.push("}".to_string());
        } else {
            let piece = format!("{id}_piece");
            body.push(format!("var {piece} = BytesSink()"));
            body.push(encode(&format!("{SOURCE}.{id}"), &piece));
            body.push(format!("let {bytes} = {piece}.bytes"));
        }
    }
    // The encoded length of each body a field holds, the count of the bytes it encoded to.
    for derived in &plan.derivations {
        let Some(number) = printed_number(&derived.element, &derived.by) else {
            continue;
        };
        if !is_encoded_length(number) {
            continue;
        }
        let measures: Vec<MeasureText> = number
            .measures
            .iter()
            .map(|measure| {
                let Measured::Encoded { body: holder, .. } = &measure.of else {
                    unreachable!(
                        "`printed_number` holds an encoded length that measures nothing else"
                    )
                };
                let bytes = body_bytes(&holder.id);
                if measure.gated {
                    MeasureText {
                        size: format!("({bytes}?.count ?? 0)"),
                        present: Some(format!("{bytes} != nil")),
                        gated: true,
                    }
                } else {
                    MeasureText {
                        size: format!("{bytes}.count"),
                        present: None,
                        gated: false,
                    }
                }
            })
            .collect();
        let named = derived.element.to_string();
        let local = derived_local(&derived.element);
        let (size, present) = combine(plan, &named, &local, &measures, &mut body);
        let ElementId::Field(id) = &derived.element else {
            unreachable!("`printed_number` holds an encoded length into a field")
        };
        field_number(
            l,
            plan,
            id,
            &number.into,
            &size,
            present.as_deref(),
            &mut body,
        );
        derived_fields.insert(id.clone());
    }
    // A field as the encode reads it: the number it derives, the carrier the flags are folded into,
    // else the source's.
    let read = |f: &CodecField| {
        if derived_fields.contains(&f.id) {
            derived_local(&ElementId::Field(f.id.clone()))
        } else {
            carrier_read(f)
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
        .any(|write| matches!(write, Write::Streamed { .. } | Write::Variant));
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
            // A list or a chain, where the value holds it: each entry written in turn by its
            // codec's encode into bytes of the list's own, appended once every entry is written.
            Write::Streamed {
                field: f,
                guard,
                list: Some(ref walk),
            } => {
                let Some(&Value::List { codec, .. }) = plan
                    .elements
                    .iter()
                    .find(|e| e.field.id == f.id)
                    .map(|e| &e.value)
                else {
                    unreachable!("a list the program walks is a list the value presents")
                };
                // A list whose entries' codec refuses a value throws its refusal, which this
                // encode passes on.
                if plan.totality.refuses(RefusalKind::Child, &f.id) {
                    plan.totality.require(RefusalKind::Child, &f.id);
                }
                // An input the chain supplies each entry from the entry before is the local the
                // walk keeps it in.
                let arguments = handed_arguments(codec, &|input: &str| {
                    walk.previous
                        .iter()
                        .any(|(bound, _)| bound == input)
                        .then(|| previous_local(&f.id, input))
                });
                let write =
                    |entries: &str| list_write(plan, f, walk, held_of(f), entries, &arguments);
                match guard {
                    Guard::Always => {
                        let entries = format!("{SOURCE}.{}", f.id);
                        body.extend(write(entries.as_str()))
                    }
                    Guard::Present => {
                        body.push(format!("if let {HELD} = {SOURCE}.{} {{", f.id));
                        body.extend(write(HELD).into_iter().map(|line| format!("    {line}")));
                        body.push("}".to_string());
                    }
                    Guard::Gate => unreachable!("a list is presented, never derived whole"),
                }
            }
            // A body appends the bytes it encoded to, where the value holds it.
            Write::Streamed {
                field: f, guard, ..
            } if matches!(f.bit_size, BitSize::Embed) => {
                let bytes = body_bytes(&f.id);
                match guard {
                    Guard::Always => body.push(format!("frame += {bytes}")),
                    Guard::Present => {
                        body.push(format!("if let {HELD} = {bytes} {{ frame += {HELD} }}"))
                    }
                    Guard::Gate => unreachable!("a body is presented, never derived"),
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
            // The arm the value holds, written by its codec's encode over the source its body is
            // presented as, handed what the arm's import binds; the catch-all, over its body.
            Write::Variant => {
                let written = format!("{}_arm", super::VARIANT_FIELD);
                body.push(format!("var {written} = BytesSink()"));
                body.push(format!("switch {SOURCE}.{} {{", super::VARIANT_FIELD));
                for (arm, text) in plan.arms.iter().zip(&arms) {
                    // An arm whose codec refuses a value throws its refusal, which this encode
                    // passes on.
                    if plan.totality.refuses(RefusalKind::Child, arm.codec) {
                        plan.totality.require(RefusalKind::Child, arm.codec);
                    }
                    body.extend(text.guarded(vec![
                        format!("case {}:", text.pattern("arm")),
                        format!(
                            "    try {}.encodeSource(arm, into: &{written}{})",
                            text.child,
                            handed_arguments(arm.codec, &|_| None)
                        ),
                    ]));
                }
                body.push("}".to_string());
                body.push(format!("frame += {written}.bytes"));
            }
        }
    }
    plan.totality.assert_all_written(&plan.model.name);
    // Each input the value says, read off any source of it by a function of its own: what a
    // parent that holds this codec derives the flag it binds to the input from, and what a log
    // over it stores (docs/adr/0065 §6 R4a).
    let said: Vec<serde_json::Value> = plan
        .said
        .iter()
        .map(|said| {
            serde_json::json!({
                "input": said.input.name,
                "reader": said_reader(&said.input.name),
                "lines": said_lines(plan, held, &arms, &said.input.name),
                "partly": plan.roles.says_input_partly(&said.input.name),
            })
        })
        .collect();
    // The arms of the variant, each the generic parameter of the arms enum its body is, bound by
    // the protocol to a source of the arm's codec (docs/adr/0058 §1).
    let arms_json: Vec<serde_json::Value> = arms
        .iter()
        .map(|arm| {
            serde_json::json!({
                "case": arm.case,
                "generic": arm.generic,
                "assoc": arm.assoc,
                "protocol": format!("{}Source", arm.child),
                "type": arm.child,
                "off": arm.off,
                "tag": arm.tag.map(|tag| format!("0x{tag:X}")),
            })
        })
        .collect();
    serde_json::json!({
        "protocol": source_protocol(&plan.model.name),
        "requirements": requirements,
        "bodies": bodies,
        "entries": entries,
        "said": said,
        "given": plan.given.iter().map(|input| input.name.clone()).collect::<Vec<_>>(),
        "refuses": !plan.totality.is_total(),
        "bounds": bounds,
        "body": body,
        "overrides": overrides,
        "arms": arms_json,
        "arms_enum": arms_enum(&struct_name),
        "unknown_tag": super::unknown_tag_type(&plan.model.name),
        "tag_type": l
            .type_name(&crate::forge::codec_totality::tag_type(plan.model))
            .into_owned(),
    })
}

/// The statements that hold the input a body or every entry of a list says to what the parent
/// hands it from elsewhere ([`InputCheck`], docs/adr/0065 §6 R4a), in a scope of their own: the
/// value is refused with `presentIfMismatch` where its body or an entry says otherwise. A body
/// that says the input in some of its values only disagrees with nothing in the others (§6 R4e).
/// The entries of a chain are each handed whether the entry before is of a kind, the first none,
/// which the check walks the chain for as the encode will (`Against::PreviousEntry`).
fn input_check(
    plan: &EncodePlan<'_>,
    held: &HeldTypes,
    arms: &[ArmText],
    check: &InputCheck<'_>,
    carrier_read: &dyn Fn(&CodecField) -> String,
) -> Vec<String> {
    let reader = said_reader(&check.input);
    let says_otherwise = |child: &str, value: &str| {
        if check.partly {
            format!("({child}.{reader}({value}).map {{ ($0 != 0) != handed }} ?? false)")
        } else {
            format!("(({child}.{reader}({value}) != 0) != handed)")
        }
    };
    let refuse = refusal(plan, RefusalKind::PresentIfMismatch, &check.at);
    let holder = match &check.site {
        CheckSite::Embed { body } => *body,
        CheckSite::List { list } => *list,
        // The arm the value holds, where it is one of the arms whose codec says the input.
        CheckSite::Arm { arms: checked } => {
            let handed = match &check.against {
                Against::Carrier { carrier, bit } => {
                    format!("(({} >> {bit}) & 1) != 0", carrier_read(carrier))
                }
                Against::Input { name } => format!("{name} != 0"),
                Against::PreviousEntry { .. } => {
                    unreachable!("an arm is handed no entry of a chain")
                }
            };
            let mut lines = vec![
                "do {".to_string(),
                format!("    let handed: Bool = {handed}"),
            ];
            for &i in checked {
                let text = &arms[i];
                lines.extend(text.guarded(vec![format!(
                    "    if case {} = {SOURCE}.{}, {} {{ {refuse} }}",
                    text.pattern("arm"),
                    super::VARIANT_FIELD,
                    says_otherwise(&text.child, "arm")
                )]));
            }
            lines.push("}".to_string());
            return lines;
        }
    };
    let child = held
        .get(&holder.id)
        .expect("a body or a list whose input is checked holds a codec of the module");
    let says_otherwise = |value: &str| says_otherwise(child, value);
    let before = format!("{}_before", holder.id);
    let handed = match &check.against {
        Against::Carrier { carrier, bit } => {
            format!("(({} >> {bit}) & 1) != 0", carrier_read(carrier))
        }
        Against::Input { name } => format!("{name} != 0"),
        Against::PreviousEntry { id } => format!("{before} == 0x{id:X}"),
    };
    let gated = holder.present_if.is_some();
    let content = if gated {
        HELD.to_string()
    } else {
        format!("{SOURCE}.{}", holder.id)
    };
    let mut lines = vec!["do {".to_string()];
    match &check.site {
        CheckSite::List { .. } => {
            let indent = if gated { "    " } else { "" };
            let previous = matches!(check.against, Against::PreviousEntry { .. });
            if previous {
                lines.push(format!("    var {before}: UInt64? = nil"));
            }
            if gated {
                lines.push(format!("    if let {HELD} = {SOURCE}.{} {{", holder.id));
            }
            lines.push(format!("    {indent}for entry in {content} {{"));
            lines.push(format!("    {indent}    let handed: Bool = {handed}"));
            lines.push(format!(
                "    {indent}    if {} {{ {refuse} }}",
                says_otherwise("entry")
            ));
            if previous {
                lines.push(format!(
                    "    {indent}    {before} = {}",
                    entry_identifier(plan, &holder.id, "entry")
                ));
            }
            lines.push(format!("    {indent}}}"));
            if gated {
                lines.push("    }".to_string());
            }
        }
        _ => {
            let differs = if gated {
                format!(
                    "let {HELD} = {SOURCE}.{}, {}",
                    holder.id,
                    says_otherwise(HELD)
                )
            } else {
                says_otherwise(&content)
            };
            lines.push(format!("    let handed: Bool = {handed}"));
            lines.push(format!("    if {differs} {{ {refuse} }}"));
        }
    }
    lines.push("}".to_string());
    lines
}

/// The local the walk of the chain `list` keeps the input `input` it hands each entry in, set from
/// the entry before (`ListWrite::previous`): zero for the first entry. A field is read through the
/// source and never bare, so the local hides no field.
fn previous_local(list: &str, input: &str) -> String {
    format!("{list}_{input}_previous")
}

/// The statements that write the entries of the list or chain `list`, which the encode reads as
/// `entries`, each a source of `child`'s codec, by its printed encode into bytes of the list's
/// own, appended to the frame once every entry is written: handed `arguments` after the sink,
/// each input the chain supplies set from the entry just written, and, on a chain that ends on
/// its entries' flag, the flag set through the entry codec's `Overrides` on every entry but the
/// last (docs/adr/0065 §6 R7), whatever flag the entry presents.
fn list_write(
    plan: &EncodePlan<'_>,
    list: &CodecField,
    walk: &ListWrite,
    child: &str,
    entries: &str,
    arguments: &str,
) -> Vec<String> {
    let id = &list.id;
    let written = format!("{id}_entries");
    let left = format!("{id}_left");
    let mut lines = vec![format!("var {written} = BytesSink()")];
    for (input, _) in &walk.previous {
        lines.push(format!("var {}: UInt8 = 0", previous_local(id, input)));
    }
    let overrides = match &walk.ends {
        Some((carrier, flag)) => {
            lines.push(format!("var {left} = {entries}.count"));
            format!(
                ", overrides: {child}.Overrides({}: {left} != 0)",
                override_member(carrier, flag)
            )
        }
        None => String::new(),
    };
    lines.push(format!("for entry in {entries} {{"));
    if walk.ends.is_some() {
        lines.push(format!("    {left} -= 1"));
    }
    lines.push(format!(
        "    try {child}.encodeSource(entry, into: &{written}{arguments}{overrides})"
    ));
    for (input, kind) in &walk.previous {
        lines.push(format!(
            "    {} = {} == 0x{kind:X} ? 1 : 0",
            previous_local(id, input),
            entry_identifier(plan, id, "entry")
        ));
    }
    lines.push("}".to_string());
    lines.push(format!("frame += {written}.bytes"));
    lines
}

/// One arm of a printed codec's variant, as the Swift printer spells it.
pub(super) struct ArmText {
    /// The case of the arms enum: the arm codec's alias in lower camel case, as the decode names
    /// it; unused for the catch-all, whose case is `default`.
    case: String,
    /// The Swift struct of the arm's codec.
    child: String,
    /// The associated type of the protocol the arm's body is a source of: the alias in upper camel
    /// case and `_Arm`, or `Default_Arm` for the catch-all. The `_` is in no codec's type name, so
    /// it hides none.
    assoc: String,
    /// The generic parameter of the arms enum the arm's body is: `A` and the arm's place, or `D`
    /// for the catch-all.
    generic: String,
    /// The condition a build leaves the arm out by, for an arm under a build feature
    /// (docs/adr/0072).
    off: Option<String>,
    /// The arm's tag, or `None` for the catch-all.
    tag: Option<u64>,
}

impl ArmText {
    /// The pattern that binds the body of this arm as `bound`.
    fn pattern(&self, bound: &str) -> String {
        match self.tag {
            Some(_) => format!(".{}(let {bound})", self.case),
            None => format!(".default(_, let {bound})"),
        }
    }

    /// The pattern that matches this arm and binds nothing.
    fn bare(&self) -> String {
        match self.tag {
            Some(_) => format!(".{}", self.case),
            None => ".default".to_string(),
        }
    }

    /// `lines` in the build that holds this arm: inside its condition where a feature decides it.
    fn guarded(&self, lines: Vec<String>) -> Vec<String> {
        match &self.off {
            None => lines,
            Some(off) => std::iter::once(format!("#if !{off}"))
                .chain(lines)
                .chain(std::iter::once("#endif".to_string()))
                .collect(),
        }
    }
}

/// The arms of the variant of `plan`, in the plan's order, the struct of each arm's codec in
/// `arm_types` at the same place.
pub(super) fn arm_texts(plan: &EncodePlan<'_>, arm_types: &[String]) -> Vec<ArmText> {
    plan.arms
        .iter()
        .zip(arm_types)
        .enumerate()
        .map(|(i, (arm, child))| {
            let known = arm.tag.is_some();
            ArmText {
                case: filters::to_camel_case(arm.codec.to_string()),
                child: child.clone(),
                assoc: if known {
                    format!("{}_Arm", filters::to_pascal_case(arm.codec.to_string()))
                } else {
                    "Default_Arm".to_string()
                },
                generic: if known {
                    format!("A{i}")
                } else {
                    "D".to_string()
                },
                off: arm.feature.map(super::feature_off_condition),
                tag: arm.tag,
            }
        })
        .collect()
}

/// The generic enum of the arms a printed codec named `struct_name` presents its variant as.
fn arms_enum(struct_name: &str) -> String {
    format!("{struct_name}Arms")
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
