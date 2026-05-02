// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};
// RFC §5.B B3 DMA alignment primitive: structural drift detection.
// Build-time validation already guaranteed `byte_offset % burst_align
// == 0` and that all preceding fields are Fixed bit-size. These
// `const _: () = assert!` declarations catch any future hand-edit to
// the byte_offset that would break the wire-layout invariant.
const _: () = assert!(
    32 % 32 == 0,
    "RFC §5.B B3: codec field 'aligned_payload' offset must be 32-aligned"
);

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecDmaAlignedBasic {
    pub msg_id: u8,
    pub reserved: u8,
    pub aligned_payload: Vec<u8>,
}

#[allow(dead_code)]
impl CodecDmaAlignedBasic {
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
        let value = Self {
            msg_id: raw[0],
            reserved: raw[1],
            aligned_payload: raw[32..].to_vec(),
        };
        cursor.advance(_frame_len)?;
        Ok(value)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut r: Vec<u8> = Vec::with_capacity(66);
        r.push(self.msg_id);
        r.push(self.reserved);
        // RFC §5.B B3 DMA padding: zero-fill any gap between the
        // current write position and this field's authored byte
        // offset (deterministic zeros on the wire so peers stay
        // byte-compatible regardless of host allocator — RFC §5.B
        // "wire layout, not host allocator").
        while r.len() < 32 {
            r.push(0);
        }
        r.extend_from_slice(&self.aligned_payload);
        r
    }
}
