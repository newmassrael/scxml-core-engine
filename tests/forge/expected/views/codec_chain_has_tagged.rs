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

use sce_forge_runtime::codec::{Bounded, ListView, ListWalk};
use super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntryView;

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
    fn entries(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3>;
    /// The `priority` field.
    fn priority(&self) -> Option<u8>;
    /// The `checksum` field.
    fn checksum(&self) -> Option<u16>;
}

impl<T: CodecChainHasTaggedSource + ?Sized> CodecChainHasTaggedSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn entries(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3> {
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
/// ([`CodecChainHasTaggedSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainHasTaggedParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `entries` field.
    pub entries: ::sce_forge_runtime::codec::Bounded<&'a [super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntryParts], 3>,
    /// The `priority` field.
    pub priority: Option<u8>,
    /// The `checksum` field.
    pub checksum: Option<u16>,
}

impl<'a> CodecChainHasTaggedSource for CodecChainHasTaggedParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn entries(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3> {
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
    fn entries(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3> {
        ::sce_forge_runtime::codec::BoundedIter::from(&self.entries)
    }
    fn priority(&self) -> Option<u8> {
        self.priority
    }
    fn checksum(&self) -> Option<u16> {
        self.checksum
    }
}

impl<'a> CodecChainHasTaggedSource for CodecChainHasTaggedView<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn entries(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3> {
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
        let _k_entries: usize = src.entries().len();
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


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecChainHasTaggedView` is what a decode of `CodecChainHasTagged` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecChainHasTagged` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecChainHasTagged::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
pub struct CodecChainHasTaggedEntriesWalk;

impl ListWalk<0> for CodecChainHasTaggedEntriesWalk {
    type Entry<'b> = CodecChainHasTaggedEntryView;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 0]) -> Result<Self::Entry<'b>, CodecError> {
        CodecChainHasTaggedEntryView::decode(_c)
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 0]) {  }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecChainHasTaggedView<'a> {
    pub header: u8,
    pub entries: Bounded<ListView<'a, CodecChainHasTaggedEntriesWalk, 0>, 3>,
    pub priority: Option<u8>,
    pub checksum: Option<u16>,
}

#[allow(dead_code)]
impl<'a> CodecChainHasTaggedView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecChainHasTagged::decode`], held as a view. On success the
    /// cursor advances past the consumed bytes.
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out)?;
        Ok(_out)
    }

    /// [`Self::decode`] into a destination the caller owns, so a view that
    /// embeds or dispatches to this one is filled where it will live. On
    /// `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self) -> Result<(), CodecError> {
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
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            let mut _more = false;
            for _ in 0..3u32 {
                if cursor.remaining() == 0 { break; }
                let _entry = CodecChainHasTaggedEntryView::decode(cursor)?;
                _more = _entry.more();
                _count += 1;
                if !_more { break; }
            }
            if _more && cursor.remaining() == 0 {
                return Err(CodecError::NeedMoreBytes);
            }
            if _more {
                return Err(CodecError::TlvChainOverflow);
            }
            _out.entries = Bounded::new(ListView::new(_rest, _count, [])).ok_or(CodecError::TlvChainOverflow)?;
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

    /// Fill `_out`, the builder form [`CodecChainHasTagged`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder(&self, _out: &mut CodecChainHasTagged) -> Result<(), CodecError> {
        _out.header = self.header;
        _out.entries.clear();
        for _e in self.entries.iter() {
            _out.entries.push(CodecChainHasTaggedEntry::default()).map_err(|_| CodecError::TooManyElements)?;
            if let Some(_slot) = _out.entries.last_mut() {
                _e.into_builder(_slot)?;
            }
        }
        _out.priority = self.priority;
        _out.checksum = self.checksum;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecChainHasTagged::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 14;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecChainHasTagged::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> where CodecError: From<S::Error> {
        CodecChainHasTagged::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses a
    /// value it cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }

    pub fn wide(&self) -> bool {
        (self.header & 0x01) != 0
    }

}
