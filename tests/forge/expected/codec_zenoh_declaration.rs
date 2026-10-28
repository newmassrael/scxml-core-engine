#![doc = "SCE-MAP: codec_zenoh_declaration:54 :: _forge_body"]
// SCE-MAP: codec_zenoh_declaration:54 :: _forge_body

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

use super::codec_zenoh_decl_kexpr::CodecZenohDeclKexpr;
use super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexpr;
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriber;
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriber;
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryable;
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryable;
use super::codec_zenoh_decl_token::CodecZenohDeclToken;
use super::codec_zenoh_undecl_token::CodecZenohUndeclToken;
use super::codec_zenoh_decl_final::CodecZenohDeclFinal;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohDeclarationVariant<'a> {
    CodecZenohDeclKexpr(CodecZenohDeclKexpr<'a>),
    CodecZenohUndeclKexpr(CodecZenohUndeclKexpr),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriber<'a>),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber<'a>),
    CodecZenohDeclQueryable(CodecZenohDeclQueryable<'a>),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryable<'a>),
    CodecZenohDeclToken(CodecZenohDeclToken<'a>),
    CodecZenohUndeclToken(CodecZenohUndeclToken<'a>),
    CodecZenohDeclFinal(CodecZenohDeclFinal),
    Default {
        tag: u8,
        body: CodecZenohDeclFinal,
    },
}

impl<'a> Default for CodecZenohDeclarationVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohDeclFinal(CodecZenohDeclFinal::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohDeclaration<'a> {
    pub header: u8,
    pub body: CodecZenohDeclarationVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclaration<'a> {
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
        let header = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match header & 0x1F {
            0u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohDeclKexpr(CodecZenohDeclKexpr::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclKexpr(_b) = &mut _out.body {
                    CodecZenohDeclKexpr::decode_into(cursor, _b, (header >> 5) & 0x1)?;
                }
            }
            1u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(CodecZenohUndeclKexpr::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(_b) = &mut _out.body {
                    CodecZenohUndeclKexpr::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(CodecZenohDeclSubscriber::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(_b) = &mut _out.body {
                    CodecZenohDeclSubscriber::decode_into(cursor, _b, (header >> 5) & 0x1)?;
                }
            }
            3u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(_b) = &mut _out.body {
                    CodecZenohUndeclSubscriber::decode_into(cursor, _b, (header >> 7) & 0x1)?;
                }
            }
            4u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohDeclQueryable(CodecZenohDeclQueryable::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclQueryable(_b) = &mut _out.body {
                    CodecZenohDeclQueryable::decode_into(cursor, _b, (header >> 5) & 0x1, (header >> 7) & 0x1)?;
                }
            }
            5u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(CodecZenohUndeclQueryable::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(_b) = &mut _out.body {
                    CodecZenohUndeclQueryable::decode_into(cursor, _b, (header >> 7) & 0x1)?;
                }
            }
            6u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohDeclToken(CodecZenohDeclToken::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclToken(_b) = &mut _out.body {
                    CodecZenohDeclToken::decode_into(cursor, _b, (header >> 5) & 0x1)?;
                }
            }
            7u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohUndeclToken(CodecZenohUndeclToken::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclToken(_b) = &mut _out.body {
                    CodecZenohUndeclToken::decode_into(cursor, _b, (header >> 7) & 0x1)?;
                }
            }
            26u8 => {
                _out.body = CodecZenohDeclarationVariant::CodecZenohDeclFinal(CodecZenohDeclFinal::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclFinal(_b) = &mut _out.body {
                    CodecZenohDeclFinal::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohDeclarationVariant::Default {
                    tag: other,
                    body: CodecZenohDeclFinal::default(),
                };
                if let CodecZenohDeclarationVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohDeclFinal::decode_into(cursor, _b)?;
                }
            }
        }
        _out.header = header;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn set_mid(&mut self, v: u8) {
        self.header = (self.header & !0x1F) | (v & 0x1F);
    }

    pub fn n(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_n(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn m(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_m(&mut self, v: bool) {
        if v {
            self.header |= 0x40;
        } else {
            self.header &= !0x40;
        }
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_z(&mut self, v: bool) {
        if v {
            self.header |= 0x80;
        } else {
            self.header &= !0x80;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 274;

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

/// The arm of `CodecZenohDeclaration`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohDeclaration` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohDeclarationVariantSource<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, A3: ?Sized, A4: ?Sized, A5: ?Sized, A6: ?Sized, A7: ?Sized, A8: ?Sized, D: ?Sized> {
    CodecZenohDeclKexpr(&'s A0),
    CodecZenohUndeclKexpr(&'s A1),
    CodecZenohDeclSubscriber(&'s A2),
    CodecZenohUndeclSubscriber(&'s A3),
    CodecZenohDeclQueryable(&'s A4),
    CodecZenohUndeclQueryable(&'s A5),
    CodecZenohDeclToken(&'s A6),
    CodecZenohUndeclToken(&'s A7),
    CodecZenohDeclFinal(&'s A8),
    Default { tag: u8, body: &'s D },
}

/// Flags of `CodecZenohDeclaration` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecZenohDeclaration`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecZenohDeclarationOverrides {
    /// The `N` flag of `header`, when a parent sets it.
    pub header_n: Option<bool>,
    /// The `M` flag of `header`, when a parent sets it.
    pub header_m: Option<bool>,
    /// The `Z` flag of `header`, when a parent sets it.
    pub header_z: Option<bool>,
}

#[allow(dead_code)]
impl CodecZenohDeclarationOverrides {
    /// Sets `N` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_n(value: bool) -> Self {
        Self { header_n: Some(value), ..Self::default() }
    }

    /// Sets `M` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_m(value: bool) -> Self {
        Self { header_m: Some(value), ..Self::default() }
    }

    /// Sets `Z` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_z(value: bool) -> Self {
        Self { header_z: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohDeclaration` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohDeclaration`
/// computes those from what is ([`CodecZenohDeclaration::encode_source`]).
pub trait CodecZenohDeclarationSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohDeclarationVariantSource<'_, impl super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprSource, impl super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprSource, impl super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberSource, impl super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberSource, impl super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableSource, impl super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableSource, impl super::codec_zenoh_decl_token::CodecZenohDeclTokenSource, impl super::codec_zenoh_undecl_token::CodecZenohUndeclTokenSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource>;
}

impl<T: CodecZenohDeclarationSource + ?Sized> CodecZenohDeclarationSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn body(&self) -> CodecZenohDeclarationVariantSource<'_, impl super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprSource, impl super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprSource, impl super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberSource, impl super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberSource, impl super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableSource, impl super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableSource, impl super::codec_zenoh_decl_token::CodecZenohDeclTokenSource, impl super::codec_zenoh_undecl_token::CodecZenohUndeclTokenSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohDeclaration`'s variant that CodecZenohDeclarationParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohDeclarationPartsBody<'a> {
    CodecZenohDeclKexpr(&'a super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprParts<'a>),
    CodecZenohUndeclKexpr(&'a super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprParts),
    CodecZenohDeclSubscriber(&'a super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberParts<'a>),
    CodecZenohUndeclSubscriber(&'a super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberParts<'a>),
    CodecZenohDeclQueryable(&'a super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableParts<'a>),
    CodecZenohUndeclQueryable(&'a super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableParts<'a>),
    CodecZenohDeclToken(&'a super::codec_zenoh_decl_token::CodecZenohDeclTokenParts<'a>),
    CodecZenohUndeclToken(&'a super::codec_zenoh_undecl_token::CodecZenohUndeclTokenParts<'a>),
    CodecZenohDeclFinal(&'a super::codec_zenoh_decl_final::CodecZenohDeclFinalParts),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: u8, body: &'a super::codec_zenoh_decl_final::CodecZenohDeclFinalParts },
}

/// A value of `CodecZenohDeclaration` held in the types a caller already has: a source
/// ([`CodecZenohDeclarationSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohDeclarationParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The arm of the variant this value holds.
    pub body: CodecZenohDeclarationPartsBody<'a>,
}

impl<'a> CodecZenohDeclarationSource for CodecZenohDeclarationParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohDeclarationVariantSource<'_, impl super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprSource, impl super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprSource, impl super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberSource, impl super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberSource, impl super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableSource, impl super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableSource, impl super::codec_zenoh_decl_token::CodecZenohDeclTokenSource, impl super::codec_zenoh_undecl_token::CodecZenohUndeclTokenSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource> {
        match &self.body {
            CodecZenohDeclarationPartsBody::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclKexpr(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohDeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclToken(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohUndeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(*_b),
            CodecZenohDeclarationPartsBody::CodecZenohDeclFinal(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclFinal(*_b),
            CodecZenohDeclarationPartsBody::Default { tag, body } => CodecZenohDeclarationVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<'a> CodecZenohDeclarationSource for CodecZenohDeclaration<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohDeclarationVariantSource<'_, impl super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprSource, impl super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprSource, impl super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberSource, impl super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberSource, impl super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableSource, impl super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableSource, impl super::codec_zenoh_decl_token::CodecZenohDeclTokenSource, impl super::codec_zenoh_undecl_token::CodecZenohUndeclTokenSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource> {
        match &self.body {
            CodecZenohDeclarationVariant::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(_b),
            CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclKexpr(_b),
            CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(_b),
            CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(_b),
            CodecZenohDeclarationVariant::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_b),
            CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(_b),
            CodecZenohDeclarationVariant::CodecZenohDeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclToken(_b),
            CodecZenohDeclarationVariant::CodecZenohUndeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(_b),
            CodecZenohDeclarationVariant::CodecZenohDeclFinal(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclFinal(_b),
            CodecZenohDeclarationVariant::Default { tag, body } => CodecZenohDeclarationVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationSource for CodecZenohDeclarationOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohDeclarationVariantSource<'_, impl super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprSource, impl super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprSource, impl super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberSource, impl super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberSource, impl super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableSource, impl super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableSource, impl super::codec_zenoh_decl_token::CodecZenohDeclTokenSource, impl super::codec_zenoh_undecl_token::CodecZenohUndeclTokenSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource> {
        match &self.body {
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclKexpr(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclToken(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(_b),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclFinal(_b),
            CodecZenohDeclarationOwnedVariant::Default { tag, body } => CodecZenohDeclarationVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclaration<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohDeclarationSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecZenohDeclarationSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new())?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohDeclarationSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source_in(src, w, &CodecZenohDeclarationOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecZenohDeclarationSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecZenohDeclarationOverrides) -> Result<(), CodecError> {
        let _d_header_mid: u8 = match src.body() {
            CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(_) => 0,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclKexpr(_) => 1,
            CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(_) => 2,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(_) => 3,
            CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_) => 4,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(_) => 5,
            CodecZenohDeclarationVariantSource::CodecZenohDeclToken(_) => 6,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(_) => 7,
            CodecZenohDeclarationVariantSource::CodecZenohDeclFinal(_) => 26,
            CodecZenohDeclarationVariantSource::Default { tag, .. } => tag,
        };
        if _d_header_mid > 0x1F {
            return Err(CodecError::LengthOutOfRange);
        }
        let mut _c_header: u8 = (src.header() & !0x1F) | _d_header_mid;
        if let Some(_o) = over.header_n {
            _c_header = (_c_header & !0x20) | (u8::from(_o) << 5);
        }
        if let Some(_o) = over.header_m {
            _c_header = (_c_header & !0x40) | (u8::from(_o) << 6);
        }
        if let Some(_o) = over.header_z {
            _c_header = (_c_header & !0x80) | (u8::from(_o) << 7);
        }
        w.write_u8(_c_header)?;
        match src.body() {
            CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(_b) => CodecZenohDeclKexpr::write_source(_b, w, (_c_header >> 5) & 0x1)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclKexpr(_b) => CodecZenohUndeclKexpr::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(_b) => CodecZenohDeclSubscriber::write_source(_b, w, (_c_header >> 5) & 0x1)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(_b) => CodecZenohUndeclSubscriber::write_source(_b, w, (_c_header >> 7) & 0x1)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_b) => CodecZenohDeclQueryable::write_source(_b, w, (_c_header >> 5) & 0x1, (_c_header >> 7) & 0x1)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(_b) => CodecZenohUndeclQueryable::write_source(_b, w, (_c_header >> 7) & 0x1)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclToken(_b) => CodecZenohDeclToken::write_source(_b, w, (_c_header >> 5) & 0x1)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(_b) => CodecZenohUndeclToken::write_source(_b, w, (_c_header >> 7) & 0x1)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclFinal(_b) => CodecZenohDeclFinal::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::Default { body: _b, .. } => CodecZenohDeclFinal::write_source(_b, w)?,
        }
        Ok(())
    }
}

impl<'a, S: CodecZenohDeclarationSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohDeclaration<'a> {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclaration<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecZenohDeclarationSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohDeclarationSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohDeclaration<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohDeclarationOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohDeclarationOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohDeclarationOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohDeclarationOwned` alone is the build's
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
// `let v: CodecZenohDeclarationOwned = CodecZenohDeclarationOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprOwned;
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberOwned;
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberOwned;
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableOwned;
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableOwned;
use super::codec_zenoh_decl_token::CodecZenohDeclTokenOwned;
use super::codec_zenoh_undecl_token::CodecZenohUndeclTokenOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohDeclarationOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub header: u8,
    pub body: CodecZenohDeclarationOwnedVariant<S>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn n(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn m(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }
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
pub enum CodecZenohDeclarationOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecZenohDeclKexpr(CodecZenohDeclKexprOwned<S>),
    CodecZenohUndeclKexpr(CodecZenohUndeclKexpr),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriberOwned<S>),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriberOwned<S>),
    CodecZenohDeclQueryable(CodecZenohDeclQueryableOwned<S>),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryableOwned<S>),
    CodecZenohDeclToken(CodecZenohDeclTokenOwned<S>),
    CodecZenohUndeclToken(CodecZenohUndeclTokenOwned<S>),
    CodecZenohDeclFinal(CodecZenohDeclFinal),
    Default {
        tag: u8,
        body: CodecZenohDeclFinal,
    },
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclarationVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohDeclarationOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecZenohDeclarationVariant::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b),
            CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohDeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohDeclFinal(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b),
            CodecZenohDeclarationVariant::Default { tag, body } => CodecZenohDeclarationOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclarationOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn as_borrowed(&self) -> CodecZenohDeclarationVariant<'_> {
        let mut _out = CodecZenohDeclarationVariant::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohDeclarationVariant<'o>) {
        match self {
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohDeclKexpr(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclKexpr(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohDeclQueryable(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclQueryable(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohDeclToken(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclToken(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohUndeclToken(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohUndeclToken(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b) => {
                *_out = CodecZenohDeclarationVariant::CodecZenohDeclFinal(Default::default());
                if let CodecZenohDeclarationVariant::CodecZenohDeclFinal(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohDeclarationOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohDeclarationVariant::Default { tag: *tag, body: Default::default() };
                if let CodecZenohDeclarationVariant::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclaration<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohDeclarationOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohDeclarationOwned<S>, CodecError> {
        Ok(CodecZenohDeclarationOwned {
            header: self.header,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclarationOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohDeclaration<'_> {
        let mut _out = CodecZenohDeclaration::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohDeclaration<'o>) {
        _out.header = self.header;
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
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohDeclarationOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `as_borrowed` followed by
    /// [`CodecZenohDeclaration::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here. The same
    /// refusals as the projection's: a list longer than the borrowed view
    /// declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> {
        CodecZenohDeclaration::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecZenohDeclaration::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
