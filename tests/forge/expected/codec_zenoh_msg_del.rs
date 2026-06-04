#![doc = "SCE-MAP: codec_zenoh_msg_del:53"]
// SCE-MAP: codec_zenoh_msg_del:53

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

use super::codec_zenoh_timestamp::CodecZenohTimestamp;
use super::codec_zenoh_ext_entry::CodecZenohExtEntry;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohMsgDel<'a> {
    pub header: u8,
    pub timestamp: Option<CodecZenohTimestamp<'a>>,
    pub extensions: Option<HeaplessVec<CodecZenohExtEntry<'a>, 4>>,
}

// RFC variant-default-uniformity Atomic β: at least one field's
// `<sce:flags>` carrier declares a wire-MID constant via
// `<sce:flag value="N"/>`. Manual `impl Default` bakes the OR of
// every declared `(value & mask) << bit` into that carrier so a
// freshly-constructed instance carries the wire-MID for its own
// dispatch tag. Fields without declared values fall through to
// `Default::default()` (preserving derive(Default) semantics).
impl<'a> Default for CodecZenohMsgDel<'a> {
    fn default() -> Self {
        Self {
            header: 0x02u8,
            timestamp: Default::default(),
            extensions: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohMsgDel<'a> {
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
        // RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
        // advances the cursor per field. Gated fields wrap their
        // read inside an `if predicate { Some(...) } else { None }`
        // block computed at codegen time from the carrier field's
        // flag bit. B2-β extends gated fields to Tail / LengthRef /
        // Vle bit-sizes via dispatch inside `present_if_decode_stmt`.
        // Per-field `is_repeat` / `is_tlv_chain` route Repeat / TLV
        // chain fields to their dedicated helpers since present-if
        // isn't allowed on `<sce:repeat>` / `<sce:tlv-chain>`.
        // Note: this branch fires before has_vle_fields so a codec
        // mixing VLE + present-if uses the unified streaming path.
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let timestamp = if (header & 0x20u8) != 0 {
            Some(CodecZenohTimestamp::decode(cursor)?)
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
        Ok(Self {
            header,
            timestamp,
            extensions,
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

    pub fn t(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_t(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn x(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_x(&mut self, v: bool) {
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
    pub const MAX_ENCODED_BYTES: usize = 429;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // RFC §5.B B1-δ + B2-β present-if encode: every field appends
        // its bytes via a per-field block; gated fields skip the
        // append when the optional is None. Per-field `is_repeat` /
        // `is_tlv_chain` route Repeat / TLV chain fields to their
        // dedicated helpers. Author keeps the carrier's flag bit and
        // the optional's truth value in sync (trust contract, mirrors
        // the variant primitive). Note: this branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified encode path.
        w.write_u8(self.header)?;
        if let Some(_v) = &self.timestamp {
            _v.encode(w)?;
        }
        if let Some(_list) = &self.extensions {
            for _e in _list {
                _e.encode(w)?;
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
// `CodecZenohMsgDel<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohMsgDelOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT). Bounded
// `String` / `Bytes` fields project to `heapless::String<N>` /
// `heapless::Vec<u8, N>` (the Rust mirror of C11's `char[N]`), so a leaf
// codec's owned form is fully no-alloc; only an unbounded owned `Vec`
// (list / embed / variant body) keeps the `alloc` gate. Construction from
// the unbounded `&str` / `&[u8]` view re-checks the decode bound, so
// `try_into_owned` is the fallible direction and `try_as_borrowed`
// (same `N`) the infallible inverse.
#[cfg(feature = "alloc")]
use super::codec_zenoh_timestamp::CodecZenohTimestampOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohMsgDelOwned {
    pub header: u8,
    pub timestamp: Option<CodecZenohTimestampOwned>,
    pub extensions: Option<Vec<CodecZenohExtEntryOwned>>,
}

#[cfg(feature = "alloc")]
#[allow(dead_code)]
impl CodecZenohMsgDelOwned {
    // RFC §5.B B1-γ + B5-α read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so alloc consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn t(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn x(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohMsgDel<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohMsgDelOwned`]. Call at a decode boundary when the
    /// decoded value must outlive the input buffer — stored in a long-lived
    /// enum, moved across an async task, or inserted by value into a
    /// bounded-collection. Bounded `String` / `Bytes` fields copy into
    /// `heapless::String<N>` / `heapless::Vec<u8, N>`; that copy re-checks
    /// the decode bound, so the method is fallible
    /// (`CodecError::TooManyElements` past `N` — the bound and error decode
    /// enforces). The borrowed zero-copy path is unaffected.
    pub fn try_into_owned(self) -> Result<CodecZenohMsgDelOwned, CodecError> {
        Ok(CodecZenohMsgDelOwned {
            header: self.header,
            timestamp: self.timestamp.map(|_v| _v.try_into_owned()).transpose()?,
            extensions: self.extensions.map(|_v| _v.into_iter().map(|_e| _e.try_into_owned()).collect::<Result<_, _>>()).transpose()?,
        })
    }
}

#[cfg(feature = "alloc")]
impl CodecZenohMsgDelOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `try_as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list whose
    /// owned `Vec` holds more than its declared `N` raises
    /// `CodecError::TooManyElements` — the same bound decode enforces.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohMsgDel<'_>, CodecError> {
        Ok(CodecZenohMsgDel {
            header: self.header,
            timestamp: self.timestamp.as_ref().map(|_v| _v.as_borrowed()),
            extensions: self.extensions.as_ref().map(|_l| sce_forge_runtime::codec::try_project_bounded(_l, |_e| Ok(_e.as_borrowed()))).transpose()?,
        })
    }
}
