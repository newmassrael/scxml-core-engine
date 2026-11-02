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
// RFC §synth-5-B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_zenoh_value_slice::CodecZenohValueSlice;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohValueZbuf<'a> {
    pub after_shm: CodecZenohValueZbufAfterShm<HeaplessVec<CodecZenohValueSlice<'a>, 4>, ::sce_forge_runtime::codec::Bytes<'a, 32>>,
    pub slice_count: Option<u64>,
}

#[allow(dead_code)]
impl<'a> CodecZenohValueZbuf<'a> {
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
    pub fn decode(cursor: &mut SceCursor<'a>, after_shm: u8) -> Result<Self, CodecError> {
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
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self, after_shm: u8) -> Result<(), CodecError> {
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
        let mut slices: Option<HeaplessVec<CodecZenohValueSlice<'a>, 4>> = None;
        if (after_shm & 0x01u8) != 0 {
            let _n = slice_count.expect("co-gating: count present-if matches repeat");
            if _n > 4 {
                return Err(CodecError::TooManyElements);
            }
            let _vec = slices.insert(HeaplessVec::<CodecZenohValueSlice<'a>, 4>::new());
            for _ in 0.._n {
                _vec.push(CodecZenohValueSlice::decode(cursor)?)
                    .map_err(|_| CodecError::CapacityExceeded)?;
            }
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
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        Self::encode_source(self, w)
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
/// or the flag a gated field is announced by is not among them; `CodecZenohValueZbuf`
/// computes those from what is ([`CodecZenohValueZbuf::encode_source`]).
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

impl<'a> CodecZenohValueZbufSource for CodecZenohValueZbuf<'a> {
    fn after_shm(&self) -> CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_slice::CodecZenohValueSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 32>> {
        match &self.after_shm {
            CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => CodecZenohValueZbufAfterShm::AfterShm { encoding: *encoding, slices: ::sce_forge_runtime::codec::BoundedIter::from(slices), value_len: *value_len },
            CodecZenohValueZbufAfterShm::NotAfterShm { value } => CodecZenohValueZbufAfterShm::NotAfterShm { value: *value },
        }
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohValueZbufSource for CodecZenohValueZbufOwned<S> {
    fn after_shm(&self) -> CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_slice::CodecZenohValueSliceSource> + Clone, 4>, ::sce_forge_runtime::codec::Bytes<'_, 32>> {
        match &self.after_shm {
            CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => CodecZenohValueZbufAfterShm::AfterShm { encoding: *encoding, slices: slices.list_iter(), value_len: *value_len },
            CodecZenohValueZbufAfterShm::NotAfterShm { value } => CodecZenohValueZbufAfterShm::NotAfterShm { value: value.as_bytes() },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohValueZbuf<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
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
                CodecZenohValueSlice::write_source(&_e, w)?;
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

impl<'a, S: CodecZenohValueZbufSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohValueZbuf<'a> {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohValueZbuf<'a> {
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
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_value_slice::CodecZenohValueSliceOwned;
/// The group `CodecZenohValueZbufAfterShm` as `CodecZenohValueZbufOwned` holds it, in the storage
/// profile `S`.
#[allow(dead_code)]
pub type CodecZenohValueZbufAfterShmOwned<S> = CodecZenohValueZbufAfterShm<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::CodecStorage>::List<CodecZenohValueSliceOwned<S>, 4>, 4>, ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::CodecStorage>::Bytes<32>, 32>>;

// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohValueZbufOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub after_shm: CodecZenohValueZbufAfterShmOwned<S>,
    pub slice_count: Option<u64>,
}

#[allow(dead_code)]
impl<'a> CodecZenohValueZbuf<'a> {
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
    /// `CodecError::TooManyElements` for a value it cannot hold.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohValueZbufOwned<S>, CodecError> {
        Ok(CodecZenohValueZbufOwned {
            after_shm: match self.after_shm { CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => CodecZenohValueZbufAfterShm::AfterShm { encoding, slices: ::sce_forge_runtime::codec::try_collect_bounded(slices, |_e| _e.try_into_owned_in::<S>())?, value_len }, CodecZenohValueZbufAfterShm::NotAfterShm { value } => CodecZenohValueZbufAfterShm::NotAfterShm { value: ::sce_forge_runtime::codec::Bounded::copied(value)? }, },
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
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohValueZbufOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohValueZbuf<'_>, CodecError> {
        let mut _out = CodecZenohValueZbuf::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohValueZbuf<'o>) -> Result<(), CodecError> {
        match &self.after_shm {
            CodecZenohValueZbufAfterShm::AfterShm { encoding, slices, value_len } => {
                let mut _o_encoding: u64 = Default::default();
                _o_encoding = *encoding;
                let mut _o_slices: HeaplessVec<CodecZenohValueSlice<'o>, 4> = Default::default();
                ::sce_forge_runtime::codec::try_project_bounded_into(&mut _o_slices, slices.inner(), |_e| Ok(_e.as_borrowed()))?;
                let mut _o_value_len: u64 = Default::default();
                _o_value_len = *value_len;
                _out.after_shm = CodecZenohValueZbufAfterShm::AfterShm { encoding: _o_encoding, slices: _o_slices, value_len: _o_value_len };
            }
            CodecZenohValueZbufAfterShm::NotAfterShm { value } => {
                let mut _o_value: ::sce_forge_runtime::codec::Bytes<'o, 32> = Default::default();
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
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohValueZbufOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohValueZbufOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `try_as_borrowed` followed by
    /// [`CodecZenohValueZbuf::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here.
    /// Every value has a frame (docs/adr/0065), so the only error is the
    /// sink's own.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohValueZbuf::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecZenohValueZbuf::MAX_ENCODED_BYTES);
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}
