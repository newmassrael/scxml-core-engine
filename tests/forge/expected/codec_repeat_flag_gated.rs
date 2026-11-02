#![doc = "SCE-MAP: codec_repeat_flag_gated:14 :: _forge_body"]
// SCE-MAP: codec_repeat_flag_gated:14 :: _forge_body

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

use super::codec_chain_flag_entry::CodecChainFlagEntry;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
/// The field `entries` of `CodecRepeatFlagGated`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecRepeatFlagGatedEntriesField;

impl ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagGatedEntriesField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 4> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecRepeatFlagGatedIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecRepeatFlagGatedStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagGatedEntriesField> {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagGatedEntriesField>> CodecRepeatFlagGatedStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecRepeatFlagGatedStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecRepeatFlagGatedFrom<'f>: CodecRepeatFlagGatedStorage {}

impl<'f, S: CodecRepeatFlagGatedStorage> CodecRepeatFlagGatedFrom<'f> for S {}

// `CodecRepeatFlagGatedIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecRepeatFlagGated<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecRepeatFlagGatedOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecRepeatFlagGatedIn<S: CodecRepeatFlagGatedStorage> {
    pub head: u8,
    pub count: Option<u8>,
    pub entries: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagGatedEntriesField>>::List<CodecChainFlagEntry>, 4>>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecRepeatFlagGated<'a> = CodecRepeatFlagGatedIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecRepeatFlagGatedOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecRepeatFlagGatedIn<S>;

impl<S: CodecRepeatFlagGatedStorage> Default for CodecRepeatFlagGatedIn<S> {
    fn default() -> Self {
        Self {
            head: Default::default(),
            count: Default::default(),
            entries: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecRepeatFlagGatedStorage> CodecRepeatFlagGatedIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecRepeatFlagGatedFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecRepeatFlagGatedFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecRepeatFlagGated::store_into(&mut _frame, &mut CodecRepeatFlagGated::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecRepeatFlagGatedFrom<'f> {
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
        let head = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let count = if (head & 0x02u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        if (head & 0x02u8) != 0 {
            let _n = count.expect("co-gating: count present-if matches repeat");
            if _n > 4 {
                return Err(CodecError::TooManyElements);
            }
            _out.entries.insert(Default::default()).refill(|_vec| {
                for _ in 0.._n {
                    ::sce_forge_runtime::codec::SceList::try_push(_vec, CodecChainFlagEntry::decode(cursor, head & 0x1)?)?;
                }
                Ok(())
            })?;
        } else {
            _out.entries = None;
        }
        _out.head = head;
        _out.count = count;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn w(&self) -> bool {
        (self.head & 0x01) != 0
    }

    pub fn set_w(&mut self, v: bool) {
        if v {
            self.head |= 0x01;
        } else {
            self.head &= !0x01;
        }
    }

    pub fn l(&self) -> bool {
        (self.head & 0x02) != 0
    }

    pub fn set_l(&mut self, v: bool) {
        if v {
            self.head |= 0x02;
        } else {
            self.head &= !0x02;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 18;

    /// Encode `self` into the caller-owned sink. Returns the value's refusal
    /// when the encoding cannot carry it (a length past its type, a list past
    /// its bound, a gate that disagrees with a presence; the document declares
    /// `sce:encode-may-fail`), or the sink's own error:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        CodecRepeatFlagGated::encode_source(self, w)
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

/// Flags of `CodecRepeatFlagGated` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecRepeatFlagGated`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecRepeatFlagGatedOverrides {
    /// The `W` flag of `head`, when a parent sets it.
    pub head_w: Option<bool>,
}

#[allow(dead_code)]
impl CodecRepeatFlagGatedOverrides {
    /// Sets `W` of `head` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn head_w(value: bool) -> Self {
        Self { head_w: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecRepeatFlagGated` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecRepeatFlagGated`
/// computes those from what is ([`CodecRepeatFlagGated::encode_source`]).
pub trait CodecRepeatFlagGatedSource {
    /// The `head` field.
    fn head(&self) -> u8;
    /// The `entries` field.
    fn entries(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>>;
}

impl<T: CodecRepeatFlagGatedSource + ?Sized> CodecRepeatFlagGatedSource for &T {
    fn head(&self) -> u8 {
        (**self).head()
    }
    fn entries(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>> {
        (**self).entries()
    }
}

/// A value of `CodecRepeatFlagGated` held in the types a caller already has: a source
/// ([`CodecRepeatFlagGatedSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecRepeatFlagGatedParts<'a> {
    /// The `head` field.
    pub head: u8,
    /// The `entries` field.
    pub entries: Option<::sce_forge_runtime::codec::Bounded<&'a [super::codec_chain_flag_entry::CodecChainFlagEntryParts], 4>>,
}

impl<'a> CodecRepeatFlagGatedSource for CodecRepeatFlagGatedParts<'a> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>> {
        self.entries.as_ref().map(|_l| _l.iter())
    }
}

impl<S: CodecRepeatFlagGatedStorage> CodecRepeatFlagGatedSource for CodecRepeatFlagGatedOwned<S> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>> {
        self.entries.as_ref().map(|_l| _l.list_iter())
    }
}

#[allow(dead_code)]
impl<'a> CodecRepeatFlagGated<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecRepeatFlagGatedSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecRepeatFlagGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatFlagGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source_in(src, w, &CodecRepeatFlagGatedOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecRepeatFlagGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecRepeatFlagGatedOverrides) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let _k_entries: usize = src.entries().map_or(0, |_l| _l.len());
        let _p_head_l_0: bool = src.entries().is_some();
        let _f_d_count: Option<usize> = if src.entries().is_some() { Some(_k_entries) } else { None };
        let _d_count: u8 = _f_d_count.unwrap_or(0) as u8;
        let mut _c_head: u8 = (src.head() & !0x2) | (u8::from(_p_head_l_0) << 1);
        if let Some(_o) = over.head_w {
            _c_head = (_c_head & !0x1) | u8::from(_o);
        }
        {
            let _h: bool = (_c_head & 0x1) != 0;
            if src.entries().is_some_and(|mut _l| _l.any(|_e| CodecChainFlagEntry::input_wide(&_e) != _h)) {
                return Err(CodecError::PresentIfMismatch);
            }
        }
        w.write_u8(_c_head)?;
        if (_c_head & 0x2) != 0 {
            w.write_u8(_d_count)?;
        }
        if let Some(_v) = src.entries() {
            for _e in _v {
                CodecChainFlagEntry::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecRepeatFlagGatedSource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<'a, S: CodecRepeatFlagGatedSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecRepeatFlagGated<'a> {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecRepeatFlagGated<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecRepeatFlagGatedSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecRepeatFlagGatedSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecRepeatFlagGated<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecRepeatFlagGatedOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecRepeatFlagGatedOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecRepeatFlagGatedOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecRepeatFlagGatedOwned` alone is the build's
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
// `let v: CodecRepeatFlagGatedOwned = CodecRepeatFlagGatedOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a> CodecRepeatFlagGated<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecRepeatFlagGatedOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecRepeatFlagGatedFrom<'a>>(self) -> Result<CodecRepeatFlagGatedOwned<S>, CodecError> {
        Ok(CodecRepeatFlagGatedOwned {
            head: self.head,
            count: self.count,
            entries: self.entries.map(|_v| ::sce_forge_runtime::codec::try_collect_bounded(_v, Ok)).transpose()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecRepeatFlagGatedOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecRepeatFlagGatedStorage> CodecRepeatFlagGatedOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecRepeatFlagGated<'_>, CodecError> {
        let mut _out = CodecRepeatFlagGated::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecRepeatFlagGated<'o>) -> Result<(), CodecError> {
        _out.head = self.head;
        _out.count = self.count;
        match &self.entries {
            Some(_l) => _out.entries.insert(Default::default()).refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, _l.inner(), |_e| Ok(_e.clone())))?,
            None => _out.entries = None,
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
    pub fn transcode_in<D: for<'x> CodecRepeatFlagGatedFrom<'x>>(&self) -> Result<CodecRepeatFlagGatedOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
