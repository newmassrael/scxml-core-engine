#![doc = "SCE-MAP: codec_zenoh_query_value:24 :: _forge_body"]
// SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

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
use super::codec_zenoh_value_entry::CodecZenohValueEntryOwned;
/// The field `exts` of `CodecZenohQueryValue`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohQueryValueExtsField;

impl ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryValueExtsField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 4> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecZenohQueryValueIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohQueryValueStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryValueExtsField> + super::codec_zenoh_value_entry::CodecZenohValueEntryStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryValueExtsField> + super::codec_zenoh_value_entry::CodecZenohValueEntryStorage> CodecZenohQueryValueStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohQueryValueStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohQueryValueFrom<'f>: CodecZenohQueryValueStorage + super::codec_zenoh_value_entry::CodecZenohValueEntryFrom<'f> {}

impl<'f, S: CodecZenohQueryValueStorage + super::codec_zenoh_value_entry::CodecZenohValueEntryFrom<'f>> CodecZenohQueryValueFrom<'f> for S {}

// `CodecZenohQueryValueIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohQueryValue<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohQueryValueOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohQueryValueIn<S: CodecZenohQueryValueStorage> {
    pub exts: CodecZenohQueryValueExtsList<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohQueryValue<'a> = CodecZenohQueryValueIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohQueryValueOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohQueryValueIn<S>;

impl<S: CodecZenohQueryValueStorage> Default for CodecZenohQueryValueIn<S> {
    fn default() -> Self {
        Self {
            exts: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohQueryValueStorage> CodecZenohQueryValueIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohQueryValueFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohQueryValueFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohQueryValue::store_into(&mut _frame, &mut CodecZenohQueryValue::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohQueryValueFrom<'f> {
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
        _out.exts.refill_from((), |_vec| {
            let mut _prev_exts_after_shm: u8 = 0;
            let mut _more = false;
            for _ in 0..4u32 {
                let _entry = CodecZenohValueEntryOwned::<S>::decode(cursor, _prev_exts_after_shm)?;
                _more = _entry.z();
                _prev_exts_after_shm = u8::from(u64::from(_entry.ext_id()) == 4u64);
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
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 656;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohQueryValueWire::encode_source(self, w)
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

/// The law the entries of `exts` follow in the inputs they say, which
/// the list holds them to (docs/adr/0065 §6 R8).
#[allow(dead_code)]
pub type CodecZenohQueryValueExtsLaw = ::sce_forge_runtime::codec::AfterKey<CodecZenohQueryValueExtsAfterShm, CodecZenohQueryValueExtsHas0x4Key>;

/// The list `exts` at the storage profile `S`, held to its law
/// (docs/adr/0065 §6 R8).
#[allow(dead_code)]
pub type CodecZenohQueryValueExtsList<S> = ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecZenohQueryValueExtsField>>::List<CodecZenohValueEntryOwned<S>>, 4, 1>, CodecZenohQueryValueExtsLaw>;

/// The input `after_shm` each entry of `exts` says, which the list hands
/// every entry as whether the entry before has the identifier 0x4 and holds to its law (docs/adr/0065 §6 R8).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohQueryValueExtsAfterShm;

impl<E: super::codec_zenoh_value_entry::CodecZenohValueEntrySource + ?Sized> ::sce_forge_runtime::codec::Says<E> for CodecZenohQueryValueExtsAfterShm {
    fn says(entry: &E) -> Option<bool> {
        super::codec_zenoh_value_entry::CodecZenohValueEntryWire::input_after_shm(entry)
    }
}

/// The kind of entry the entry after which `exts` supplies its input
/// set: an entry whose identifier is 0x4 (docs/adr/0065 §6 R8b).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohQueryValueExtsHas0x4Key;

impl<E: super::codec_zenoh_value_entry::CodecZenohValueEntrySource + ?Sized> ::sce_forge_runtime::codec::Key<E> for CodecZenohQueryValueExtsHas0x4Key {
    fn matches(entry: &E) -> bool {
        (u64::from(super::codec_zenoh_value_entry::CodecZenohValueEntrySource::header(entry)) & 0xF) == 4u64
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohQueryValue` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohQueryValue`
/// computes those from what is ([`CodecZenohQueryValue::encode_source`]).
pub trait CodecZenohQueryValueSource {
    /// The `exts` field.
    fn exts(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_entry::CodecZenohValueEntrySource> + Clone, 4, 1>, CodecZenohQueryValueExtsLaw>;
}

impl<T: CodecZenohQueryValueSource + ?Sized> CodecZenohQueryValueSource for &T {
    fn exts(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_entry::CodecZenohValueEntrySource> + Clone, 4, 1>, CodecZenohQueryValueExtsLaw> {
        (**self).exts()
    }
}

/// A value of `CodecZenohQueryValue` held in the types a caller already has: a source
/// ([`CodecZenohQueryValueSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohQueryValueParts<'a> {
    /// The `exts` field.
    pub exts: ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::Bounded<&'a [super::codec_zenoh_value_entry::CodecZenohValueEntryParts<'a>], 4, 1>, CodecZenohQueryValueExtsLaw>,
}

impl<'a> CodecZenohQueryValueSource for CodecZenohQueryValueParts<'a> {
    fn exts(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_entry::CodecZenohValueEntrySource> + Clone, 4, 1>, CodecZenohQueryValueExtsLaw> {
        self.exts.walk()
    }
}

impl<S: CodecZenohQueryValueStorage> CodecZenohQueryValueSource for CodecZenohQueryValueOwned<S> {
    fn exts(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_value_entry::CodecZenohValueEntrySource> + Clone, 4, 1>, CodecZenohQueryValueExtsLaw> {
        self.exts.list_walk()
    }
}

/// The codec `CodecZenohQueryValue` itself: what writes a value read through
/// [`CodecZenohQueryValueSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohQueryValueWire;

#[allow(dead_code)]
impl CodecZenohQueryValueWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohQueryValue::encode` writes for the value it holds, read through
    /// [`CodecZenohQueryValueSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohQueryValueSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohQueryValueSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _k_exts: usize = src.exts().len();
        for (_i, _e) in src.exts().into_iter().enumerate() {
            super::codec_zenoh_value_entry::CodecZenohValueEntryWire::write_source_in(&_e, w, &super::codec_zenoh_value_entry::CodecZenohValueEntryOverrides::header_z(_i + 1 < _k_exts))?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohQueryValueSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohQueryValueSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohQueryValueWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohQueryValueWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohQueryValueSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohQueryValueSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohQueryValue<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohQueryValueOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohQueryValueOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohQueryValueOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohQueryValueOwned` alone is the build's
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
// `let v: CodecZenohQueryValueOwned = CodecZenohQueryValueOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohQueryValue<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohQueryValueIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohQueryValueStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohQueryValueOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohQueryValueFrom<'a>>(self) -> Result<CodecZenohQueryValueOwned<S>, CodecError> {
        Ok(CodecZenohQueryValueOwned {
            exts: ::sce_forge_runtime::codec::try_collect_lawful(self.exts, |_e| _e.try_into_owned_in::<S>())?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohQueryValueOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohQueryValueStorage> CodecZenohQueryValueOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohQueryValueIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohQueryValueIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        _out.exts.refill_from(self.exts.start(), |_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, self.exts.inner(), |_e| _e.try_as_borrowed()))?;
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
    pub fn transcode_in<D: for<'x> CodecZenohQueryValueFrom<'x>>(&self) -> Result<CodecZenohQueryValueOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
