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
pub struct CodecLengthRefDottedBasic {
    pub carrier: u8,
    pub payload: Vec<u8>,
}

#[allow(dead_code)]
impl CodecLengthRefDottedBasic {
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
        // Variable-length codec. RFC §5.B B3 stream-correct shape:
        // a codec without `<sce:field sce:bit-size="tail">` consumes
        // only `min_bytes + length_value` rather than the entire
        // cursor remaining. Codecs WITH a tail field still consume
        // to end (tail's definition forces it). The prior
        // "consume entire cursor" behaviour deferred to "the first
        // multi-frame consumer" — TLV chain (B3-α) is that consumer,
        // so length-ref entry codecs now decode-iterably from a
        // shared cursor without each entry eating the next entry's
        // bytes.
        let _frame_len = cursor.remaining();
        if _frame_len < 1 {
            return Err(CodecError::NeedMoreBytes);
        }
        let raw = cursor.peek_slice(_frame_len)?;
        let value = Self {
            carrier: raw[0],
            payload: raw[1..1 + (((raw[0] >> 4) & 0xF) as usize)].to_vec(),
        };
        // Stream-correct: advance only the bytes actually decoded.
        // For each length-ref field, end = byte_off + raw[byte_off-1]
        // value (length-field byte is read just before). Take the max
        // across all length-ref fields; min_bytes is the lower bound.
        let mut _consumed: usize = 1;
        {
            let _end = 1usize + value.payload.len();
            if _end > _consumed { _consumed = _end; }
        }
        if _consumed > _frame_len {
            return Err(CodecError::NeedMoreBytes);
        }
        cursor.advance(_consumed)?;
        Ok(value)
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn hdr(&self) -> u8 {
        (((self.carrier >> 0) & (0x0F as u8))) as u8
    }

    pub fn set_hdr(&mut self, v: u8) {
        let _mask: u8 = (0x0F as u8) << 0;
        let _val: u8 = ((v as u8) & (0x0F as u8)) << 0;
        self.carrier = (self.carrier & !_mask) | _val;
    }

    pub fn payload_len(&self) -> u8 {
        (((self.carrier >> 4) & (0x0F as u8))) as u8
    }

    pub fn set_payload_len(&mut self, v: u8) {
        let _mask: u8 = (0x0F as u8) << 4;
        let _val: u8 = ((v as u8) & (0x0F as u8)) << 4;
        self.carrier = (self.carrier & !_mask) | _val;
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut r: Vec<u8> = Vec::with_capacity(16);
        r.push(self.carrier);
        r.extend_from_slice(&self.payload);
        r
    }
}
