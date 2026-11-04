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

use super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexpr;
use super::codec_zenoh_decl_final::CodecZenohDeclFinal;

use super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprView;
use super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprView;
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberView;
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberView;
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableView;
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableView;
use super::codec_zenoh_decl_token::CodecZenohDeclTokenView;
use super::codec_zenoh_undecl_token::CodecZenohUndeclTokenView;
use super::codec_zenoh_decl_final::CodecZenohDeclFinalView;

/// A tag `CodecZenohDeclaration` names no arm for, which its default arm holds: a
/// value inside the 5 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecZenohDeclarationUnknownTag(u8);

#[allow(dead_code)]
impl CodecZenohDeclarationUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            0..=7 | 26 => None,
            _ if tag > 31 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecZenohDeclarationUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(31, &[(0, 7), (26, 26)]);

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
use super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprOwned;
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberOwned;
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberOwned;
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableOwned;
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableOwned;
use super::codec_zenoh_decl_token::CodecZenohDeclTokenOwned;
use super::codec_zenoh_undecl_token::CodecZenohUndeclTokenOwned;
/// What `CodecZenohDeclarationIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohDeclarationStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprStorage + super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberStorage + super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberStorage + super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableStorage + super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableStorage + super::codec_zenoh_decl_token::CodecZenohDeclTokenStorage + super::codec_zenoh_undecl_token::CodecZenohUndeclTokenStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprStorage + super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberStorage + super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberStorage + super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableStorage + super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableStorage + super::codec_zenoh_decl_token::CodecZenohDeclTokenStorage + super::codec_zenoh_undecl_token::CodecZenohUndeclTokenStorage> CodecZenohDeclarationStorage for S {}

/// What `CodecZenohDeclarationIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohDeclaration` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohDeclarationLentStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprLentStorage + super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberLentStorage + super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberLentStorage + super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableLentStorage + super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableLentStorage + super::codec_zenoh_decl_token::CodecZenohDeclTokenLentStorage + super::codec_zenoh_undecl_token::CodecZenohUndeclTokenLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprLentStorage + super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberLentStorage + super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberLentStorage + super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableLentStorage + super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableLentStorage + super::codec_zenoh_decl_token::CodecZenohDeclTokenLentStorage + super::codec_zenoh_undecl_token::CodecZenohUndeclTokenLentStorage> CodecZenohDeclarationLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohDeclarationStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohDeclarationFrom<'f>: CodecZenohDeclarationStorage + super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprFrom<'f> + super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberFrom<'f> + super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberFrom<'f> + super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableFrom<'f> + super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableFrom<'f> + super::codec_zenoh_decl_token::CodecZenohDeclTokenFrom<'f> + super::codec_zenoh_undecl_token::CodecZenohUndeclTokenFrom<'f> {}

impl<'f, S: CodecZenohDeclarationStorage + super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprFrom<'f> + super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberFrom<'f> + super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberFrom<'f> + super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableFrom<'f> + super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableFrom<'f> + super::codec_zenoh_decl_token::CodecZenohDeclTokenFrom<'f> + super::codec_zenoh_undecl_token::CodecZenohUndeclTokenFrom<'f>> CodecZenohDeclarationFrom<'f> for S {}

// `CodecZenohDeclarationIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohDeclaration<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohDeclarationOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohDeclarationIn<S: CodecZenohDeclarationStorage> {
    pub header: u8,
    pub body: CodecZenohDeclarationVariantIn<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohDeclaration<'a> = CodecZenohDeclarationIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohDeclarationOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohDeclarationIn<S>;

impl<S: CodecZenohDeclarationStorage> Default for CodecZenohDeclarationIn<S> {
    fn default() -> Self {
        Self {
            header: Default::default(),
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
pub enum CodecZenohDeclarationVariantIn<S: CodecZenohDeclarationStorage> {
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
        tag: CodecZenohDeclarationUnknownTag,
        body: CodecZenohDeclFinal,
    },
}

#[allow(dead_code)]
pub type CodecZenohDeclarationVariant<'a> = CodecZenohDeclarationVariantIn<::sce_forge_runtime::codec::Lent<'a>>;

#[allow(dead_code)]
pub type CodecZenohDeclarationOwnedVariant<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohDeclarationVariantIn<S>;

impl<S: CodecZenohDeclarationStorage> Default for CodecZenohDeclarationVariantIn<S> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohDeclFinal(CodecZenohDeclFinal::default())
    }
}

#[allow(dead_code)]
impl<S: CodecZenohDeclarationStorage> CodecZenohDeclarationIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohDeclarationFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohDeclarationFrom<'f> {
        // A field past what the profile holds is read on and not held; the
        // frame says so once it is read whole (docs/adr/0077 §7).
        let mut _over = false;
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
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclKexpr(CodecZenohDeclKexprOwned::<S>::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclKexpr(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohDeclKexprOwned::<S>::decode_into(cursor, _b, (header >> 5) & 0x1), &mut _over)?;
                }
            }
            1u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclKexpr(CodecZenohUndeclKexpr::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclKexpr(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohUndeclKexpr::decode_into(cursor, _b), &mut _over)?;
                }
            }
            2u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclSubscriber(CodecZenohDeclSubscriberOwned::<S>::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclSubscriber(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohDeclSubscriberOwned::<S>::decode_into(cursor, _b, (header >> 5) & 0x1), &mut _over)?;
                }
            }
            3u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriberOwned::<S>::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclSubscriber(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohUndeclSubscriberOwned::<S>::decode_into(cursor, _b, (header >> 7) & 0x1), &mut _over)?;
                }
            }
            4u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclQueryable(CodecZenohDeclQueryableOwned::<S>::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclQueryable(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohDeclQueryableOwned::<S>::decode_into(cursor, _b, (header >> 5) & 0x1, (header >> 7) & 0x1), &mut _over)?;
                }
            }
            5u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclQueryable(CodecZenohUndeclQueryableOwned::<S>::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclQueryable(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohUndeclQueryableOwned::<S>::decode_into(cursor, _b, (header >> 7) & 0x1), &mut _over)?;
                }
            }
            6u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclToken(CodecZenohDeclTokenOwned::<S>::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclToken(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohDeclTokenOwned::<S>::decode_into(cursor, _b, (header >> 5) & 0x1), &mut _over)?;
                }
            }
            7u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclToken(CodecZenohUndeclTokenOwned::<S>::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclToken(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohUndeclTokenOwned::<S>::decode_into(cursor, _b, (header >> 7) & 0x1), &mut _over)?;
                }
            }
            26u8 => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclFinal(CodecZenohDeclFinal::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclFinal(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohDeclFinal::decode_into(cursor, _b), &mut _over)?;
                }
            }
            other => {
                _out.body = CodecZenohDeclarationVariantIn::Default {
                    tag: CodecZenohDeclarationUnknownTag(other),
                    body: CodecZenohDeclFinal::default(),
                };
                if let CodecZenohDeclarationVariantIn::Default { body: _b, .. } = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohDeclFinal::decode_into(cursor, _b), &mut _over)?;
                }
            }
        }
        _out.header = header;
        if _over {
            return Err(CodecError::CapacityExceeded);
        }
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
    pub const MAX_ENCODED_BYTES: usize = 65571;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohDeclarationWire::encode_source(self, w)
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
    Default { tag: CodecZenohDeclarationUnknownTag, body: &'s D },
}

impl<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, A3: ?Sized, A4: ?Sized, A5: ?Sized, A6: ?Sized, A7: ?Sized, A8: ?Sized, D: ?Sized> CodecZenohDeclarationVariantSource<'s, A0, A1, A2, A3, A4, A5, A6, A7, A8, D> {
    /// The tag of the arm this is: the value the document gives a known
    /// arm, and the tag a default arm holds. It is the tag the encode
    /// writes, or the one a caller hands the decode for a variant whose
    /// wire form carries none.
    pub fn tag(&self) -> u8 {
        match self {
            Self::CodecZenohDeclKexpr(_) => 0,
            Self::CodecZenohUndeclKexpr(_) => 1,
            Self::CodecZenohDeclSubscriber(_) => 2,
            Self::CodecZenohUndeclSubscriber(_) => 3,
            Self::CodecZenohDeclQueryable(_) => 4,
            Self::CodecZenohUndeclQueryable(_) => 5,
            Self::CodecZenohDeclToken(_) => 6,
            Self::CodecZenohUndeclToken(_) => 7,
            Self::CodecZenohDeclFinal(_) => 26,
            Self::Default { tag, .. } => tag.get(),
        }
    }
}

/// Flags of `CodecZenohDeclaration` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecZenohDeclaration`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecZenohDeclarationOverrides {
    /// The `M` flag of `header`, when a parent sets it.
    pub header_m: Option<bool>,
}

#[allow(dead_code)]
impl CodecZenohDeclarationOverrides {
    /// Sets `M` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_m(value: bool) -> Self {
        Self { header_m: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohDeclaration` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohDeclarationWire::encode_source`]).
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
    Default { tag: CodecZenohDeclarationUnknownTag, body: &'a super::codec_zenoh_decl_final::CodecZenohDeclFinalParts },
}

/// A value of `CodecZenohDeclaration` held in the types a caller already has: a source
/// ([`CodecZenohDeclarationSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
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

impl<S: CodecZenohDeclarationStorage> CodecZenohDeclarationSource for CodecZenohDeclarationOwned<S> {
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

impl<'a> CodecZenohDeclarationSource for CodecZenohDeclarationView<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohDeclarationVariantSource<'_, impl super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprSource, impl super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprSource, impl super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberSource, impl super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberSource, impl super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableSource, impl super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableSource, impl super::codec_zenoh_decl_token::CodecZenohDeclTokenSource, impl super::codec_zenoh_undecl_token::CodecZenohUndeclTokenSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource, impl super::codec_zenoh_decl_final::CodecZenohDeclFinalSource> {
        match &self.body {
            CodecZenohDeclarationViewVariant::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(_b),
            CodecZenohDeclarationViewVariant::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclKexpr(_b),
            CodecZenohDeclarationViewVariant::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(_b),
            CodecZenohDeclarationViewVariant::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(_b),
            CodecZenohDeclarationViewVariant::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_b),
            CodecZenohDeclarationViewVariant::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(_b),
            CodecZenohDeclarationViewVariant::CodecZenohDeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclToken(_b),
            CodecZenohDeclarationViewVariant::CodecZenohUndeclToken(_b) => CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(_b),
            CodecZenohDeclarationViewVariant::CodecZenohDeclFinal(_b) => CodecZenohDeclarationVariantSource::CodecZenohDeclFinal(_b),
            CodecZenohDeclarationViewVariant::Default { tag, body } => CodecZenohDeclarationVariantSource::Default { tag: *tag, body },
        }
    }
}

/// The codec `CodecZenohDeclaration` itself: what writes a value read through
/// [`CodecZenohDeclarationSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohDeclarationWire;

#[allow(dead_code)]
impl CodecZenohDeclarationWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohDeclaration::encode` writes for the value it holds, read through
    /// [`CodecZenohDeclarationSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohDeclarationSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The tag of the arm `src` holds ([`CodecZenohDeclarationVariantSource::tag`]).
    pub fn tag_of<S: CodecZenohDeclarationSource + ?Sized>(src: &S) -> u8 {
        src.body().tag()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohDeclarationSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source_in(src, w, &CodecZenohDeclarationOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecZenohDeclarationSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecZenohDeclarationOverrides) -> Result<(), K::Error> {
        let _d_header_mid: u8 = src.body().tag();
        let _o_d_header_n_arm: Option<usize> = match src.body() {
            CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(_b) => Some(usize::from(super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprWire::input_n(_b))),
            CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(_b) => Some(usize::from(super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberWire::input_n(_b))),
            CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_b) => Some(usize::from(super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableWire::input_n(_b))),
            CodecZenohDeclarationVariantSource::CodecZenohDeclToken(_b) => Some(usize::from(super::codec_zenoh_decl_token::CodecZenohDeclTokenWire::input_n(_b))),
            _ => None,
        };
        let _f_d_header_n: Option<usize> = if _o_d_header_n_arm.is_some() { Some(_o_d_header_n_arm.unwrap_or(0)) } else { None };
        let _o_d_header_z_arm: Option<usize> = match src.body() {
            CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(_b) => Some(usize::from(super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberWire::input_z(_b))),
            CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_b) => Some(usize::from(super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableWire::input_z(_b))),
            CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(_b) => Some(usize::from(super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableWire::input_z(_b))),
            CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(_b) => Some(usize::from(super::codec_zenoh_undecl_token::CodecZenohUndeclTokenWire::input_z(_b))),
            _ => None,
        };
        let _f_d_header_z: Option<usize> = if _o_d_header_z_arm.is_some() { Some(_o_d_header_z_arm.unwrap_or(0)) } else { None };
        let mut _c_header: u8 = (src.header() & !0x1F) | _d_header_mid;
        if let Some(_n) = _f_d_header_n {
            _c_header = (_c_header & !0x20) | ((_n as u8) << 5);
        }
        if let Some(_n) = _f_d_header_z {
            _c_header = (_c_header & !0x80) | ((_n as u8) << 7);
        }
        if let Some(_o) = over.header_m {
            _c_header = (_c_header & !0x40) | (u8::from(_o) << 6);
        }
        w.write_u8(_c_header)?;
        match src.body() {
            CodecZenohDeclarationVariantSource::CodecZenohDeclKexpr(_b) => super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclKexpr(_b) => super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexprWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclSubscriber(_b) => super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclSubscriber(_b) => super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclQueryable(_b) => super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclQueryable(_b) => super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclToken(_b) => super::codec_zenoh_decl_token::CodecZenohDeclTokenWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohUndeclToken(_b) => super::codec_zenoh_undecl_token::CodecZenohUndeclTokenWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::CodecZenohDeclFinal(_b) => super::codec_zenoh_decl_final::CodecZenohDeclFinalWire::write_source(_b, w)?,
            CodecZenohDeclarationVariantSource::Default { body: _b, .. } => super::codec_zenoh_decl_final::CodecZenohDeclFinalWire::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohDeclarationSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohDeclarationSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohDeclarationWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohDeclarationWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohDeclarationSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
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


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecZenohDeclarationView` is what a decode of `CodecZenohDeclaration` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecZenohDeclaration` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecZenohDeclaration::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohDeclarationViewVariant<'a> {
    CodecZenohDeclKexpr(CodecZenohDeclKexprView<'a>),
    CodecZenohUndeclKexpr(CodecZenohUndeclKexprView),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriberView<'a>),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriberView<'a>),
    CodecZenohDeclQueryable(CodecZenohDeclQueryableView<'a>),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryableView<'a>),
    CodecZenohDeclToken(CodecZenohDeclTokenView<'a>),
    CodecZenohUndeclToken(CodecZenohUndeclTokenView<'a>),
    CodecZenohDeclFinal(CodecZenohDeclFinalView),
    Default {
        tag: CodecZenohDeclarationUnknownTag,
        body: CodecZenohDeclFinalView,
    },
}

impl<'a> Default for CodecZenohDeclarationViewVariant<'a> {
    fn default() -> Self {
        Self::CodecZenohDeclFinal(CodecZenohDeclFinalView::default())
    }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohDeclarationView<'a> {
    pub header: u8,
    pub body: CodecZenohDeclarationViewVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclarationView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecZenohDeclaration::decode`], held as a view. On success the
    /// cursor advances past the consumed bytes.
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out)?;
        Ok(_out)
    }

    /// [`Self::decode`] into a destination the caller owns, so a view that
    /// embeds or dispatches to this one is filled where it will live. On
    /// `Err`, `_out` holds the fields decoded so far and is not to be read.
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
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohDeclKexpr(CodecZenohDeclKexprView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohDeclKexpr(_b) = &mut _out.body {
                    CodecZenohDeclKexprView::decode_into(cursor, _b, (header >> 5) & 0x1)?;
                }
            }
            1u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohUndeclKexpr(CodecZenohUndeclKexprView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohUndeclKexpr(_b) = &mut _out.body {
                    CodecZenohUndeclKexprView::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohDeclSubscriber(CodecZenohDeclSubscriberView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohDeclSubscriber(_b) = &mut _out.body {
                    CodecZenohDeclSubscriberView::decode_into(cursor, _b, (header >> 5) & 0x1)?;
                }
            }
            3u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriberView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohUndeclSubscriber(_b) = &mut _out.body {
                    CodecZenohUndeclSubscriberView::decode_into(cursor, _b, (header >> 7) & 0x1)?;
                }
            }
            4u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohDeclQueryable(CodecZenohDeclQueryableView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohDeclQueryable(_b) = &mut _out.body {
                    CodecZenohDeclQueryableView::decode_into(cursor, _b, (header >> 5) & 0x1, (header >> 7) & 0x1)?;
                }
            }
            5u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohUndeclQueryable(CodecZenohUndeclQueryableView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohUndeclQueryable(_b) = &mut _out.body {
                    CodecZenohUndeclQueryableView::decode_into(cursor, _b, (header >> 7) & 0x1)?;
                }
            }
            6u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohDeclToken(CodecZenohDeclTokenView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohDeclToken(_b) = &mut _out.body {
                    CodecZenohDeclTokenView::decode_into(cursor, _b, (header >> 5) & 0x1)?;
                }
            }
            7u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohUndeclToken(CodecZenohUndeclTokenView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohUndeclToken(_b) = &mut _out.body {
                    CodecZenohUndeclTokenView::decode_into(cursor, _b, (header >> 7) & 0x1)?;
                }
            }
            26u8 => {
                _out.body = CodecZenohDeclarationViewVariant::CodecZenohDeclFinal(CodecZenohDeclFinalView::default());
                if let CodecZenohDeclarationViewVariant::CodecZenohDeclFinal(_b) = &mut _out.body {
                    CodecZenohDeclFinalView::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohDeclarationViewVariant::Default {
                    tag: CodecZenohDeclarationUnknownTag(other),
                    body: CodecZenohDeclFinalView::default(),
                };
                if let CodecZenohDeclarationViewVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohDeclFinalView::decode_into(cursor, _b)?;
                }
            }
        }
        _out.header = header;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecZenohDeclaration`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder<D>(&self, _out: &mut CodecZenohDeclarationIn<::sce_forge_runtime::codec::Lent<'a, D>>) -> Result<(), CodecError> where D: CodecZenohDeclarationLentStorage {
        _out.header = self.header;
        match &self.body {
            CodecZenohDeclarationViewVariant::CodecZenohDeclKexpr(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclKexpr(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclKexpr(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohUndeclKexpr(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclKexpr(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclKexpr(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohDeclSubscriber(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclSubscriber(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclSubscriber(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohUndeclSubscriber(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclSubscriber(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclSubscriber(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohDeclQueryable(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclQueryable(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclQueryable(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohUndeclQueryable(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclQueryable(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclQueryable(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohDeclToken(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclToken(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclToken(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohUndeclToken(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohUndeclToken(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclToken(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::CodecZenohDeclFinal(_b) => {
                _out.body = CodecZenohDeclarationVariantIn::CodecZenohDeclFinal(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclFinal(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohDeclarationViewVariant::Default { tag, body } => {
                _out.body = CodecZenohDeclarationVariantIn::Default {
                    tag: *tag,
                    body: Default::default(),
                };
                if let CodecZenohDeclarationVariantIn::Default { body: _o, .. } = &mut _out.body {
                    body.into_builder(_o)?;
                }
            }
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecZenohDeclaration::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 65571;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecZenohDeclaration::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecZenohDeclarationWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }

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
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a, D> CodecZenohDeclarationVariantIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohDeclarationLentStorage {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: CodecZenohDeclarationFrom<'a>>(self) -> Result<CodecZenohDeclarationOwnedVariant<S>, CodecError> {
        Ok(match self {
            Self::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b),
            Self::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohDeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohUndeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohDeclFinal(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b),
            Self::Default { tag, body } => CodecZenohDeclarationOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclarationOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohDeclarationStorage> CodecZenohDeclarationOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn as_borrowed(&self) -> CodecZenohDeclarationVariantIn<::sce_forge_runtime::codec::Lent<'_, S>> {
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohDeclarationVariantIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        match self {
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohDeclKexpr(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclKexpr(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohUndeclKexpr(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclKexpr(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohDeclSubscriber(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclSubscriber(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohUndeclSubscriber(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclSubscriber(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohDeclQueryable(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclQueryable(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohUndeclQueryable(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclQueryable(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohDeclToken(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclToken(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohUndeclToken(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohUndeclToken(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b) => {
                *_out = CodecZenohDeclarationVariantIn::CodecZenohDeclFinal(Default::default());
                if let CodecZenohDeclarationVariantIn::CodecZenohDeclFinal(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohDeclarationOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohDeclarationVariantIn::Default { tag: *tag, body: Default::default() };
                if let CodecZenohDeclarationVariantIn::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        
    }
}

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohDeclaration<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohDeclarationIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohDeclarationLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohDeclarationOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohDeclarationFrom<'a>>(self) -> Result<CodecZenohDeclarationOwned<S>, CodecError> {
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
impl<S: CodecZenohDeclarationStorage> CodecZenohDeclarationOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohDeclarationIn<::sce_forge_runtime::codec::Lent<'_, S>> {
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohDeclarationIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
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
    pub fn transcode_in<D: for<'x> CodecZenohDeclarationFrom<'x>>(&self) -> Result<CodecZenohDeclarationOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}
