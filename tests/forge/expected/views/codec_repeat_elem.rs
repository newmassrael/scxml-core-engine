#![doc = "SCE-MAP: codec_repeat_elem:5 :: _forge_body"]
// SCE-MAP: codec_repeat_elem:5 :: _forge_body

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


// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecRepeatElem {
    pub seq: u16,
}

#[allow(dead_code)]
impl CodecRepeatElem {
    /// Construct an instance with every field at its own type's
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call. An
    /// enum-typed field starts at the first variant its document
    /// declares, not at the carrier's zero: a closed set does not hold
    /// a value it never declared.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519).
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        let raw = cursor.peek_slice(2)?;
        let seq = ((raw[0] as u16) << 8) | raw[1] as u16;
        let value = Self {
            seq,
        };
        cursor.advance(2)?;
        Ok(value)
    }

    /// [`Self::decode`] into a destination the caller owns, so a codec that
    /// embeds or dispatches to this one decodes it where it will live (see
    /// `decode_into` on a codec that holds a bounded list). This body has no
    /// list of its own to build in place, so it decodes and stores.
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
        *_out = Self::decode(cursor)?;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 2;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        Self::encode_source(self, w)
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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecRepeatElem` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecRepeatElem`
/// computes those from what is ([`CodecRepeatElem::encode_source`]).
pub trait CodecRepeatElemSource {
    /// The `seq` field.
    fn seq(&self) -> u16;
}

impl<T: CodecRepeatElemSource + ?Sized> CodecRepeatElemSource for &T {
    fn seq(&self) -> u16 {
        (**self).seq()
    }
}

impl CodecRepeatElemSource for CodecRepeatElem {
    fn seq(&self) -> u16 {
        self.seq
    }
}

#[allow(dead_code)]
impl CodecRepeatElem {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecRepeatElemSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecRepeatElemSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatElemSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        w.write_u8((src.seq() >> 8 & 0xFF) as u8)?;
        w.write_u8((src.seq() & 0xFF) as u8)?;
        Ok(())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecRepeatElem` holds no bounded list, embedded body or variant, so a
// view of it would hold what it holds. Its view is the codec itself under the
// name every codec's view has, which lets a codec that embeds, lists or
// dispatches to it name `CodecRepeatElemView` whatever kind of codec it is.
#[allow(dead_code)]
pub type CodecRepeatElemView = CodecRepeatElem;

#[allow(dead_code)]
impl CodecRepeatElem {
    /// Fill `_out` with what this value says, as the view of any other codec
    /// does (`CodecRepeatElemView` is this type, so it takes part in the same
    /// call).
    pub fn into_builder(&self, _out: &mut Self) -> Result<(), CodecError> {
        _out.clone_from(self);
        Ok(())
    }
}
