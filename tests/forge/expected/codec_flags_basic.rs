#![doc = "SCE-MAP: codec_flags_basic:7 :: _forge_body"]
// SCE-MAP: codec_flags_basic:7 :: _forge_body

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

    /// Decode the next frame from `cursor`. On success the cursor advances
    /// past the consumed bytes; on a refusal it does not move, so a caller that
    /// receives `NeedMoreBytes` reads the frame again from the same place once
    /// more bytes have arrived (RFC §synth-5-B L494-519, docs/adr/0128).
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        cursor.read_frame(Self::read)
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
    /// once the frame is read (docs/adr/0128). A codec that embeds or
    /// dispatches to this one reads it through this one function, out of line
    /// where the set reads it from two sites or more so that no site holds a
    /// copy (docs/adr/0136, docs/adr/0137).
    pub fn read(cursor: &mut SceReader<'_>) -> Self {
        let raw = cursor.take_array::<1>();
        let header = raw[0];
        Self {
            header,
        }
    }

    /// [`Self::read`] into a destination the caller owns, so a codec that
    /// embeds or dispatches to this one reads it where it will live.
    pub fn read_into(cursor: &mut SceReader<'_>, _out: &mut Self) {
        *_out = Self::read(cursor);
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

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecFlagsBasicWire::encode_source(self, w)
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
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecFlagsBasicWire::encode_source`]).
pub trait CodecFlagsBasicSource {
    /// The `header` field.
    fn header(&self) -> u8;
}

impl<T: CodecFlagsBasicSource + ?Sized> CodecFlagsBasicSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
}

/// A value of `CodecFlagsBasic` held in the types a caller already has: a source
/// ([`CodecFlagsBasicSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecFlagsBasicParts {
    /// The `header` field.
    pub header: u8,
}

impl CodecFlagsBasicSource for CodecFlagsBasicParts {
    fn header(&self) -> u8 {
        self.header
    }
}

#[allow(dead_code)]
impl CodecFlagsBasicParts {
    /// The parts of what `src` says, each accessor of [`CodecFlagsBasicSource`] read
    /// once: the one form a codec that holds this one writes it from,
    /// whatever form the value it is handed is in (docs/adr/0058).
    #[doc(hidden)]
    pub fn of<S: CodecFlagsBasicSource + ?Sized>(src: &S) -> Self {
        Self {
            header: src.header(),
        }
    }
}

impl CodecFlagsBasicSource for CodecFlagsBasic {
    fn header(&self) -> u8 {
        self.header
    }
}

/// The codec `CodecFlagsBasic` itself: what writes a value read through
/// [`CodecFlagsBasicSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecFlagsBasicWire;

#[allow(dead_code)]
impl CodecFlagsBasicWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecFlagsBasic::encode` writes for the value it holds, read through
    /// [`CodecFlagsBasicSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecFlagsBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecFlagsBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source_in(src, w, &CodecFlagsBasicOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents. `src` is read once into `CodecFlagsBasicParts`, whatever form
    /// it is in, and the wire is written from that one form (`write_parts_in`).
    #[doc(hidden)]
    pub fn write_source_in<S: CodecFlagsBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecFlagsBasicOverrides) -> Result<(), K::Error> {
        Self::write_parts_in(&CodecFlagsBasicParts::of(src), w, over)
    }

    /// The wire of `src`, the one form every source of this codec is read
    /// into: one writer for each sink, however many forms are written. It is
    /// out of line unless every write is asked inlined (`--inline-encode`):
    /// inlined into `write_source_in`, it is copied into each form.
    #[inline(never)]
    fn write_parts_in<K: SceSink>(src: &CodecFlagsBasicParts, w: &mut K, over: &CodecFlagsBasicOverrides) -> Result<(), K::Error> {
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

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecFlagsBasicSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecFlagsBasicSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecFlagsBasicWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecFlagsBasicWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecFlagsBasicSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecFlagsBasicSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
