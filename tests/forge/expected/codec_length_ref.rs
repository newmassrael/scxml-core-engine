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
pub struct CodecLengthRef {
    pub msg_id: u8,
    pub len: u8,
    pub payload: Vec<u8>,
}

#[allow(dead_code)]
impl CodecLengthRef {
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
        // Variable-length codec: tail / length-ref fields consume bytes
        // beyond the fixed prefix. B1-prep treats the entire cursor
        // remaining as one frame (matches the harness "one frame per
        // call" pattern). Stream-correct length-ref consumption (advance
        // by `min_bytes + length_value` only) lands with its first
        // multi-frame consumer in a later B-stage.
        let _frame_len = cursor.remaining();
        if _frame_len < 2 {
            return Err(CodecError::NeedMoreBytes);
        }
        let raw = cursor.peek_slice(_frame_len)?;
        let value = Self {
            msg_id: raw[0],
            len: raw[1],
            payload: raw[2..2 + raw[1] as usize].to_vec(),
        };
        cursor.advance(_frame_len)?;
        Ok(value)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut r: Vec<u8> = Vec::with_capacity(34);
        r.push(self.msg_id);
        r.push(self.len);
        r.extend_from_slice(&self.payload);
        r
    }
}
