#![doc = "SCE-MAP: codec_simple_frame:3 :: _forge_body"]
// SCE-MAP: codec_simple_frame:3 :: _forge_body

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
pub struct CodecSimpleFrame {
    pub msg_id: u8,
    pub length: u8,
    pub payload: u16,
}

#[allow(dead_code)]
impl CodecSimpleFrame {
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
        let raw = cursor.peek_slice(4)?;
        let msg_id = raw[0];
        let length = raw[1];
        let payload = ((raw[2] as u16) << 8) | raw[3] as u16;
        let value = Self {
            msg_id,
            length,
            payload,
        };
        cursor.advance(4)?;
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
    pub const MAX_ENCODED_BYTES: usize = 4;

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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecSimpleFrame` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecSimpleFrame`
/// computes those from what is ([`CodecSimpleFrame::encode_source`]).
pub trait CodecSimpleFrameSource {
    /// The `msgId` field.
    fn msg_id(&self) -> u8;
    /// The `length` field.
    fn length(&self) -> u8;
    /// The `payload` field.
    fn payload(&self) -> u16;
}

impl<T: CodecSimpleFrameSource + ?Sized> CodecSimpleFrameSource for &T {
    fn msg_id(&self) -> u8 {
        (**self).msg_id()
    }
    fn length(&self) -> u8 {
        (**self).length()
    }
    fn payload(&self) -> u16 {
        (**self).payload()
    }
}

/// A value of `CodecSimpleFrame` held in the types a caller already has: a source
/// ([`CodecSimpleFrameSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecSimpleFrameParts {
    /// The `msgId` field.
    pub msg_id: u8,
    /// The `length` field.
    pub length: u8,
    /// The `payload` field.
    pub payload: u16,
}

impl CodecSimpleFrameSource for CodecSimpleFrameParts {
    fn msg_id(&self) -> u8 {
        self.msg_id
    }
    fn length(&self) -> u8 {
        self.length
    }
    fn payload(&self) -> u16 {
        self.payload
    }
}

impl CodecSimpleFrameSource for CodecSimpleFrame {
    fn msg_id(&self) -> u8 {
        self.msg_id
    }
    fn length(&self) -> u8 {
        self.length
    }
    fn payload(&self) -> u16 {
        self.payload
    }
}

#[allow(dead_code)]
impl CodecSimpleFrame {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecSimpleFrameSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecSimpleFrameSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecSimpleFrameSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        w.write_u8(src.msg_id())?;
        w.write_u8(src.length())?;
        w.write_u8((src.payload() >> 8 & 0xFF) as u8)?;
        w.write_u8((src.payload() & 0xFF) as u8)?;
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecSimpleFrameSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecSimpleFrameSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecSimpleFrame {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecSimpleFrame {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecSimpleFrameSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecSimpleFrameSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecSimpleFrame` holds no bounded list, embedded body or variant, so a
// view of it would hold what it holds. Its view is the codec itself under the
// name every codec's view has, which lets a codec that embeds, lists or
// dispatches to it name `CodecSimpleFrameView` whatever kind of codec it is.
#[allow(dead_code)]
pub type CodecSimpleFrameView = CodecSimpleFrame;

#[allow(dead_code)]
impl CodecSimpleFrame {
    /// Fill `_out` with what this value says, as the view of any other codec
    /// does (`CodecSimpleFrameView` is this type, so it takes part in the same
    /// call).
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder(&self, _out: &mut Self) -> Result<(), CodecError> {
        _out.clone_from(self);
        Ok(())
    }
}
