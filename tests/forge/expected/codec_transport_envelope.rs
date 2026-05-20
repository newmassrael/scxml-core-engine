#![doc = "SCE-MAP: codec_transport_envelope:69"]
// SCE-MAP: codec_transport_envelope:69

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink, VecSink};

use super::codec_zenoh_init_body::CodecZenohInitBody;
use super::codec_zenoh_open_body::CodecZenohOpenBody;
use super::codec_zenoh_close::CodecZenohClose;
use super::codec_zenoh_keep_alive::CodecZenohKeepAlive;
use super::codec_zenoh_frame::CodecZenohFrame;
use super::codec_zenoh_fragment::CodecZenohFragment;
use super::codec_zenoh_join::CodecZenohJoin;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecTransportEnvelopeVariant {
    CodecZenohInitBody(CodecZenohInitBody),
    CodecZenohOpenBody(CodecZenohOpenBody),
    CodecZenohClose(CodecZenohClose),
    CodecZenohKeepAlive(CodecZenohKeepAlive),
    CodecZenohFrame(CodecZenohFrame),
    CodecZenohFragment(CodecZenohFragment),
    CodecZenohJoin(CodecZenohJoin),
    Default {
        tag: u8,
        body: CodecZenohClose,
    },
}

impl Default for CodecTransportEnvelopeVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohClose(CodecZenohClose::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecTransportEnvelope {
    pub header: u8,
    pub body: CodecTransportEnvelopeVariant,
}

#[allow(dead_code)]
impl CodecTransportEnvelope {
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
        // Decode fixed prefix (RFC §5.B variant primitive B1-β: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let header = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match ((header >> 0) & (0x1F as u8)) as u8 {
            1u8 => CodecTransportEnvelopeVariant::CodecZenohInitBody(CodecZenohInitBody::decode(cursor, ((header >> 6) & 0x1) as u8, ((header >> 5) & 0x1) as u8)?),
            2u8 => CodecTransportEnvelopeVariant::CodecZenohOpenBody(CodecZenohOpenBody::decode(cursor, ((header >> 5) & 0x1) as u8)?),
            3u8 => CodecTransportEnvelopeVariant::CodecZenohClose(CodecZenohClose::decode(cursor)?),
            4u8 => CodecTransportEnvelopeVariant::CodecZenohKeepAlive(CodecZenohKeepAlive::decode(cursor)?),
            5u8 => CodecTransportEnvelopeVariant::CodecZenohFrame(CodecZenohFrame::decode(cursor)?),
            6u8 => CodecTransportEnvelopeVariant::CodecZenohFragment(CodecZenohFragment::decode(cursor)?),
            7u8 => CodecTransportEnvelopeVariant::CodecZenohJoin(CodecZenohJoin::decode(cursor, ((header >> 6) & 0x1) as u8)?),
            other => CodecTransportEnvelopeVariant::Default {
                tag: other,
                body: CodecZenohClose::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            body,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn mid(&self) -> u8 {
        (((self.header >> 0) & (0x1F as u8))) as u8
    }

    pub fn set_mid(&mut self, v: u8) {
        let _mask: u8 = (0x1F as u8) << 0;
        let _val: u8 = ((v as u8) & (0x1F as u8)) << 0;
        self.header = (self.header & !_mask) | _val;
    }

    pub fn a(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_a(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
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
    pub const MAX_ENCODED_BYTES: usize = 65547;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // Encode fixed prefix (tag field is part of the prefix). The
        // tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set msg_id / body in
        // sync is the caller's responsibility (v1 keeps the layout
        // simple; future extensions may auto-sync via a typed setter).
        w.write_u8(self.header)?;
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecTransportEnvelopeVariant::CodecZenohInitBody(b) => {
                b.encode(w, ((self.header >> 6) & 0x1) as u8, ((self.header >> 5) & 0x1) as u8)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohOpenBody(b) => {
                b.encode(w, ((self.header >> 5) & 0x1) as u8)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohClose(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohKeepAlive(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohFrame(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohFragment(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohJoin(b) => {
                b.encode(w, ((self.header >> 6) & 0x1) as u8)?;
            }
            CodecTransportEnvelopeVariant::Default { body, .. } => {
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
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)
            .expect("VecSink is infallible");
        _sce_v
    }
}
