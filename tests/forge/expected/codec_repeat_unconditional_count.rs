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

use super::codec_repeat_elem::CodecRepeatElem;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
/// The field `links` of `CodecRepeatUnconditionalCount`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecRepeatUnconditionalCountLinksField;

impl ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountLinksField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 64> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// The field `weights` of `CodecRepeatUnconditionalCount`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecRepeatUnconditionalCountWeightsField;

impl ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountWeightsField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 64> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecRepeatUnconditionalCountIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecRepeatUnconditionalCountStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountLinksField> + ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountWeightsField> {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountLinksField> + ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountWeightsField>> CodecRepeatUnconditionalCountStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecRepeatUnconditionalCountStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecRepeatUnconditionalCountFrom<'f>: CodecRepeatUnconditionalCountStorage {}

impl<'f, S: CodecRepeatUnconditionalCountStorage> CodecRepeatUnconditionalCountFrom<'f> for S {}

/// The group `CodecRepeatUnconditionalCountOptionsH` as `CodecRepeatUnconditionalCountOwned` holds it, in the storage
/// profile `S`.
#[allow(dead_code)]
pub type CodecRepeatUnconditionalCountOptionsHOwned<S> = CodecRepeatUnconditionalCountOptionsH<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountLinksField>>::List<CodecRepeatElem>, 64>, ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountWeightsField>>::List<CodecRepeatElem>, 64>>;

// `CodecRepeatUnconditionalCountIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecRepeatUnconditionalCount<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecRepeatUnconditionalCountOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecRepeatUnconditionalCountIn<S: CodecRepeatUnconditionalCountStorage> {
    pub options: u8,
    pub links_len: u8,
    pub options_h: CodecRepeatUnconditionalCountOptionsHOwned<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecRepeatUnconditionalCount<'a> = CodecRepeatUnconditionalCountIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecRepeatUnconditionalCountOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecRepeatUnconditionalCountIn<S>;

impl<S: CodecRepeatUnconditionalCountStorage> Default for CodecRepeatUnconditionalCountIn<S> {
    fn default() -> Self {
        Self {
            options: Default::default(),
            links_len: Default::default(),
            options_h: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecRepeatUnconditionalCountStorage> CodecRepeatUnconditionalCountIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecRepeatUnconditionalCountFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecRepeatUnconditionalCountFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecRepeatUnconditionalCount::store_into(&mut _frame, &mut CodecRepeatUnconditionalCount::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecRepeatUnconditionalCountFrom<'f> {
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
        let mut links: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountLinksField>>::List<CodecRepeatElem>, 64> = Default::default();
        if links_len > 64 {
                return Err(CodecError::TooManyElements);
            }
            links.refill(|_vec| {
            for _ in 0..links_len {
                ::sce_forge_runtime::codec::SceList::try_push(_vec, CodecRepeatElem::decode(cursor)?)?;
            }
            Ok(())
        })?;
        let mut weights: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountWeightsField>>::List<CodecRepeatElem>, 64>> = None;
        if (options & 0x01u8) != 0 {
            let _n = links_len;
            if _n > 64 {
                return Err(CodecError::TooManyElements);
            }
            weights.insert(Default::default()).refill(|_vec| {
                for _ in 0.._n {
                    ::sce_forge_runtime::codec::SceList::try_push(_vec, CodecRepeatElem::decode(cursor)?)?;
                }
                Ok(())
            })?;
        } else {
            weights = None;
        }
        _out.options = options;
        _out.links_len = links_len;
        _out.options_h = if (options & 0x01u8) != 0 { CodecRepeatUnconditionalCountOptionsH::H { links_weights: ::sce_forge_runtime::codec::Aligned::new(links, weights.expect("the gate read the list")).expect("one count read both lists") } } else { CodecRepeatUnconditionalCountOptionsH::NotH { links } };
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

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecRepeatUnconditionalCount::encode_source(self, w)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`, and returns the freshly-encoded bytes: the encode refuses
    /// no value and the vector cannot run out of room. Callers targeting
    /// zero-alloc hot paths should call `encode` directly against a
    /// caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}

/// The fields `options.H` decide (docs/adr/0065 §6 R4b, R4c): a value holds the
/// ones of one arm, and `CodecRepeatUnconditionalCount` derives the gates from which.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodecRepeatUnconditionalCountOptionsH<Links, Weights> {
    /// While `options.H` holds.
    H {
        /// The `links` and `weights` lists, of one count.
        links_weights: ::sce_forge_runtime::codec::Aligned<Links, Weights>,
    },
    /// While `options.H` does not hold.
    NotH {
        /// The `links` field.
        links: Links,
    },
}

impl<Links: Default, Weights: Default> Default for CodecRepeatUnconditionalCountOptionsH<Links, Weights> {
    /// The arm held while `options.H` does not hold, each field at its type's default.
    fn default() -> Self {
        Self::NotH { links: Default::default(), }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecRepeatUnconditionalCount` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecRepeatUnconditionalCount`
/// computes those from what is ([`CodecRepeatUnconditionalCount::encode_source`]).
pub trait CodecRepeatUnconditionalCountSource {
    /// The `options` field.
    fn options(&self) -> u8;
    /// The fields `options.H` decides, held together (docs/adr/0065 §6 R4).
    fn options_h(&self) -> CodecRepeatUnconditionalCountOptionsH<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>>;
}

impl<T: CodecRepeatUnconditionalCountSource + ?Sized> CodecRepeatUnconditionalCountSource for &T {
    fn options(&self) -> u8 {
        (**self).options()
    }
    fn options_h(&self) -> CodecRepeatUnconditionalCountOptionsH<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>> {
        (**self).options_h()
    }
}

/// A value of `CodecRepeatUnconditionalCount` held in the types a caller already has: a source
/// ([`CodecRepeatUnconditionalCountSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecRepeatUnconditionalCountParts<'a> {
    /// The `options` field.
    pub options: u8,
    /// The fields `options.H` decides, held together (docs/adr/0065 §6 R4).
    pub options_h: CodecRepeatUnconditionalCountOptionsH<::sce_forge_runtime::codec::Bounded<&'a [super::codec_repeat_elem::CodecRepeatElemParts], 64>, ::sce_forge_runtime::codec::Bounded<&'a [super::codec_repeat_elem::CodecRepeatElemParts], 64>>,
}

impl<'a> CodecRepeatUnconditionalCountSource for CodecRepeatUnconditionalCountParts<'a> {
    fn options_h(&self) -> CodecRepeatUnconditionalCountOptionsH<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>> {
        match &self.options_h {
            CodecRepeatUnconditionalCountOptionsH::H { links_weights } => CodecRepeatUnconditionalCountOptionsH::H { links_weights: ::sce_forge_runtime::codec::Aligned::new(links_weights.first().iter(), links_weights.second().iter()).expect("a walk keeps its list's count") },
            CodecRepeatUnconditionalCountOptionsH::NotH { links } => CodecRepeatUnconditionalCountOptionsH::NotH { links: links.iter() },
        }
    }
    fn options(&self) -> u8 {
        self.options
    }
}

impl<S: CodecRepeatUnconditionalCountStorage> CodecRepeatUnconditionalCountSource for CodecRepeatUnconditionalCountOwned<S> {
    fn options_h(&self) -> CodecRepeatUnconditionalCountOptionsH<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>, ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_repeat_elem::CodecRepeatElemSource> + Clone, 64>> {
        match &self.options_h {
            CodecRepeatUnconditionalCountOptionsH::H { links_weights } => CodecRepeatUnconditionalCountOptionsH::H { links_weights: ::sce_forge_runtime::codec::Aligned::new(links_weights.first().list_iter(), links_weights.second().list_iter()).expect("a walk keeps its list's count") },
            CodecRepeatUnconditionalCountOptionsH::NotH { links } => CodecRepeatUnconditionalCountOptionsH::NotH { links: links.list_iter() },
        }
    }
    fn options(&self) -> u8 {
        self.options
    }
}

#[allow(dead_code)]
impl<'a> CodecRepeatUnconditionalCount<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecRepeatUnconditionalCountSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecRepeatUnconditionalCountSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatUnconditionalCountSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let (_m_links, _m_weights) = match src.options_h() {
            CodecRepeatUnconditionalCountOptionsH::H { links_weights } => { let (links, weights) = links_weights.into_parts(); (Some(links), Some(weights)) }
            CodecRepeatUnconditionalCountOptionsH::NotH { links } => (Some(links), None),
        };
        let _k_links: usize = _m_links.clone().map_or(0, |_l| _l.len());
        let _k_weights: usize = _m_weights.clone().map_or(0, |_l| _l.len());
        let _p_options_h_0: bool = _m_weights.clone().is_some();
        let _d_links_len: u8 = _k_links as u8;
        let _c_options: u8 = (src.options() & !0x1) | u8::from(_p_options_h_0);
        w.write_u8(_c_options)?;
        w.write_u8(_d_links_len)?;
        if let Some(_v) = _m_links.clone() {
            for _e in _v {
                CodecRepeatElem::write_source(&_e, w)?;
            }
        }
        if let Some(_v) = _m_weights.clone() {
            for _e in _v {
                CodecRepeatElem::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecRepeatUnconditionalCountSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<'a, S: CodecRepeatUnconditionalCountSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecRepeatUnconditionalCount<'a> {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecRepeatUnconditionalCount<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecRepeatUnconditionalCountSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecRepeatUnconditionalCountSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecRepeatUnconditionalCount<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecRepeatUnconditionalCountOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecRepeatUnconditionalCountOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecRepeatUnconditionalCountOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecRepeatUnconditionalCountOwned` alone is the build's
// default profile. Because the non-allocating profile is a *type*, this mirror
// carries no `alloc` gate at all — a list- or embed-bearing codec has an owned
// form on the heap-free tier too, and a heap-capable consumer can still pin a
// value to storage that is guaranteed not to allocate.
//
// `try_into_owned` is the fallible direction (one `?` per profile: on the
// growable profile the copy cannot fail, on the inline profile it enforces
// each declared bound); `try_as_borrowed` re-borrows any profile back
// into the single borrowed view that owns `encode`; `transcode_in` moves a
// value between profiles as a checked projection rather than a re-decode.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling one instead names it on the value or its binding —
// `let v: CodecRepeatUnconditionalCountOwned = CodecRepeatUnconditionalCountOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a> CodecRepeatUnconditionalCount<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecRepeatUnconditionalCountOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity. Every value of this type is within
    /// each declared bound, since its lists, bytes and text hold their bounds
    /// in their types (docs/adr/0077 §3), and the inline and growable profiles
    /// hold each field at its bound, so the projection does not fail on them.
    /// A profile whose capacity is below a bound raises
    /// `CodecError::CapacityExceeded` for a value it cannot hold.
    pub fn try_into_owned_in<S: CodecRepeatUnconditionalCountFrom<'a>>(self) -> Result<CodecRepeatUnconditionalCountOwned<S>, CodecError> {
        Ok(CodecRepeatUnconditionalCountOwned {
            options: self.options,
            links_len: self.links_len,
            options_h: match self.options_h { CodecRepeatUnconditionalCountOptionsH::H { links_weights } => CodecRepeatUnconditionalCountOptionsH::H { links_weights: { let (links, weights) = links_weights.into_parts(); ::sce_forge_runtime::codec::Aligned::new(::sce_forge_runtime::codec::try_collect_bounded(links, Ok)?, ::sce_forge_runtime::codec::try_collect_bounded(weights, Ok)?).expect("a projection keeps a list's count") } }, CodecRepeatUnconditionalCountOptionsH::NotH { links } => CodecRepeatUnconditionalCountOptionsH::NotH { links: ::sce_forge_runtime::codec::try_collect_bounded(links, Ok)? }, },
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecRepeatUnconditionalCountOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecRepeatUnconditionalCountStorage> CodecRepeatUnconditionalCountOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecRepeatUnconditionalCount<'_>, CodecError> {
        let mut _out = CodecRepeatUnconditionalCount::default();
        self.try_as_borrowed_into(&mut _out)?;
        Ok(_out)
    }

    /// [`Self::try_as_borrowed`] into a destination the caller owns, each
    /// bounded list and embedded body projected in the field where it sits.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through (the optional wrapper, the borrowed struct, the
    /// variant, the caller's binding), however few entries the list holds. A
    /// caller on a path where that shows projects into a destination it owns,
    /// then encodes from it. On `Err`, `_out` holds what was projected so far
    /// and is not to be read.
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecRepeatUnconditionalCount<'o>) -> Result<(), CodecError> {
        _out.options = self.options;
        _out.links_len = self.links_len;
        match &self.options_h {
            CodecRepeatUnconditionalCountOptionsH::H { links_weights } => {
                let mut _o_links: ::sce_forge_runtime::codec::Bounded<<::sce_forge_runtime::codec::Lent<'o> as ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountLinksField>>::List<CodecRepeatElem>, 64> = Default::default();
                _o_links.refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, links_weights.first().inner(), |_e| Ok(_e.clone())))?;
                let mut _o_weights: ::sce_forge_runtime::codec::Bounded<<::sce_forge_runtime::codec::Lent<'o> as ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountWeightsField>>::List<CodecRepeatElem>, 64> = Default::default();
                _o_weights.refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, links_weights.second().inner(), |_e| Ok(_e.clone())))?;
                _out.options_h = CodecRepeatUnconditionalCountOptionsH::H { links_weights: ::sce_forge_runtime::codec::Aligned::new(_o_links, _o_weights).expect("a projection keeps a list's count") };
            }
            CodecRepeatUnconditionalCountOptionsH::NotH { links } => {
                let mut _o_links: ::sce_forge_runtime::codec::Bounded<<::sce_forge_runtime::codec::Lent<'o> as ::sce_forge_runtime::codec::HoldsList<CodecRepeatUnconditionalCountLinksField>>::List<CodecRepeatElem>, 64> = Default::default();
                _o_links.refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, links.inner(), |_e| Ok(_e.clone())))?;
                _out.options_h = CodecRepeatUnconditionalCountOptionsH::NotH { links: _o_links };
            }
        }
        Ok(())
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: for<'x> CodecRepeatUnconditionalCountFrom<'x>>(&self) -> Result<CodecRepeatUnconditionalCountOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
