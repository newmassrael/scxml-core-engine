#![doc = "SCE-MAP: codec_repeat_present_if_basic:37"]
// SCE-MAP: codec_repeat_present_if_basic:37

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_repeat_elem::CodecRepeatElem;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecRepeatPresentIfBasic {
    pub carrier: u8,
    pub num_elems: Option<u8>,
    pub elems: Option<Vec<CodecRepeatElem>>,
}

#[allow(dead_code)]
impl CodecRepeatPresentIfBasic {
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
        let carrier = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let num_elems = if (carrier & 0x01u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let elems = if (carrier & 0x01u8) != 0 {
            let _n = num_elems.expect("co-gating: count present-if matches repeat");
            let mut _vec: Vec<CodecRepeatElem> = Vec::with_capacity(_n as usize);
            for _ in 0.._n {
                _vec.push(CodecRepeatElem::decode(cursor)?);
            }
            Some(_vec)
        } else {
            None
        };
        Ok(Self {
            carrier,
            num_elems,
            elems,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn has_list(&self) -> bool {
        (self.carrier & 0x01) != 0
    }

    pub fn set_has_list(&mut self, v: bool) {
        if v {
            self.carrier |= 0x01;
        } else {
            self.carrier &= !0x01;
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
        let mut r: Vec<u8> = Vec::with_capacity(66);
        r.push(self.carrier);
        if let Some(_v) = self.num_elems {
            r.push(_v);
        }
        if let Some(_list) = &self.elems {
            for _e in _list {
                r.extend(_e.encode());
            }
        }
        r
    }
}
