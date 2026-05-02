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
pub struct CodecPresentIfTail {
    pub flags: u8,
    pub payload: Option<Vec<u8>>,
}

#[allow(dead_code)]
impl CodecPresentIfTail {
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
        // Per-field `is_repeat` routes Repeat fields to the dedicated
        // helper since present-if isn't allowed on `<sce:repeat>`.
        // Note: this branch fires before has_vle_fields so a codec
        // mixing VLE + present-if uses the unified streaming path.
        let flags = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let payload = if (flags & 0x01u8) != 0 {
            let _n = cursor.remaining();
            let raw = cursor.peek_slice(_n)?;
            let _v = raw.to_vec();
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        Ok(Self {
            flags,
            payload,
        })
    }

    // RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    // field. Read returns a bool from `(field & mask) != 0`; write
    // toggles the bit on/off without disturbing siblings on the same
    // carrier. Wire layout is unchanged — the carrier still occupies
    // its declared bytes.
    pub fn has_payload(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    pub fn set_has_payload(&mut self, v: bool) {
        if v {
            self.flags |= 0x01;
        } else {
            self.flags &= !0x01;
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B B1-δ + B2-β present-if encode: every field appends
        // its bytes via a per-field block; gated fields skip the
        // append when the optional is None. Per-field `is_repeat`
        // routes Repeat fields to the dedicated helper. Author keeps
        // the carrier's flag bit and the optional's truth value in
        // sync (trust contract, mirrors the variant primitive).
        // Note: this branch fires before has_vle_fields so a codec
        // mixing VLE + present-if uses the unified encode path.
        let mut r: Vec<u8> = Vec::with_capacity(65);
        r.push(self.flags);
        if let Some(_v) = &self.payload {
            r.extend_from_slice(_v);
        }
        r
    }
}
