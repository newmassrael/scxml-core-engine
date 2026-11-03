#![doc = "SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body"]
// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body

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
use super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerOwned;
/// What `CodecZenohDeclExtKeyexprIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohDeclExtKeyexprStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerStorage> CodecZenohDeclExtKeyexprStorage for S {}

/// What `CodecZenohDeclExtKeyexprIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohDeclExtKeyexpr` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohDeclExtKeyexprLentStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerLentStorage> CodecZenohDeclExtKeyexprLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohDeclExtKeyexprStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohDeclExtKeyexprFrom<'f>: CodecZenohDeclExtKeyexprStorage + super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerFrom<'f> {}

impl<'f, S: CodecZenohDeclExtKeyexprStorage + super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerFrom<'f>> CodecZenohDeclExtKeyexprFrom<'f> for S {}

// `CodecZenohDeclExtKeyexprIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohDeclExtKeyexpr<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohDeclExtKeyexprOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohDeclExtKeyexprIn<S: CodecZenohDeclExtKeyexprStorage> {
    pub outer_header: u8,
    pub total_length: u64,
    pub inner: CodecZenohDeclExtKeyexprInnerOwned<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohDeclExtKeyexpr<'a> = CodecZenohDeclExtKeyexprIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohDeclExtKeyexprOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohDeclExtKeyexprIn<S>;

impl<S: CodecZenohDeclExtKeyexprStorage> Default for CodecZenohDeclExtKeyexprIn<S> {
    fn default() -> Self {
        Self {
            outer_header: Default::default(),
            total_length: Default::default(),
            inner: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohDeclExtKeyexprStorage> CodecZenohDeclExtKeyexprIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohDeclExtKeyexprFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohDeclExtKeyexprFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohDeclExtKeyexpr::store_into(&mut _frame, &mut CodecZenohDeclExtKeyexpr::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohDeclExtKeyexprFrom<'f> {
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
        let outer_header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let total_length = cursor.read_vle_u64()?;
        {
            let _len = total_length as usize;
            let _raw = cursor.peek_slice(_len)?;
            let mut _inner = SceCursor::new(_raw);
            CodecZenohDeclExtKeyexprInnerOwned::<S>::decode_into(&mut _inner, &mut _out.inner)?;
            cursor.advance(_len)?;
        }
        _out.outer_header = outer_header;
        _out.total_length = total_length;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn ext_id(&self) -> u8 {
        self.outer_header & 0x0F
    }

    pub fn set_ext_id(&mut self, v: u8) {
        self.outer_header = (self.outer_header & !0x0F) | (v & 0x0F);
    }

    pub fn m(&self) -> bool {
        (self.outer_header & 0x10) != 0
    }

    pub fn set_m(&mut self, v: bool) {
        if v {
            self.outer_header |= 0x10;
        } else {
            self.outer_header &= !0x10;
        }
    }

    pub fn enc(&self) -> u8 {
        (self.outer_header >> 5) & 0x03
    }

    pub fn set_enc(&mut self, v: u8) {
        self.outer_header = (self.outer_header & !0x60) | ((v & 0x03) << 5);
    }

    pub fn z(&self) -> bool {
        (self.outer_header & 0x80) != 0
    }

    pub fn set_z(&mut self, v: bool) {
        if v {
            self.outer_header |= 0x80;
        } else {
            self.outer_header &= !0x80;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 148;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohDeclExtKeyexprWire::encode_source(self, w)
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

/// Flags of `CodecZenohDeclExtKeyexpr` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecZenohDeclExtKeyexpr`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecZenohDeclExtKeyexprOverrides {
    /// The `M` flag of `outer_header`, when a parent sets it.
    pub outer_header_m: Option<bool>,
    /// The `Z` flag of `outer_header`, when a parent sets it.
    pub outer_header_z: Option<bool>,
}

#[allow(dead_code)]
impl CodecZenohDeclExtKeyexprOverrides {
    /// Sets `M` of `outer_header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn outer_header_m(value: bool) -> Self {
        Self { outer_header_m: Some(value), ..Self::default() }
    }

    /// Sets `Z` of `outer_header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn outer_header_z(value: bool) -> Self {
        Self { outer_header_z: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohDeclExtKeyexpr` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohDeclExtKeyexprWire::encode_source`]).
pub trait CodecZenohDeclExtKeyexprSource {
    /// The `outer_header` field.
    fn outer_header(&self) -> u8;
    /// The `inner` field.
    fn inner(&self) -> &impl super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerSource;
}

impl<T: CodecZenohDeclExtKeyexprSource + ?Sized> CodecZenohDeclExtKeyexprSource for &T {
    fn outer_header(&self) -> u8 {
        (**self).outer_header()
    }
    fn inner(&self) -> &impl super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerSource {
        (**self).inner()
    }
}

/// A value of `CodecZenohDeclExtKeyexpr` held in the types a caller already has: a source
/// ([`CodecZenohDeclExtKeyexprSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohDeclExtKeyexprParts<'a> {
    /// The `outer_header` field.
    pub outer_header: u8,
    /// The `inner` field.
    pub inner: &'a super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerParts<'a>,
}

impl<'a> CodecZenohDeclExtKeyexprSource for CodecZenohDeclExtKeyexprParts<'a> {
    fn outer_header(&self) -> u8 {
        self.outer_header
    }
    fn inner(&self) -> &impl super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerSource {
        self.inner
    }
}

impl<S: CodecZenohDeclExtKeyexprStorage> CodecZenohDeclExtKeyexprSource for CodecZenohDeclExtKeyexprOwned<S> {
    fn outer_header(&self) -> u8 {
        self.outer_header
    }
    fn inner(&self) -> &impl super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerSource {
        &self.inner
    }
}

/// The codec `CodecZenohDeclExtKeyexpr` itself: what writes a value read through
/// [`CodecZenohDeclExtKeyexprSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohDeclExtKeyexprWire;

#[allow(dead_code)]
impl CodecZenohDeclExtKeyexprWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohDeclExtKeyexpr::encode` writes for the value it holds, read through
    /// [`CodecZenohDeclExtKeyexprSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohDeclExtKeyexprSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohDeclExtKeyexprSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source_in(src, w, &CodecZenohDeclExtKeyexprOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecZenohDeclExtKeyexprSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecZenohDeclExtKeyexprOverrides) -> Result<(), K::Error> {
        let _s_inner: usize = {
            let _v = src.inner();
            super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerWire::count_source(_v)
        };
        let _d_total_length: u64 = ::sce_forge_runtime::codec::widen_usize(_s_inner);
        let mut _c_outer_header: u8 = src.outer_header();
        if let Some(_o) = over.outer_header_m {
            _c_outer_header = (_c_outer_header & !0x10) | (u8::from(_o) << 4);
        }
        if let Some(_o) = over.outer_header_z {
            _c_outer_header = (_c_outer_header & !0x80) | (u8::from(_o) << 7);
        }
        w.write_u8(_c_outer_header)?;
        w.write_vle_u64(_d_total_length)?;
        super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInnerWire::write_source(src.inner(), w)?;
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohDeclExtKeyexprSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohDeclExtKeyexprSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohDeclExtKeyexprWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohDeclExtKeyexprWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohDeclExtKeyexprSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohDeclExtKeyexprSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohDeclExtKeyexpr<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohDeclExtKeyexprOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohDeclExtKeyexprOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohDeclExtKeyexprOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohDeclExtKeyexprOwned` alone is the build's
// default profile. Because the non-allocating profile is a *type*, this mirror
// carries no `alloc` gate at all — a list- or embed-bearing codec has an owned
// form on the heap-free tier too, and a heap-capable consumer can still pin a
// value to storage that is guaranteed not to allocate.
//
// `try_into_owned` is the fallible direction (one `?` per profile: on the
// growable profile the copy cannot fail, on the inline profile it enforces
// each declared bound); `as_borrowed` re-borrows any profile back
// into the single borrowed view that owns `encode`; `transcode_in` moves a
// value between profiles as a checked projection rather than a re-decode.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling one instead names it on the value or its binding —
// `let v: CodecZenohDeclExtKeyexprOwned = CodecZenohDeclExtKeyexprOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohDeclExtKeyexpr<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohDeclExtKeyexprIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohDeclExtKeyexprLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohDeclExtKeyexprOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohDeclExtKeyexprFrom<'a>>(self) -> Result<CodecZenohDeclExtKeyexprOwned<S>, CodecError> {
        Ok(CodecZenohDeclExtKeyexprOwned {
            outer_header: self.outer_header,
            total_length: self.total_length,
            inner: self.inner.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclExtKeyexprOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohDeclExtKeyexprStorage> CodecZenohDeclExtKeyexprOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohDeclExtKeyexprIn<::sce_forge_runtime::codec::Lent<'_, S>> {
        let mut _out = Default::default();
        self.as_borrowed_into(&mut _out);
        _out
    }

    /// [`Self::as_borrowed`] into a destination the caller owns, each
    /// bounded list and embedded body projected in the field where it sits.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through (the optional wrapper, the borrowed struct, the
    /// variant, the caller's binding), however few entries the list holds. A
    /// caller on a path where that shows projects into a destination it owns,
    /// then encodes from it. On `Err`, `_out` holds what was projected so far
    /// and is not to be read.
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohDeclExtKeyexprIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        _out.outer_header = self.outer_header;
        _out.total_length = self.total_length;
        self.inner.as_borrowed_into(&mut _out.inner);
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: for<'x> CodecZenohDeclExtKeyexprFrom<'x>>(&self) -> Result<CodecZenohDeclExtKeyexprOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}
