#![doc = "SCE-MAP: codec_zenoh_declaration:54"]
// SCE-MAP: codec_zenoh_declaration:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor};

use super::codec_zenoh_decl_keyexpr::CodecZenohDeclKeyexpr;
use super::codec_zenoh_undecl_keyexpr::CodecZenohUndeclKeyexpr;
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriber;
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriber;
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryable;
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryable;
use super::codec_zenoh_decl_token::CodecZenohDeclToken;
use super::codec_zenoh_undecl_token::CodecZenohUndeclToken;
use super::codec_decl_final::CodecDeclFinal;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohDeclarationVariant {
    CodecZenohDeclKeyexpr(CodecZenohDeclKeyexpr),
    CodecZenohUndeclKeyexpr(CodecZenohUndeclKeyexpr),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriber),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber),
    CodecZenohDeclQueryable(CodecZenohDeclQueryable),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryable),
    CodecZenohDeclToken(CodecZenohDeclToken),
    CodecZenohUndeclToken(CodecZenohUndeclToken),
    CodecDeclFinal(CodecDeclFinal),
    Default {
        tag: u8,
        body: CodecDeclFinal,
    },
}

impl Default for CodecZenohDeclarationVariant {
    fn default() -> Self {
        // Default to the first declared arm's body — every imported
        // codec is `#[derive(Default)]`, so this is infallible. A
        // freshly-constructed envelope is overwritten by `decode()` or
        // by an explicit user assignment before any `encode()` call.
        Self::CodecZenohDeclKeyexpr(CodecZenohDeclKeyexpr::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecZenohDeclaration {
    pub header: u8,
    pub body: CodecZenohDeclarationVariant,
}

#[allow(dead_code)]
impl CodecZenohDeclaration {
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
        let header = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match ((header >> 0) & (0x1F as u8)) as u8 {
            0u8 => CodecZenohDeclarationVariant::CodecZenohDeclKeyexpr(CodecZenohDeclKeyexpr::decode(cursor, header)?),
            1u8 => CodecZenohDeclarationVariant::CodecZenohUndeclKeyexpr(CodecZenohUndeclKeyexpr::decode(cursor)?),
            2u8 => CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(CodecZenohDeclSubscriber::decode(cursor, header)?),
            3u8 => CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber::decode(cursor, header)?),
            4u8 => CodecZenohDeclarationVariant::CodecZenohDeclQueryable(CodecZenohDeclQueryable::decode(cursor, header)?),
            5u8 => CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(CodecZenohUndeclQueryable::decode(cursor, header)?),
            6u8 => CodecZenohDeclarationVariant::CodecZenohDeclToken(CodecZenohDeclToken::decode(cursor, header)?),
            7u8 => CodecZenohDeclarationVariant::CodecZenohUndeclToken(CodecZenohUndeclToken::decode(cursor, header)?),
            26u8 => CodecZenohDeclarationVariant::CodecDeclFinal(CodecDeclFinal::decode(cursor)?),
            other => CodecZenohDeclarationVariant::Default {
                tag: other,
                body: CodecDeclFinal::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            body,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn mid(&self) -> u8 {
        (((self.header >> 0) & (0x1F as u8))) as u8
    }

    pub fn set_mid(&mut self, v: u8) {
        let _mask: u8 = (0x1F as u8) << 0;
        let _val: u8 = ((v as u8) & (0x1F as u8)) << 0;
        self.header = (self.header & !_mask) | _val;
    }

    pub fn n(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_n(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn m(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_m(&mut self, v: bool) {
        if v {
            self.header |= 0x40;
        } else {
            self.header &= !0x40;
        }
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_z(&mut self, v: bool) {
        if v {
            self.header |= 0x80;
        } else {
            self.header &= !0x80;
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        // Encode fixed prefix (tag field is part of the prefix). The
        // tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set msg_id / body in
        // sync is the caller's responsibility (v1 keeps the layout
        // simple; future extensions may auto-sync via a typed setter).
        let mut r: Vec<u8> = Vec::with_capacity(275);
        r.push(self.header);
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohDeclarationVariant::CodecZenohDeclKeyexpr(b) => {
                r.extend(b.encode(self.header));
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclKeyexpr(b) => {
                r.extend(b.encode());
            }
            CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(b) => {
                r.extend(b.encode(self.header));
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(b) => {
                r.extend(b.encode(self.header));
            }
            CodecZenohDeclarationVariant::CodecZenohDeclQueryable(b) => {
                r.extend(b.encode(self.header));
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(b) => {
                r.extend(b.encode(self.header));
            }
            CodecZenohDeclarationVariant::CodecZenohDeclToken(b) => {
                r.extend(b.encode(self.header));
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclToken(b) => {
                r.extend(b.encode(self.header));
            }
            CodecZenohDeclarationVariant::CodecDeclFinal(b) => {
                r.extend(b.encode());
            }
            CodecZenohDeclarationVariant::Default { body, .. } => {
                r.extend(body.encode());
            }
        }
        r
    }
}
