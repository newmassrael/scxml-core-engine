#![doc = "SCE-MAP: codec_chain_has_tagged:11 :: _forge_body"]
// SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

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
// RFC §synth-5-B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntry;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecChainHasTagged {
    pub header: u8,
    pub entries: HeaplessVec<CodecChainHasTaggedEntry, 3>,
    pub priority: Option<u8>,
    pub checksum: Option<u16>,
}

#[allow(dead_code)]
impl CodecChainHasTagged {
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
    /// On `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
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
        {
            let _vec = &mut _out.entries;
            _vec.clear();
            let mut _more = false;
            for _ in 0..3u32 {
                if cursor.remaining() == 0 { break; }
                let _entry = CodecChainHasTaggedEntry::decode(cursor)?;
                _more = _entry.more();
                // Bounded by max-depth on both sides — loop count and `_vec`
                // capacity are the same literal — so this push cannot fail. An
                // over-long chain is refused by the guard after the loop.
                _vec.push(_entry).map_err(|_| CodecError::TooManyElements)?;
                if !_more { break; }
            }
            if _more && cursor.remaining() == 0 {
                return Err(CodecError::NeedMoreBytes);
            }
            if _more {
                return Err(CodecError::TlvChainOverflow);
            }
        }
        let _has_entries_7 = _out.entries.iter().any(|_e| u64::from(_e.entry_type) == 7u64);
        let _has_entries_9 = _out.entries.iter().any(|_e| u64::from(_e.entry_type) == 9u64);
        let priority = if _has_entries_7 || (header & 0x01u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let checksum = if !_has_entries_9 {
            let raw = cursor.peek_slice(2)?;
            let _v = ((raw[0] as u16) << 8) | raw[1] as u16;
            cursor.advance(2)?;
            Some(_v)
        } else {
            None
        };
        _out.header = header;
        _out.priority = priority;
        _out.checksum = checksum;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn wide(&self) -> bool {
        (self.header & 0x01) != 0
    }

    pub fn set_wide(&mut self, v: bool) {
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
    pub const MAX_ENCODED_BYTES: usize = 14;

    /// Encode `self` into the caller-owned sink. Returns the value's refusal
    /// when the encoding cannot carry it (a length past its type, a list past
    /// its bound, a gate that disagrees with a presence; the document declares
    /// `sce:encode-may-fail`), or the sink's own error:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> where CodecError: From<S::Error> {
        Self::encode_source(self, w)
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
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}

/// Flags of `CodecChainHasTagged` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecChainHasTagged`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecChainHasTaggedOverrides {
    /// The `wide` flag of `header`, when a parent sets it.
    pub header_wide: Option<bool>,
}

#[allow(dead_code)]
impl CodecChainHasTaggedOverrides {
    /// Sets `wide` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_wide(value: bool) -> Self {
        Self { header_wide: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainHasTagged` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainHasTagged`
/// computes those from what is ([`CodecChainHasTagged::encode_source`]).
pub trait CodecChainHasTaggedSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `entries` field.
    fn entries(&self) -> impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone;
    /// The `priority` field.
    fn priority(&self) -> Option<u8>;
    /// The `checksum` field.
    fn checksum(&self) -> Option<u16>;
}

impl<T: CodecChainHasTaggedSource + ?Sized> CodecChainHasTaggedSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn entries(&self) -> impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone {
        (**self).entries()
    }
    fn priority(&self) -> Option<u8> {
        (**self).priority()
    }
    fn checksum(&self) -> Option<u16> {
        (**self).checksum()
    }
}

/// A value of `CodecChainHasTagged` held in the types a caller already has: a source
/// ([`CodecChainHasTaggedSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainHasTaggedParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `entries` field.
    pub entries: &'a [super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntryParts],
    /// The `priority` field.
    pub priority: Option<u8>,
    /// The `checksum` field.
    pub checksum: Option<u16>,
}

impl<'a> CodecChainHasTaggedSource for CodecChainHasTaggedParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn entries(&self) -> impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone {
        self.entries.iter()
    }
    fn priority(&self) -> Option<u8> {
        self.priority
    }
    fn checksum(&self) -> Option<u16> {
        self.checksum
    }
}

impl CodecChainHasTaggedSource for CodecChainHasTagged {
    fn header(&self) -> u8 {
        self.header
    }
    fn entries(&self) -> impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone {
        self.entries.iter()
    }
    fn priority(&self) -> Option<u8> {
        self.priority
    }
    fn checksum(&self) -> Option<u16> {
        self.checksum
    }
}

#[allow(dead_code)]
impl CodecChainHasTagged {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecChainHasTaggedSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecChainHasTaggedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainHasTaggedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source_in(src, w, &CodecChainHasTaggedOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecChainHasTaggedSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecChainHasTaggedOverrides) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let _k_entries: usize = src.entries().count();
        if _k_entries > 3 {
            return Err(CodecError::TlvChainOverflow);
        }
        let mut _c_header: u8 = src.header();
        if let Some(_o) = over.header_wide {
            _c_header = (_c_header & !0x1) | u8::from(_o);
        }
        if (src.entries().any(|_e| u64::from(super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource::entry_type(&_e)) == 7u64) || (_c_header & 0x1) != 0) != src.priority().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        if (!src.entries().any(|_e| u64::from(super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource::entry_type(&_e)) == 9u64)) != src.checksum().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        w.write_u8(_c_header)?;
        for (_i, _e) in src.entries().enumerate() {
            CodecChainHasTaggedEntry::write_source_in(&_e, w, &super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntryOverrides::ctl_more(_i + 1 < _k_entries))?;
        }
        if let Some(_v) = src.priority() {
            w.write_u8(_v)?;
        }
        if let Some(_v) = src.checksum() {
            w.write_u8((_v >> 8 & 0xFF) as u8)?;
            w.write_u8((_v & 0xFF) as u8)?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecChainHasTaggedSource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<S: CodecChainHasTaggedSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecChainHasTagged {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecChainHasTagged {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecChainHasTaggedSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecChainHasTaggedSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
