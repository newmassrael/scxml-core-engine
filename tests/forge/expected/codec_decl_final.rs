#![doc = "SCE-MAP: codec_decl_final:19"]
// SCE-MAP: codec_decl_final:19

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
pub struct CodecDeclFinal {
}

#[allow(dead_code)]
impl CodecDeclFinal {
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
        // RFC §5.B B5-α empty body — nothing to read; the surrounding
        // header byte (and the wire-protocol layer that demuxes it)
        // already proved this codec was selected, so return the
        // default-constructed envelope without touching the cursor.
        let _ = cursor;
        Ok(Self {})
    }

    pub fn encode(&self) -> Vec<u8> {
        // RFC §5.B B5-α empty body — zero-byte payload (the surrounding
        // wire-protocol header byte alone marks this codec on the wire).
        Vec::new()
    }
}
