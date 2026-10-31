#![doc = "SCE-MAP: codec_init_syn_body:30 :: _forge_body"]
// SCE-MAP: codec_init_syn_body:30 :: _forge_body

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

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecInitSynBody {
    pub version: u8,
    pub sn_res: Option<u8>,
    pub batch_size: Option<u16>,
}

#[allow(dead_code)]
impl CodecInitSynBody {
    /// Construct an instance with every field at its own type's
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call. An
    /// enum-typed field starts at the first variant its document
    /// declares, not at the carrier's zero: a closed set does not hold
    /// a value it never declared.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519).
    pub fn decode(cursor: &mut SceCursor<'_>, s: u8) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, s)?;
        Ok(_out)
    }

    /// Decode the next frame from `cursor` into `_out`: the bytes, the bounds
    /// and the errors of [`Self::decode`], with each bounded list and embedded
    /// body built in the field where it will live.
    ///
    /// `decode` returns its value, and a `heapless::Vec` moves by copying its
    /// whole declared capacity, so every hop a decoded list takes on the way
    /// up (the struct literal, `Ok(..)`, a variant wrapper, the caller's own
    /// binding) copies the declared maximum however few entries the wire held.
    /// A caller on a path where that shows decodes into a destination it owns.
    /// On `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self, s: u8) -> Result<(), CodecError> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = s;
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
        let version = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
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
            let _v = ((raw[0] as u16) << 8) | raw[1] as u16;
            cursor.advance(2)?;
            Some(_v)
        } else {
            None
        };
        _out.version = version;
        _out.sn_res = sn_res;
        _out.batch_size = batch_size;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 4;

    /// Encode `self` into the caller-owned sink. Returns the value's refusal
    /// when the encoding cannot carry it (a length past its type, a list past
    /// its bound, a gate that disagrees with a presence; the document declares
    /// `sce:encode-may-fail`), or the sink's own error:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity.
    pub fn encode<S: SceSink>(&self, w: &mut S, s: u8) -> Result<(), CodecError> where CodecError: From<S::Error> {
        Self::encode_source(self, w, s)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector, or the error
    /// `encode` refuses a value with: one the encoding cannot carry (a
    /// length past its type, a list past its bound) or one a gate
    /// disagrees with. Callers targeting zero-alloc hot paths should call
    /// `encode` directly against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self, s: u8) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink, s)?;
        Ok(_sce_v)
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecInitSynBody` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecInitSynBody`
/// computes those from what is ([`CodecInitSynBody::encode_source`]).
pub trait CodecInitSynBodySource {
    /// The `version` field.
    fn version(&self) -> u8;
    /// The `sn_res` field.
    fn sn_res(&self) -> Option<u8>;
    /// The `batch_size` field.
    fn batch_size(&self) -> Option<u16>;
}

impl<T: CodecInitSynBodySource + ?Sized> CodecInitSynBodySource for &T {
    fn version(&self) -> u8 {
        (**self).version()
    }
    fn sn_res(&self) -> Option<u8> {
        (**self).sn_res()
    }
    fn batch_size(&self) -> Option<u16> {
        (**self).batch_size()
    }
}

/// A value of `CodecInitSynBody` held in the types a caller already has: a source
/// ([`CodecInitSynBodySource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecInitSynBodyParts {
    /// The `version` field.
    pub version: u8,
    /// The `sn_res` field.
    pub sn_res: Option<u8>,
    /// The `batch_size` field.
    pub batch_size: Option<u16>,
}

impl CodecInitSynBodySource for CodecInitSynBodyParts {
    fn version(&self) -> u8 {
        self.version
    }
    fn sn_res(&self) -> Option<u8> {
        self.sn_res
    }
    fn batch_size(&self) -> Option<u16> {
        self.batch_size
    }
}

impl CodecInitSynBodySource for CodecInitSynBody {
    fn version(&self) -> u8 {
        self.version
    }
    fn sn_res(&self) -> Option<u8> {
        self.sn_res
    }
    fn batch_size(&self) -> Option<u16> {
        self.batch_size
    }
}

#[allow(dead_code)]
impl CodecInitSynBody {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecInitSynBodySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecInitSynBodySource + ?Sized, K: SceSink>(src: &S, w: &mut K, s: u8) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w, s)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecInitSynBodySource + ?Sized, K: SceSink>(src: &S, w: &mut K, s: u8) -> Result<(), CodecError> where CodecError: From<K::Error> {
        if ((s & 0x1) != 0) != src.sn_res().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        if ((s & 0x1) != 0) != src.batch_size().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        let _ = s;
        w.write_u8(src.version())?;
        if let Some(_v) = src.sn_res() {
            w.write_u8(_v)?;
        }
        if let Some(_v) = src.batch_size() {
            w.write_u8((_v >> 8 & 0xFF) as u8)?;
            w.write_u8((_v & 0xFF) as u8)?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecInitSynBodySource + ?Sized>(src: &S, s: u8) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c, s)?;
        Ok(_c.len())
    }
}
