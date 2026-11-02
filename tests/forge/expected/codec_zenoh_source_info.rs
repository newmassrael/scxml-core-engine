#![doc = "SCE-MAP: codec_zenoh_source_info:57 :: _forge_body"]
// SCE-MAP: codec_zenoh_source_info:57 :: _forge_body

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
/// The field `zid` of `CodecZenohSourceInfo`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohSourceInfoZidField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecZenohSourceInfoZidField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16>;
}

/// What `CodecZenohSourceInfoIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohSourceInfoStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohSourceInfoZidField> {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohSourceInfoZidField>> CodecZenohSourceInfoStorage for S {}

/// What `CodecZenohSourceInfoIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohSourceInfo` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohSourceInfoLentStorage: ::core::fmt::Debug + Clone + PartialEq {}

impl<D: ::core::fmt::Debug + Clone + PartialEq> CodecZenohSourceInfoLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohSourceInfoStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohSourceInfoFrom<'f>: CodecZenohSourceInfoStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohSourceInfoZidField> {}

impl<'f, S: CodecZenohSourceInfoStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohSourceInfoZidField>> CodecZenohSourceInfoFrom<'f> for S {}

// `CodecZenohSourceInfoIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohSourceInfo<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohSourceInfoOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohSourceInfoIn<S: CodecZenohSourceInfoStorage> {
    pub header: u8,
    pub zid: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecZenohSourceInfoZidField>>::Bytes, 16, 1>,
    pub eid: u32,
    pub sn: u32,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohSourceInfo<'a> = CodecZenohSourceInfoIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohSourceInfoOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohSourceInfoIn<S>;

impl<S: CodecZenohSourceInfoStorage> Default for CodecZenohSourceInfoIn<S> {
    fn default() -> Self {
        Self {
            header: Default::default(),
            zid: Default::default(),
            eid: Default::default(),
            sn: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohSourceInfoStorage> CodecZenohSourceInfoIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohSourceInfoFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohSourceInfoFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohSourceInfo::store_into(&mut _frame, &mut CodecZenohSourceInfo::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohSourceInfoFrom<'f> {
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
        let zid = {
            let _len = (header >> 4) & 0xF;
            let _n = (_len as usize) + 1;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            _v
        };
        let zid = ::sce_forge_runtime::codec::Bounded::new(zid).ok_or(CodecError::LengthOutOfRange)?;
        let zid = zid.store::<S, CodecZenohSourceInfoZidField>()?;
        let eid = cursor.read_vle_u32()?;
        let sn = cursor.read_vle_u32()?;
        _out.header = header;
        _out.zid = zid;
        _out.eid = eid;
        _out.sn = sn;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn zidlen_m1(&self) -> u8 {
        (self.header >> 4) & 0x0F
    }

    pub fn set_zidlen_m1(&mut self, v: u8) {
        self.header = (self.header & !0xF0) | ((v & 0x0F) << 4);
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 27;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohSourceInfoWire::encode_source(self, w)
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
/// What a value of `CodecZenohSourceInfo` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohSourceInfo`
/// computes those from what is ([`CodecZenohSourceInfo::encode_source`]).
pub trait CodecZenohSourceInfoSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `zid` field.
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1>;
    /// The `eid` field.
    fn eid(&self) -> u32;
    /// The `sn` field.
    fn sn(&self) -> u32;
}

impl<T: CodecZenohSourceInfoSource + ?Sized> CodecZenohSourceInfoSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        (**self).zid()
    }
    fn eid(&self) -> u32 {
        (**self).eid()
    }
    fn sn(&self) -> u32 {
        (**self).sn()
    }
}

/// A value of `CodecZenohSourceInfo` held in the types a caller already has: a source
/// ([`CodecZenohSourceInfoSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohSourceInfoParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `zid` field.
    pub zid: ::sce_forge_runtime::codec::Bytes<'a, 16, 1>,
    /// The `eid` field.
    pub eid: u32,
    /// The `sn` field.
    pub sn: u32,
}

impl<'a> CodecZenohSourceInfoSource for CodecZenohSourceInfoParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        self.zid
    }
    fn eid(&self) -> u32 {
        self.eid
    }
    fn sn(&self) -> u32 {
        self.sn
    }
}

impl<S: CodecZenohSourceInfoStorage> CodecZenohSourceInfoSource for CodecZenohSourceInfoOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        self.zid.as_bytes()
    }
    fn eid(&self) -> u32 {
        self.eid
    }
    fn sn(&self) -> u32 {
        self.sn
    }
}

/// The codec `CodecZenohSourceInfo` itself: what writes a value read through
/// [`CodecZenohSourceInfoSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohSourceInfoWire;

#[allow(dead_code)]
impl CodecZenohSourceInfoWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohSourceInfo::encode` writes for the value it holds, read through
    /// [`CodecZenohSourceInfoSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohSourceInfoSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohSourceInfoSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _n_d_header_zidlen_m1_0: usize = src.zid().len() - 1;
        let _v_header_zidlen_m1: u8 = _n_d_header_zidlen_m1_0 as u8;
        let _c_header: u8 = (src.header() & !0xF0) | (_v_header_zidlen_m1 << 4);
        w.write_u8(_c_header)?;
        w.write_bytes(src.zid().into_inner())?;
        w.write_vle_u32(src.eid())?;
        w.write_vle_u32(src.sn())?;
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohSourceInfoSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohSourceInfoSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohSourceInfoWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohSourceInfoWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohSourceInfoSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohSourceInfoSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohSourceInfo<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohSourceInfoOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohSourceInfoOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohSourceInfoOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohSourceInfoOwned` alone is the build's
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
// `let v: CodecZenohSourceInfoOwned = CodecZenohSourceInfoOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohSourceInfo<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohSourceInfoIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohSourceInfoLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohSourceInfoOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohSourceInfoFrom<'a>>(self) -> Result<CodecZenohSourceInfoOwned<S>, CodecError> {
        Ok(CodecZenohSourceInfoOwned {
            header: self.header,
            zid: self.zid.store::<S, CodecZenohSourceInfoZidField>()?,
            eid: self.eid,
            sn: self.sn,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohSourceInfoOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohSourceInfoStorage> CodecZenohSourceInfoOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohSourceInfoIn<::sce_forge_runtime::codec::Lent<'_, S>> {
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohSourceInfoIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        _out.header = self.header;
        _out.zid = self.zid.as_bytes();
        _out.eid = self.eid;
        _out.sn = self.sn;
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: for<'x> CodecZenohSourceInfoFrom<'x>>(&self) -> Result<CodecZenohSourceInfoOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}
