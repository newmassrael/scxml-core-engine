#![doc = "SCE-MAP: codec_chain_prev_gated:17 :: _forge_body"]
// SCE-MAP: codec_chain_prev_gated:17 :: _forge_body

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

use super::codec_chain_prev_entry::CodecChainPrevEntry;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
/// The field `entries` of `CodecChainPrevGated`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecChainPrevGatedEntriesField;

impl ::sce_forge_runtime::codec::HoldsList<CodecChainPrevGatedEntriesField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 4> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecChainPrevGatedIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecChainPrevGatedStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecChainPrevGatedEntriesField> {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecChainPrevGatedEntriesField>> CodecChainPrevGatedStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecChainPrevGatedStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecChainPrevGatedFrom<'f>: CodecChainPrevGatedStorage {}

impl<'f, S: CodecChainPrevGatedStorage> CodecChainPrevGatedFrom<'f> for S {}

// `CodecChainPrevGatedIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecChainPrevGated<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecChainPrevGatedOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecChainPrevGatedIn<S: CodecChainPrevGatedStorage> {
    pub head: u8,
    pub entries: Option<CodecChainPrevGatedEntriesList<S>>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecChainPrevGated<'a> = CodecChainPrevGatedIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecChainPrevGatedOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecChainPrevGatedIn<S>;

impl<S: CodecChainPrevGatedStorage> Default for CodecChainPrevGatedIn<S> {
    fn default() -> Self {
        Self {
            head: Default::default(),
            entries: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecChainPrevGatedStorage> CodecChainPrevGatedIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecChainPrevGatedFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecChainPrevGatedFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out) {
            Err(CodecError::CapacityExceeded) => {
                CodecChainPrevGated::store_into(&mut _frame, &mut CodecChainPrevGated::default())?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecChainPrevGatedFrom<'f> {
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
        if (head & 0x02u8) != 0 {
            _out.entries.insert(Default::default()).refill_from(((head & 0x1) != 0, (),), |_vec| {
                let mut _prev_entries_after_marker: u8 = 0;
                let mut _more = false;
                for _ in 0..4u32 {
                    let _entry = CodecChainPrevEntry::decode(cursor, _prev_entries_after_marker, head & 0x1)?;
                    _more = _entry.z();
                    _prev_entries_after_marker = u8::from((u64::from(_entry.header) & 127u64) == 4u64);
                    // The loop walks the chain's depth and `_vec` holds the capacity, so
                // this push fails only where the deployment holds fewer entries than
                // the frame says. An over-long chain is refused after the loop.
                ::sce_forge_runtime::codec::SceList::try_push(_vec, _entry)?;
                    if !_more { break; }
                }
                if _more && cursor.remaining() == 0 {
                    return Err(CodecError::NeedMoreBytes);
                }
                if _more {
                    return Err(CodecError::TooManyElements);
                }
                Ok(())
            })?;
        } else {
            _out.entries = None;
        }
        _out.head = head;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn t(&self) -> bool {
        (self.head & 0x01) != 0
    }

    pub fn set_t(&mut self, v: bool) {
        if v {
            self.head |= 0x01;
        } else {
            self.head &= !0x01;
        }
    }

    pub fn e(&self) -> bool {
        (self.head & 0x02) != 0
    }

    pub fn set_e(&mut self, v: bool) {
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
    pub const MAX_ENCODED_BYTES: usize = 21;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecChainPrevGated::encode_source(self, w)
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

/// The law the entries of `entries` follow in the inputs they say, which
/// the list holds them to (docs/adr/0065 §6 R8).
#[allow(dead_code)]
pub type CodecChainPrevGatedEntriesLaw = (::sce_forge_runtime::codec::Uniform<CodecChainPrevGatedEntriesTagged>, ::sce_forge_runtime::codec::AfterKey<CodecChainPrevGatedEntriesAfterMarker, CodecChainPrevGatedEntriesHas0x4Key>);

/// The list `entries` at the storage profile `S`, held to its law
/// (docs/adr/0065 §6 R8).
#[allow(dead_code)]
pub type CodecChainPrevGatedEntriesList<S> = ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecChainPrevGatedEntriesField>>::List<CodecChainPrevEntry>, 4, 1>, CodecChainPrevGatedEntriesLaw>;

/// The input `tagged` each entry of `entries` says, which the list hands
/// every entry as `head.T` and holds to its law (docs/adr/0065 §6 R8).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecChainPrevGatedEntriesTagged;

impl<E: super::codec_chain_prev_entry::CodecChainPrevEntrySource + ?Sized> ::sce_forge_runtime::codec::Says<E> for CodecChainPrevGatedEntriesTagged {
    fn says(entry: &E) -> Option<bool> {
        Some(CodecChainPrevEntry::input_tagged(entry))
    }
}

/// The input `after_marker` each entry of `entries` says, which the list hands
/// every entry as whether the entry before has the identifier 0x4 and holds to its law (docs/adr/0065 §6 R8).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecChainPrevGatedEntriesAfterMarker;

impl<E: super::codec_chain_prev_entry::CodecChainPrevEntrySource + ?Sized> ::sce_forge_runtime::codec::Says<E> for CodecChainPrevGatedEntriesAfterMarker {
    fn says(entry: &E) -> Option<bool> {
        Some(CodecChainPrevEntry::input_after_marker(entry))
    }
}

/// The kind of entry the entry after which `entries` supplies its input
/// set: an entry whose identifier is 0x4 (docs/adr/0065 §6 R8b).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecChainPrevGatedEntriesHas0x4Key;

impl<E: super::codec_chain_prev_entry::CodecChainPrevEntrySource + ?Sized> ::sce_forge_runtime::codec::Key<E> for CodecChainPrevGatedEntriesHas0x4Key {
    fn matches(entry: &E) -> bool {
        (u64::from(super::codec_chain_prev_entry::CodecChainPrevEntrySource::header(entry)) & 0x7F) == 4u64
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainPrevGated` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainPrevGated`
/// computes those from what is ([`CodecChainPrevGated::encode_source`]).
pub trait CodecChainPrevGatedSource {
    /// The `head` field.
    fn head(&self) -> u8;
    /// The `entries` field.
    fn entries(&self) -> Option<::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone, 4, 1>, CodecChainPrevGatedEntriesLaw>>;
}

impl<T: CodecChainPrevGatedSource + ?Sized> CodecChainPrevGatedSource for &T {
    fn head(&self) -> u8 {
        (**self).head()
    }
    fn entries(&self) -> Option<::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone, 4, 1>, CodecChainPrevGatedEntriesLaw>> {
        (**self).entries()
    }
}

/// A value of `CodecChainPrevGated` held in the types a caller already has: a source
/// ([`CodecChainPrevGatedSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainPrevGatedParts<'a> {
    /// The `head` field.
    pub head: u8,
    /// The `entries` field.
    pub entries: Option<::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::Bounded<&'a [super::codec_chain_prev_entry::CodecChainPrevEntryParts], 4, 1>, CodecChainPrevGatedEntriesLaw>>,
}

impl<'a> CodecChainPrevGatedSource for CodecChainPrevGatedParts<'a> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> Option<::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone, 4, 1>, CodecChainPrevGatedEntriesLaw>> {
        self.entries.as_ref().map(|_l| _l.walk())
    }
}

impl<S: CodecChainPrevGatedStorage> CodecChainPrevGatedSource for CodecChainPrevGatedOwned<S> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> Option<::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_prev_entry::CodecChainPrevEntrySource> + Clone, 4, 1>, CodecChainPrevGatedEntriesLaw>> {
        self.entries.as_ref().map(|_l| _l.list_walk())
    }
}

#[allow(dead_code)]
impl<'a> CodecChainPrevGated<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecChainPrevGatedSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecChainPrevGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainPrevGatedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _k_entries: usize = src.entries().map_or(0, |_l| _l.len());
        let _n_d_head_t_0: usize = src.entries().map_or(0, |_l| usize::from(_l.start().0));
        let _f_d_head_t: Option<usize> = if src.entries().is_some() { Some(_n_d_head_t_0) } else { None };
        let _p_head_e_0: bool = src.entries().is_some();
        let mut _c_head: u8 = (src.head() & !0x2) | (u8::from(_p_head_e_0) << 1);
        if let Some(_n) = _f_d_head_t {
            _c_head = (_c_head & !0x1) | (_n as u8);
        }
        w.write_u8(_c_head)?;
        if let Some(_v) = src.entries() {
            for (_i, _e) in _v.into_iter().enumerate() {
                CodecChainPrevEntry::write_source_in(&_e, w, &super::codec_chain_prev_entry::CodecChainPrevEntryOverrides::header_z(_i + 1 < _k_entries))?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecChainPrevGatedSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<'a, S: CodecChainPrevGatedSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecChainPrevGated<'a> {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecChainPrevGated<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecChainPrevGatedSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecChainPrevGatedSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecChainPrevGated<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecChainPrevGatedOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecChainPrevGatedOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecChainPrevGatedOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecChainPrevGatedOwned` alone is the build's
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
// `let v: CodecChainPrevGatedOwned = CodecChainPrevGatedOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

#[allow(dead_code)]
impl<'a> CodecChainPrevGated<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecChainPrevGatedOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecChainPrevGatedFrom<'a>>(self) -> Result<CodecChainPrevGatedOwned<S>, CodecError> {
        Ok(CodecChainPrevGatedOwned {
            head: self.head,
            entries: self.entries.map(|_v| ::sce_forge_runtime::codec::try_collect_lawful(_v, Ok)).transpose()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecChainPrevGatedOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecChainPrevGatedStorage> CodecChainPrevGatedOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecChainPrevGated<'_>, CodecError> {
        let mut _out = CodecChainPrevGated::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecChainPrevGated<'o>) -> Result<(), CodecError> {
        _out.head = self.head;
        match &self.entries {
            Some(_l) => _out.entries.insert(Default::default()).refill_from(_l.start(), |_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, _l.inner(), |_e| Ok(_e.clone())))?,
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
    pub fn transcode_in<D: for<'x> CodecChainPrevGatedFrom<'x>>(&self) -> Result<CodecChainPrevGatedOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
