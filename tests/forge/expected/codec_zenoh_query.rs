#![doc = "SCE-MAP: codec_zenoh_query:51 :: _forge_body"]
// SCE-MAP: codec_zenoh_query:51 :: _forge_body

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

use super::codec_zenoh_ext_entry::CodecZenohExtEntry;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
/// The field `parameters` of `CodecZenohQuery`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohQueryParametersField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecZenohQueryParametersField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<256>;
}

/// The field `extensions` of `CodecZenohQuery`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohQueryExtensionsField;

impl ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryExtensionsField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 8> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecZenohQueryIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohQueryStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohQueryParametersField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohQueryParametersField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage> CodecZenohQueryStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohQueryStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohQueryFrom<'f>: CodecZenohQueryStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohQueryParametersField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f> {}

impl<'f, S: CodecZenohQueryStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohQueryParametersField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f>> CodecZenohQueryFrom<'f> for S {}

// `CodecZenohQueryIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohQuery<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohQueryOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohQueryIn<S: CodecZenohQueryStorage> {
    pub header: u8,
    pub consolidation: Option<u8>,
    pub parameters_len: Option<u64>,
    pub parameters: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecZenohQueryParametersField>>::Bytes, 256>>,
    pub extensions: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryExtensionsField>>::List<CodecZenohExtEntryOwned<S>>, 8>>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohQuery<'a> = CodecZenohQueryIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohQueryOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohQueryIn<S>;

impl<S: CodecZenohQueryStorage> Default for CodecZenohQueryIn<S> {
    fn default() -> Self {
        Self {
            header: 0x03u8,
            consolidation: Default::default(),
            parameters_len: Default::default(),
            parameters: Default::default(),
            extensions: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohQueryStorage> CodecZenohQueryIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohQueryFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohQueryFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohQuery::store_into(&mut _frame, &mut CodecZenohQuery::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohQueryFrom<'f> {
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
        let consolidation = if (header & 0x20u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let parameters_len = if (header & 0x40u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let parameters = if (header & 0x40u8) != 0 {
            let _len = parameters_len.unwrap();
            if _len > 256 {
                return Err(CodecError::LengthOutOfRange);
            }
            let _n = _len as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        let parameters = parameters.map(|_v| ::sce_forge_runtime::codec::Bounded::new(_v).ok_or(CodecError::LengthOutOfRange)).transpose()?;
        let parameters = parameters.map(|_v| _v.store::<S, CodecZenohQueryParametersField>()).transpose()?;
        if (header & 0x80u8) != 0 {
            _out.extensions.insert(Default::default()).refill(|_vec| {
                for _ in 0..8u32 {
                if cursor.remaining() == 0 { break; }
                // The loop walks the chain's depth and `_vec` holds the capacity, so
                // this push fails only where the deployment holds fewer entries than
                // the frame says. An over-long chain is refused after the loop.
                ::sce_forge_runtime::codec::SceList::try_push(_vec, CodecZenohExtEntryOwned::<S>::decode(cursor)?)?;
            }
                if cursor.remaining() > 0 {
                    return Err(CodecError::TooManyElements);
                }
                Ok(())
            })?;
        } else {
            _out.extensions = None;
        }
        _out.header = header;
        _out.consolidation = consolidation;
        _out.parameters_len = parameters_len;
        _out.parameters = parameters;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn set_mid(&mut self, v: u8) {
        self.header = (self.header & !0x1F) | (v & 0x1F);
    }

    pub fn c(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_c(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn p(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_p(&mut self, v: bool) {
        if v {
            self.header |= 0x40;
        } else {
            self.header &= !0x40;
        }
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
    pub const MAX_ENCODED_BYTES: usize = 603;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohQuery::encode_source(self, w)
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
/// What a value of `CodecZenohQuery` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohQuery`
/// computes those from what is ([`CodecZenohQuery::encode_source`]).
pub trait CodecZenohQuerySource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `consolidation` field.
    fn consolidation(&self) -> Option<u8>;
    /// The `parameters` field.
    fn parameters(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 256>>;
    /// The `extensions` field.
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 8>>;
}

impl<T: CodecZenohQuerySource + ?Sized> CodecZenohQuerySource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn consolidation(&self) -> Option<u8> {
        (**self).consolidation()
    }
    fn parameters(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 256>> {
        (**self).parameters()
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 8>> {
        (**self).extensions()
    }
}

/// A value of `CodecZenohQuery` held in the types a caller already has: a source
/// ([`CodecZenohQuerySource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohQueryParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `consolidation` field.
    pub consolidation: Option<u8>,
    /// The `parameters` field.
    pub parameters: Option<::sce_forge_runtime::codec::Bytes<'a, 256>>,
    /// The `extensions` field.
    pub extensions: Option<::sce_forge_runtime::codec::Bounded<&'a [super::codec_zenoh_ext_entry::CodecZenohExtEntryParts<'a>], 8>>,
}

impl<'a> CodecZenohQuerySource for CodecZenohQueryParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn consolidation(&self) -> Option<u8> {
        self.consolidation
    }
    fn parameters(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 256>> {
        self.parameters
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 8>> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
}

impl<S: CodecZenohQueryStorage> CodecZenohQuerySource for CodecZenohQueryOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn consolidation(&self) -> Option<u8> {
        self.consolidation
    }
    fn parameters(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 256>> {
        self.parameters.as_ref().map(::sce_forge_runtime::codec::Bounded::as_bytes)
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 8>> {
        self.extensions.as_ref().map(|_l| _l.list_iter())
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohQuery<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohQuerySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohQuerySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohQuerySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _k_extensions: usize = src.extensions().map_or(0, |_l| _l.len());
        let _p_header_c_0: bool = src.consolidation().is_some();
        let _p_header_p_0: bool = src.parameters().is_some();
        let _p_header_z_0: bool = src.extensions().is_some();
        let _n_d_parameters_len_0: usize = match src.parameters() {
            Some(_v) => _v.len(),
            None => 0,
        };
        let _f_d_parameters_len: Option<usize> = if src.parameters().is_some() { Some(_n_d_parameters_len_0) } else { None };
        let _d_parameters_len: u64 = ::sce_forge_runtime::codec::widen_usize(_f_d_parameters_len.unwrap_or(0));
        let _c_header: u8 = (((((src.header() & !0x20) | (u8::from(_p_header_c_0) << 5)) & !0x40) | (u8::from(_p_header_p_0) << 6)) & !0x80) | (u8::from(_p_header_z_0) << 7);
        w.write_u8(_c_header)?;
        if let Some(_v) = src.consolidation() {
            w.write_u8(_v)?;
        }
        if (_c_header & 0x40) != 0 {
            w.write_vle_u64(_d_parameters_len)?;
        }
        if let Some(_v) = src.parameters() {
            w.write_bytes(_v.into_inner())?;
        }
        if let Some(_v) = src.extensions() {
            for _e in _v {
                CodecZenohExtEntry::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohQuerySource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<'a, S: CodecZenohQuerySource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohQuery<'a> {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohQuery<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohQuerySource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohQuerySource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohQuery<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohQueryOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohQueryOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohQueryOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohQueryOwned` alone is the build's
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
// `let v: CodecZenohQueryOwned = CodecZenohQueryOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a> CodecZenohQuery<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohQueryOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity. Every value of this type is within
    /// each declared bound, since its lists, bytes and text hold their bounds
    /// in their types (docs/adr/0077 §3), and the inline and growable profiles
    /// hold each field at its bound, so the projection does not fail on them.
    /// A profile whose capacity is below a bound raises
    /// `CodecError::CapacityExceeded` for a value it cannot hold.
    pub fn try_into_owned_in<S: CodecZenohQueryFrom<'a>>(self) -> Result<CodecZenohQueryOwned<S>, CodecError> {
        Ok(CodecZenohQueryOwned {
            header: self.header,
            consolidation: self.consolidation,
            parameters_len: self.parameters_len,
            parameters: self.parameters.map(|_v| _v.store::<S, CodecZenohQueryParametersField>()).transpose()?,
            extensions: self.extensions.map(|_v| ::sce_forge_runtime::codec::try_collect_bounded(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohQueryOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohQueryStorage> CodecZenohQueryOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohQuery<'_>, CodecError> {
        let mut _out = CodecZenohQuery::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohQuery<'o>) -> Result<(), CodecError> {
        _out.header = self.header;
        _out.consolidation = self.consolidation;
        _out.parameters_len = self.parameters_len;
        _out.parameters = self.parameters.as_ref().map(::sce_forge_runtime::codec::Bounded::as_bytes);
        match &self.extensions {
            Some(_l) => _out.extensions.insert(Default::default()).refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, _l.inner(), |_e| Ok(_e.as_borrowed())))?,
            None => _out.extensions = None,
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
    pub fn transcode_in<D: for<'x> CodecZenohQueryFrom<'x>>(&self) -> Result<CodecZenohQueryOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
