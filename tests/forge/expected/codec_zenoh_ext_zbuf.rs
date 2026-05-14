#![doc = "SCE-MAP: codec_zenoh_ext_zbuf:17"]
// SCE-MAP: codec_zenoh_ext_zbuf:17

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
pub struct CodecZenohExtZbuf {
    pub value_len: u64,
    pub value: Vec<u8>,
}

#[allow(dead_code)]
impl CodecZenohExtZbuf {
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
        let value_len = cursor.read_vle_u64()?;
        let value = {
            let _n = value_len as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw.to_vec();
            cursor.advance(_n)?;
            _v
        };
        Ok(Self {
            value_len,
            value,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef / Tail siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable: the non-gated VLE arm there reuses
        // `vle_encode_block` with the language-appropriate self/
        // struct prefix.
        let mut r: Vec<u8> = Vec::with_capacity(42);
        {
            let mut _w = self.value_len as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        r.extend_from_slice(&self.value);
        r
    }
}
