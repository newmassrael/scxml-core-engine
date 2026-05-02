// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_variant_session_open::CodecVariantSessionOpen;
use super::codec_variant_session_close::CodecVariantSessionClose;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecVariantDispatchBody {
    CodecVariantSessionOpen(CodecVariantSessionOpen),
    CodecVariantSessionClose(CodecVariantSessionClose),
    Default {
        tag: u8,
        body: CodecVariantSessionClose,
    },
}

impl Default for CodecVariantDispatchBody {
    fn default() -> Self {
        // Default to the first declared arm's body — every imported
        // codec is `#[derive(Default)]`, so this is infallible. A
        // freshly-constructed envelope is overwritten by `decode()` or
        // by an explicit user assignment before any `encode()` call.
        Self::CodecVariantSessionOpen(CodecVariantSessionOpen::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecVariantDispatch {
    pub msg_id: u8,
    pub body: CodecVariantDispatchBody,
}

#[allow(dead_code)]
impl CodecVariantDispatch {
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
        // Decode fixed prefix (RFC §5.B variant primitive B1-β: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let msg_id = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match msg_id {
            1u8 => CodecVariantDispatchBody::CodecVariantSessionOpen(CodecVariantSessionOpen::decode(cursor)?),
            2u8 => CodecVariantDispatchBody::CodecVariantSessionClose(CodecVariantSessionClose::decode(cursor)?),
            other => CodecVariantDispatchBody::Default {
                tag: other,
                body: CodecVariantSessionClose::decode(cursor)?,
            },
        };
        Ok(Self {
            msg_id,
            body,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        // Encode fixed prefix (tag field is part of the prefix). The
        // tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set msg_id / body in
        // sync is the caller's responsibility (v1 keeps the layout
        // simple; future extensions may auto-sync via a typed setter).
        let mut r: Vec<u8> = Vec::with_capacity(3);
        r.push(self.msg_id);
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecVariantDispatchBody::CodecVariantSessionOpen(b) => {
                r.extend(b.encode());
            }
            CodecVariantDispatchBody::CodecVariantSessionClose(b) => {
                r.extend(b.encode());
            }
            CodecVariantDispatchBody::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
