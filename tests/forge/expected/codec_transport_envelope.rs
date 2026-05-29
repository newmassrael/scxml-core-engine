#![doc = "SCE-MAP: codec_transport_envelope:69"]
// SCE-MAP: codec_transport_envelope:69

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

use super::codec_zenoh_init_body::CodecZenohInitBody;
use super::codec_zenoh_open_body::CodecZenohOpenBody;
use super::codec_zenoh_close::CodecZenohClose;
use super::codec_zenoh_keep_alive::CodecZenohKeepAlive;
use super::codec_zenoh_frame::CodecZenohFrame;
use super::codec_zenoh_fragment::CodecZenohFragment;
use super::codec_zenoh_join::CodecZenohJoin;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecTransportEnvelopeVariant<'a> {
    CodecZenohInitBody(CodecZenohInitBody<'a>),
    CodecZenohOpenBody(CodecZenohOpenBody<'a>),
    CodecZenohClose(CodecZenohClose),
    CodecZenohKeepAlive(CodecZenohKeepAlive),
    CodecZenohFrame(CodecZenohFrame<'a>),
    CodecZenohFragment(CodecZenohFragment<'a>),
    CodecZenohJoin(CodecZenohJoin<'a>),
    Default {
        tag: u8,
        body: CodecZenohClose,
    },
}

impl<'a> Default for CodecTransportEnvelopeVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohClose(CodecZenohClose::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecTransportEnvelope<'a> {
    pub header: u8,
    pub body: CodecTransportEnvelopeVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecTransportEnvelope<'a> {
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
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        // Decode fixed prefix (RFC §5.B variant primitive B1-β: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let header = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match header & 0x1F {
            1u8 => CodecTransportEnvelopeVariant::CodecZenohInitBody(CodecZenohInitBody::decode(cursor, (header >> 6) & 0x1, (header >> 5) & 0x1)?),
            2u8 => CodecTransportEnvelopeVariant::CodecZenohOpenBody(CodecZenohOpenBody::decode(cursor, (header >> 5) & 0x1)?),
            3u8 => CodecTransportEnvelopeVariant::CodecZenohClose(CodecZenohClose::decode(cursor)?),
            4u8 => CodecTransportEnvelopeVariant::CodecZenohKeepAlive(CodecZenohKeepAlive::decode(cursor)?),
            5u8 => CodecTransportEnvelopeVariant::CodecZenohFrame(CodecZenohFrame::decode(cursor)?),
            6u8 => CodecTransportEnvelopeVariant::CodecZenohFragment(CodecZenohFragment::decode(cursor)?),
            7u8 => CodecTransportEnvelopeVariant::CodecZenohJoin(CodecZenohJoin::decode(cursor, (header >> 6) & 0x1)?),
            other => CodecTransportEnvelopeVariant::Default {
                tag: other,
                body: CodecZenohClose::decode(cursor)?,
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
        self.header & 0x1F
    }

    pub fn set_mid(&mut self, v: u8) {
        self.header = (self.header & !0x1F) | (v & 0x1F);
    }

    pub fn a(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_a(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn s(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_s(&mut self, v: bool) {
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
    pub const MAX_ENCODED_BYTES: usize = 65547;

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
            CodecTransportEnvelopeVariant::CodecZenohInitBody(b) => {
                b.encode(w, (self.header >> 6) & 0x1, (self.header >> 5) & 0x1)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohOpenBody(b) => {
                b.encode(w, (self.header >> 5) & 0x1)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohClose(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohKeepAlive(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohFrame(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohFragment(b) => {
                b.encode(w)?;
            }
            CodecTransportEnvelopeVariant::CodecZenohJoin(b) => {
                b.encode(w, (self.header >> 6) & 0x1)?;
            }
            CodecTransportEnvelopeVariant::Default { body, .. } => {
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

// ── Owned projection (consumer-requested; alloc-gated) ────────────────
// `CodecTransportEnvelope<'a>` above is a zero-copy view borrowing the decode
// buffer. AP / async consumers that persist a decoded message beyond the
// buffer's lifetime call `.into_owned()` for this lifetime-free
// `CodecTransportEnvelopeOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split — both generated from the one SCXML source (SSOT). `Vec`
// / `String` are alloc, so the whole projection is gated; the no-alloc
// borrowed path above is untouched.
#[cfg(feature = "alloc")]
use super::codec_zenoh_init_body::CodecZenohInitBodyOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_open_body::CodecZenohOpenBodyOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_frame::CodecZenohFrameOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_fragment::CodecZenohFragmentOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_join::CodecZenohJoinOwned;
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecTransportEnvelopeOwned {
    pub header: u8,
    pub body: CodecTransportEnvelopeOwnedVariant,
}

#[cfg(feature = "alloc")]
#[allow(dead_code)]
impl CodecTransportEnvelopeOwned {
    // RFC §5.B B1-γ + B5-α read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so alloc consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn a(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn s(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecTransportEnvelopeOwnedVariant {
    CodecZenohInitBody(CodecZenohInitBodyOwned),
    CodecZenohOpenBody(CodecZenohOpenBodyOwned),
    CodecZenohClose(CodecZenohClose),
    CodecZenohKeepAlive(CodecZenohKeepAlive),
    CodecZenohFrame(CodecZenohFrameOwned),
    CodecZenohFragment(CodecZenohFragmentOwned),
    CodecZenohJoin(CodecZenohJoinOwned),
    Default {
        tag: u8,
        body: CodecZenohClose,
    },
}

#[cfg(feature = "alloc")]
impl<'a> CodecTransportEnvelopeVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror.
    pub fn into_owned(self) -> CodecTransportEnvelopeOwnedVariant {
        match self {
            CodecTransportEnvelopeVariant::CodecZenohInitBody(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohInitBody(_b.into_owned()),
            CodecTransportEnvelopeVariant::CodecZenohOpenBody(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohOpenBody(_b.into_owned()),
            CodecTransportEnvelopeVariant::CodecZenohClose(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohClose(_b),
            CodecTransportEnvelopeVariant::CodecZenohKeepAlive(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohKeepAlive(_b),
            CodecTransportEnvelopeVariant::CodecZenohFrame(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohFrame(_b.into_owned()),
            CodecTransportEnvelopeVariant::CodecZenohFragment(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohFragment(_b.into_owned()),
            CodecTransportEnvelopeVariant::CodecZenohJoin(_b) => CodecTransportEnvelopeOwnedVariant::CodecZenohJoin(_b.into_owned()),
            CodecTransportEnvelopeVariant::Default { tag, body } => CodecTransportEnvelopeOwnedVariant::Default { tag, body },
        }
    }
}

#[cfg(feature = "alloc")]
impl CodecTransportEnvelopeOwnedVariant {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of `into_owned`. Reuses the borrowed view's single
    /// `encode`; the owned form deliberately carries no encode of its own.
    pub fn as_borrowed(&self) -> CodecTransportEnvelopeVariant<'_> {
        match self {
            CodecTransportEnvelopeOwnedVariant::CodecZenohInitBody(_b) => CodecTransportEnvelopeVariant::CodecZenohInitBody(_b.as_borrowed()),
            CodecTransportEnvelopeOwnedVariant::CodecZenohOpenBody(_b) => CodecTransportEnvelopeVariant::CodecZenohOpenBody(_b.as_borrowed()),
            CodecTransportEnvelopeOwnedVariant::CodecZenohClose(_b) => CodecTransportEnvelopeVariant::CodecZenohClose(_b.clone()),
            CodecTransportEnvelopeOwnedVariant::CodecZenohKeepAlive(_b) => CodecTransportEnvelopeVariant::CodecZenohKeepAlive(_b.clone()),
            CodecTransportEnvelopeOwnedVariant::CodecZenohFrame(_b) => CodecTransportEnvelopeVariant::CodecZenohFrame(_b.as_borrowed()),
            CodecTransportEnvelopeOwnedVariant::CodecZenohFragment(_b) => CodecTransportEnvelopeVariant::CodecZenohFragment(_b.as_borrowed()),
            CodecTransportEnvelopeOwnedVariant::CodecZenohJoin(_b) => CodecTransportEnvelopeVariant::CodecZenohJoin(_b.as_borrowed()),
            CodecTransportEnvelopeOwnedVariant::Default { tag, body } => CodecTransportEnvelopeVariant::Default { tag: *tag, body: body.clone() },
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> CodecTransportEnvelope<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecTransportEnvelopeOwned`] (alloc). Call at a decode boundary when
    /// the decoded value must outlive the input buffer — e.g. stored in a
    /// long-lived enum or moved across an async task. The no-alloc
    /// borrowed path is unaffected; this method exists only under
    /// `feature = "alloc"`.
    pub fn into_owned(self) -> CodecTransportEnvelopeOwned {
        CodecTransportEnvelopeOwned {
            header: self.header,
            body: self.body.into_owned(),
        }
    }
}

#[cfg(feature = "alloc")]
impl CodecTransportEnvelopeOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecTransportEnvelope<'_> {
        CodecTransportEnvelope {
            header: self.header,
            body: self.body.as_borrowed(),
        }
    }
}
