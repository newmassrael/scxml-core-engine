#![doc = "SCE-MAP: codec_peek_arm_a:13 :: _forge_body"]
// SCE-MAP: codec_peek_arm_a:13 :: _forge_body

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
pub struct CodecPeekArmA {
    pub header: u8,
    pub payload: u8,
}

#[allow(dead_code)]
impl CodecPeekArmA {
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
        let header = raw[0];
        let payload = raw[1];
        let value = Self {
            header,
            payload,
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

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn kind(&self) -> bool {
        (self.header & 0x01) != 0
    }

    pub fn set_kind(&mut self, v: bool) {
        if v {
            self.header |= 0x01;
        } else {
            self.header &= !0x01;
        }
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
    /// `VecSink`. Returns the freshly-encoded byte vector, or the error
    /// `encode` refuses a value with: one the encoding cannot carry (a
    /// length past its type, a list past its bound) or one a gate
    /// disagrees with. It is a `Result` for every codec, so that a codec
    /// gaining a derived element, or a build that gates one in, does not
    /// change what a caller writes. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}

/// Flags of `CodecPeekArmA` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecPeekArmA`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecPeekArmAOverrides {
    /// The `kind` flag of `header`, when a parent sets it.
    pub header_kind: Option<bool>,
}

#[allow(dead_code)]
impl CodecPeekArmAOverrides {
    /// Sets `kind` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_kind(value: bool) -> Self {
        Self { header_kind: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecPeekArmA` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecPeekArmA`
/// computes those from what is ([`CodecPeekArmA::encode_source`]).
pub trait CodecPeekArmASource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `payload` field.
    fn payload(&self) -> u8;
}

impl<T: CodecPeekArmASource + ?Sized> CodecPeekArmASource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn payload(&self) -> u8 {
        (**self).payload()
    }
}

impl CodecPeekArmASource for CodecPeekArmA {
    fn header(&self) -> u8 {
        self.header
    }
    fn payload(&self) -> u8 {
        self.payload
    }
}

#[allow(dead_code)]
impl CodecPeekArmA {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecPeekArmASource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecPeekArmASource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecPeekArmASource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source_in(src, w, &CodecPeekArmAOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecPeekArmASource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecPeekArmAOverrides) -> Result<(), CodecError> {
        let mut _c_header: u8 = src.header();
        if let Some(_o) = over.header_kind {
            _c_header = (_c_header & !0x1) | u8::from(_o);
        }
        w.write_u8(_c_header)?;
        w.write_u8(src.payload())?;
        Ok(())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecPeekArmA` holds no bounded list, embedded body or variant, so a
// view of it would hold what it holds. Its view is the codec itself under the
// name every codec's view has, which lets a codec that embeds, lists or
// dispatches to it name `CodecPeekArmAView` whatever kind of codec it is.
#[allow(dead_code)]
pub type CodecPeekArmAView = CodecPeekArmA;

#[allow(dead_code)]
impl CodecPeekArmA {
    /// Fill `_out` with what this value says, as the view of any other codec
    /// does (`CodecPeekArmAView` is this type, so it takes part in the same
    /// call).
    pub fn into_builder(&self, _out: &mut Self) -> Result<(), CodecError> {
        _out.clone_from(self);
        Ok(())
    }
}
