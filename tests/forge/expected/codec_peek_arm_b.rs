#![doc = "SCE-MAP: codec_peek_arm_b:13"]
// SCE-MAP: codec_peek_arm_b:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
pub struct CodecPeekArmB {
    pub header: u8,
    pub payload: u16,
}

// RFC variant-default-uniformity Atomic β: at least one field's
// `<sce:flags>` carrier declares a wire-MID constant via
// `<sce:flag value="N"/>`. Manual `impl Default` bakes the OR of
// every declared `(value & mask) << bit` into that carrier so a
// freshly-constructed instance carries the wire-MID for its own
// dispatch tag. Fields without declared values fall through to
// `Default::default()` (preserving derive(Default) semantics).
impl Default for CodecPeekArmB {
    fn default() -> Self {
        Self {
            header: 0x01u8,
            payload: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl CodecPeekArmB {
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
        let raw = cursor.peek_slice(3)?;
        let header = raw[0];
        let payload = ((raw[1] as u16) << 8) | raw[2] as u16;
        let value = Self {
            header,
            payload,
        };
        cursor.advance(3)?;
        Ok(value)
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn kind(&self) -> bool {
        (self.header & 0x01) != 0
    }

    pub fn set_kind(&mut self, v: bool) {
        if v {
            self.header |= 0x01;
        } else {
            self.header &= !0x01;
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![
            self.header,
            (self.payload >> 8 & 0xFF) as u8,
            (self.payload & 0xFF) as u8,
        ]
    }
}
