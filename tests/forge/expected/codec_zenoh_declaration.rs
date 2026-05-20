#![doc = "SCE-MAP: codec_zenoh_declaration:54"]
// SCE-MAP: codec_zenoh_declaration:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// RFC §5.B B1-α: `VecSink` and the heap-backed `encode_to_vec` facade
// are gated on the `alloc` feature (see
// `sce-forge-runtime/rust/src/codec.rs`). MCU / `no_std` consumers see
// only the sink-based primary `encode` + `SliceSink` paths.
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use sce_forge_runtime::codec::VecSink;

use super::codec_zenoh_decl_kexpr::CodecZenohDeclKexpr;
use super::codec_zenoh_undecl_kexpr::CodecZenohUndeclKexpr;
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriber;
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriber;
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryable;
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryable;
use super::codec_zenoh_decl_token::CodecZenohDeclToken;
use super::codec_zenoh_undecl_token::CodecZenohUndeclToken;
use super::codec_zenoh_decl_final::CodecZenohDeclFinal;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
pub enum CodecZenohDeclarationVariant {
    CodecZenohDeclKexpr(CodecZenohDeclKexpr),
    CodecZenohUndeclKexpr(CodecZenohUndeclKexpr),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriber),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber),
    CodecZenohDeclQueryable(CodecZenohDeclQueryable),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryable),
    CodecZenohDeclToken(CodecZenohDeclToken),
    CodecZenohUndeclToken(CodecZenohUndeclToken),
    CodecZenohDeclFinal(CodecZenohDeclFinal),
    Default {
        tag: u8,
        body: CodecZenohDeclFinal,
    },
}

impl Default for CodecZenohDeclarationVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohDeclFinal(CodecZenohDeclFinal::default())
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
            0u8 => CodecZenohDeclarationVariant::CodecZenohDeclKexpr(CodecZenohDeclKexpr::decode(cursor, ((header >> 5) & 0x1) as u8)?),
            1u8 => CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(CodecZenohUndeclKexpr::decode(cursor)?),
            2u8 => CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(CodecZenohDeclSubscriber::decode(cursor, ((header >> 5) & 0x1) as u8)?),
            3u8 => CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber::decode(cursor, ((header >> 7) & 0x1) as u8)?),
            4u8 => CodecZenohDeclarationVariant::CodecZenohDeclQueryable(CodecZenohDeclQueryable::decode(cursor, ((header >> 5) & 0x1) as u8, ((header >> 7) & 0x1) as u8)?),
            5u8 => CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(CodecZenohUndeclQueryable::decode(cursor, ((header >> 7) & 0x1) as u8)?),
            6u8 => CodecZenohDeclarationVariant::CodecZenohDeclToken(CodecZenohDeclToken::decode(cursor, ((header >> 5) & 0x1) as u8)?),
            7u8 => CodecZenohDeclarationVariant::CodecZenohUndeclToken(CodecZenohUndeclToken::decode(cursor, ((header >> 7) & 0x1) as u8)?),
            26u8 => CodecZenohDeclarationVariant::CodecZenohDeclFinal(CodecZenohDeclFinal::decode(cursor)?),
            other => CodecZenohDeclarationVariant::Default {
                tag: other,
                body: CodecZenohDeclFinal::decode(cursor)?,
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 275;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // Encode fixed prefix (tag field is part of the prefix). The
        // tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set msg_id / body in
        // sync is the caller's responsibility (v1 keeps the layout
        // simple; future extensions may auto-sync via a typed setter).
        w.write_u8(self.header)?;
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohDeclarationVariant::CodecZenohDeclKexpr(b) => {
                b.encode(w, ((self.header >> 5) & 0x1) as u8)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(b) => {
                b.encode(w)?;
            }
            CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(b) => {
                b.encode(w, ((self.header >> 5) & 0x1) as u8)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(b) => {
                b.encode(w, ((self.header >> 7) & 0x1) as u8)?;
            }
            CodecZenohDeclarationVariant::CodecZenohDeclQueryable(b) => {
                b.encode(w, ((self.header >> 5) & 0x1) as u8, ((self.header >> 7) & 0x1) as u8)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(b) => {
                b.encode(w, ((self.header >> 7) & 0x1) as u8)?;
            }
            CodecZenohDeclarationVariant::CodecZenohDeclToken(b) => {
                b.encode(w, ((self.header >> 5) & 0x1) as u8)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclToken(b) => {
                b.encode(w, ((self.header >> 7) & 0x1) as u8)?;
            }
            CodecZenohDeclarationVariant::CodecZenohDeclFinal(b) => {
                b.encode(w)?;
            }
            CodecZenohDeclarationVariant::Default { body, .. } => {
                body.encode(w)?;
            }
        }
        Ok(())
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector. Callers
    /// targeting zero-alloc hot paths should call `encode` directly
    /// against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `sce-forge-runtime/rust/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)
            .expect("VecSink is infallible");
        _sce_v
    }
}
