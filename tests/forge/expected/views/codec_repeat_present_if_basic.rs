#![doc = "SCE-MAP: codec_repeat_present_if_basic:37 :: _forge_body"]
// SCE-MAP: codec_repeat_present_if_basic:37 :: _forge_body

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
pub struct CodecRepeatPresentIfBasic {
    pub carrier: u8,
    pub num_elems: Option<u8>,
    pub elems: Option<HeaplessVec<CodecRepeatElem, 32>>,
}

#[allow(dead_code)]
impl CodecRepeatPresentIfBasic {
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
        let carrier = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let num_elems = if (carrier & 0x01u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        if (carrier & 0x01u8) != 0 {
            let _n = num_elems.expect("co-gating: count present-if matches repeat");
            let _vec = _out.elems.insert(HeaplessVec::<CodecRepeatElem, 32>::new());
            for _ in 0.._n {
                _vec.push(CodecRepeatElem::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
        } else {
            _out.elems = None;
        }
        _out.carrier = carrier;
        _out.num_elems = num_elems;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn has_list(&self) -> bool {
        (self.carrier & 0x01) != 0
    }

    pub fn set_has_list(&mut self, v: bool) {
        if v {
            self.carrier |= 0x01;
        } else {
            self.carrier &= !0x01;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 66;

    /// Encode `self` into the caller-owned sink. Returns the value's refusal
    /// when the encoding cannot carry it (a length past its type, a list past
    /// its bound, a gate that disagrees with a presence; the document declares
    /// `sce:encode-may-fail`), or the sink's own error:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> where CodecError: From<S::Error> {
        Self::encode_source(self, w)
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
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecRepeatPresentIfBasic` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecRepeatPresentIfBasic`
/// computes those from what is ([`CodecRepeatPresentIfBasic::encode_source`]).
pub trait CodecRepeatPresentIfBasicSource {
    /// The `carrier` field.
    fn carrier(&self) -> u8;
    /// The `elems` field.
    fn elems(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone>;
}

impl<T: CodecRepeatPresentIfBasicSource + ?Sized> CodecRepeatPresentIfBasicSource for &T {
    fn carrier(&self) -> u8 {
        (**self).carrier()
    }
    fn elems(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone> {
        (**self).elems()
    }
}

/// A value of `CodecRepeatPresentIfBasic` held in the types a caller already has: a source
/// ([`CodecRepeatPresentIfBasicSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecRepeatPresentIfBasicParts<'a> {
    /// The `carrier` field.
    pub carrier: u8,
    /// The `elems` field.
    pub elems: Option<&'a [super::codec_repeat_elem::CodecRepeatElemParts]>,
}

impl<'a> CodecRepeatPresentIfBasicSource for CodecRepeatPresentIfBasicParts<'a> {
    fn carrier(&self) -> u8 {
        self.carrier
    }
    fn elems(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone> {
        self.elems.map(<[_]>::iter)
    }
}

impl CodecRepeatPresentIfBasicSource for CodecRepeatPresentIfBasic {
    fn carrier(&self) -> u8 {
        self.carrier
    }
    fn elems(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone> {
        self.elems.as_ref().map(|_l| _l.iter())
    }
}

impl<'a> CodecRepeatPresentIfBasicSource for CodecRepeatPresentIfBasicView<'a> {
    fn carrier(&self) -> u8 {
        self.carrier
    }
    fn elems(&self) -> Option<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone> {
        self.elems.as_ref().map(|_l| _l.iter())
    }
}

#[allow(dead_code)]
impl CodecRepeatPresentIfBasic {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecRepeatPresentIfBasicSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecRepeatPresentIfBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatPresentIfBasicSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let _k_elems: usize = src.elems().map_or(0, |_l| _l.count());
        if _k_elems > 32 {
            return Err(CodecError::TooManyElements);
        }
        let _p_carrier_has_list_0: bool = src.elems().is_some();
        let _f_d_num_elems: Option<usize> = if src.elems().is_some() { Some(_k_elems) } else { None };
        let _d_num_elems: u8 = _f_d_num_elems.unwrap_or(0) as u8;
        let _c_carrier: u8 = (src.carrier() & !0x1) | u8::from(_p_carrier_has_list_0);
        w.write_u8(_c_carrier)?;
        if (_c_carrier & 0x1) != 0 {
            w.write_u8(_d_num_elems)?;
        }
        if let Some(_v) = src.elems() {
            for _e in _v {
                CodecRepeatElem::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecRepeatPresentIfBasicSource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<S: CodecRepeatPresentIfBasicSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecRepeatPresentIfBasic {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecRepeatPresentIfBasic {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecRepeatPresentIfBasicSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecRepeatPresentIfBasicSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecRepeatPresentIfBasicView` is what a decode of `CodecRepeatPresentIfBasic` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecRepeatPresentIfBasic` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecRepeatPresentIfBasic::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
pub struct CodecRepeatPresentIfBasicElemsWalk;

impl ListWalk<0> for CodecRepeatPresentIfBasicElemsWalk {
    type Entry<'b> = CodecRepeatElemView;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 0]) -> Result<Self::Entry<'b>, CodecError> {
        CodecRepeatElemView::decode(_c)
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 0]) {  }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecRepeatPresentIfBasicView<'a> {
    pub carrier: u8,
    pub num_elems: Option<u8>,
    pub elems: Option<ListView<'a, CodecRepeatPresentIfBasicElemsWalk, 0>>,
}

#[allow(dead_code)]
impl<'a> CodecRepeatPresentIfBasicView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecRepeatPresentIfBasic::decode`], held as a view. On success the
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
        let carrier = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let num_elems = if (carrier & 0x01u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        if (carrier & 0x01u8) != 0 {
            let _n = num_elems.expect("co-gating: count present-if matches repeat");
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            for _ in 0.._n {
                CodecRepeatElemView::decode(cursor)?;
                if _count == 32 {
                    return Err(CodecError::TooManyElements);
                }
                _count += 1;
            }
            _out.elems = Some(ListView::new(_rest, _count, []));
        } else {
            _out.elems = None;
        }
        _out.carrier = carrier;
        _out.num_elems = num_elems;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecRepeatPresentIfBasic`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder(&self, _out: &mut CodecRepeatPresentIfBasic) -> Result<(), CodecError> {
        _out.carrier = self.carrier;
        _out.num_elems = self.num_elems;
        match &self.elems {
            Some(_l) => {
                let _vec = _out.elems.insert(HeaplessVec::new());
                _vec.clear();
                for _e in _l.iter() {
                    _vec.push(CodecRepeatElem::default()).map_err(|_| CodecError::TooManyElements)?;
                    if let Some(_slot) = _vec.last_mut() {
                        _e.into_builder(_slot)?;
                    }
                }
            }
            None => _out.elems = None,
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecRepeatPresentIfBasic::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 66;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecRepeatPresentIfBasic::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> where CodecError: From<S::Error> {
        CodecRepeatPresentIfBasic::encode_source(self, w)
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

    pub fn has_list(&self) -> bool {
        (self.carrier & 0x01) != 0
    }

}
