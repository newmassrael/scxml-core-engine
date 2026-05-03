// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_wireexpr::CodecZenohWireexpr;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohInterestBody {
    pub id: u32,
    pub header: u8,
    pub keyexpr: Option<CodecZenohWireexpr>,
}

#[allow(dead_code)]
impl CodecZenohInterestBody {
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
        // RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
        // advances the cursor per field. Gated fields wrap their
        // read inside an `if predicate { Some(...) } else { None }`
        // block computed at codegen time from the carrier field's
        // flag bit. B2-β extends gated fields to Tail / LengthRef /
        // Vle bit-sizes via dispatch inside `present_if_decode_stmt`.
        // Per-field `is_repeat` / `is_tlv_chain` route Repeat / TLV
        // chain fields to their dedicated helpers since present-if
        // isn't allowed on `<sce:repeat>` / `<sce:tlv-chain>`.
        // Note: this branch fires before has_vle_fields so a codec
        // mixing VLE + present-if uses the unified streaming path.
        let id = cursor.read_vle_u32()?;
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let keyexpr = if (header & 0x10u8) != 0 {
            Some(CodecZenohWireexpr::decode(cursor, header)?)
        } else {
            None
        };
        Ok(Self {
            id,
            header,
            keyexpr,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn keyexprs(&self) -> bool {
        (self.header & 0x01) != 0
    }

    pub fn set_keyexprs(&mut self, v: bool) {
        if v {
            self.header |= 0x01;
        } else {
            self.header &= !0x01;
        }
    }

    pub fn subscribers(&self) -> bool {
        (self.header & 0x02) != 0
    }

    pub fn set_subscribers(&mut self, v: bool) {
        if v {
            self.header |= 0x02;
        } else {
            self.header &= !0x02;
        }
    }

    pub fn queryables(&self) -> bool {
        (self.header & 0x04) != 0
    }

    pub fn set_queryables(&mut self, v: bool) {
        if v {
            self.header |= 0x04;
        } else {
            self.header &= !0x04;
        }
    }

    pub fn tokens(&self) -> bool {
        (self.header & 0x08) != 0
    }

    pub fn set_tokens(&mut self, v: bool) {
        if v {
            self.header |= 0x08;
        } else {
            self.header &= !0x08;
        }
    }

    pub fn restricted(&self) -> bool {
        (self.header & 0x10) != 0
    }

    pub fn set_restricted(&mut self, v: bool) {
        if v {
            self.header |= 0x10;
        } else {
            self.header &= !0x10;
        }
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

    pub fn aggregate(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_aggregate(&mut self, v: bool) {
        if v {
            self.header |= 0x80;
        } else {
            self.header &= !0x80;
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B B1-δ + B2-β present-if encode: every field appends
        // its bytes via a per-field block; gated fields skip the
        // append when the optional is None. Per-field `is_repeat` /
        // `is_tlv_chain` route Repeat / TLV chain fields to their
        // dedicated helpers. Author keeps the carrier's flag bit and
        // the optional's truth value in sync (trust contract, mirrors
        // the variant primitive). Note: this branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified encode path.
        let mut r: Vec<u8> = Vec::with_capacity(263);
        {
            let mut _w = self.id as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        r.push(self.header);
        if (header & 0x10u8) != 0 {
            if let Some(_v) = &self.keyexpr {
                r.extend(_v.encode(self.header));
            }
        }
        r
    }
}
