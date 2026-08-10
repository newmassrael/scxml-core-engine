#![doc = "SCE-MAP: codec_zenoh_declaration:54 :: _forge_body"]
// SCE-MAP: codec_zenoh_declaration:54 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// RFC §synth-5-B: `VecSink` and the heap-backed `encode_to_vec` facade
// are gated on the `alloc` feature (see
// `backends/rust/forge-runtime/src/codec.rs`). MCU / `no_std` consumers see
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
    pub const MAX_ENCODED_BYTES: usize = 274;

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
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
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

// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohDeclaration<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohDeclarationOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohDeclarationOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohDeclarationOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohDeclarationOwned` alone is the build's
// default profile. Because the non-allocating profile is a *type*, this mirror
// carries no `alloc` gate at all — a list- or embed-bearing codec has an owned
// form on the heap-free tier too, and a heap-capable consumer can still pin a
// value to storage that is guaranteed not to allocate.
//
// `try_into_owned` is the fallible direction (one `?` per profile: on the
// growable profile the copy cannot fail, on the inline profile it enforces
// each declared bound); `as_borrowed` re-borrows any profile back
// into the single borrowed view that owns `encode`; `transcode_in` moves a
// value between profiles as a checked projection rather than a re-decode.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling one instead names it on the value or its binding —
// `let v: CodecZenohDeclarationOwned = CodecZenohDeclarationOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_decl_kexpr::CodecZenohDeclKexprOwned;
use super::codec_zenoh_decl_subscriber::CodecZenohDeclSubscriberOwned;
use super::codec_zenoh_undecl_subscriber::CodecZenohUndeclSubscriberOwned;
use super::codec_zenoh_decl_queryable::CodecZenohDeclQueryableOwned;
use super::codec_zenoh_undecl_queryable::CodecZenohUndeclQueryableOwned;
use super::codec_zenoh_decl_token::CodecZenohDeclTokenOwned;
use super::codec_zenoh_undecl_token::CodecZenohUndeclTokenOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohDeclarationOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub header: u8,
    pub body: CodecZenohDeclarationOwnedVariant<S>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
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

// Variant arms wrap distinct body codecs whose owned mirrors differ in
// field count and size, so the tagged union is inherently size-disparate.
// The lint's only remedy is boxing the large arm, which adds an
// indirection (and allocation) the generated decode path does not need —
// and which the non-allocating storage profile could not take at all.
// The size spread is the deliberate tagged-union trade-off.
#[allow(clippy::large_enum_variant)]
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohDeclarationOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecZenohDeclKexpr(CodecZenohDeclKexprOwned<S>),
    CodecZenohUndeclKexpr(CodecZenohUndeclKexpr),
    CodecZenohDeclSubscriber(CodecZenohDeclSubscriberOwned<S>),
    CodecZenohUndeclSubscriber(CodecZenohUndeclSubscriberOwned<S>),
    CodecZenohDeclQueryable(CodecZenohDeclQueryableOwned<S>),
    CodecZenohUndeclQueryable(CodecZenohUndeclQueryableOwned<S>),
    CodecZenohDeclToken(CodecZenohDeclTokenOwned<S>),
    CodecZenohUndeclToken(CodecZenohUndeclTokenOwned<S>),
    CodecZenohDeclFinal(CodecZenohDeclFinal),
    Default {
        tag: u8,
        body: CodecZenohDeclFinal,
    },
}

#[allow(dead_code)]
impl<'a> CodecZenohDeclarationVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohDeclarationOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecZenohDeclarationVariant::CodecZenohDeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclKexpr(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclKexpr(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclKexpr(_b),
            CodecZenohDeclarationVariant::CodecZenohDeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclSubscriber(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclSubscriber(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclSubscriber(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohDeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclQueryable(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclQueryable(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclQueryable(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohDeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclToken(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohUndeclToken(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohUndeclToken(_b.try_into_owned_in::<S>()?),
            CodecZenohDeclarationVariant::CodecZenohDeclFinal(_b) => CodecZenohDeclarationOwnedVariant::CodecZenohDeclFinal(_b),
            CodecZenohDeclarationVariant::Default { tag, body } => CodecZenohDeclarationOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclarationOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
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

#[allow(dead_code)]
impl<'a> CodecZenohDeclaration<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohDeclarationOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohDeclarationOwned<S>, CodecError> {
        Ok(CodecZenohDeclarationOwned {
            header: self.header,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohDeclarationOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohDeclarationOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohDeclaration<'_> {
        CodecZenohDeclaration {
            header: self.header,
            body: self.body.as_borrowed(),
        }
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohDeclarationOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}
