#![doc = "SCE-MAP: codec_zenoh_network_envelope:60 :: _forge_body"]
// SCE-MAP: codec_zenoh_network_envelope:60 :: _forge_body

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

use super::codec_zenoh_push::CodecZenohPush;

use super::codec_zenoh_interest::CodecZenohInterestView;
use super::codec_zenoh_response_final::CodecZenohResponseFinalView;
use super::codec_zenoh_response::CodecZenohResponseView;
use super::codec_zenoh_request::CodecZenohRequestView;
use super::codec_zenoh_push::CodecZenohPushView;
use super::codec_zenoh_declare::CodecZenohDeclareView;
use super::codec_zenoh_oam::CodecZenohOamView;

/// A tag `CodecZenohNetworkEnvelope` names no arm for, which its default arm holds: a
/// value inside the 5 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecZenohNetworkEnvelopeUnknownTag(u8);

#[allow(dead_code)]
impl CodecZenohNetworkEnvelopeUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            25..=31 => None,
            _ if tag > 31 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecZenohNetworkEnvelopeUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(31, &[(25, 31)]);

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
use super::codec_zenoh_interest::CodecZenohInterestOwned;
use super::codec_zenoh_response_final::CodecZenohResponseFinalOwned;
use super::codec_zenoh_response::CodecZenohResponseOwned;
use super::codec_zenoh_request::CodecZenohRequestOwned;
use super::codec_zenoh_declare::CodecZenohDeclareOwned;
use super::codec_zenoh_oam::CodecZenohOamOwned;
/// What `CodecZenohNetworkEnvelopeIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohNetworkEnvelopeStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_interest::CodecZenohInterestStorage + super::codec_zenoh_response_final::CodecZenohResponseFinalStorage + super::codec_zenoh_response::CodecZenohResponseStorage + super::codec_zenoh_request::CodecZenohRequestStorage + super::codec_zenoh_declare::CodecZenohDeclareStorage + super::codec_zenoh_oam::CodecZenohOamStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_interest::CodecZenohInterestStorage + super::codec_zenoh_response_final::CodecZenohResponseFinalStorage + super::codec_zenoh_response::CodecZenohResponseStorage + super::codec_zenoh_request::CodecZenohRequestStorage + super::codec_zenoh_declare::CodecZenohDeclareStorage + super::codec_zenoh_oam::CodecZenohOamStorage> CodecZenohNetworkEnvelopeStorage for S {}

/// What `CodecZenohNetworkEnvelopeIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohNetworkEnvelope` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohNetworkEnvelopeLentStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_interest::CodecZenohInterestLentStorage + super::codec_zenoh_response_final::CodecZenohResponseFinalLentStorage + super::codec_zenoh_response::CodecZenohResponseLentStorage + super::codec_zenoh_request::CodecZenohRequestLentStorage + super::codec_zenoh_declare::CodecZenohDeclareLentStorage + super::codec_zenoh_oam::CodecZenohOamLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_interest::CodecZenohInterestLentStorage + super::codec_zenoh_response_final::CodecZenohResponseFinalLentStorage + super::codec_zenoh_response::CodecZenohResponseLentStorage + super::codec_zenoh_request::CodecZenohRequestLentStorage + super::codec_zenoh_declare::CodecZenohDeclareLentStorage + super::codec_zenoh_oam::CodecZenohOamLentStorage> CodecZenohNetworkEnvelopeLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohNetworkEnvelopeStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohNetworkEnvelopeFrom<'f>: CodecZenohNetworkEnvelopeStorage + super::codec_zenoh_interest::CodecZenohInterestFrom<'f> + super::codec_zenoh_response_final::CodecZenohResponseFinalFrom<'f> + super::codec_zenoh_response::CodecZenohResponseFrom<'f> + super::codec_zenoh_request::CodecZenohRequestFrom<'f> + super::codec_zenoh_declare::CodecZenohDeclareFrom<'f> + super::codec_zenoh_oam::CodecZenohOamFrom<'f> {}

impl<'f, S: CodecZenohNetworkEnvelopeStorage + super::codec_zenoh_interest::CodecZenohInterestFrom<'f> + super::codec_zenoh_response_final::CodecZenohResponseFinalFrom<'f> + super::codec_zenoh_response::CodecZenohResponseFrom<'f> + super::codec_zenoh_request::CodecZenohRequestFrom<'f> + super::codec_zenoh_declare::CodecZenohDeclareFrom<'f> + super::codec_zenoh_oam::CodecZenohOamFrom<'f>> CodecZenohNetworkEnvelopeFrom<'f> for S {}

// `CodecZenohNetworkEnvelopeIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohNetworkEnvelope<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohNetworkEnvelopeOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohNetworkEnvelopeIn<S: CodecZenohNetworkEnvelopeStorage> {
    pub body: CodecZenohNetworkEnvelopeVariantIn<S>,
}

// No eager form at the default profile: `Lent<'a>` holds each list as `Inline`
// does, and `Inline` holds no list whose protocol states no bound
// (docs/adr/0077 §7). Name the eager form at a profile that holds its lists,
// `CodecZenohNetworkEnvelopeIn<Lent<'a, D>>`.

/// The value at a profile that holds its bytes and text, named
/// with the profile: the build's default on the heap-free tier is `Inline`, which
/// holds no field whose protocol states no bound (docs/adr/0077 §7).
#[allow(dead_code)]
pub type CodecZenohNetworkEnvelopeOwned<S> = CodecZenohNetworkEnvelopeIn<S>;

impl<S: CodecZenohNetworkEnvelopeStorage> Default for CodecZenohNetworkEnvelopeIn<S> {
    fn default() -> Self {
        Self {
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
pub enum CodecZenohNetworkEnvelopeVariantIn<S: CodecZenohNetworkEnvelopeStorage> {
    CodecZenohInterest(CodecZenohInterestOwned<S>),
    CodecZenohResponseFinal(CodecZenohResponseFinalOwned<S>),
    CodecZenohResponse(CodecZenohResponseOwned<S>),
    CodecZenohRequest(CodecZenohRequestOwned<S>),
    CodecZenohPush(CodecZenohPush),
    CodecZenohDeclare(CodecZenohDeclareOwned<S>),
    CodecZenohOam(CodecZenohOamOwned<S>),
    Default {
        tag: CodecZenohNetworkEnvelopeUnknownTag,
        body: CodecZenohOamOwned<S>,
    },
}


#[allow(dead_code)]
pub type CodecZenohNetworkEnvelopeOwnedVariant<S> = CodecZenohNetworkEnvelopeVariantIn<S>;

impl<S: CodecZenohNetworkEnvelopeStorage> Default for CodecZenohNetworkEnvelopeVariantIn<S> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohOam(CodecZenohOamOwned::<S>::default())
    }
}

#[allow(dead_code)]
impl<S: CodecZenohNetworkEnvelopeStorage> CodecZenohNetworkEnvelopeIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohNetworkEnvelopeFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohNetworkEnvelopeFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohNetworkEnvelopeView::decode(&mut _frame)?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohNetworkEnvelopeFrom<'f> {
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for the variant
        // tag without advancing — the arm body decoder reads the peeked
        // byte as its own header byte (Zenoh response/request body MID
        // dispatch shape per network.c:347-364 + 220-235).
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match _peek & 0x1F {
            25u8 => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohInterest(CodecZenohInterestOwned::<S>::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohInterest(_b) = &mut _out.body {
                    CodecZenohInterestOwned::<S>::decode_into(cursor, _b)?;
                }
            }
            26u8 => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponseFinal(CodecZenohResponseFinalOwned::<S>::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponseFinal(_b) = &mut _out.body {
                    CodecZenohResponseFinalOwned::<S>::decode_into(cursor, _b)?;
                }
            }
            27u8 => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponse(CodecZenohResponseOwned::<S>::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponse(_b) = &mut _out.body {
                    CodecZenohResponseOwned::<S>::decode_into(cursor, _b)?;
                }
            }
            28u8 => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohRequest(CodecZenohRequestOwned::<S>::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohRequest(_b) = &mut _out.body {
                    CodecZenohRequestOwned::<S>::decode_into(cursor, _b)?;
                }
            }
            29u8 => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohPush(CodecZenohPush::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohPush(_b) = &mut _out.body {
                    CodecZenohPush::decode_into(cursor, _b)?;
                }
            }
            30u8 => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohDeclare(CodecZenohDeclareOwned::<S>::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohDeclare(_b) = &mut _out.body {
                    CodecZenohDeclareOwned::<S>::decode_into(cursor, _b)?;
                }
            }
            31u8 => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohOam(CodecZenohOamOwned::<S>::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohOam(_b) = &mut _out.body {
                    CodecZenohOamOwned::<S>::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::Default {
                    tag: CodecZenohNetworkEnvelopeUnknownTag(other),
                    body: CodecZenohOamOwned::<S>::default(),
                };
                if let CodecZenohNetworkEnvelopeVariantIn::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohOamOwned::<S>::decode_into(cursor, _b)?;
                }
            }
        }
        Ok(())
    }


    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohNetworkEnvelopeWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade. It reserves nothing, since the
    /// largest frame passes `u32` (docs/adr/0077 §7), and delegates to
    /// `encode` over a
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
        let mut _sce_v: Vec<u8> = Vec::new();
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}

/// The arm of `CodecZenohNetworkEnvelope`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohNetworkEnvelope` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohNetworkEnvelopeVariantSource<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, A3: ?Sized, A4: ?Sized, A5: ?Sized, A6: ?Sized, D: ?Sized> {
    CodecZenohInterest(&'s A0),
    CodecZenohResponseFinal(&'s A1),
    CodecZenohResponse(&'s A2),
    CodecZenohRequest(&'s A3),
    CodecZenohPush(&'s A4),
    CodecZenohDeclare(&'s A5),
    CodecZenohOam(&'s A6),
    Default { tag: CodecZenohNetworkEnvelopeUnknownTag, body: &'s D },
}

impl<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, A3: ?Sized, A4: ?Sized, A5: ?Sized, A6: ?Sized, D: ?Sized> CodecZenohNetworkEnvelopeVariantSource<'s, A0, A1, A2, A3, A4, A5, A6, D> {
    /// The tag of the arm this is: the value the document gives a known
    /// arm, and the tag a default arm holds. It is the tag the encode
    /// writes, or the one a caller hands the decode for a variant whose
    /// wire form carries none.
    pub fn tag(&self) -> u8 {
        match self {
            Self::CodecZenohInterest(_) => 25,
            Self::CodecZenohResponseFinal(_) => 26,
            Self::CodecZenohResponse(_) => 27,
            Self::CodecZenohRequest(_) => 28,
            Self::CodecZenohPush(_) => 29,
            Self::CodecZenohDeclare(_) => 30,
            Self::CodecZenohOam(_) => 31,
            Self::Default { tag, .. } => tag.get(),
        }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohNetworkEnvelope` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohNetworkEnvelopeWire::encode_source`]).
pub trait CodecZenohNetworkEnvelopeSource {
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohNetworkEnvelopeVariantSource<'_, impl super::codec_zenoh_interest::CodecZenohInterestSource, impl super::codec_zenoh_response_final::CodecZenohResponseFinalSource, impl super::codec_zenoh_response::CodecZenohResponseSource, impl super::codec_zenoh_request::CodecZenohRequestSource, impl super::codec_zenoh_push::CodecZenohPushSource, impl super::codec_zenoh_declare::CodecZenohDeclareSource, impl super::codec_zenoh_oam::CodecZenohOamSource, impl super::codec_zenoh_oam::CodecZenohOamSource>;
}

impl<T: CodecZenohNetworkEnvelopeSource + ?Sized> CodecZenohNetworkEnvelopeSource for &T {
    fn body(&self) -> CodecZenohNetworkEnvelopeVariantSource<'_, impl super::codec_zenoh_interest::CodecZenohInterestSource, impl super::codec_zenoh_response_final::CodecZenohResponseFinalSource, impl super::codec_zenoh_response::CodecZenohResponseSource, impl super::codec_zenoh_request::CodecZenohRequestSource, impl super::codec_zenoh_push::CodecZenohPushSource, impl super::codec_zenoh_declare::CodecZenohDeclareSource, impl super::codec_zenoh_oam::CodecZenohOamSource, impl super::codec_zenoh_oam::CodecZenohOamSource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohNetworkEnvelope`'s variant that CodecZenohNetworkEnvelopeParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohNetworkEnvelopePartsBody<'a> {
    CodecZenohInterest(&'a super::codec_zenoh_interest::CodecZenohInterestParts<'a>),
    CodecZenohResponseFinal(&'a super::codec_zenoh_response_final::CodecZenohResponseFinalParts<'a>),
    CodecZenohResponse(&'a super::codec_zenoh_response::CodecZenohResponseParts<'a>),
    CodecZenohRequest(&'a super::codec_zenoh_request::CodecZenohRequestParts<'a>),
    CodecZenohPush(&'a super::codec_zenoh_push::CodecZenohPushParts<'a>),
    CodecZenohDeclare(&'a super::codec_zenoh_declare::CodecZenohDeclareParts<'a>),
    CodecZenohOam(&'a super::codec_zenoh_oam::CodecZenohOamParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecZenohNetworkEnvelopeUnknownTag, body: &'a super::codec_zenoh_oam::CodecZenohOamParts<'a> },
}

/// A value of `CodecZenohNetworkEnvelope` held in the types a caller already has: a source
/// ([`CodecZenohNetworkEnvelopeSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohNetworkEnvelopeParts<'a> {
    /// The arm of the variant this value holds.
    pub body: CodecZenohNetworkEnvelopePartsBody<'a>,
}

impl<'a> CodecZenohNetworkEnvelopeSource for CodecZenohNetworkEnvelopeParts<'a> {
    fn body(&self) -> CodecZenohNetworkEnvelopeVariantSource<'_, impl super::codec_zenoh_interest::CodecZenohInterestSource, impl super::codec_zenoh_response_final::CodecZenohResponseFinalSource, impl super::codec_zenoh_response::CodecZenohResponseSource, impl super::codec_zenoh_request::CodecZenohRequestSource, impl super::codec_zenoh_push::CodecZenohPushSource, impl super::codec_zenoh_declare::CodecZenohDeclareSource, impl super::codec_zenoh_oam::CodecZenohOamSource, impl super::codec_zenoh_oam::CodecZenohOamSource> {
        match &self.body {
            CodecZenohNetworkEnvelopePartsBody::CodecZenohInterest(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohInterest(*_b),
            CodecZenohNetworkEnvelopePartsBody::CodecZenohResponseFinal(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponseFinal(*_b),
            CodecZenohNetworkEnvelopePartsBody::CodecZenohResponse(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponse(*_b),
            CodecZenohNetworkEnvelopePartsBody::CodecZenohRequest(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohRequest(*_b),
            CodecZenohNetworkEnvelopePartsBody::CodecZenohPush(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohPush(*_b),
            CodecZenohNetworkEnvelopePartsBody::CodecZenohDeclare(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohDeclare(*_b),
            CodecZenohNetworkEnvelopePartsBody::CodecZenohOam(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohOam(*_b),
            CodecZenohNetworkEnvelopePartsBody::Default { tag, body } => CodecZenohNetworkEnvelopeVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<S: CodecZenohNetworkEnvelopeStorage> CodecZenohNetworkEnvelopeSource for CodecZenohNetworkEnvelopeOwned<S> {
    fn body(&self) -> CodecZenohNetworkEnvelopeVariantSource<'_, impl super::codec_zenoh_interest::CodecZenohInterestSource, impl super::codec_zenoh_response_final::CodecZenohResponseFinalSource, impl super::codec_zenoh_response::CodecZenohResponseSource, impl super::codec_zenoh_request::CodecZenohRequestSource, impl super::codec_zenoh_push::CodecZenohPushSource, impl super::codec_zenoh_declare::CodecZenohDeclareSource, impl super::codec_zenoh_oam::CodecZenohOamSource, impl super::codec_zenoh_oam::CodecZenohOamSource> {
        match &self.body {
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohInterest(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohInterest(_b),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponseFinal(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponseFinal(_b),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponse(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponse(_b),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohRequest(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohRequest(_b),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohPush(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohPush(_b),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohDeclare(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohDeclare(_b),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohOam(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohOam(_b),
            CodecZenohNetworkEnvelopeOwnedVariant::Default { tag, body } => CodecZenohNetworkEnvelopeVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<'a> CodecZenohNetworkEnvelopeSource for CodecZenohNetworkEnvelopeView<'a> {
    fn body(&self) -> CodecZenohNetworkEnvelopeVariantSource<'_, impl super::codec_zenoh_interest::CodecZenohInterestSource, impl super::codec_zenoh_response_final::CodecZenohResponseFinalSource, impl super::codec_zenoh_response::CodecZenohResponseSource, impl super::codec_zenoh_request::CodecZenohRequestSource, impl super::codec_zenoh_push::CodecZenohPushSource, impl super::codec_zenoh_declare::CodecZenohDeclareSource, impl super::codec_zenoh_oam::CodecZenohOamSource, impl super::codec_zenoh_oam::CodecZenohOamSource> {
        match &self.body {
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohInterest(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohInterest(_b),
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponseFinal(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponseFinal(_b),
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponse(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponse(_b),
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohRequest(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohRequest(_b),
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohPush(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohPush(_b),
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohDeclare(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohDeclare(_b),
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohOam(_b) => CodecZenohNetworkEnvelopeVariantSource::CodecZenohOam(_b),
            CodecZenohNetworkEnvelopeViewVariant::Default { tag, body } => CodecZenohNetworkEnvelopeVariantSource::Default { tag: *tag, body },
        }
    }
}

/// The codec `CodecZenohNetworkEnvelope` itself: what writes a value read through
/// [`CodecZenohNetworkEnvelopeSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohNetworkEnvelopeWire;

#[allow(dead_code)]
impl CodecZenohNetworkEnvelopeWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohNetworkEnvelope::encode` writes for the value it holds, read through
    /// [`CodecZenohNetworkEnvelopeSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohNetworkEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The tag of the arm `src` holds ([`CodecZenohNetworkEnvelopeVariantSource::tag`]).
    pub fn tag_of<S: CodecZenohNetworkEnvelopeSource + ?Sized>(src: &S) -> u8 {
        src.body().tag()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohNetworkEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        match src.body() {
            CodecZenohNetworkEnvelopeVariantSource::CodecZenohInterest(_b) => super::codec_zenoh_interest::CodecZenohInterestWire::write_source(_b, w)?,
            CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponseFinal(_b) => super::codec_zenoh_response_final::CodecZenohResponseFinalWire::write_source(_b, w)?,
            CodecZenohNetworkEnvelopeVariantSource::CodecZenohResponse(_b) => super::codec_zenoh_response::CodecZenohResponseWire::write_source(_b, w)?,
            CodecZenohNetworkEnvelopeVariantSource::CodecZenohRequest(_b) => super::codec_zenoh_request::CodecZenohRequestWire::write_source(_b, w)?,
            CodecZenohNetworkEnvelopeVariantSource::CodecZenohPush(_b) => super::codec_zenoh_push::CodecZenohPushWire::write_source(_b, w)?,
            CodecZenohNetworkEnvelopeVariantSource::CodecZenohDeclare(_b) => super::codec_zenoh_declare::CodecZenohDeclareWire::write_source(_b, w)?,
            CodecZenohNetworkEnvelopeVariantSource::CodecZenohOam(_b) => super::codec_zenoh_oam::CodecZenohOamWire::write_source(_b, w)?,
            CodecZenohNetworkEnvelopeVariantSource::Default { body: _b, .. } => super::codec_zenoh_oam::CodecZenohOamWire::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohNetworkEnvelopeSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohNetworkEnvelopeSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohNetworkEnvelopeWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohNetworkEnvelopeWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohNetworkEnvelopeSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohNetworkEnvelopeSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecZenohNetworkEnvelopeView` is what a decode of `CodecZenohNetworkEnvelope` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecZenohNetworkEnvelope` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecZenohNetworkEnvelope::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohNetworkEnvelopeViewVariant<'a> {
    CodecZenohInterest(CodecZenohInterestView<'a>),
    CodecZenohResponseFinal(CodecZenohResponseFinalView<'a>),
    CodecZenohResponse(CodecZenohResponseView<'a>),
    CodecZenohRequest(CodecZenohRequestView<'a>),
    CodecZenohPush(CodecZenohPushView),
    CodecZenohDeclare(CodecZenohDeclareView<'a>),
    CodecZenohOam(CodecZenohOamView<'a>),
    Default {
        tag: CodecZenohNetworkEnvelopeUnknownTag,
        body: CodecZenohOamView<'a>,
    },
}

impl<'a> Default for CodecZenohNetworkEnvelopeViewVariant<'a> {
    fn default() -> Self {
        Self::CodecZenohOam(CodecZenohOamView::default())
    }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohNetworkEnvelopeView<'a> {
    pub body: CodecZenohNetworkEnvelopeViewVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecZenohNetworkEnvelopeView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecZenohNetworkEnvelopeIn::decode`], held as a view. On success the
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
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for the variant
        // tag without advancing — the arm body decoder reads the peeked
        // byte as its own header byte (Zenoh response/request body MID
        // dispatch shape per network.c:347-364 + 220-235).
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match _peek & 0x1F {
            25u8 => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::CodecZenohInterest(CodecZenohInterestView::default());
                if let CodecZenohNetworkEnvelopeViewVariant::CodecZenohInterest(_b) = &mut _out.body {
                    CodecZenohInterestView::decode_into(cursor, _b)?;
                }
            }
            26u8 => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponseFinal(CodecZenohResponseFinalView::default());
                if let CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponseFinal(_b) = &mut _out.body {
                    CodecZenohResponseFinalView::decode_into(cursor, _b)?;
                }
            }
            27u8 => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponse(CodecZenohResponseView::default());
                if let CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponse(_b) = &mut _out.body {
                    CodecZenohResponseView::decode_into(cursor, _b)?;
                }
            }
            28u8 => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::CodecZenohRequest(CodecZenohRequestView::default());
                if let CodecZenohNetworkEnvelopeViewVariant::CodecZenohRequest(_b) = &mut _out.body {
                    CodecZenohRequestView::decode_into(cursor, _b)?;
                }
            }
            29u8 => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::CodecZenohPush(CodecZenohPushView::default());
                if let CodecZenohNetworkEnvelopeViewVariant::CodecZenohPush(_b) = &mut _out.body {
                    CodecZenohPushView::decode_into(cursor, _b)?;
                }
            }
            30u8 => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::CodecZenohDeclare(CodecZenohDeclareView::default());
                if let CodecZenohNetworkEnvelopeViewVariant::CodecZenohDeclare(_b) = &mut _out.body {
                    CodecZenohDeclareView::decode_into(cursor, _b)?;
                }
            }
            31u8 => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::CodecZenohOam(CodecZenohOamView::default());
                if let CodecZenohNetworkEnvelopeViewVariant::CodecZenohOam(_b) = &mut _out.body {
                    CodecZenohOamView::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohNetworkEnvelopeViewVariant::Default {
                    tag: CodecZenohNetworkEnvelopeUnknownTag(other),
                    body: CodecZenohOamView::default(),
                };
                if let CodecZenohNetworkEnvelopeViewVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohOamView::decode_into(cursor, _b)?;
                }
            }
        }
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecZenohNetworkEnvelope`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder<D>(&self, _out: &mut CodecZenohNetworkEnvelopeIn<::sce_forge_runtime::codec::Lent<'a, D>>) -> Result<(), CodecError> where D: CodecZenohNetworkEnvelopeLentStorage {
        match &self.body {
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohInterest(_b) => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohInterest(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohInterest(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponseFinal(_b) => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponseFinal(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponseFinal(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohResponse(_b) => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponse(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponse(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohRequest(_b) => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohRequest(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohRequest(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohPush(_b) => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohPush(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohPush(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohDeclare(_b) => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohDeclare(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohDeclare(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeViewVariant::CodecZenohOam(_b) => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::CodecZenohOam(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohOam(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeViewVariant::Default { tag, body } => {
                _out.body = CodecZenohNetworkEnvelopeVariantIn::Default {
                    tag: *tag,
                    body: Default::default(),
                };
                if let CodecZenohNetworkEnvelopeVariantIn::Default { body: _o, .. } = &mut _out.body {
                    body.into_builder(_o)?;
                }
            }
        }
        Ok(())
    }


    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecZenohNetworkEnvelope::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecZenohNetworkEnvelopeWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::new();
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}

// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohNetworkEnvelope<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohNetworkEnvelopeOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohNetworkEnvelopeOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohNetworkEnvelopeOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohNetworkEnvelopeOwned` alone is the build's
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
// `let v: CodecZenohNetworkEnvelopeOwned = CodecZenohNetworkEnvelopeOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a, D> CodecZenohNetworkEnvelopeVariantIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohNetworkEnvelopeLentStorage {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: CodecZenohNetworkEnvelopeFrom<'a>>(self) -> Result<CodecZenohNetworkEnvelopeOwnedVariant<S>, CodecError> {
        Ok(match self {
            Self::CodecZenohInterest(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohInterest(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohResponseFinal(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponseFinal(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohResponse(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponse(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohRequest(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohRequest(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohPush(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohPush(_b),
            Self::CodecZenohDeclare(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohDeclare(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohOam(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohOam(_b.try_into_owned_in::<S>()?),
            Self::Default { tag, body } => CodecZenohNetworkEnvelopeOwnedVariant::Default { tag, body: body.try_into_owned_in::<S>()? },
        })
    }
}

#[allow(dead_code)]
impl<S: CodecZenohNetworkEnvelopeStorage> CodecZenohNetworkEnvelopeOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohNetworkEnvelopeVariantIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
        let mut _out = Default::default();
        self.try_as_borrowed_into(&mut _out)?;
        Ok(_out)
    }

    /// [`Self::try_as_borrowed`] into a destination the caller
    /// owns: the active arm is installed in `_out` holding its body's default
    /// and that body is projected where it sits, so the arm body is never
    /// returned up through the variant wrapper.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through. A caller on a path where that shows projects into a
    /// destination it owns. On `Err`, `_out` holds what was projected so far.
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohNetworkEnvelopeVariantIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        match self {
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohInterest(_b) => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::CodecZenohInterest(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohInterest(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponseFinal(_b) => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponseFinal(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponseFinal(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponse(_b) => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponse(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohResponse(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohRequest(_b) => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::CodecZenohRequest(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohRequest(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohPush(_b) => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::CodecZenohPush(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohPush(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohDeclare(_b) => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::CodecZenohDeclare(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohDeclare(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohOam(_b) => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::CodecZenohOam(Default::default());
                if let CodecZenohNetworkEnvelopeVariantIn::CodecZenohOam(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohNetworkEnvelopeOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohNetworkEnvelopeVariantIn::Default { tag: *tag, body: Default::default() };
                if let CodecZenohNetworkEnvelopeVariantIn::Default { body: _o, .. } = _out {
                    body.try_as_borrowed_into(_o)?;
                }
            }
        }
        Ok(())
    }
}

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohNetworkEnvelope<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohNetworkEnvelopeIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohNetworkEnvelopeLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohNetworkEnvelopeOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohNetworkEnvelopeFrom<'a>>(self) -> Result<CodecZenohNetworkEnvelopeOwned<S>, CodecError> {
        Ok(CodecZenohNetworkEnvelopeOwned {
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    // No projection at the build's default profile: on the heap-free tier
    // that is `Inline`, which holds no field whose protocol states no bound
    // (docs/adr/0077 §7). Name the profile, `try_into_owned_in::<S>()`.
}

#[allow(dead_code)]
impl<S: CodecZenohNetworkEnvelopeStorage> CodecZenohNetworkEnvelopeOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohNetworkEnvelopeIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohNetworkEnvelopeIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        self.body.try_as_borrowed_into(&mut _out.body)?;
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
    pub fn transcode_in<D: for<'x> CodecZenohNetworkEnvelopeFrom<'x>>(&self) -> Result<CodecZenohNetworkEnvelopeOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
