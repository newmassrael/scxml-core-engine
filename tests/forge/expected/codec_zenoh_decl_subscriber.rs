#![doc = "SCE-MAP: codec_zenoh_decl_subscriber:41 :: _forge_body"]
// SCE-MAP: codec_zenoh_decl_subscriber:41 :: _forge_body

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

use super::codec_zenoh_wireexpr::CodecZenohWireexprView;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
use super::codec_zenoh_wireexpr::CodecZenohWireexprOwned;
/// What `CodecZenohDeclSubscriberIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohDeclSubscriberStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_wireexpr::CodecZenohWireexprStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_wireexpr::CodecZenohWireexprStorage> CodecZenohDeclSubscriberStorage for S {}

/// What `CodecZenohDeclSubscriberIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohDeclSubscriber` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohDeclSubscriberLentStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_wireexpr::CodecZenohWireexprLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_wireexpr::CodecZenohWireexprLentStorage> CodecZenohDeclSubscriberLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohDeclSubscriberStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohDeclSubscriberFrom<'f>: CodecZenohDeclSubscriberStorage + super::codec_zenoh_wireexpr::CodecZenohWireexprFrom<'f> {}

impl<'f, S: CodecZenohDeclSubscriberStorage + super::codec_zenoh_wireexpr::CodecZenohWireexprFrom<'f>> CodecZenohDeclSubscriberFrom<'f> for S {}

// `CodecZenohDeclSubscriberIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohDeclSubscriber<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohDeclSubscriberOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohDeclSubscriberIn<S: CodecZenohDeclSubscriberStorage> {
    pub id: u32,
    pub wireexpr: CodecZenohWireexprOwned<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohDeclSubscriber<'a> = CodecZenohDeclSubscriberIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohDeclSubscriberOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohDeclSubscriberIn<S>;

impl<S: CodecZenohDeclSubscriberStorage> Default for CodecZenohDeclSubscriberIn<S> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            wireexpr: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohDeclSubscriberStorage> CodecZenohDeclSubscriberIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>, n: u8) -> Result<Self, CodecError> where S: CodecZenohDeclSubscriberFrom<'f> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, n)?;
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
    ///
    /// A field past what the profile holds is read on and not held, and the
    /// frame is refused with `CapacityExceeded` only once it is read whole and
    /// every refusal of its own has had its turn (docs/adr/0077 §7), so this
    /// node and one that holds more refuse a malformed frame alike. On `Err`,
    /// `_out` holds the fields decoded so far and is not to be read; each of
    /// them holds what its type says.
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, n: u8) -> Result<(), CodecError> where S: CodecZenohDeclSubscriberFrom<'f> {
        // A field past what the profile holds is read on and not held; the
        // frame says so once it is read whole (docs/adr/0077 §7).
        let mut _over = false;
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = n;
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
        let id = cursor.read_vle_u32()?;
        ::sce_forge_runtime::codec::note_capacity(CodecZenohWireexprOwned::<S>::decode_into(cursor, &mut _out.wireexpr, n), &mut _over)?;
        _out.id = id;
        if _over {
            return Err(CodecError::CapacityExceeded);
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 65558;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohDeclSubscriberWire::encode_source(self, w)
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
/// What a value of `CodecZenohDeclSubscriber` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohDeclSubscriberWire::encode_source`]).
pub trait CodecZenohDeclSubscriberSource {
    /// The `id` field.
    fn id(&self) -> u32;
    /// The `wireexpr` field.
    fn wireexpr(&self) -> &impl super::codec_zenoh_wireexpr::CodecZenohWireexprSource;
}

impl<T: CodecZenohDeclSubscriberSource + ?Sized> CodecZenohDeclSubscriberSource for &T {
    fn id(&self) -> u32 {
        (**self).id()
    }
    fn wireexpr(&self) -> &impl super::codec_zenoh_wireexpr::CodecZenohWireexprSource {
        (**self).wireexpr()
    }
}

/// A value of `CodecZenohDeclSubscriber` held in the types a caller already has: a source
/// ([`CodecZenohDeclSubscriberSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohDeclSubscriberParts<'a> {
    /// The `id` field.
    pub id: u32,
    /// The `wireexpr` field.
    pub wireexpr: &'a super::codec_zenoh_wireexpr::CodecZenohWireexprParts<'a>,
}

impl<'a> CodecZenohDeclSubscriberSource for CodecZenohDeclSubscriberParts<'a> {
    fn id(&self) -> u32 {
        self.id
    }
    fn wireexpr(&self) -> &impl super::codec_zenoh_wireexpr::CodecZenohWireexprSource {
        self.wireexpr
    }
}

impl<S: CodecZenohDeclSubscriberStorage> CodecZenohDeclSubscriberSource for CodecZenohDeclSubscriberOwned<S> {
    fn id(&self) -> u32 {
        self.id
    }
    fn wireexpr(&self) -> &impl super::codec_zenoh_wireexpr::CodecZenohWireexprSource {
        &self.wireexpr
    }
}

impl<'a> CodecZenohDeclSubscriberSource for CodecZenohDeclSubscriberView<'a> {
    fn id(&self) -> u32 {
        self.id
    }
    fn wireexpr(&self) -> &impl super::codec_zenoh_wireexpr::CodecZenohWireexprSource {
        &self.wireexpr
    }
}

/// The codec `CodecZenohDeclSubscriber` itself: what writes a value read through
/// [`CodecZenohDeclSubscriberSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohDeclSubscriberWire;

#[allow(dead_code)]
impl CodecZenohDeclSubscriberWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohDeclSubscriber::encode` writes for the value it holds, read through
    /// [`CodecZenohDeclSubscriberSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohDeclSubscriberSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The flag-input `N` that `src` says: the input `N` that the body `wireexpr` says.
    /// A parent hands the decode this bit and derives it from here for the
    /// encode.
    pub fn input_n<S: CodecZenohDeclSubscriberSource + ?Sized>(src: &S) -> bool {
        super::codec_zenoh_wireexpr::CodecZenohWireexprWire::input_n(CodecZenohDeclSubscriberSource::wireexpr(src))
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohDeclSubscriberSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        w.write_vle_u32(src.id())?;
        super::codec_zenoh_wireexpr::CodecZenohWireexprWire::write_source(src.wireexpr(), w)?;
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohDeclSubscriberSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohDeclSubscriberSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohDeclSubscriberWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohDeclSubscriberWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohDeclSubscriberSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohDeclSubscriberSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecZenohDeclSubscriberView` is what a decode of `CodecZenohDeclSubscriber` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecZenohDeclSubscriber` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecZenohDeclSubscriber::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohDeclSubscriberView<'a> {
    pub id: u32,
    pub wireexpr: CodecZenohWireexprView<'a>,
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclSubscriberView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecZenohDeclSubscriber::decode`], held as a view. On success the
    /// cursor advances past the consumed bytes.
    pub fn decode(cursor: &mut SceCursor<'a>, n: u8) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, n)?;
        Ok(_out)
    }

    /// [`Self::decode`] into a destination the caller owns, so a view that
    /// embeds or dispatches to this one is filled where it will live. On
    /// `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self, n: u8) -> Result<(), CodecError> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = n;
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
        let id = cursor.read_vle_u32()?;
        CodecZenohWireexprView::decode_into(cursor, &mut _out.wireexpr, n)?;
        _out.id = id;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecZenohDeclSubscriber`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder<D>(&self, _out: &mut CodecZenohDeclSubscriberIn<::sce_forge_runtime::codec::Lent<'a, D>>) -> Result<(), CodecError> where D: CodecZenohDeclSubscriberLentStorage {
        _out.id = self.id;
        self.wireexpr.into_builder(&mut _out.wireexpr)?;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecZenohDeclSubscriber::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 65558;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecZenohDeclSubscriber::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecZenohDeclSubscriberWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}

// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohDeclSubscriber<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohDeclSubscriberOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohDeclSubscriberOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohDeclSubscriberOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohDeclSubscriberOwned` alone is the build's
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
// `let v: CodecZenohDeclSubscriberOwned = CodecZenohDeclSubscriberOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohDeclSubscriber<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohDeclSubscriberIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohDeclSubscriberLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohDeclSubscriberOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohDeclSubscriberFrom<'a>>(self) -> Result<CodecZenohDeclSubscriberOwned<S>, CodecError> {
        Ok(CodecZenohDeclSubscriberOwned {
            id: self.id,
            wireexpr: self.wireexpr.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclSubscriberOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohDeclSubscriberStorage> CodecZenohDeclSubscriberOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohDeclSubscriberIn<::sce_forge_runtime::codec::Lent<'_, S>> {
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohDeclSubscriberIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        _out.id = self.id;
        self.wireexpr.as_borrowed_into(&mut _out.wireexpr);
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: for<'x> CodecZenohDeclSubscriberFrom<'x>>(&self) -> Result<CodecZenohDeclSubscriberOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}
