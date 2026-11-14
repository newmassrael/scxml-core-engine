#![doc = "SCE-MAP: codec_origin_envelope:19 :: _forge_body"]
// SCE-MAP: codec_origin_envelope:19 :: _forge_body

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
use super::codec_origin_leaf::CodecOriginLeafOwned;
/// The field `note` of `CodecOriginEnvelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecOriginEnvelopeNoteField;

impl ::sce_forge_runtime::codec::HoldsStr<CodecOriginEnvelopeNoteField> for ::sce_forge_runtime::codec::Inline {
    type Str = ::sce_forge_runtime::codec::InlineStr<16>;
}

/// The field `items` of `CodecOriginEnvelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecOriginEnvelopeItemsField;

impl ::sce_forge_runtime::codec::HoldsList<CodecOriginEnvelopeItemsField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 3> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecOriginEnvelopeIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecOriginEnvelopeStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsStr<CodecOriginEnvelopeNoteField> + ::sce_forge_runtime::codec::HoldsList<CodecOriginEnvelopeItemsField> + super::codec_origin_leaf::CodecOriginLeafStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsStr<CodecOriginEnvelopeNoteField> + ::sce_forge_runtime::codec::HoldsList<CodecOriginEnvelopeItemsField> + super::codec_origin_leaf::CodecOriginLeafStorage> CodecOriginEnvelopeStorage for S {}

/// What `CodecOriginEnvelopeIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecOriginEnvelope` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecOriginEnvelopeLentStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecOriginEnvelopeItemsField> + super::codec_origin_leaf::CodecOriginLeafLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecOriginEnvelopeItemsField> + super::codec_origin_leaf::CodecOriginLeafLentStorage> CodecOriginEnvelopeLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecOriginEnvelopeStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecOriginEnvelopeFrom<'f>: CodecOriginEnvelopeStorage + ::sce_forge_runtime::codec::StoresStr<'f, CodecOriginEnvelopeNoteField> + super::codec_origin_leaf::CodecOriginLeafFrom<'f> {}

impl<'f, S: CodecOriginEnvelopeStorage + ::sce_forge_runtime::codec::StoresStr<'f, CodecOriginEnvelopeNoteField> + super::codec_origin_leaf::CodecOriginLeafFrom<'f>> CodecOriginEnvelopeFrom<'f> for S {}

// `CodecOriginEnvelopeIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecOriginEnvelope<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecOriginEnvelopeOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecOriginEnvelopeIn<S: CodecOriginEnvelopeStorage> {
    pub hdr: u8,
    pub note_len: u8,
    pub note: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsStr<CodecOriginEnvelopeNoteField>>::Str, 16>,
    pub m: u8,
    pub required: CodecOriginLeafOwned<S>,
    pub optional: Option<CodecOriginLeafOwned<S>>,
    pub items: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecOriginEnvelopeItemsField>>::List<CodecOriginLeafOwned<S>>, 3>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecOriginEnvelope<'a> = CodecOriginEnvelopeIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecOriginEnvelopeOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecOriginEnvelopeIn<S>;

impl<S: CodecOriginEnvelopeStorage> Default for CodecOriginEnvelopeIn<S> {
    fn default() -> Self {
        Self {
            hdr: Default::default(),
            note_len: Default::default(),
            note: Default::default(),
            m: Default::default(),
            required: Default::default(),
            optional: Default::default(),
            items: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecOriginEnvelopeStorage> CodecOriginEnvelopeIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecOriginEnvelopeFrom<'f> {
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
    ///
    /// A field past what the profile holds is read on and not held, and the
    /// frame is refused with `CapacityExceeded` only once it is read whole and
    /// every refusal of its own has had its turn (docs/adr/0077 §7), so this
    /// node and one that holds more refuse a malformed frame alike. On `Err`,
    /// `_out` holds the fields decoded so far and is not to be read; each of
    /// them holds what its type says.
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecOriginEnvelopeFrom<'f> {
        // A field past what the profile holds is read on and not held; the
        // frame says so once it is read whole (docs/adr/0077 §7).
        let mut _over = false;
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
        let note_len = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let note = {
            let _len = note_len;
            if _len > 16 {
                return Err(CodecError::LengthOutOfRange);
            }
            let _n = _len as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = core::str::from_utf8(raw)
                .map_err(|_| CodecError::InvalidUtf8)?;
            cursor.advance(_n)?;
            _v
        };
        let note = ::sce_forge_runtime::codec::Bounded::new(note).ok_or(CodecError::LengthOutOfRange)?;
        let note = ::sce_forge_runtime::codec::note_held(note.store::<S, CodecOriginEnvelopeNoteField>(), &mut _over)?;
        let m = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        ::sce_forge_runtime::codec::note_capacity(CodecOriginLeafOwned::<S>::decode_into(cursor, &mut _out.required), &mut _over)?;
        if (hdr & 0x01u8) != 0 {
            ::sce_forge_runtime::codec::note_capacity(CodecOriginLeafOwned::<S>::decode_into(cursor, _out.optional.insert(CodecOriginLeafOwned::<S>::default())), &mut _over)?;
        } else {
            _out.optional = None;
        }
        if m > 3 {
                return Err(CodecError::TooManyElements);
            }
            ::sce_forge_runtime::codec::note_capacity(_out.items.refill(|_vec| {
            let mut _full = false;
            for _ in 0..m {
                let mut _entry = CodecOriginLeafOwned::<S>::default();
                ::sce_forge_runtime::codec::note_capacity(CodecOriginLeafOwned::<S>::decode_into(cursor, &mut _entry), &mut _full)?;
                ::sce_forge_runtime::codec::push_keeping(_vec, _entry, &[], &mut _full);
            }
            if _full {
                return Err(CodecError::CapacityExceeded);
            }
            Ok(())
        }), &mut _over)?;
        _out.hdr = hdr;
        _out.note_len = note_len;
        _out.note = note;
        _out.m = m;
        if _over {
            return Err(CodecError::CapacityExceeded);
        }
        Ok(())
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
    pub const MAX_ENCODED_BYTES: usize = 105;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecOriginEnvelopeWire::encode_source(self, w)
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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecOriginEnvelope` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecOriginEnvelopeWire::encode_source`]).
pub trait CodecOriginEnvelopeSource {
    /// The `hdr` field.
    fn hdr(&self) -> u8;
    /// The `note` field.
    fn note(&self) -> ::sce_forge_runtime::codec::Text<'_, 16>;
    /// The `required` field.
    fn required(&self) -> &impl super::codec_origin_leaf::CodecOriginLeafSource;
    /// The `optional` field.
    fn optional(&self) -> Option<&impl super::codec_origin_leaf::CodecOriginLeafSource>;
    /// The `items` field.
    fn items(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_origin_leaf::CodecOriginLeafSource> + Clone, 3>;
}

impl<T: CodecOriginEnvelopeSource + ?Sized> CodecOriginEnvelopeSource for &T {
    fn hdr(&self) -> u8 {
        (**self).hdr()
    }
    fn note(&self) -> ::sce_forge_runtime::codec::Text<'_, 16> {
        (**self).note()
    }
    fn required(&self) -> &impl super::codec_origin_leaf::CodecOriginLeafSource {
        (**self).required()
    }
    fn optional(&self) -> Option<&impl super::codec_origin_leaf::CodecOriginLeafSource> {
        (**self).optional()
    }
    fn items(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_origin_leaf::CodecOriginLeafSource> + Clone, 3> {
        (**self).items()
    }
}

/// A value of `CodecOriginEnvelope` held in the types a caller already has: a source
/// ([`CodecOriginEnvelopeSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecOriginEnvelopeParts<'a> {
    /// The `hdr` field.
    pub hdr: u8,
    /// The `note` field.
    pub note: ::sce_forge_runtime::codec::Text<'a, 16>,
    /// The `required` field.
    pub required: &'a super::codec_origin_leaf::CodecOriginLeafParts<'a>,
    /// The `optional` field.
    pub optional: Option<&'a super::codec_origin_leaf::CodecOriginLeafParts<'a>>,
    /// The `items` field.
    pub items: ::sce_forge_runtime::codec::Bounded<&'a [super::codec_origin_leaf::CodecOriginLeafParts<'a>], 3>,
}

impl<'a> CodecOriginEnvelopeSource for CodecOriginEnvelopeParts<'a> {
    fn hdr(&self) -> u8 {
        self.hdr
    }
    fn note(&self) -> ::sce_forge_runtime::codec::Text<'_, 16> {
        self.note
    }
    fn required(&self) -> &impl super::codec_origin_leaf::CodecOriginLeafSource {
        self.required
    }
    fn optional(&self) -> Option<&impl super::codec_origin_leaf::CodecOriginLeafSource> {
        self.optional
    }
    fn items(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_origin_leaf::CodecOriginLeafSource> + Clone, 3> {
        self.items.iter()
    }
}

impl<S: CodecOriginEnvelopeStorage> CodecOriginEnvelopeSource for CodecOriginEnvelopeOwned<S> {
    fn hdr(&self) -> u8 {
        self.hdr
    }
    fn note(&self) -> ::sce_forge_runtime::codec::Text<'_, 16> {
        self.note.as_text()
    }
    fn required(&self) -> &impl super::codec_origin_leaf::CodecOriginLeafSource {
        &self.required
    }
    fn optional(&self) -> Option<&impl super::codec_origin_leaf::CodecOriginLeafSource> {
        self.optional.as_ref()
    }
    fn items(&self) -> ::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_origin_leaf::CodecOriginLeafSource> + Clone, 3> {
        self.items.list_iter()
    }
}

/// The codec `CodecOriginEnvelope` itself: what writes a value read through
/// [`CodecOriginEnvelopeSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecOriginEnvelopeWire;

#[allow(dead_code)]
impl CodecOriginEnvelopeWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecOriginEnvelope::encode` writes for the value it holds, read through
    /// [`CodecOriginEnvelopeSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecOriginEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecOriginEnvelopeSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _k_items: usize = src.items().len();
        let _p_hdr_has_opt_0: bool = src.optional().is_some();
        let _n_d_note_len_0: usize = src.note().len();
        let _d_note_len: u8 = _n_d_note_len_0 as u8;
        let _d_m: u8 = _k_items as u8;
        let _c_hdr: u8 = (src.hdr() & !0x1) | u8::from(_p_hdr_has_opt_0);
        w.write_u8(_c_hdr)?;
        w.write_u8(_d_note_len)?;
        w.write_bytes(src.note().into_inner().as_bytes())?;
        w.write_u8(_d_m)?;
        super::codec_origin_leaf::CodecOriginLeafWire::write_source(src.required(), w)?;
        if let Some(_v) = src.optional() {
            super::codec_origin_leaf::CodecOriginLeafWire::write_source(_v, w)?;
        }
        for _e in src.items() {
            super::codec_origin_leaf::CodecOriginLeafWire::write_source(&_e, w)?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecOriginEnvelopeSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecOriginEnvelopeSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecOriginEnvelopeWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecOriginEnvelopeWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecOriginEnvelopeSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecOriginEnvelopeSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecOriginEnvelope<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecOriginEnvelopeOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecOriginEnvelopeOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecOriginEnvelopeOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecOriginEnvelopeOwned` alone is the build's
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
// `let v: CodecOriginEnvelopeOwned = CodecOriginEnvelopeOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecOriginEnvelope<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecOriginEnvelopeIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecOriginEnvelopeLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecOriginEnvelopeOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecOriginEnvelopeFrom<'a>>(self) -> Result<CodecOriginEnvelopeOwned<S>, CodecError> {
        Ok(CodecOriginEnvelopeOwned {
            hdr: self.hdr,
            note_len: self.note_len,
            note: self.note.store::<S, CodecOriginEnvelopeNoteField>()?,
            m: self.m,
            required: self.required.try_into_owned_in::<S>()?,
            optional: self.optional.map(|_v| _v.try_into_owned_in::<S>()).transpose()?,
            items: ::sce_forge_runtime::codec::try_collect_bounded(self.items, |_e| _e.try_into_owned_in::<S>())?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecOriginEnvelopeOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecOriginEnvelopeStorage> CodecOriginEnvelopeOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecOriginEnvelopeIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
        let mut _out = Default::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecOriginEnvelopeIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        _out.hdr = self.hdr;
        _out.note_len = self.note_len;
        _out.note = self.note.as_text();
        _out.m = self.m;
        self.required.as_borrowed_into(&mut _out.required);
        match &self.optional {
            Some(_v) => _v.as_borrowed_into(_out.optional.insert(Default::default())),
            None => _out.optional = None,
        }
        _out.items.refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, self.items.inner(), |_e| Ok(_e.as_borrowed())))?;
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
    pub fn transcode_in<D: for<'x> CodecOriginEnvelopeFrom<'x>>(&self) -> Result<CodecOriginEnvelopeOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}

/// What the origin projection of `CodecOriginEnvelope` asks of a storage profile
/// beside CodecOriginEnvelopeStorage: that it is an origin profile, and
/// that each byte string and text of `CodecOriginEnvelope` and of the codecs it
/// holds is built from the origin by the container the profile holds it in.
#[allow(dead_code)]
pub trait CodecOriginEnvelopeOrigin: CodecOriginEnvelopeStorage + ::sce_forge_runtime::codec::OriginStorage + ::sce_forge_runtime::codec::OriginStr<CodecOriginEnvelopeNoteField> + super::codec_origin_leaf::CodecOriginLeafOrigin {}

impl<S: CodecOriginEnvelopeStorage + ::sce_forge_runtime::codec::OriginStorage + ::sce_forge_runtime::codec::OriginStr<CodecOriginEnvelopeNoteField> + super::codec_origin_leaf::CodecOriginLeafOrigin> CodecOriginEnvelopeOrigin for S {}

#[allow(dead_code)]
impl<'a, D> CodecOriginEnvelopeIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecOriginEnvelopeLentStorage {
    /// Project this borrowed view into an owned [`CodecOriginEnvelopeOwned`]
    /// whose byte and text containers are each built from the slice AND
    /// `origin`, the buffer the view was decoded from, by the
    /// container the profile holds that field in. A container that shares
    /// that buffer refers back to it instead of copying out of it; what the
    /// origin is, and what to do with a slice that does not come from it, is
    /// the profile's to say, field by field.
    ///
    /// `try_into_owned_in` stays the copying projection. Every codec this one
    /// embeds, repeats or dispatches to must have been generated with
    /// `--owned-origin` as well.
    pub fn try_into_owned_in_origin<S: CodecOriginEnvelopeOrigin>(self, origin: &<S as ::sce_forge_runtime::codec::OriginStorage>::Origin) -> Result<CodecOriginEnvelopeOwned<S>, CodecError> {
        Ok(CodecOriginEnvelopeOwned {
            hdr: self.hdr,
            note_len: self.note_len,
            note: ::sce_forge_runtime::codec::Bounded::from_built_text(<S as ::sce_forge_runtime::codec::OriginStr<CodecOriginEnvelopeNoteField>>::str_from(origin, self.note.into_inner()))?,
            m: self.m,
            required: self.required.try_into_owned_in_origin::<S>(origin)?,
            optional: self.optional.map(|_v| _v.try_into_owned_in_origin::<S>(origin)).transpose()?,
            items: ::sce_forge_runtime::codec::try_collect_bounded(self.items, |_e| _e.try_into_owned_in_origin::<S>(origin))?,
        })
    }
}
