#![doc = "SCE-MAP: codec_chain_has_marker:28 :: _forge_body"]
// SCE-MAP: codec_chain_has_marker:28 :: _forge_body

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
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
use super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceOwned;
/// The field `extensions` of `CodecChainHasMarker`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecChainHasMarkerExtensionsField;

impl ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerExtensionsField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 4> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// The field `payload` of `CodecChainHasMarker`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecChainHasMarkerPayloadField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecChainHasMarkerPayloadField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<256>;
}

/// The field `slices` of `CodecChainHasMarker`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecChainHasMarkerSlicesField;

impl ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerSlicesField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 4> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecChainHasMarkerIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecChainHasMarkerStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerExtensionsField> + ::sce_forge_runtime::codec::HoldsBytes<CodecChainHasMarkerPayloadField> + ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerSlicesField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage + super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerExtensionsField> + ::sce_forge_runtime::codec::HoldsBytes<CodecChainHasMarkerPayloadField> + ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerSlicesField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage + super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceStorage> CodecChainHasMarkerStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecChainHasMarkerStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecChainHasMarkerFrom<'f>: CodecChainHasMarkerStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecChainHasMarkerPayloadField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f> + super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceFrom<'f> {}

impl<'f, S: CodecChainHasMarkerStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecChainHasMarkerPayloadField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f> + super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceFrom<'f>> CodecChainHasMarkerFrom<'f> for S {}

/// The group `CodecChainHasMarkerExtensionsHas0x12` as `CodecChainHasMarkerOwned` holds it, in the storage
/// profile `S`.
#[allow(dead_code)]
pub type CodecChainHasMarkerExtensionsHas0x12Owned<S> = CodecChainHasMarkerExtensionsHas0x12<<S as ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerExtensionsField>>::List<CodecZenohExtEntryOwned<S>>, ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerSlicesField>>::List<CodecChainHasMarkerSliceOwned<S>>, 4>, ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecChainHasMarkerPayloadField>>::Bytes, 256>>;

// `CodecChainHasMarkerIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecChainHasMarker<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecChainHasMarkerOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecChainHasMarkerIn<S: CodecChainHasMarkerStorage> {
    pub header: u8,
    pub extensions_has_0x12: CodecChainHasMarkerExtensionsHas0x12Owned<S>,
    pub payload_len: Option<u64>,
    pub slice_count: Option<u32>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecChainHasMarker<'a> = CodecChainHasMarkerIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecChainHasMarkerOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecChainHasMarkerIn<S>;

impl<S: CodecChainHasMarkerStorage> Default for CodecChainHasMarkerIn<S> {
    fn default() -> Self {
        Self {
            header: Default::default(),
            extensions_has_0x12: Default::default(),
            payload_len: Default::default(),
            slice_count: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecChainHasMarkerStorage> CodecChainHasMarkerIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecChainHasMarkerFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecChainHasMarkerFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecChainHasMarker::store_into(&mut _frame, &mut CodecChainHasMarker::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecChainHasMarkerFrom<'f> {
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
        let mut extensions: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerExtensionsField>>::List<CodecZenohExtEntryOwned<S>>, 4, 1>> = None;
        if (header & 0x80u8) != 0 {
            extensions.insert(Default::default()).refill(|_vec| {
                let mut _more = false;
                for _ in 0..4u32 {
                    let _entry = CodecZenohExtEntryOwned::<S>::decode(cursor)?;
                    _more = _entry.z();
                    // The loop walks the chain's depth and `_vec` holds the capacity, so
                // this push fails only where the deployment holds fewer entries than
                // the frame says. An over-long chain is refused after the loop.
                ::sce_forge_runtime::codec::SceList::try_push(_vec, _entry)?;
                    if !_more { break; }
                }
                if _more && cursor.remaining() == 0 {
                    return Err(CodecError::NeedMoreBytes);
                }
                if _more {
                    return Err(CodecError::TooManyElements);
                }
                Ok(())
            })?;
        } else {
            extensions = None;
        }
        let _has_extensions_18 = extensions.as_ref().is_some_and(|_c| _c.list_iter::<CodecZenohExtEntryOwned<S>>().any(|_e| (u64::from(_e.header) & 127u64) == 18u64));
        let payload_len = if !_has_extensions_18 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let payload = if !_has_extensions_18 {
            let _len = payload_len.unwrap();
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
        let payload = payload.map(|_v| ::sce_forge_runtime::codec::Bounded::new(_v).ok_or(CodecError::LengthOutOfRange)).transpose()?;
        let payload = payload.map(|_v| _v.store::<S, CodecChainHasMarkerPayloadField>()).transpose()?;
        let slice_count = if _has_extensions_18 {
            let _v = cursor.read_vle_u32()?;
            Some(_v)
        } else {
            None
        };
        let mut slices: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerSlicesField>>::List<CodecChainHasMarkerSliceOwned<S>>, 4>> = None;
        if _has_extensions_18 {
            let _n = slice_count.expect("co-gating: count present-if matches repeat");
            if _n > 4 {
                return Err(CodecError::TooManyElements);
            }
            slices.insert(Default::default()).refill(|_vec| {
                for _ in 0.._n {
                    ::sce_forge_runtime::codec::SceList::try_push(_vec, CodecChainHasMarkerSliceOwned::<S>::decode(cursor)?)?;
                }
                Ok(())
            })?;
        } else {
            slices = None;
        }
        _out.header = header;
        _out.extensions_has_0x12 = if _has_extensions_18 { CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions: match ::sce_forge_runtime::codec::place_first::<_, CodecChainHasMarkerExtensionsHas0x12Key, 4>(extensions.expect("the gate found the chain").into_inner()) { Ok(_p) => _p, Err(_) => unreachable!("the gate found an entry of the kind") }, slices: slices.unwrap_or_default() } } else { CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions: extensions.map(|_l| ::sce_forge_runtime::codec::Chain::new(_l.into_inner()).expect("the gates found the chain's kinds as the arm says")), payload: payload.unwrap_or_default() } };
        _out.payload_len = payload_len;
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

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecChainHasMarkerWire::encode_source(self, w)
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

/// The fields `extensions.has(0x12)` decide (docs/adr/0065 §6 R4b, R4c): a value holds the
/// ones of one arm, and `CodecChainHasMarker` derives the gates from which.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodecChainHasMarkerExtensionsHas0x12<Extensions, Slices, Payload> {
    /// While `extensions.has(0x12)` holds.
    Has0x12 {
        /// The `extensions` field.
        extensions: ::sce_forge_runtime::codec::Placed<Extensions, CodecChainHasMarkerExtensionsHas0x12Key, 4>,
        /// The `slices` field.
        slices: Slices,
    },
    /// While `extensions.has(0x12)` does not hold.
    NotHas0x12 {
        /// The `extensions` field.
        extensions: Option<::sce_forge_runtime::codec::Chain<Extensions, (), (CodecChainHasMarkerExtensionsHas0x12Key,), 4, 1>>,
        /// The `payload` field.
        payload: Payload,
    },
}

impl<Extensions: Default, Slices: Default, Payload: Default> Default for CodecChainHasMarkerExtensionsHas0x12<Extensions, Slices, Payload> {
    /// The arm held while `extensions.has(0x12)` does not hold, each field at its type's default.
    fn default() -> Self {
        Self::NotHas0x12 { extensions: Default::default(), payload: Default::default(), }
    }
}

/// The kind of entry `extensions.has(0x12)` names: an entry whose identifier is
/// 0x12, which is all of its value (docs/adr/0065 §6 R6a).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecChainHasMarkerExtensionsHas0x12Key;

impl<E: super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> ::sce_forge_runtime::codec::Key<E> for CodecChainHasMarkerExtensionsHas0x12Key {
    fn matches(entry: &E) -> bool {
        (u64::from(super::codec_zenoh_ext_entry::CodecZenohExtEntrySource::header(entry)) & 0x7F) == 18u64
    }
}

#[allow(dead_code)]
impl CodecChainHasMarkerExtensionsHas0x12Key {
    /// The entry of the kind: its identifier, and nothing else to say.
    pub fn named() -> super::codec_zenoh_ext_entry::CodecZenohExtEntry<'static> {
        super::codec_zenoh_ext_entry::CodecZenohExtEntry { header: 0x12, body: super::codec_zenoh_ext_entry::CodecZenohExtEntryVariant::CodecZenohExtUnit(Default::default()) }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainHasMarker` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainHasMarker`
/// computes those from what is ([`CodecChainHasMarker::encode_source`]).
pub trait CodecChainHasMarkerSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The fields `extensions.has(0x12)` decides, held together (docs/adr/0065 §6 R4).
    fn extensions_has_0x12(&self) -> CodecChainHasMarkerExtensionsHas0x12<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 4>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 256>>;
}

impl<T: CodecChainHasMarkerSource + ?Sized> CodecChainHasMarkerSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn extensions_has_0x12(&self) -> CodecChainHasMarkerExtensionsHas0x12<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 4>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 256>> {
        (**self).extensions_has_0x12()
    }
}

/// A value of `CodecChainHasMarker` held in the types a caller already has: a source
/// ([`CodecChainHasMarkerSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainHasMarkerParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The fields `extensions.has(0x12)` decides, held together (docs/adr/0065 §6 R4).
    pub extensions_has_0x12: CodecChainHasMarkerExtensionsHas0x12<&'a [super::codec_zenoh_ext_entry::CodecZenohExtEntryParts<'a>], ::sce_forge_runtime::codec::Bounded<&'a [super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceParts<'a>], 4>, ::sce_forge_runtime::codec::Bytes<'a, 256>>,
}

impl<'a> CodecChainHasMarkerSource for CodecChainHasMarkerParts<'a> {
    fn extensions_has_0x12(&self) -> CodecChainHasMarkerExtensionsHas0x12<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 4>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 256>> {
        match &self.extensions_has_0x12 {
            CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions, slices } => CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions: extensions.iter(), slices: slices.iter() },
            CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions, payload } => CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions: extensions.as_ref().map(|_e| _e.iter()), payload: *payload },
        }
    }
    fn header(&self) -> u8 {
        self.header
    }
}

impl<S: CodecChainHasMarkerStorage> CodecChainHasMarkerSource for CodecChainHasMarkerOwned<S> {
    fn extensions_has_0x12(&self) -> CodecChainHasMarkerExtensionsHas0x12<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, 4>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 256>> {
        match &self.extensions_has_0x12 {
            CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions, slices } => CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions: extensions.list_iter(), slices: slices.list_iter() },
            CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions, payload } => CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions: extensions.as_ref().map(|_e| _e.list_iter()), payload: payload.as_bytes() },
        }
    }
    fn header(&self) -> u8 {
        self.header
    }
}

/// The codec `CodecChainHasMarker` itself: what writes a value read through
/// [`CodecChainHasMarkerSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecChainHasMarkerWire;

#[allow(dead_code)]
impl CodecChainHasMarkerWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecChainHasMarker::encode` writes for the value it holds, read through
    /// [`CodecChainHasMarkerSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecChainHasMarkerSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainHasMarkerSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let (_m_extensions, _m_slices, _m_payload) = match src.extensions_has_0x12() {
            CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions, slices } => (Some(extensions.slots()), Some(slices), None),
            CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions, payload } => (extensions.map(|_e| _e.slots()), None, Some(payload)),
        };
        let _k_extensions: usize = _m_extensions.clone().map_or(0, |_l| _l.len());
        let _k_slices: usize = _m_slices.clone().map_or(0, |_l| _l.len());
        let _p_header_e_0: bool = _m_extensions.clone().is_some();
        let _n_d_payload_len_0: usize = match _m_payload {
            Some(_v) => _v.len(),
            None => 0,
        };
        let _f_d_payload_len: Option<usize> = if _m_payload.is_some() { Some(_n_d_payload_len_0) } else { None };
        let _d_payload_len: u64 = ::sce_forge_runtime::codec::widen_usize(_f_d_payload_len.unwrap_or(0));
        let _f_d_slice_count: Option<usize> = if _m_slices.clone().is_some() { Some(_k_slices) } else { None };
        let _d_slice_count: u32 = _f_d_slice_count.unwrap_or(0) as u32;
        let _c_header: u8 = (src.header() & !0x80) | (u8::from(_p_header_e_0) << 7);
        w.write_u8(_c_header)?;
        if let Some(_v) = _m_extensions.clone() {
            for (_i, _s) in _v.enumerate() {
                match _s {
                    ::sce_forge_runtime::codec::Slot::Entry(_e) => {
                        super::codec_zenoh_ext_entry::CodecZenohExtEntryWire::write_source_in(&_e, w, &super::codec_zenoh_ext_entry::CodecZenohExtEntryOverrides::header_z(_i + 1 < _k_extensions))?;
                    }
                    ::sce_forge_runtime::codec::Slot::Named => {
                        super::codec_zenoh_ext_entry::CodecZenohExtEntryWire::write_source_in(&CodecChainHasMarkerExtensionsHas0x12Key::named(), w, &super::codec_zenoh_ext_entry::CodecZenohExtEntryOverrides::header_z(_i + 1 < _k_extensions))?;
                    }
                }
            }
        }
        if !matches!(src.extensions_has_0x12(), CodecChainHasMarkerExtensionsHas0x12::Has0x12 { .. }) {
            w.write_vle_u64(_d_payload_len)?;
        }
        if let Some(_v) = _m_payload {
            w.write_bytes(_v.into_inner())?;
        }
        if matches!(src.extensions_has_0x12(), CodecChainHasMarkerExtensionsHas0x12::Has0x12 { .. }) {
            w.write_vle_u32(_d_slice_count)?;
        }
        if let Some(_v) = _m_slices.clone() {
            for _e in _v {
                super::codec_chain_has_marker_slice::CodecChainHasMarkerSliceWire::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecChainHasMarkerSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecChainHasMarkerSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecChainHasMarkerWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecChainHasMarkerWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecChainHasMarkerSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecChainHasMarkerSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecChainHasMarker<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecChainHasMarkerOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecChainHasMarkerOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecChainHasMarkerOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecChainHasMarkerOwned` alone is the build's
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
// `let v: CodecChainHasMarkerOwned = CodecChainHasMarkerOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecChainHasMarker<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecChainHasMarkerIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecChainHasMarkerStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecChainHasMarkerOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecChainHasMarkerFrom<'a>>(self) -> Result<CodecChainHasMarkerOwned<S>, CodecError> {
        Ok(CodecChainHasMarkerOwned {
            header: self.header,
            extensions_has_0x12: match self.extensions_has_0x12 { CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions, slices } => CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions: { let (_r, _p) = extensions.into_parts(); ::sce_forge_runtime::codec::Placed::from_list(::sce_forge_runtime::storage::try_collect_list(_r, |_e| _e.try_into_owned_in::<S>())?, _p).expect("the projection keeps each entry's identifier") }, slices: ::sce_forge_runtime::codec::try_collect_bounded(slices, |_e| _e.try_into_owned_in::<S>())? }, CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions, payload } => CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions: match extensions { Some(_l) => Some(::sce_forge_runtime::codec::Chain::from_list(::sce_forge_runtime::storage::try_collect_list(_l.into_inner(), |_e| _e.try_into_owned_in::<S>())?).expect("the projection keeps each entry's identifier")), None => None }, payload: payload.store::<S, CodecChainHasMarkerPayloadField>()? }, },
            payload_len: self.payload_len,
            slice_count: self.slice_count,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecChainHasMarkerOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecChainHasMarkerStorage> CodecChainHasMarkerOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecChainHasMarkerIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
        let mut _out = Default::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecChainHasMarkerIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        _out.header = self.header;
        match &self.extensions_has_0x12 {
            CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions, slices } => {
                let _o_extensions = ::sce_forge_runtime::codec::Placed::new({ let mut _v = ::sce_forge_runtime::codec::SceList::empty(); ::sce_forge_runtime::codec::try_project_bounded_into(&mut _v, extensions.rest(), |_e| Ok(_e.as_borrowed()))?; _v }, extensions.place()).expect("the projection keeps each entry's identifier");
                let mut _o_slices: ::sce_forge_runtime::codec::Bounded<<::sce_forge_runtime::codec::Lent<'o, S> as ::sce_forge_runtime::codec::HoldsList<CodecChainHasMarkerSlicesField>>::List<CodecChainHasMarkerSliceOwned<::sce_forge_runtime::codec::Lent<'o, S>>>, 4> = Default::default();
                _o_slices.refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, slices.inner(), |_e| Ok(_e.as_borrowed())))?;
                _out.extensions_has_0x12 = CodecChainHasMarkerExtensionsHas0x12::Has0x12 { extensions: _o_extensions, slices: _o_slices };
            }
            CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions, payload } => {
                let _o_extensions = match extensions { Some(_l) => Some(::sce_forge_runtime::codec::Chain::new({ let mut _v = ::sce_forge_runtime::codec::SceList::empty(); ::sce_forge_runtime::codec::try_project_bounded_into(&mut _v, _l.inner(), |_e| Ok(_e.as_borrowed()))?; _v }).expect("the projection keeps each entry's identifier")), None => None };
                let mut _o_payload: ::sce_forge_runtime::codec::Bounded<<::sce_forge_runtime::codec::Lent<'o, S> as ::sce_forge_runtime::codec::HoldsBytes<CodecChainHasMarkerPayloadField>>::Bytes, 256> = Default::default();
                _o_payload = payload.as_bytes();
                _out.extensions_has_0x12 = CodecChainHasMarkerExtensionsHas0x12::NotHas0x12 { extensions: _o_extensions, payload: _o_payload };
            }
        }
        _out.payload_len = self.payload_len;
        _out.slice_count = self.slice_count;
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
    pub fn transcode_in<D: for<'x> CodecChainHasMarkerFrom<'x>>(&self) -> Result<CodecChainHasMarkerOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
