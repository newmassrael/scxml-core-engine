#![doc = "SCE-MAP: codec_qos_byte:15 :: _forge_body"]
// SCE-MAP: codec_qos_byte:15 :: _forge_body

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
pub struct CodecQosByte {
    pub qos: u8,
}

#[allow(dead_code)]
impl CodecQosByte {
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
        let qos = raw[0];
        let value = Self {
            qos,
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
    pub fn priority(&self) -> u8 {
        self.qos & 0x07
    }

    pub fn set_priority(&mut self, v: u8) {
        self.qos = (self.qos & !0x07) | (v & 0x07);
    }

    pub fn reliable(&self) -> bool {
        (self.qos & 0x08) != 0
    }

    pub fn set_reliable(&mut self, v: bool) {
        if v {
            self.qos |= 0x08;
        } else {
            self.qos &= !0x08;
        }
    }

    pub fn congestion(&self) -> u8 {
        (self.qos >> 4) & 0x03
    }

    pub fn set_congestion(&mut self, v: u8) {
        self.qos = (self.qos & !0x30) | ((v & 0x03) << 4);
    }

    pub fn express(&self) -> bool {
        (self.qos & 0x40) != 0
    }

    pub fn set_express(&mut self, v: bool) {
        if v {
            self.qos |= 0x40;
        } else {
            self.qos &= !0x40;
        }
    }

    pub fn reserved(&self) -> bool {
        (self.qos & 0x80) != 0
    }

    pub fn set_reserved(&mut self, v: bool) {
        if v {
            self.qos |= 0x80;
        } else {
            self.qos &= !0x80;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 1;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        Self::encode_source(self, w)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`, and returns the freshly-encoded bytes: the encode refuses
    /// no value and the vector cannot run out of room. Callers targeting
    /// zero-alloc hot paths should call `encode` directly against a
    /// caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}

/// Flags of `CodecQosByte` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecQosByte`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecQosByteOverrides {
    /// The `reliable` flag of `qos`, when a parent sets it.
    pub qos_reliable: Option<bool>,
    /// The `express` flag of `qos`, when a parent sets it.
    pub qos_express: Option<bool>,
    /// The `reserved` flag of `qos`, when a parent sets it.
    pub qos_reserved: Option<bool>,
}

#[allow(dead_code)]
impl CodecQosByteOverrides {
    /// Sets `reliable` of `qos` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn qos_reliable(value: bool) -> Self {
        Self { qos_reliable: Some(value), ..Self::default() }
    }

    /// Sets `express` of `qos` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn qos_express(value: bool) -> Self {
        Self { qos_express: Some(value), ..Self::default() }
    }

    /// Sets `reserved` of `qos` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn qos_reserved(value: bool) -> Self {
        Self { qos_reserved: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecQosByte` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecQosByte`
/// computes those from what is ([`CodecQosByte::encode_source`]).
pub trait CodecQosByteSource {
    /// The `qos` field.
    fn qos(&self) -> u8;
}

impl<T: CodecQosByteSource + ?Sized> CodecQosByteSource for &T {
    fn qos(&self) -> u8 {
        (**self).qos()
    }
}

/// A value of `CodecQosByte` held in the types a caller already has: a source
/// ([`CodecQosByteSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecQosByteParts {
    /// The `qos` field.
    pub qos: u8,
}

impl CodecQosByteSource for CodecQosByteParts {
    fn qos(&self) -> u8 {
        self.qos
    }
}

impl CodecQosByteSource for CodecQosByte {
    fn qos(&self) -> u8 {
        self.qos
    }
}

#[allow(dead_code)]
impl CodecQosByte {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecQosByteSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecQosByteSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecQosByteSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source_in(src, w, &CodecQosByteOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecQosByteSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecQosByteOverrides) -> Result<(), K::Error> {
        let mut _c_qos: u8 = src.qos();
        if let Some(_o) = over.qos_reliable {
            _c_qos = (_c_qos & !0x8) | (u8::from(_o) << 3);
        }
        if let Some(_o) = over.qos_express {
            _c_qos = (_c_qos & !0x40) | (u8::from(_o) << 6);
        }
        if let Some(_o) = over.qos_reserved {
            _c_qos = (_c_qos & !0x80) | (u8::from(_o) << 7);
        }
        w.write_u8(_c_qos)?;
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecQosByteSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecQosByteSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecQosByte {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecQosByte {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecQosByteSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecQosByteSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
