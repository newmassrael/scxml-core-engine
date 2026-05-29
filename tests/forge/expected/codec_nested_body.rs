#![doc = "SCE-MAP: codec_nested_body:18"]
// SCE-MAP: codec_nested_body:18

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
// RFC §5.B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_zenoh_locator::CodecZenohLocator;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecNestedBody<'a> {
    pub n: u8,
    pub locs: HeaplessVec<CodecZenohLocator<'a>, 4>,
}

#[allow(dead_code)]
impl<'a> CodecNestedBody<'a> {
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
        // RFC §5.B B2 repeat / B3 TLV chain primitives: streaming
        // decode mixes plain fixed-width reads (per-field via the
        // present-if helper's non-gated arm) with repeat loops that
        // iterate the imported codec's `decode()`. Repeat: bounded by
        // `count_ref` (length-field) or cursor exhaustion (until-eof).
        // TLV chain: bounded by `max_depth` with on-overflow check.
        // Element bodies recurse into their own codec — each may
        // itself surface NeedMoreBytes, unwinding the partial frame.
        let n = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let locs = {
            let mut _vec: HeaplessVec<CodecZenohLocator<'a>, 4> = HeaplessVec::new();
            for _ in 0..n {
                _vec.push(CodecZenohLocator::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
            _vec
        };
        Ok(Self {
            n,
            locs,
        })
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 553;

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
        w.write_u8(self.n)?;
        for _e in &self.locs {
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
// `CodecNestedBody<'a>` above is a zero-copy view borrowing the decode
// buffer. AP / async consumers that persist a decoded message beyond the
// buffer's lifetime call `.into_owned()` for this lifetime-free
// `CodecNestedBodyOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split — both generated from the one SCXML source (SSOT). `Vec`
// / `String` are alloc, so the whole projection is gated; the no-alloc
// borrowed path above is untouched.
#[cfg(feature = "alloc")]
use super::codec_zenoh_locator::CodecZenohLocatorOwned;
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecNestedBodyOwned {
    pub n: u8,
    pub locs: Vec<CodecZenohLocatorOwned>,
}

#[cfg(feature = "alloc")]
impl<'a> CodecNestedBody<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecNestedBodyOwned`] (alloc). Call at a decode boundary when
    /// the decoded value must outlive the input buffer — e.g. stored in a
    /// long-lived enum or moved across an async task. The no-alloc
    /// borrowed path is unaffected; this method exists only under
    /// `feature = "alloc"`.
    pub fn into_owned(self) -> CodecNestedBodyOwned {
        CodecNestedBodyOwned {
            n: self.n,
            locs: self.locs.into_iter().map(|_e| _e.into_owned()).collect(),
        }
    }
}

#[cfg(feature = "alloc")]
impl CodecNestedBodyOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `try_as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list whose
    /// owned `Vec` holds more than its declared `N` raises
    /// `CodecError::TooManyElements` — the same bound decode enforces.
    pub fn try_as_borrowed(&self) -> Result<CodecNestedBody<'_>, CodecError> {
        Ok(CodecNestedBody {
            n: self.n,
            locs: sce_forge_runtime::codec::try_project_bounded(&self.locs, |_e| Ok(_e.as_borrowed()))?,
        })
    }
}
