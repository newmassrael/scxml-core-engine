#![doc = "SCE-MAP: codec_tlv_entry:10 :: _forge_body"]
// SCE-MAP: codec_tlv_entry:10 :: _forge_body

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
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecTlvEntry<'a> {
    pub entry_type: u8,
    pub entry_len: u8,
    pub entry_body: &'a [u8],
}

#[allow(dead_code)]
impl<'a> CodecTlvEntry<'a> {
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
        if _frame_len < 2 {
            return Err(CodecError::NeedMoreBytes);
        }
        let raw = cursor.peek_slice(_frame_len)?;
        let entry_type = raw[0];
        let entry_len = raw[1];
        let entry_body = raw.get(2..2 + entry_len as usize).ok_or(CodecError::NeedMoreBytes)?;
        let value = Self {
            entry_type,
            entry_len,
            entry_body,
        };
        // Stream-correct: advance only the bytes actually decoded.
        // For each length-ref field, end = byte_off + sibling local
        // value (the sibling let-binding ran before the payload's).
        // Take the max across all length-ref fields; min_bytes is the
        // lower bound.
        let mut _consumed: usize = 2;
        {
            let _end = 2usize + value.entry_body.len();
            if _end > _consumed { _consumed = _end; }
        }
        if _consumed > _frame_len {
            return Err(CodecError::NeedMoreBytes);
        }
        cursor.advance(_consumed)?;
        Ok(value)
    }

    /// [`Self::decode`] into a destination the caller owns, so a codec that
    /// embeds or dispatches to this one decodes it where it will live (see
    /// `decode_into` on a codec that holds a bounded list). This body has no
    /// list of its own to build in place, so it decodes and stores.
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self) -> Result<(), CodecError> {
        *_out = Self::decode(cursor)?;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 34;

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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecTlvEntry` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecTlvEntry`
/// computes those from what is ([`CodecTlvEntry::encode_source`]).
pub trait CodecTlvEntrySource {
    /// The `entry_type` field.
    fn entry_type(&self) -> u8;
    /// The `entry_body` field.
    fn entry_body(&self) -> &[u8];
}

impl<T: CodecTlvEntrySource + ?Sized> CodecTlvEntrySource for &T {
    fn entry_type(&self) -> u8 {
        (**self).entry_type()
    }
    fn entry_body(&self) -> &[u8] {
        (**self).entry_body()
    }
}

/// A value of `CodecTlvEntry` held in the types a caller already has: a source
/// ([`CodecTlvEntrySource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecTlvEntryParts<'a> {
    /// The `entry_type` field.
    pub entry_type: u8,
    /// The `entry_body` field.
    pub entry_body: &'a [u8],
}

impl<'a> CodecTlvEntrySource for CodecTlvEntryParts<'a> {
    fn entry_type(&self) -> u8 {
        self.entry_type
    }
    fn entry_body(&self) -> &[u8] {
        self.entry_body
    }
}

impl<'a> CodecTlvEntrySource for CodecTlvEntry<'a> {
    fn entry_type(&self) -> u8 {
        self.entry_type
    }
    fn entry_body(&self) -> &[u8] {
        self.entry_body
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTlvEntrySource for CodecTlvEntryOwned<S> {
    fn entry_type(&self) -> u8 {
        self.entry_type
    }
    fn entry_body(&self) -> &[u8] {
        ::sce_forge_runtime::codec::SceByteBuf::as_slice(&self.entry_body)
    }
}

#[allow(dead_code)]
impl<'a> CodecTlvEntry<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecTlvEntrySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecTlvEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecTlvEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        let _n_d_entry_len_0: usize = src.entry_body().len();
        let _d_entry_len: u8 = u8::try_from(_n_d_entry_len_0).map_err(|_| CodecError::LengthOutOfRange)?;
        w.write_u8(src.entry_type())?;
        w.write_u8(_d_entry_len)?;
        w.write_bytes(src.entry_body())?;
        Ok(())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecTlvEntry` holds no bounded list, embedded body or variant, so a
// view of it would hold what it holds. Its view is the codec itself under the
// name every codec's view has, which lets a codec that embeds, lists or
// dispatches to it name `CodecTlvEntryView` whatever kind of codec it is.
#[allow(dead_code)]
pub type CodecTlvEntryView<'a> = CodecTlvEntry<'a>;

#[allow(dead_code)]
impl<'a> CodecTlvEntry<'a> {
    /// Fill `_out` with what this value says, as the view of any other codec
    /// does (`CodecTlvEntryView` is this type, so it takes part in the same
    /// call).
    pub fn into_builder(&self, _out: &mut Self) -> Result<(), CodecError> {
        _out.clone_from(self);
        Ok(())
    }
}

// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecTlvEntry<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecTlvEntryOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecTlvEntryOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecTlvEntryOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecTlvEntryOwned` alone is the build's
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
// `let v: CodecTlvEntryOwned = CodecTlvEntryOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecTlvEntryOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub entry_type: u8,
    pub entry_len: u8,
    pub entry_body: S::Bytes<32>,
}

#[allow(dead_code)]
impl<'a> CodecTlvEntry<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecTlvEntryOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecTlvEntryOwned<S>, CodecError> {
        Ok(CodecTlvEntryOwned {
            entry_type: self.entry_type,
            entry_len: self.entry_len,
            entry_body: <S::Bytes<32> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice(self.entry_body)?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecTlvEntryOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTlvEntryOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecTlvEntry<'_> {
        let mut _out = CodecTlvEntry::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecTlvEntry<'o>) {
        _out.entry_type = self.entry_type;
        _out.entry_len = self.entry_len;
        _out.entry_body = ::sce_forge_runtime::codec::SceByteBuf::as_slice(&self.entry_body);
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecTlvEntryOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTlvEntryOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `as_borrowed` followed by
    /// [`CodecTlvEntry::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here. The same
    /// refusals as the projection's: a list longer than the borrowed view
    /// declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> {
        CodecTlvEntry::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecTlvEntry::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
