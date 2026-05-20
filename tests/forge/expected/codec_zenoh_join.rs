#![doc = "SCE-MAP: codec_zenoh_join:41"]
// SCE-MAP: codec_zenoh_join:41

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
#[derive(Default)]
pub struct CodecZenohJoin {
    pub version: u8,
    pub cbyte: u8,
    pub zid: Vec<u8>,
    pub sn_res: Option<u8>,
    pub batch_size: Option<u16>,
    pub lease: u64,
    pub next_sn_reliable: u64,
    pub next_sn_best_effort: u64,
}

#[allow(dead_code)]
impl CodecZenohJoin {
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
    pub fn decode(cursor: &mut SceCursor<'_>, s: u8) -> Result<Self, CodecError> {
        // RFC Axis-1 inversion: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = s;
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
        let version = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let cbyte = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let zid = {
            let _n = (((cbyte >> 4) & 0xF) as usize).wrapping_add(1);
            let raw = cursor.peek_slice(_n)?;
            let _v = raw.to_vec();
            cursor.advance(_n)?;
            _v
        };
        let sn_res = if (s & 0x01u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let batch_size = if (s & 0x01u8) != 0 {
            let raw = cursor.peek_slice(2)?;
            let _v = raw[0] as u16 | ((raw[1] as u16) << 8);
            cursor.advance(2)?;
            Some(_v)
        } else {
            None
        };
        let lease = cursor.read_vle_u64()?;
        let next_sn_reliable = cursor.read_vle_u64()?;
        let next_sn_best_effort = cursor.read_vle_u64()?;
        Ok(Self {
            version,
            cbyte,
            zid,
            sn_res,
            batch_size,
            lease,
            next_sn_reliable,
            next_sn_best_effort,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn whatami(&self) -> u8 {
        (((self.cbyte >> 0) & (0x03 as u8))) as u8
    }

    pub fn set_whatami(&mut self, v: u8) {
        let _mask: u8 = (0x03 as u8) << 0;
        let _val: u8 = ((v as u8) & (0x03 as u8)) << 0;
        self.cbyte = (self.cbyte & !_mask) | _val;
    }

    pub fn zid_len_m1(&self) -> u8 {
        (((self.cbyte >> 4) & (0x0F as u8))) as u8
    }

    pub fn set_zid_len_m1(&mut self, v: u8) {
        let _mask: u8 = (0x0F as u8) << 4;
        let _val: u8 = ((v as u8) & (0x0F as u8)) << 4;
        self.cbyte = (self.cbyte & !_mask) | _val;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 52;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S, s: u8) -> Result<(), CodecError> {
        // RFC Axis-1 inversion: see `decode` — same suppress per
        // declared `<sce:flag-input>`.
        let _ = s;
        // RFC §5.B B1-δ + B2-β present-if encode: every field appends
        // its bytes via a per-field block; gated fields skip the
        // append when the optional is None. Per-field `is_repeat` /
        // `is_tlv_chain` route Repeat / TLV chain fields to their
        // dedicated helpers. Author keeps the carrier's flag bit and
        // the optional's truth value in sync (trust contract, mirrors
        // the variant primitive). Note: this branch fires before
        // has_vle_fields so a codec mixing VLE + present-if uses the
        // unified encode path.
        w.write_u8(self.version)?;
        w.write_u8(self.cbyte)?;
        w.write_bytes(&self.zid)?;
        if let Some(_v) = self.sn_res {
            w.write_u8(_v)?;
        }
        if let Some(_v) = self.batch_size {
            w.write_u8(_v as u8)?;
            w.write_u8((_v >> 8) as u8)?;
        }
        {
            let mut _vle = self.lease as u64;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
        }
        {
            let mut _vle = self.next_sn_reliable as u64;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
        }
        {
            let mut _vle = self.next_sn_best_effort as u64;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
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
    pub fn encode_to_vec(&self, s: u8) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink, s)
            .expect("VecSink is infallible");
        _sce_v
    }
}
