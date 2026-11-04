#![doc = "SCE-MAP: codec_zenoh_response:76 :: _forge_body"]
// SCE-MAP: codec_zenoh_response:76 :: _forge_body

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

use sce_forge_runtime::codec::{Bounded, ListView, ListWalk};
use super::codec_zenoh_ext_entry::CodecZenohExtEntryView;
use super::codec_zenoh_reply::CodecZenohReplyView;
use super::codec_zenoh_err::CodecZenohErrView;

/// A tag `CodecZenohResponse` names no arm for, which its default arm holds: a
/// value inside the 5 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecZenohResponseUnknownTag(u8);

#[allow(dead_code)]
impl CodecZenohResponseUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            4..=5 => None,
            _ if tag > 31 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecZenohResponseUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(31, &[(4, 5)]);

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
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
use super::codec_zenoh_reply::CodecZenohReplyOwned;
use super::codec_zenoh_err::CodecZenohErrOwned;
/// The field `suffix` of `CodecZenohResponse`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohResponseSuffixField;

impl ::sce_forge_runtime::codec::HoldsStr<CodecZenohResponseSuffixField> for ::sce_forge_runtime::codec::Inline {
    type Str = ::sce_forge_runtime::codec::InlineStr<65535>;
}

/// The field `extensions` of `CodecZenohResponse`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohResponseExtensionsField;

// `Inline` does not answer for `extensions`: the protocol states no bound of
// it, so it is held only by a profile whose container grows, or by a deployment
// that gives it a capacity (docs/adr/0077 §7).

/// What `CodecZenohResponseIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohResponseStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsStr<CodecZenohResponseSuffixField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohResponseExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage + super::codec_zenoh_reply::CodecZenohReplyStorage + super::codec_zenoh_err::CodecZenohErrStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsStr<CodecZenohResponseSuffixField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohResponseExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage + super::codec_zenoh_reply::CodecZenohReplyStorage + super::codec_zenoh_err::CodecZenohErrStorage> CodecZenohResponseStorage for S {}

/// What `CodecZenohResponseIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohResponse` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohResponseLentStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohResponseExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryLentStorage + super::codec_zenoh_reply::CodecZenohReplyLentStorage + super::codec_zenoh_err::CodecZenohErrLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohResponseExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryLentStorage + super::codec_zenoh_reply::CodecZenohReplyLentStorage + super::codec_zenoh_err::CodecZenohErrLentStorage> CodecZenohResponseLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohResponseStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohResponseFrom<'f>: CodecZenohResponseStorage + ::sce_forge_runtime::codec::StoresStr<'f, CodecZenohResponseSuffixField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f> + super::codec_zenoh_reply::CodecZenohReplyFrom<'f> + super::codec_zenoh_err::CodecZenohErrFrom<'f> {}

impl<'f, S: CodecZenohResponseStorage + ::sce_forge_runtime::codec::StoresStr<'f, CodecZenohResponseSuffixField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f> + super::codec_zenoh_reply::CodecZenohReplyFrom<'f> + super::codec_zenoh_err::CodecZenohErrFrom<'f>> CodecZenohResponseFrom<'f> for S {}

// `CodecZenohResponseIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohResponse<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohResponseOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohResponseIn<S: CodecZenohResponseStorage> {
    pub header: u8,
    pub request_id: u64,
    pub key_id: u32,
    pub suffix_len: Option<u64>,
    pub suffix: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsStr<CodecZenohResponseSuffixField>>::Str, 65535>>,
    pub extensions: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecZenohResponseExtensionsField>>::List<CodecZenohExtEntryOwned<S>>, { usize::MAX }, 1>>,
    pub body: CodecZenohResponseVariantIn<S>,
}

// No eager form at the default profile: `Lent<'a>` holds each list as `Inline`
// does, and `Inline` holds no list whose protocol states no bound
// (docs/adr/0077 §7). Name the eager form at a profile that holds its lists,
// `CodecZenohResponseIn<Lent<'a, D>>`.

/// The value at a profile that holds its bytes and text, named
/// with the profile: the build's default on the heap-free tier is `Inline`, which
/// holds no field whose protocol states no bound (docs/adr/0077 §7).
#[allow(dead_code)]
pub type CodecZenohResponseOwned<S> = CodecZenohResponseIn<S>;

impl<S: CodecZenohResponseStorage> Default for CodecZenohResponseIn<S> {
    fn default() -> Self {
        Self {
            header: 0x1bu8,
            request_id: Default::default(),
            key_id: Default::default(),
            suffix_len: Default::default(),
            suffix: Default::default(),
            extensions: Default::default(),
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
pub enum CodecZenohResponseVariantIn<S: CodecZenohResponseStorage> {
    CodecZenohReply(CodecZenohReplyOwned<S>),
    CodecZenohErr(CodecZenohErrOwned<S>),
    Default {
        tag: CodecZenohResponseUnknownTag,
        body: CodecZenohReplyOwned<S>,
    },
}


#[allow(dead_code)]
pub type CodecZenohResponseOwnedVariant<S> = CodecZenohResponseVariantIn<S>;

impl<S: CodecZenohResponseStorage> Default for CodecZenohResponseVariantIn<S> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohReply(CodecZenohReplyOwned::<S>::default())
    }
}

#[allow(dead_code)]
impl<S: CodecZenohResponseStorage> CodecZenohResponseIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohResponseFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohResponseFrom<'f> {
        // A field past what the profile holds is read on and not held; the
        // frame says so once it is read whole (docs/adr/0077 §7).
        let mut _over = false;
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for the variant
        // tag without advancing — the arm body decoder reads the peeked
        // byte as its own header byte (Zenoh response/request body MID
        // dispatch shape per network.c:347-364 + 220-235).
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let request_id = cursor.read_vle_u64()?;
        let key_id = cursor.read_vle_u32()?;
        let suffix_len = if (header & 0x20u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let suffix = if (header & 0x20u8) != 0 {
            let _len = suffix_len.unwrap();
            if _len > 65535 {
                return Err(CodecError::LengthOutOfRange);
            }
            let _n = _len as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = core::str::from_utf8(raw)
                .map_err(|_| CodecError::InvalidUtf8)?;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        let suffix = suffix.map(|_v| ::sce_forge_runtime::codec::Bounded::new(_v).ok_or(CodecError::LengthOutOfRange)).transpose()?;
        let suffix = ::sce_forge_runtime::codec::note_held(suffix.map(|_v| _v.store::<S, CodecZenohResponseSuffixField>()).transpose(), &mut _over)?;
        if (header & 0x80u8) != 0 {
            ::sce_forge_runtime::codec::note_capacity(_out.extensions.insert(Default::default()).refill(|_vec| {
                let mut _full = false;
                let mut _more = false;
                loop {
                    let mut _entry = CodecZenohExtEntryOwned::<S>::default();
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohExtEntryOwned::<S>::decode_into(cursor, &mut _entry), &mut _full)?;
                    _more = _entry.z();
                    ::sce_forge_runtime::codec::push_keeping(_vec, _entry, &[], &mut _full);
                    if !_more { break; }
                }
                if _more && cursor.remaining() == 0 {
                    return Err(CodecError::NeedMoreBytes);
                }
                if _full {
                    return Err(CodecError::CapacityExceeded);
                }
                Ok(())
            }), &mut _over)?;
        } else {
            _out.extensions = None;
        }
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match _peek & 0x1F {
            4u8 => {
                _out.body = CodecZenohResponseVariantIn::CodecZenohReply(CodecZenohReplyOwned::<S>::default());
                if let CodecZenohResponseVariantIn::CodecZenohReply(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohReplyOwned::<S>::decode_into(cursor, _b), &mut _over)?;
                }
            }
            5u8 => {
                _out.body = CodecZenohResponseVariantIn::CodecZenohErr(CodecZenohErrOwned::<S>::default());
                if let CodecZenohResponseVariantIn::CodecZenohErr(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohErrOwned::<S>::decode_into(cursor, _b), &mut _over)?;
                }
            }
            other => {
                _out.body = CodecZenohResponseVariantIn::Default {
                    tag: CodecZenohResponseUnknownTag(other),
                    body: CodecZenohReplyOwned::<S>::default(),
                };
                if let CodecZenohResponseVariantIn::Default { body: _b, .. } = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohReplyOwned::<S>::decode_into(cursor, _b), &mut _over)?;
                }
            }
        }
        _out.header = header;
        _out.request_id = request_id;
        _out.key_id = key_id;
        _out.suffix_len = suffix_len;
        _out.suffix = suffix;
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


    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohResponseWire::encode_source(self, w)
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

/// The arm of `CodecZenohResponse`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohResponse` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohResponseVariantSource<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> {
    CodecZenohReply(&'s A0),
    CodecZenohErr(&'s A1),
    Default { tag: CodecZenohResponseUnknownTag, body: &'s D },
}

impl<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> CodecZenohResponseVariantSource<'s, A0, A1, D> {
    /// The tag of the arm this is: the value the document gives a known
    /// arm, and the tag a default arm holds. It is the tag the encode
    /// writes, or the one a caller hands the decode for a variant whose
    /// wire form carries none.
    pub fn tag(&self) -> u8 {
        match self {
            Self::CodecZenohReply(_) => 4,
            Self::CodecZenohErr(_) => 5,
            Self::Default { tag, .. } => tag.get(),
        }
    }
}

/// Flags of `CodecZenohResponse` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecZenohResponse`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecZenohResponseOverrides {
    /// The `M` flag of `header`, when a parent sets it.
    pub header_m: Option<bool>,
}

#[allow(dead_code)]
impl CodecZenohResponseOverrides {
    /// Sets `M` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_m(value: bool) -> Self {
        Self { header_m: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohResponse` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohResponseWire::encode_source`]).
pub trait CodecZenohResponseSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `request_id` field.
    fn request_id(&self) -> u64;
    /// The `key_id` field.
    fn key_id(&self) -> u32;
    /// The `suffix` field.
    fn suffix(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 65535>>;
    /// The `extensions` field.
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>>;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource>;
}

impl<T: CodecZenohResponseSource + ?Sized> CodecZenohResponseSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn request_id(&self) -> u64 {
        (**self).request_id()
    }
    fn key_id(&self) -> u32 {
        (**self).key_id()
    }
    fn suffix(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 65535>> {
        (**self).suffix()
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        (**self).extensions()
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohResponse`'s variant that CodecZenohResponseParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohResponsePartsBody<'a> {
    CodecZenohReply(&'a super::codec_zenoh_reply::CodecZenohReplyParts<'a>),
    CodecZenohErr(&'a super::codec_zenoh_err::CodecZenohErrParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecZenohResponseUnknownTag, body: &'a super::codec_zenoh_reply::CodecZenohReplyParts<'a> },
}

/// A value of `CodecZenohResponse` held in the types a caller already has: a source
/// ([`CodecZenohResponseSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohResponseParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `request_id` field.
    pub request_id: u64,
    /// The `key_id` field.
    pub key_id: u32,
    /// The `suffix` field.
    pub suffix: Option<::sce_forge_runtime::codec::Text<'a, 65535>>,
    /// The `extensions` field.
    pub extensions: Option<::sce_forge_runtime::codec::Bounded<&'a [super::codec_zenoh_ext_entry::CodecZenohExtEntryParts<'a>], { usize::MAX }, 1>>,
    /// The arm of the variant this value holds.
    pub body: CodecZenohResponsePartsBody<'a>,
}

impl<'a> CodecZenohResponseSource for CodecZenohResponseParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn request_id(&self) -> u64 {
        self.request_id
    }
    fn key_id(&self) -> u32 {
        self.key_id
    }
    fn suffix(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 65535>> {
        self.suffix
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        match &self.body {
            CodecZenohResponsePartsBody::CodecZenohReply(_b) => CodecZenohResponseVariantSource::CodecZenohReply(*_b),
            CodecZenohResponsePartsBody::CodecZenohErr(_b) => CodecZenohResponseVariantSource::CodecZenohErr(*_b),
            CodecZenohResponsePartsBody::Default { tag, body } => CodecZenohResponseVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<S: CodecZenohResponseStorage> CodecZenohResponseSource for CodecZenohResponseOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn request_id(&self) -> u64 {
        self.request_id
    }
    fn key_id(&self) -> u32 {
        self.key_id
    }
    fn suffix(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 65535>> {
        self.suffix.as_ref().map(::sce_forge_runtime::codec::Bounded::as_text)
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        self.extensions.as_ref().map(|_l| _l.list_iter())
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        match &self.body {
            CodecZenohResponseOwnedVariant::CodecZenohReply(_b) => CodecZenohResponseVariantSource::CodecZenohReply(_b),
            CodecZenohResponseOwnedVariant::CodecZenohErr(_b) => CodecZenohResponseVariantSource::CodecZenohErr(_b),
            CodecZenohResponseOwnedVariant::Default { tag, body } => CodecZenohResponseVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<'a> CodecZenohResponseSource for CodecZenohResponseView<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn request_id(&self) -> u64 {
        self.request_id
    }
    fn key_id(&self) -> u32 {
        self.key_id
    }
    fn suffix(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 65535>> {
        self.suffix
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        match &self.body {
            CodecZenohResponseViewVariant::CodecZenohReply(_b) => CodecZenohResponseVariantSource::CodecZenohReply(_b),
            CodecZenohResponseViewVariant::CodecZenohErr(_b) => CodecZenohResponseVariantSource::CodecZenohErr(_b),
            CodecZenohResponseViewVariant::Default { tag, body } => CodecZenohResponseVariantSource::Default { tag: *tag, body },
        }
    }
}

/// The codec `CodecZenohResponse` itself: what writes a value read through
/// [`CodecZenohResponseSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohResponseWire;

#[allow(dead_code)]
impl CodecZenohResponseWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohResponse::encode` writes for the value it holds, read through
    /// [`CodecZenohResponseSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohResponseSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The tag of the arm `src` holds ([`CodecZenohResponseVariantSource::tag`]).
    pub fn tag_of<S: CodecZenohResponseSource + ?Sized>(src: &S) -> u8 {
        src.body().tag()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohResponseSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source_in(src, w, &CodecZenohResponseOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecZenohResponseSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecZenohResponseOverrides) -> Result<(), K::Error> {
        let _k_extensions: usize = src.extensions().map_or(0, |_l| _l.len());
        let _p_header_n_0: bool = src.suffix().is_some();
        let _p_header_z_0: bool = src.extensions().is_some();
        let _n_d_suffix_len_0: usize = match src.suffix() {
            Some(_v) => _v.len(),
            None => 0,
        };
        let _f_d_suffix_len: Option<usize> = if src.suffix().is_some() { Some(_n_d_suffix_len_0) } else { None };
        let _d_suffix_len: u64 = ::sce_forge_runtime::codec::widen_usize(_f_d_suffix_len.unwrap_or(0));
        let mut _c_header: u8 = (((src.header() & !0x20) | (u8::from(_p_header_n_0) << 5)) & !0x80) | (u8::from(_p_header_z_0) << 7);
        if let Some(_o) = over.header_m {
            _c_header = (_c_header & !0x40) | (u8::from(_o) << 6);
        }
        w.write_u8(_c_header)?;
        w.write_vle_u64(src.request_id())?;
        w.write_vle_u32(src.key_id())?;
        if (_c_header & 0x20) != 0 {
            w.write_vle_u64(_d_suffix_len)?;
        }
        if let Some(_v) = src.suffix() {
            w.write_bytes(_v.into_inner().as_bytes())?;
        }
        if let Some(_v) = src.extensions() {
            for (_i, _e) in _v.enumerate() {
                super::codec_zenoh_ext_entry::CodecZenohExtEntryWire::write_source_in(&_e, w, &super::codec_zenoh_ext_entry::CodecZenohExtEntryOverrides::header_z(_i + 1 < _k_extensions))?;
            }
        }
        match src.body() {
            CodecZenohResponseVariantSource::CodecZenohReply(_b) => super::codec_zenoh_reply::CodecZenohReplyWire::write_source(_b, w)?,
            CodecZenohResponseVariantSource::CodecZenohErr(_b) => super::codec_zenoh_err::CodecZenohErrWire::write_source(_b, w)?,
            CodecZenohResponseVariantSource::Default { body: _b, .. } => super::codec_zenoh_reply::CodecZenohReplyWire::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohResponseSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohResponseSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohResponseWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohResponseWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohResponseSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohResponseSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecZenohResponseView` is what a decode of `CodecZenohResponse` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecZenohResponse` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecZenohResponse::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohResponseViewVariant<'a> {
    CodecZenohReply(CodecZenohReplyView<'a>),
    CodecZenohErr(CodecZenohErrView<'a>),
    Default {
        tag: CodecZenohResponseUnknownTag,
        body: CodecZenohReplyView<'a>,
    },
}

impl<'a> Default for CodecZenohResponseViewVariant<'a> {
    fn default() -> Self {
        Self::CodecZenohReply(CodecZenohReplyView::default())
    }
}

#[allow(dead_code)]
pub struct CodecZenohResponseExtensionsWalk;

impl ListWalk<0> for CodecZenohResponseExtensionsWalk {
    type Entry<'b> = CodecZenohExtEntryView<'b>;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 0]) -> Result<Self::Entry<'b>, CodecError> {
        CodecZenohExtEntryView::decode(_c)
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 0]) {  }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohResponseView<'a> {
    pub header: u8,
    pub request_id: u64,
    pub key_id: u32,
    pub suffix_len: Option<u64>,
    pub suffix: Option<::sce_forge_runtime::codec::Text<'a, 65535>>,
    pub extensions: Option<Bounded<ListView<'a, CodecZenohResponseExtensionsWalk, 0>, { usize::MAX }, 1>>,
    pub body: CodecZenohResponseViewVariant<'a>,
}

impl<'a> Default for CodecZenohResponseView<'a> {
    fn default() -> Self {
        Self {
            header: 0x1bu8,
            request_id: Default::default(),
            key_id: Default::default(),
            suffix_len: Default::default(),
            suffix: Default::default(),
            extensions: Default::default(),
            body: CodecZenohResponseViewVariant::default(),
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohResponseView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecZenohResponseIn::decode`], held as a view. On success the
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
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let request_id = cursor.read_vle_u64()?;
        let key_id = cursor.read_vle_u32()?;
        let suffix_len = if (header & 0x20u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let suffix = if (header & 0x20u8) != 0 {
            let _len = suffix_len.unwrap();
            if _len > 65535 {
                return Err(CodecError::LengthOutOfRange);
            }
            let _n = _len as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = core::str::from_utf8(raw)
                .map_err(|_| CodecError::InvalidUtf8)?;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        let suffix = suffix.map(|_v| ::sce_forge_runtime::codec::Bounded::new(_v).ok_or(CodecError::LengthOutOfRange)).transpose()?;
        if (header & 0x80u8) != 0 {
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            let mut _more = false;
            loop {
                let _entry = CodecZenohExtEntryView::decode(cursor)?;
                _more = _entry.z();
                _count += 1;
                if !_more { break; }
            }
            if _more && cursor.remaining() == 0 {
                return Err(CodecError::NeedMoreBytes);
            }
            _out.extensions = Some(Bounded::new(ListView::new(_rest, _count, [])).ok_or(CodecError::TooManyElements)?);
        } else {
            _out.extensions = None;
        }
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match _peek & 0x1F {
            4u8 => {
                _out.body = CodecZenohResponseViewVariant::CodecZenohReply(CodecZenohReplyView::default());
                if let CodecZenohResponseViewVariant::CodecZenohReply(_b) = &mut _out.body {
                    CodecZenohReplyView::decode_into(cursor, _b)?;
                }
            }
            5u8 => {
                _out.body = CodecZenohResponseViewVariant::CodecZenohErr(CodecZenohErrView::default());
                if let CodecZenohResponseViewVariant::CodecZenohErr(_b) = &mut _out.body {
                    CodecZenohErrView::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohResponseViewVariant::Default {
                    tag: CodecZenohResponseUnknownTag(other),
                    body: CodecZenohReplyView::default(),
                };
                if let CodecZenohResponseViewVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohReplyView::decode_into(cursor, _b)?;
                }
            }
        }
        _out.header = header;
        _out.request_id = request_id;
        _out.key_id = key_id;
        _out.suffix_len = suffix_len;
        _out.suffix = suffix;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecZenohResponse`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder<D>(&self, _out: &mut CodecZenohResponseIn<::sce_forge_runtime::codec::Lent<'a, D>>) -> Result<(), CodecError> where D: CodecZenohResponseLentStorage {
        _out.header = self.header;
        _out.request_id = self.request_id;
        _out.key_id = self.key_id;
        _out.suffix_len = self.suffix_len;
        _out.suffix = self.suffix;
        match &self.extensions {
            Some(_l) => _out.extensions.insert(Default::default()).refill(|_vec| {
                    for _e in _l.iter() {
                        ::sce_forge_runtime::codec::SceList::try_push(_vec, Default::default())?;
                        if let Some(_slot) = ::sce_forge_runtime::codec::SceList::last_mut(_vec) {
                            _e.into_builder(_slot)?;
                        }
                    }
                    Ok(())
                })?,
            None => _out.extensions = None,
        }
        match &self.body {
            CodecZenohResponseViewVariant::CodecZenohReply(_b) => {
                _out.body = CodecZenohResponseVariantIn::CodecZenohReply(Default::default());
                if let CodecZenohResponseVariantIn::CodecZenohReply(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohResponseViewVariant::CodecZenohErr(_b) => {
                _out.body = CodecZenohResponseVariantIn::CodecZenohErr(Default::default());
                if let CodecZenohResponseVariantIn::CodecZenohErr(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohResponseViewVariant::Default { tag, body } => {
                _out.body = CodecZenohResponseVariantIn::Default {
                    tag: *tag,
                    body: Default::default(),
                };
                if let CodecZenohResponseVariantIn::Default { body: _o, .. } = &mut _out.body {
                    body.into_builder(_o)?;
                }
            }
        }
        Ok(())
    }


    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecZenohResponse::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecZenohResponseWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::new();
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
// `CodecZenohResponse<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohResponseOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohResponseOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohResponseOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohResponseOwned` alone is the build's
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
// `let v: CodecZenohResponseOwned = CodecZenohResponseOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a, D> CodecZenohResponseVariantIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohResponseLentStorage {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: CodecZenohResponseFrom<'a>>(self) -> Result<CodecZenohResponseOwnedVariant<S>, CodecError> {
        Ok(match self {
            Self::CodecZenohReply(_b) => CodecZenohResponseOwnedVariant::CodecZenohReply(_b.try_into_owned_in::<S>()?),
            Self::CodecZenohErr(_b) => CodecZenohResponseOwnedVariant::CodecZenohErr(_b.try_into_owned_in::<S>()?),
            Self::Default { tag, body } => CodecZenohResponseOwnedVariant::Default { tag, body: body.try_into_owned_in::<S>()? },
        })
    }
}

#[allow(dead_code)]
impl<S: CodecZenohResponseStorage> CodecZenohResponseOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohResponseVariantIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohResponseVariantIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        match self {
            CodecZenohResponseOwnedVariant::CodecZenohReply(_b) => {
                *_out = CodecZenohResponseVariantIn::CodecZenohReply(Default::default());
                if let CodecZenohResponseVariantIn::CodecZenohReply(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohResponseOwnedVariant::CodecZenohErr(_b) => {
                *_out = CodecZenohResponseVariantIn::CodecZenohErr(Default::default());
                if let CodecZenohResponseVariantIn::CodecZenohErr(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohResponseOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohResponseVariantIn::Default { tag: *tag, body: Default::default() };
                if let CodecZenohResponseVariantIn::Default { body: _o, .. } = _out {
                    body.try_as_borrowed_into(_o)?;
                }
            }
        }
        Ok(())
    }
}

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohResponse<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohResponseIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohResponseLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohResponseOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohResponseFrom<'a>>(self) -> Result<CodecZenohResponseOwned<S>, CodecError> {
        Ok(CodecZenohResponseOwned {
            header: self.header,
            request_id: self.request_id,
            key_id: self.key_id,
            suffix_len: self.suffix_len,
            suffix: self.suffix.map(|_v| _v.store::<S, CodecZenohResponseSuffixField>()).transpose()?,
            extensions: self.extensions.map(|_v| ::sce_forge_runtime::codec::try_collect_bounded(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    // No projection at the build's default profile: on the heap-free tier
    // that is `Inline`, which holds no field whose protocol states no bound
    // (docs/adr/0077 §7). Name the profile, `try_into_owned_in::<S>()`.
}

#[allow(dead_code)]
impl<S: CodecZenohResponseStorage> CodecZenohResponseOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohResponseIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohResponseIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        _out.header = self.header;
        _out.request_id = self.request_id;
        _out.key_id = self.key_id;
        _out.suffix_len = self.suffix_len;
        _out.suffix = self.suffix.as_ref().map(::sce_forge_runtime::codec::Bounded::as_text);
        match &self.extensions {
            Some(_l) => _out.extensions.insert(Default::default()).refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, _l.inner(), |_e| Ok(_e.as_borrowed())))?,
            None => _out.extensions = None,
        }
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
    pub fn transcode_in<D: for<'x> CodecZenohResponseFrom<'x>>(&self) -> Result<CodecZenohResponseOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
