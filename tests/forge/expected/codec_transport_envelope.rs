#![doc = "SCE-MAP: codec_transport_envelope:68 :: _forge_body"]
// SCE-MAP: codec_transport_envelope:68 :: _forge_body

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

use super::codec_zenoh_init_body::CodecZenohInitBody;
use super::codec_zenoh_open_body::CodecZenohOpenBody;
use super::codec_zenoh_close::CodecZenohClose;
use super::codec_zenoh_keep_alive::CodecZenohKeepAlive;
use super::codec_zenoh_frame::CodecZenohFrame;
use super::codec_zenoh_fragment::CodecZenohFragment;
use super::codec_zenoh_join::CodecZenohJoin;

/// A tag `CodecTransportEnvelope` names no arm for, which its default arm holds: a
/// value inside the 5 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecTransportEnvelopeUnknownTag(u8);

#[allow(dead_code)]
impl CodecTransportEnvelopeUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            1..=7 => None,
            _ if tag > 31 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecTransportEnvelopeUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(31, &[(1, 7)]);

    fn widened(self) -> u64 {
        u64::from(self.0)
    }

    fn from_admitted(tag: u64, _proof: ::sce_forge_runtime::codec::Admitted) -> Self {
        Self(tag as u8)
    }
}

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecTransportEnvelopeVariant<'a> {
    CodecZenohInitBody(CodecZenohInitBody<'a>),
    CodecZenohOpenBody(CodecZenohOpenBody<'a>),
    CodecZenohClose(CodecZenohClose),
    CodecZenohKeepAlive(CodecZenohKeepAlive),
    CodecZenohFrame(CodecZenohFrame<'a>),
    CodecZenohFragment(CodecZenohFragment<'a>),
    CodecZenohJoin(CodecZenohJoin<'a>),
    Default {
        tag: CodecTransportEnvelopeUnknownTag,
        body: CodecZenohClose,
    },
}

impl<'a> Default for CodecTransportEnvelopeVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohClose(CodecZenohClose::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecTransportEnvelope<'a> {
    pub header: u8,
    pub body: CodecTransportEnvelopeVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecTransportEnvelope<'a> {
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
            1u8 => {
                _out.body = CodecTransportEnvelopeVariant::CodecZenohInitBody(CodecZenohInitBody::default());
                if let CodecTransportEnvelopeVariant::CodecZenohInitBody(_b) = &mut _out.body {
                    CodecZenohInitBody::decode_into(cursor, _b, (header >> 6) & 0x1, (header >> 5) & 0x1)?;
                }
            }
            2u8 => {
                _out.body = CodecTransportEnvelopeVariant::CodecZenohOpenBody(CodecZenohOpenBody::default());
                if let CodecTransportEnvelopeVariant::CodecZenohOpenBody(_b) = &mut _out.body {
                    CodecZenohOpenBody::decode_into(cursor, _b, (header >> 5) & 0x1)?;
                }
            }
            3u8 => {
                _out.body = CodecTransportEnvelopeVariant::CodecZenohClose(CodecZenohClose::default());
                if let CodecTransportEnvelopeVariant::CodecZenohClose(_b) = &mut _out.body {
                    CodecZenohClose::decode_into(cursor, _b)?;
                }
            }
            4u8 => {
                _out.body = CodecTransportEnvelopeVariant::CodecZenohKeepAlive(CodecZenohKeepAlive::default());
                if let CodecTransportEnvelopeVariant::CodecZenohKeepAlive(_b) = &mut _out.body {
                    CodecZenohKeepAlive::decode_into(cursor, _b)?;
                }
            }
            5u8 => {
                _out.body = CodecTransportEnvelopeVariant::CodecZenohFrame(CodecZenohFrame::default());
                if let CodecTransportEnvelopeVariant::CodecZenohFrame(_b) = &mut _out.body {
                    CodecZenohFrame::decode_into(cursor, _b)?;
                }
            }
            6u8 => {
                _out.body = CodecTransportEnvelopeVariant::CodecZenohFragment(CodecZenohFragment::default());
                if let CodecTransportEnvelopeVariant::CodecZenohFragment(_b) = &mut _out.body {
                    CodecZenohFragment::decode_into(cursor, _b)?;
                }
            }
            7u8 => {
                _out.body = CodecTransportEnvelopeVariant::CodecZenohJoin(CodecZenohJoin::default());
                if let CodecTransportEnvelopeVariant::CodecZenohJoin(_b) = &mut _out.body {
                    CodecZenohJoin::decode_into(cursor, _b, (header >> 6) & 0x1)?;
                }
            }
            other => {
                _out.body = CodecTransportEnvelopeVariant::Default {
                    tag: CodecTransportEnvelopeUnknownTag(other),
                    body: CodecZenohClose::default(),
                };
                if let CodecTransportEnvelopeVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohClose::decode_into(cursor, _b)?;
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

    pub fn a(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_a(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn s(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_s(&mut self, v: bool) {
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
    pub const MAX_ENCODED_BYTES: usize = 65546;

    /// Encode `self` into the caller-owned sink. Returns the value's refusal
    /// when the encoding cannot carry it (a length past its type, a list past
    /// its bound, a gate that disagrees with a presence; the document declares
    /// `sce:encode-may-fail`), or the sink's own error:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> where CodecError: From<S::Error> {
        Self::encode_source(self, w)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector, or the error
    /// `encode` refuses a value with: one the encoding cannot carry (a
    /// length past its type, a list past its bound) or one a gate
    /// disagrees with. Callers targeting zero-alloc hot paths should call
    /// `encode` directly against a caller-owned sink.
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

/// The arm of `CodecTransportEnvelope`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecTransportEnvelope` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecTransportEnvelopeVariantSource<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, A3: ?Sized, A4: ?Sized, A5: ?Sized, A6: ?Sized, D: ?Sized> {
    CodecZenohInitBody(&'s A0),
    CodecZenohOpenBody(&'s A1),
    CodecZenohClose(&'s A2),
    CodecZenohKeepAlive(&'s A3),
    CodecZenohFrame(&'s A4),
    CodecZenohFragment(&'s A5),
    CodecZenohJoin(&'s A6),
    Default { tag: CodecTransportEnvelopeUnknownTag, body: &'s D },
}

/// Flags of `CodecTransportEnvelope` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecTransportEnvelope`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecTransportEnvelopeOverrides {
    /// The `A` flag of `header`, when a parent sets it.
    pub header_a: Option<bool>,
    /// The `S` flag of `header`, when a parent sets it.
    pub header_s: Option<bool>,
    /// The `Z` flag of `header`, when a parent sets it.
    pub header_z: Option<bool>,
}

#[allow(dead_code)]
impl CodecTransportEnvelopeOverrides {
    /// Sets `A` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_a(value: bool) -> Self {
        Self { header_a: Some(value), ..Self::default() }
    }

    /// Sets `S` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_s(value: bool) -> Self {
        Self { header_s: Some(value), ..Self::default() }
    }

    /// Sets `Z` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_z(value: bool) -> Self {
        Self { header_z: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecTransportEnvelope` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecTransportEnvelope`
/// computes those from what is ([`CodecTransportEnvelope::encode_source`]).
pub trait CodecTransportEnvelopeSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecTransportEnvelopeVariantSource<'_, impl super::codec_zenoh_init_body::CodecZenohInitBodySource, impl super::codec_zenoh_open_body::CodecZenohOpenBodySource, impl super::codec_zenoh_close::CodecZenohCloseSource, impl super::codec_zenoh_keep_alive::CodecZenohKeepAliveSource, impl super::codec_zenoh_frame::CodecZenohFrameSource, impl super::codec_zenoh_fragment::CodecZenohFragmentSource, impl super::codec_zenoh_join::CodecZenohJoinSource, impl super::codec_zenoh_close::CodecZenohCloseSource>;
}

impl<T: CodecTransportEnvelopeSource + ?Sized> CodecTransportEnvelopeSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn body(&self) -> CodecTransportEnvelopeVariantSource<'_, impl super::codec_zenoh_init_body::CodecZenohInitBodySource, impl super::codec_zenoh_open_body::CodecZenohOpenBodySource, impl super::codec_zenoh_close::CodecZenohCloseSource, impl super::codec_zenoh_keep_alive::CodecZenohKeepAliveSource, impl super::codec_zenoh_frame::CodecZenohFrameSource, impl super::codec_zenoh_fragment::CodecZenohFragmentSource, impl super::codec_zenoh_join::CodecZenohJoinSource, impl super::codec_zenoh_close::CodecZenohCloseSource> {
        (**self).body()
    }
}

/// The arm of `CodecTransportEnvelope`'s variant that CodecTransportEnvelopeParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecTransportEnvelopePartsBody<'a> {
    CodecZenohInitBody(&'a super::codec_zenoh_init_body::CodecZenohInitBodyParts<'a>),
    CodecZenohOpenBody(&'a super::codec_zenoh_open_body::CodecZenohOpenBodyParts<'a>),
    CodecZenohClose(&'a super::codec_zenoh_close::CodecZenohCloseParts),
    CodecZenohKeepAlive(&'a super::codec_zenoh_keep_alive::CodecZenohKeepAliveParts),
    CodecZenohFrame(&'a super::codec_zenoh_frame::CodecZenohFrameParts<'a>),
    CodecZenohFragment(&'a super::codec_zenoh_fragment::CodecZenohFragmentParts<'a>),
    CodecZenohJoin(&'a super::codec_zenoh_join::CodecZenohJoinParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecTransportEnvelopeUnknownTag, body: &'a super::codec_zenoh_close::CodecZenohCloseParts },
}

/// A value of `CodecTransportEnvelope` held in the types a caller already has: a source
/// ([`CodecTransportEnvelopeSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecTransportEnvelopeParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The arm of the variant this value holds.
    pub body: CodecTransportEnvelopePartsBody<'a>,
}

impl<'a> CodecTransportEnvelopeSource for CodecTransportEnvelopeParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecTransportEnvelopeVariantSource<'_, impl super::codec_zenoh_init_body::CodecZenohInitBodySource, impl super::codec_zenoh_open_body::CodecZenohOpenBodySource, impl super::codec_zenoh_close::CodecZenohCloseSource, impl super::codec_zenoh_keep_alive::CodecZenohKeepAliveSource, impl super::codec_zenoh_frame::CodecZenohFrameSource, impl super::codec_zenoh_fragment::CodecZenohFragmentSource, impl super::codec_zenoh_join::CodecZenohJoinSource, impl super::codec_zenoh_close::CodecZenohCloseSource> {
        match &self.body {
            CodecTransportEnvelopePartsBody::CodecZenohInitBody(_b) => CodecTransportEnvelopeVariantSource::CodecZenohInitBody(*_b),
            CodecTransportEnvelopePartsBody::CodecZenohOpenBody(_b) => CodecTransportEnvelopeVariantSource::CodecZenohOpenBody(*_b),
            CodecTransportEnvelopePartsBody::CodecZenohClose(_b) => CodecTransportEnvelopeVariantSource::CodecZenohClose(*_b),
            CodecTransportEnvelopePartsBody::CodecZenohKeepAlive(_b) => CodecTransportEnvelopeVariantSource::CodecZenohKeepAlive(*_b),
            CodecTransportEnvelopePartsBody::CodecZenohFrame(_b) => CodecTransportEnvelopeVariantSource::CodecZenohFrame(*_b),
            CodecTransportEnvelopePartsBody::CodecZenohFragment(_b) => CodecTransportEnvelopeVariantSource::CodecZenohFragment(*_b),
            CodecTransportEnvelopePartsBody::CodecZenohJoin(_b) => CodecTransportEnvelopeVariantSource::CodecZenohJoin(*_b),
            CodecTransportEnvelopePartsBody::Default { tag, body } => CodecTransportEnvelopeVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<'a> CodecTransportEnvelopeSource for CodecTransportEnvelope<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecTransportEnvelopeVariantSource<'_, impl super::codec_zenoh_init_body::CodecZenohInitBodySource, impl super::codec_zenoh_open_body::CodecZenohOpenBodySource, impl super::codec_zenoh_close::CodecZenohCloseSource, impl super::codec_zenoh_keep_alive::CodecZenohKeepAliveSource, impl super::codec_zenoh_frame::CodecZenohFrameSource, impl super::codec_zenoh_fragment::CodecZenohFragmentSource, impl super::codec_zenoh_join::CodecZenohJoinSource, impl super::codec_zenoh_close::CodecZenohCloseSource> {
        match &self.body {
            CodecTransportEnvelopeVariant::CodecZenohInitBody(_b) => CodecTransportEnvelopeVariantSource::CodecZenohInitBody(_b),
            CodecTransportEnvelopeVariant::CodecZenohOpenBody(_b) => CodecTransportEnvelopeVariantSource::CodecZenohOpenBody(_b),
            CodecTransportEnvelopeVariant::CodecZenohClose(_b) => CodecTransportEnvelopeVariantSource::CodecZenohClose(_b),
            CodecTransportEnvelopeVariant::CodecZenohKeepAlive(_b) => CodecTransportEnvelopeVariantSource::CodecZenohKeepAlive(_b),
            CodecTransportEnvelopeVariant::CodecZenohFrame(_b) => CodecTransportEnvelopeVariantSource::CodecZenohFrame(_b),
            CodecTransportEnvelopeVariant::CodecZenohFragment(_b) => CodecTransportEnvelopeVariantSource::CodecZenohFragment(_b),
            CodecTransportEnvelopeVariant::CodecZenohJoin(_b) => CodecTransportEnvelopeVariantSource::CodecZenohJoin(_b),
            CodecTransportEnvelopeVariant::Default { tag, body } => CodecTransportEnvelopeVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTransportEnvelopeSource for CodecTransportEnvelopeOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecTransportEnvelopeVariantSource<'_, impl super::codec_zenoh_init_body::CodecZenohInitBodySource, impl super::codec_zenoh_open_body::CodecZenohOpenBodySource, impl super::codec_zenoh_close::CodecZenohCloseSource, impl super::codec_zenoh_keep_alive::CodecZenohKeepAliveSource, impl super::codec_zenoh_frame::CodecZenohFrameSource, impl super::codec_zenoh_fragment::CodecZenohFragmentSource, impl super::codec_zenoh_join::CodecZenohJoinSource, impl super::codec_zenoh_close::CodecZenohCloseSource> {
        match &self.body {
            CodecTransportEnvelopeOwnedVariant::CodecZenohInitBody(_b) => CodecTransportEnvelopeVariantSource::CodecZenohInitBody(_b),
            CodecTransportEnvelopeOwnedVariant::CodecZenohOpenBody(_b) => CodecTransportEnvelopeVariantSource::CodecZenohOpenBody(_b),
            CodecTransportEnvelopeOwnedVariant::CodecZenohClose(_b) => CodecTransportEnvelopeVariantSource::CodecZenohClose(_b),
            CodecTransportEnvelopeOwnedVariant::CodecZenohKeepAlive(_b) => CodecTransportEnvelopeVariantSource::CodecZenohKeepAlive(_b),
            CodecTransportEnvelopeOwnedVariant::CodecZenohFrame(_b) => CodecTransportEnvelopeVariantSource::CodecZenohFrame(_b),
            CodecTransportEnvelopeOwnedVariant::CodecZenohFragment(_b) => CodecTransportEnvelopeVariantSource::CodecZenohFragment(_b),
            CodecTransportEnvelopeOwnedVariant::CodecZenohJoin(_b) => CodecTransportEnvelopeVariantSource::CodecZenohJoin(_b),
            CodecTransportEnvelopeOwnedVariant::Default { tag, body } => CodecTransportEnvelopeVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecTransportEnvelope<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecTransportEnvelopeSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecTransportEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::count_source(src)?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecTransportEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source_in(src, w, &CodecTransportEnvelopeOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecTransportEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecTransportEnvelopeOverrides) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let _d_header_mid: u8 = match src.body() {
            CodecTransportEnvelopeVariantSource::CodecZenohInitBody(_) => 1,
            CodecTransportEnvelopeVariantSource::CodecZenohOpenBody(_) => 2,
            CodecTransportEnvelopeVariantSource::CodecZenohClose(_) => 3,
            CodecTransportEnvelopeVariantSource::CodecZenohKeepAlive(_) => 4,
            CodecTransportEnvelopeVariantSource::CodecZenohFrame(_) => 5,
            CodecTransportEnvelopeVariantSource::CodecZenohFragment(_) => 6,
            CodecTransportEnvelopeVariantSource::CodecZenohJoin(_) => 7,
            CodecTransportEnvelopeVariantSource::Default { tag, .. } => tag.get(),
        };
        let mut _c_header: u8 = (src.header() & !0x1F) | _d_header_mid;
        if let Some(_o) = over.header_a {
            _c_header = (_c_header & !0x20) | (u8::from(_o) << 5);
        }
        if let Some(_o) = over.header_s {
            _c_header = (_c_header & !0x40) | (u8::from(_o) << 6);
        }
        if let Some(_o) = over.header_z {
            _c_header = (_c_header & !0x80) | (u8::from(_o) << 7);
        }
        w.write_u8(_c_header)?;
        match src.body() {
            CodecTransportEnvelopeVariantSource::CodecZenohInitBody(_b) => CodecZenohInitBody::write_source(_b, w, (_c_header >> 6) & 0x1, (_c_header >> 5) & 0x1)?,
            CodecTransportEnvelopeVariantSource::CodecZenohOpenBody(_b) => CodecZenohOpenBody::write_source(_b, w, (_c_header >> 5) & 0x1)?,
            CodecTransportEnvelopeVariantSource::CodecZenohClose(_b) => CodecZenohClose::write_source(_b, w)?,
            CodecTransportEnvelopeVariantSource::CodecZenohKeepAlive(_b) => CodecZenohKeepAlive::write_source(_b, w)?,
            CodecTransportEnvelopeVariantSource::CodecZenohFrame(_b) => CodecZenohFrame::write_source(_b, w)?,
            CodecTransportEnvelopeVariantSource::CodecZenohFragment(_b) => CodecZenohFragment::write_source(_b, w)?,
            CodecTransportEnvelopeVariantSource::CodecZenohJoin(_b) => CodecZenohJoin::write_source(_b, w, (_c_header >> 6) & 0x1)?,
            CodecTransportEnvelopeVariantSource::Default { body: _b, .. } => CodecZenohClose::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecTransportEnvelopeSource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<'a, S: CodecTransportEnvelopeSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecTransportEnvelope<'a> {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecTransportEnvelope<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecTransportEnvelopeSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecTransportEnvelopeSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecTransportEnvelope<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecTransportEnvelopeOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecTransportEnvelopeOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecTransportEnvelopeOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecTransportEnvelopeOwned` alone is the build's
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
// `let v: CodecTransportEnvelopeOwned = CodecTransportEnvelopeOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_init_body::CodecZenohInitBodyOwned;
use super::codec_zenoh_open_body::CodecZenohOpenBodyOwned;
use super::codec_zenoh_frame::CodecZenohFrameOwned;
use super::codec_zenoh_fragment::CodecZenohFragmentOwned;
use super::codec_zenoh_join::CodecZenohJoinOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecTransportEnvelopeOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub header: u8,
    pub body: CodecTransportEnvelopeOwnedVariant<S>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTransportEnvelopeOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn a(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn s(&self) -> bool {
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
pub enum CodecTransportEnvelopeOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecZenohInitBody(CodecZenohInitBodyOwned<S>),
    CodecZenohOpenBody(CodecZenohOpenBodyOwned<S>),
    CodecZenohClose(CodecZenohClose),
    CodecZenohKeepAlive(CodecZenohKeepAlive),
    CodecZenohFrame(CodecZenohFrameOwned<S>),
    CodecZenohFragment(CodecZenohFragmentOwned<S>),
    CodecZenohJoin(CodecZenohJoinOwned<S>),
    Default {
        tag: CodecTransportEnvelopeUnknownTag,
        body: CodecZenohClose,
    },
}

#[allow(dead_code)]
impl<'a> CodecTransportEnvelopeVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecTransportEnvelopeOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecTransportEnvelopeVariant::CodecZenohInitBody(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohInitBody(_b.try_into_owned_in::<S>()?),
            CodecTransportEnvelopeVariant::CodecZenohOpenBody(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohOpenBody(_b.try_into_owned_in::<S>()?),
            CodecTransportEnvelopeVariant::CodecZenohClose(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohClose(_b),
            CodecTransportEnvelopeVariant::CodecZenohKeepAlive(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohKeepAlive(_b),
            CodecTransportEnvelopeVariant::CodecZenohFrame(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohFrame(_b.try_into_owned_in::<S>()?),
            CodecTransportEnvelopeVariant::CodecZenohFragment(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohFragment(_b.try_into_owned_in::<S>()?),
            CodecTransportEnvelopeVariant::CodecZenohJoin(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohJoin(_b.try_into_owned_in::<S>()?),
            CodecTransportEnvelopeVariant::Default { tag, body } => CodecTransportEnvelopeOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecTransportEnvelopeOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTransportEnvelopeOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn as_borrowed(&self) -> CodecTransportEnvelopeVariant<'_> {
        let mut _out = CodecTransportEnvelopeVariant::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecTransportEnvelopeVariant<'o>) {
        match self {
            CodecTransportEnvelopeOwnedVariant::CodecZenohInitBody(_b) => {
                *_out = CodecTransportEnvelopeVariant::CodecZenohInitBody(Default::default());
                if let CodecTransportEnvelopeVariant::CodecZenohInitBody(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecTransportEnvelopeOwnedVariant::CodecZenohOpenBody(_b) => {
                *_out = CodecTransportEnvelopeVariant::CodecZenohOpenBody(Default::default());
                if let CodecTransportEnvelopeVariant::CodecZenohOpenBody(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecTransportEnvelopeOwnedVariant::CodecZenohClose(_b) => {
                *_out = CodecTransportEnvelopeVariant::CodecZenohClose(Default::default());
                if let CodecTransportEnvelopeVariant::CodecZenohClose(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecTransportEnvelopeOwnedVariant::CodecZenohKeepAlive(_b) => {
                *_out = CodecTransportEnvelopeVariant::CodecZenohKeepAlive(Default::default());
                if let CodecTransportEnvelopeVariant::CodecZenohKeepAlive(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecTransportEnvelopeOwnedVariant::CodecZenohFrame(_b) => {
                *_out = CodecTransportEnvelopeVariant::CodecZenohFrame(Default::default());
                if let CodecTransportEnvelopeVariant::CodecZenohFrame(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecTransportEnvelopeOwnedVariant::CodecZenohFragment(_b) => {
                *_out = CodecTransportEnvelopeVariant::CodecZenohFragment(Default::default());
                if let CodecTransportEnvelopeVariant::CodecZenohFragment(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecTransportEnvelopeOwnedVariant::CodecZenohJoin(_b) => {
                *_out = CodecTransportEnvelopeVariant::CodecZenohJoin(Default::default());
                if let CodecTransportEnvelopeVariant::CodecZenohJoin(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecTransportEnvelopeOwnedVariant::Default { tag, body } => {
                *_out = CodecTransportEnvelopeVariant::Default { tag: *tag, body: Default::default() };
                if let CodecTransportEnvelopeVariant::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        
    }
}

#[allow(dead_code)]
impl<'a> CodecTransportEnvelope<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecTransportEnvelopeOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecTransportEnvelopeOwned<S>, CodecError> {
        Ok(CodecTransportEnvelopeOwned {
            header: self.header,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecTransportEnvelopeOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTransportEnvelopeOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecTransportEnvelope<'_> {
        let mut _out = CodecTransportEnvelope::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecTransportEnvelope<'o>) {
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
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecTransportEnvelopeOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecTransportEnvelopeOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `as_borrowed` followed by
    /// [`CodecTransportEnvelope::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here.
    /// The same refusals as the projection's: a list longer than the borrowed
    /// view declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        CodecTransportEnvelope::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecTransportEnvelope::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
