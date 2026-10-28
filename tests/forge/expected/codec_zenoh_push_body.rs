#![doc = "SCE-MAP: codec_zenoh_push_body:30 :: _forge_body"]
// SCE-MAP: codec_zenoh_push_body:30 :: _forge_body

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

use super::codec_zenoh_put::CodecZenohPut;
use super::codec_zenoh_del::CodecZenohDel;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohPushBodyVariant {
    CodecZenohPut(CodecZenohPut),
    CodecZenohDel(CodecZenohDel),
    Default {
        tag: u8,
        body: CodecZenohPut,
    },
}

impl Default for CodecZenohPushBodyVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohPut(CodecZenohPut::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohPushBody {
    pub header: u8,
    pub body: CodecZenohPushBodyVariant,
}

#[allow(dead_code)]
impl CodecZenohPushBody {
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
                _out.body = CodecZenohPushBodyVariant::CodecZenohPut(CodecZenohPut::default());
                if let CodecZenohPushBodyVariant::CodecZenohPut(_b) = &mut _out.body {
                    CodecZenohPut::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecZenohPushBodyVariant::CodecZenohDel(CodecZenohDel::default());
                if let CodecZenohPushBodyVariant::CodecZenohDel(_b) = &mut _out.body {
                    CodecZenohDel::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohPushBodyVariant::Default {
                    tag: other,
                    body: CodecZenohPut::default(),
                };
                if let CodecZenohPushBodyVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohPut::decode_into(cursor, _b)?;
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
    pub const MAX_ENCODED_BYTES: usize = 2;

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

/// The arm of `CodecZenohPushBody`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohPushBody` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohPushBodyVariantSource<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> {
    CodecZenohPut(&'s A0),
    CodecZenohDel(&'s A1),
    Default { tag: u8, body: &'s D },
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohPushBody` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohPushBody`
/// computes those from what is ([`CodecZenohPushBody::encode_source`]).
pub trait CodecZenohPushBodySource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohPushBodyVariantSource<'_, impl super::codec_zenoh_put::CodecZenohPutSource, impl super::codec_zenoh_del::CodecZenohDelSource, impl super::codec_zenoh_put::CodecZenohPutSource>;
}

impl<T: CodecZenohPushBodySource + ?Sized> CodecZenohPushBodySource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn body(&self) -> CodecZenohPushBodyVariantSource<'_, impl super::codec_zenoh_put::CodecZenohPutSource, impl super::codec_zenoh_del::CodecZenohDelSource, impl super::codec_zenoh_put::CodecZenohPutSource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohPushBody`'s variant that CodecZenohPushBodyParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohPushBodyPartsBody<'a> {
    CodecZenohPut(&'a super::codec_zenoh_put::CodecZenohPutParts),
    CodecZenohDel(&'a super::codec_zenoh_del::CodecZenohDelParts),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: u8, body: &'a super::codec_zenoh_put::CodecZenohPutParts },
}

/// A value of `CodecZenohPushBody` held in the types a caller already has: a source
/// ([`CodecZenohPushBodySource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohPushBodyParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The arm of the variant this value holds.
    pub body: CodecZenohPushBodyPartsBody<'a>,
}

impl<'a> CodecZenohPushBodySource for CodecZenohPushBodyParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohPushBodyVariantSource<'_, impl super::codec_zenoh_put::CodecZenohPutSource, impl super::codec_zenoh_del::CodecZenohDelSource, impl super::codec_zenoh_put::CodecZenohPutSource> {
        match &self.body {
            CodecZenohPushBodyPartsBody::CodecZenohPut(_b) => CodecZenohPushBodyVariantSource::CodecZenohPut(*_b),
            CodecZenohPushBodyPartsBody::CodecZenohDel(_b) => CodecZenohPushBodyVariantSource::CodecZenohDel(*_b),
            CodecZenohPushBodyPartsBody::Default { tag, body } => CodecZenohPushBodyVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl CodecZenohPushBodySource for CodecZenohPushBody {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohPushBodyVariantSource<'_, impl super::codec_zenoh_put::CodecZenohPutSource, impl super::codec_zenoh_del::CodecZenohDelSource, impl super::codec_zenoh_put::CodecZenohPutSource> {
        match &self.body {
            CodecZenohPushBodyVariant::CodecZenohPut(_b) => CodecZenohPushBodyVariantSource::CodecZenohPut(_b),
            CodecZenohPushBodyVariant::CodecZenohDel(_b) => CodecZenohPushBodyVariantSource::CodecZenohDel(_b),
            CodecZenohPushBodyVariant::Default { tag, body } => CodecZenohPushBodyVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl CodecZenohPushBody {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohPushBodySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecZenohPushBodySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohPushBodySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        let _d_header_mid: u8 = match src.body() {
            CodecZenohPushBodyVariantSource::CodecZenohPut(_) => 1,
            CodecZenohPushBodyVariantSource::CodecZenohDel(_) => 2,
            CodecZenohPushBodyVariantSource::Default { tag, .. } => tag,
        };
        if _d_header_mid > 0x1F {
            return Err(CodecError::LengthOutOfRange);
        }
        let _c_header: u8 = (src.header() & !0x1F) | _d_header_mid;
        w.write_u8(_c_header)?;
        match src.body() {
            CodecZenohPushBodyVariantSource::CodecZenohPut(_b) => CodecZenohPut::write_source(_b, w)?,
            CodecZenohPushBodyVariantSource::CodecZenohDel(_b) => CodecZenohDel::write_source(_b, w)?,
            CodecZenohPushBodyVariantSource::Default { body: _b, .. } => CodecZenohPut::write_source(_b, w)?,
        }
        Ok(())
    }
}

impl<S: CodecZenohPushBodySource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohPushBody {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohPushBody {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecZenohPushBodySource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohPushBodySource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
