#![doc = "SCE-MAP: codec_variant_peek_basic:29 :: _forge_body"]
// SCE-MAP: codec_variant_peek_basic:29 :: _forge_body

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

use super::codec_peek_arm_a::CodecPeekArmA;
use super::codec_peek_arm_b::CodecPeekArmB;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecVariantPeekBasicVariant {
    CodecPeekArmA(CodecPeekArmA),
    CodecPeekArmB(CodecPeekArmB),
}

impl Default for CodecVariantPeekBasicVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecPeekArmA(CodecPeekArmA::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecVariantPeekBasic {
    pub body: CodecVariantPeekBasicVariant,
}

#[allow(dead_code)]
impl CodecVariantPeekBasic {
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
        match _peek & 0x01 {
            0u8 => {
                _out.body = CodecVariantPeekBasicVariant::CodecPeekArmA(CodecPeekArmA::default());
                if let CodecVariantPeekBasicVariant::CodecPeekArmA(_b) = &mut _out.body {
                    CodecPeekArmA::decode_into(cursor, _b)?;
                }
            }
            1u8 => {
                _out.body = CodecVariantPeekBasicVariant::CodecPeekArmB(CodecPeekArmB::default());
                if let CodecVariantPeekBasicVariant::CodecPeekArmB(_b) = &mut _out.body {
                    CodecPeekArmB::decode_into(cursor, _b)?;
                }
            }
            // Build-time `codec/variant-arm-unreachable` proves the
            // arm set covers the tag domain without a default.
            _ => unreachable!("variant exhaustiveness gated by codec/variant-arm-unreachable at parse time"),
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 3;

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

/// The arm of `CodecVariantPeekBasic`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecVariantPeekBasic` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecVariantPeekBasicVariantSource<'s, A0: ?Sized, A1: ?Sized> {
    CodecPeekArmA(&'s A0),
    CodecPeekArmB(&'s A1),
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecVariantPeekBasic` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecVariantPeekBasic`
/// computes those from what is ([`CodecVariantPeekBasic::encode_source`]).
pub trait CodecVariantPeekBasicSource {
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecVariantPeekBasicVariantSource<'_, impl super::codec_peek_arm_a::CodecPeekArmASource, impl super::codec_peek_arm_b::CodecPeekArmBSource>;
}

impl<T: CodecVariantPeekBasicSource + ?Sized> CodecVariantPeekBasicSource for &T {
    fn body(&self) -> CodecVariantPeekBasicVariantSource<'_, impl super::codec_peek_arm_a::CodecPeekArmASource, impl super::codec_peek_arm_b::CodecPeekArmBSource> {
        (**self).body()
    }
}

/// The arm of `CodecVariantPeekBasic`'s variant that CodecVariantPeekBasicParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecVariantPeekBasicPartsBody<'a> {
    CodecPeekArmA(&'a super::codec_peek_arm_a::CodecPeekArmAParts),
    CodecPeekArmB(&'a super::codec_peek_arm_b::CodecPeekArmBParts),
}

/// A value of `CodecVariantPeekBasic` held in the types a caller already has: a source
/// ([`CodecVariantPeekBasicSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecVariantPeekBasicParts<'a> {
    /// The arm of the variant this value holds.
    pub body: CodecVariantPeekBasicPartsBody<'a>,
}

impl<'a> CodecVariantPeekBasicSource for CodecVariantPeekBasicParts<'a> {
    fn body(&self) -> CodecVariantPeekBasicVariantSource<'_, impl super::codec_peek_arm_a::CodecPeekArmASource, impl super::codec_peek_arm_b::CodecPeekArmBSource> {
        match &self.body {
            CodecVariantPeekBasicPartsBody::CodecPeekArmA(_b) => CodecVariantPeekBasicVariantSource::CodecPeekArmA(*_b),
            CodecVariantPeekBasicPartsBody::CodecPeekArmB(_b) => CodecVariantPeekBasicVariantSource::CodecPeekArmB(*_b),
        }
    }
}

impl CodecVariantPeekBasicSource for CodecVariantPeekBasic {
    fn body(&self) -> CodecVariantPeekBasicVariantSource<'_, impl super::codec_peek_arm_a::CodecPeekArmASource, impl super::codec_peek_arm_b::CodecPeekArmBSource> {
        match &self.body {
            CodecVariantPeekBasicVariant::CodecPeekArmA(_b) => CodecVariantPeekBasicVariantSource::CodecPeekArmA(_b),
            CodecVariantPeekBasicVariant::CodecPeekArmB(_b) => CodecVariantPeekBasicVariantSource::CodecPeekArmB(_b),
        }
    }
}

#[allow(dead_code)]
impl CodecVariantPeekBasic {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecVariantPeekBasicSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecVariantPeekBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecVariantPeekBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        match src.body() {
            CodecVariantPeekBasicVariantSource::CodecPeekArmA(_b) => CodecPeekArmA::write_source(_b, w)?,
            CodecVariantPeekBasicVariantSource::CodecPeekArmB(_b) => CodecPeekArmB::write_source(_b, w)?,
        }
        Ok(())
    }
}

impl<S: CodecVariantPeekBasicSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecVariantPeekBasic {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecVariantPeekBasic {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecVariantPeekBasicSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecVariantPeekBasicSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
