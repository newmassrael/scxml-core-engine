#![doc = "SCE-MAP: codec_ext_attachment:27"]
// SCE-MAP: codec_ext_attachment:27

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// RFC §5.B B1-α: `VecSink` and the heap-backed `encode_to_vec` facade
// are gated on the `alloc` feature (see
// `sce-forge-runtime/rust/src/codec.rs`). MCU / `no_std` consumers see
// only the sink-based primary `encode` + `SliceSink` paths.
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use sce_forge_runtime::codec::VecSink;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecExtAttachment<'a> {
    pub length: u8,
    pub body: &'a [u8],
}

#[allow(dead_code)]
impl<'a> CodecExtAttachment<'a> {
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
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
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
        let length = raw[0];
        let body = &raw[1..1 + length as usize];
        let value = Self {
            length,
            body,
        };
        // Stream-correct: advance only the bytes actually decoded.
        // For each length-ref field, end = byte_off + sibling local
        // value (the sibling let-binding ran before the payload's).
        // Take the max across all length-ref fields; min_bytes is the
        // lower bound.
        let mut _consumed: usize = 1;
        {
            let _end = 1usize + value.body.len();
            if _end > _consumed { _consumed = _end; }
        }
        if _consumed > _frame_len {
            return Err(CodecError::NeedMoreBytes);
        }
        cursor.advance(_consumed)?;
        Ok(value)
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
        w.write_u8(self.length)?;
        // `self.<id>` is the borrowed `&'a [u8]` view — pass it directly;
        // `&self.<id>` would be `&&[u8]` (clippy::needless_borrow).
        w.write_bytes(self.body)?;
        Ok(())
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector. Callers
    /// targeting zero-alloc hot paths should call `encode` directly
    /// against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `sce-forge-runtime/rust/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)
            .expect("VecSink is infallible");
        _sce_v
    }
}

// ── Owned projection (consumer-requested; alloc-gated) ────────────────
// `CodecExtAttachment<'a>` above is a zero-copy view borrowing the decode
// buffer. AP / async consumers that persist a decoded message beyond the
// buffer's lifetime call `.into_owned()` for this lifetime-free
// `CodecExtAttachmentOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split — both generated from the one SCXML source (SSOT). `Vec`
// / `String` are alloc, so the whole projection is gated; the no-alloc
// borrowed path above is untouched.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecExtAttachmentOwned {
    pub length: u8,
    pub body: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl<'a> CodecExtAttachment<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecExtAttachmentOwned`] (alloc). Call at a decode boundary when
    /// the decoded value must outlive the input buffer — e.g. stored in a
    /// long-lived enum or moved across an async task. The no-alloc
    /// borrowed path is unaffected; this method exists only under
    /// `feature = "alloc"`.
    pub fn into_owned(self) -> CodecExtAttachmentOwned {
        CodecExtAttachmentOwned {
            length: self.length,
            body: self.body.to_vec(),
        }
    }
}
