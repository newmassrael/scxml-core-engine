// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_ext_unit::CodecZenohExtUnit;
use super::codec_zenoh_ext_zint::CodecZenohExtZint;
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbuf;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohExtEntryVariant {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohExtZbuf(CodecZenohExtZbuf),
    Default {
        tag: u8,
        body: CodecZenohExtUnit,
    },
}

impl Default for CodecZenohExtEntryVariant {
    fn default() -> Self {
        // Default to the first declared arm's body — every imported
        // codec is `#[derive(Default)]`, so this is infallible. A
        // freshly-constructed envelope is overwritten by `decode()` or
        // by an explicit user assignment before any `encode()` call.
        Self::CodecZenohExtUnit(CodecZenohExtUnit::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohExtEntry {
    pub header: u8,
    pub body: CodecZenohExtEntryVariant,
}

#[allow(dead_code)]
impl CodecZenohExtEntry {
    /// Construct an instance with every field zero-initialized via
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519).
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        // Decode fixed prefix (RFC §5.B variant primitive B1-β: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let header = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match (((header >> 5) & (0x03 as u8)) as u8) {
            0u8 => CodecZenohExtEntryVariant::CodecZenohExtUnit(CodecZenohExtUnit::decode(cursor)?),
            1u8 => CodecZenohExtEntryVariant::CodecZenohExtZint(CodecZenohExtZint::decode(cursor)?),
            2u8 => CodecZenohExtEntryVariant::CodecZenohExtZbuf(CodecZenohExtZbuf::decode(cursor)?),
            other => CodecZenohExtEntryVariant::Default {
                tag: other,
                body: CodecZenohExtUnit::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            body,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn id(&self) -> u8 {
        (((self.header >> 0) & (0x1F as u8))) as u8
    }

    pub fn set_id(&mut self, v: u8) {
        let _mask: u8 = (0x1F as u8) << 0;
        let _val: u8 = ((v as u8) & (0x1F as u8)) << 0;
        self.header = (self.header & !_mask) | _val;
    }

    pub fn enc(&self) -> u8 {
        (((self.header >> 5) & (0x03 as u8))) as u8
    }

    pub fn set_enc(&mut self, v: u8) {
        let _mask: u8 = (0x03 as u8) << 5;
        let _val: u8 = ((v as u8) & (0x03 as u8)) << 5;
        self.header = (self.header & !_mask) | _val;
    }

    pub fn encode(&self) -> Vec<u8> {
        // Encode fixed prefix (tag field is part of the prefix). The
        // tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set msg_id / body in
        // sync is the caller's responsibility (v1 keeps the layout
        // simple; future extensions may auto-sync via a typed setter).
        let mut r: Vec<u8> = Vec::with_capacity(43);
        r.push(self.header);
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohExtEntryVariant::CodecZenohExtUnit(b) => {
                r.extend(b.encode());
            }
            CodecZenohExtEntryVariant::CodecZenohExtZint(b) => {
                r.extend(b.encode());
            }
            CodecZenohExtEntryVariant::CodecZenohExtZbuf(b) => {
                r.extend(b.encode());
            }
            CodecZenohExtEntryVariant::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
