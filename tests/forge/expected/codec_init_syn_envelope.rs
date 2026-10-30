#![doc = "SCE-MAP: codec_init_syn_envelope:24 :: _forge_body"]
// SCE-MAP: codec_init_syn_envelope:24 :: _forge_body

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

use super::codec_init_syn_body::CodecInitSynBody;

/// A tag `CodecInitSynEnvelope` names no arm for, which its default arm holds: a
/// value inside the 5 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecInitSynEnvelopeUnknownTag(u8);

#[allow(dead_code)]
impl CodecInitSynEnvelopeUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            1 => None,
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
pub enum CodecInitSynEnvelopeVariant {
    CodecInitSynBody(CodecInitSynBody),
    Default {
        tag: CodecInitSynEnvelopeUnknownTag,
        body: CodecInitSynBody,
    },
}

impl Default for CodecInitSynEnvelopeVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecInitSynBody(CodecInitSynBody::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecInitSynEnvelope {
    pub header: u8,
    pub body: CodecInitSynEnvelopeVariant,
}

#[allow(dead_code)]
impl CodecInitSynEnvelope {
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
            1u8 => {
                _out.body = CodecInitSynEnvelopeVariant::CodecInitSynBody(CodecInitSynBody::default());
                if let CodecInitSynEnvelopeVariant::CodecInitSynBody(_b) = &mut _out.body {
                    CodecInitSynBody::decode_into(cursor, _b, (header >> 6) & 0x1)?;
                }
            }
            other => {
                _out.body = CodecInitSynEnvelopeVariant::Default {
                    tag: CodecInitSynEnvelopeUnknownTag(other),
                    body: CodecInitSynBody::default(),
                };
                if let CodecInitSynEnvelopeVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecInitSynBody::decode_into(cursor, _b, (header >> 6) & 0x1)?;
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 5;

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

/// The arm of `CodecInitSynEnvelope`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecInitSynEnvelope` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecInitSynEnvelopeVariantSource<'s, A0: ?Sized, D: ?Sized> {
    CodecInitSynBody(&'s A0),
    Default { tag: CodecInitSynEnvelopeUnknownTag, body: &'s D },
}

/// Flags of `CodecInitSynEnvelope` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecInitSynEnvelope`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecInitSynEnvelopeOverrides {
    /// The `S` flag of `header`, when a parent sets it.
    pub header_s: Option<bool>,
}

#[allow(dead_code)]
impl CodecInitSynEnvelopeOverrides {
    /// Sets `S` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_s(value: bool) -> Self {
        Self { header_s: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecInitSynEnvelope` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecInitSynEnvelope`
/// computes those from what is ([`CodecInitSynEnvelope::encode_source`]).
pub trait CodecInitSynEnvelopeSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecInitSynEnvelopeVariantSource<'_, impl super::codec_init_syn_body::CodecInitSynBodySource, impl super::codec_init_syn_body::CodecInitSynBodySource>;
}

impl<T: CodecInitSynEnvelopeSource + ?Sized> CodecInitSynEnvelopeSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn body(&self) -> CodecInitSynEnvelopeVariantSource<'_, impl super::codec_init_syn_body::CodecInitSynBodySource, impl super::codec_init_syn_body::CodecInitSynBodySource> {
        (**self).body()
    }
}

/// The arm of `CodecInitSynEnvelope`'s variant that CodecInitSynEnvelopeParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecInitSynEnvelopePartsBody<'a> {
    CodecInitSynBody(&'a super::codec_init_syn_body::CodecInitSynBodyParts),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecInitSynEnvelopeUnknownTag, body: &'a super::codec_init_syn_body::CodecInitSynBodyParts },
}

/// A value of `CodecInitSynEnvelope` held in the types a caller already has: a source
/// ([`CodecInitSynEnvelopeSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecInitSynEnvelopeParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The arm of the variant this value holds.
    pub body: CodecInitSynEnvelopePartsBody<'a>,
}

impl<'a> CodecInitSynEnvelopeSource for CodecInitSynEnvelopeParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecInitSynEnvelopeVariantSource<'_, impl super::codec_init_syn_body::CodecInitSynBodySource, impl super::codec_init_syn_body::CodecInitSynBodySource> {
        match &self.body {
            CodecInitSynEnvelopePartsBody::CodecInitSynBody(_b) => CodecInitSynEnvelopeVariantSource::CodecInitSynBody(*_b),
            CodecInitSynEnvelopePartsBody::Default { tag, body } => CodecInitSynEnvelopeVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl CodecInitSynEnvelopeSource for CodecInitSynEnvelope {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecInitSynEnvelopeVariantSource<'_, impl super::codec_init_syn_body::CodecInitSynBodySource, impl super::codec_init_syn_body::CodecInitSynBodySource> {
        match &self.body {
            CodecInitSynEnvelopeVariant::CodecInitSynBody(_b) => CodecInitSynEnvelopeVariantSource::CodecInitSynBody(_b),
            CodecInitSynEnvelopeVariant::Default { tag, body } => CodecInitSynEnvelopeVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl CodecInitSynEnvelope {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecInitSynEnvelopeSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecInitSynEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::count_source(src)?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecInitSynEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source_in(src, w, &CodecInitSynEnvelopeOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecInitSynEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecInitSynEnvelopeOverrides) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let _d_header_mid: u8 = match src.body() {
            CodecInitSynEnvelopeVariantSource::CodecInitSynBody(_) => 1,
            CodecInitSynEnvelopeVariantSource::Default { tag, .. } => tag.get(),
        };
        let mut _c_header: u8 = (src.header() & !0x1F) | _d_header_mid;
        if let Some(_o) = over.header_s {
            _c_header = (_c_header & !0x40) | (u8::from(_o) << 6);
        }
        w.write_u8(_c_header)?;
        match src.body() {
            CodecInitSynEnvelopeVariantSource::CodecInitSynBody(_b) => CodecInitSynBody::write_source(_b, w, (_c_header >> 6) & 0x1)?,
            CodecInitSynEnvelopeVariantSource::Default { body: _b, .. } => CodecInitSynBody::write_source(_b, w, (_c_header >> 6) & 0x1)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecInitSynEnvelopeSource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<S: CodecInitSynEnvelopeSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecInitSynEnvelope {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecInitSynEnvelope {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecInitSynEnvelopeSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecInitSynEnvelopeSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
