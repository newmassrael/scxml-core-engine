#![doc = "SCE-MAP: codec_zenoh_value_entry:14 :: _forge_body"]
// SCE-MAP: codec_zenoh_value_entry:14 :: _forge_body

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

use super::codec_zenoh_ext_unit::CodecZenohExtUnit;
use super::codec_zenoh_ext_zint::CodecZenohExtZint;
use super::codec_zenoh_value_zbuf::CodecZenohValueZbuf;

/// A tag `CodecZenohValueEntry` names no arm for, which its default arm holds: a
/// value inside the 2 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecZenohValueEntryUnknownTag(u8);

#[allow(dead_code)]
impl CodecZenohValueEntryUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            0..=2 => None,
            _ if tag > 3 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecZenohValueEntryUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(3, &[(0, 2)]);

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
use super::codec_zenoh_value_zbuf::CodecZenohValueZbufOwned;
/// What `CodecZenohValueEntryIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohValueEntryStorage: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_value_zbuf::CodecZenohValueZbufStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + super::codec_zenoh_value_zbuf::CodecZenohValueZbufStorage> CodecZenohValueEntryStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohValueEntryStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohValueEntryFrom<'f>: CodecZenohValueEntryStorage + super::codec_zenoh_value_zbuf::CodecZenohValueZbufFrom<'f> {}

impl<'f, S: CodecZenohValueEntryStorage + super::codec_zenoh_value_zbuf::CodecZenohValueZbufFrom<'f>> CodecZenohValueEntryFrom<'f> for S {}

// `CodecZenohValueEntryIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohValueEntry<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohValueEntryOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohValueEntryIn<S: CodecZenohValueEntryStorage> {
    pub header: u8,
    pub body: CodecZenohValueEntryVariantIn<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohValueEntry<'a> = CodecZenohValueEntryIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohValueEntryOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohValueEntryIn<S>;

impl<S: CodecZenohValueEntryStorage> Default for CodecZenohValueEntryIn<S> {
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
pub enum CodecZenohValueEntryVariantIn<S: CodecZenohValueEntryStorage> {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohValueZbuf(CodecZenohValueZbufOwned<S>),
    Default {
        tag: CodecZenohValueEntryUnknownTag,
        body: CodecZenohExtUnit,
    },
}

#[allow(dead_code)]
pub type CodecZenohValueEntryVariant<'a> = CodecZenohValueEntryVariantIn<::sce_forge_runtime::codec::Lent<'a>>;

#[allow(dead_code)]
pub type CodecZenohValueEntryOwnedVariant<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohValueEntryVariantIn<S>;

impl<S: CodecZenohValueEntryStorage> Default for CodecZenohValueEntryVariantIn<S> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohExtUnit(CodecZenohExtUnit::default())
    }
}

#[allow(dead_code)]
impl<S: CodecZenohValueEntryStorage> CodecZenohValueEntryIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>, after_shm: u8) -> Result<Self, CodecError> where S: CodecZenohValueEntryFrom<'f> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, after_shm)?;
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, after_shm: u8) -> Result<(), CodecError> where S: CodecZenohValueEntryFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out, after_shm) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohValueEntry::store_into(&mut _frame, &mut CodecZenohValueEntry::default(), after_shm)?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, after_shm: u8) -> Result<(), CodecError> where S: CodecZenohValueEntryFrom<'f> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = after_shm;
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
        match (header >> 5) & 0x03 {
            0u8 => {
                _out.body = CodecZenohValueEntryVariantIn::CodecZenohExtUnit(CodecZenohExtUnit::default());
                if let CodecZenohValueEntryVariantIn::CodecZenohExtUnit(_b) = &mut _out.body {
                    CodecZenohExtUnit::decode_into(cursor, _b)?;
                }
            }
            1u8 => {
                _out.body = CodecZenohValueEntryVariantIn::CodecZenohExtZint(CodecZenohExtZint::default());
                if let CodecZenohValueEntryVariantIn::CodecZenohExtZint(_b) = &mut _out.body {
                    CodecZenohExtZint::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecZenohValueEntryVariantIn::CodecZenohValueZbuf(CodecZenohValueZbufOwned::<S>::default());
                if let CodecZenohValueEntryVariantIn::CodecZenohValueZbuf(_b) = &mut _out.body {
                    CodecZenohValueZbufOwned::<S>::decode_into(cursor, _b, after_shm)?;
                }
            }
            other => {
                _out.body = CodecZenohValueEntryVariantIn::Default {
                    tag: CodecZenohValueEntryUnknownTag(other),
                    body: CodecZenohExtUnit::default(),
                };
                if let CodecZenohValueEntryVariantIn::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohExtUnit::decode_into(cursor, _b)?;
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
    pub fn ext_id(&self) -> u8 {
        self.header & 0x0F
    }

    pub fn set_ext_id(&mut self, v: u8) {
        self.header = (self.header & !0x0F) | (v & 0x0F);
    }

    pub fn m(&self) -> bool {
        (self.header & 0x10) != 0
    }

    pub fn set_m(&mut self, v: bool) {
        if v {
            self.header |= 0x10;
        } else {
            self.header &= !0x10;
        }
    }

    pub fn enc(&self) -> u8 {
        (self.header >> 5) & 0x03
    }

    pub fn set_enc(&mut self, v: u8) {
        self.header = (self.header & !0x60) | ((v & 0x03) << 5);
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
    pub const MAX_ENCODED_BYTES: usize = 164;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohValueEntry::encode_source(self, w)
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

/// The arm of `CodecZenohValueEntry`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohValueEntry` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohValueEntryVariantSource<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, D: ?Sized> {
    CodecZenohExtUnit(&'s A0),
    CodecZenohExtZint(&'s A1),
    CodecZenohValueZbuf(&'s A2),
    Default { tag: CodecZenohValueEntryUnknownTag, body: &'s D },
}

impl<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, D: ?Sized> CodecZenohValueEntryVariantSource<'s, A0, A1, A2, D> {
    /// The tag of the arm this is: the value the document gives a known
    /// arm, and the tag a default arm holds. It is the tag the encode
    /// writes, or the one a caller hands the decode for a variant whose
    /// wire form carries none.
    pub fn tag(&self) -> u8 {
        match self {
            Self::CodecZenohExtUnit(_) => 0,
            Self::CodecZenohExtZint(_) => 1,
            Self::CodecZenohValueZbuf(_) => 2,
            Self::Default { tag, .. } => tag.get(),
        }
    }
}

/// Flags of `CodecZenohValueEntry` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecZenohValueEntry`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecZenohValueEntryOverrides {
    /// The `M` flag of `header`, when a parent sets it.
    pub header_m: Option<bool>,
    /// The `Z` flag of `header`, when a parent sets it.
    pub header_z: Option<bool>,
}

#[allow(dead_code)]
impl CodecZenohValueEntryOverrides {
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
/// What a value of `CodecZenohValueEntry` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohValueEntry`
/// computes those from what is ([`CodecZenohValueEntry::encode_source`]).
pub trait CodecZenohValueEntrySource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohValueEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_value_zbuf::CodecZenohValueZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource>;
}

impl<T: CodecZenohValueEntrySource + ?Sized> CodecZenohValueEntrySource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn body(&self) -> CodecZenohValueEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_value_zbuf::CodecZenohValueZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohValueEntry`'s variant that CodecZenohValueEntryParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohValueEntryPartsBody<'a> {
    CodecZenohExtUnit(&'a super::codec_zenoh_ext_unit::CodecZenohExtUnitParts),
    CodecZenohExtZint(&'a super::codec_zenoh_ext_zint::CodecZenohExtZintParts),
    CodecZenohValueZbuf(&'a super::codec_zenoh_value_zbuf::CodecZenohValueZbufParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecZenohValueEntryUnknownTag, body: &'a super::codec_zenoh_ext_unit::CodecZenohExtUnitParts },
}

/// A value of `CodecZenohValueEntry` held in the types a caller already has: a source
/// ([`CodecZenohValueEntrySource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohValueEntryParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The arm of the variant this value holds.
    pub body: CodecZenohValueEntryPartsBody<'a>,
}

impl<'a> CodecZenohValueEntrySource for CodecZenohValueEntryParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohValueEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_value_zbuf::CodecZenohValueZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohValueEntryPartsBody::CodecZenohExtUnit(_b) => CodecZenohValueEntryVariantSource::CodecZenohExtUnit(*_b),
            CodecZenohValueEntryPartsBody::CodecZenohExtZint(_b) => CodecZenohValueEntryVariantSource::CodecZenohExtZint(*_b),
            CodecZenohValueEntryPartsBody::CodecZenohValueZbuf(_b) => CodecZenohValueEntryVariantSource::CodecZenohValueZbuf(*_b),
            CodecZenohValueEntryPartsBody::Default { tag, body } => CodecZenohValueEntryVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<S: CodecZenohValueEntryStorage> CodecZenohValueEntrySource for CodecZenohValueEntryOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohValueEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_value_zbuf::CodecZenohValueZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohValueEntryOwnedVariant::CodecZenohExtUnit(_b) => CodecZenohValueEntryVariantSource::CodecZenohExtUnit(_b),
            CodecZenohValueEntryOwnedVariant::CodecZenohExtZint(_b) => CodecZenohValueEntryVariantSource::CodecZenohExtZint(_b),
            CodecZenohValueEntryOwnedVariant::CodecZenohValueZbuf(_b) => CodecZenohValueEntryVariantSource::CodecZenohValueZbuf(_b),
            CodecZenohValueEntryOwnedVariant::Default { tag, body } => CodecZenohValueEntryVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohValueEntry<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohValueEntrySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohValueEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The flag-input `after_shm` that `src` says: the input the arm it holds says, and `None` from an arm that does not read it.
    /// A parent hands the decode this bit and derives it from here for the
    /// encode.
    pub fn input_after_shm<S: CodecZenohValueEntrySource + ?Sized>(src: &S) -> Option<bool> {
        match CodecZenohValueEntrySource::body(src) { CodecZenohValueEntryVariantSource::CodecZenohValueZbuf(_b) => Some(CodecZenohValueZbuf::input_after_shm(_b)), _ => None, }
    }

    /// The tag of the arm `src` holds ([`CodecZenohValueEntryVariantSource::tag`]).
    pub fn tag_of<S: CodecZenohValueEntrySource + ?Sized>(src: &S) -> u8 {
        src.body().tag()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohValueEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source_in(src, w, &CodecZenohValueEntryOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecZenohValueEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecZenohValueEntryOverrides) -> Result<(), K::Error> {
        let _d_header_enc: u8 = src.body().tag();
        let mut _c_header: u8 = (src.header() & !0x60) | (_d_header_enc << 5);
        if let Some(_o) = over.header_m {
            _c_header = (_c_header & !0x10) | (u8::from(_o) << 4);
        }
        if let Some(_o) = over.header_z {
            _c_header = (_c_header & !0x80) | (u8::from(_o) << 7);
        }
        w.write_u8(_c_header)?;
        match src.body() {
            CodecZenohValueEntryVariantSource::CodecZenohExtUnit(_b) => CodecZenohExtUnit::write_source(_b, w)?,
            CodecZenohValueEntryVariantSource::CodecZenohExtZint(_b) => CodecZenohExtZint::write_source(_b, w)?,
            CodecZenohValueEntryVariantSource::CodecZenohValueZbuf(_b) => CodecZenohValueZbuf::write_source(_b, w)?,
            CodecZenohValueEntryVariantSource::Default { body: _b, .. } => CodecZenohExtUnit::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohValueEntrySource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<'a, S: CodecZenohValueEntrySource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohValueEntry<'a> {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohValueEntry<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohValueEntrySource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohValueEntrySource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohValueEntry<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohValueEntryOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohValueEntryOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohValueEntryOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohValueEntryOwned` alone is the build's
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
// `let v: CodecZenohValueEntryOwned = CodecZenohValueEntryOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a> CodecZenohValueEntryVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: CodecZenohValueEntryFrom<'a>>(self) -> Result<CodecZenohValueEntryOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecZenohValueEntryVariant::CodecZenohExtUnit(_b) => CodecZenohValueEntryOwnedVariant::CodecZenohExtUnit(_b),
            CodecZenohValueEntryVariant::CodecZenohExtZint(_b) => CodecZenohValueEntryOwnedVariant::CodecZenohExtZint(_b),
            CodecZenohValueEntryVariant::CodecZenohValueZbuf(_b) => CodecZenohValueEntryOwnedVariant::CodecZenohValueZbuf(_b.try_into_owned_in::<S>()?),
            CodecZenohValueEntryVariant::Default { tag, body } => CodecZenohValueEntryOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohValueEntryOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohValueEntryStorage> CodecZenohValueEntryOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohValueEntryVariant<'_>, CodecError> {
        let mut _out = CodecZenohValueEntryVariant::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohValueEntryVariant<'o>) -> Result<(), CodecError> {
        match self {
            CodecZenohValueEntryOwnedVariant::CodecZenohExtUnit(_b) => {
                *_out = CodecZenohValueEntryVariant::CodecZenohExtUnit(Default::default());
                if let CodecZenohValueEntryVariant::CodecZenohExtUnit(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohValueEntryOwnedVariant::CodecZenohExtZint(_b) => {
                *_out = CodecZenohValueEntryVariant::CodecZenohExtZint(Default::default());
                if let CodecZenohValueEntryVariant::CodecZenohExtZint(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohValueEntryOwnedVariant::CodecZenohValueZbuf(_b) => {
                *_out = CodecZenohValueEntryVariant::CodecZenohValueZbuf(Default::default());
                if let CodecZenohValueEntryVariant::CodecZenohValueZbuf(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohValueEntryOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohValueEntryVariant::Default { tag: *tag, body: Default::default() };
                if let CodecZenohValueEntryVariant::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohValueEntry<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohValueEntryOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohValueEntryFrom<'a>>(self) -> Result<CodecZenohValueEntryOwned<S>, CodecError> {
        Ok(CodecZenohValueEntryOwned {
            header: self.header,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohValueEntryOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohValueEntryStorage> CodecZenohValueEntryOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohValueEntry<'_>, CodecError> {
        let mut _out = CodecZenohValueEntry::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohValueEntry<'o>) -> Result<(), CodecError> {
        _out.header = self.header;
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
    pub fn transcode_in<D: for<'x> CodecZenohValueEntryFrom<'x>>(&self) -> Result<CodecZenohValueEntryOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
