#![doc = "SCE-MAP: codec_zenoh_fragment:23"]
// SCE-MAP: codec_zenoh_fragment:23

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
pub struct CodecZenohFragment<'a> {
    pub sn: u64,
    pub payload: &'a [u8],
}

#[allow(dead_code)]
impl<'a> CodecZenohFragment<'a> {
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
        // Streaming codec: each field reads its own bytes from the
        // cursor (VLE = base-128 1..=ceil(N/7) bytes). No pre-peek of
        // a fixed window; cursor advances per-field. RFC §5.B B4:
        // per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        let sn = cursor.read_vle_u64()?;
        let payload = {
            let _n = cursor.remaining();
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            _v
        };
        Ok(Self {
            sn,
            payload,
        })
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 65546;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef / Tail siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable: the non-gated VLE arm there reuses
        // `vle_encode_block` with the language-appropriate self/
        // struct prefix.
        {
            let mut _vle = self.sn;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
        }
        w.write_bytes(self.payload)?;
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
// `CodecZenohFragment<'a>` above is a zero-copy view borrowing the decode
// buffer. AP / async consumers that persist a decoded message beyond the
// buffer's lifetime call `.into_owned()` for this lifetime-free
// `CodecZenohFragmentOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split — both generated from the one SCXML source (SSOT). `Vec`
// / `String` are alloc, so the whole projection is gated; the no-alloc
// borrowed path above is untouched.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohFragmentOwned {
    pub sn: u64,
    pub payload: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohFragment<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohFragmentOwned`] (alloc). Call at a decode boundary when
    /// the decoded value must outlive the input buffer — e.g. stored in a
    /// long-lived enum or moved across an async task. The no-alloc
    /// borrowed path is unaffected; this method exists only under
    /// `feature = "alloc"`.
    pub fn into_owned(self) -> CodecZenohFragmentOwned {
        CodecZenohFragmentOwned {
            sn: self.sn,
            payload: self.payload.to_vec(),
        }
    }
}

#[cfg(feature = "alloc")]
impl CodecZenohFragmentOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohFragment<'_> {
        CodecZenohFragment {
            sn: self.sn,
            payload: self.payload.as_slice(),
        }
    }
}
