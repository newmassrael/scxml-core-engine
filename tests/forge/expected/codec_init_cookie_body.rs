#![doc = "SCE-MAP: codec_init_cookie_body:36"]
// SCE-MAP: codec_init_cookie_body:36

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
pub struct CodecInitCookieBody {
    pub version: u8,
    pub cookie_size: Option<u16>,
    pub cookie: Option<Vec<u8>>,
}

#[allow(dead_code)]
impl CodecInitCookieBody {
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
        let cookie_size = if (parent_flags & 0x20u8) != 0 {
            let _v = cursor.read_vle_u16()?;
            Some(_v)
        } else {
            None
        };
        let cookie = if (parent_flags & 0x20u8) != 0 {
            let _n = cookie_size.unwrap() as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw.to_vec();
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        Ok(Self {
            version,
            cookie_size,
            cookie,
        })
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
        let mut r: Vec<u8> = Vec::with_capacity(68);
        r.push(self.version);
        if let Some(_v) = self.cookie_size {
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
