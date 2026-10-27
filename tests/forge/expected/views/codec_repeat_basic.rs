#![doc = "SCE-MAP: codec_repeat_basic:11 :: _forge_body"]
// SCE-MAP: codec_repeat_basic:11 :: _forge_body

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

use sce_forge_runtime::codec::{ListView, ListWalk};
use super::codec_repeat_elem::CodecRepeatElemView;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecRepeatBasic {
    pub num_frags: u8,
    pub frags: HeaplessVec<CodecRepeatElem, 32>,
}

#[allow(dead_code)]
impl CodecRepeatBasic {
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
        let num_frags = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        {
            let _vec = &mut _out.frags;
            _vec.clear();
            for _ in 0..num_frags {
                _vec.push(CodecRepeatElem::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
        }
        _out.num_frags = num_frags;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 65;

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
/// What a value of `CodecRepeatBasic` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecRepeatBasic`
/// computes those from what is ([`CodecRepeatBasic::encode_source`]).
pub trait CodecRepeatBasicSource {
    /// The `frags` field.
    fn frags(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone;
}

impl<T: CodecRepeatBasicSource + ?Sized> CodecRepeatBasicSource for &T {
    fn frags(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone {
        (**self).frags()
    }
}

/// A value of `CodecRepeatBasic` held in the types a caller already has: a source
/// ([`CodecRepeatBasicSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecRepeatBasicParts<'a> {
    /// The `frags` field.
    pub frags: &'a [super::codec_repeat_elem::CodecRepeatElemParts],
}

impl<'a> CodecRepeatBasicSource for CodecRepeatBasicParts<'a> {
    fn frags(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone {
        self.frags.iter()
    }
}

impl CodecRepeatBasicSource for CodecRepeatBasic {
    fn frags(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone {
        self.frags.iter()
    }
}

impl<'a> CodecRepeatBasicSource for CodecRepeatBasicView<'a> {
    fn frags(&self) -> impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone {
        self.frags.iter()
    }
}

#[allow(dead_code)]
impl CodecRepeatBasic {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecRepeatBasicSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecRepeatBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        let _k_frags: usize = src.frags().count();
        if _k_frags > 32 {
            return Err(CodecError::TooManyElements);
        }
        let _d_num_frags: u8 = u8::try_from(_k_frags).map_err(|_| CodecError::LengthOutOfRange)?;
        w.write_u8(_d_num_frags)?;
        for _e in src.frags() {
            CodecRepeatElem::write_source(&_e, w)?;
        }
        Ok(())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecRepeatBasicView` is what a decode of `CodecRepeatBasic` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecRepeatBasic` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecRepeatBasic::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
pub struct CodecRepeatBasicFragsWalk;

impl ListWalk<0> for CodecRepeatBasicFragsWalk {
    type Entry<'b> = CodecRepeatElemView;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 0]) -> Result<Self::Entry<'b>, CodecError> {
        CodecRepeatElemView::decode(_c)
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 0]) {  }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecRepeatBasicView<'a> {
    pub num_frags: u8,
    pub frags: ListView<'a, CodecRepeatBasicFragsWalk, 0>,
}

#[allow(dead_code)]
impl<'a> CodecRepeatBasicView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecRepeatBasic::decode`], held as a view. On success the
    /// cursor advances past the consumed bytes.
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out)?;
        Ok(_out)
    }

    /// [`Self::decode`] into a destination the caller owns, so a view that
    /// embeds or dispatches to this one is filled where it will live. On
    /// `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self) -> Result<(), CodecError> {
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
        let num_frags = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        {
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            for _ in 0..num_frags {
                CodecRepeatElemView::decode(cursor)?;
                if _count == 32 {
                    return Err(CodecError::TooManyElements);
                }
                _count += 1;
            }
            _out.frags = ListView::new(_rest, _count, []);
        }
        _out.num_frags = num_frags;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecRepeatBasic`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    pub fn into_builder(&self, _out: &mut CodecRepeatBasic) -> Result<(), CodecError> {
        _out.num_frags = self.num_frags;
        _out.frags.clear();
        for _e in self.frags.iter() {
            _out.frags.push(CodecRepeatElem::default()).map_err(|_| CodecError::TooManyElements)?;
            if let Some(_slot) = _out.frags.last_mut() {
                _e.into_builder(_slot)?;
            }
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecRepeatBasic::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 65;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecRepeatBasic::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        CodecRepeatBasic::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses a
    /// value it cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
