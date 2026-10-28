#![doc = "SCE-MAP: codec_chain_prev_gated:17 :: _forge_body"]
// SCE-MAP: codec_chain_prev_gated:17 :: _forge_body

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

use super::codec_chain_prev_entry::CodecChainPrevEntry;

use sce_forge_runtime::codec::{ListView, ListWalk};
use super::codec_chain_prev_entry::CodecChainPrevEntryView;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecChainPrevGated {
    pub head: u8,
    pub entries: Option<HeaplessVec<CodecChainPrevEntry, 4>>,
}

#[allow(dead_code)]
impl CodecChainPrevGated {
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
        let head = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        if (head & 0x02u8) != 0 {
            let _vec = _out.entries.insert(HeaplessVec::<CodecChainPrevEntry, 4>::new());
            let mut _prev_entries_after_marker: u8 = 0;
            let mut _more = false;
            for _ in 0..4u32 {
                if cursor.remaining() == 0 { break; }
                let _entry = CodecChainPrevEntry::decode(cursor, _prev_entries_after_marker, head & 0x1)?;
                _more = _entry.z();
                _prev_entries_after_marker = u8::from((u64::from(_entry.header) & 127u64) == 4u64);
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
        } else {
            _out.entries = None;
        }
        _out.head = head;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn t(&self) -> bool {
        (self.head & 0x01) != 0
    }

    pub fn set_t(&mut self, v: bool) {
        if v {
            self.head |= 0x01;
        } else {
            self.head &= !0x01;
        }
    }

    pub fn e(&self) -> bool {
        (self.head & 0x02) != 0
    }

    pub fn set_e(&mut self, v: bool) {
        if v {
            self.head |= 0x02;
        } else {
            self.head &= !0x02;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 21;

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

/// Flags of `CodecChainPrevGated` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecChainPrevGated`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecChainPrevGatedOverrides {
    /// The `T` flag of `head`, when a parent sets it.
    pub head_t: Option<bool>,
}

#[allow(dead_code)]
impl CodecChainPrevGatedOverrides {
    /// Sets `T` of `head` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn head_t(value: bool) -> Self {
        Self { head_t: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainPrevGated` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainPrevGated`
/// computes those from what is ([`CodecChainPrevGated::encode_source`]).
pub trait CodecChainPrevGatedSource {
    /// The `head` field.
    fn head(&self) -> u8;
    /// The `entries` field.
    fn entries(&self) -> Option<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone>;
}

impl<T: CodecChainPrevGatedSource + ?Sized> CodecChainPrevGatedSource for &T {
    fn head(&self) -> u8 {
        (**self).head()
    }
    fn entries(&self) -> Option<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone> {
        (**self).entries()
    }
}

/// A value of `CodecChainPrevGated` held in the types a caller already has: a source
/// ([`CodecChainPrevGatedSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainPrevGatedParts<'a> {
    /// The `head` field.
    pub head: u8,
    /// The `entries` field.
    pub entries: Option<&'a [super::codec_chain_prev_entry::CodecChainPrevEntryParts]>,
}

impl<'a> CodecChainPrevGatedSource for CodecChainPrevGatedParts<'a> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> Option<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone> {
        self.entries.map(<[_]>::iter)
    }
}

impl CodecChainPrevGatedSource for CodecChainPrevGated {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> Option<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone> {
        self.entries.as_ref().map(|_l| _l.iter())
    }
}

impl<'a> CodecChainPrevGatedSource for CodecChainPrevGatedView<'a> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> Option<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone> {
        self.entries.as_ref().map(|_l| _l.iter())
    }
}

#[allow(dead_code)]
impl CodecChainPrevGated {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecChainPrevGatedSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecChainPrevGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new())?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainPrevGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source_in(src, w, &CodecChainPrevGatedOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecChainPrevGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecChainPrevGatedOverrides) -> Result<(), CodecError> {
        let _k_entries: usize = src.entries().map_or(0, |_l| _l.count());
        if _k_entries > 4 {
            return Err(CodecError::TlvChainOverflow);
        }
        let _p_head_e_0: bool = src.entries().is_some();
        let mut _c_head: u8 = (src.head() & !0x2) | (u8::from(_p_head_e_0) << 1);
        if let Some(_o) = over.head_t {
            _c_head = (_c_head & !0x1) | u8::from(_o);
        }
        w.write_u8(_c_head)?;
        if let Some(_v) = src.entries() {
            let mut _prev_after_marker: u8 = 0;
            for (_i, _e) in _v.enumerate() {
                CodecChainPrevEntry::write_source_in(&_e, w, _prev_after_marker, _c_head & 0x1, &super::codec_chain_prev_entry::CodecChainPrevEntryOverrides::header_z(_i + 1 < _k_entries))?;
                _prev_after_marker = u8::from((u64::from(super::codec_chain_prev_entry::CodecChainPrevEntrySource::header(&_e)) & 0x7F) == 4u64);
            }
        }
        Ok(())
    }
}

impl<S: CodecChainPrevGatedSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecChainPrevGated {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecChainPrevGated {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecChainPrevGatedSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecChainPrevGatedSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        ::sce_forge_runtime::codec::write_window::<Self, S>(src, window)
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecChainPrevGatedView` is what a decode of `CodecChainPrevGated` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecChainPrevGated` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecChainPrevGated::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
pub struct CodecChainPrevGatedEntriesWalk;

impl ListWalk<2> for CodecChainPrevGatedEntriesWalk {
    type Entry<'b> = CodecChainPrevEntryView;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 2]) -> Result<Self::Entry<'b>, CodecError> {
        CodecChainPrevEntryView::decode(_c, _a[0], _a[1])
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 2]) { _s[0] = u8::from((u64::from(_e.header) & 127u64) == 4u64); }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecChainPrevGatedView<'a> {
    pub head: u8,
    pub entries: Option<ListView<'a, CodecChainPrevGatedEntriesWalk, 2>>,
}

#[allow(dead_code)]
impl<'a> CodecChainPrevGatedView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecChainPrevGated::decode`], held as a view. On success the
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
        let head = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        if (head & 0x02u8) != 0 {
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            let mut _prev_entries_after_marker: u8 = 0;
            let mut _more = false;
            for _ in 0..4u32 {
                if cursor.remaining() == 0 { break; }
                let _entry = CodecChainPrevEntryView::decode(cursor, _prev_entries_after_marker, head & 0x1)?;
                _more = _entry.z();
                _prev_entries_after_marker = u8::from((u64::from(_entry.header) & 127u64) == 4u64);
                _count += 1;
                if !_more { break; }
            }
            if _more && cursor.remaining() == 0 {
                return Err(CodecError::NeedMoreBytes);
            }
            if _more {
                return Err(CodecError::TlvChainOverflow);
            }
            _out.entries = Some(ListView::new(_rest, _count, [0, head & 0x1]));
        } else {
            _out.entries = None;
        }
        _out.head = head;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecChainPrevGated`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    pub fn into_builder(&self, _out: &mut CodecChainPrevGated) -> Result<(), CodecError> {
        _out.head = self.head;
        match &self.entries {
            Some(_l) => {
                let _vec = _out.entries.insert(HeaplessVec::new());
                _vec.clear();
                for _e in _l.iter() {
                    _vec.push(CodecChainPrevEntry::default()).map_err(|_| CodecError::TooManyElements)?;
                    if let Some(_slot) = _vec.last_mut() {
                        _e.into_builder(_slot)?;
                    }
                }
            }
            None => _out.entries = None,
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecChainPrevGated::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 21;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecChainPrevGated::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        CodecChainPrevGated::encode_source(self, w)
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

    pub fn t(&self) -> bool {
        (self.head & 0x01) != 0
    }

    pub fn e(&self) -> bool {
        (self.head & 0x02) != 0
    }

}
