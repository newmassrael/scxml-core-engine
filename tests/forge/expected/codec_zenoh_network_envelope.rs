#![doc = "SCE-MAP: codec_zenoh_network_envelope:60"]
// SCE-MAP: codec_zenoh_network_envelope:60

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

use super::codec_zenoh_interest::CodecZenohInterest;
use super::codec_zenoh_response_final::CodecZenohResponseFinal;
use super::codec_zenoh_response::CodecZenohResponse;
use super::codec_zenoh_request::CodecZenohRequest;
use super::codec_zenoh_push::CodecZenohPush;
use super::codec_zenoh_declare::CodecZenohDeclare;
use super::codec_zenoh_oam::CodecZenohOam;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohNetworkEnvelopeVariant<'a> {
    CodecZenohInterest(CodecZenohInterest<'a>),
    CodecZenohResponseFinal(CodecZenohResponseFinal<'a>),
    CodecZenohResponse(CodecZenohResponse<'a>),
    CodecZenohRequest(CodecZenohRequest<'a>),
    CodecZenohPush(CodecZenohPush),
    CodecZenohDeclare(CodecZenohDeclare<'a>),
    CodecZenohOam(CodecZenohOam<'a>),
    Default {
        tag: u8,
        body: CodecZenohOam<'a>,
    },
}

impl<'a> Default for CodecZenohNetworkEnvelopeVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohOam(CodecZenohOam::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohNetworkEnvelope<'a> {
    pub body: CodecZenohNetworkEnvelopeVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecZenohNetworkEnvelope<'a> {
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
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for the variant
        // tag without advancing — the arm body decoder reads the peeked
        // byte as its own header byte (Zenoh response/request body MID
        // dispatch shape per network.c:347-364 + 220-235).
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match _peek & 0x1F {
            25u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohInterest(CodecZenohInterest::decode(cursor)?),
            26u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohResponseFinal(CodecZenohResponseFinal::decode(cursor)?),
            27u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohResponse(CodecZenohResponse::decode(cursor)?),
            28u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohRequest(CodecZenohRequest::decode(cursor)?),
            29u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohPush(CodecZenohPush::decode(cursor)?),
            30u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohDeclare(CodecZenohDeclare::decode(cursor)?),
            31u8 => CodecZenohNetworkEnvelopeVariant::CodecZenohOam(CodecZenohOam::decode(cursor)?),
            other => CodecZenohNetworkEnvelopeVariant::Default {
                tag: other,
                body: CodecZenohOam::decode(cursor)?,
            },
        };
        Ok(Self {
            body,
        })
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 1212;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix encode (per-field present_if/tlv-chain/embed/
        // repeat helpers). Peek-byte mode: the arm body's encode prepends
        // its own header byte (which the decoder peeked); no separate
        // tag byte is emitted here. Streaming-prefix mode (own-field
        // variant): the carrier is part of the prefix fields and emits
        // through the same per-field path.
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohNetworkEnvelopeVariant::CodecZenohInterest(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponseFinal(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponse(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohRequest(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohPush(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohDeclare(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::CodecZenohOam(b) => {
                b.encode(w)?;
            }
            CodecZenohNetworkEnvelopeVariant::Default { body, .. } => {
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
// `CodecZenohNetworkEnvelope<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohNetworkEnvelopeOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohNetworkEnvelopeOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohNetworkEnvelopeOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohNetworkEnvelopeOwned` alone is the build's
// default profile. Because the non-allocating profile is a *type*, this mirror
// carries no `alloc` gate at all — a list- or embed-bearing codec has an owned
// form on the heap-free tier too, and a heap-capable consumer can still pin a
// value to storage that is guaranteed not to allocate.
//
// `try_into_owned` is the fallible direction (one `?` per profile: on the
// growable profile the copy cannot fail, on the inline profile it enforces
// each declared bound); `try_as_borrowed` re-borrows any profile back
// into the single borrowed view that owns `encode`; `transcode_in` moves a
// value between profiles as a checked projection rather than a re-decode.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling one instead names it on the value or its binding —
// `let v: CodecZenohNetworkEnvelopeOwned = CodecZenohNetworkEnvelopeOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_interest::CodecZenohInterestOwned;
use super::codec_zenoh_response_final::CodecZenohResponseFinalOwned;
use super::codec_zenoh_response::CodecZenohResponseOwned;
use super::codec_zenoh_request::CodecZenohRequestOwned;
use super::codec_zenoh_declare::CodecZenohDeclareOwned;
use super::codec_zenoh_oam::CodecZenohOamOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohNetworkEnvelopeOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub body: CodecZenohNetworkEnvelopeOwnedVariant<S>,
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
pub enum CodecZenohNetworkEnvelopeOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecZenohInterest(CodecZenohInterestOwned<S>),
    CodecZenohResponseFinal(CodecZenohResponseFinalOwned<S>),
    CodecZenohResponse(CodecZenohResponseOwned<S>),
    CodecZenohRequest(CodecZenohRequestOwned<S>),
    CodecZenohPush(CodecZenohPush),
    CodecZenohDeclare(CodecZenohDeclareOwned<S>),
    CodecZenohOam(CodecZenohOamOwned<S>),
    Default {
        tag: u8,
        body: CodecZenohOamOwned<S>,
    },
}

#[allow(dead_code)]
impl<'a> CodecZenohNetworkEnvelopeVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohNetworkEnvelopeOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecZenohNetworkEnvelopeVariant::CodecZenohInterest(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohInterest(_b.try_into_owned_in::<S>()?),
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponseFinal(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponseFinal(_b.try_into_owned_in::<S>()?),
            CodecZenohNetworkEnvelopeVariant::CodecZenohResponse(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponse(_b.try_into_owned_in::<S>()?),
            CodecZenohNetworkEnvelopeVariant::CodecZenohRequest(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohRequest(_b.try_into_owned_in::<S>()?),
            CodecZenohNetworkEnvelopeVariant::CodecZenohPush(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohPush(_b),
            CodecZenohNetworkEnvelopeVariant::CodecZenohDeclare(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohDeclare(_b.try_into_owned_in::<S>()?),
            CodecZenohNetworkEnvelopeVariant::CodecZenohOam(_b) => CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohOam(_b.try_into_owned_in::<S>()?),
            CodecZenohNetworkEnvelopeVariant::Default { tag, body } => CodecZenohNetworkEnvelopeOwnedVariant::Default { tag, body: body.try_into_owned_in::<S>()? },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohNetworkEnvelopeOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohNetworkEnvelopeOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohNetworkEnvelopeVariant<'_>, CodecError> {
        Ok(match self {
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohInterest(_b) => CodecZenohNetworkEnvelopeVariant::CodecZenohInterest(_b.try_as_borrowed()?),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponseFinal(_b) => CodecZenohNetworkEnvelopeVariant::CodecZenohResponseFinal(_b.try_as_borrowed()?),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohResponse(_b) => CodecZenohNetworkEnvelopeVariant::CodecZenohResponse(_b.try_as_borrowed()?),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohRequest(_b) => CodecZenohNetworkEnvelopeVariant::CodecZenohRequest(_b.try_as_borrowed()?),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohPush(_b) => CodecZenohNetworkEnvelopeVariant::CodecZenohPush(_b.clone()),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohDeclare(_b) => CodecZenohNetworkEnvelopeVariant::CodecZenohDeclare(_b.try_as_borrowed()?),
            CodecZenohNetworkEnvelopeOwnedVariant::CodecZenohOam(_b) => CodecZenohNetworkEnvelopeVariant::CodecZenohOam(_b.try_as_borrowed()?),
            CodecZenohNetworkEnvelopeOwnedVariant::Default { tag, body } => CodecZenohNetworkEnvelopeVariant::Default { tag: *tag, body: body.try_as_borrowed()? },
        })
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohNetworkEnvelope<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohNetworkEnvelopeOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohNetworkEnvelopeOwned<S>, CodecError> {
        Ok(CodecZenohNetworkEnvelopeOwned {
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohNetworkEnvelopeOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohNetworkEnvelopeOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohNetworkEnvelope<'_>, CodecError> {
        Ok(CodecZenohNetworkEnvelope {
            body: self.body.try_as_borrowed()?,
        })
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohNetworkEnvelopeOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
