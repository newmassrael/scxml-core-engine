#![doc = "SCE-MAP: codec_repeat_elem:5 :: _forge_body"]
// SCE-MAP: codec_repeat_elem:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceReader, SceSink};
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

    /// Decode the next frame from `cursor`. On success the cursor advances
    /// past the consumed bytes; on a refusal it does not move, so a caller that
    /// receives `NeedMoreBytes` reads the frame again from the same place once
    /// more bytes have arrived (RFC §synth-5-B L494-519, docs/adr/0128).
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out)?;
        Ok(_out)
    }

    /// Decode the next frame from `cursor` into `_out`: the bytes, the bounds
    /// and the errors of [`Self::decode`], with each bounded list and embedded
    /// body built in the field where it will live.
    ///
    /// `decode` returns its value, and a `heapless::Vec` moves by copying its
    /// whole declared capacity, so every hop a decoded list takes on the way
    /// up (the struct literal, `Ok(..)`, a variant wrapper, the caller's own
    /// binding) copies the declared maximum however few entries the wire held.
    /// A caller on a path where that shows decodes into a destination it owns.
    ///
    /// A field past what the profile holds is read on and not held, and the
    /// frame is refused with `CapacityExceeded` only once it is read whole and
    /// every refusal of its own has had its turn (docs/adr/0077 §7), so this
    /// node and one that holds more refuse a malformed frame alike. On `Err`,
    /// `_out` holds the fields decoded so far and is not to be read; each of
    /// them holds what its type says.
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
        cursor.read_frame(|cursor| Self::read_into(cursor, _out))
    }

    /// Read the frame through `cursor`, the frame's reader, as a value built
    /// once from its fields: every read total, each refusal recorded in the
    /// reader and the first of them the frame's verdict, which the reader says
    /// once the frame is read (docs/adr/0128).
    pub fn read(cursor: &mut SceReader<'_>) -> Self {
        let raw = cursor.take_array::<2>();
        let seq = ((raw[0] as u16) << 8) | raw[1] as u16;
        Self {
            seq,
        }
    }

    /// [`Self::read`] into a destination the caller owns, so a codec that
    /// embeds or dispatches to this one reads it where it will live.
    pub fn read_into(cursor: &mut SceReader<'_>, _out: &mut Self) {
        *_out = Self::read(cursor);
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 2;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecRepeatElemWire::encode_source(self, w)
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
/// What a value of `CodecRepeatElem` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecRepeatElemWire::encode_source`]).
pub trait CodecRepeatElemSource {
    /// The `seq` field.
    fn seq(&self) -> u16;
}

impl<T: CodecRepeatElemSource + ?Sized> CodecRepeatElemSource for &T {
    fn seq(&self) -> u16 {
        (**self).seq()
    }
}

/// A value of `CodecRepeatElem` held in the types a caller already has: a source
/// ([`CodecRepeatElemSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecRepeatElemParts {
    /// The `seq` field.
    pub seq: u16,
}

impl CodecRepeatElemSource for CodecRepeatElemParts {
    fn seq(&self) -> u16 {
        self.seq
    }
}

#[allow(dead_code)]
impl CodecRepeatElemParts {
    /// The parts of what `src` says, each accessor of [`CodecRepeatElemSource`] read
    /// once: the one form a codec that holds this one writes it from,
    /// whatever form the value it is handed is in (docs/adr/0058).
    #[doc(hidden)]
    pub fn of<S: CodecRepeatElemSource + ?Sized>(src: &S) -> Self {
        Self {
            seq: src.seq(),
        }
    }
}

impl CodecRepeatElemSource for CodecRepeatElem {
    fn seq(&self) -> u16 {
        self.seq
    }
}

/// The codec `CodecRepeatElem` itself: what writes a value read through
/// [`CodecRepeatElemSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecRepeatElemWire;

#[allow(dead_code)]
impl CodecRepeatElemWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecRepeatElem::encode` writes for the value it holds, read through
    /// [`CodecRepeatElemSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecRepeatElemSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatElemSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        w.write_u8((src.seq() >> 8 & 0xFF) as u8)?;
        w.write_u8((src.seq() & 0xFF) as u8)?;
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecRepeatElemSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecRepeatElemSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecRepeatElemWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecRepeatElemWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecRepeatElemSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecRepeatElemSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
