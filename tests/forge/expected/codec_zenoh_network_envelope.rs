#![doc = "SCE-MAP: codec_zenoh_network_envelope:60"]
// SCE-MAP: codec_zenoh_network_envelope:60

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_interest::CodecZenohInterest;
use super::codec_zenoh_response_final::CodecZenohResponseFinal;
use super::codec_zenoh_response::CodecZenohResponse;
use super::codec_zenoh_request::CodecZenohRequest;
use super::codec_zenoh_push::CodecZenohPush;
use super::codec_zenoh_declare::CodecZenohDeclare;
use super::codec_zenoh_oam::CodecZenohOam;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohNetworkEnvelopeVariant {
    CodecZenohInterest(CodecZenohInterest),
    CodecZenohResponseFinal(CodecZenohResponseFinal),
    CodecZenohResponse(CodecZenohResponse),
    CodecZenohRequest(CodecZenohRequest),
    CodecZenohPush(CodecZenohPush),
    CodecZenohDeclare(CodecZenohDeclare),
    CodecZenohOam(CodecZenohOam),
    Default {
        tag: u8,
        body: CodecZenohOam,
    },
}

impl Default for CodecZenohNetworkEnvelopeVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohOam(CodecZenohOam::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohNetworkEnvelope {
    pub body: CodecZenohNetworkEnvelopeVariant,
}

#[allow(dead_code)]
impl CodecZenohNetworkEnvelope {
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
        let body = match ((_peek >> 0) & (0x1F as u8)) as u8 {
            25u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohInterest(CodecZenohInterest::decode(cursor)?),
            26u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohResponseFinal(CodecZenohResponseFinal::decode(cursor)?),
            27u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohResponse(CodecZenohResponse::decode(cursor)?),
            28u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohRequest(CodecZenohRequest::decode(cursor)?),
            29u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohPush(CodecZenohPush::decode(cursor)?),
            30u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohDeclare(CodecZenohDeclare::decode(cursor)?),
            31u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohOam(CodecZenohOam::decode(cursor)?),
            other => CodecZenohNetworkEnvelopeVariant::Default {
                tag: other,
                body: CodecZenohOam::decode(cursor)?,
            },
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
        let mut r: Vec<u8> = Vec::with_capacity(1218);
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohNetworkEnvelopeVariant::CodecZenohInterest(b) => {
                r.extend(b.encode());
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponseFinal(b) => {
                r.extend(b.encode());
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponse(b) => {
                r.extend(b.encode());
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohRequest(b) => {
                r.extend(b.encode());
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohPush(b) => {
                r.extend(b.encode());
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohDeclare(b) => {
                r.extend(b.encode());
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohOam(b) => {
                r.extend(b.encode());
            }
            CodecZenohNetworkEnvelopeVariant::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
