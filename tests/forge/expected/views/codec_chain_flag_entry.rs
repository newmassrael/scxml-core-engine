#![doc = "SCE-MAP: codec_chain_flag_entry:15 :: _forge_body"]
// SCE-MAP: codec_chain_flag_entry:15 :: _forge_body

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
pub struct CodecChainFlagEntry {
    pub header: u8,
    pub short_value: Option<u8>,
    pub long_value: Option<u16>,
}

#[allow(dead_code)]
impl CodecChainFlagEntry {
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
    pub fn decode(cursor: &mut SceCursor<'_>, wide: u8) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, wide)?;
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
    /// On `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self, wide: u8) -> Result<(), CodecError> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = wide;
        // Streaming cursor decode (SSOT selection: `needs_streaming`).
        // The positional `raw[byte_off]` path is valid only when every
        // field's absolute offset is fixed at codegen time; this branch
        // handles every codec where it is not — present-if-gated fields
        // (runtime presence), VLE / repeat / TLV-chain / embed fields
        // (runtime width), string fields (UTF-8 decode), and a fixed field
        // after a variable-length payload (offset depends on the payload
        // length). Each field reads its own bytes from the cursor and
        // advances past exactly what it consumed. Per-field `is_repeat` /
        // `is_tlv_chain` / `is_embed` route to their dedicated helpers;
        // every other field flows through `present_if_decode_stmt`, whose
        // non-gated arm covers plain fixed / tail / length-ref / VLE reads.
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let short_value = if (wide & 0x01u8) == 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let long_value = if (wide & 0x01u8) != 0 {
            let raw = cursor.peek_slice(2)?;
            let _v = ((raw[0] as u16) << 8) | raw[1] as u16;
            cursor.advance(2)?;
            Some(_v)
        } else {
            None
        };
        _out.header = header;
        _out.short_value = short_value;
        _out.long_value = long_value;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn kind(&self) -> u8 {
        self.header & 0x0F
    }

    pub fn set_kind(&mut self, v: u8) {
        self.header = (self.header & !0x0F) | (v & 0x0F);
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_z(&mut self, v: bool) {
        if v {
            self.header |= 0x80;
        } else {
            self.header &= !0x80;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 4;

    /// Encode `self` into the caller-owned sink. Returns the value's refusal
    /// when the encoding cannot carry it (a length past its type, a list past
    /// its bound, a gate that disagrees with a presence; the document declares
    /// `sce:encode-may-fail`), or the sink's own error:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity.
    pub fn encode<S: SceSink>(&self, w: &mut S, wide: u8) -> Result<(), CodecError> where CodecError: From<S::Error> {
        Self::encode_source(self, w, wide)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector, or the error
    /// `encode` refuses a value with: one the encoding cannot carry (a
    /// length past its type, a list past its bound) or one a gate
    /// disagrees with. Callers targeting zero-alloc hot paths should call
    /// `encode` directly against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self, wide: u8) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink, wide)?;
        Ok(_sce_v)
    }
}

/// Flags of `CodecChainFlagEntry` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecChainFlagEntry`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecChainFlagEntryOverrides {
    /// The `Z` flag of `header`, when a parent sets it.
    pub header_z: Option<bool>,
}

#[allow(dead_code)]
impl CodecChainFlagEntryOverrides {
    /// Sets `Z` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_z(value: bool) -> Self {
        Self { header_z: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainFlagEntry` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainFlagEntry`
/// computes those from what is ([`CodecChainFlagEntry::encode_source`]).
pub trait CodecChainFlagEntrySource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `short_value` field.
    fn short_value(&self) -> Option<u8>;
    /// The `long_value` field.
    fn long_value(&self) -> Option<u16>;
}

impl<T: CodecChainFlagEntrySource + ?Sized> CodecChainFlagEntrySource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn short_value(&self) -> Option<u8> {
        (**self).short_value()
    }
    fn long_value(&self) -> Option<u16> {
        (**self).long_value()
    }
}

/// A value of `CodecChainFlagEntry` held in the types a caller already has: a source
/// ([`CodecChainFlagEntrySource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainFlagEntryParts {
    /// The `header` field.
    pub header: u8,
    /// The `short_value` field.
    pub short_value: Option<u8>,
    /// The `long_value` field.
    pub long_value: Option<u16>,
}

impl CodecChainFlagEntrySource for CodecChainFlagEntryParts {
    fn header(&self) -> u8 {
        self.header
    }
    fn short_value(&self) -> Option<u8> {
        self.short_value
    }
    fn long_value(&self) -> Option<u16> {
        self.long_value
    }
}

impl CodecChainFlagEntrySource for CodecChainFlagEntry {
    fn header(&self) -> u8 {
        self.header
    }
    fn short_value(&self) -> Option<u8> {
        self.short_value
    }
    fn long_value(&self) -> Option<u16> {
        self.long_value
    }
}

#[allow(dead_code)]
impl CodecChainFlagEntry {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecChainFlagEntrySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecChainFlagEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K, wide: u8) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w, wide)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainFlagEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K, wide: u8) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source_in(src, w, wide, &CodecChainFlagEntryOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecChainFlagEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K, wide: u8, over: &CodecChainFlagEntryOverrides) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let mut _c_header: u8 = src.header();
        if let Some(_o) = over.header_z {
            _c_header = (_c_header & !0x80) | (u8::from(_o) << 7);
        }
        if ((wide & 0x1) == 0) != src.short_value().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        if ((wide & 0x1) != 0) != src.long_value().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        let _ = wide;
        w.write_u8(_c_header)?;
        if let Some(_v) = src.short_value() {
            w.write_u8(_v)?;
        }
        if let Some(_v) = src.long_value() {
            w.write_u8((_v >> 8 & 0xFF) as u8)?;
            w.write_u8((_v & 0xFF) as u8)?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecChainFlagEntrySource + ?Sized>(src: &S, wide: u8) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c, wide)?;
        Ok(_c.len())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecChainFlagEntry` holds no bounded list, embedded body or variant, so a
// view of it would hold what it holds. Its view is the codec itself under the
// name every codec's view has, which lets a codec that embeds, lists or
// dispatches to it name `CodecChainFlagEntryView` whatever kind of codec it is.
#[allow(dead_code)]
pub type CodecChainFlagEntryView = CodecChainFlagEntry;

#[allow(dead_code)]
impl CodecChainFlagEntry {
    /// Fill `_out` with what this value says, as the view of any other codec
    /// does (`CodecChainFlagEntryView` is this type, so it takes part in the same
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
