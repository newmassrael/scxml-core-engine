#![doc = "SCE-MAP: codec_chain_has_envelope:23 :: _forge_body"]
// SCE-MAP: codec_chain_has_envelope:23 :: _forge_body

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

use super::codec_zenoh_ext_entry::CodecZenohExtEntry;
use super::codec_chain_has_slice::CodecChainHasSlice;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecChainHasEnvelope<'a> {
    pub header: u8,
    pub extensions: Option<HeaplessVec<CodecZenohExtEntry<'a>, 4>>,
    pub payload_len: Option<u64>,
    pub payload: Option<&'a [u8]>,
    pub slice_count: Option<u32>,
    pub slices: Option<HeaplessVec<CodecChainHasSlice<'a>, 4>>,
}

#[allow(dead_code)]
impl<'a> CodecChainHasEnvelope<'a> {
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
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
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
        if (header & 0x80u8) != 0 {
            let _vec = _out.extensions.insert(HeaplessVec::<CodecZenohExtEntry<'a>, 4>::new());
            let mut _more = false;
            for _ in 0..4u32 {
                if cursor.remaining() == 0 { break; }
                let _entry = CodecZenohExtEntry::decode(cursor)?;
                _more = _entry.z();
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
            _out.extensions = None;
        }
        let _has_extensions_2 = _out.extensions.as_ref().is_some_and(|_c| _c.iter().any(|_e| u64::from(_e.ext_id()) == 2u64));
        let payload_len = if !_has_extensions_2 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let payload = if !_has_extensions_2 {
            let _n = payload_len.unwrap() as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        let slice_count = if _has_extensions_2 {
            let _v = cursor.read_vle_u32()?;
            Some(_v)
        } else {
            None
        };
        if _has_extensions_2 {
            let _n = slice_count.expect("co-gating: count present-if matches repeat");
            let _vec = _out.slices.insert(HeaplessVec::<CodecChainHasSlice<'a>, 4>::new());
            for _ in 0.._n {
                _vec.push(CodecChainHasSlice::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
        } else {
            _out.slices = None;
        }
        _out.header = header;
        _out.payload_len = payload_len;
        _out.payload = payload;
        _out.slice_count = slice_count;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn kind(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn set_kind(&mut self, v: u8) {
        self.header = (self.header & !0x1F) | (v & 0x1F);
    }

    pub fn e(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_e(&mut self, v: bool) {
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
    pub const MAX_ENCODED_BYTES: usize = 1511;

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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainHasEnvelope` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainHasEnvelope`
/// computes those from what is ([`CodecChainHasEnvelope::encode_source`]).
pub trait CodecChainHasEnvelopeSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `extensions` field.
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone>;
    /// The `payload` field.
    fn payload(&self) -> Option<&[u8]>;
    /// The `slices` field.
    fn slices(&self) -> Option<impl Iterator<Item = impl super::codec_chain_has_slice::CodecChainHasSliceSource> + Clone>;
}

impl<T: CodecChainHasEnvelopeSource + ?Sized> CodecChainHasEnvelopeSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        (**self).extensions()
    }
    fn payload(&self) -> Option<&[u8]> {
        (**self).payload()
    }
    fn slices(&self) -> Option<impl Iterator<Item = impl super::codec_chain_has_slice::CodecChainHasSliceSource> + Clone> {
        (**self).slices()
    }
}

/// A value of `CodecChainHasEnvelope` held in the types a caller already has: a source
/// ([`CodecChainHasEnvelopeSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainHasEnvelopeParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `extensions` field.
    pub extensions: Option<&'a [super::codec_zenoh_ext_entry::CodecZenohExtEntryParts<'a>]>,
    /// The `payload` field.
    pub payload: Option<&'a [u8]>,
    /// The `slices` field.
    pub slices: Option<&'a [super::codec_chain_has_slice::CodecChainHasSliceParts<'a>]>,
}

impl<'a> CodecChainHasEnvelopeSource for CodecChainHasEnvelopeParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.map(<[_]>::iter)
    }
    fn payload(&self) -> Option<&[u8]> {
        self.payload
    }
    fn slices(&self) -> Option<impl Iterator<Item = impl super::codec_chain_has_slice::CodecChainHasSliceSource> + Clone> {
        self.slices.map(<[_]>::iter)
    }
}

impl<'a> CodecChainHasEnvelopeSource for CodecChainHasEnvelope<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
    fn payload(&self) -> Option<&[u8]> {
        self.payload
    }
    fn slices(&self) -> Option<impl Iterator<Item = impl super::codec_chain_has_slice::CodecChainHasSliceSource> + Clone> {
        self.slices.as_ref().map(|_l| _l.iter())
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecChainHasEnvelopeSource for CodecChainHasEnvelopeOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.as_ref().map(|_l| ::sce_forge_runtime::codec::SceList::as_slice(_l).iter())
    }
    fn payload(&self) -> Option<&[u8]> {
        self.payload.as_ref().map(::sce_forge_runtime::codec::SceByteBuf::as_slice)
    }
    fn slices(&self) -> Option<impl Iterator<Item = impl super::codec_chain_has_slice::CodecChainHasSliceSource> + Clone> {
        self.slices.as_ref().map(|_l| ::sce_forge_runtime::codec::SceList::as_slice(_l).iter())
    }
}

#[allow(dead_code)]
impl<'a> CodecChainHasEnvelope<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecChainHasEnvelopeSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecChainHasEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainHasEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let _k_extensions: usize = src.extensions().map_or(0, |_l| _l.count());
        if _k_extensions > 4 {
            return Err(CodecError::TlvChainOverflow);
        }
        let _k_slices: usize = src.slices().map_or(0, |_l| _l.count());
        if _k_slices > 4 {
            return Err(CodecError::TooManyElements);
        }
        let _p_header_e_0: bool = src.extensions().is_some();
        let _n_d_payload_len_0: usize = match src.payload() {
            Some(_v) => _v.len(),
            None => 0,
        };
        let _f_d_payload_len: Option<usize> = if src.payload().is_some() { Some(_n_d_payload_len_0) } else { None };
        let _d_payload_len: u64 = ::sce_forge_runtime::codec::widen_usize(_f_d_payload_len.unwrap_or(0));
        let _f_d_slice_count: Option<usize> = if src.slices().is_some() { Some(_k_slices) } else { None };
        let _d_slice_count: u32 = _f_d_slice_count.unwrap_or(0) as u32;
        let _c_header: u8 = (src.header() & !0x80) | (u8::from(_p_header_e_0) << 7);
        if (!src.extensions().is_some_and(|mut _l| _l.any(|_e| (u64::from(super::codec_zenoh_ext_entry::CodecZenohExtEntrySource::header(&_e)) & 0xF) == 2u64))) != src.payload().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        if (src.extensions().is_some_and(|mut _l| _l.any(|_e| (u64::from(super::codec_zenoh_ext_entry::CodecZenohExtEntrySource::header(&_e)) & 0xF) == 2u64))) != src.slices().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        w.write_u8(_c_header)?;
        if let Some(_v) = src.extensions() {
            for (_i, _e) in _v.enumerate() {
                CodecZenohExtEntry::write_source_in(&_e, w, &super::codec_zenoh_ext_entry::CodecZenohExtEntryOverrides::header_z(_i + 1 < _k_extensions))?;
            }
        }
        if !src.extensions().is_some_and(|mut _l| _l.any(|_e| (u64::from(super::codec_zenoh_ext_entry::CodecZenohExtEntrySource::header(&_e)) & 0xF) == 2u64)) {
            w.write_vle_u64(_d_payload_len)?;
        }
        if let Some(_v) = src.payload() {
            w.write_bytes(_v)?;
        }
        if src.extensions().is_some_and(|mut _l| _l.any(|_e| (u64::from(super::codec_zenoh_ext_entry::CodecZenohExtEntrySource::header(&_e)) & 0xF) == 2u64)) {
            w.write_vle_u32(_d_slice_count)?;
        }
        if let Some(_v) = src.slices() {
            for _e in _v {
                CodecChainHasSlice::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecChainHasEnvelopeSource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<'a, S: CodecChainHasEnvelopeSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecChainHasEnvelope<'a> {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecChainHasEnvelope<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecChainHasEnvelopeSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecChainHasEnvelopeSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecChainHasEnvelope<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecChainHasEnvelopeOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecChainHasEnvelopeOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecChainHasEnvelopeOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecChainHasEnvelopeOwned` alone is the build's
// default profile. Because the non-allocating profile is a *type*, this mirror
// carries no `alloc` gate at all — a list- or embed-bearing codec has an owned
// form on the heap-free tier too, and a heap-capable consumer can still pin a
// value to storage that is guaranteed not to allocate.
//
// `try_into_owned` is the fallible direction (one `?` per profile: on the
// growable profile the copy cannot fail, on the inline profile it enforces
// each declared bound); `try_as_borrowed` re-borrows any profile back
// into the single borrowed view that owns `encode`; `transcode_in` moves a
// value between profiles as a checked projection rather than a re-decode.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling one instead names it on the value or its binding —
// `let v: CodecChainHasEnvelopeOwned = CodecChainHasEnvelopeOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
use super::codec_chain_has_slice::CodecChainHasSliceOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecChainHasEnvelopeOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub header: u8,
    pub extensions: Option<S::List<CodecZenohExtEntryOwned<S>, 16>>,
    pub payload_len: Option<u64>,
    pub payload: Option<S::Bytes<256>>,
    pub slice_count: Option<u32>,
    pub slices: Option<S::List<CodecChainHasSliceOwned<S>, 4>>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecChainHasEnvelopeOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn kind(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn e(&self) -> bool {
        (self.header & 0x80) != 0
    }
}

#[allow(dead_code)]
impl<'a> CodecChainHasEnvelope<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecChainHasEnvelopeOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecChainHasEnvelopeOwned<S>, CodecError> {
        Ok(CodecChainHasEnvelopeOwned {
            header: self.header,
            extensions: self.extensions.map(|_v| ::sce_forge_runtime::codec::try_collect_list(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
            payload_len: self.payload_len,
            payload: self.payload.map(<S::Bytes<256> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice).transpose()?,
            slice_count: self.slice_count,
            slices: self.slices.map(|_v| ::sce_forge_runtime::codec::try_collect_list(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecChainHasEnvelopeOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecChainHasEnvelopeOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecChainHasEnvelope<'_>, CodecError> {
        let mut _out = CodecChainHasEnvelope::default();
        self.try_as_borrowed_into(&mut _out)?;
        Ok(_out)
    }

    /// [`Self::try_as_borrowed`] into a destination the caller owns, each
    /// bounded list and embedded body projected in the field where it sits.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through (the optional wrapper, the borrowed struct, the
    /// variant, the caller's binding), however few entries the list holds. A
    /// caller on a path where that shows projects into a destination it owns,
    /// then encodes from it. On `Err`, `_out` holds what was projected so far
    /// and is not to be read.
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecChainHasEnvelope<'o>) -> Result<(), CodecError> {
        _out.header = self.header;
        match &self.extensions {
            Some(_l) => ::sce_forge_runtime::codec::try_project_bounded_into(_out.extensions.insert(Default::default()), _l, |_e| Ok(_e.as_borrowed()))?,
            None => _out.extensions = None,
        }
        _out.payload_len = self.payload_len;
        _out.payload = self.payload.as_ref().map(::sce_forge_runtime::codec::SceByteBuf::as_slice);
        _out.slice_count = self.slice_count;
        match &self.slices {
            Some(_l) => ::sce_forge_runtime::codec::try_project_bounded_into(_out.slices.insert(Default::default()), _l, |_e| Ok(_e.as_borrowed()))?,
            None => _out.slices = None,
        }
        Ok(())
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecChainHasEnvelopeOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecChainHasEnvelopeOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `try_as_borrowed` followed by
    /// [`CodecChainHasEnvelope::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here.
    /// The same refusals as the projection's: a list longer than the borrowed
    /// view declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        CodecChainHasEnvelope::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecChainHasEnvelope::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
