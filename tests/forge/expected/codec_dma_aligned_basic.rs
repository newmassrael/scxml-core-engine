#![doc = "SCE-MAP: codec_dma_aligned_basic:20 :: _forge_body"]
// SCE-MAP: codec_dma_aligned_basic:20 :: _forge_body

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
// RFC §synth-5-B B3 DMA alignment primitive: structural drift detection.
// Build-time validation already guaranteed `byte_offset % burst_align
// == 0` and that all preceding fields are Fixed bit-size. These
// `const _: () = assert!` declarations catch any future hand-edit to
// the byte_offset that would break the wire-layout invariant.
// `byte_off` and `dma_burst_align` are baked from independent SCXML inputs,
// but clippy sees only integer literals: the check reads as a constant
// (assertions_on_constants) and, whenever the offset equals the alignment,
// as `x % x` (eq_op, deny-by-default). The assert is nonetheless a genuine
// compile-time drift guard — a future hand-edit that misaligns the offset
// must fail to compile — so the lints are suppressed at this site only.
#[allow(clippy::eq_op, clippy::assertions_on_constants)]
const _: () = assert!(
    32 % 32 == 0,
    "RFC §synth-5-B B3: codec field 'aligned_payload' offset must be 32-aligned"
);

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
/// The field `aligned_payload` of `CodecDmaAlignedBasic`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecDmaAlignedBasicAlignedPayloadField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecDmaAlignedBasicAlignedPayloadField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<64>;
}

/// What `CodecDmaAlignedBasicIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecDmaAlignedBasicStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecDmaAlignedBasicAlignedPayloadField> {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecDmaAlignedBasicAlignedPayloadField>> CodecDmaAlignedBasicStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecDmaAlignedBasicStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecDmaAlignedBasicFrom<'f>: CodecDmaAlignedBasicStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecDmaAlignedBasicAlignedPayloadField> {}

impl<'f, S: CodecDmaAlignedBasicStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecDmaAlignedBasicAlignedPayloadField>> CodecDmaAlignedBasicFrom<'f> for S {}

// `CodecDmaAlignedBasicIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecDmaAlignedBasic<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecDmaAlignedBasicOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecDmaAlignedBasicIn<S: CodecDmaAlignedBasicStorage> {
    pub msg_id: u8,
    pub reserved: u8,
    pub aligned_payload: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecDmaAlignedBasicAlignedPayloadField>>::Bytes, 64>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecDmaAlignedBasic<'a> = CodecDmaAlignedBasicIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecDmaAlignedBasicOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecDmaAlignedBasicIn<S>;

impl<S: CodecDmaAlignedBasicStorage> Default for CodecDmaAlignedBasicIn<S> {
    fn default() -> Self {
        Self {
            msg_id: Default::default(),
            reserved: Default::default(),
            aligned_payload: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecDmaAlignedBasicStorage> CodecDmaAlignedBasicIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecDmaAlignedBasicFrom<'f> {
        // Judged as [`Self::decode_into`] judges a frame past a capacity
        // (docs/adr/0077 §7).
        let mut _frame = *cursor;
        match Self::stored(cursor) {
            Err(CodecError::CapacityExceeded) => {
                CodecDmaAlignedBasic::stored(&mut _frame)?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode`], which stops where a field is past its
    /// capacity.
    fn stored<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecDmaAlignedBasicFrom<'f> {
        // Variable-length codec. RFC §synth-5-B B3 stream-correct shape:
        // a codec without `<sce:field sce:bit-size="tail">` consumes
        // only `min_bytes + length_value` rather than the entire
        // cursor remaining. Codecs WITH a tail field still consume
        // to end (tail's definition forces it). The prior
        // "consume entire cursor" behaviour deferred to "the first
        // multi-frame consumer" — the TLV chain is that consumer,
        // so length-ref entry codecs now decode-iterably from a
        // shared cursor without each entry eating the next entry's
        // bytes.
        let _frame_len = cursor.remaining();
        if _frame_len < 32 {
            return Err(CodecError::NeedMoreBytes);
        }
        let raw = cursor.peek_slice(_frame_len)?;
        let msg_id = raw[0];
        let reserved = raw[1];
        if (_frame_len - 32) > 64 {
            return Err(CodecError::LengthOutOfRange);
        }
        let aligned_payload = &raw[32..];
        let aligned_payload = ::sce_forge_runtime::codec::Bounded::new(aligned_payload).ok_or(CodecError::LengthOutOfRange)?;
        let aligned_payload = aligned_payload.store::<S, CodecDmaAlignedBasicAlignedPayloadField>()?;
        let value = Self {
            msg_id,
            reserved,
            aligned_payload,
        };
        cursor.advance(_frame_len)?;
        Ok(value)
    }

    /// [`Self::decode`] into a destination the caller owns, so a codec that
    /// embeds or dispatches to this one decodes it where it will live (see
    /// `decode_into` on a codec that holds a bounded list). This body has no
    /// list of its own to build in place, so it decodes and stores.
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecDmaAlignedBasicFrom<'f> {
        *_out = Self::decode(cursor)?;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 96;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecDmaAlignedBasicWire::encode_source(self, w)
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
/// What a value of `CodecDmaAlignedBasic` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecDmaAlignedBasic`
/// computes those from what is ([`CodecDmaAlignedBasic::encode_source`]).
pub trait CodecDmaAlignedBasicSource {
    /// The `msg_id` field.
    fn msg_id(&self) -> u8;
    /// The `reserved` field.
    fn reserved(&self) -> u8;
    /// The `aligned_payload` field.
    fn aligned_payload(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 64>;
}

impl<T: CodecDmaAlignedBasicSource + ?Sized> CodecDmaAlignedBasicSource for &T {
    fn msg_id(&self) -> u8 {
        (**self).msg_id()
    }
    fn reserved(&self) -> u8 {
        (**self).reserved()
    }
    fn aligned_payload(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 64> {
        (**self).aligned_payload()
    }
}

/// A value of `CodecDmaAlignedBasic` held in the types a caller already has: a source
/// ([`CodecDmaAlignedBasicSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecDmaAlignedBasicParts<'a> {
    /// The `msg_id` field.
    pub msg_id: u8,
    /// The `reserved` field.
    pub reserved: u8,
    /// The `aligned_payload` field.
    pub aligned_payload: ::sce_forge_runtime::codec::Bytes<'a, 64>,
}

impl<'a> CodecDmaAlignedBasicSource for CodecDmaAlignedBasicParts<'a> {
    fn msg_id(&self) -> u8 {
        self.msg_id
    }
    fn reserved(&self) -> u8 {
        self.reserved
    }
    fn aligned_payload(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 64> {
        self.aligned_payload
    }
}

impl<S: CodecDmaAlignedBasicStorage> CodecDmaAlignedBasicSource for CodecDmaAlignedBasicOwned<S> {
    fn msg_id(&self) -> u8 {
        self.msg_id
    }
    fn reserved(&self) -> u8 {
        self.reserved
    }
    fn aligned_payload(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 64> {
        self.aligned_payload.as_bytes()
    }
}

/// The codec `CodecDmaAlignedBasic` itself: what writes a value read through
/// [`CodecDmaAlignedBasicSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecDmaAlignedBasicWire;

#[allow(dead_code)]
impl CodecDmaAlignedBasicWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecDmaAlignedBasic::encode` writes for the value it holds, read through
    /// [`CodecDmaAlignedBasicSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecDmaAlignedBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecDmaAlignedBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        w.write_u8(src.msg_id())?;
        w.write_u8(src.reserved())?;
        while w.position() < 32 {
            w.write_u8(0)?;
        }
        w.write_bytes(src.aligned_payload().into_inner())?;
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecDmaAlignedBasicSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecDmaAlignedBasicSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecDmaAlignedBasicWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecDmaAlignedBasicWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecDmaAlignedBasicSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecDmaAlignedBasicSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecDmaAlignedBasic<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecDmaAlignedBasicOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecDmaAlignedBasicOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecDmaAlignedBasicOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecDmaAlignedBasicOwned` alone is the build's
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
// `let v: CodecDmaAlignedBasicOwned = CodecDmaAlignedBasicOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecDmaAlignedBasic<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecDmaAlignedBasicIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecDmaAlignedBasicStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecDmaAlignedBasicOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecDmaAlignedBasicFrom<'a>>(self) -> Result<CodecDmaAlignedBasicOwned<S>, CodecError> {
        Ok(CodecDmaAlignedBasicOwned {
            msg_id: self.msg_id,
            reserved: self.reserved,
            aligned_payload: self.aligned_payload.store::<S, CodecDmaAlignedBasicAlignedPayloadField>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecDmaAlignedBasicOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecDmaAlignedBasicStorage> CodecDmaAlignedBasicOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecDmaAlignedBasicIn<::sce_forge_runtime::codec::Lent<'_, S>> {
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecDmaAlignedBasicIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        _out.msg_id = self.msg_id;
        _out.reserved = self.reserved;
        _out.aligned_payload = self.aligned_payload.as_bytes();
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: for<'x> CodecDmaAlignedBasicFrom<'x>>(&self) -> Result<CodecDmaAlignedBasicOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}
