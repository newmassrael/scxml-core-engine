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
        CodecEncoding::Cbor => return Some("a `sce:encoding=\"cbor\"` codec".to_string()),
        CodecEncoding::ContentLine => {
            return Some("a `sce:encoding=\"content-line\"` codec".to_string())
        }
    }
    if m.variant.is_some() {
        return Some("a `<sce:variant>`".to_string());
    }
    if !m.flag_inputs.is_empty() {
        return Some("a flag input".to_string());
    }
    // A codec with no field is a frame of no bytes: what a body whose tag is written outside it
    // carries (`codec_zenoh_keep_alive`).
    m.fields
        .iter()
        .find_map(|f| field_unlowered(m, f, element))
        .or_else(|| flags_unlowered(m))
}

/// The members a codec struct has besides its fields and the flags its fields carry.
const MEMBERS: &[&str] = &["frameLength", "init", "decode", "encode"];

/// The property that reads and writes the flag `name` of a carrier: lowerCamel, as a Swift
/// property is spelled.
fn flag_property(name: &str) -> String {
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
    if let Some(predicate) = &f.present_if {
        if let Some(reason) = present_if_unlowered(predicate) {
            return what(&reason);
        }
        if !f.flags.is_empty() {
            return what("a flags carrier that is present only on a condition");
        }
    }
    if let BitSize::Repeat { count_ref } = &f.bit_size {
        return repeat_unlowered(m, f, count_ref, element);
    }
    if f.is_tlv_chain() {
        return what("a TLV chain");
    }
    if f.is_embed() {
        return what("an embedded codec");
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
    m.fields
        .iter()
        .any(|f| f.is_vle() || is_variable(f) || f.is_repeat() || f.present_if.is_some())
}

/// What of the predicate `predicate` the Swift codec does not lower yet: a clause on a flag input
/// reads a parameter the codec does not take, and a clause on the entries of a chain reads a chain
/// the codec does not hold. The carrier a local clause names, and its flag, exist and are unsigned
/// integers: validation has refused a codec where they do not.
fn present_if_unlowered(predicate: &PresentIfPredicate) -> Option<String> {
    if predicate.scope != PresentIfScope::Local {
        return Some("a `sce:present-if` on a flag input or on a chain".to_string());
    }
    predicate.or_with.as_deref().and_then(present_if_unlowered)
}

/// The Swift condition under which a field gated by `predicate` is on the wire: the bit a flag of
/// an earlier carrier names is set (or clear, for a negated clause), and any clause of a
/// disjunction holds. `holder` names the value whose carriers are read: the frame being decoded,
/// or `self` when it is encoded. The mask is the one every backend tests, from the shared
/// resolution of the predicate. [`present_if_unlowered`] has found nothing to refuse in it.
fn present_if_condition(m: &CodecModel, predicate: &PresentIfPredicate, holder: &str) -> String {
    let (mask, _, carrier) = present_if_carrier_info(&m.fields, predicate);
    let PresentIfCarrier::Local(carrier) = carrier else {
        unreachable!("a clause on a flag input is refused before its condition is written");
    };
    let test = format!(
        "({holder}.{} & 0x{mask:X}) {} 0",
        carrier.id,
        if predicate.negate { "==" } else { "!=" }
    );
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
/// It is a `bytes` field, held as an array. A `length-ref` takes its count from an unsigned integer
/// field of the codec, plus the `length-arith` the document names. Each shape past that (a string,
/// a length held in the flag of a carrier or in a signed field) is named.
fn variable_field_unlowered(m: &CodecModel, f: &CodecField) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    if !matches!(f.sce_type, SceType::Bytes) {
        return what(&format!(
            "a `{}` field of variable length",
            f.sce_type.as_attr()
        ));
    }
    if matches!(f.bit_size, BitSize::LengthRef) {
        let Some(name) = f.length_field.as_deref() else {
            return what("a length-ref with no length field");
        };
        if dotted_length_field(name).is_some() {
            return what("a length held in a flag of a carrier");
        }
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
            .any(|g| g.id == name && g.present_if.is_some())
        {
            return what(&format!(
                "a length held in `{name}`, which is on the wire only on a condition"
            ));
        }
    }
    None
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
    let l = LangCtx::new(crate::generator::Language::Swift, imports);
    let sequential = is_sequential(m);
    // The bytes of the frame: for fields at fixed places the end of the last of them, for a frame
    // read field by field the fewest bytes it can have, which is each integer at its width, each
    // variable-length integer at one byte, and the bytes of a variable-length field at none.
    let mut frame_length = 0;
    // The struct each list's entries are, by the alias the list names its entry codec by: the
    // unit of an imported codec is a type of the same module, named as the document is.
    let mut element_types = std::collections::HashMap::new();
    for f in m.fields.iter().filter(|f| f.is_repeat()) {
        if let Some(alias) = f.repeat_body_alias.as_deref() {
            element_types.insert(
                f.id.clone(),
                resolve_repeat_body_type(&m.name, alias, imports, Language::Swift)?,
            );
        }
    }
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
            if let BitSize::Repeat { count_ref } = &f.bit_size {
                let element = element_types
                    .get(&f.id)
                    .expect("a list this renderer is given names an entry codec")
                    .clone();
                // The list is read into a local and set when it is whole, so a gated list, an
                // optional on the frame, is not appended to through the optional.
                let items = format!("{}_items", f.id);
                let pad = if present.is_some() { 4 } else { 0 };
                let at = |extra: usize| " ".repeat(8 + pad + extra);
                let list = match count_ref {
                    CountRef::LengthField(name) => {
                        // A count that is on the wire only on a condition is an optional; it is
                        // read inside the gate that holds the list to the same predicate, where
                        // it is present.
                        let count = if m.fields.iter().any(|g| g.id == *name && g.present_if.is_some()) {
                            format!("(frame.{name} ?? 0)")
                        } else {
                            format!("frame.{name}")
                        };
                        serde_json::json!({
                            "kind": "count",
                            "count_field": name,
                            "element": element,
                            "items": items,
                            "count": count,
                            "guard": repeat_count_guard(f, &m.fields, &count, Language::Swift, &at(0)),
                        })
                    }
                    CountRef::UntilEof => serde_json::json!({
                        "kind": "until_eof",
                        "element": element,
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
                        format!("frame.{name}"),
                        length_guard::carried_held(name, &m.fields, Language::Swift),
                    ),
                    None => ("reader.remaining".to_string(), length_guard::Held::HostInt),
                };
                let indent = if present.is_some() {
                    "            "
                } else {
                    "        "
                };
                let guard = length_guard::Guard::of(f, &m.fields)
                    .map(|g| g.stmt(Language::Swift, &carried, held, indent))
                    .unwrap_or_default();
                return serde_json::json!({
                    "id": f.id,
                    "swift_type": format!("[UInt8]{optional}"),
                    "offset": f.byte_offset,
                    "default": if present.is_some() { "nil" } else { "[]" },
                    "flags": Vec::<serde_json::Value>::new(),
                    "bits": serde_json::Value::Null,
                    "present": present,
                    "variable": {
                        "kind": kind,
                        "length_field": f.length_field.clone().unwrap_or_default(),
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
    let mut ctx = l.base_context(&m.name);
    ctx.insert("fields".into(), fields.into());
    // The refusals an encode makes before its first byte, of a value outside its bound.
    let (bound_checks, _) = encode_bound_guard_lines(&m.fields, Language::Swift);
    ctx.insert("bound_checks".into(), bound_checks.into());
    ctx.insert("frame_length".into(), frame_length.into());
    // Whether the frame is read and written field by field (`is_sequential`), each field beginning
    // where the last one ended, and not at the byte its field names.
    ctx.insert("sequential".into(), sequential.into());
    l.render(env, "codec", ctx)
}
