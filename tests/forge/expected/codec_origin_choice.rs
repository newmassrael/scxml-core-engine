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

use super::codec_origin_leaf::CodecOriginLeaf;
use super::codec_origin_scalar::CodecOriginScalar;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecOriginChoiceVariant<'a> {
    CodecOriginLeaf(CodecOriginLeaf<'a>),
    CodecOriginScalar(CodecOriginScalar),
    Default {
        tag: u8,
        body: CodecOriginScalar,
    },
}

impl<'a> Default for CodecOriginChoiceVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecOriginScalar(CodecOriginScalar::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecOriginChoice<'a> {
    pub tag: u8,
    pub body: CodecOriginChoiceVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecOriginChoice<'a> {
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
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self) -> Result<(), CodecError> {
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
                _out.body = CodecOriginChoiceVariant::CodecOriginLeaf(CodecOriginLeaf::default());
                if let CodecOriginChoiceVariant::CodecOriginLeaf(_b) = &mut _out.body {
                    CodecOriginLeaf::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecOriginChoiceVariant::CodecOriginScalar(CodecOriginScalar::default());
                if let CodecOriginChoiceVariant::CodecOriginScalar(_b) = &mut _out.body {
                    CodecOriginScalar::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecOriginChoiceVariant::Default {
                    tag: other,
                    body: CodecOriginScalar::default(),
                };
                if let CodecOriginChoiceVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecOriginScalar::decode_into(cursor, _b)?;
                }
            }
        }
        _out.tag = tag;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 18;

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
    Default { tag: u8, body: &'s D },
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecOriginChoice` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecOriginChoice`
/// computes those from what is ([`CodecOriginChoice::encode_source`]).
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
    Default { tag: u8, body: &'a super::codec_origin_scalar::CodecOriginScalarParts },
}

/// A value of `CodecOriginChoice` held in the types a caller already has: a source
/// ([`CodecOriginChoiceSource`]) whose lists are slices and whose bodies are references.
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

impl<'a> CodecOriginChoiceSource for CodecOriginChoice<'a> {
    fn body(&self) -> CodecOriginChoiceVariantSource<'_, impl super::codec_origin_leaf::CodecOriginLeafSource, impl super::codec_origin_scalar::CodecOriginScalarSource, impl super::codec_origin_scalar::CodecOriginScalarSource> {
        match &self.body {
            CodecOriginChoiceVariant::CodecOriginLeaf(_b) => CodecOriginChoiceVariantSource::CodecOriginLeaf(_b),
            CodecOriginChoiceVariant::CodecOriginScalar(_b) => CodecOriginChoiceVariantSource::CodecOriginScalar(_b),
            CodecOriginChoiceVariant::Default { tag, body } => CodecOriginChoiceVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecOriginChoiceSource for CodecOriginChoiceOwned<S> {
    fn body(&self) -> CodecOriginChoiceVariantSource<'_, impl super::codec_origin_leaf::CodecOriginLeafSource, impl super::codec_origin_scalar::CodecOriginScalarSource, impl super::codec_origin_scalar::CodecOriginScalarSource> {
        match &self.body {
            CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b) => CodecOriginChoiceVariantSource::CodecOriginLeaf(_b),
            CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b) => CodecOriginChoiceVariantSource::CodecOriginScalar(_b),
            CodecOriginChoiceOwnedVariant::Default { tag, body } => CodecOriginChoiceVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecOriginChoice<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecOriginChoiceSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecOriginChoiceSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new())?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecOriginChoiceSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        let _d_tag: u8 = match src.body() {
            CodecOriginChoiceVariantSource::CodecOriginLeaf(_) => 1,
            CodecOriginChoiceVariantSource::CodecOriginScalar(_) => 2,
            CodecOriginChoiceVariantSource::Default { tag, .. } => tag,
        };
        w.write_u8(_d_tag)?;
        match src.body() {
            CodecOriginChoiceVariantSource::CodecOriginLeaf(_b) => CodecOriginLeaf::write_source(_b, w)?,
            CodecOriginChoiceVariantSource::CodecOriginScalar(_b) => CodecOriginScalar::write_source(_b, w)?,
            CodecOriginChoiceVariantSource::Default { body: _b, .. } => CodecOriginScalar::write_source(_b, w)?,
        }
        Ok(())
    }
}

impl<'a, S: CodecOriginChoiceSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecOriginChoice<'a> {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecOriginChoice<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecOriginChoiceSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecOriginChoiceSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        ::sce_forge_runtime::codec::write_window::<Self, S>(src, window)
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
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_origin_leaf::CodecOriginLeafOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecOriginChoiceOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub tag: u8,
    pub body: CodecOriginChoiceOwnedVariant<S>,
}

// Variant arms wrap distinct body codecs whose owned mirrors differ in
// field count and size, so the tagged union is inherently size-disparate.
// The lint's only remedy is boxing the large arm, which adds an
// indirection (and allocation) the generated decode path does not need —
// and which the non-allocating storage profile could not take at all.
// The size spread is the deliberate tagged-union trade-off.
#[allow(clippy::large_enum_variant)]
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecOriginChoiceOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecOriginLeaf(CodecOriginLeafOwned<S>),
    CodecOriginScalar(CodecOriginScalar),
    Default {
        tag: u8,
        body: CodecOriginScalar,
    },
}

#[allow(dead_code)]
impl<'a> CodecOriginChoiceVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecOriginChoiceOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecOriginChoiceVariant::CodecOriginLeaf(_b) => CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b.try_into_owned_in::<S>()?),
            CodecOriginChoiceVariant::CodecOriginScalar(_b) => CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b),
            CodecOriginChoiceVariant::Default { tag, body } => CodecOriginChoiceOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecOriginChoiceOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecOriginChoiceOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn as_borrowed(&self) -> CodecOriginChoiceVariant<'_> {
        let mut _out = CodecOriginChoiceVariant::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecOriginChoiceVariant<'o>) {
        match self {
            CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b) => {
                *_out = CodecOriginChoiceVariant::CodecOriginLeaf(Default::default());
                if let CodecOriginChoiceVariant::CodecOriginLeaf(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b) => {
                *_out = CodecOriginChoiceVariant::CodecOriginScalar(Default::default());
                if let CodecOriginChoiceVariant::CodecOriginScalar(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecOriginChoiceOwnedVariant::Default { tag, body } => {
                *_out = CodecOriginChoiceVariant::Default { tag: *tag, body: Default::default() };
                if let CodecOriginChoiceVariant::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        
    }
}

#[allow(dead_code)]
impl<'a> CodecOriginChoice<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecOriginChoiceOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecOriginChoiceOwned<S>, CodecError> {
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
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecOriginChoiceOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecOriginChoice<'_> {
        let mut _out = CodecOriginChoice::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecOriginChoice<'o>) {
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
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecOriginChoiceOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecOriginChoiceOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `as_borrowed` followed by
    /// [`CodecOriginChoice::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here. The same
    /// refusals as the projection's: a list longer than the borrowed view
    /// declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> {
        CodecOriginChoice::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecOriginChoice::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}

#[allow(dead_code)]
impl<'a> CodecOriginChoiceVariant<'a> {
    /// The projection above, with the profile's byte and text containers made
    /// from `origin` — what the decoded bytes were taken from.
    pub fn try_into_owned_in_origin<S: ::sce_forge_runtime::codec::OriginStorage>(self, origin: &<S as ::sce_forge_runtime::codec::OriginStorage>::Origin) -> Result<CodecOriginChoiceOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecOriginChoiceVariant::CodecOriginLeaf(_b) => CodecOriginChoiceOwnedVariant::CodecOriginLeaf(_b.try_into_owned_in_origin::<S>(origin)?),
            CodecOriginChoiceVariant::CodecOriginScalar(_b) => CodecOriginChoiceOwnedVariant::CodecOriginScalar(_b),
            CodecOriginChoiceVariant::Default { tag, body } => CodecOriginChoiceOwnedVariant::Default { tag, body },
        })
    }
}

#[allow(dead_code)]
impl<'a> CodecOriginChoice<'a> {
    /// Project this borrowed view into an owned [`CodecOriginChoiceOwned`]
    /// whose byte and text containers are made by the profile's
    /// `OriginStorage` from each slice AND `origin`, the buffer the
    /// view was decoded from. A profile that shares that buffer builds
    /// containers that refer back to it instead of copying out of it; what the
    /// origin is, and what to do with a slice that does not come from it, is
    /// the profile's to say.
    ///
    /// `try_into_owned_in` stays the copying projection. Every codec this one
    /// embeds, repeats or dispatches to must have been generated with
    /// `--owned-origin` as well.
    pub fn try_into_owned_in_origin<S: ::sce_forge_runtime::codec::OriginStorage>(self, origin: &<S as ::sce_forge_runtime::codec::OriginStorage>::Origin) -> Result<CodecOriginChoiceOwned<S>, CodecError> {
        Ok(CodecOriginChoiceOwned {
            tag: self.tag,
            body: self.body.try_into_owned_in_origin::<S>(origin)?,
        })
    }
}
