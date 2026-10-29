// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The largest number of bytes a codec document encodes to, as a constant of
//! the build (ADR 0062, section 4).
//!
//! A slot of a broadcast log holds the wire bytes of its element as a fixed
//! number of words, so the generator needs a size no value of the element can
//! exceed. [`largest_encoded_bytes`] answers it for the three wire forms a
//! codec has, or says why no such size exists ([`Unbounded`]).
//!
//! ⚠ SOUND BY REFUSAL. A bound that a value can exceed is a defect, because the
//! slot would hold part of a frame; a refusal is never one. So nothing here takes
//! a default: a variable field with no `sce:max-size`, a list with no
//! `max-count`, an import that does not resolve and a codec that imports itself
//! are all refusals, where the buffer-sizing figure of
//! [`CodecModel::max_frame_bytes`] falls back to a default cap (`limits`) and
//! leaves an embedded body out. That figure sizes a hint; this one sizes a slot.
//!
//! The bound is the size of what the encoder writes, read from the encoders:
//! - the positional path (`CodecModel::positional_layout_is_valid`) writes the
//!   fixed fields by ascending byte offset, one byte for the sub-byte fields
//!   that share an offset, and a trailing payload after the padding a
//!   `sce:dma-burst-align` codec zero-fills up to the payload's own offset;
//! - the streaming path writes each field in declaration order: a fixed field
//!   at the bytes its width rounds up to, a VLE integer at its cap, a payload at
//!   its `sce:max-size`, a list at its count times the body, an embedded body or
//!   the body of the arm a variant holds at that codec's own bound;
//! - a CBOR map and a content-line component are bounded by the arithmetic
//!   their C11 renderers already carry for `MAX_ENCODED_BYTES`, which this
//!   module calls and does not repeat.
//!
//! An optional field (`sce:present-if`, `optional<T>`) counts at its full width:
//! the bound is of the largest frame, the one with every optional present.
//!
//! The module is pure. The file graph that resolves a codec's imports is read
//! by the caller (`validate_broadcast_log_cross_refs`), which hands over a
//! [`SizedCodec`] with every import it could read already resolved.

use std::collections::BTreeMap;
use std::fmt;

use crate::forge::model::{
    vle_cap_bytes, BitSize, CodecEncoding, CodecField, CodecModel, ForgeKind, SceType,
};

/// A codec document together with what each of its imports resolved to, so that
/// its size can be judged without touching a file.
#[derive(Debug, Clone)]
pub struct SizedCodec {
    pub model: CodecModel,
    /// What each `<sce:import>` alias of the document names, for the aliases
    /// that name a readable document of the kind the size needs. An alias that
    /// is absent did not resolve (no such file, another kind, or a cycle of
    /// imports): a codec that needs it has no bound.
    pub imports: BTreeMap<String, SizedImport>,
}

/// What an import contributes to the size of the codec that names it.
#[derive(Debug, Clone)]
pub enum SizedImport {
    /// A codec held as an embedded body, a list entry or a variant arm.
    Codec(Box<SizedCodec>),
    /// An enum a content-line entry is written as: the text of a variant, so
    /// the length in bytes of the longest of its texts (docs/adr/0015).
    Enum { longest_text: u64 },
}

/// Why a codec has no encoded size that is a constant of the build. It reads as
/// the clause that follows "is not a build-time constant: " in
/// `broadcast-log/element-size-unbounded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unbounded {
    /// The document has no wire form at all.
    NoWireForm(String),
    /// A declaration the bound needs is not written.
    Missing {
        /// The field or entry, and how it is declared.
        subject: String,
        /// The attribute that would bound it, as written in the document.
        declaration: &'static str,
    },
    /// An import the bound needs does not resolve to a readable document of the
    /// kind it needs, or closes a cycle of imports.
    Unresolved {
        /// The field, entry or arm that names the import.
        site: String,
        alias: String,
    },
    /// A codec held by this one has no bound.
    Within {
        site: String,
        alias: String,
        inner: Box<Unbounded>,
    },
}

impl Unbounded {
    /// A document of a kind that is not a codec: it writes no bytes of its own
    /// (a procedure is a state machine; only a codec has a wire form).
    pub fn no_wire_form(kind: ForgeKind) -> Self {
        Self::NoWireForm(format!(
            "a {kind} document has no wire form, so it has no encoded size"
        ))
    }
}

impl fmt::Display for Unbounded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoWireForm(why) => f.write_str(why),
            Self::Missing {
                subject,
                declaration,
            } => write!(f, "{subject} has no {declaration}"),
            Self::Unresolved { site, alias } => write!(
                f,
                "{site} names the import '{alias}', which is not a readable document of the \
                 kind needed here or closes a cycle of imports (a recursive codec has no \
                 largest size)"
            ),
            Self::Within { site, alias, inner } => write!(
                f,
                "{site} holds '{alias}', whose own size is unbounded: {inner}"
            ),
        }
    }
}

/// The most bytes any value of `codec` encodes to, or why there is no such
/// number. Saturating arithmetic keeps an absurd declaration a bound (the
/// largest `u64`) and never a wrapped, smaller one.
pub fn largest_encoded_bytes(codec: &SizedCodec) -> Result<u64, Unbounded> {
    match codec.model.encoding {
        CodecEncoding::Positional => positional_codec(codec),
        CodecEncoding::Cbor => cbor_codec(&codec.model),
        CodecEncoding::ContentLine => content_line_codec(codec),
    }
}

/// A CBOR map: the arithmetic of the C11 renderer, once every string and byte
/// string entry has a bound to take.
fn cbor_codec(m: &CodecModel) -> Result<u64, Unbounded> {
    for entry in &m.cbor_entries {
        let shape = match entry.present_type() {
            SceType::String => "string",
            SceType::Bytes => "bytes",
            _ => continue,
        };
        if entry.length.or(entry.max_size).is_none() {
            return Err(Unbounded::Missing {
                subject: format!("entry '{}' (a {shape} entry)", entry.id),
                declaration: "sce:length or sce:max-size",
            });
        }
    }
    Ok(crate::forge::cbor_codec::max_encoded_bytes(m))
}

/// A content-line component: the arithmetic of the C11 renderer, over the
/// longest text of each enum the codec imports. The parser already holds every
/// string entry to a `sce:max-size` and every list of values to a
/// `sce:max-values`, so what can still be missing here is an enum whose
/// document did not resolve.
fn content_line_codec(codec: &SizedCodec) -> Result<u64, Unbounded> {
    let Some(component) = codec.model.content_line.as_ref() else {
        return Err(Unbounded::NoWireForm(
            "the codec is written as content lines and declares no component".to_string(),
        ));
    };
    for entry in &component.entries {
        match entry.present_type() {
            SceType::Enum(reference) => {
                if !matches!(
                    codec.imports.get(&reference.alias),
                    Some(SizedImport::Enum { .. })
                ) {
                    return Err(Unbounded::Unresolved {
                        site: format!("entry '{}'", entry.id),
                        alias: reference.alias.clone(),
                    });
                }
            }
            SceType::String if entry.max_size.is_none() => {
                return Err(Unbounded::Missing {
                    subject: format!("entry '{}' (a string entry)", entry.id),
                    declaration: "sce:max-size",
                });
            }
            _ => {}
        }
    }
    let longest_text = |alias: &str| match codec.imports.get(alias) {
        Some(SizedImport::Enum { longest_text }) => *longest_text,
        _ => 0,
    };
    Ok(crate::forge::content_line_codec::max_encoded_bytes_with(
        &codec.model,
        &longest_text,
    ))
}

/// A codec of byte-positioned fields: the fields in wire order, then the arm a
/// variant holds.
fn positional_codec(codec: &SizedCodec) -> Result<u64, Unbounded> {
    let m = &codec.model;
    let mut total = if m.positional_layout_is_valid() {
        positional_fields(m)?
    } else {
        streamed_fields(codec)?
    };
    if let Some(variant) = &m.variant {
        // Exactly one arm is written, so the codec is bounded by its largest.
        let mut widest = 0u64;
        for arm in &variant.arms {
            let site = format!("the variant arm for tag value {}", arm.value);
            widest = widest.max(body_bytes(codec, &site, &arm.body_alias)?);
        }
        if let Some(arm) = &variant.default_arm {
            widest = widest.max(body_bytes(
                codec,
                "the default variant arm",
                &arm.body_alias,
            )?);
        }
        total = total.saturating_add(widest);
    }
    Ok(total)
}

/// The positional encode: the fixed fields grouped by byte offset in ascending
/// order, then the payload.
fn positional_fields(m: &CodecModel) -> Result<u64, Unbounded> {
    let mut groups: BTreeMap<u32, Vec<&CodecField>> = BTreeMap::new();
    for field in m.fields.iter().filter(|f| !f.is_variable_length()) {
        groups.entry(field.byte_offset).or_default().push(field);
    }
    // A group of one field writes the bytes its width rounds up to (a sub-byte
    // or bit-offset field, one); several fields at one offset share one byte.
    let mut written: u64 = groups
        .values()
        .map(|group| match group.as_slice() {
            [field] => {
                let bits = field.fixed_bits().unwrap_or(8);
                if bits < 8 || field.bit_offset.unwrap_or(0) > 0 {
                    1
                } else {
                    u64::from(bits.div_ceil(8))
                }
            }
            _ => 1,
        })
        .sum();
    for field in m.fields.iter().filter(|f| f.is_variable_length()) {
        // A codec with a `sce:dma-burst-align` field zero-fills up to the offset
        // of each payload before writing it.
        if m.has_dma_aligned_fields() {
            written = written.max(u64::from(field.byte_offset));
        }
        written = written.saturating_add(payload_bytes(field)?);
    }
    Ok(written)
}

/// The streaming encode: each field in declaration order.
fn streamed_fields(codec: &SizedCodec) -> Result<u64, Unbounded> {
    let mut total = 0u64;
    for field in &codec.model.fields {
        let site = format!("field '{}'", field.id);
        let bytes = match &field.bit_size {
            BitSize::Fixed { bits } => u64::from(bits.div_ceil(8)),
            BitSize::Tail | BitSize::LengthRef => payload_bytes(field)?,
            BitSize::Vle { width_bits } => u64::from(vle_cap_bytes(*width_bits)),
            BitSize::Repeat { .. } => {
                let count = field.max_count.ok_or_else(|| Unbounded::Missing {
                    subject: format!("field '{}' (a <sce:repeat>)", field.id),
                    declaration: "max-count",
                })?;
                let alias = field.repeat_body_alias.as_deref().unwrap_or_default();
                body_bytes(codec, &site, alias)?.saturating_mul(u64::from(count))
            }
            BitSize::TlvChain { max_depth, .. } => {
                let alias = field.tlv_chain_body_alias.as_deref().unwrap_or_default();
                body_bytes(codec, &site, alias)?.saturating_mul(u64::from(*max_depth))
            }
            BitSize::Embed => {
                let alias = field.embed_body_alias.as_deref().unwrap_or_default();
                body_bytes(codec, &site, alias)?
            }
        };
        total = total.saturating_add(bytes);
    }
    Ok(total)
}

/// A `tail` or `length-ref` payload at its declared `sce:max-size`.
fn payload_bytes(field: &CodecField) -> Result<u64, Unbounded> {
    field
        .max_size
        .map(u64::from)
        .ok_or_else(|| Unbounded::Missing {
            subject: format!(
                "field '{}' (sce:bit-size=\"{}\")",
                field.id,
                field.bit_size.as_attr()
            ),
            declaration: "sce:max-size",
        })
}

/// The bound of the codec the import `alias` names, held at `site`.
fn body_bytes(codec: &SizedCodec, site: &str, alias: &str) -> Result<u64, Unbounded> {
    match codec.imports.get(alias) {
        Some(SizedImport::Codec(body)) => {
            largest_encoded_bytes(body).map_err(|inner| Unbounded::Within {
                site: site.to_string(),
                alias: alias.to_string(),
                inner: Box::new(inner),
            })
        }
        _ => Err(Unbounded::Unresolved {
            site: site.to_string(),
            alias: alias.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::ForgeDocument;
    use crate::forge::parser::parse_forge;
    use crate::DocumentLabel;

    /// A codec document: `attrs` follow `sce:kind="codec"` on the root, `imports`
    /// are the `<sce:import>` lines and `body` the data model's elements.
    fn codec_xml(attrs: &str, imports: &str, body: &str) -> String {
        format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" {attrs} name="sized" version="1.0">
{imports}
  <datamodel>
{body}
  </datamodel>
</scxml>"##
        )
    }

    /// A byte-positioned codec, big endian.
    fn positional_xml(imports: &str, body: &str) -> String {
        codec_xml(r#"sce:default-endian="big""#, imports, body)
    }

    fn model_of(xml: &str) -> CodecModel {
        let label = DocumentLabel {
            identifier: "sized",
            diagnostic_label: ".scxml-fixture",
        };
        match parse_forge(xml, label) {
            Ok(Some(ForgeDocument::Codec(model))) => model,
            Ok(_) => panic!("the fixture is not a codec document"),
            Err(located) => panic!("the fixture does not parse: {:?}", located.error),
        }
    }

    /// The codec with the imports the test resolves by hand.
    fn sized(xml: &str, imports: Vec<(&str, SizedImport)>) -> SizedCodec {
        SizedCodec {
            model: model_of(xml),
            imports: imports
                .into_iter()
                .map(|(alias, import)| (alias.to_string(), import))
                .collect(),
        }
    }

    fn body(xml: &str) -> SizedImport {
        SizedImport::Codec(Box::new(sized(xml, Vec::new())))
    }

    /// The refusal of a codec that has none of its imports.
    fn refusal(xml: &str) -> Unbounded {
        largest_encoded_bytes(&sized(xml, Vec::new())).expect_err("the codec has no bound")
    }

    /// A body whose bound is the bytes of one `uint16` / one `uint32`.
    fn uint16_body() -> SizedImport {
        body(&positional_xml(
            "",
            r#"<data id="seq" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>"#,
        ))
    }

    fn uint32_body() -> SizedImport {
        body(&positional_xml(
            "",
            r#"<data id="word" sce:type="uint32" sce:byte="0" sce:bit-size="32"/>"#,
        ))
    }

    /// A locator: a 64-bit VLE length and a string of at most 128 bytes.
    fn locator_xml(string_attrs: &str) -> String {
        positional_xml(
            "",
            &format!(
                r#"<sce:field id="locator_len" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="locator" sce:type="string" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="locator_len" {string_attrs}/>"#
            ),
        )
    }

    // --- The positional path ---

    #[test]
    fn fixed_fields_are_the_sum_of_their_widths() {
        // byte 0: uint32 = 4 bytes; byte 4: uint16 = 2 bytes. 4 + 2 = 6.
        let xml = positional_xml(
            "",
            r#"<data id="sensor" sce:type="uint32" sce:byte="0" sce:bit-size="32"/>
    <data id="level" sce:type="uint16" sce:byte="4" sce:bit-size="16"/>"#,
        );
        assert_eq!(largest_encoded_bytes(&sized(&xml, Vec::new())), Ok(6));
    }

    #[test]
    fn sub_byte_fields_that_share_an_offset_are_one_byte() {
        // Three fields at byte 0 (3 + 3 + 2 bits) are one byte; the uint8 at
        // byte 1 is another. 1 + 1 = 2.
        let xml = positional_xml(
            "",
            r#"<data id="priority" sce:type="uint8" sce:byte="0" sce:bit-offset="5" sce:bit-size="3"/>
    <data id="channel" sce:type="uint8" sce:byte="0" sce:bit-offset="2" sce:bit-size="3"/>
    <data id="direction" sce:type="uint8" sce:byte="0" sce:bit-offset="0" sce:bit-size="2"/>
    <data id="after" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
        );
        assert_eq!(largest_encoded_bytes(&sized(&xml, Vec::new())), Ok(2));
    }

    #[test]
    fn a_length_ref_payload_adds_its_max_size() {
        // msgId (1) + len (1) + payload at most 32 = 34.
        let xml = positional_xml(
            "",
            r#"<data id="msgId" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <data id="len" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
    <data id="payload" sce:type="bytes" sce:byte="2" sce:bit-size="length-ref"
          sce:length-field="len" sce:max-size="32"/>"#,
        );
        assert_eq!(largest_encoded_bytes(&sized(&xml, Vec::new())), Ok(34));
    }

    #[test]
    fn a_dma_aligned_payload_starts_at_its_own_offset() {
        // msg_id (1) + reserved (1) are zero-filled up to byte 32, where the
        // payload of at most 64 bytes starts: 32 + 64 = 96.
        let xml = positional_xml(
            "",
            r#"<sce:field id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="reserved" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
    <sce:field id="aligned_payload" sce:type="bytes" sce:byte="32"
               sce:bit-size="tail" sce:max-size="64"
               sce:dma-burst-align="32"/>"#,
        );
        assert_eq!(largest_encoded_bytes(&sized(&xml, Vec::new())), Ok(96));
    }

    // --- The streaming path ---

    #[test]
    fn a_gated_field_counts_at_its_full_width() {
        // flags (1) + seq (2, present when the flag is set) = 3.
        let xml = positional_xml(
            "",
            r#"<sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="has_seq" bit="0"/>
    </sce:flags>
    <sce:field id="seq" sce:type="uint16" sce:byte="1" sce:bit-size="16"
               sce:present-if="flags.has_seq"/>"#,
        );
        assert_eq!(largest_encoded_bytes(&sized(&xml, Vec::new())), Ok(3));
    }

    #[test]
    fn a_vle_integer_counts_at_its_cap_and_a_string_at_its_max_size() {
        // A u64 VLE is at most 9 bytes; the string is at most 128: 9 + 128 = 137.
        let xml = locator_xml(r#"sce:max-size="128""#);
        assert_eq!(largest_encoded_bytes(&sized(&xml, Vec::new())), Ok(137));
    }

    #[test]
    fn an_embedded_codec_adds_its_own_bound() {
        // tag (1) + the locator embedded as is (137) = 138.
        let xml = positional_xml(
            r#"<sce:import src="locator.scxml" kind="codec" as="locator_codec"/>"#,
            r#"<sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="locator" type="locator_codec" sce:byte="1"/>"#,
        );
        let locator = body(&locator_xml(r#"sce:max-size="128""#));
        assert_eq!(
            largest_encoded_bytes(&sized(&xml, vec![("locator_codec", locator)])),
            Ok(138)
        );
    }

    #[test]
    fn a_repeat_is_its_count_times_its_body() {
        // num_frags (1) + at most 32 entries of a uint16 (2 each) = 1 + 64 = 65.
        let xml = positional_xml(
            r#"<sce:import src="elem.scxml" kind="codec" as="elem"/>"#,
            r#"<sce:field id="num_frags" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:repeat id="frags" type="elem" sce:byte="1" count="num_frags" max-count="32"/>"#,
        );
        assert_eq!(
            largest_encoded_bytes(&sized(&xml, vec![("elem", uint16_body())])),
            Ok(65)
        );
    }

    #[test]
    fn a_tlv_chain_is_its_depth_times_its_entry() {
        // The entry is type (1) + len (1) + a value of at most 32 = 34.
        // header (1) + at most 8 entries = 1 + 8 * 34 = 273.
        let entry = body(&positional_xml(
            "",
            r#"<sce:field id="entry_type" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="entry_len"  sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
    <sce:field id="entry_body" sce:type="bytes" sce:byte="2"
               sce:bit-size="length-ref" sce:length-field="entry_len"
               sce:max-size="32"/>"#,
        ));
        let xml = positional_xml(
            r#"<sce:import src="entry.scxml" kind="codec" as="entry"/>"#,
            r#"<sce:field id="header_flags" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:tlv-chain id="extensions" type="entry" sce:byte="1"
                   max-depth="8" on-overflow="reject"/>"#,
        );
        assert_eq!(
            largest_encoded_bytes(&sized(&xml, vec![("entry", entry)])),
            Ok(273)
        );
    }

    #[test]
    fn a_variant_is_its_tag_and_its_largest_arm() {
        // tag (1) + the largest of the arms: leaf 2, scalar 4, default 4 = 5.
        let xml = positional_xml(
            r#"<sce:import src="leaf.scxml" kind="codec" as="leaf"/>
  <sce:import src="scalar.scxml" kind="codec" as="scalar"/>"#,
            r#"<data id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="tag">
      <sce:arm value="0x01" type="leaf"/>
      <sce:arm value="0x02" type="scalar" default="true"/>
      <sce:default type="scalar"/>
    </sce:variant>"#,
        );
        let imports = vec![("leaf", uint16_body()), ("scalar", uint32_body())];
        assert_eq!(largest_encoded_bytes(&sized(&xml, imports)), Ok(5));
    }

    // --- The CBOR map ---

    const CBOR_ENTRIES: &str = r#"    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="source" sce:type="string" sce:key="1" sce:max-size="64" sce:direction="out"/>
    <data id="id" sce:type="bytes" sce:key="0" sce:length="16" sce:direction="out"/>
    <data id="pattern" sce:type="uint8" sce:key="3" sce:direction="out"/>
    <data id="payload" sce:type="bytes" sce:key="5" sce:max-size="256" sce:direction="out"/>
    <data id="deadline" sce:type="optional&lt;uint64&gt;" sce:key="12" sce:direction="out"/>
    <data id="urgent" sce:type="optional&lt;bool&gt;" sce:key="7" sce:direction="out"/>"#;

    const CBOR_ATTRS: &str = r#"sce:encoding="cbor" sce:codec-format="2""#;

    #[test]
    fn a_cbor_map_is_the_bound_its_c11_renderer_carries() {
        // The map head for 6 entries is 1 byte, then key + value per entry:
        //   source   (key 1)  1 + (head(64) = 2 + 64)   = 67
        //   id       (key 0)  1 + (head(16) = 1 + 16)   = 18
        //   pattern  (key 3)  1 + 9                     = 10
        //   payload  (key 5)  1 + (head(256) = 3 + 256) = 260
        //   deadline (key 12) 1 + 9                     = 10
        //   urgent   (key 7)  1 + 1                     = 2
        // 1 + 67 + 18 + 10 + 260 + 10 + 2 = 368.
        let xml = codec_xml(CBOR_ATTRS, "", CBOR_ENTRIES);
        let codec = sized(&xml, Vec::new());
        assert_eq!(largest_encoded_bytes(&codec), Ok(368));
        assert_eq!(
            crate::forge::cbor_codec::max_encoded_bytes(&codec.model),
            368
        );
    }

    #[test]
    fn a_cbor_byte_string_with_no_bound_is_refused() {
        let xml = codec_xml(
            CBOR_ATTRS,
            "",
            r#"    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="payload" sce:type="bytes" sce:key="5" sce:direction="out"/>"#,
        );
        match refusal(&xml) {
            Unbounded::Missing {
                subject,
                declaration,
            } => {
                assert!(subject.contains("'payload'"), "{subject}");
                assert_eq!(declaration, "sce:length or sce:max-size");
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    // --- The content-line component ---

    const CONTENT_LINE_ATTRS: &str =
        r#"sce:encoding="content-line" sce:codec-format="2" sce:component="VEVENT""#;

    #[test]
    fn a_content_line_component_is_the_bound_its_c11_renderer_carries() {
        // BEGIN:VEVENT CRLF and END:VEVENT CRLF: 2 * 6 + 6 + 4 + 4 = 26, for the
        // six letters of VEVENT.
        //   UID      logical 3 + 1 + 8 (the string)   = 12; physical 12 + 2 + 3 * 1 = 17
        //   SEQUENCE logical 8 + 1 + 20 (an integer)  = 29; physical 29 + 2 + 3 * 1 = 34
        // 26 + 17 + 34 = 77.
        let xml = codec_xml(
            CONTENT_LINE_ATTRS,
            "",
            r#"    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="uid" sce:type="string" sce:property="UID" sce:max-size="8"/>
    <data id="seq" sce:type="optional&lt;uint16&gt;" sce:property="SEQUENCE"/>"#,
        );
        assert_eq!(largest_encoded_bytes(&sized(&xml, Vec::new())), Ok(77));
    }

    fn enum_entry_xml() -> String {
        codec_xml(
            CONTENT_LINE_ATTRS,
            r#"<sce:import src="lifecycle.scxml" kind="enum" as="lifecycle"/>"#,
            r#"    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="status" sce:type="optional&lt;enum:lifecycle&gt;" sce:property="STATUS"/>"#,
        )
    }

    #[test]
    fn a_content_line_enum_is_written_as_its_longest_text() {
        // 26 for the component (see the test above), then STATUS: logical
        // 6 + 1 + 12 (the longest text) = 19; physical 19 + 2 + 3 * 1 = 24.
        // 26 + 24 = 50.
        let imports = vec![("lifecycle", SizedImport::Enum { longest_text: 12 })];
        assert_eq!(
            largest_encoded_bytes(&sized(&enum_entry_xml(), imports)),
            Ok(50)
        );
    }

    #[test]
    fn a_content_line_enum_that_did_not_resolve_is_refused() {
        match refusal(&enum_entry_xml()) {
            Unbounded::Unresolved { site, alias } => {
                assert!(site.contains("'status'"), "{site}");
                assert_eq!(alias, "lifecycle");
            }
            other => panic!("expected Unresolved, got {other:?}"),
        }
    }

    #[test]
    fn a_content_line_string_without_a_bound_is_refused() {
        // The parser holds every string to a bound; a model built another way
        // is refused here and not bounded by zero.
        let xml = codec_xml(
            CONTENT_LINE_ATTRS,
            "",
            r#"    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="uid" sce:type="string" sce:property="UID" sce:max-size="8"/>"#,
        );
        let mut codec = sized(&xml, Vec::new());
        let component = codec.model.content_line.as_mut().expect("a component");
        component.entries[0].max_size = None;
        match largest_encoded_bytes(&codec) {
            Err(Unbounded::Missing {
                subject,
                declaration,
            }) => {
                assert!(subject.contains("'uid'"), "{subject}");
                assert_eq!(declaration, "sce:max-size");
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn a_content_line_codec_with_no_component_is_refused() {
        let xml = codec_xml(
            CONTENT_LINE_ATTRS,
            "",
            r#"    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="uid" sce:type="string" sce:property="UID" sce:max-size="8"/>"#,
        );
        let mut codec = sized(&xml, Vec::new());
        codec.model.content_line = None;
        assert!(matches!(
            largest_encoded_bytes(&codec),
            Err(Unbounded::NoWireForm(_))
        ));
    }

    // --- What has no bound ---

    #[test]
    fn a_tail_payload_without_a_max_size_is_refused_by_name() {
        let xml = positional_xml(
            "",
            r#"<data id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <data id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="tail"/>"#,
        );
        match refusal(&xml) {
            Unbounded::Missing {
                subject,
                declaration,
            } => {
                assert!(subject.contains("'payload'"), "{subject}");
                assert!(subject.contains("tail"), "{subject}");
                assert_eq!(declaration, "sce:max-size");
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn a_length_ref_payload_without_a_max_size_is_refused_by_name() {
        let xml = positional_xml(
            "",
            r#"<data id="len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <data id="body" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
          sce:length-field="len"/>"#,
        );
        match refusal(&xml) {
            Unbounded::Missing {
                subject,
                declaration,
            } => {
                assert!(subject.contains("'body'"), "{subject}");
                assert!(subject.contains("length-ref"), "{subject}");
                assert_eq!(declaration, "sce:max-size");
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn a_string_without_a_max_size_is_refused_on_the_streaming_path_too() {
        match refusal(&locator_xml("")) {
            Unbounded::Missing {
                subject,
                declaration,
            } => {
                assert!(subject.contains("'locator'"), "{subject}");
                assert_eq!(declaration, "sce:max-size");
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn a_repeat_with_no_count_bound_is_refused_by_name() {
        let xml = positional_xml(
            r#"<sce:import src="elem.scxml" kind="codec" as="elem"/>"#,
            r#"<sce:repeat id="msgs" type="elem" sce:byte="0" until-eof="true"/>"#,
        );
        let codec = sized(&xml, vec![("elem", uint16_body())]);
        match largest_encoded_bytes(&codec) {
            Err(Unbounded::Missing {
                subject,
                declaration,
            }) => {
                assert!(subject.contains("'msgs'"), "{subject}");
                assert_eq!(declaration, "max-count");
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn an_embedded_codec_that_did_not_resolve_is_refused() {
        // The alias is absent from the resolved imports: a file that is not
        // there, a document of another kind, or a codec that imports itself.
        let xml = positional_xml(
            r#"<sce:import src="locator.scxml" kind="codec" as="locator_codec"/>"#,
            r#"<sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="locator" type="locator_codec" sce:byte="1"/>"#,
        );
        match refusal(&xml) {
            Unbounded::Unresolved { site, alias } => {
                assert!(site.contains("'locator'"), "{site}");
                assert_eq!(alias, "locator_codec");
            }
            other => panic!("expected Unresolved, got {other:?}"),
        }
    }

    #[test]
    fn a_variant_arm_that_did_not_resolve_is_refused() {
        let xml = positional_xml(
            r#"<sce:import src="leaf.scxml" kind="codec" as="leaf"/>"#,
            r#"<data id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="tag">
      <sce:arm value="0x01" type="leaf" default="true"/>
      <sce:default type="leaf"/>
    </sce:variant>"#,
        );
        match refusal(&xml) {
            Unbounded::Unresolved { site, alias } => {
                assert!(site.contains("variant arm"), "{site}");
                assert_eq!(alias, "leaf");
            }
            other => panic!("expected Unresolved, got {other:?}"),
        }
    }

    #[test]
    fn a_body_with_no_bound_is_named_through_the_codec_that_holds_it() {
        let xml = positional_xml(
            r#"<sce:import src="locator.scxml" kind="codec" as="locator_codec"/>"#,
            r#"<sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="locator" type="locator_codec" sce:byte="1"/>"#,
        );
        let locator = body(&locator_xml(""));
        let codec = sized(&xml, vec![("locator_codec", locator)]);
        let refused = largest_encoded_bytes(&codec).expect_err("the body has no bound");
        match &refused {
            Unbounded::Within { site, alias, inner } => {
                assert!(site.contains("'locator'"), "{site}");
                assert_eq!(alias, "locator_codec");
                assert!(matches!(**inner, Unbounded::Missing { .. }), "{inner:?}");
            }
            other => panic!("expected Within, got {other:?}"),
        }
        let text = refused.to_string();
        assert!(text.contains("'locator_codec'"), "{text}");
        assert!(text.contains("sce:max-size"), "{text}");
    }

    #[test]
    fn a_document_that_is_not_a_codec_has_no_wire_form() {
        let refused = Unbounded::no_wire_form(ForgeKind::Procedure);
        assert!(matches!(refused, Unbounded::NoWireForm(_)));
        assert!(refused.to_string().contains("procedure"), "{refused}");
    }

    #[test]
    fn every_refusal_reads_as_a_clause() {
        let missing = Unbounded::Missing {
            subject: "field 'payload'".to_string(),
            declaration: "sce:max-size",
        };
        assert_eq!(missing.to_string(), "field 'payload' has no sce:max-size");
        let unresolved = Unbounded::Unresolved {
            site: "field 'body'".to_string(),
            alias: "elem".to_string(),
        };
        let text = unresolved.to_string();
        assert!(
            text.starts_with("field 'body' names the import 'elem'"),
            "{text}"
        );
        assert!(text.ends_with("has no largest size)"), "{text}");
        for reason in [&missing, &unresolved] {
            assert!(!reason.to_string().ends_with('.'), "{reason}");
        }
    }
}
