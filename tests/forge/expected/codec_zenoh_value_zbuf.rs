#![doc = "SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body"]
// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

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
use super::codec_zenoh_value_slice::CodecZenohValueSliceOwned;
/// The field `value` of `CodecZenohValueZbuf`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohValueZbufValueField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecZenohValueZbufValueField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<32>;
}

/// The field `slices` of `CodecZenohValueZbuf`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohValueZbufSlicesField;

impl ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 4> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecZenohValueZbufIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohValueZbufStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohValueZbufValueField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField> + super::codec_zenoh_value_slice::CodecZenohValueSliceStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohValueZbufValueField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField> + super::codec_zenoh_value_slice::CodecZenohValueSliceStorage> CodecZenohValueZbufStorage for S {}

/// What `CodecZenohValueZbufIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohValueZbuf` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohValueZbufLentStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField> + super::codec_zenoh_value_slice::CodecZenohValueSliceLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField> + super::codec_zenoh_value_slice::CodecZenohValueSliceLentStorage> CodecZenohValueZbufLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohValueZbufStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohValueZbufFrom<'f>: CodecZenohValueZbufStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohValueZbufValueField> + super::codec_zenoh_value_slice::CodecZenohValueSliceFrom<'f> {}

impl<'f, S: CodecZenohValueZbufStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohValueZbufValueField> + super::codec_zenoh_value_slice::CodecZenohValueSliceFrom<'f>> CodecZenohValueZbufFrom<'f> for S {}

/// The group `CodecZenohValueZbufAfterShm` as `CodecZenohValueZbufOwned` holds it, in the storage
/// profile `S`.
#[allow(dead_code)]
pub type CodecZenohValueZbufAfterShmOwned<S> = CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField>>::List<CodecZenohValueSliceOwned<S>>, 4>, ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecZenohValueZbufValueField>>::Bytes, 32>>;

// `CodecZenohValueZbufIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohValueZbuf<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohValueZbufOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohValueZbufIn<S: CodecZenohValueZbufStorage> {
    pub after_shm: CodecZenohValueZbufAfterShmOwned<S>,
    pub slice_count: Option<u64>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohValueZbuf<'a> = CodecZenohValueZbufIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohValueZbufOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohValueZbufIn<S>;

impl<S: CodecZenohValueZbufStorage> Default for CodecZenohValueZbufIn<S> {
    fn default() -> Self {
        Self {
            after_shm: Default::default(),
            slice_count: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohValueZbufStorage> CodecZenohValueZbufIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>, after_shm: u8) -> Result<Self, CodecError> where S: CodecZenohValueZbufFrom<'f> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, after_shm)?;
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, after_shm: u8) -> Result<(), CodecError> where S: CodecZenohValueZbufFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out, after_shm) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohValueZbuf::store_into(&mut _frame, &mut CodecZenohValueZbuf::default(), after_shm)?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, after_shm: u8) -> Result<(), CodecError> where S: CodecZenohValueZbufFrom<'f> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = after_shm;
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
        let value_len = cursor.read_vle_u64()?;
        let value = if (after_shm & 0x01u8) == 0 {
            let _len = value_len;
            if _len > 32 {
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
        let value = value.map(|_v| ::sce_forge_runtime::codec::Bounded::new(_v).ok_or(CodecError::LengthOutOfRange)).transpose()?;
        let value = value.map(|_v| _v.store::<S, CodecZenohValueZbufValueField>()).transpose()?;
        let encoding = if (after_shm & 0x01u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let slice_count = if (after_shm & 0x01u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let mut slices: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField>>::List<CodecZenohValueSliceOwned<S>>, 4>> = None;
        if (after_shm & 0x01u8) != 0 {
            let _n = slice_count.expect("co-gating: count present-if matches repeat");
            if _n > 4 {
                return Err(CodecError::TooManyElements);
            }
            slices.insert(Default::default()).refill(|_vec| {
                for _ in 0.._n {
                    ::sce_forge_runtime::codec::SceList::try_push(_vec, CodecZenohValueSliceOwned::<S>::decode(cursor)?)?;
                }
                Ok(())
            })?;
        } else {
            slices = None;
        }
        _out.after_shm = if (after_shm & 0x01u8) != 0 { CodecZenohValueZbufAfterShm::AfterShm { encoding: encoding.unwrap_or_default(), slices: slices.unwrap_or_default(), value_len } } else { CodecZenohValueZbufAfterShm::NotAfterShm { value: value.unwrap_or_default() } };
        _out.slice_count = slice_count;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 163;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohValueZbufWire::encode_source(self, w)
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

/// The fields `after_shm` decide (docs/adr/0065 §6 R4b, R4c): a value holds the
/// ones of one arm, and `CodecZenohValueZbuf` derives the gates from which.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodecZenohValueZbufAfterShm<Slices, Value> {
    /// While `after_shm` holds.
    AfterShm {
        /// The `encoding` field.
        encoding: u64,
        /// The `slices` field.
        slices: Slices,
        /// The `value_len` field.
        value_len: u64,
    },
    /// While `after_shm` does not hold.
    NotAfterShm {
        /// The `value` field.
        value: Value,
    },
}

impl<Slices: Default, Value: Default> Default for CodecZenohValueZbufAfterShm<Slices, Value> {
    /// The arm held while `after_shm` does not hold, each field at its type's default.
    fn default() -> Self {
        Self::NotAfterShm { value: Default::default(), }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohValueZbuf` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohValueZbufWire::encode_source`]).
pub trait CodecZenohValueZbufSource {
    /// The fields `after_shm` decides, held together (docs/adr/0065 §6 R4).
    fn after_shm(&self) -> CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_slice::CodecZenohValueSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 32>>;
}

impl<T: CodecZenohValueZbufSource + ?Sized> CodecZenohValueZbufSource for &T {
    fn after_shm(&self) -> CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_slice::CodecZenohValueSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 32>> {
        (**self).after_shm()
    }
}

/// A value of `CodecZenohValueZbuf` held in the types a caller already has: a source
/// ([`CodecZenohValueZbufSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohValueZbufParts<'a> {
    /// The fields `after_shm` decides, held together (docs/adr/0065 §6 R4).
    pub after_shm: CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::Bounded<&'a [super::codec_zenoh_value_slice::CodecZenohValueSliceParts<'a>], 4>, ::sce_forge_runtime::codec::Bytes<'a, 32>>,
}

impl<'a> CodecZenohValueZbufSource for CodecZenohValueZbufParts<'a> {
    fn after_shm(&self) -> CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_slice::CodecZenohValueSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 32>> {
        match &self.after_shm {
            CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => CodecZenohValueZbufAfterShm::AfterShm { encoding: *encoding, slices: slices.iter(), value_len: *value_len },
            CodecZenohValueZbufAfterShm::NotAfterShm { value } => CodecZenohValueZbufAfterShm::NotAfterShm { value: *value },
        }
    }
}

impl<S: CodecZenohValueZbufStorage> CodecZenohValueZbufSource for CodecZenohValueZbufOwned<S> {
    fn after_shm(&self) -> CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_slice::CodecZenohValueSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 32>> {
        match &self.after_shm {
            CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => CodecZenohValueZbufAfterShm::AfterShm { encoding: *encoding, slices: slices.list_iter(), value_len: *value_len },
            CodecZenohValueZbufAfterShm::NotAfterShm { value } => CodecZenohValueZbufAfterShm::NotAfterShm { value: value.as_bytes() },
        }
    }
}

/// The codec `CodecZenohValueZbuf` itself: what writes a value read through
/// [`CodecZenohValueZbufSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohValueZbufWire;

#[allow(dead_code)]
impl CodecZenohValueZbufWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohValueZbuf::encode` writes for the value it holds, read through
    /// [`CodecZenohValueZbufSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohValueZbufSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The flag-input `after_shm` that `src` says: set exactly when the value holds the side of `after_shm` present while it holds.
    /// A parent hands the decode this bit and derives it from here for the
    /// encode.
    pub fn input_after_shm<S: CodecZenohValueZbufSource + ?Sized>(src: &S) -> bool {
        matches!(CodecZenohValueZbufSource::after_shm(src), CodecZenohValueZbufAfterShm::AfterShm { .. })
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohValueZbufSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let (_m_encoding, _m_slices, _m_value_len, _m_value) = match src.after_shm() {
            CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => (Some(encoding), Some(slices), Some(value_len), None),
            CodecZenohValueZbufAfterShm::NotAfterShm { value } => (None, None, None, Some(value)),
        };
        let after_shm: u8 = u8::from(matches!(CodecZenohValueZbufSource::after_shm(src), CodecZenohValueZbufAfterShm::AfterShm { .. }));
        let _k_slices: usize = _m_slices.clone().map_or(0, |_l| _l.len());
        let _n_d_value_len_0: usize = match _m_value {
            Some(_v) => _v.len(),
            None => 0,
        };
        let _f_d_value_len: Option<usize> = if _m_value.is_some() { Some(_n_d_value_len_0) } else { None };
        let _d_value_len: u64 = match _f_d_value_len {
            Some(_n) => ::sce_forge_runtime::codec::widen_usize(_n),
            None => _m_value_len.unwrap_or_default(),
        };
        let _f_d_slice_count: Option<usize> = if _m_slices.clone().is_some() { Some(_k_slices) } else { None };
        let _d_slice_count: u64 = ::sce_forge_runtime::codec::widen_usize(_f_d_slice_count.unwrap_or(0));
        w.write_vle_u64(_d_value_len)?;
        if let Some(_v) = _m_value {
            w.write_bytes(_v.into_inner())?;
        }
        if let Some(_v) = _m_encoding {
            w.write_vle_u64(_v)?;
        }
        if (after_shm & 0x1) != 0 {
            w.write_vle_u64(_d_slice_count)?;
        }
        if let Some(_v) = _m_slices.clone() {
            for _e in _v {
                super::codec_zenoh_value_slice::CodecZenohValueSliceWire::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohValueZbufSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohValueZbufSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohValueZbufWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohValueZbufWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohValueZbufSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohValueZbufSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohValueZbuf<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohValueZbufOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohValueZbufOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohValueZbufOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohValueZbufOwned` alone is the build's
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
// `let v: CodecZenohValueZbufOwned = CodecZenohValueZbufOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohValueZbuf<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohValueZbufIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohValueZbufLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohValueZbufOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohValueZbufFrom<'a>>(self) -> Result<CodecZenohValueZbufOwned<S>, CodecError> {
        Ok(CodecZenohValueZbufOwned {
            after_shm: match self.after_shm { CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => CodecZenohValueZbufAfterShm::AfterShm { encoding, slices: ::sce_forge_runtime::codec::try_collect_bounded(slices, |_e| _e.try_into_owned_in::<S>())?, value_len }, CodecZenohValueZbufAfterShm::NotAfterShm { value } => CodecZenohValueZbufAfterShm::NotAfterShm { value: value.store::<S, CodecZenohValueZbufValueField>()? }, },
            slice_count: self.slice_count,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohValueZbufOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohValueZbufStorage> CodecZenohValueZbufOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohValueZbufIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohValueZbufIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        match &self.after_shm {
            CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => {
                let mut _o_encoding: u64 = Default::default();
                _o_encoding = *encoding;
                let mut _o_slices: ::sce_forge_runtime::codec::Bounded<<::sce_forge_runtime::codec::Lent<'o, S> as ::sce_forge_runtime::codec::HoldsList<CodecZenohValueZbufSlicesField>>::List<CodecZenohValueSliceOwned<::sce_forge_runtime::codec::Lent<'o, S>>>, 4> = Default::default();
                _o_slices.refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, slices.inner(), |_e| Ok(_e.as_borrowed())))?;
                let mut _o_value_len: u64 = Default::default();
                _o_value_len = *value_len;
                _out.after_shm = CodecZenohValueZbufAfterShm::AfterShm { encoding: _o_encoding, slices: _o_slices, value_len: _o_value_len };
            }
            CodecZenohValueZbufAfterShm::NotAfterShm { value } => {
                let mut _o_value: ::sce_forge_runtime::codec::Bounded<<::sce_forge_runtime::codec::Lent<'o, S> as ::sce_forge_runtime::codec::HoldsBytes<CodecZenohValueZbufValueField>>::Bytes, 32> = Default::default();
                _o_value = value.as_bytes();
                _out.after_shm = CodecZenohValueZbufAfterShm::NotAfterShm { value: _o_value };
            }
        }
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
    pub fn transcode_in<D: for<'x> CodecZenohValueZbufFrom<'x>>(&self) -> Result<CodecZenohValueZbufOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
