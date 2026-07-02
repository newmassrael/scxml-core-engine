#![doc = "SCE-MAP: codec_dma_aligned_basic:20"]
// SCE-MAP: codec_dma_aligned_basic:20

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// RFC §synth-5-B: `VecSink` and the heap-backed `encode_to_vec` facade
// are gated on the `alloc` feature (see
// `backends/rust/forge-runtime/src/codec.rs`). MCU / `no_std` consumers see
// only the sink-based primary `encode` + `SliceSink` paths.
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use sce_forge_runtime::codec::VecSink;
// RFC §synth-5-B B3 DMA alignment primitive: structural drift detection.
// Build-time validation already guaranteed `byte_offset % burst_align
// == 0` and that all preceding fields are Fixed bit-size. These
// `const _: () = assert!` declarations catch any future hand-edit to
// the byte_offset that would break the wire-layout invariant.
// `byte_off` and `dma_burst_align` are baked from independent SCXML inputs,
// but clippy sees only integer literals: the check reads as a constant
// (assertions_on_constants) and, whenever the offset equals the alignment,
// as `x % x` (eq_op, deny-by-default). The assert is nonetheless a genuine
// compile-time drift guard — a future hand-edit that misaligns the offset
// must fail to compile — so the lints are suppressed at this site only.
#[allow(clippy::eq_op, clippy::assertions_on_constants)]
const _: () = assert!(
    32 % 32 == 0,
    "RFC §synth-5-B B3: codec field 'aligned_payload' offset must be 32-aligned"
);

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecDmaAlignedBasic<'a> {
    pub msg_id: u8,
    pub reserved: u8,
    pub aligned_payload: &'a [u8],
}

#[allow(dead_code)]
impl<'a> CodecDmaAlignedBasic<'a> {
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
    /// bytes (RFC §synth-5-B L494-519).
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        // Variable-length codec. RFC §synth-5-B B3 stream-correct shape:
        // a codec without `<sce:field sce:bit-size="tail">` consumes
        // only `min_bytes + length_value` rather than the entire
        // cursor remaining. Codecs WITH a tail field still consume
        // to end (tail's definition forces it). The prior
        // "consume entire cursor" behaviour deferred to "the first
        // multi-frame consumer" — the TLV chain is that consumer,
        // so length-ref entry codecs now decode-iterably from a
        // shared cursor without each entry eating the next entry's
        // bytes.
        let _frame_len = cursor.remaining();
        if _frame_len < 2 {
            return Err(CodecError::NeedMoreBytes);
        }
        let raw = cursor.peek_slice(_frame_len)?;
        let msg_id = raw[0];
        let reserved = raw[1];
        let aligned_payload = &raw[32..];
        let value = Self {
            msg_id,
            reserved,
            aligned_payload,
        };
        cursor.advance(_frame_len)?;
        Ok(value)
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 66;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        w.write_u8(self.msg_id)?;
        w.write_u8(self.reserved)?;
        // RFC §synth-5-B B3 DMA padding: zero-fill any gap between the
        // current write position and this field's authored byte
        // offset (deterministic zeros on the wire so peers stay
        // byte-compatible regardless of host allocator — RFC §synth-5-B
        // "wire layout, not host allocator").
        while w.position() < 32 {
            w.write_u8(0)?;
        }
        // `self.<id>` is the borrowed `&'a [u8]` view — pass it directly;
        // `&self.<id>` would be `&&[u8]` (clippy::needless_borrow).
        w.write_bytes(self.aligned_payload)?;
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
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
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

// ── Owned projection (portable native form) ───────────────────────────
// `CodecDmaAlignedBasic<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecDmaAlignedBasicOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
// `String` / `Bytes` fields project to the portable runtime newtypes
// `SceString<N>` / `SceBytes<N>`: an unbounded `String` / `Vec<u8>` under
// `alloc` (the on-wire protocol caps no payload, so the AP profile must
// not either — `N` is advisory) and the heap-free `heapless::String<N>` /
// `heapless::Vec<u8, N>` (the C11 `char[N]` analog) without it, where `N`
// is the hard capacity. A leaf codec's owned form therefore still compiles
// on a no-alloc MCU; only an unbounded owned `Vec` (list / embed / variant
// body) keeps the `alloc` gate. `try_into_owned` stays the fallible
// direction (one `?` per profile: the `alloc` copy cannot fail, the
// no-alloc copy enforces `N`); `as_borrowed` re-borrows either
// form infallibly via `.as_slice()` / `.as_str()`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodecDmaAlignedBasicOwned {
    pub msg_id: u8,
    pub reserved: u8,
    pub aligned_payload: ::sce_forge_runtime::codec::SceBytes<64>,
}

impl<'a> CodecDmaAlignedBasic<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecDmaAlignedBasicOwned`]. Call at a decode boundary when the
    /// decoded value must outlive the input buffer — stored in a long-lived
    /// enum, moved across an async task, or inserted by value into a
    /// bounded-collection. `String` / `Bytes` fields copy into the portable
    /// `SceString<N>` / `SceBytes<N>`: an unbounded heap copy under `alloc`
    /// (`N` advisory), else a fixed `heapless` copy capped at `N`. The
    /// method is fallible for profile uniformity — without `alloc` an
    /// over-`N` view raises `CodecError::TooManyElements` (the same bound
    /// and error decode enforces); under `alloc` the copy never fails. The
    /// borrowed zero-copy path is unaffected.
    pub fn try_into_owned(self) -> Result<CodecDmaAlignedBasicOwned, CodecError> {
        Ok(CodecDmaAlignedBasicOwned {
            msg_id: self.msg_id,
            reserved: self.reserved,
            aligned_payload: ::sce_forge_runtime::codec::SceBytes::from_slice(self.aligned_payload)?,
        })
    }
}

impl CodecDmaAlignedBasicOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecDmaAlignedBasic<'_> {
        CodecDmaAlignedBasic {
            msg_id: self.msg_id,
            reserved: self.reserved,
            aligned_payload: self.aligned_payload.as_slice(),
        }
    }
}
