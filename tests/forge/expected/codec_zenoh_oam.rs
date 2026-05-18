#![doc = "SCE-MAP: codec_zenoh_oam:56"]
// SCE-MAP: codec_zenoh_oam:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_ext_entry::CodecZenohExtEntry;
use super::codec_zenoh_ext_unit::CodecZenohExtUnit;
use super::codec_zenoh_ext_zint::CodecZenohExtZint;
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbuf;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohOamVariant {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohExtZbuf(CodecZenohExtZbuf),
    Default {
        tag: u8,
        body: CodecZenohExtUnit,
    },
}

impl Default for CodecZenohOamVariant {
    fn default() -> Self {
        // Default to the first declared arm's body — every imported
        // codec is `#[derive(Default)]`, so this is infallible. A
        // freshly-constructed envelope is overwritten by `decode()` or
        // by an explicit user assignment before any `encode()` call.
        Self::CodecZenohExtUnit(CodecZenohExtUnit::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
pub struct CodecZenohOam {
    pub header: u8,
    pub id: u16,
    pub extensions: Option<Vec<CodecZenohExtEntry>>,
    pub body: CodecZenohOamVariant,
}

// RFC variant-default-uniformity Atomic β: at least one field's
// `<sce:flags>` carrier declares a wire-MID constant via
// `<sce:flag value="N"/>`. Manual `impl Default` bakes the OR of
// every declared `(value & mask) << bit` into that carrier so a
// freshly-constructed instance carries the wire-MID for its own
// dispatch tag. Fields without declared values fall through to
// `Default::default()` (preserving derive(Default) semantics).
impl Default for CodecZenohOam {
    fn default() -> Self {
        Self {
            header: 0x1fu8,
            id: Default::default(),
            extensions: Default::default(),
            body: CodecZenohOamVariant::default(),
        }
    }
}

#[allow(dead_code)]
impl CodecZenohOam {
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
        let id = cursor.read_vle_u16()?;
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
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match ((header >> 5) & (0x03 as u8)) as u8 {
            0u8 => CodecZenohOamVariant::CodecZenohExtUnit(CodecZenohExtUnit::decode(cursor)?),
            1u8 => CodecZenohOamVariant::CodecZenohExtZint(CodecZenohExtZint::decode(cursor)?),
            2u8 => CodecZenohOamVariant::CodecZenohExtZbuf(CodecZenohExtZbuf::decode(cursor)?),
            other => CodecZenohOamVariant::Default {
                tag: other,
                body: CodecZenohExtUnit::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            id,
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

    pub fn enc(&self) -> u8 {
        (((self.header >> 5) & (0x03 as u8))) as u8
    }

    pub fn set_enc(&mut self, v: u8) {
        let _mask: u8 = (0x03 as u8) << 5;
        let _val: u8 = ((v as u8) & (0x03 as u8)) << 5;
        self.header = (self.header & !_mask) | _val;
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
        let mut r: Vec<u8> = Vec::with_capacity(46);
        r.push(self.header);
        {
            let mut _w = self.id as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        if let Some(_list) = &self.extensions {
            for _e in _list {
                r.extend(_e.encode());
            }
        }
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohOamVariant::CodecZenohExtUnit(b) => {
                r.extend(b.encode());
            }
            CodecZenohOamVariant::CodecZenohExtZint(b) => {
                r.extend(b.encode());
            }
            CodecZenohOamVariant::CodecZenohExtZbuf(b) => {
                r.extend(b.encode());
            }
            CodecZenohOamVariant::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
