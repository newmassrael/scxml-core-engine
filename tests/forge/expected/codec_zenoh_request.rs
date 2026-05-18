#![doc = "SCE-MAP: codec_zenoh_request:73"]
// SCE-MAP: codec_zenoh_request:73

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_wireexpr::CodecZenohWireexpr;
use super::codec_zenoh_ext_entry::CodecZenohExtEntry;
use super::codec_zenoh_msg_put::CodecZenohMsgPut;
use super::codec_zenoh_msg_del::CodecZenohMsgDel;
use super::codec_zenoh_query::CodecZenohQuery;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohRequestVariant {
    CodecZenohMsgPut(CodecZenohMsgPut),
    CodecZenohMsgDel(CodecZenohMsgDel),
    CodecZenohQuery(CodecZenohQuery),
    Default {
        tag: u8,
        body: CodecZenohQuery,
    },
}

impl Default for CodecZenohRequestVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohMsgPut(CodecZenohMsgPut::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohRequest {
    pub header: u8,
    pub rid: u64,
    pub keyexpr: CodecZenohWireexpr,
    pub extensions: Option<Vec<CodecZenohExtEntry>>,
    pub body: CodecZenohRequestVariant,
}

#[allow(dead_code)]
impl CodecZenohRequest {
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
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let rid = cursor.read_vle_u64()?;
        let keyexpr = CodecZenohWireexpr::decode(cursor, header)?;
        let extensions = if (header & 0x80u8) != 0 {
            let mut _vec: Vec<CodecZenohExtEntry> = Vec::with_capacity(4 as usize);
            for _ in 0..4u32 {
                    if cursor.remaining() == 0 { break; }
                    let _entry = CodecZenohExtEntry::decode(cursor)?;
                    let _continue = _entry.z();
                    _vec.push(_entry);
                    if !_continue { break; }
                }
            Some(_vec)
        } else {
            None
        };
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match ((_peek >> 0) & (0x1F as u8)) as u8 {
            1u8 => CodecZenohRequestVariant::CodecZenohMsgPut(CodecZenohMsgPut::decode(cursor)?),
            2u8 => CodecZenohRequestVariant::CodecZenohMsgDel(CodecZenohMsgDel::decode(cursor)?),
            3u8 => CodecZenohRequestVariant::CodecZenohQuery(CodecZenohQuery::decode(cursor)?),
            other => CodecZenohRequestVariant::Default {
                tag: other,
                body: CodecZenohQuery::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            rid,
            keyexpr,
            extensions,
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

    pub fn n(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_n(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn m(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_m(&mut self, v: bool) {
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

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode (per-field present_if/tlv-chain/embed/
        // repeat helpers). Peek-byte mode: the arm body's encode prepends
        // its own header byte (which the decoder peeked); no separate
        // tag byte is emitted here. Streaming-prefix mode (own-field
        // variant): the carrier is part of the prefix fields and emits
        // through the same per-field path.
        let mut r: Vec<u8> = Vec::with_capacity(1218);
        r.push(self.header);
        {
            let mut _w = self.rid as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        r.extend(self.keyexpr.encode(self.header));
        if let Some(_list) = &self.extensions {
            for _e in _list {
                r.extend(_e.encode());
            }
        }
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohRequestVariant::CodecZenohMsgPut(b) => {
                r.extend(b.encode());
            }
            CodecZenohRequestVariant::CodecZenohMsgDel(b) => {
                r.extend(b.encode());
            }
            CodecZenohRequestVariant::CodecZenohQuery(b) => {
                r.extend(b.encode());
            }
            CodecZenohRequestVariant::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
