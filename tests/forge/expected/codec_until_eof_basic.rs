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
pub struct CodecUntilEofBasic {
    pub msgs: Vec<CodecRepeatElem>,
}

#[allow(dead_code)]
impl CodecUntilEofBasic {
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
        // RFC §5.B B2 repeat primitive (trunk): streaming decode
        // mixes plain fixed-width reads (per-field via the present-if
        // helper's non-gated arm) with repeat loops that iterate the
        // imported codec's `decode()` either `count_ref` times
        // (length-field) or until cursor exhaustion (until-eof).
        // Element bodies recurse into their own codec — each may
        // itself surface NeedMoreBytes, unwinding the partial frame.
        let msgs = {
            let mut _vec: Vec<CodecRepeatElem> = Vec::new();
            while cursor.remaining() > 0 {
                _vec.push(CodecRepeatElem::decode(cursor)?);
            }
            _vec
        };
        Ok(Self {
            msgs,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B B2 repeat encode: fixed prefix fields append byte-
        // by-byte; repeat fields iterate the host-language list and
        // splice each element's encode() into the parent buffer.
        // Author keeps the count field's value consistent with the
        // list length (same trust contract as variant tag/body).
        let mut r: Vec<u8> = Vec::with_capacity(128);
        for _e in &self.msgs {
            r.extend(_e.encode());
        }
        r
    }
}
