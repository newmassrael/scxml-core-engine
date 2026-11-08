// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift codec, in part: a positional codec whose fields are native-width integers laid at
//! byte offsets, written as a struct with the `decode` and `encode` the runtime's cursor and sink
//! (`backends/swift/forge-runtime/Sources/SceForgeRuntime/Codec.swift`) carry out.
//!
//! It is written beside the shared `render_codec` and not through it. That function is the codec
//! of every other backend and carries a branch for each of the shapes a codec can hold in each
//! language, and a backend that lowers one shape in part does not need the rest of them threaded
//! through a function that is not its own: what the Swift codec does not write is refused by name,
//! before a field is spelled ([`unlowered`]), which is what
//! [`crate::forge::codegen_matrix::ships_in_part`] promises of an in-part backend. The refusal
//! shrinks as the shapes land, and the codec moves to the shared renderer when nothing is left.

use super::*;
use crate::generator::Language;

/// The Swift spelling of a native-width integer type, and its width in bits: the one set of types a
/// field may have.
fn integer(sce_type: &SceType) -> Option<(&'static str, u32)> {
    Some(match sce_type {
        SceType::Uint8 => ("UInt8", 8),
        SceType::Uint16 => ("UInt16", 16),
        SceType::Uint32 => ("UInt32", 32),
        SceType::Uint64 => ("UInt64", 64),
        SceType::Int8 => ("Int8", 8),
        SceType::Int16 => ("Int16", 16),
        SceType::Int32 => ("Int32", 32),
        SceType::Int64 => ("Int64", 64),
        _ => return None,
    })
}

/// What of the codec `m` the Swift backend does not lower yet, described for a refusal by name
/// (`generate/unsupported-feature`) before [`render`] writes a field; `None` for a codec it does.
///
/// Swift lowers a positional codec of fields that are integers of a native width, each at a byte
/// offset and in a byte order the document names. Each construct past that is named here until its
/// spelling is written.
///
/// `element` answers, for the alias a `<sce:repeat>` names its entries by, what of that codec the
/// Swift backend does not write (`None` for one it does): a list of entries whose unit is not
/// written names a type that is not there.
pub(super) fn unlowered(
    m: &CodecModel,
    element: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    match m.encoding {
        CodecEncoding::Positional => {}
        CodecEncoding::Cbor => return cbor_unlowered(m),
        CodecEncoding::ContentLine => return content_line_unlowered(m),
    }
    if let Some(variant) = &m.variant {
        if let Some(why) = variant_unlowered(m, variant, element) {
            return Some(why);
        }
    }
    // A flag input is a parameter of `decode` and of `encode`, read in the body by the name the
    // document gives it, so it must not be a name the body already uses for something else.
    if let Some(input) = m
        .flag_inputs
        .iter()
        .find(|input| LOCALS.contains(&input.name.as_str()))
    {
        return Some(format!(
            "a flag input named `{}`, a name the body of the codec uses",
            input.name
        ));
    }
    // A codec with no field is a frame of no bytes: what a body whose tag is written outside it
    // carries (`codec_zenoh_keep_alive`).
    m.fields
        .iter()
        .find_map(|f| field_unlowered(m, f, element))
        .or_else(|| flags_unlowered(m))
}

/// What of the `sce:encoding="cbor"` codec `m` the Swift codec does not lower yet; `None` for one it
/// does.
///
/// It is a struct with one property per entry, so an entry may not be named for a member the struct
/// has. An entry that is an enumeration is read as its carrier and converted by the enum's own
/// `fromUnderlying`, which for an open enum is total ([`crate::forge::cbor_codec`]). A list of records
/// is written where `cbor_codec::lowers_record_list` says, which `cbor_codec::refusal` answers before
/// this is asked.
pub(super) fn cbor_unlowered(m: &CodecModel) -> Option<String> {
    for e in &m.cbor_entries {
        if MEMBERS.contains(&e.id.as_str()) {
            return Some(format!(
                "the CBOR entry '{}', named for a member of the struct",
                e.id
            ));
        }
    }
    None
}

/// What of the content-line codec `m` (SCE_FORGE.md section 4.6.4) the Swift codec does not lower yet:
/// an entry, or a parameter of one, named for a member of the struct, which the struct would declare
/// twice. The line grammar and the shapes of an entry are the runtime's and the template's, and are the
/// same on every backend that lowers one ([`crate::forge::content_line_codec::lowers`]).
pub(super) fn content_line_unlowered(m: &CodecModel) -> Option<String> {
    if let Some(why) = crate::forge::content_line_codec::refusal(Language::Swift, m) {
        return Some(why);
    }
    let model = m.content_line.as_ref()?;
    model
        .entries
        .iter()
        .find(|e| MEMBERS.contains(&e.id.as_str()))
        .map(|e| {
            format!(
                "the content-line entry '{}', named for a member of the struct",
                e.id
            )
        })
}

/// What of the codec `m` the Swift codec does not lower yet when another document holds it as an
/// element (an embedded codec, the entries of a list or a chain, the element of a log) and not as a
/// document of its own. A codec of another encoding is a document of its own: a CBOR map is
/// self-delimiting, but it has no fixed length for the document that holds it to add to its own, and
/// is held by no other document yet.
pub(super) fn element_unlowered(
    m: &CodecModel,
    element: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    match m.encoding {
        CodecEncoding::Positional => unlowered(m, element),
        CodecEncoding::Cbor => Some("a `sce:encoding=\"cbor\"` codec".to_string()),
        CodecEncoding::ContentLine => Some("a `sce:encoding=\"content-line\"` codec".to_string()),
    }
}

/// The members a codec struct has besides its fields and the flags its fields carry.
const MEMBERS: &[&str] = &["frameLength", "init", "decode", "encode"];

/// The names the body of `decode` and `encode` binds besides the fields it reads through `frame` and
/// `self`, which a parameter of the same name would hide.
const LOCALS: &[&str] = &[
    "cursor", "reader", "frame", "sink", "piece", "entry", "arm", "value",
];

/// The property that reads and writes the flag `name` of a carrier: lowerCamel, as a Swift
/// property is spelled.
pub(super) fn flag_property(name: &str) -> String {
    filters::to_camel_case(name.to_string())
}

/// What of the flags the fields of `m` carry the Swift codec does not lower yet: a flag is a
/// property of the struct beside the fields, so a name the struct already has (a member of it,
/// or a field), or that two flags would share, is refused and not renamed.
///
/// A name that is a reserved word, in Swift's list or in any backend's, and two flags whose names
/// are the one name in any backend, are refused when the document is validated
/// (`ReservedCodeIdentifier`, `CollidingCodeIdentifier`), spelled as each backend spells them; a
/// flag and a field are not compared there, and the members the struct has are Swift's own.
fn flags_unlowered(m: &CodecModel) -> Option<String> {
    let mut taken: std::collections::BTreeMap<String, String> = m
        .fields
        .iter()
        .map(|f| (f.id.clone(), format!("the field '{}'", f.id)))
        .collect();
    for member in MEMBERS {
        taken.insert((*member).to_string(), format!("the member `{member}`"));
    }
    for f in &m.fields {
        for flag in &f.flags {
            let property = flag_property(&flag.name);
            if let Some(other) = taken.insert(property.clone(), format!("the flag `{}`", flag.name))
            {
                return Some(format!(
                    "field '{}': the flag `{}` and {other}, which are the one property `{property}`",
                    f.id, flag.name
                ));
            }
        }
    }
    None
}

/// What of the field `f` of `m` the Swift codec does not lower yet.
fn field_unlowered(
    m: &CodecModel,
    f: &CodecField,
    element: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    if f.present_if.is_some() && !f.flags.is_empty() {
        return what("a flags carrier that is present only on a condition");
    }
    if let BitSize::Repeat { count_ref } = &f.bit_size {
        return repeat_unlowered(m, f, count_ref, element);
    }
    if f.is_tlv_chain() {
        return chain_unlowered(f, element);
    }
    if f.is_embed() {
        return embed_unlowered(m, f, element);
    }
    if f.dma_burst_align.is_some() {
        return what("a DMA alignment");
    }
    if f.is_vle() {
        return vle_field_unlowered(f);
    }
    if matches!(f.bit_size, BitSize::Tail | BitSize::LengthRef) {
        return variable_field_unlowered(m, f);
    }
    if f.is_variable_length() || f.length_field.is_some() {
        return what("a variable-length field");
    }
    // A field of bits inside one byte has no byte order: it is the byte's own bits. It is laid
    // at a byte of the frame, which a frame that is read field by field has no fixed place for.
    if f.bit_offset.is_some() {
        if is_sequential(m) {
            return what("a bit offset in a frame whose layout depends on what it holds");
        }
        return match bit_field(f) {
            Ok(_) => None,
            Err(reason) => what(&reason),
        };
    }
    if f.effective_endian(m.default_endian) == Endian::Native {
        return what("the native byte order");
    }
    let Some((swift_type, width)) = integer(&f.sce_type) else {
        return what(&format!("a `{}` field", f.sce_type.as_attr()));
    };
    match f.bit_size {
        BitSize::Fixed { bits } if bits == width => None,
        BitSize::Fixed { bits } => what(&format!(
            "{bits} bits where its type {swift_type} holds {width}"
        )),
        _ => what("a bit size that is not fixed"),
    }
}

/// What of the variant `v` of `m` the Swift codec does not lower yet; `None` for one it does.
///
/// It is the arm a tag selects, held with the fields before it: the tag is a field of the codec
/// (the whole of it, or a range of bits of a flags carrier), and each arm is the struct of an
/// imported entry codec, read from the frame once the tag is. The unit of every arm must be
/// written (`element`). An arm that is built only under a feature is named. Two arms of one entry
/// codec would be two cases of one name, and a field named `body` would be the name of the arm's
/// own. A tag its caller supplies is the `tag` the decode is given; an encode needs none, since the
/// arm that is held says which it is.
fn variant_unlowered(
    m: &CodecModel,
    v: &CodecVariant,
    element: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let what = |text: &str| Some(format!("a `<sce:variant>` {text}"));
    // The tag its caller supplies is the `tag` of the decode, and the value answers it as `tag`: a
    // field of that name would be the name of both.
    if v.tag_field.is_none() && m.fields.iter().any(|f| f.id == CALLER_TAG) {
        return what(&format!(
            "whose tag its caller supplies, in a codec with a field named `{CALLER_TAG}`"
        ));
    }
    if v.arms.iter().any(|arm| arm.feature.is_some()) {
        return what("with an arm built only under a feature");
    }
    if m.fields.iter().any(|f| f.id == VARIANT_FIELD) {
        return what(&format!(
            "in a codec with a field named `{VARIANT_FIELD}`, which holds the arm"
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for arm in &v.arms {
        if seen.contains(&arm.body_alias.as_str()) {
            return what(&format!("with two arms of '{}'", arm.body_alias));
        }
        seen.push(arm.body_alias.as_str());
    }
    v.arms
        .iter()
        .chain(v.default_arm.as_ref())
        .find_map(|arm| {
            element(&arm.body_alias).map(|why| {
                what(&format!(
                    "with an arm '{}', which has no Swift lowering ({why})",
                    arm.body_alias
                ))
            })
        })
        .flatten()
}

/// The name of the field a variant's arm is held in.
const VARIANT_FIELD: &str = "body";

/// The name of the tag a caller gives a decode, and of the property that answers it for a value.
const CALLER_TAG: &str = "tag";

/// What of the list `f` (a `<sce:repeat>`) of `m` the Swift codec does not lower yet; `None` for one
/// it does.
///
/// It is an `Array` of the struct the entry codec is written as, read from the frame until the count
/// an earlier field states is reached, or to the end of the frame. The entries' unit must be
/// written (`element`), and a count is read from an unsigned field: a signed one can be negative,
/// which no range of entries is. A gated list may have a gated count (validation holds the two to
/// one predicate) or an ungated one.
fn repeat_unlowered(
    m: &CodecModel,
    f: &CodecField,
    count_ref: &CountRef,
    element: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    let Some(alias) = f.repeat_body_alias.as_deref() else {
        return what("a list with no entry codec");
    };
    if let Some(why) = element(alias) {
        return what(&format!(
            "a list of '{alias}', which has no Swift lowering ({why})"
        ));
    }
    if let CountRef::LengthField(name) = count_ref {
        let unsigned = m.fields.iter().find(|g| g.id == *name).map(|g| {
            matches!(
                g.sce_type,
                SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64
            )
        });
        if unsigned != Some(true) {
            return what(&format!(
                "a count held in `{name}`, which is not an unsigned integer"
            ));
        }
    }
    None
}

/// What of the chain `f` (a `<sce:tlv-chain>`) the Swift codec does not lower yet; `None` for one
/// it does.
///
/// It is an `Array` of the struct the entry codec is written as, read one entry after another until
/// the frame ends or the entry says it is the last, and at most as many as the document's `max-depth`
/// (`terminate_on`). The entry codec's unit must be written (`element`). A chain may hand its
/// entries an input read off the entry before, which the loop carries in a local of its own.
fn chain_unlowered(f: &CodecField, element: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    let Some(alias) = f.tlv_chain_body_alias.as_deref() else {
        return what("a chain with no entry codec");
    };
    if let Some(why) = element(alias) {
        return what(&format!(
            "a chain of '{alias}', which has no Swift lowering ({why})"
        ));
    }
    None
}

/// What of the embedded codec `f` (a `<sce:embed>`) of `m` the Swift codec does not lower yet;
/// `None` for one it does.
///
/// It is a field of the struct the entry codec is written as, read in place from the frame with no
/// bytes of its own, optional when gated. The unit of the embedded codec must be written
/// (`element`), which a codec that takes a flag input or dispatches on a tag is not. When the
/// embedded codec is bounded by the length an earlier field states (`sce:length-from`), that field
/// must be an unsigned integer: validation holds it to one every frame carries, and a signed one
/// can be negative, which no number of bytes is.
fn embed_unlowered(
    m: &CodecModel,
    f: &CodecField,
    element: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    let Some(alias) = f.embed_body_alias.as_deref() else {
        return what("an embedded codec with no codec");
    };
    if let Some(why) = element(alias) {
        return what(&format!(
            "an embedded '{alias}', which has no Swift lowering ({why})"
        ));
    }
    if let Some(name) = f.embed_length_from.as_deref() {
        let unsigned = m.fields.iter().find(|g| g.id == name).map(|g| {
            matches!(
                g.sce_type,
                SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64
            )
        });
        if unsigned != Some(true) {
            return what(&format!(
                "a length held in `{name}`, which is not an unsigned integer"
            ));
        }
    }
    None
}

/// Whether `f` runs to the end of the frame or for the length an earlier field states.
fn is_variable(f: &CodecField) -> bool {
    matches!(f.bit_size, BitSize::Tail | BitSize::LengthRef)
}

/// Whether the frame of `m` is read and written field by field.
///
/// A frame of fields that are the whole of an integer, or bits inside a byte, is laid out at the
/// byte each field names: the fields are at fixed places. A frame with a field whose size depends
/// on what it holds (a variable-length integer, a `tail`, a `length-ref`) has no fixed place for
/// the fields after it, so it is read from the cursor in the order of the document, and each field
/// begins where the last one ended; the `sce:byte` of a field is then its place in that order.
fn is_sequential(m: &CodecModel) -> bool {
    m.variant.is_some()
        || m.fields.iter().any(|f| {
            f.is_vle()
                || is_variable(f)
                || f.is_repeat()
                || f.is_embed()
                || f.is_tlv_chain()
                || f.present_if.is_some()
        })
}

/// The Swift condition under which a field gated by `predicate` is on the wire: the bit a flag of
/// an earlier carrier names is set, or the flag input the codec is given (or clear, for a negated
/// clause), or an entry of a chain the codec holds has a given identifier (or none does, for a
/// negated clause), and any clause of a disjunction holds. `holder` names the value whose carriers
/// are read: the frame being decoded, or `self` when it is encoded; an input is a parameter of both,
/// and a chain's answer is the boolean the codec binds beside the chain ([`chain_has_lines`]). The
/// mask is the one every backend tests, from the shared resolution of the predicate. The carrier a
/// local clause names, and its flag, exist and are unsigned integers, and an input a clause names is
/// one the codec declares: validation has refused a codec where they do not.
fn present_if_condition(m: &CodecModel, predicate: &PresentIfPredicate, holder: &str) -> String {
    let test = if predicate.scope == PresentIfScope::Chain {
        present_if_chain_clause(predicate, Language::Swift)
    } else {
        let (mask, _, carrier) = present_if_carrier_info(&m.fields, predicate);
        let operand = match carrier {
            PresentIfCarrier::Local(carrier) => format!("{holder}.{}", carrier.id),
            PresentIfCarrier::Input => predicate.flag_name.clone(),
        };
        format!(
            "({operand} & 0x{mask:X}) {} 0",
            if predicate.negate { "==" } else { "!=" }
        )
    };
    match &predicate.or_with {
        Some(rest) => format!("{test} || {}", present_if_condition(m, rest, holder)),
        None => test,
    }
}

/// What of the variable-length integer `f` the Swift codec does not lower yet; `None` for one it
/// does. The parser takes a `uint16`, a `uint32` or a `uint64` for one and gives it the width of
/// its type, which is the width the runtime's `readVle` and `appendVle` take from the type they
/// are given, so every variable-length integer a document can declare is lowered.
fn vle_field_unlowered(f: &CodecField) -> Option<String> {
    let lowered = matches!(
        (&f.sce_type, &f.bit_size),
        (
            SceType::Uint16 | SceType::Uint32 | SceType::Uint64,
            BitSize::Vle { .. }
        )
    );
    if lowered {
        None
    } else {
        Some(format!(
            "field '{}': a variable-length integer of type `{}`",
            f.id,
            f.sce_type.as_attr()
        ))
    }
}

/// What of the variable-length field `f` of `m` (a `tail` or a `length-ref`) the Swift codec does
/// not lower yet; `None` for one it does.
///
/// It is a `bytes` field, held as an array, or a `string` field, held as the `String` its UTF-8 bytes
/// spell. A `length-ref` takes its count from an unsigned integer field of the codec, plus the
/// `length-arith` the document names; a length field that is on the wire only on a condition is
/// read inside the gate of the field it counts, so the two must hold the one predicate. A length
/// held in the bits of a flag is read from the carrier the frame has read. A length held in a signed
/// field is named.
fn variable_field_unlowered(m: &CodecModel, f: &CodecField) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    if !matches!(f.sce_type, SceType::Bytes | SceType::String) {
        return what(&format!(
            "a `{}` field of variable length",
            f.sce_type.as_attr()
        ));
    }
    if matches!(f.bit_size, BitSize::LengthRef) {
        let Some(name) = f.length_field.as_deref() else {
            return what("a length-ref with no length field");
        };
        // A length held in the bits of a flag is read from a carrier the frame has already
        // read, which validation holds to a flags carrier of an unsigned type that every frame
        // carries; only a length held in a field of its own has more to be held to.
        if dotted_length_field(name).is_none() {
            let unsigned = m.fields.iter().find(|g| g.id == name).map(|g| {
                matches!(
                    g.sce_type,
                    SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64
                )
            });
            if unsigned != Some(true) {
                return what(&format!(
                    "a length held in `{name}`, which is not an unsigned integer"
                ));
            }
            if m.fields
                .iter()
                .any(|g| g.id == name && g.present_if.is_some() && g.present_if != f.present_if)
            {
                return what(&format!(
                    "a length held in `{name}`, which is on the wire only on a condition the field is not"
                ));
            }
        }
    }
    None
}

/// The length a variable-length field `f` reads from the field `name`, as Swift spells it while the
/// frame is decoded: the field, or, where it is on the wire only on the predicate `f` holds too, the
/// value it has where `f` is read, which the gate the two share proves to be present.
fn carried_length_expr(m: &CodecModel, name: &str) -> String {
    // The bits of a flag of a carrier the frame has read, shifted down and masked.
    if let Some((carrier, flag)) = dotted_length_field(name) {
        let (shift, mask) = dotted_length_resolve(carrier, flag, &m.fields);
        return format!("((frame.{carrier} >> {shift}) & 0x{mask:X})");
    }
    if m.fields
        .iter()
        .any(|g| g.id == name && g.present_if.is_some())
    {
        format!("(frame.{name} ?? 0)")
    } else {
        format!("frame.{name}")
    }
}

/// Where the bits of a field that names a `sce:bit-offset` lie in its byte: the shift that brings
/// them to the least significant end and the mask of their width at zero. The field is a `uint8`
/// holding `bit-size` bits from `bit-offset`, wholly inside the byte at `sce:byte`; any other
/// shape (a wider type, a bit size that is not fixed, bits past the byte) is the reason it is not
/// lowered.
fn bit_field(f: &CodecField) -> Result<(u32, u8), String> {
    if !matches!(f.sce_type, SceType::Uint8) {
        return Err(format!(
            "a bit offset on a `{}` field, which only a `uint8` takes",
            f.sce_type.as_attr()
        ));
    }
    let BitSize::Fixed { bits } = f.bit_size else {
        return Err("a bit offset with a bit size that is not fixed".to_string());
    };
    let offset = f.bit_offset.unwrap_or(0);
    if bits == 0 || offset + bits > 8 {
        return Err(format!(
            "{bits} bits from bit {offset}, which are not inside one byte"
        ));
    }
    Ok((offset, ((1u32 << bits) - 1) as u8))
}

/// The refusals, at the top of an encode, of a value whose gated field and whose gate contradict each
/// other: the field is held by the value when the gate is clear, or is not when it is set. They are
/// made for the fields `checks` names, which are the ones whose gate the value does not carry (a flag
/// input the caller hands in), and before the first byte is written, as the other backends make them.
fn presence_mismatch_checks(
    m: &CodecModel,
    checks: &[crate::forge::codec_roles::Check],
) -> Vec<String> {
    let mut lines = Vec::new();
    for field in &m.fields {
        let Some(predicate) = &field.present_if else {
            continue;
        };
        if !checks.iter().any(|check| check.field == field.id) {
            continue;
        }
        let test = present_if_condition(m, predicate, "self");
        let refuse = "throw CodecError.presentIfMismatch";
        lines.push(format!("if {test} {{"));
        lines.push(format!("    if self.{} == nil {{", field.id));
        lines.push(format!("        {refuse}"));
        lines.push("    }".to_string());
        lines.push(format!("}} else if self.{} != nil {{", field.id));
        lines.push(format!("    {refuse}"));
        lines.push("}".to_string());
    }
    lines
}

/// What the template needs to write a variant: the field that holds the arm, and the enum of arms
/// that field has the type of.
struct VariantParts {
    /// The `body` field, which the struct holds the active arm in.
    field: serde_json::Value,
    /// The enum of arms, declared before the struct.
    arms_enum: serde_json::Value,
    /// The fewest bytes of any arm: what the variant adds to the codec's own `frameLength`.
    fewest_bytes: String,
}

/// The smallest unsigned Swift integer type that holds `width` bits.
fn smallest_unsigned(width: u32) -> &'static str {
    match width {
        0..=8 => "UInt8",
        9..=16 => "UInt16",
        17..=32 => "UInt32",
        _ => "UInt64",
    }
}

/// The variant `v` of `m`, [`variant_unlowered`] having found nothing to refuse in it.
///
/// The tag is read from the fields before the arm, as they have been read into `frame`: the whole of
/// a field, or the bits of a flag in a carrier, shifted down and masked. Each arm of the enum holds the
/// struct of its entry codec; the catch-all holds the tag the frame said besides, so that a value
/// that names no arm is not lost. Where there is no catch-all the tag domain is covered by the arms
/// (validation has refused a codec where it is not), and the last arm stands for what is left, as
/// Swift requires every `switch` over an integer to end with one that matches the rest.
fn variant_context(
    m: &CodecModel,
    v: &CodecVariant,
    imports: &[ImportContext],
    thread: &dyn Fn(&str) -> (String, String),
) -> Result<VariantParts, ForgeError> {
    let enum_name = format!("{}Variant", filters::to_pascal_case(m.name.clone()));
    // The tag as the fields before the arm have left it, and the type a tag the arms do not name is
    // held in. A tag that is peeked is a range of bits of the next byte, which stays where it is
    // for the arm to read as its own header; one the caller supplies is the `tag` the decode is
    // given, a byte; any other is a field of the codec.
    let (tag, tag_type) = if v.tag_field.is_none() {
        ("tag".to_string(), "UInt8")
    } else if let Some(peek) = &v.peek_byte {
        let name = v
            .tag_flag
            .as_ref()
            .expect("validation holds a peeked tag to a flag of the byte");
        let flag = peek
            .flags
            .iter()
            .find(|fl| &fl.name == name)
            .expect("validation holds a peeked tag to a flag of the byte");
        let width = flag.width.max(1);
        (
            format!(
                "try (reader.peekInteger(UInt8.self, at: 0, .big) >> {}) & 0x{:X}",
                flag.bit,
                (1u64 << width) - 1
            ),
            smallest_unsigned(width),
        )
    } else {
        let tag_field = v
            .tag_field
            .as_deref()
            .expect("a variant that takes no tag of its caller has a tag field");
        let carrier = m
            .fields
            .iter()
            .find(|f| f.id == tag_field)
            .expect("validation holds the tag to a field of the codec");
        let (carrier_type, _) =
            integer(&carrier.sce_type).expect("validation holds the tag to an unsigned integer");
        match &v.tag_flag {
            Some(name) => {
                let flag = carrier
                    .flags
                    .iter()
                    .find(|fl| &fl.name == name)
                    .expect("validation holds the tag to a flag of its carrier");
                let width = flag.width.max(1);
                let mask = if width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << width) - 1
                };
                (
                    format!("(frame.{tag_field} >> {}) & 0x{mask:X}", flag.bit),
                    smallest_unsigned(width),
                )
            }
            None => (format!("frame.{tag_field}"), carrier_type),
        }
    };
    let local = format!("{VARIANT_FIELD}_tag");
    let narrowed = if v.tag_flag.is_some() {
        format!("{tag_type}(truncatingIfNeeded: {local})")
    } else {
        local.clone()
    };
    let entry = |alias: &str| resolve_repeat_body_type(&m.name, alias, imports, Language::Swift);
    let mut arms = Vec::new();
    for arm in &v.arms {
        arms.push((
            arm,
            filters::to_camel_case(arm.body_alias.clone()),
            entry(&arm.body_alias)?,
        ));
    }
    let default = match &v.default_arm {
        Some(arm) => Some(entry(&arm.body_alias)?),
        None => None,
    };
    // Without a catch-all the last arm matches whatever the others do not.
    let last = arms.len().saturating_sub(1);
    let mut cases: Vec<serde_json::Value> = arms
        .iter()
        .enumerate()
        .map(|(index, (arm, case, ty))| {
            let label = if default.is_none() && index == last {
                "default:".to_string()
            } else {
                format!("case 0x{:X}:", arm.value)
            };
            let (decode_args, encode_args) = thread(&arm.body_alias);
            serde_json::json!({
                "label": label,
                "value": format!(".{case}(try {ty}.decode(&reader{decode_args}))"),
                "pattern": format!(".{case}(let arm)"),
                "encode_args": encode_args,
            })
        })
        .collect();
    if let (Some(ty), Some(arm)) = (&default, &v.default_arm) {
        let (decode_args, encode_args) = thread(&arm.body_alias);
        cases.push(serde_json::json!({
            "label": "default:",
            "value": format!(".default(tag: {narrowed}, body: try {ty}.decode(&reader{decode_args}))"),
            "pattern": ".default(_, let arm)",
            "encode_args": encode_args,
        }));
    }
    // What a value that has read no frame yet holds: the arm the document marks as its default, and
    // else the first.
    let initial = match arms
        .iter()
        .find(|(arm, ..)| arm.is_default)
        .or(arms.first())
    {
        Some((_, case, ty)) => format!(".{case}({ty}())"),
        None => format!(
            ".default(tag: 0, body: {}())",
            default.as_deref().expect("a variant holds an arm")
        ),
    };
    let mut bodies: Vec<String> = Vec::new();
    for ty in arms.iter().map(|(_, _, ty)| ty).chain(default.as_ref()) {
        if !bodies.contains(ty) {
            bodies.push(ty.clone());
        }
    }
    let fewest_bytes = match bodies.as_slice() {
        [only] => format!("{only}.frameLength"),
        many => format!(
            "Swift.min({})",
            many.iter()
                .map(|ty| format!("{ty}.frameLength"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    Ok(VariantParts {
        field: serde_json::json!({
            "id": VARIANT_FIELD,
            "swift_type": enum_name,
            "offset": 0,
            "default": initial,
            "flags": Vec::<serde_json::Value>::new(),
            "bits": serde_json::Value::Null,
            "present": serde_json::Value::Null,
            "variant": {
                "tag": tag,
                "local": local,
                "cases": cases,
            },
        }),
        arms_enum: serde_json::json!({
            "name": enum_name,
            "cases": arms
                .iter()
                .map(|(arm, case, ty)| serde_json::json!({
                    "case": case,
                    "type": ty,
                    "tag": format!("0x{:X}", arm.value),
                }))
                .collect::<Vec<_>>(),
            "default": default.as_ref().map(|ty| serde_json::json!({
                "type": ty,
                "tag_type": tag_type,
            })),
        }),
        fewest_bytes,
    })
}

/// What an encode writes into the carrier `carrier` besides its stored value, when the arm of an
/// embedded variant fills flags of it (ADR 0058): the bits of the stored value it keeps, every bit
/// outside those flags, and, per embedded variant, the tag of the arm it holds shifted into its
/// flag. Each embedded variant is read by the arm it holds, as every backend reads it, so a
/// variant that finds its own tag on the wire is driven as one whose caller hands it the tag, and
/// a body held by the catch-all presents the tag it was read for. The shifted tags are the locals
/// [`parent_tag_locals`] declares, which an embed that is absent leaves at zero. `null` for a
/// carrier no embedded variant fills.
fn parent_tag_derived(derivations: &[ParentTagDerivation], carrier: &str) -> serde_json::Value {
    let mut keep: u64 = 0xFF;
    let mut terms = Vec::new();
    for (index, d) in derivations.iter().enumerate() {
        if d.carrier_field_id != carrier {
            continue;
        }
        let mask = (1u64 << d.width) - 1;
        keep &= !((mask << d.bit) & 0xFF);
        terms.push(parent_tag_local(index));
    }
    if terms.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::json!({
        "keep": format!("0x{keep:02X}"),
        "terms": terms.join(" | "),
    })
}

/// The local an encode keeps the bits the embedded variant of derivation `index` fills.
fn parent_tag_local(index: usize) -> String {
    format!("derived_tag_{index}")
}

/// The lines an encode opens with, one per derivation: the bits of the flag the arm that the
/// embedded variant holds names, in place in the carrier's byte, and zero for an embed a gate leaves
/// off the wire. A switch over the variant's arms declares the local without a value first, so the
/// compiler holds every arm to give it one.
fn parent_tag_locals(derivations: &[ParentTagDerivation]) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, d) in derivations.iter().enumerate() {
        let local = parent_tag_local(index);
        let mask = (1u64 << d.width) - 1;
        let embed = &d.embed_field_id;
        lines.push(format!("        let {local}: UInt8"));
        let (held, pad) = if d.present_if.is_some() {
            lines.push(format!("        if let held = self.{embed} {{"));
            ("held".to_string(), "    ")
        } else {
            (format!("self.{embed}"), "")
        };
        lines.push(format!("{pad}        switch {held}.{VARIANT_FIELD} {{"));
        for arm in &d.arms {
            let shifted = ((arm.value & mask) << d.bit) & 0xFF;
            lines.push(format!(
                "{pad}        case .{}: {local} = 0x{shifted:02X}",
                filters::to_camel_case(arm.body_alias.clone())
            ));
        }
        if d.catch_all {
            lines.push(format!(
                "{pad}        case .default(let tag, _): {local} = (UInt8(truncatingIfNeeded: tag) & 0x{mask:X}) << {}",
                d.bit
            ));
        }
        lines.push(format!("{pad}        }}"));
        if d.present_if.is_some() {
            lines.push("        } else {".to_string());
            lines.push(format!("            {local} = 0"));
            lines.push("        }".to_string());
        }
    }
    lines
}

/// What the template needs to write the property of one flag: its name, where its bits lie in
/// the carrier (`mask` is the bits shifted into place, `value_mask` the same bits at zero), and,
/// for a flag of more than one bit, the unsigned integer type that reads and writes it (the
/// smallest that holds its width). A flag of one bit is a `Bool`.
fn flag_context(flag: &FlagDef) -> serde_json::Value {
    let width = flag.width.max(1);
    let value_mask = if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let result_type = match width {
        1 => "Bool",
        2..=8 => "UInt8",
        9..=16 => "UInt16",
        17..=32 => "UInt32",
        _ => "UInt64",
    };
    serde_json::json!({
        "property": flag_property(&flag.name),
        "name": flag.name,
        "bit": flag.bit,
        "width": width,
        "multi_bit": width > 1,
        "mask": format!("0x{:X}", value_mask << flag.bit),
        "value_mask": format!("0x{value_mask:X}"),
        "result_type": result_type,
    })
}

/// What of the import `alias` of `m`, whose arm the importer drives into a flag of its own
/// (`<sce:variant-dispatch>`), the Swift codec does not lower: the dispatch is written where the
/// codec is embedded (the carrier of the flag is derived from the arm the embedded value holds), and
/// not for the entries of a list or a chain of that codec.
pub(super) fn dispatch_outside_an_embed(m: &CodecModel, alias: &str) -> Option<String> {
    m.fields
        .iter()
        .any(|f| {
            f.repeat_body_alias.as_deref() == Some(alias)
                || f.tlv_chain_body_alias.as_deref() == Some(alias)
        })
        .then(|| "its arm is driven into a flag of the codec that holds its entries".to_string())
}

/// The refusal of the codec `m` for the Swift construct `construct` it holds ([`unlowered`]).
pub(super) fn refusal(m: &CodecModel, construct: &str) -> String {
    format!("codec '{}': {construct} has no Swift lowering yet", m.name)
}

/// The Swift unit of the codec `m`, which [`unlowered`] has found nothing to refuse in.
pub(super) fn render(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    // A codec that writes one CBOR map has none of the positional codec's layout to render.
    if m.encoding == CodecEncoding::Cbor {
        return crate::forge::cbor_codec::render(env, m, imports, Language::Swift);
    }
    // So is a component of content lines (SCE_FORGE.md section 4.6.4), whose line grammar is the
    // runtime's.
    if m.encoding == CodecEncoding::ContentLine {
        return crate::forge::content_line_codec::render(env, m, imports, Language::Swift);
    }
    let l = LangCtx::new(crate::generator::Language::Swift, imports);
    let sequential = is_sequential(m);
    // The bits of an entry's value that are its identifier, for each chain that leaves flags out of it.
    let masks = chain_identifier_masks(m, imports);
    // The carriers whose flags an embedded variant's arm fills at encode (ADR 0058).
    let derivations = collect_parent_tag_derivations(m, imports);
    // The bytes of the frame: for fields at fixed places the end of the last of them, for a frame
    // read field by field the fewest bytes it can have, which is each integer at its width, each
    // variable-length integer at one byte, and the bytes of a variable-length field at none.
    let mut frame_length = 0;
    // The fewest bytes of each codec embedded where it is always on the wire: its own `frameLength`.
    let mut embedded_lengths: Vec<String> = Vec::new();
    // The struct each list's entries or embedded codec are, by the alias the field names its codec
    // by: the unit of an imported codec is a type of the same module, named as the document is.
    let mut element_types = std::collections::HashMap::new();
    for f in m
        .fields
        .iter()
        .filter(|f| f.is_repeat() || f.is_embed() || f.is_tlv_chain())
    {
        if let Some(alias) = f
            .repeat_body_alias
            .as_deref()
            .or(f.embed_body_alias.as_deref())
            .or(f.tlv_chain_body_alias.as_deref())
        {
            element_types.insert(
                f.id.clone(),
                resolve_repeat_body_type(&m.name, alias, imports, Language::Swift)?,
            );
            // An entry that says it is the last does so through a flag of its own carrier, which its
            // struct holds as a `Bool` when the flag is one bit wide.
            if let BitSize::TlvChain {
                terminate_on: TlvTerminateStrategy::EntryFlag { flag_name },
                ..
            } = &f.bit_size
            {
                let one_bit = imports
                    .iter()
                    .find(|import| import.alias == alias)
                    .is_some_and(|import| {
                        import
                            .codec_fields
                            .iter()
                            .flat_map(|g| g.flags.iter())
                            .any(|flag| flag.name == *flag_name && flag.width.max(1) == 1)
                    });
                if !one_bit {
                    return Err(ForgeError::from(GenerateError::unsupported(format!(
                        "codec '{}': field '{}': a chain closed by the flag `{flag_name}` of \
                         '{alias}', which is not a flag of one bit that codec carries has no \
                         Swift lowering yet",
                        m.name, f.id
                    ))));
                }
            }
        }
    }
    // What the codec an embed, a list or a variant names is handed for each flag input it declares,
    // after the cursor of a decode and the sink of an encode: the bit of a carrier of this codec, or
    // an input of its own handed on.
    let thread = |alias: &str| {
        let args = embed_flag_bind_thread_args(
            imports.iter().find(|import| import.alias == alias),
            m,
            Language::Swift,
        );
        (args.decode_arg, args.encode_arg)
    };
    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            // The condition under which a gated field is on the wire, as the decode of the frame
            // and the encode of the value read it; `null` for a field that always is. A gated
            // field is optional, `nil` when it is not on the wire.
            let present = f.present_if.as_ref().map(|p| {
                serde_json::json!({
                    "decode": present_if_condition(m, p, "frame"),
                    "encode": present_if_condition(m, p, "self"),
                })
            });
            let optional = if present.is_some() { "?" } else { "" };
            if f.is_embed() {
                let element = element_types
                    .get(&f.id)
                    .expect("an embedded codec this renderer is given names a codec")
                    .clone();
                if present.is_none() {
                    embedded_lengths.push(format!("{element}.frameLength"));
                }
                // The embedded codec is read from the bytes an earlier field says, when the
                // document bounds it so, and from the frame's own cursor otherwise.
                let length = f
                    .embed_length_from
                    .as_deref()
                    .map(|name| format!("Int(clamping: frame.{name})"));
                // The tag a variant takes from its caller is a flag of this frame when the import
                // declares which, and so is handed to the decode after the flag inputs.
                let embed_args = embed_parent_flags_thread_args(
                    f,
                    f.embed_body_alias.as_deref().unwrap_or_default(),
                    imports,
                    m,
                    Language::Swift,
                );
                let (decode_args, encode_args) = (embed_args.decode_arg, embed_args.encode_arg);
                return serde_json::json!({
                    "id": f.id,
                    "swift_type": format!("{element}{optional}"),
                    "offset": f.byte_offset,
                    "default": if present.is_some() { "nil".to_string() } else { format!("{element}()") },
                    "flags": Vec::<serde_json::Value>::new(),
                    "bits": serde_json::Value::Null,
                    "present": present,
                    "embed": {
                        "element": element,
                        "length": length,
                        "inner": format!("{}_inner", f.id),
                        "decode_args": decode_args,
                        "encode_args": encode_args,
                    },
                });
            }
            if let BitSize::TlvChain {
                max_depth,
                terminate_on,
                ..
            } = &f.bit_size
            {
                let element = element_types
                    .get(&f.id)
                    .expect("a chain this renderer is given names an entry codec")
                    .clone();
                // An input the chain supplies is a local of its own, which the loop declares before
                // the first entry, hands to each entry, and rewrites from the entry it has just handled.
                let (decode_args, encode_args) = {
                    let alias = f.tlv_chain_body_alias.as_deref().unwrap_or_default();
                    let args = embed_flag_bind_thread_args_supplying(
                        imports.iter().find(|import| import.alias == alias),
                        m,
                        Language::Swift,
                        |input| ChainEntryBinds::local_of(f, input),
                    );
                    (args.decode_arg, args.encode_arg)
                };
                let binds = ChainEntryBinds::of(f, &masks);
                let pad = if present.is_some() { 4 } else { 0 };
                let at = |extra: usize| " ".repeat(8 + pad + extra);
                // What the codec's gates ask of the chain's entries, bound once the chain is read.
                let has_decode: Vec<String> = chain_has_values(&m.fields, &f.id)
                    .into_iter()
                    .flat_map(|value| {
                        chain_has_decl_lines(
                            f,
                            value,
                            &masks,
                            Language::Swift,
                            PresentIfSite::Decode,
                            None,
                        )
                    })
                    .collect();
                let accessor = match terminate_on {
                    TlvTerminateStrategy::ExhaustOrDepth => serde_json::Value::Null,
                    TlvTerminateStrategy::EntryFlag { flag_name } => {
                        serde_json::Value::String(flag_property(flag_name))
                    }
                };
                return serde_json::json!({
                    "id": f.id,
                    "swift_type": format!("[{element}]{optional}"),
                    "offset": f.byte_offset,
                    "default": if present.is_some() { "nil" } else { "[]" },
                    "flags": Vec::<serde_json::Value>::new(),
                    "bits": serde_json::Value::Null,
                    "present": present,
                    "chain": {
                        "element": element,
                        "items": format!("{}_items", f.id),
                        "more": format!("{}_more", f.id),
                        "max_depth": max_depth,
                        "accessor": accessor,
                        "decode_args": decode_args,
                        "encode_args": encode_args,
                        "decls": binds.decls(Language::Swift, &at(0)),
                        "updates": binds.update_lines(f, "entry", Language::Swift, &at(4)),
                        "has_decode": has_decode,
                    },
                });
            }
            if let BitSize::Repeat { count_ref } = &f.bit_size {
                let element = element_types
                    .get(&f.id)
                    .expect("a list this renderer is given names an entry codec")
                    .clone();
                // The list is read into a local and set when it is whole, so a gated list, an
                // optional on the frame, is not appended to through the optional.
                let items = format!("{}_items", f.id);
                let (decode_args, encode_args) =
                    thread(f.repeat_body_alias.as_deref().unwrap_or_default());
                let pad = if present.is_some() { 4 } else { 0 };
                let at = |extra: usize| " ".repeat(8 + pad + extra);
                let list = match count_ref {
                    CountRef::LengthField(name) => {
                        // A count that is on the wire only on a condition is an optional; it is
                        // read inside the gate that holds the list to the same predicate, where
                        // it is present.
                        let count = carried_length_expr(m, name);
                        serde_json::json!({
                            "kind": "count",
                            "count_field": name,
                            "element": element,
                            "decode_args": decode_args,
                            "encode_args": encode_args,
                            "items": items,
                            "count": count,
                            "guard": repeat_count_guard(f, &m.fields, &count, Language::Swift, &at(0)),
                        })
                    }
                    CountRef::UntilEof => serde_json::json!({
                        "kind": "until_eof",
                        "element": element,
                        "decode_args": decode_args,
                        "encode_args": encode_args,
                        "items": items,
                        "room": repeat_room_guard(
                            f,
                            &m.fields,
                            &format!("{items}.count"),
                            Language::Swift,
                            &at(4),
                        ),
                    }),
                };
                return serde_json::json!({
                    "id": f.id,
                    "swift_type": format!("[{element}]{optional}"),
                    "offset": f.byte_offset,
                    "default": if present.is_some() { "nil" } else { "[]" },
                    "flags": Vec::<serde_json::Value>::new(),
                    "bits": serde_json::Value::Null,
                    "present": present,
                    "list": list,
                });
            }
            if is_variable(f) {
                let kind = match f.bit_size {
                    BitSize::Tail => "tail",
                    _ => "length_ref",
                };
                // The check of the length the frame says against the bound the document
                // declares, made before the bytes are cut so a frame that says too many is
                // refused and never waited on (docs/adr/0077 §2). A `tail` says its length by
                // the bytes that are left.
                let (carried, held) = match &f.length_field {
                    Some(name) => (
                        carried_length_expr(m, name),
                        length_guard::carried_held(name, &m.fields, Language::Swift),
                    ),
                    None => ("reader.remaining".to_string(), length_guard::Held::HostInt),
                };
                let text = matches!(f.sce_type, SceType::String);
                let indent = if present.is_some() {
                    "            "
                } else {
                    "        "
                };
                let guard = length_guard::Guard::of(f, &m.fields)
                    .map(|g| g.stmt(Language::Swift, &carried, held, indent))
                    .unwrap_or_default();
                let (value_type, empty) = if text {
                    ("String", "\"\"")
                } else {
                    ("[UInt8]", "[]")
                };
                return serde_json::json!({
                    "id": f.id,
                    "swift_type": format!("{value_type}{optional}"),
                    "offset": f.byte_offset,
                    "default": if present.is_some() { "nil" } else { empty },
                    "flags": Vec::<serde_json::Value>::new(),
                    "bits": serde_json::Value::Null,
                    "present": present,
                    "variable": {
                        "kind": kind,
                        "text": text,
                        "length_field": f.length_field.clone().unwrap_or_default(),
                        "carried": carried,
                        "arith": f.length_arith.unwrap_or(0),
                        "guard": guard,
                    },
                });
            }
            let (swift_type, width) =
                integer(&f.sce_type).expect("a field this renderer is given is an integer");
            if present.is_some() {
                // On the wire only on a condition: it adds nothing to the fewest bytes.
            } else if f.is_vle() {
                frame_length += 1;
            } else if sequential {
                frame_length += width / 8;
            } else {
                frame_length = frame_length.max(f.byte_offset + width / 8);
            }
            let order = match f.effective_endian(m.default_endian) {
                Endian::Little => "little",
                Endian::Big | Endian::Native => "big",
            };
            // The carrier's value before anything is read into it: the bits the document's
            // `<sce:flag value=>` declarations set, and zero where it declares none.
            let default = match flag_default_carrier_value(f) {
                _ if present.is_some() => "nil".to_string(),
                0 => "0".to_string(),
                bits => format!("0x{bits:X}"),
            };
            serde_json::json!({
                "id": f.id,
                "swift_type": format!("{swift_type}{optional}"),
                "value_type": swift_type,
                "present": present,
                "offset": f.byte_offset,
                "order": order,
                "default": default,
                "vle": f.is_vle(),
                "flags": f.flags.iter().map(flag_context).collect::<Vec<_>>(),
                "derived": parent_tag_derived(&derivations, &f.id),
                // Where its bits lie in the byte, for a field of bits inside one: `null` for a
                // field that is the whole of its integer.
                "bits": bit_field(f).ok().filter(|_| f.bit_offset.is_some()).map(|(shift, mask)| {
                    serde_json::json!({
                        "shift": shift,
                        "mask": format!("0x{mask:X}"),
                        "size": mask.count_ones(),
                    })
                }),
            })
        })
        .collect();
    let mut fields = fields;
    let mut ctx = l.base_context(&m.name);
    // The arm a tag selects is the last of the fields, and its enum is declared before the struct.
    if let Some(variant) = &m.variant {
        let parts = variant_context(m, variant, imports, &thread)?;
        fields.push(parts.field);
        embedded_lengths.push(parts.fewest_bytes);
        ctx.insert("variant_enum".into(), parts.arms_enum);
    }
    ctx.insert("fields".into(), fields.into());
    ctx.insert(
        "parent_tag_locals".into(),
        parent_tag_locals(&derivations).into(),
    );
    // The refusals an encode makes before its first byte: of a field and the gate it is held to that
    // contradict each other, and of a value outside its bound.
    let roles = crate::forge::codec_roles::classify(m, imports);
    let mismatches = presence_mismatch_checks(m, &roles.checks);
    // The value is asked what the gates asked of the frame, once, before a check reads the answer;
    // when no check does, the answer is not bound, and the compiler has no unused value to warn of.
    let mut bound_checks: Vec<String> = Vec::new();
    if !mismatches.is_empty() {
        for chain in m.fields.iter().filter(|f| f.is_tlv_chain()) {
            for value in chain_has_values(&m.fields, &chain.id) {
                bound_checks.extend(chain_has_decl_lines(
                    chain,
                    value,
                    &masks,
                    Language::Swift,
                    PresentIfSite::Encode,
                    None,
                ));
            }
        }
    }
    bound_checks.extend(mismatches);
    bound_checks.extend(encode_bound_guard_lines(&m.fields, Language::Swift).0);
    ctx.insert("bound_checks".into(), bound_checks.into());
    // The flag inputs the codec is handed by whoever decodes or encodes it, in declared order.
    ctx.insert(
        "flag_params".into(),
        m.flag_inputs
            .iter()
            .map(|input| input.name.clone())
            .collect::<Vec<_>>()
            .into(),
    );
    // A variant with no tag of its own is told which arm the frame holds by its caller, as the last
    // argument of a decode; an encode is told nothing, the arm that is held says which it is.
    ctx.insert(
        "caller_tag".into(),
        m.variant
            .as_ref()
            .is_some_and(|v| v.tag_field.is_none())
            .into(),
    );
    // A number of bytes and each embedded codec's own, summed: a constant of the codec's unit.
    let frame_length = match (frame_length, embedded_lengths.is_empty()) {
        (n, true) => n.to_string(),
        (0, false) => embedded_lengths.join(" + "),
        (n, false) => format!("{n} + {}", embedded_lengths.join(" + ")),
    };
    ctx.insert("frame_length".into(), frame_length.into());
    // Whether the frame is read and written field by field (`is_sequential`), each field beginning
    // where the last one ended, and not at the byte its field names.
    ctx.insert("sequential".into(), sequential.into());
    l.render(env, "codec", ctx)
}
