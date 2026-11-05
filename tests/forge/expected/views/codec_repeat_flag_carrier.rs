#![doc = "SCE-MAP: codec_repeat_flag_carrier:15 :: _forge_body"]
// SCE-MAP: codec_repeat_flag_carrier:15 :: _forge_body

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

use sce_forge_runtime::codec::{Bounded, ListView, ListWalk};
use super::codec_chain_flag_entry::CodecChainFlagEntryView;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
/// The field `entries` of `CodecRepeatFlagCarrier`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecRepeatFlagCarrierEntriesField;

impl ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagCarrierEntriesField> for ::sce_forge_runtime::codec::Inline {
    type List<T> = ::sce_forge_runtime::heapless::Vec<T, 4> where T: Default + ::core::fmt::Debug + Clone + PartialEq;
}

/// What `CodecRepeatFlagCarrierIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecRepeatFlagCarrierStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagCarrierEntriesField> {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagCarrierEntriesField>> CodecRepeatFlagCarrierStorage for S {}

/// What `CodecRepeatFlagCarrierIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecRepeatFlagCarrier` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecRepeatFlagCarrierLentStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagCarrierEntriesField> {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagCarrierEntriesField>> CodecRepeatFlagCarrierLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecRepeatFlagCarrierStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecRepeatFlagCarrierFrom<'f>: CodecRepeatFlagCarrierStorage {}

impl<'f, S: CodecRepeatFlagCarrierStorage> CodecRepeatFlagCarrierFrom<'f> for S {}

// `CodecRepeatFlagCarrierIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecRepeatFlagCarrier<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecRepeatFlagCarrierOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecRepeatFlagCarrierIn<S: CodecRepeatFlagCarrierStorage> {
    pub head: u8,
    pub count: u8,
    pub entries: CodecRepeatFlagCarrierEntriesList<S>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecRepeatFlagCarrier<'a> = CodecRepeatFlagCarrierIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecRepeatFlagCarrierOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecRepeatFlagCarrierIn<S>;

impl<S: CodecRepeatFlagCarrierStorage> Default for CodecRepeatFlagCarrierIn<S> {
    fn default() -> Self {
        Self {
            head: Default::default(),
            count: Default::default(),
            entries: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecRepeatFlagCarrierStorage> CodecRepeatFlagCarrierIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecRepeatFlagCarrierFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecRepeatFlagCarrierFrom<'f> {
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
        let head = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let count = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        if count > 4 {
                return Err(CodecError::TooManyElements);
            }
            ::sce_forge_runtime::codec::note_capacity(_out.entries.refill_from((head & 0x1) != 0, |_vec| {
            let mut _full = false;
            for _ in 0..count {
                let mut _entry = CodecChainFlagEntry::default();
                ::sce_forge_runtime::codec::note_capacity(CodecChainFlagEntry::decode_into(cursor, &mut _entry, head & 0x1), &mut _full)?;
                ::sce_forge_runtime::codec::push_keeping(_vec, _entry, &[], &mut _full);
            }
            if _full {
                return Err(CodecError::CapacityExceeded);
            }
            Ok(())
        }), &mut _over)?;
        _out.head = head;
        _out.count = count;
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 18;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecRepeatFlagCarrierWire::encode_source(self, w)
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
pub type CodecRepeatFlagCarrierEntriesLaw = ::sce_forge_runtime::codec::Uniform<CodecRepeatFlagCarrierEntriesWide>;

/// The list `entries` at the storage profile `S`, held to its law
/// (docs/adr/0065 §6 R8).
#[allow(dead_code)]
pub type CodecRepeatFlagCarrierEntriesList<S> = ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecRepeatFlagCarrierEntriesField>>::List<CodecChainFlagEntry>, 4>, CodecRepeatFlagCarrierEntriesLaw>;

/// The input `wide` each entry of `entries` says, which the list hands
/// every entry as `head.W` and holds to its law (docs/adr/0065 §6 R8).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecRepeatFlagCarrierEntriesWide;

impl<E: super::codec_chain_flag_entry::CodecChainFlagEntrySource + ?Sized> ::sce_forge_runtime::codec::Says<E> for CodecRepeatFlagCarrierEntriesWide {
    fn says(entry: &E) -> Option<bool> {
        Some(super::codec_chain_flag_entry::CodecChainFlagEntryWire::input_wide(entry))
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecRepeatFlagCarrier` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecRepeatFlagCarrierWire::encode_source`]).
pub trait CodecRepeatFlagCarrierSource {
    /// The `head` field.
    fn head(&self) -> u8;
    /// The `entries` field.
    fn entries(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>, CodecRepeatFlagCarrierEntriesLaw>;
}

impl<T: CodecRepeatFlagCarrierSource + ?Sized> CodecRepeatFlagCarrierSource for &T {
    fn head(&self) -> u8 {
        (**self).head()
    }
    fn entries(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>, CodecRepeatFlagCarrierEntriesLaw> {
        (**self).entries()
    }
}

/// A value of `CodecRepeatFlagCarrier` held in the types a caller already has: a source
/// ([`CodecRepeatFlagCarrierSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecRepeatFlagCarrierParts<'a> {
    /// The `head` field.
    pub head: u8,
    /// The `entries` field.
    pub entries: ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::Bounded<&'a [super::codec_chain_flag_entry::CodecChainFlagEntryParts], 4>, CodecRepeatFlagCarrierEntriesLaw>,
}

impl<'a> CodecRepeatFlagCarrierSource for CodecRepeatFlagCarrierParts<'a> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>, CodecRepeatFlagCarrierEntriesLaw> {
        self.entries.walk()
    }
}

impl<S: CodecRepeatFlagCarrierStorage> CodecRepeatFlagCarrierSource for CodecRepeatFlagCarrierOwned<S> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>, CodecRepeatFlagCarrierEntriesLaw> {
        self.entries.list_walk()
    }
}

impl<'a> CodecRepeatFlagCarrierSource for CodecRepeatFlagCarrierView<'a> {
    fn head(&self) -> u8 {
        self.head
    }
    fn entries(&self) -> ::sce_forge_runtime::codec::Lawful<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_flag_entry::CodecChainFlagEntrySource> + Clone, 4>, CodecRepeatFlagCarrierEntriesLaw> {
        self.entries.walk()
    }
}

/// The codec `CodecRepeatFlagCarrier` itself: what writes a value read through
/// [`CodecRepeatFlagCarrierSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecRepeatFlagCarrierWire;

#[allow(dead_code)]
impl CodecRepeatFlagCarrierWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecRepeatFlagCarrier::encode` writes for the value it holds, read through
    /// [`CodecRepeatFlagCarrierSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecRepeatFlagCarrierSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecRepeatFlagCarrierSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _k_entries: usize = src.entries().len();
        let _n_d_head_w_0: usize = usize::from(src.entries().start());
        let _d_count: u8 = _k_entries as u8;
        let _v_head_w: u8 = _n_d_head_w_0 as u8;
        let _c_head: u8 = (src.head() & !0x1) | _v_head_w;
        w.write_u8(_c_head)?;
        w.write_u8(_d_count)?;
        for _e in src.entries() {
            super::codec_chain_flag_entry::CodecChainFlagEntryWire::write_source(&_e, w)?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecRepeatFlagCarrierSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecRepeatFlagCarrierSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecRepeatFlagCarrierWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecRepeatFlagCarrierWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecRepeatFlagCarrierSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecRepeatFlagCarrierSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecRepeatFlagCarrierView` is what a decode of `CodecRepeatFlagCarrier` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecRepeatFlagCarrier` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecRepeatFlagCarrier::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
pub struct CodecRepeatFlagCarrierEntriesWalk;

impl ListWalk<1> for CodecRepeatFlagCarrierEntriesWalk {
    type Entry<'b> = CodecChainFlagEntryView;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 1]) -> Result<Self::Entry<'b>, CodecError> {
        CodecChainFlagEntryView::decode(_c, _a[0])
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 1]) {  }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecRepeatFlagCarrierView<'a> {
    pub head: u8,
    pub count: u8,
    pub entries: ::sce_forge_runtime::codec::Lawful<Bounded<ListView<'a, CodecRepeatFlagCarrierEntriesWalk, 1>, 4>, CodecRepeatFlagCarrierEntriesLaw>,
}

#[allow(dead_code)]
impl<'a> CodecRepeatFlagCarrierView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecRepeatFlagCarrierIn::decode`], held as a view. On success the
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
        let head = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let count = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        {
            if count > 4 {
                return Err(CodecError::TooManyElements);
            }
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            for _ in 0..count {
                if _count == 4 {
                    return Err(CodecError::TooManyElements);
                }
                CodecChainFlagEntryView::decode(cursor, head & 0x1)?;
                _count += 1;
            }
            _out.entries = ::sce_forge_runtime::codec::Lawful::new((head & 0x1) != 0, Bounded::new(ListView::new(_rest, _count, [head & 0x1])).ok_or(CodecError::TooManyElements)?).ok_or(CodecError::PresentIfMismatch)?;
        }
        _out.head = head;
        _out.count = count;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecRepeatFlagCarrierIn`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder<D>(&self, _out: &mut CodecRepeatFlagCarrierIn<::sce_forge_runtime::codec::Lent<'a, D>>) -> Result<(), CodecError> where D: CodecRepeatFlagCarrierLentStorage {
        _out.head = self.head;
        _out.count = self.count;
        _out.entries.refill_from(self.entries.start(), |_vec| {
            for _e in self.entries.iter() {
                ::sce_forge_runtime::codec::SceList::try_push(_vec, Default::default())?;
                if let Some(_slot) = ::sce_forge_runtime::codec::SceList::last_mut(_vec) {
                    _e.into_builder(_slot)?;
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecRepeatFlagCarrierIn::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 18;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecRepeatFlagCarrierIn::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecRepeatFlagCarrierWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }

    pub fn w(&self) -> bool {
        (self.head & 0x01) != 0
    }

}

// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecRepeatFlagCarrier<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecRepeatFlagCarrierOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecRepeatFlagCarrierOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecRepeatFlagCarrierOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecRepeatFlagCarrierOwned` alone is the build's
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
// `let v: CodecRepeatFlagCarrierOwned = CodecRepeatFlagCarrierOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecRepeatFlagCarrier<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecRepeatFlagCarrierIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecRepeatFlagCarrierLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecRepeatFlagCarrierOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecRepeatFlagCarrierFrom<'a>>(self) -> Result<CodecRepeatFlagCarrierOwned<S>, CodecError> {
        Ok(CodecRepeatFlagCarrierOwned {
            head: self.head,
            count: self.count,
            entries: ::sce_forge_runtime::codec::try_collect_lawful(self.entries, Ok)?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecRepeatFlagCarrierOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecRepeatFlagCarrierStorage> CodecRepeatFlagCarrierOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecRepeatFlagCarrierIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecRepeatFlagCarrierIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        _out.head = self.head;
        _out.count = self.count;
        _out.entries.refill_from(self.entries.start(), |_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, self.entries.inner(), |_e| Ok(_e.clone())))?;
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
    pub fn transcode_in<D: for<'x> CodecRepeatFlagCarrierFrom<'x>>(&self) -> Result<CodecRepeatFlagCarrierOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
