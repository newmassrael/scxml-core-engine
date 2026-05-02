// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecQosByte {
    pub qos: u8,
}

#[allow(dead_code)]
impl CodecQosByte {
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
        let raw = cursor.peek_slice(1)?;
        let value = Self {
            qos: raw[0],
        };
        cursor.advance(1)?;
        Ok(value)
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn priority(&self) -> u8 {
        (((self.qos >> 0) & (0x07 as u8))) as u8
    }

    pub fn set_priority(&mut self, v: u8) {
        let _mask: u8 = (0x07 as u8) << 0;
        let _val: u8 = ((v as u8) & (0x07 as u8)) << 0;
        self.qos = (self.qos & !_mask) | _val;
    }

    pub fn reliable(&self) -> bool {
        (self.qos & 0x08) != 0
    }

    pub fn set_reliable(&mut self, v: bool) {
        if v {
            self.qos |= 0x08;
        } else {
            self.qos &= !0x08;
        }
    }

    pub fn congestion(&self) -> u8 {
        (((self.qos >> 4) & (0x03 as u8))) as u8
    }

    pub fn set_congestion(&mut self, v: u8) {
        let _mask: u8 = (0x03 as u8) << 4;
        let _val: u8 = ((v as u8) & (0x03 as u8)) << 4;
        self.qos = (self.qos & !_mask) | _val;
    }

    pub fn express(&self) -> bool {
        (self.qos & 0x40) != 0
    }

    pub fn set_express(&mut self, v: bool) {
        if v {
            self.qos |= 0x40;
        } else {
            self.qos &= !0x40;
        }
    }

    pub fn reserved(&self) -> bool {
        (self.qos & 0x80) != 0
    }

    pub fn set_reserved(&mut self, v: bool) {
        if v {
            self.qos |= 0x80;
        } else {
            self.qos &= !0x80;
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![
            self.qos,
        ]
    }
}
