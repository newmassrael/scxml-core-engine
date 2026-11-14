#![doc = "SCE-MAP: codec_origin_choice:9 :: _forge_body"]
// SCE-MAP: codec_origin_choice:9 :: _forge_body

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

use super::codec_origin_scalar::CodecOriginScalar;

/// A tag `CodecOriginChoice` names no arm for, which its default arm holds: a
/// value inside the 8 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecOriginChoiceUnknownTag(u8);

#[allow(dead_code)]
impl CodecOriginChoiceUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            1..=2 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecOriginChoiceUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(255, &[(1, 2)]);

    fn widened(self) -> u64 {
        u64::from(self.0)
    }

    fn from_admitted(tag: u64, _proof: ::sce_forge_runtime::codec::Admitted) -> Self {
        Self(tag as u8)
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
use super::codec_origin_leaf::CodecOriginLeafOwned;
/// What `CodecOriginChoiceIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecOriginChoiceStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_origin_leaf::CodecOriginLeafStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + super::codec_origin_leaf::CodecOriginLeafStorage> CodecOriginChoiceStorage for S {}

/// What `CodecOriginChoiceIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecOriginChoice` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecOriginChoiceLentStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_origin_leaf::CodecOriginLeafLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + super::codec_origin_leaf::CodecOriginLeafLentStorage> CodecOriginChoiceLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecOriginChoiceStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecOriginChoiceFrom<'f>: CodecOriginChoiceStorage + super::codec_origin_leaf::CodecOriginLeafFrom<'f> {}

impl<'f, S: CodecOriginChoiceStorage + super::codec_origin_leaf::CodecOriginLeafFrom<'f>> CodecOriginChoiceFrom<'f> for S {}

// `CodecOriginChoiceIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecOriginChoice<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecOriginChoiceOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecOriginChoiceIn<S: CodecOriginChoiceStorage> {
    pub tag: u8,
    pub body: CodecOriginChoiceVariantIn<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecOriginChoice<'a> = CodecOriginChoiceIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecOriginChoiceOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecOriginChoiceIn<S>;

impl<S: CodecOriginChoiceStorage> Default for CodecOriginChoiceIn<S> {
    fn default() -> Self {
        Self {
            tag: Default::default(),
            body: Default::default(),
        }
    }
}

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
//
// Variant arms wrap distinct body codecs whose stored forms differ in
// field count and size, so the tagged union is inherently size-disparate.
// The lint's only remedy is boxing the large arm, which adds an
// indirection (and allocation) the generated decode path does not need —
// and which the non-allocating storage profile could not take at all.
// The size spread is the deliberate tagged-union trade-off.
#[allow(clippy::large_enum_variant)]
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecOriginChoiceVariantIn<S: CodecOriginChoiceStorage> {
    CodecOriginLeaf(CodecOriginLeafOwned<S>),
    CodecOriginScalar(CodecOriginScalar),
    Default {
        tag: CodecOriginChoiceUnknownTag,
        body: CodecOriginScalar,
    },
}

#[allow(dead_code)]
pub type CodecOriginChoiceVariant<'a> = CodecOriginChoiceVariantIn<::sce_forge_runtime::codec::Lent<'a>>;

#[allow(dead_code)]
pub type CodecOriginChoiceOwnedVariant<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecOriginChoiceVariantIn<S>;

impl<S: CodecOriginChoiceStorage> Default for CodecOriginChoiceVariantIn<S> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecOriginScalar(CodecOriginScalar::default())
    }
}

#[allow(dead_code)]
impl<S: CodecOriginChoiceStorage> CodecOriginChoiceIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecOriginChoiceFrom<'f> {
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
    ///
    /// A field past what the profile holds is read on and not held, and the
    /// frame is refused with `CapacityExceeded` only once it is read whole and
    /// every refusal of its own has had its turn (docs/adr/0077 §7), so this
    /// node and one that holds more refuse a malformed frame alike. On `Err`,
    /// `_out` holds the fields decoded so far and is not to be read; each of
    /// them holds what its type says.
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecOriginChoiceFrom<'f> {
        // A field past what the profile holds is read on and not held; the
        // frame says so once it is read whole (docs/adr/0077 §7).
        let mut _over = false;
        // Decode fixed prefix (RFC §synth-5-B variant primitive: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let tag = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match tag {
            1u8 => {
                _out.body = CodecOriginChoiceVariantIn::CodecOriginLeaf(CodecOriginLeafOwned::<S>::default());
                if let CodecOriginChoiceVariantIn::CodecOriginLeaf(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecOriginLeafOwned::<S>::decode_into(cursor, _b), &mut _over)?;
                }
            }
            2u8 => {
                _out.body = CodecOriginChoiceVariantIn::CodecOriginScalar(CodecOriginScalar::default());
                if let CodecOriginChoiceVariantIn::CodecOriginScalar(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecOriginScalar::decode_into(cursor, _b), &mut _over)?;
                }
            }
            other => {
                _out.body = CodecOriginChoiceVariantIn::Default {
                    tag: CodecOriginChoiceUnknownTag(other),
                    body: CodecOriginScalar::default(),
                };
                if let CodecOriginChoiceVariantIn::Default { body: _b, .. } = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecOriginScalar::decode_into(cursor, _b), &mut _over)?;
                }
            }
        }
        _out.tag = tag;
        if _over {
            return Err(CodecError::CapacityExceeded);
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 18;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecOriginChoiceWire::encode_source(self, w)
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

/// The arm of `CodecOriginChoice`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecOriginChoice` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecOriginChoiceVariantSource<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> {
    CodecOriginLeaf(&'s A0),
    CodecOriginScalar(&'s A1),
    Default { tag: CodecOriginChoiceUnknownTag, body: &'s D },
}

impl<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> CodecOriginChoiceVariantSource<'s, A0, A1, D> {
    /// The tag of the arm this is: the value the document gives a known
    /// arm, and the tag a default arm holds. It is the tag the encode
    /// writes, or the one a caller hands the decode for a variant whose
    /// wire form carries none.
    pub fn tag(&self) -> u8 {
        match self {
            Self::CodecOriginLeaf(_) => 1,
            Self::CodecOriginScalar(_) => 2,
            Self::Default { tag, .. } => tag.get(),
        }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecOriginChoice` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecOriginChoiceWire::encode_source`]).
pub trait CodecOriginChoiceSource {
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecOriginChoiceVariantSource<'_, impl super::codec_origin_leaf::CodecOriginLeafSource, impl super::codec_origin_scalar::CodecOriginScalarSource, impl super::codec_origin_scalar::CodecOriginScalarSource>;
}

impl<T: CodecOriginChoiceSource + ?Sized> CodecOriginChoiceSource for &T {
    fn body(&self) -> CodecOriginChoiceVariantSource<'_, impl super::codec_origin_leaf::CodecOriginLeafSource, impl super::codec_origin_scalar::CodecOriginScalarSource, impl super::codec_origin_scalar::CodecOriginScalarSource> {
        (**self).body()
    }
}

/// The arm of `CodecOriginChoice`'s variant that CodecOriginChoiceParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecOriginChoicePartsBody<'a> {
    CodecOriginLeaf(&'a super::codec_origin_leaf::CodecOriginLeafParts<'a>),
    CodecOriginScalar(&'a super::codec_origin_scalar::CodecOriginScalarParts),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecOriginChoiceUnknownTag, body: &'a super::codec_origin_scalar::CodecOriginScalarParts },
}

/// A value of `CodecOriginChoice` held in the types a caller already has: a source
/// ([`CodecOriginChoiceSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecOriginChoiceParts<'a> {
    /// The arm of the variant this value holds.
    pub body: CodecOriginChoicePartsBody<'a>,
}

impl<'a> CodecOriginChoiceSource for CodecOriginChoiceParts<'a> {
    fn body(&self) -> CodecOriginChoiceVariantSource<'_, impl super::codec_origin_leaf::CodecOriginLeafSource, impl super::codec_origin_scalar::CodecOriginScalarSource, impl super::codec_origin_scalar::CodecOriginScalarSource> {
        match &self.body {
            CodecOriginChoicePartsBody::CodecOriginLeaf(_b) => CodecOriginChoiceVariantSource::CodecOriginLeaf(*_b),
            CodecOriginChoicePartsBody::CodecOriginScalar(_b) => CodecOriginChoiceVariantSource::CodecOriginScalar(*_b),
            CodecOriginChoicePartsBody::Default { tag, body } => CodecOriginChoiceVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<S: CodecOriginChoiceStorage> CodecOriginChoiceSource for CodecOriginChoiceOwned<S> {
    fn body(&self) -> CodecOriginChoiceVariantSource<'_, impl super::codec_origin_leaf::CodecOriginLeafSource, impl super::codec_origin_scalar::CodecOriginScalarSource, impl super::codec_origin_scalar::CodecOriginScalarSource> {
        match &self.body {
            CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b) => CodecOriginChoiceVariantSource::CodecOriginLeaf(_b),
            CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b) => CodecOriginChoiceVariantSource::CodecOriginScalar(_b),
            CodecOriginChoiceOwnedVariant::Default { tag, body } => CodecOriginChoiceVariantSource::Default { tag: *tag, body },
        }
    }
}

/// The codec `CodecOriginChoice` itself: what writes a value read through
/// [`CodecOriginChoiceSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecOriginChoiceWire;

#[allow(dead_code)]
impl CodecOriginChoiceWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecOriginChoice::encode` writes for the value it holds, read through
    /// [`CodecOriginChoiceSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecOriginChoiceSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The tag of the arm `src` holds ([`CodecOriginChoiceVariantSource::tag`]).
    pub fn tag_of<S: CodecOriginChoiceSource + ?Sized>(src: &S) -> u8 {
        src.body().tag()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecOriginChoiceSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _d_tag: u8 = src.body().tag();
        w.write_u8(_d_tag)?;
        match src.body() {
            CodecOriginChoiceVariantSource::CodecOriginLeaf(_b) => super::codec_origin_leaf::CodecOriginLeafWire::write_source(_b, w)?,
            CodecOriginChoiceVariantSource::CodecOriginScalar(_b) => super::codec_origin_scalar::CodecOriginScalarWire::write_source(_b, w)?,
            CodecOriginChoiceVariantSource::Default { body: _b, .. } => super::codec_origin_scalar::CodecOriginScalarWire::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecOriginChoiceSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecOriginChoiceSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecOriginChoiceWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecOriginChoiceWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecOriginChoiceSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecOriginChoiceSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecOriginChoice<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecOriginChoiceOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecOriginChoiceOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecOriginChoiceOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecOriginChoiceOwned` alone is the build's
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
// `let v: CodecOriginChoiceOwned = CodecOriginChoiceOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a, D> CodecOriginChoiceVariantIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecOriginChoiceLentStorage {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: CodecOriginChoiceFrom<'a>>(self) -> Result<CodecOriginChoiceOwnedVariant<S>, CodecError> {
        Ok(match self {
            Self::CodecOriginLeaf(_b) => CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b.try_into_owned_in::<S>()?),
            Self::CodecOriginScalar(_b) => CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b),
            Self::Default { tag, body } => CodecOriginChoiceOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecOriginChoiceOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecOriginChoiceStorage> CodecOriginChoiceOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn as_borrowed(&self) -> CodecOriginChoiceVariantIn<::sce_forge_runtime::codec::Lent<'_, S>> {
        let mut _out = Default::default();
        self.as_borrowed_into(&mut _out);
        _out
    }

    /// [`Self::as_borrowed`] into a destination the caller
    /// owns: the active arm is installed in `_out` holding its body's default
    /// and that body is projected where it sits, so the arm body is never
    /// returned up through the variant wrapper.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through. A caller on a path where that shows projects into a
    /// destination it owns. On `Err`, `_out` holds what was projected so far.
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecOriginChoiceVariantIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        match self {
            CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b) => {
                *_out = CodecOriginChoiceVariantIn::CodecOriginLeaf(Default::default());
                if let CodecOriginChoiceVariantIn::CodecOriginLeaf(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b) => {
                *_out = CodecOriginChoiceVariantIn::CodecOriginScalar(Default::default());
                if let CodecOriginChoiceVariantIn::CodecOriginScalar(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecOriginChoiceOwnedVariant::Default { tag, body } => {
                *_out = CodecOriginChoiceVariantIn::Default { tag: *tag, body: Default::default() };
                if let CodecOriginChoiceVariantIn::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        
    }
}

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecOriginChoice<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecOriginChoiceIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecOriginChoiceLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecOriginChoiceOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecOriginChoiceFrom<'a>>(self) -> Result<CodecOriginChoiceOwned<S>, CodecError> {
        Ok(CodecOriginChoiceOwned {
            tag: self.tag,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecOriginChoiceOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecOriginChoiceStorage> CodecOriginChoiceOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecOriginChoiceIn<::sce_forge_runtime::codec::Lent<'_, S>> {
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecOriginChoiceIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        _out.tag = self.tag;
        self.body.as_borrowed_into(&mut _out.body);
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: for<'x> CodecOriginChoiceFrom<'x>>(&self) -> Result<CodecOriginChoiceOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}

/// What the origin projection of `CodecOriginChoice` asks of a storage profile
/// beside CodecOriginChoiceStorage: that it is an origin profile, and
/// that each byte string and text of `CodecOriginChoice` and of the codecs it
/// holds is built from the origin by the container the profile holds it in.
#[allow(dead_code)]
pub trait CodecOriginChoiceOrigin: CodecOriginChoiceStorage + ::sce_forge_runtime::codec::OriginStorage + super::codec_origin_leaf::CodecOriginLeafOrigin {}

impl<S: CodecOriginChoiceStorage + ::sce_forge_runtime::codec::OriginStorage + super::codec_origin_leaf::CodecOriginLeafOrigin> CodecOriginChoiceOrigin for S {}

#[allow(dead_code)]
impl<'a, D> CodecOriginChoiceVariantIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecOriginChoiceLentStorage {
    /// The projection above, with the profile's byte and text containers made
    /// from `origin` — what the decoded bytes were taken from.
    pub fn try_into_owned_in_origin<S: CodecOriginChoiceOrigin>(self, origin: &<S as ::sce_forge_runtime::codec::OriginStorage>::Origin) -> Result<CodecOriginChoiceOwnedVariant<S>, CodecError> {
        Ok(match self {
            Self::CodecOriginLeaf(_b) => CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b.try_into_owned_in_origin::<S>(origin)?),
            Self::CodecOriginScalar(_b) => CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b),
            Self::Default { tag, body } => CodecOriginChoiceOwnedVariant::Default { tag, body },
        })
    }
}

#[allow(dead_code)]
impl<'a, D> CodecOriginChoiceIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecOriginChoiceLentStorage {
    /// Project this borrowed view into an owned [`CodecOriginChoiceOwned`]
    /// whose byte and text containers are each built from the slice AND
    /// `origin`, the buffer the view was decoded from, by the
    /// container the profile holds that field in. A container that shares
    /// that buffer refers back to it instead of copying out of it; what the
    /// origin is, and what to do with a slice that does not come from it, is
    /// the profile's to say, field by field.
    ///
    /// `try_into_owned_in` stays the copying projection. Every codec this one
    /// embeds, repeats or dispatches to must have been generated with
    /// `--owned-origin` as well.
    pub fn try_into_owned_in_origin<S: CodecOriginChoiceOrigin>(self, origin: &<S as ::sce_forge_runtime::codec::OriginStorage>::Origin) -> Result<CodecOriginChoiceOwned<S>, CodecError> {
        Ok(CodecOriginChoiceOwned {
            tag: self.tag,
            body: self.body.try_into_owned_in_origin::<S>(origin)?,
        })
    }
}
