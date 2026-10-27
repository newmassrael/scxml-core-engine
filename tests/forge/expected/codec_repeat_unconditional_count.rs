#![doc = "SCE-MAP: codec_repeat_unconditional_count:34 :: _forge_body"]
// SCE-MAP: codec_repeat_unconditional_count:34 :: _forge_body

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
// RFC §synth-5-B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_repeat_elem::CodecRepeatElem;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecRepeatUnconditionalCount {
    pub options: u8,
    pub links_len: u8,
    pub links: HeaplessVec<CodecRepeatElem, 64>,
    pub weights: Option<HeaplessVec<CodecRepeatElem, 64>>,
}

#[allow(dead_code)]
impl CodecRepeatUnconditionalCount {
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
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out)?;
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
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
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
        let options = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let links_len = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        {
            let _vec = &mut _out.links;
            _vec.clear();
            for _ in 0..links_len {
                _vec.push(CodecRepeatElem::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
        }
        if (options & 0x01u8) != 0 {
            let _n = links_len;
            let _vec = _out.weights.insert(HeaplessVec::<CodecRepeatElem, 64>::new());
            for _ in 0.._n {
                _vec.push(CodecRepeatElem::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
        } else {
            _out.weights = None;
        }
        _out.options = options;
        _out.links_len = links_len;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn h(&self) -> bool {
        (self.options & 0x01) != 0
    }

    pub fn set_h(&mut self, v: bool) {
        if v {
            self.options |= 0x01;
        } else {
            self.options &= !0x01;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 258;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        Self::encode_source(self, w)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector, or the error
    /// `encode` refuses a value with: one the encoding cannot carry (a
    /// length past its type, a list past its bound) or one a gate
    /// disagrees with. It is a `Result` for every codec, so that a codec
    /// gaining a derived element, or a build that gates one in, does not
    /// change what a caller writes. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecRepeatUnconditionalCount` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecRepeatUnconditionalCount`
/// computes those from what is ([`CodecRepeatUnconditionalCount::encode_source`]).
pub trait CodecRepeatUnconditionalCountSource {
    /// The `options` field.
    fn options(&self) -> u8;
    /// The `links` field.
    fn links(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone;
    /// The `weights` field.
    fn weights(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone>;
}

impl<T: CodecRepeatUnconditionalCountSource + ?Sized> CodecRepeatUnconditionalCountSource for &T {
    fn options(&self) -> u8 {
        (**self).options()
    }
    fn links(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone {
        (**self).links()
    }
    fn weights(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone> {
        (**self).weights()
    }
}

/// A value of `CodecRepeatUnconditionalCount` held in the types a caller already has: a source
/// ([`CodecRepeatUnconditionalCountSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecRepeatUnconditionalCountParts<'a> {
    /// The `options` field.
    pub options: u8,
    /// The `links` field.
    pub links: &'a [super::codec_repeat_elem::CodecRepeatElemParts],
    /// The `weights` field.
    pub weights: Option<&'a [super::codec_repeat_elem::CodecRepeatElemParts]>,
}

impl<'a> CodecRepeatUnconditionalCountSource for CodecRepeatUnconditionalCountParts<'a> {
    fn options(&self) -> u8 {
        self.options
    }
    fn links(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone {
        self.links.iter()
    }
    fn weights(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone> {
        self.weights.map(<[_]>::iter)
    }
}

impl CodecRepeatUnconditionalCountSource for CodecRepeatUnconditionalCount {
    fn options(&self) -> u8 {
        self.options
    }
    fn links(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone {
        self.links.iter()
    }
    fn weights(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone> {
        self.weights.as_ref().map(|_l| _l.iter())
    }
}

#[allow(dead_code)]
impl CodecRepeatUnconditionalCount {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecRepeatUnconditionalCountSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecRepeatUnconditionalCountSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatUnconditionalCountSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        let _k_links: usize = src.links().count();
        if _k_links > 64 {
            return Err(CodecError::TooManyElements);
        }
        let _k_weights: usize = src.weights().map_or(0, |_l| _l.count());
        if _k_weights > 64 {
            return Err(CodecError::TooManyElements);
        }
        let _p_options_h_0: bool = src.weights().is_some();
        if src.weights().is_some() && _k_weights != _k_links {
            return Err(CodecError::InconsistentSource);
        }
        let _d_links_len: u8 = u8::try_from(_k_links).map_err(|_| CodecError::LengthOutOfRange)?;
        let _c_options: u8 = (src.options() & !0x1) | u8::from(_p_options_h_0);
        w.write_u8(_c_options)?;
        w.write_u8(_d_links_len)?;
        for _e in src.links() {
            CodecRepeatElem::write_source(&_e, w)?;
        }
        if let Some(_v) = src.weights() {
            for _e in _v {
                CodecRepeatElem::write_source(&_e, w)?;
            }
        }
        Ok(())
    }
}

impl<S: CodecRepeatUnconditionalCountSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecRepeatUnconditionalCount {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecRepeatUnconditionalCount {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecRepeatUnconditionalCountSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }
}
