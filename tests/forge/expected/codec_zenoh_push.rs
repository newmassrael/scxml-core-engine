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
    /// On `Err`, `_out` holds the fields decoded so far and is not to be read.
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
                _out.body = CodecZenohPushVariant::CodecZenohPushBody(CodecZenohPushBody::default());
                if let CodecZenohPushVariant::CodecZenohPushBody(_b) = &mut _out.body {
                    CodecZenohPushBody::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohPushVariant::Default {
                    tag: CodecZenohPushUnknownTag(other),
                    body: CodecZenohPushBody::default(),
                };
                if let CodecZenohPushVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohPushBody::decode_into(cursor, _b)?;
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
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        Self::encode_source(self, w)
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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohPush` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohPush`
/// computes those from what is ([`CodecZenohPush::encode_source`]).
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
/// ([`CodecZenohPushSource`]) whose lists are slices and whose bodies are references.
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

#[allow(dead_code)]
impl CodecZenohPush {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohPushSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohPushSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohPushSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _d_header_mid: u8 = match src.body() {
            CodecZenohPushVariantSource::CodecZenohPushBody(_) => 29,
            CodecZenohPushVariantSource::Default { tag, .. } => tag.get(),
        };
        let _c_header: u8 = (src.header() & !0x1F) | _d_header_mid;
        w.write_u8(_c_header)?;
        match src.body() {
            CodecZenohPushVariantSource::CodecZenohPushBody(_b) => CodecZenohPushBody::write_source(_b, w)?,
            CodecZenohPushVariantSource::Default { body: _b, .. } => CodecZenohPushBody::write_source(_b, w)?,
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

impl<S: CodecZenohPushSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohPush {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohPush {
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
