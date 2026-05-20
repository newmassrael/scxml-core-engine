#![doc = "SCE-MAP: codec_length_ref_uint16_be:12"]
// SCE-MAP: codec_length_ref_uint16_be:12

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
pub struct CodecLengthRefUint16Be {
    pub payload_len: u16,
    pub payload: Vec<u8>,
}

#[allow(dead_code)]
impl CodecLengthRefUint16Be {
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
        if _frame_len < 2 {
            return Err(CodecError::NeedMoreBytes);
        }
        let raw = cursor.peek_slice(_frame_len)?;
        let payload_len = ((raw[0] as u16) << 8) | raw[1] as u16;
        let payload = raw[2..2 + payload_len as usize].to_vec();
        let value = Self {
            payload_len,
            payload,
        };
        // Stream-correct: advance only the bytes actually decoded.
        // For each length-ref field, end = byte_off + sibling local
        // value (the sibling let-binding ran before the payload's).
        // Take the max across all length-ref fields; min_bytes is the
        // lower bound.
        let mut _consumed: usize = 2;
        {
            let _end = 2usize + value.payload.len();
            if _end > _consumed { _consumed = _end; }
        }
        if _consumed > _frame_len {
            return Err(CodecError::NeedMoreBytes);
        }
        cursor.advance(_consumed)?;
        Ok(value)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut r: Vec<u8> = Vec::with_capacity(1026);
        r.push((self.payload_len >> 8 & 0xFF) as u8);
        r.push((self.payload_len & 0xFF) as u8);
        r.extend_from_slice(&self.payload);
        r
    }
}
