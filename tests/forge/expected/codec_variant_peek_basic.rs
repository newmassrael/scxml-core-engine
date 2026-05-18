#![doc = "SCE-MAP: codec_variant_peek_basic:29"]
// SCE-MAP: codec_variant_peek_basic:29

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_peek_arm_a::CodecPeekArmA;
use super::codec_peek_arm_b::CodecPeekArmB;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
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
#[derive(Default)]
pub struct CodecVariantPeekBasic {
    pub body: CodecVariantPeekBasicVariant,
}

#[allow(dead_code)]
impl CodecVariantPeekBasic {
    /// Construct an instance with every field zero-initialized via
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519).
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
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
        let body = match ((_peek >> 0) & (0x01 as u8)) as u8 {
            0u8 => CodecVariantPeekBasicVariant::CodecPeekArmA(CodecPeekArmA::decode(cursor)?),
            1u8 => CodecVariantPeekBasicVariant::CodecPeekArmB(CodecPeekArmB::decode(cursor)?),
            // Build-time `codec/variant-arm-unreachable` proves the
            // arm set covers the tag domain without a default.
            _ => unreachable!("variant exhaustiveness gated by codec/variant-arm-unreachable at parse time"),
        };
        Ok(Self {
            body,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode (per-field present_if/tlv-chain/embed/
        // repeat helpers). Peek-byte mode: the arm body's encode prepends
        // its own header byte (which the decoder peeked); no separate
        // tag byte is emitted here. Streaming-prefix mode (own-field
        // variant): the carrier is part of the prefix fields and emits
        // through the same per-field path.
        let mut r: Vec<u8> = Vec::with_capacity(3);
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecVariantPeekBasicVariant::CodecPeekArmA(b) => {
                r.extend(b.encode());
            }
            CodecVariantPeekBasicVariant::CodecPeekArmB(b) => {
                r.extend(b.encode());
            }
        }
        r
    }
}
