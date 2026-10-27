#![doc = "SCE-MAP: codec_subbyte:3 :: _forge_body"]
// SCE-MAP: codec_subbyte:3 :: _forge_body

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
pub struct CodecSubbyte {
    pub priority: u8,
    pub channel: u8,
    pub direction: u8,
}

#[allow(dead_code)]
impl CodecSubbyte {
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
        let priority = (raw[0] >> 5) & 0x07;
        let channel = (raw[0] >> 2) & 0x07;
        let direction = raw[0] & 0x03;
        let value = Self {
            priority,
            channel,
            direction,
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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecSubbyte` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecSubbyte`
/// computes those from what is ([`CodecSubbyte::encode_source`]).
pub trait CodecSubbyteSource {
    /// The `priority` field.
    fn priority(&self) -> u8;
    /// The `channel` field.
    fn channel(&self) -> u8;
    /// The `direction` field.
    fn direction(&self) -> u8;
}

impl<T: CodecSubbyteSource + ?Sized> CodecSubbyteSource for &T {
    fn priority(&self) -> u8 {
        (**self).priority()
    }
    fn channel(&self) -> u8 {
        (**self).channel()
    }
    fn direction(&self) -> u8 {
        (**self).direction()
    }
}

/// A value of `CodecSubbyte` held in the types a caller already has: a source
/// ([`CodecSubbyteSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecSubbyteParts {
    /// The `priority` field.
    pub priority: u8,
    /// The `channel` field.
    pub channel: u8,
    /// The `direction` field.
    pub direction: u8,
}

impl CodecSubbyteSource for CodecSubbyteParts {
    fn priority(&self) -> u8 {
        self.priority
    }
    fn channel(&self) -> u8 {
        self.channel
    }
    fn direction(&self) -> u8 {
        self.direction
    }
}

impl CodecSubbyteSource for CodecSubbyte {
    fn priority(&self) -> u8 {
        self.priority
    }
    fn channel(&self) -> u8 {
        self.channel
    }
    fn direction(&self) -> u8 {
        self.direction
    }
}

#[allow(dead_code)]
impl CodecSubbyte {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecSubbyteSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecSubbyteSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecSubbyteSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        w.write_u8(((src.priority() & 0x07) << 5) | ((src.channel() & 0x07) << 2) | (src.direction() & 0x03))?;
        Ok(())
    }
}

impl<S: CodecSubbyteSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecSubbyte {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecSubbyte {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecSubbyteSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }
}
