// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_put::CodecZenohPut;
use super::codec_zenoh_del::CodecZenohDel;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohPushBodyVariant {
    CodecZenohPut(CodecZenohPut),
    CodecZenohDel(CodecZenohDel),
    Default {
        tag: u8,
        body: CodecZenohPut,
    },
}

impl Default for CodecZenohPushBodyVariant {
    fn default() -> Self {
        // Default to the first declared arm's body — every imported
        // codec is `#[derive(Default)]`, so this is infallible. A
        // freshly-constructed envelope is overwritten by `decode()` or
        // by an explicit user assignment before any `encode()` call.
        Self::CodecZenohPut(CodecZenohPut::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohPushBody {
    pub header: u8,
    pub body: CodecZenohPushBodyVariant,
}

#[allow(dead_code)]
impl CodecZenohPushBody {
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
        let body = match ((header >> 0) & (0x1F as u8)) as u8 {
            1u8 => CodecZenohPushBodyVariant::CodecZenohPut(CodecZenohPut::decode(cursor)?),
            2u8 => CodecZenohPushBodyVariant::CodecZenohDel(CodecZenohDel::decode(cursor)?),
            other => CodecZenohPushBodyVariant::Default {
                tag: other,
                body: CodecZenohPut::decode(cursor)?,
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
    pub fn mid(&self) -> u8 {
        (((self.header >> 0) & (0x1F as u8))) as u8
    }

    pub fn set_mid(&mut self, v: u8) {
        let _mask: u8 = (0x1F as u8) << 0;
        let _val: u8 = ((v as u8) & (0x1F as u8)) << 0;
        self.header = (self.header & !_mask) | _val;
    }

    pub fn rest(&self) -> u8 {
        (((self.header >> 5) & (0x07 as u8))) as u8
    }

    pub fn set_rest(&mut self, v: u8) {
        let _mask: u8 = (0x07 as u8) << 5;
        let _val: u8 = ((v as u8) & (0x07 as u8)) << 5;
        self.header = (self.header & !_mask) | _val;
    }

    pub fn encode(&self) -> Vec<u8> {
        // Encode fixed prefix (tag field is part of the prefix). The
        // tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set msg_id / body in
        // sync is the caller's responsibility (v1 keeps the layout
        // simple; future extensions may auto-sync via a typed setter).
        let mut r: Vec<u8> = Vec::with_capacity(2);
        r.push(self.header);
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohPushBodyVariant::CodecZenohPut(b) => {
                r.extend(b.encode());
            }
            CodecZenohPushBodyVariant::CodecZenohDel(b) => {
                r.extend(b.encode());
            }
            CodecZenohPushBodyVariant::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
