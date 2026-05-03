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
pub struct CodecZenohDeclToken {
    pub id: u32,
    pub wireexpr: CodecZenohWireexpr,
}

#[allow(dead_code)]
impl CodecZenohDeclToken {
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
        // Streaming codec: each field reads its own bytes from the
        // cursor (VLE = base-128 1..=ceil(N/7) bytes). No pre-peek of
        // a fixed window; cursor advances per-field. RFC §5.B B4:
        // per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        let id = cursor.read_vle_u32()?;
        let wireexpr = CodecZenohWireexpr::decode(cursor, parent_flags)?;
        Ok(Self {
            id,
            wireexpr,
        })
    }

    pub fn encode(&self, parent_flags: u8) -> Vec<u8> {
        // RFC §5.B B5-γ: see `decode` — same parameter, same suppress.
        let _ = parent_flags;
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef / Tail siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable: the non-gated VLE arm there reuses
        // `vle_encode_block` with the language-appropriate self/
        // struct prefix.
        let mut r: Vec<u8> = Vec::with_capacity(261);
        {
            let mut _w = self.id as u64;
            while _w >= 0x80 {
                r.push((_w as u8 & 0x7F) | 0x80);
                _w >>= 7;
            }
            r.push(_w as u8);
        }
        r.extend(self.wireexpr.encode(parent_flags));
        r
    }
}
