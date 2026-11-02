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
pub(super) fn unlowered(m: &CodecModel) -> Option<String> {
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
        .find_map(|f| field_unlowered(m, f))
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
fn field_unlowered(m: &CodecModel, f: &CodecField) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    if f.present_if.is_some() {
        return what("a `sce:present-if` condition");
    }
    if f.is_repeat() {
        return what("a repeat");
    }
    if f.is_tlv_chain() {
        return what("a TLV chain");
    }
    if f.is_embed() {
        return what("an embedded codec");
    }
    if f.is_vle() {
        return what("a variable-length integer");
    }
    if f.dma_burst_align.is_some() {
        return what("a DMA alignment");
    }
    if matches!(f.bit_size, BitSize::Tail | BitSize::LengthRef) {
        return variable_field_unlowered(m, f);
    }
    if f.is_variable_length() || f.length_field.is_some() {
        return what("a variable-length field");
    }
    // A field of bits inside one byte has no byte order: it is the byte's own bits.
    if f.bit_offset.is_some() {
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

/// Whether `f` is the one field of `m` that runs to the end of the frame or for the length an
/// earlier field states: the bytes the frame carries after its fixed part.
fn is_variable(f: &CodecField) -> bool {
    matches!(f.bit_size, BitSize::Tail | BitSize::LengthRef)
}

/// What of the variable-length field `f` of `m` (a `tail` or a `length-ref`) the Swift codec does
/// not lower yet; `None` for one it does.
///
/// It is lowered where it is the last of the frame and a `bytes` field: the fixed fields come
/// before it, in front of the byte it begins at, and the bytes it holds are an array. A `length-ref`
/// takes its count from an unsigned integer field of the codec, plus the `length-arith` the
/// document names. Each shape past that (a string, a field after it, a second one, a length held
/// in the flag of a carrier or in a signed field) is named.
fn variable_field_unlowered(m: &CodecModel, f: &CodecField) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    if !matches!(f.sce_type, SceType::Bytes) {
        return what(&format!(
            "a `{}` field of variable length",
            f.sce_type.as_attr()
        ));
    }
    if m.fields.iter().filter(|g| is_variable(g)).count() > 1 {
        return what("a second field of variable length in the frame");
    }
    let fixed = m.fields.iter().filter(|g| !std::ptr::eq(*g, f));
    for g in fixed {
        let end =
            integer(&g.sce_type).map_or(g.byte_offset, |(_, width)| g.byte_offset + width / 8);
        if g.byte_offset >= f.byte_offset || end > f.byte_offset {
            return what(&format!(
                "the field '{}' that follows or overlaps it in the frame",
                g.id
            ));
        }
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
    }
    None
}

/// The variable-length field of `m`, which [`unlowered`] has found nothing to refuse in.
fn variable_field(m: &CodecModel) -> Option<&CodecField> {
    m.fields.iter().find(|f| is_variable(f))
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
    let mut frame_length = 0;
    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            // The bytes after the fixed part of the frame: they begin where the document says,
            // and the fixed part is the frame up to there.
            if is_variable(f) {
                frame_length = frame_length.max(f.byte_offset);
                let kind = match f.bit_size {
                    BitSize::Tail => "tail",
                    _ => "length_ref",
                };
                return serde_json::json!({
                    "id": f.id,
                    "swift_type": "[UInt8]",
                    "offset": f.byte_offset,
                    "default": "[]",
                    "flags": Vec::<serde_json::Value>::new(),
                    "bits": serde_json::Value::Null,
                    "variable": {
                        "kind": kind,
                        "length_field": f.length_field.clone().unwrap_or_default(),
                        "arith": f.length_arith.unwrap_or(0),
                    },
                });
            }
            let (swift_type, width) =
                integer(&f.sce_type).expect("a field this renderer is given is an integer");
            frame_length = frame_length.max(f.byte_offset + width / 8);
            let order = match f.effective_endian(m.default_endian) {
                Endian::Little => "little",
                Endian::Big | Endian::Native => "big",
            };
            // The carrier's value before anything is read into it: the bits the document's
            // `<sce:flag value=>` declarations set, and zero where it declares none.
            let default = match flag_default_carrier_value(f) {
                0 => "0".to_string(),
                bits => format!("0x{bits:X}"),
            };
            serde_json::json!({
                "id": f.id,
                "swift_type": swift_type,
                "offset": f.byte_offset,
                "order": order,
                "default": default,
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
    ctx.insert("frame_length".into(), frame_length.into());
    // Whether a field runs on past the fixed part of the frame: a decode then builds the value
    // field by field, as the length of that field is read from another.
    ctx.insert("has_variable".into(), variable_field(m).is_some().into());
    l.render(env, "codec", ctx)
}
