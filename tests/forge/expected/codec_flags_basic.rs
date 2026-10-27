#![doc = "SCE-MAP: codec_flags_basic:7 :: _forge_body"]
// SCE-MAP: codec_flags_basic:7 :: _forge_body

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
pub struct CodecFlagsBasic {
    pub header: u8,
}

#[allow(dead_code)]
impl CodecFlagsBasic {
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
        let raw = cursor.peek_slice(1)?;
        let header = raw[0];
        let value = Self {
            header,
        };
        cursor.advance(1)?;
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
    pub fn reliable(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_reliable(&mut self, v: bool) {
        if v {
            self.header |= 0x80;
        } else {
            self.header &= !0x80;
        }
    }

    pub fn more(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_more(&mut self, v: bool) {
        if v {
            self.header |= 0x40;
        } else {
            self.header &= !0x40;
        }
    }

    pub fn drop(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_drop(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn first(&self) -> bool {
        (self.header & 0x10) != 0
    }

    pub fn set_first(&mut self, v: bool) {
        if v {
            self.header |= 0x10;
        } else {
            self.header &= !0x10;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 1;

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

/// Flags of `CodecFlagsBasic` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecFlagsBasic`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecFlagsBasicOverrides {
    /// The `reliable` flag of `header`, when a parent sets it.
    pub header_reliable: Option<bool>,
    /// The `more` flag of `header`, when a parent sets it.
    pub header_more: Option<bool>,
    /// The `drop` flag of `header`, when a parent sets it.
    pub header_drop: Option<bool>,
    /// The `first` flag of `header`, when a parent sets it.
    pub header_first: Option<bool>,
}

#[allow(dead_code)]
impl CodecFlagsBasicOverrides {
    /// Sets `reliable` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_reliable(value: bool) -> Self {
        Self { header_reliable: Some(value), ..Self::default() }
    }

    /// Sets `more` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_more(value: bool) -> Self {
        Self { header_more: Some(value), ..Self::default() }
    }

    /// Sets `drop` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_drop(value: bool) -> Self {
        Self { header_drop: Some(value), ..Self::default() }
    }

    /// Sets `first` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_first(value: bool) -> Self {
        Self { header_first: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecFlagsBasic` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecFlagsBasic`
/// computes those from what is ([`CodecFlagsBasic::encode_source`]).
pub trait CodecFlagsBasicSource {
    /// The `header` field.
    fn header(&self) -> u8;
}

impl<T: CodecFlagsBasicSource + ?Sized> CodecFlagsBasicSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
}

impl CodecFlagsBasicSource for CodecFlagsBasic {
    fn header(&self) -> u8 {
        self.header
    }
}

#[allow(dead_code)]
impl CodecFlagsBasic {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecFlagsBasicSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecFlagsBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecFlagsBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source_in(src, w, &CodecFlagsBasicOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecFlagsBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecFlagsBasicOverrides) -> Result<(), CodecError> {
        let mut _c_header: u8 = src.header();
        if let Some(_o) = over.header_reliable {
            _c_header = (_c_header & !0x80) | (u8::from(_o) << 7);
        }
        if let Some(_o) = over.header_more {
            _c_header = (_c_header & !0x40) | (u8::from(_o) << 6);
        }
        if let Some(_o) = over.header_drop {
            _c_header = (_c_header & !0x20) | (u8::from(_o) << 5);
        }
        if let Some(_o) = over.header_first {
            _c_header = (_c_header & !0x10) | (u8::from(_o) << 4);
        }
        w.write_u8(_c_header)?;
        Ok(())
    }
}
