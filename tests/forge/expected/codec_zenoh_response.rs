#![doc = "SCE-MAP: codec_zenoh_response:75"]
// SCE-MAP: codec_zenoh_response:75

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_ext_entry::CodecZenohExtEntry;
use super::codec_zenoh_reply::CodecZenohReply;
use super::codec_zenoh_err::CodecZenohErr;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohResponseVariant {
    CodecZenohReply(CodecZenohReply),
    CodecZenohErr(CodecZenohErr),
    Default {
        tag: u8,
        body: CodecZenohReply,
    },
}

impl Default for CodecZenohResponseVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohReply(CodecZenohReply::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohResponse {
    pub header: u8,
    pub request_id: u64,
    pub key_id: u32,
    pub suffix_len: Option<u64>,
    pub suffix: Option<String>,
    pub extensions: Option<Vec<CodecZenohExtEntry>>,
    pub body: CodecZenohResponseVariant,
}

#[allow(dead_code)]
impl CodecZenohResponse {
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
        let request_id = cursor.read_vle_u64()?;
        let key_id = cursor.read_vle_u32()?;
        let suffix_len = if (header & 0x20u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let suffix = if (header & 0x20u8) != 0 {
            let _n = suffix_len.unwrap() as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = core::str::from_utf8(raw)
                .map_err(|_| CodecError::InvalidUtf8)?
                .to_string();
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
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
            4u8 => CodecZenohResponseVariant::CodecZenohReply(CodecZenohReply::decode(cursor)?),
            5u8 => CodecZenohResponseVariant::CodecZenohErr(CodecZenohErr::decode(cursor)?),
            other => CodecZenohResponseVariant::Default {
                tag: other,
                body: CodecZenohReply::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            request_id,
            key_id,
            suffix_len,
            suffix,
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
        let mut r: Vec<u8> = Vec::with_capacity(977);
        r.push(self.header);
        {
            let mut _w = self.request_id as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        {
            let mut _w = self.key_id as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        if let Some(_v) = self.suffix_len {
        {
            let mut _w = _v as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        }
        if let Some(_v) = &self.suffix {
            r.extend_from_slice(_v.as_bytes());
        }
        if let Some(_list) = &self.extensions {
            for _e in _list {
                r.extend(_e.encode());
            }
        }
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohResponseVariant::CodecZenohReply(b) => {
                r.extend(b.encode());
            }
            CodecZenohResponseVariant::CodecZenohErr(b) => {
                r.extend(b.encode());
            }
            CodecZenohResponseVariant::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
