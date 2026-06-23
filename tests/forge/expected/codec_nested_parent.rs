#![doc = "SCE-MAP: codec_nested_parent:22"]
// SCE-MAP: codec_nested_parent:22

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
// RFC §synth-5-B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_nested_body::CodecNestedBody;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecNestedParent<'a> {
    pub hdr: u8,
    pub m: u8,
    pub required_body: CodecNestedBody<'a>,
    pub optional_body: Option<CodecNestedBody<'a>>,
    pub body_list: HeaplessVec<CodecNestedBody<'a>, 4>,
}

#[allow(dead_code)]
impl<'a> CodecNestedParent<'a> {
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
        // Streaming cursor decode (SSOT selection: `needs_streaming`).
        // The positional `raw[byte_off]` path is valid only when every
        // field's absolute offset is fixed at codegen time; this branch
        // handles every codec where it is not — present-if-gated fields
        // (runtime presence), VLE / repeat / TLV-chain / embed fields
        // (runtime width), string fields (UTF-8 decode), and a fixed field
        // after a variable-length payload (offset depends on the payload
        // length). Each field reads its own bytes from the cursor and
        // advances past exactly what it consumed. Per-field `is_repeat` /
        // `is_tlv_chain` / `is_embed` route to their dedicated helpers;
        // every other field flows through `present_if_decode_stmt`, whose
        // non-gated arm covers plain fixed / tail / length-ref / VLE reads.
        let hdr = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let m = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let required_body = CodecNestedBody::decode(cursor)?;
        let optional_body = if (hdr & 0x01u8) != 0 {
            Some(CodecNestedBody::decode(cursor)?)
        } else {
            None
        };
        let body_list = {
            let mut _vec: HeaplessVec<CodecNestedBody<'a>, 4> = HeaplessVec::new();
            for _ in 0..m {
                _vec.push(CodecNestedBody::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
            _vec
        };
        Ok(Self {
            hdr,
            m,
            required_body,
            optional_body,
            body_list,
        })
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn has_opt(&self) -> bool {
        (self.hdr & 0x01) != 0
    }

    pub fn set_has_opt(&mut self, v: bool) {
        if v {
            self.hdr |= 0x01;
        } else {
            self.hdr &= !0x01;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 2710;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when absent, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`
        // (its non-gated arm covers plain fixed / tail / length-ref / VLE).
        w.write_u8(self.hdr)?;
        w.write_u8(self.m)?;
        self.required_body.encode(w)?;
        if let Some(_v) = &self.optional_body {
            _v.encode(w)?;
        }
        for _e in &self.body_list {
            _e.encode(w)?;
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
// `CodecNestedParent<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecNestedParentOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
// `String` / `Bytes` fields project to the portable runtime newtypes
// `SceString<N>` / `SceBytes<N>`: an unbounded `String` / `Vec<u8>` under
// `alloc` (the on-wire protocol caps no payload, so the AP profile must
// not either — `N` is advisory) and the heap-free `heapless::String<N>` /
// `heapless::Vec<u8, N>` (the C11 `char[N]` analog) without it, where `N`
// is the hard capacity. A leaf codec's owned form therefore still compiles
// on a no-alloc MCU; only an unbounded owned `Vec` (list / embed / variant
// body) keeps the `alloc` gate. `try_into_owned` stays the fallible
// direction (one `?` per profile: the `alloc` copy cannot fail, the
// no-alloc copy enforces `N`); `try_as_borrowed` re-borrows either
// form infallibly via `.as_slice()` / `.as_str()`.
#[cfg(feature = "alloc")]
use super::codec_nested_body::CodecNestedBodyOwned;
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecNestedParentOwned {
    pub hdr: u8,
    pub m: u8,
    pub required_body: CodecNestedBodyOwned,
    pub optional_body: Option<CodecNestedBodyOwned>,
    pub body_list: Vec<CodecNestedBodyOwned>,
}

#[cfg(feature = "alloc")]
#[allow(dead_code)]
impl CodecNestedParentOwned {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so alloc consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn has_opt(&self) -> bool {
        (self.hdr & 0x01) != 0
    }
}

#[cfg(feature = "alloc")]
impl<'a> CodecNestedParent<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecNestedParentOwned`]. Call at a decode boundary when the
    /// decoded value must outlive the input buffer — stored in a long-lived
    /// enum, moved across an async task, or inserted by value into a
    /// bounded-collection. `String` / `Bytes` fields copy into the portable
    /// `SceString<N>` / `SceBytes<N>`: an unbounded heap copy under `alloc`
    /// (`N` advisory), else a fixed `heapless` copy capped at `N`. The
    /// method is fallible for profile uniformity — without `alloc` an
    /// over-`N` view raises `CodecError::TooManyElements` (the same bound
    /// and error decode enforces); under `alloc` the copy never fails. The
    /// borrowed zero-copy path is unaffected.
    pub fn try_into_owned(self) -> Result<CodecNestedParentOwned, CodecError> {
        Ok(CodecNestedParentOwned {
            hdr: self.hdr,
            m: self.m,
            required_body: self.required_body.try_into_owned()?,
            optional_body: self.optional_body.map(|_v| _v.try_into_owned()).transpose()?,
            body_list: self.body_list.into_iter().map(|_e| _e.try_into_owned()).collect::<Result<_, _>>()?,
        })
    }
}

#[cfg(feature = "alloc")]
impl CodecNestedParentOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `try_as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list whose
    /// owned `Vec` holds more than its declared `N` raises
    /// `CodecError::TooManyElements` — the same bound decode enforces.
    pub fn try_as_borrowed(&self) -> Result<CodecNestedParent<'_>, CodecError> {
        Ok(CodecNestedParent {
            hdr: self.hdr,
            m: self.m,
            required_body: self.required_body.try_as_borrowed()?,
            optional_body: self.optional_body.as_ref().map(|_v| _v.try_as_borrowed()).transpose()?,
            body_list: sce_forge_runtime::codec::try_project_bounded(&self.body_list, |_e| _e.try_as_borrowed())?,
        })
    }
}
