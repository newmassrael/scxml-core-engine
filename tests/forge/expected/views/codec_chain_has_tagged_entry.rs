#![doc = "SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body"]
// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

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
pub struct CodecChainHasTaggedEntry {
    pub entry_type: u8,
    pub ctl: u8,
    pub body: u8,
}

#[allow(dead_code)]
impl CodecChainHasTaggedEntry {
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
        let raw = cursor.peek_slice(3)?;
        let entry_type = raw[0];
        let ctl = raw[1];
        let body = raw[2];
        let value = Self {
            entry_type,
            ctl,
            body,
        };
        cursor.advance(3)?;
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
    pub fn more(&self) -> bool {
        (self.ctl & 0x01) != 0
    }

    pub fn set_more(&mut self, v: bool) {
        if v {
            self.ctl |= 0x01;
        } else {
            self.ctl &= !0x01;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 3;

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

/// Flags of `CodecChainHasTaggedEntry` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecChainHasTaggedEntry`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecChainHasTaggedEntryOverrides {
    /// The `more` flag of `ctl`, when a parent sets it.
    pub ctl_more: Option<bool>,
}

#[allow(dead_code)]
impl CodecChainHasTaggedEntryOverrides {
    /// Sets `more` of `ctl` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn ctl_more(value: bool) -> Self {
        Self { ctl_more: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainHasTaggedEntry` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainHasTaggedEntry`
/// computes those from what is ([`CodecChainHasTaggedEntry::encode_source`]).
pub trait CodecChainHasTaggedEntrySource {
    /// The `entry_type` field.
    fn entry_type(&self) -> u8;
    /// The `ctl` field.
    fn ctl(&self) -> u8;
    /// The `body` field.
    fn body(&self) -> u8;
}

impl<T: CodecChainHasTaggedEntrySource + ?Sized> CodecChainHasTaggedEntrySource for &T {
    fn entry_type(&self) -> u8 {
        (**self).entry_type()
    }
    fn ctl(&self) -> u8 {
        (**self).ctl()
    }
    fn body(&self) -> u8 {
        (**self).body()
    }
}

/// A value of `CodecChainHasTaggedEntry` held in the types a caller already has: a source
/// ([`CodecChainHasTaggedEntrySource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainHasTaggedEntryParts {
    /// The `entry_type` field.
    pub entry_type: u8,
    /// The `ctl` field.
    pub ctl: u8,
    /// The `body` field.
    pub body: u8,
}

impl CodecChainHasTaggedEntrySource for CodecChainHasTaggedEntryParts {
    fn entry_type(&self) -> u8 {
        self.entry_type
    }
    fn ctl(&self) -> u8 {
        self.ctl
    }
    fn body(&self) -> u8 {
        self.body
    }
}

impl CodecChainHasTaggedEntrySource for CodecChainHasTaggedEntry {
    fn entry_type(&self) -> u8 {
        self.entry_type
    }
    fn ctl(&self) -> u8 {
        self.ctl
    }
    fn body(&self) -> u8 {
        self.body
    }
}

#[allow(dead_code)]
impl CodecChainHasTaggedEntry {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecChainHasTaggedEntrySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecChainHasTaggedEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainHasTaggedEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source_in(src, w, &CodecChainHasTaggedEntryOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecChainHasTaggedEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecChainHasTaggedEntryOverrides) -> Result<(), CodecError> {
        let mut _c_ctl: u8 = src.ctl();
        if let Some(_o) = over.ctl_more {
            _c_ctl = (_c_ctl & !0x1) | u8::from(_o);
        }
        w.write_u8(src.entry_type())?;
        w.write_u8(_c_ctl)?;
        w.write_u8(src.body())?;
        Ok(())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecChainHasTaggedEntry` holds no bounded list, embedded body or variant, so a
// view of it would hold what it holds. Its view is the codec itself under the
// name every codec's view has, which lets a codec that embeds, lists or
// dispatches to it name `CodecChainHasTaggedEntryView` whatever kind of codec it is.
#[allow(dead_code)]
pub type CodecChainHasTaggedEntryView = CodecChainHasTaggedEntry;

#[allow(dead_code)]
impl CodecChainHasTaggedEntry {
    /// Fill `_out` with what this value says, as the view of any other codec
    /// does (`CodecChainHasTaggedEntryView` is this type, so it takes part in the same
    /// call).
    pub fn into_builder(&self, _out: &mut Self) -> Result<(), CodecError> {
        _out.clone_from(self);
        Ok(())
    }
}
