#![doc = "SCE-MAP: codec_zenoh_decl_ext_keyexpr:89"]
// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_decl_ext_keyexpr_inner::CodecZenohDeclExtKeyexprInner;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohDeclExtKeyexpr {
    pub outer_header: u8,
    pub total_length: u64,
    pub inner: CodecZenohDeclExtKeyexprInner,
}

#[allow(dead_code)]
impl CodecZenohDeclExtKeyexpr {
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
        // Streaming codec: each field reads its own bytes from the
        // cursor (VLE = base-128 1..=ceil(N/7) bytes). No pre-peek of
        // a fixed window; cursor advances per-field. RFC §5.B B4:
        // per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        let outer_header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let total_length = cursor.read_vle_u64()?;
        let inner = {
            let _len = total_length as usize;
            let _raw = cursor.peek_slice(_len)?;
            let mut _inner = SceCursor::new(_raw);
            let _v = CodecZenohDeclExtKeyexprInner::decode(&mut _inner)?;
            cursor.advance(_len)?;
            _v
        };
        Ok(Self {
            outer_header,
            total_length,
            inner,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn ext_id(&self) -> u8 {
        (((self.outer_header >> 0) & (0x0F as u8))) as u8
    }

    pub fn set_ext_id(&mut self, v: u8) {
        let _mask: u8 = (0x0F as u8) << 0;
        let _val: u8 = ((v as u8) & (0x0F as u8)) << 0;
        self.outer_header = (self.outer_header & !_mask) | _val;
    }

    pub fn m(&self) -> bool {
        (self.outer_header & 0x10) != 0
    }

    pub fn set_m(&mut self, v: bool) {
        if v {
            self.outer_header |= 0x10;
        } else {
            self.outer_header &= !0x10;
        }
    }

    pub fn enc(&self) -> u8 {
        (((self.outer_header >> 5) & (0x03 as u8))) as u8
    }

    pub fn set_enc(&mut self, v: u8) {
        let _mask: u8 = (0x03 as u8) << 5;
        let _val: u8 = ((v as u8) & (0x03 as u8)) << 5;
        self.outer_header = (self.outer_header & !_mask) | _val;
    }

    pub fn z(&self) -> bool {
        (self.outer_header & 0x80) != 0
    }

    pub fn set_z(&mut self, v: bool) {
        if v {
            self.outer_header |= 0x80;
        } else {
            self.outer_header &= !0x80;
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef / Tail siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable: the non-gated VLE arm there reuses
        // `vle_encode_block` with the language-appropriate self/
        // struct prefix.
        let mut r: Vec<u8> = Vec::with_capacity(267);
        r.push(self.outer_header);
        {
            let mut _w = self.total_length as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        r.extend(self.inner.encode());
        r
    }
}
