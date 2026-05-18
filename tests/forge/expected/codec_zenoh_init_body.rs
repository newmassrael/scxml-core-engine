#![doc = "SCE-MAP: codec_zenoh_init_body:42"]
// SCE-MAP: codec_zenoh_init_body:42

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohInitBody {
    pub version: u8,
    pub cbyte: u8,
    pub zid: Vec<u8>,
    pub sn_res: Option<u8>,
    pub batch_size: Option<u16>,
    pub cookie_len: Option<u64>,
    pub cookie: Option<Vec<u8>>,
}

#[allow(dead_code)]
impl CodecZenohInitBody {
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
    pub fn decode(cursor: &mut SceCursor<'_>, parent_flags: u8) -> Result<Self, CodecError> {
        // RFC §5.B B5-γ: `parent_flags` is the parent codec's flags
        // carrier value, threaded through by the variant arm dispatcher.
        // Body fields gated via `parent.<flag>` predicates read from
        // this parameter; suppress unused-variable warnings when no
        // gated field happens to consume it (defensive guard for
        // codecs that declare `<sce:requires-parent-flags>` but
        // don't yet wire any predicate to it).
        let _ = parent_flags;
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
        let version = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let cbyte = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let zid = {
            let _n = (((cbyte >> 4) & 0xF) as usize).wrapping_add(1);
            let raw = cursor.peek_slice(_n)?;
            let _v = raw.to_vec();
            cursor.advance(_n)?;
            _v
        };
        let sn_res = if (parent_flags & 0x40u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let batch_size = if (parent_flags & 0x40u8) != 0 {
            let raw = cursor.peek_slice(2)?;
            let _v = raw[0] as u16 | ((raw[1] as u16) << 8);
            cursor.advance(2)?;
            Some(_v)
        } else {
            None
        };
        let cookie_len = if (parent_flags & 0x20u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let cookie = if (parent_flags & 0x20u8) != 0 {
            let _n = cookie_len.unwrap() as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw.to_vec();
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        Ok(Self {
            version,
            cbyte,
            zid,
            sn_res,
            batch_size,
            cookie_len,
            cookie,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn whatami(&self) -> u8 {
        (((self.cbyte >> 0) & (0x03 as u8))) as u8
    }

    pub fn set_whatami(&mut self, v: u8) {
        let _mask: u8 = (0x03 as u8) << 0;
        let _val: u8 = ((v as u8) & (0x03 as u8)) << 0;
        self.cbyte = (self.cbyte & !_mask) | _val;
    }

    pub fn zid_len_m1(&self) -> u8 {
        (((self.cbyte >> 4) & (0x0F as u8))) as u8
    }

    pub fn set_zid_len_m1(&mut self, v: u8) {
        let _mask: u8 = (0x0F as u8) << 4;
        let _val: u8 = ((v as u8) & (0x0F as u8)) << 4;
        self.cbyte = (self.cbyte & !_mask) | _val;
    }

    pub fn encode(&self, parent_flags: u8) -> Vec<u8> {
        // RFC §5.B B5-γ: see `decode` — same parameter, same suppress.
        let _ = parent_flags;
        // RFC §5.B B1-δ + B2-β present-if encode: every field appends
        // its bytes via a per-field block; gated fields skip the
        // append when the optional is None. Per-field `is_repeat` /
        // `is_tlv_chain` route Repeat / TLV chain fields to their
        // dedicated helpers. Author keeps the carrier's flag bit and
        // the optional's truth value in sync (trust contract, mirrors
        // the variant primitive). Note: this branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified encode path.
        let mut r: Vec<u8> = Vec::with_capacity(160);
        r.push(self.version);
        r.push(self.cbyte);
        r.extend_from_slice(&self.zid);
        if let Some(_v) = self.sn_res {
            r.push(_v);
        }
        if let Some(_v) = self.batch_size {
            r.push(_v as u8);
            r.push((_v >> 8) as u8);
        }
        if let Some(_v) = self.cookie_len {
        {
            let mut _w = _v as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        }
        if let Some(_v) = &self.cookie {
            r.extend_from_slice(_v);
        }
        r
    }
}
