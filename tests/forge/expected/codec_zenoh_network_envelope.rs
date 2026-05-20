#![doc = "SCE-MAP: codec_zenoh_network_envelope:60"]
// SCE-MAP: codec_zenoh_network_envelope:60

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// RFC §5.B B1-α: `VecSink` and the heap-backed `encode_to_vec` facade
// are gated on the `alloc` feature (see
// `sce-forge-runtime/rust/src/codec.rs`). MCU / `no_std` consumers see
// only the sink-based primary `encode` + `SliceSink` paths.
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use sce_forge_runtime::codec::VecSink;

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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 1218;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode (per-field present_if/tlv-chain/embed/
        // repeat helpers). Peek-byte mode: the arm body's encode prepends
        // its own header byte (which the decoder peeked); no separate
        // tag byte is emitted here. Streaming-prefix mode (own-field
        // variant): the carrier is part of the prefix fields and emits
        // through the same per-field path.
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohNetworkEnvelopeVariant::CodecZenohInterest(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponseFinal(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponse(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohRequest(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohPush(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohDeclare(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohOam(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::Default { body, .. } => {
                body.encode(w)?;
            }
        }
        Ok(())
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector. Callers
    /// targeting zero-alloc hot paths should call `encode` directly
    /// against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `sce-forge-runtime/rust/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)
            .expect("VecSink is infallible");
        _sce_v
    }
}
