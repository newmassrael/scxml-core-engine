#![doc = "SCE-MAP: codec_zenoh_wireexpr:53"]
// SCE-MAP: codec_zenoh_wireexpr:53

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

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohWireexpr<'a> {
    pub id: u64,
    pub suffix_len: Option<u64>,
    pub suffix: Option<&'a str>,
}

#[allow(dead_code)]
impl<'a> CodecZenohWireexpr<'a> {
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
    pub fn decode(cursor: &mut SceCursor<'a>, n: u8) -> Result<Self, CodecError> {
        // RFC Axis-1 inversion: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = n;
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
        let id = cursor.read_vle_u64()?;
        let suffix_len = if (n & 0x01u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let suffix = if (n & 0x01u8) != 0 {
            let _n = suffix_len.unwrap() as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = core::str::from_utf8(raw)
                .map_err(|_| CodecError::InvalidUtf8)?;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        Ok(Self {
            id,
            suffix_len,
            suffix,
        })
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 148;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S, n: u8) -> Result<(), CodecError> {
        // RFC Axis-1 inversion: see `decode` — same suppress per
        // declared `<sce:flag-input>`.
        let _ = n;
        // RFC §5.B B1-δ + B2-β present-if encode: every field appends
        // its bytes via a per-field block; gated fields skip the
        // append when the optional is None. Per-field `is_repeat` /
        // `is_tlv_chain` route Repeat / TLV chain fields to their
        // dedicated helpers. Author keeps the carrier's flag bit and
        // the optional's truth value in sync (trust contract, mirrors
        // the variant primitive). Note: this branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified encode path.
        {
            let mut _vle = self.id;
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
    pub fn encode_to_vec(&self, n: u8) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink, n)
            .expect("VecSink is infallible");
        _sce_v
    }
}

// ── Owned projection (no-alloc bounded-inline native form) ────────────
// `CodecZenohWireexpr<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohWireexprOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT). Bounded
// `String` / `Bytes` fields project to `heapless::String<N>` /
// `heapless::Vec<u8, N>` (the Rust mirror of C11's `char[N]`), so a leaf
// codec's owned form is fully no-alloc; only an unbounded owned `Vec`
// (list / embed / variant body) keeps the `alloc` gate. Construction from
// the unbounded `&str` / `&[u8]` view re-checks the decode bound, so
// `try_into_owned` is the fallible direction and `as_borrowed`
// (same `N`) the infallible inverse.
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohWireexprOwned {
    pub id: u64,
    pub suffix_len: Option<u64>,
    pub suffix: Option<::sce_forge_runtime::heapless::String<128>>,
}

impl<'a> CodecZenohWireexpr<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohWireexprOwned`]. Call at a decode boundary when the
    /// decoded value must outlive the input buffer — stored in a long-lived
    /// enum, moved across an async task, or inserted by value into a
    /// bounded-collection. Bounded `String` / `Bytes` fields copy into
    /// `heapless::String<N>` / `heapless::Vec<u8, N>`; that copy re-checks
    /// the decode bound, so the method is fallible
    /// (`CodecError::TooManyElements` past `N` — the bound and error decode
    /// enforces). The borrowed zero-copy path is unaffected.
    pub fn try_into_owned(self) -> Result<CodecZenohWireexprOwned, CodecError> {
        Ok(CodecZenohWireexprOwned {
            id: self.id,
            suffix_len: self.suffix_len,
            suffix: self.suffix.map(|_v| ::sce_forge_runtime::heapless::String::try_from(_v).map_err(|_| CodecError::TooManyElements)).transpose()?,
        })
    }
}

impl CodecZenohWireexprOwned {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned`. `encode` lives only on the borrowed
    /// view (the owned form is read-only), so an owned consumer reaches it
    /// via `as_borrowed` then `encode` / `encode_to_vec`. Each
    /// field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohWireexpr<'_> {
        CodecZenohWireexpr {
            id: self.id,
            suffix_len: self.suffix_len,
            suffix: self.suffix.as_deref(),
        }
    }
}
