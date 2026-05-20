#![doc = "SCE-MAP: codec_repeat_basic:11"]
// SCE-MAP: codec_repeat_basic:11

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink, VecSink};

use super::codec_repeat_elem::CodecRepeatElem;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecRepeatBasic {
    pub num_frags: u8,
    pub frags: Vec<CodecRepeatElem>,
}

#[allow(dead_code)]
impl CodecRepeatBasic {
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
        // RFC §5.B B2 repeat / B3 TLV chain primitives: streaming
        // decode mixes plain fixed-width reads (per-field via the
        // present-if helper's non-gated arm) with repeat loops that
        // iterate the imported codec's `decode()`. Repeat: bounded by
        // `count_ref` (length-field) or cursor exhaustion (until-eof).
        // TLV chain: bounded by `max_depth` with on-overflow check.
        // Element bodies recurse into their own codec — each may
        // itself surface NeedMoreBytes, unwinding the partial frame.
        let num_frags = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let frags = {
            let mut _vec: Vec<CodecRepeatElem> = Vec::with_capacity(num_frags as usize);
            for _ in 0..num_frags {
                _vec.push(CodecRepeatElem::decode(cursor)?);
            }
            _vec
        };
        Ok(Self {
            num_frags,
            frags,
        })
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 65;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // RFC §5.B B2 repeat / B3 TLV chain encode: fixed prefix
        // fields append byte-by-byte; list fields iterate the host-
        // language list and splice each element's encode() into the
        // parent buffer. Author keeps the count field (repeat) /
        // chain length (tlv-chain) consistent with the list length
        // (same trust contract as variant tag/body).
        w.write_u8(self.num_frags)?;
        for _e in &self.frags {
            _e.encode(w)?;
        }
        Ok(())
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector. Callers
    /// targeting zero-alloc hot paths should call `encode` directly
    /// against a caller-owned sink.
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)
            .expect("VecSink is infallible");
        _sce_v
    }
}
