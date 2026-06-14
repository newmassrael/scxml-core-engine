#![doc = "SCE-MAP: codec_zenoh_declaration:54"]
// SCE-MAP: codec_zenoh_declaration:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// RFC §synth-5-B: `VecSink` and the heap-backed `encode_to_vec` facade
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

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohDeclarationVariant<'a> {
    CodecZenohDeclKexpr(CodecZenohDeclKexpr<'a>),
    CodecZenohUndeclKexpr(CodecZenohUndeclKexpr),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriber<'a>),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber<'a>),
    CodecZenohDeclQueryable(CodecZenohDeclQueryable<'a>),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryable<'a>),
    CodecZenohDeclToken(CodecZenohDeclToken<'a>),
    CodecZenohUndeclToken(CodecZenohUndeclToken<'a>),
    CodecZenohDeclFinal(CodecZenohDeclFinal),
    Default {
        tag: u8,
        body: CodecZenohDeclFinal,
    },
}

impl<'a> Default for CodecZenohDeclarationVariant<'a> {
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
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohDeclaration<'a> {
    pub header: u8,
    pub body: CodecZenohDeclarationVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclaration<'a> {
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
    /// bytes (RFC §synth-5-B L494-519).
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        // Decode fixed prefix (RFC §synth-5-B variant primitive: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let header = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match header & 0x1F {
            0u8 => CodecZenohDeclarationVariant::CodecZenohDeclKexpr(CodecZenohDeclKexpr::decode(cursor, (header >> 5) & 0x1)?),
            1u8 => CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(CodecZenohUndeclKexpr::decode(cursor)?),
            2u8 => CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(CodecZenohDeclSubscriber::decode(cursor, (header >> 5) & 0x1)?),
            3u8 => CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriber::decode(cursor, (header >> 7) & 0x1)?),
            4u8 => CodecZenohDeclarationVariant::CodecZenohDeclQueryable(CodecZenohDeclQueryable::decode(cursor, (header >> 5) & 0x1, (header >> 7) & 0x1)?),
            5u8 => CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(CodecZenohUndeclQueryable::decode(cursor, (header >> 7) & 0x1)?),
            6u8 => CodecZenohDeclarationVariant::CodecZenohDeclToken(CodecZenohDeclToken::decode(cursor, (header >> 5) & 0x1)?),
            7u8 => CodecZenohDeclarationVariant::CodecZenohUndeclToken(CodecZenohUndeclToken::decode(cursor, (header >> 7) & 0x1)?),
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

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn set_mid(&mut self, v: u8) {
        self.header = (self.header & !0x1F) | (v & 0x1F);
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
                b.encode(w, (self.header >> 5) & 0x1)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(b) => {
                b.encode(w)?;
            }
            CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(b) => {
                b.encode(w, (self.header >> 5) & 0x1)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(b) => {
                b.encode(w, (self.header >> 7) & 0x1)?;
            }
            CodecZenohDeclarationVariant::CodecZenohDeclQueryable(b) => {
                b.encode(w, (self.header >> 5) & 0x1, (self.header >> 7) & 0x1)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(b) => {
                b.encode(w, (self.header >> 7) & 0x1)?;
            }
            CodecZenohDeclarationVariant::CodecZenohDeclToken(b) => {
                b.encode(w, (self.header >> 5) & 0x1)?;
            }
            CodecZenohDeclarationVariant::CodecZenohUndeclToken(b) => {
                b.encode(w, (self.header >> 7) & 0x1)?;
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

// ── Owned projection (portable native form) ───────────────────────────
// `CodecZenohDeclaration<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohDeclarationOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
// `String` / `Bytes` fields project to the portable runtime aliases
// `SceString<N>` / `SceBytes<N>`: an unbounded `String` / `Vec<u8>` under
// `alloc` (the on-wire protocol caps no payload, so the AP profile must
// not either — `N` is advisory) and the heap-free `heapless::String<N>` /
// `heapless::Vec<u8, N>` (the C11 `char[N]` analog) without it, where `N`
// is the hard capacity. A leaf codec's owned form therefore still compiles
// on a no-alloc MCU; only an unbounded owned `Vec` (list / embed / variant
// body) keeps the `alloc` gate. `try_into_owned` stays the fallible
// direction (one `?` per profile: the `alloc` copy cannot fail, the
// no-alloc copy enforces `N`); `as_borrowed` re-borrows either
// form infallibly via `.as_slice()` / `.as_str()`.
#[cfg(feature = "alloc")]
use super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_decl_token::CodecZenohDeclTokenOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_undecl_token::CodecZenohUndeclTokenOwned;
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohDeclarationOwned {
    pub header: u8,
    pub body: CodecZenohDeclarationOwnedVariant,
}

#[cfg(feature = "alloc")]
#[allow(dead_code)]
impl CodecZenohDeclarationOwned {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so alloc consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn n(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn m(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }
}

#[cfg(feature = "alloc")]
// Variant arms wrap distinct body codecs whose owned mirrors differ in
// field count and size, so the tagged union is inherently size-disparate.
// The lint's only remedy is boxing the large arm, which adds an
// indirection (and allocation) the generated decode path does not need.
// The size spread is the deliberate tagged-union trade-off.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohDeclarationOwnedVariant {
    CodecZenohDeclKexpr(CodecZenohDeclKexprOwned),
    CodecZenohUndeclKexpr(CodecZenohUndeclKexpr),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriberOwned),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriberOwned),
    CodecZenohDeclQueryable(CodecZenohDeclQueryableOwned),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryableOwned),
    CodecZenohDeclToken(CodecZenohDeclTokenOwned),
    CodecZenohUndeclToken(CodecZenohUndeclTokenOwned),
    CodecZenohDeclFinal(CodecZenohDeclFinal),
    Default {
        tag: u8,
        body: CodecZenohDeclFinal,
    },
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohDeclarationVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror. Fallible
    /// because a borrowed arm body's `try_into_owned` re-checks its bounded
    /// fields against their inline capacity (the same bound decode enforces).
    pub fn try_into_owned(self) -> Result<CodecZenohDeclarationOwnedVariant, CodecError> {
        Ok(match self {
            CodecZenohDeclarationVariant::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b.try_into_owned()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b),
            CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b.try_into_owned()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b.try_into_owned()?),
            CodecZenohDeclarationVariant::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b.try_into_owned()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b.try_into_owned()?),
            CodecZenohDeclarationVariant::CodecZenohDeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b.try_into_owned()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b.try_into_owned()?),
            CodecZenohDeclarationVariant::CodecZenohDeclFinal(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b),
            CodecZenohDeclarationVariant::Default { tag, body } => CodecZenohDeclarationOwnedVariant::Default { tag, body },
        })
    }
}

#[cfg(feature = "alloc")]
impl CodecZenohDeclarationOwnedVariant {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of `into_owned`. Reuses the borrowed view's single
    /// `encode`; the owned form deliberately carries no encode of its own.
    pub fn as_borrowed(&self) -> CodecZenohDeclarationVariant<'_> {
        match self {
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationVariant::CodecZenohDeclKexpr(_b.as_borrowed()),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(_b.clone()),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(_b.as_borrowed()),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(_b.as_borrowed()),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationVariant::CodecZenohDeclQueryable(_b.as_borrowed()),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(_b.as_borrowed()),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b) => CodecZenohDeclarationVariant::CodecZenohDeclToken(_b.as_borrowed()),
            CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b) => CodecZenohDeclarationVariant::CodecZenohUndeclToken(_b.as_borrowed()),
            CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b) => CodecZenohDeclarationVariant::CodecZenohDeclFinal(_b.clone()),
            CodecZenohDeclarationOwnedVariant::Default { tag, body } => CodecZenohDeclarationVariant::Default { tag: *tag, body: body.clone() },
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohDeclaration<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohDeclarationOwned`]. Call at a decode boundary when the
    /// decoded value must outlive the input buffer — stored in a long-lived
    /// enum, moved across an async task, or inserted by value into a
    /// bounded-collection. `String` / `Bytes` fields copy into the portable
    /// `SceString<N>` / `SceBytes<N>`: an unbounded heap copy under `alloc`
    /// (`N` advisory), else a fixed `heapless` copy capped at `N`. The
    /// method is fallible for profile uniformity — without `alloc` an
    /// over-`N` view raises `CodecError::TooManyElements` (the same bound
    /// and error decode enforces); under `alloc` the copy never fails. The
    /// borrowed zero-copy path is unaffected.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclarationOwned, CodecError> {
        Ok(CodecZenohDeclarationOwned {
            header: self.header,
            body: self.body.try_into_owned()?,
        })
    }
}

#[cfg(feature = "alloc")]
impl CodecZenohDeclarationOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohDeclaration<'_> {
        CodecZenohDeclaration {
            header: self.header,
            body: self.body.as_borrowed(),
        }
    }
}
