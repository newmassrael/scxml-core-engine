#![doc = "SCE-MAP: codec_zenoh_response:75"]
// SCE-MAP: codec_zenoh_response:75

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
// RFC §5.B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_zenoh_ext_entry::CodecZenohExtEntry;
use super::codec_zenoh_reply::CodecZenohReply;
use super::codec_zenoh_err::CodecZenohErr;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohResponseVariant<'a> {
    CodecZenohReply(CodecZenohReply<'a>),
    CodecZenohErr(CodecZenohErr<'a>),
    Default {
        tag: u8,
        body: CodecZenohReply<'a>,
    },
}

impl<'a> Default for CodecZenohResponseVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohReply(CodecZenohReply::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohResponse<'a> {
    pub header: u8,
    pub request_id: u64,
    pub key_id: u32,
    pub suffix_len: Option<u64>,
    pub suffix: Option<&'a str>,
    pub extensions: Option<HeaplessVec<CodecZenohExtEntry<'a>, 4>>,
    pub body: CodecZenohResponseVariant<'a>,
}

// RFC variant-default-uniformity Atomic β: at least one field's
// `<sce:flags>` carrier declares a wire-MID constant via
// `<sce:flag value="N"/>`. Manual `impl Default` bakes the OR of
// every declared `(value & mask) << bit` into that carrier so a
// freshly-constructed instance carries the wire-MID for its own
// dispatch tag. Fields without declared values fall through to
// `Default::default()` (preserving derive(Default) semantics).
impl<'a> Default for CodecZenohResponse<'a> {
    fn default() -> Self {
        Self {
            header: 0x1bu8,
            request_id: Default::default(),
            key_id: Default::default(),
            suffix_len: Default::default(),
            suffix: Default::default(),
            extensions: Default::default(),
            body: CodecZenohResponseVariant::default(),
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohResponse<'a> {
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
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for the variant
        // tag without advancing — the arm body decoder reads the peeked
        // byte as its own header byte (Zenoh response/request body MID
        // dispatch shape per network.c:347-364 + 220-235).
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let request_id = cursor.read_vle_u64()?;
        let key_id = cursor.read_vle_u32()?;
        let suffix_len = if (header & 0x20u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let suffix = if (header & 0x20u8) != 0 {
            let _n = suffix_len.unwrap() as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = core::str::from_utf8(raw)
                .map_err(|_| CodecError::InvalidUtf8)?;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        let extensions = if (header & 0x80u8) != 0 {
            let mut _vec: HeaplessVec<CodecZenohExtEntry<'a>, 4> = HeaplessVec::new();
            for _ in 0..4u32 {
                    if cursor.remaining() == 0 { break; }
                    let _entry = CodecZenohExtEntry::decode(cursor)?;
                    let _continue = _entry.z();
                    _vec.push(_entry).map_err(|_| CodecError::TooManyElements)?;
                    if !_continue { break; }
                }
            Some(_vec)
        } else {
            None
        };
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match _peek & 0x1F {
            4u8 => CodecZenohResponseVariant::CodecZenohReply(CodecZenohReply::decode(cursor)?),
            5u8 => CodecZenohResponseVariant::CodecZenohErr(CodecZenohErr::decode(cursor)?),
            other => CodecZenohResponseVariant::Default {
                tag: other,
                body: CodecZenohReply::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            request_id,
            key_id,
            suffix_len,
            suffix,
            extensions,
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
    pub const MAX_ENCODED_BYTES: usize = 977;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode (per-field present_if/tlv-chain/embed/
        // repeat helpers). Peek-byte mode: the arm body's encode prepends
        // its own header byte (which the decoder peeked); no separate
        // tag byte is emitted here. Streaming-prefix mode (own-field
        // variant): the carrier is part of the prefix fields and emits
        // through the same per-field path.
        w.write_u8(self.header)?;
        {
            let mut _vle = self.request_id;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
        }
        {
            let mut _vle = self.key_id as u64;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
        }
        if let Some(_v) = self.suffix_len {
        {
            let mut _vle = _v;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
        }
        }
        if let Some(_v) = &self.suffix {
            w.write_bytes(_v.as_bytes())?;
        }
        if let Some(_list) = &self.extensions {
            for _e in _list {
                _e.encode(w)?;
            }
        }
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohResponseVariant::CodecZenohReply(b) => {
                b.encode(w)?;
            }
            CodecZenohResponseVariant::CodecZenohErr(b) => {
                b.encode(w)?;
            }
            CodecZenohResponseVariant::Default { body, .. } => {
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

// ── Owned projection (no-alloc bounded-inline native form) ────────────
// `CodecZenohResponse<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohResponseOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT). Bounded
// `String` / `Bytes` fields project to `heapless::String<N>` /
// `heapless::Vec<u8, N>` (the Rust mirror of C11's `char[N]`), so a leaf
// codec's owned form is fully no-alloc; only an unbounded owned `Vec`
// (list / embed / variant body) keeps the `alloc` gate. Construction from
// the unbounded `&str` / `&[u8]` view re-checks the decode bound, so
// `try_into_owned` is the fallible direction and `try_as_borrowed`
// (same `N`) the infallible inverse.
#[cfg(feature = "alloc")]
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_reply::CodecZenohReplyOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_err::CodecZenohErrOwned;
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohResponseOwned {
    pub header: u8,
    pub request_id: u64,
    pub key_id: u32,
    pub suffix_len: Option<u64>,
    pub suffix: Option<::sce_forge_runtime::heapless::String<256>>,
    pub extensions: Option<Vec<CodecZenohExtEntryOwned>>,
    pub body: CodecZenohResponseOwnedVariant,
}

#[cfg(feature = "alloc")]
#[allow(dead_code)]
impl CodecZenohResponseOwned {
    // RFC §5.B B1-γ + B5-α read-accessor parity with the borrowed view: pure
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
// Bounded `String` / `Bytes` fields store inline (`heapless::String<N>` /
// `heapless::Vec<u8, N>`) in the owned mirror — the no-alloc native form,
// the Rust analog of C11's `char[N]` tagged union. That makes the arm
// bodies inherently size-disparate; the lint's only remedy is boxing the
// large arm, which reintroduces the `alloc` dependency this inline form
// exists to avoid. The size is the deliberate no-alloc trade-off.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohResponseOwnedVariant {
    CodecZenohReply(CodecZenohReplyOwned),
    CodecZenohErr(CodecZenohErrOwned),
    Default {
        tag: u8,
        body: CodecZenohReplyOwned,
    },
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohResponseVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror. Fallible
    /// because a borrowed arm body's `try_into_owned` re-checks its bounded
    /// fields against their inline capacity (the same bound decode enforces).
    pub fn try_into_owned(self) -> Result<CodecZenohResponseOwnedVariant, CodecError> {
        Ok(match self {
            CodecZenohResponseVariant::CodecZenohReply(_b) => CodecZenohResponseOwnedVariant::CodecZenohReply(_b.try_into_owned()?),
            CodecZenohResponseVariant::CodecZenohErr(_b) => CodecZenohResponseOwnedVariant::CodecZenohErr(_b.try_into_owned()?),
            CodecZenohResponseVariant::Default { tag, body } => CodecZenohResponseOwnedVariant::Default { tag, body: body.try_into_owned()? },
        })
    }
}

#[cfg(feature = "alloc")]
impl CodecZenohResponseOwnedVariant {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of `into_owned`. Reuses the borrowed view's single
    /// `encode`; the owned form deliberately carries no encode of its own.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohResponseVariant<'_>, CodecError> {
        Ok(match self {
            CodecZenohResponseOwnedVariant::CodecZenohReply(_b) => CodecZenohResponseVariant::CodecZenohReply(_b.try_as_borrowed()?),
            CodecZenohResponseOwnedVariant::CodecZenohErr(_b) => CodecZenohResponseVariant::CodecZenohErr(_b.try_as_borrowed()?),
            CodecZenohResponseOwnedVariant::Default { tag, body } => CodecZenohResponseVariant::Default { tag: *tag, body: body.try_as_borrowed()? },
        })
    }
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohResponse<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohResponseOwned`]. Call at a decode boundary when the
    /// decoded value must outlive the input buffer — stored in a long-lived
    /// enum, moved across an async task, or inserted by value into a
    /// bounded-collection. Bounded `String` / `Bytes` fields copy into
    /// `heapless::String<N>` / `heapless::Vec<u8, N>`; that copy re-checks
    /// the decode bound, so the method is fallible
    /// (`CodecError::TooManyElements` past `N` — the bound and error decode
    /// enforces). The borrowed zero-copy path is unaffected.
    pub fn try_into_owned(self) -> Result<CodecZenohResponseOwned, CodecError> {
        Ok(CodecZenohResponseOwned {
            header: self.header,
            request_id: self.request_id,
            key_id: self.key_id,
            suffix_len: self.suffix_len,
            suffix: self.suffix.map(|_v| ::sce_forge_runtime::heapless::String::try_from(_v).map_err(|_| CodecError::TooManyElements)).transpose()?,
            extensions: self.extensions.map(|_v| _v.into_iter().map(|_e| _e.try_into_owned()).collect::<Result<_, _>>()).transpose()?,
            body: self.body.try_into_owned()?,
        })
    }
}

#[cfg(feature = "alloc")]
impl CodecZenohResponseOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `try_as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list whose
    /// owned `Vec` holds more than its declared `N` raises
    /// `CodecError::TooManyElements` — the same bound decode enforces.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohResponse<'_>, CodecError> {
        Ok(CodecZenohResponse {
            header: self.header,
            request_id: self.request_id,
            key_id: self.key_id,
            suffix_len: self.suffix_len,
            suffix: self.suffix.as_deref(),
            extensions: self.extensions.as_ref().map(|_l| sce_forge_runtime::codec::try_project_bounded(_l, |_e| Ok(_e.as_borrowed()))).transpose()?,
            body: self.body.try_as_borrowed()?,
        })
    }
}
