#![doc = "SCE-MAP: codec_zenoh_push:45 :: _forge_body"]
// SCE-MAP: codec_zenoh_push:45 :: _forge_body

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

use super::codec_zenoh_push_body::CodecZenohPushBody;

use super::codec_zenoh_push_body::CodecZenohPushBodyView;

/// A tag `CodecZenohPush` names no arm for, which its default arm holds: a
/// value inside the 5 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecZenohPushUnknownTag(u8);

#[allow(dead_code)]
impl CodecZenohPushUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            29 => None,
            _ if tag > 31 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecZenohPushUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(31, &[(29, 29)]);

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
pub enum CodecZenohPushVariant {
    CodecZenohPushBody(CodecZenohPushBody),
    Default {
        tag: CodecZenohPushUnknownTag,
        body: CodecZenohPushBody,
    },
}

impl Default for CodecZenohPushVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohPushBody(CodecZenohPushBody::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohPush {
    pub header: u8,
    pub body: CodecZenohPushVariant,
}

// RFC variant-default-uniformity: at least one field's
// `<sce:flags>` carrier declares a wire-MID constant via
// `<sce:flag value="N"/>`. Manual `impl Default` bakes the OR of
// every declared `(value & mask) << bit` into that carrier so a
// freshly-constructed instance carries the wire-MID for its own
// dispatch tag. Fields without declared values fall through to
// `Default::default()` (preserving derive(Default) semantics).
impl Default for CodecZenohPush {
    fn default() -> Self {
        Self {
            header: 0x1du8,
            body: CodecZenohPushVariant::default(),
        }
    }
}

#[allow(dead_code)]
impl CodecZenohPush {
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
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
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
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
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
            29u8 => {
                _out.body = CodecZenohPushVariant::CodecZenohPushBody(CodecZenohPushBody::default());
                if let CodecZenohPushVariant::CodecZenohPushBody(_b) = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohPushBody::decode_into(cursor, _b), &mut _over)?;
                }
            }
            other => {
                _out.body = CodecZenohPushVariant::Default {
                    tag: CodecZenohPushUnknownTag(other),
                    body: CodecZenohPushBody::default(),
                };
                if let CodecZenohPushVariant::Default { body: _b, .. } = &mut _out.body {
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohPushBody::decode_into(cursor, _b), &mut _over)?;
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

    pub fn rest(&self) -> u8 {
        (self.header >> 5) & 0x07
    }

    pub fn set_rest(&mut self, v: u8) {
        self.header = (self.header & !0xE0) | ((v & 0x07) << 5);
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 3;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohPushWire::encode_source(self, w)
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

/// The arm of `CodecZenohPush`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohPush` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohPushVariantSource<'s, A0: ?Sized, D: ?Sized> {
    CodecZenohPushBody(&'s A0),
    Default { tag: CodecZenohPushUnknownTag, body: &'s D },
}

impl<'s, A0: ?Sized, D: ?Sized> CodecZenohPushVariantSource<'s, A0, D> {
    /// The tag of the arm this is: the value the document gives a known
    /// arm, and the tag a default arm holds. It is the tag the encode
    /// writes, or the one a caller hands the decode for a variant whose
    /// wire form carries none.
    pub fn tag(&self) -> u8 {
        match self {
            Self::CodecZenohPushBody(_) => 29,
            Self::Default { tag, .. } => tag.get(),
        }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohPush` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohPushWire::encode_source`]).
pub trait CodecZenohPushSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohPushVariantSource<'_, impl super::codec_zenoh_push_body::CodecZenohPushBodySource, impl super::codec_zenoh_push_body::CodecZenohPushBodySource>;
}

impl<T: CodecZenohPushSource + ?Sized> CodecZenohPushSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn body(&self) -> CodecZenohPushVariantSource<'_, impl super::codec_zenoh_push_body::CodecZenohPushBodySource, impl super::codec_zenoh_push_body::CodecZenohPushBodySource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohPush`'s variant that CodecZenohPushParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohPushPartsBody<'a> {
    CodecZenohPushBody(&'a super::codec_zenoh_push_body::CodecZenohPushBodyParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecZenohPushUnknownTag, body: &'a super::codec_zenoh_push_body::CodecZenohPushBodyParts<'a> },
}

/// A value of `CodecZenohPush` held in the types a caller already has: a source
/// ([`CodecZenohPushSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohPushParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The arm of the variant this value holds.
    pub body: CodecZenohPushPartsBody<'a>,
}

impl<'a> CodecZenohPushSource for CodecZenohPushParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohPushVariantSource<'_, impl super::codec_zenoh_push_body::CodecZenohPushBodySource, impl super::codec_zenoh_push_body::CodecZenohPushBodySource> {
        match &self.body {
            CodecZenohPushPartsBody::CodecZenohPushBody(_b) => CodecZenohPushVariantSource::CodecZenohPushBody(*_b),
            CodecZenohPushPartsBody::Default { tag, body } => CodecZenohPushVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl CodecZenohPushSource for CodecZenohPush {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohPushVariantSource<'_, impl super::codec_zenoh_push_body::CodecZenohPushBodySource, impl super::codec_zenoh_push_body::CodecZenohPushBodySource> {
        match &self.body {
            CodecZenohPushVariant::CodecZenohPushBody(_b) => CodecZenohPushVariantSource::CodecZenohPushBody(_b),
            CodecZenohPushVariant::Default { tag, body } => CodecZenohPushVariantSource::Default { tag: *tag, body },
        }
    }
}

impl CodecZenohPushSource for CodecZenohPushView {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohPushVariantSource<'_, impl super::codec_zenoh_push_body::CodecZenohPushBodySource, impl super::codec_zenoh_push_body::CodecZenohPushBodySource> {
        match &self.body {
            CodecZenohPushViewVariant::CodecZenohPushBody(_b) => CodecZenohPushVariantSource::CodecZenohPushBody(_b),
            CodecZenohPushViewVariant::Default { tag, body } => CodecZenohPushVariantSource::Default { tag: *tag, body },
        }
    }
}

/// The codec `CodecZenohPush` itself: what writes a value read through
/// [`CodecZenohPushSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohPushWire;

#[allow(dead_code)]
impl CodecZenohPushWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohPush::encode` writes for the value it holds, read through
    /// [`CodecZenohPushSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohPushSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The tag of the arm `src` holds ([`CodecZenohPushVariantSource::tag`]).
    pub fn tag_of<S: CodecZenohPushSource + ?Sized>(src: &S) -> u8 {
        src.body().tag()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohPushSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _d_header_mid: u8 = src.body().tag();
        let _c_header: u8 = (src.header() & !0x1F) | _d_header_mid;
        w.write_u8(_c_header)?;
        match src.body() {
            CodecZenohPushVariantSource::CodecZenohPushBody(_b) => super::codec_zenoh_push_body::CodecZenohPushBodyWire::write_source(_b, w)?,
            CodecZenohPushVariantSource::Default { body: _b, .. } => super::codec_zenoh_push_body::CodecZenohPushBodyWire::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohPushSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohPushSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohPushWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohPushWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohPushSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohPushSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecZenohPushView` is what a decode of `CodecZenohPush` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecZenohPush` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecZenohPush::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohPushViewVariant {
    CodecZenohPushBody(CodecZenohPushBodyView),
    Default {
        tag: CodecZenohPushUnknownTag,
        body: CodecZenohPushBodyView,
    },
}

impl Default for CodecZenohPushViewVariant {
    fn default() -> Self {
        Self::CodecZenohPushBody(CodecZenohPushBodyView::default())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohPushView {
    pub header: u8,
    pub body: CodecZenohPushViewVariant,
}

impl Default for CodecZenohPushView {
    fn default() -> Self {
        Self {
            header: 0x1du8,
            body: CodecZenohPushViewVariant::default(),
        }
    }
}

#[allow(dead_code)]
impl CodecZenohPushView {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecZenohPush::decode`], held as a view. On success the
    /// cursor advances past the consumed bytes.
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out)?;
        Ok(_out)
    }

    /// [`Self::decode`] into a destination the caller owns, so a view that
    /// embeds or dispatches to this one is filled where it will live. On
    /// `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
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
            29u8 => {
                _out.body = CodecZenohPushViewVariant::CodecZenohPushBody(CodecZenohPushBodyView::default());
                if let CodecZenohPushViewVariant::CodecZenohPushBody(_b) = &mut _out.body {
                    CodecZenohPushBodyView::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohPushViewVariant::Default {
                    tag: CodecZenohPushUnknownTag(other),
                    body: CodecZenohPushBodyView::default(),
                };
                if let CodecZenohPushViewVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohPushBodyView::decode_into(cursor, _b)?;
                }
            }
        }
        _out.header = header;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecZenohPush`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder(&self, _out: &mut CodecZenohPush) -> Result<(), CodecError> {
        _out.header = self.header;
        match &self.body {
            CodecZenohPushViewVariant::CodecZenohPushBody(_b) => {
                _out.body = CodecZenohPushVariant::CodecZenohPushBody(Default::default());
                if let CodecZenohPushVariant::CodecZenohPushBody(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecZenohPushViewVariant::Default { tag, body } => {
                _out.body = CodecZenohPushVariant::Default {
                    tag: *tag,
                    body: Default::default(),
                };
                if let CodecZenohPushVariant::Default { body: _o, .. } = &mut _out.body {
                    body.into_builder(_o)?;
                }
            }
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecZenohPush::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 3;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecZenohPush::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecZenohPushWire::encode_source(self, w)
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

    pub fn rest(&self) -> u8 {
        (self.header >> 5) & 0x07
    }

}
