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
    if m.fields.is_empty() {
        return Some("a codec with no field".to_string());
    }
    m.fields.iter().find_map(|f| field_unlowered(m, f))
}

/// What of the field `f` of `m` the Swift codec does not lower yet.
fn field_unlowered(m: &CodecModel, f: &CodecField) -> Option<String> {
    let what = |text: &str| Some(format!("field '{}': {text}", f.id));
    if !f.flags.is_empty() {
        return what("a `<sce:flags>` carrier");
    }
    if f.present_if.is_some() {
        return what("a `sce:present-if` condition");
    }
    if f.bit_offset.is_some() {
        return what("a bit offset");
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
    if f.is_variable_length() || f.length_field.is_some() {
        return what("a variable-length field");
    }
    if f.dma_burst_align.is_some() {
        return what("a DMA alignment");
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
            let (swift_type, width) =
                integer(&f.sce_type).expect("a field this renderer is given is an integer");
            frame_length = frame_length.max(f.byte_offset + width / 8);
            let order = match f.effective_endian(m.default_endian) {
                Endian::Little => "little",
                Endian::Big | Endian::Native => "big",
            };
            serde_json::json!({
                "id": f.id,
                "swift_type": swift_type,
                "offset": f.byte_offset,
                "order": order,
            })
        })
        .collect();
    let mut ctx = l.base_context(&m.name);
    ctx.insert("fields".into(), fields.into());
    ctx.insert("frame_length".into(), frame_length.into());
    l.render(env, "codec", ctx)
}
