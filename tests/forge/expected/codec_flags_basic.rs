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
pub struct CodecFlagsBasic {
    pub header: u8,
}

#[allow(dead_code)]
impl CodecFlagsBasic {
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
            header: raw[0],
        };
        cursor.advance(1)?;
        Ok(value)
    }

    // RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    // field. Read returns a bool from `(field & mask) != 0`; write
    // toggles the bit on/off without disturbing siblings on the same
    // carrier. Wire layout is unchanged — the carrier still occupies
    // its declared bytes.
    pub fn reliable(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_reliable(&mut self, v: bool) {
        if v {
            self.header |= 0x80;
        } else {
            self.header &= !0x80;
        }
    }

    pub fn more(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_more(&mut self, v: bool) {
        if v {
            self.header |= 0x40;
        } else {
            self.header &= !0x40;
        }
    }

    pub fn drop(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_drop(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn first(&self) -> bool {
        (self.header & 0x10) != 0
    }

    pub fn set_first(&mut self, v: bool) {
        if v {
            self.header |= 0x10;
        } else {
            self.header &= !0x10;
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![
            self.header,
        ]
    }
}
