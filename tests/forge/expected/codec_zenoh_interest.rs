#![doc = "SCE-MAP: codec_zenoh_interest:73 :: _forge_body"]
// SCE-MAP: codec_zenoh_interest:73 :: _forge_body

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

use sce_forge_runtime::codec::{Bounded, ListView, ListWalk};
use super::codec_zenoh_ext_entry::CodecZenohExtEntryView;
use super::codec_zenoh_interest_body::CodecZenohInterestBodyView;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
use super::codec_zenoh_interest_body::CodecZenohInterestBodyOwned;
/// The field `extensions` of `CodecZenohInterest`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohInterestExtensionsField;

// `Inline` does not answer for `extensions`: the protocol states no bound of
// it, so it is held only by a profile whose container grows, or by a deployment
// that gives it a capacity (docs/adr/0077 §7).

/// What `CodecZenohInterestIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohInterestStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohInterestExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage + super::codec_zenoh_interest_body::CodecZenohInterestBodyStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohInterestExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryStorage + super::codec_zenoh_interest_body::CodecZenohInterestBodyStorage> CodecZenohInterestStorage for S {}

/// What `CodecZenohInterestIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohInterest` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohInterestLentStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohInterestExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryLentStorage + super::codec_zenoh_interest_body::CodecZenohInterestBodyLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohInterestExtensionsField> + super::codec_zenoh_ext_entry::CodecZenohExtEntryLentStorage + super::codec_zenoh_interest_body::CodecZenohInterestBodyLentStorage> CodecZenohInterestLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohInterestStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohInterestFrom<'f>: CodecZenohInterestStorage + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f> + super::codec_zenoh_interest_body::CodecZenohInterestBodyFrom<'f> {}

impl<'f, S: CodecZenohInterestStorage + super::codec_zenoh_ext_entry::CodecZenohExtEntryFrom<'f> + super::codec_zenoh_interest_body::CodecZenohInterestBodyFrom<'f>> CodecZenohInterestFrom<'f> for S {}

/// The group `CodecZenohInterestHeaderCurrentOrFuture` as `CodecZenohInterestOwned` holds it, in the storage
/// profile `S`.
#[allow(dead_code)]
pub type CodecZenohInterestHeaderCurrentOrFutureOwned<S> = CodecZenohInterestHeaderCurrentOrFuture<CodecZenohInterestBodyOwned<S>>;

// `CodecZenohInterestIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohInterest<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohInterestOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohInterestIn<S: CodecZenohInterestStorage> {
    pub header: u8,
    pub id: u64,
    pub header_current_or_future: Option<CodecZenohInterestHeaderCurrentOrFutureOwned<S>>,
    pub extensions: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecZenohInterestExtensionsField>>::List<CodecZenohExtEntryOwned<S>>, { usize::MAX }, 1>>,
}

// No eager form at the default profile: `Lent<'a>` holds each list as `Inline`
// does, and `Inline` holds no list whose protocol states no bound
// (docs/adr/0077 §7). Name the eager form at a profile that holds its lists,
// `CodecZenohInterestIn<Lent<'a, D>>`.

/// The value at a profile that holds its bytes and text, named
/// with the profile: the build's default on the heap-free tier is `Inline`, which
/// holds no field whose protocol states no bound (docs/adr/0077 §7).
#[allow(dead_code)]
pub type CodecZenohInterestOwned<S> = CodecZenohInterestIn<S>;

impl<S: CodecZenohInterestStorage> Default for CodecZenohInterestIn<S> {
    fn default() -> Self {
        Self {
            header: 0x19u8,
            id: Default::default(),
            header_current_or_future: Default::default(),
            extensions: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohInterestStorage> CodecZenohInterestIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>) -> Result<Self, CodecError> where S: CodecZenohInterestFrom<'f> {
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self) -> Result<(), CodecError> where S: CodecZenohInterestFrom<'f> {
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
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let id = cursor.read_vle_u64()?;
        _out.header_current_or_future = match ((header & 0x20u8) != 0, (header & 0x40u8) != 0) { (true, false) => Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body: Default::default() }), (false, true) => Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body: Default::default() }), (true, true) => Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body: Default::default() }), _ => None, };
        if let Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body: _p, .. }) | Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body: _p, .. }) | Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body: _p, .. }) = &mut _out.header_current_or_future {
            if (header & 0x20u8) != 0 || (header & 0x40u8) != 0 {
            ::sce_forge_runtime::codec::note_capacity(CodecZenohInterestBodyOwned::<S>::decode_into(cursor, _p), &mut _over)?;
        } else {
            unreachable!("the arm its gates installed holds the field they read");
        }
        }
        if (header & 0x80u8) != 0 {
            ::sce_forge_runtime::codec::note_capacity(_out.extensions.insert(Default::default()).refill(|_vec| {
                let mut _full = false;
                let mut _more = false;
                loop {
                    let mut _entry = CodecZenohExtEntryOwned::<S>::default();
                    ::sce_forge_runtime::codec::note_capacity(CodecZenohExtEntryOwned::<S>::decode_into(cursor, &mut _entry), &mut _full)?;
                    _more = _entry.z();
                    ::sce_forge_runtime::codec::push_keeping(_vec, _entry, &[], &mut _full);
                    if !_more { break; }
                }
                if _more && cursor.remaining() == 0 {
                    return Err(CodecError::NeedMoreBytes);
                }
                if _full {
                    return Err(CodecError::CapacityExceeded);
                }
                Ok(())
            }), &mut _over)?;
        } else {
            _out.extensions = None;
        }
        _out.header = header;
        _out.id = id;
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
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn set_mid(&mut self, v: u8) {
        self.header = (self.header & !0x1F) | (v & 0x1F);
    }

    pub fn current(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_current(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn future(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_future(&mut self, v: bool) {
        if v {
            self.header |= 0x40;
        } else {
            self.header &= !0x40;
        }
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }

    pub fn set_z(&mut self, v: bool) {
        if v {
            self.header |= 0x80;
        } else {
            self.header &= !0x80;
        }
    }


    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohInterestWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade. It reserves nothing, since the
    /// largest frame passes `u32` (docs/adr/0077 §7), and delegates to
    /// `encode` over a
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
        let mut _sce_v: Vec<u8> = Vec::new();
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}

/// The fields `header.CURRENT`, `header.FUTURE` decide (docs/adr/0065 §6 R4b, R4c): a value holds the
/// ones of one arm, and `CodecZenohInterest` derives the gates from which. A value holds no arm while `header.CURRENT` does not hold and `header.FUTURE` does not hold.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodecZenohInterestHeaderCurrentOrFuture<Body> {
    /// While `header.CURRENT` holds and `header.FUTURE` does not hold.
    Current {
        /// The `body` field.
        body: Body,
    },
    /// While `header.CURRENT` does not hold and `header.FUTURE` holds.
    Future {
        /// The `body` field.
        body: Body,
    },
    /// While `header.CURRENT` holds and `header.FUTURE` holds.
    CurrentFuture {
        /// The `body` field.
        body: Body,
    },
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohInterest` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohInterestWire::encode_source`]).
pub trait CodecZenohInterestSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `id` field.
    fn id(&self) -> u64;
    /// The `extensions` field.
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>>;
    /// The fields `header.CURRENT || header.FUTURE` decides, held together (docs/adr/0065 §6 R4).
    fn header_current_or_future(&self) -> Option<CodecZenohInterestHeaderCurrentOrFuture<&impl super::codec_zenoh_interest_body::CodecZenohInterestBodySource>>;
}

impl<T: CodecZenohInterestSource + ?Sized> CodecZenohInterestSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn id(&self) -> u64 {
        (**self).id()
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        (**self).extensions()
    }
    fn header_current_or_future(&self) -> Option<CodecZenohInterestHeaderCurrentOrFuture<&impl super::codec_zenoh_interest_body::CodecZenohInterestBodySource>> {
        (**self).header_current_or_future()
    }
}

/// A value of `CodecZenohInterest` held in the types a caller already has: a source
/// ([`CodecZenohInterestSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohInterestParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `id` field.
    pub id: u64,
    /// The `extensions` field.
    pub extensions: Option<::sce_forge_runtime::codec::Bounded<&'a [super::codec_zenoh_ext_entry::CodecZenohExtEntryParts<'a>], { usize::MAX }, 1>>,
    /// The fields `header.CURRENT || header.FUTURE` decides, held together (docs/adr/0065 §6 R4).
    pub header_current_or_future: Option<CodecZenohInterestHeaderCurrentOrFuture<&'a super::codec_zenoh_interest_body::CodecZenohInterestBodyParts<'a>>>,
}

impl<'a> CodecZenohInterestSource for CodecZenohInterestParts<'a> {
    fn header_current_or_future(&self) -> Option<CodecZenohInterestHeaderCurrentOrFuture<&impl super::codec_zenoh_interest_body::CodecZenohInterestBodySource>> {
        self.header_current_or_future.as_ref().map(|_g| match _g {
            CodecZenohInterestHeaderCurrentOrFuture::Current { body } => CodecZenohInterestHeaderCurrentOrFuture::Current { body: *body },
            CodecZenohInterestHeaderCurrentOrFuture::Future { body } => CodecZenohInterestHeaderCurrentOrFuture::Future { body: *body },
            CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body } => CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body: *body },
        })
    }
    fn header(&self) -> u8 {
        self.header
    }
    fn id(&self) -> u64 {
        self.id
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
}

impl<S: CodecZenohInterestStorage> CodecZenohInterestSource for CodecZenohInterestOwned<S> {
    fn header_current_or_future(&self) -> Option<CodecZenohInterestHeaderCurrentOrFuture<&impl super::codec_zenoh_interest_body::CodecZenohInterestBodySource>> {
        self.header_current_or_future.as_ref().map(|_g| match _g {
            CodecZenohInterestHeaderCurrentOrFuture::Current { body } => CodecZenohInterestHeaderCurrentOrFuture::Current { body },
            CodecZenohInterestHeaderCurrentOrFuture::Future { body } => CodecZenohInterestHeaderCurrentOrFuture::Future { body },
            CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body } => CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body },
        })
    }
    fn header(&self) -> u8 {
        self.header
    }
    fn id(&self) -> u64 {
        self.id
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        self.extensions.as_ref().map(|_l| _l.list_iter())
    }
}

impl<'a> CodecZenohInterestSource for CodecZenohInterestView<'a> {
    fn header_current_or_future(&self) -> Option<CodecZenohInterestHeaderCurrentOrFuture<&impl super::codec_zenoh_interest_body::CodecZenohInterestBodySource>> {
        self.header_current_or_future.as_ref().map(|_g| match _g {
            CodecZenohInterestHeaderCurrentOrFuture::Current { body } => CodecZenohInterestHeaderCurrentOrFuture::Current { body },
            CodecZenohInterestHeaderCurrentOrFuture::Future { body } => CodecZenohInterestHeaderCurrentOrFuture::Future { body },
            CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body } => CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body },
        })
    }
    fn header(&self) -> u8 {
        self.header
    }
    fn id(&self) -> u64 {
        self.id
    }
    fn extensions(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone, { usize::MAX }, 1>> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
}

/// The codec `CodecZenohInterest` itself: what writes a value read through
/// [`CodecZenohInterestSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohInterestWire;

#[allow(dead_code)]
impl CodecZenohInterestWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohInterest::encode` writes for the value it holds, read through
    /// [`CodecZenohInterestSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohInterestSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohInterestSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _m_body = src.header_current_or_future().map(|_g| match _g {
            CodecZenohInterestHeaderCurrentOrFuture::Current { body } | CodecZenohInterestHeaderCurrentOrFuture::Future { body } | CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body } => body,
        });
        let _k_extensions: usize = src.extensions().map_or(0, |_l| _l.len());
        let _d_header_current: bool = matches!(src.header_current_or_future(), Some(CodecZenohInterestHeaderCurrentOrFuture::Current { .. } | CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { .. }));
        let _d_header_future: bool = matches!(src.header_current_or_future(), Some(CodecZenohInterestHeaderCurrentOrFuture::Future { .. } | CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { .. }));
        let _p_header_z_0: bool = src.extensions().is_some();
        let _c_header: u8 = (((((src.header() & !0x20) | (u8::from(_d_header_current) << 5)) & !0x40) | (u8::from(_d_header_future) << 6)) & !0x80) | (u8::from(_p_header_z_0) << 7);
        w.write_u8(_c_header)?;
        w.write_vle_u64(src.id())?;
        if let Some(_v) = _m_body {
            super::codec_zenoh_interest_body::CodecZenohInterestBodyWire::write_source(_v, w)?;
        }
        if let Some(_v) = src.extensions() {
            for (_i, _e) in _v.enumerate() {
                super::codec_zenoh_ext_entry::CodecZenohExtEntryWire::write_source_in(&_e, w, &super::codec_zenoh_ext_entry::CodecZenohExtEntryOverrides::header_z(_i + 1 < _k_extensions))?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohInterestSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohInterestSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohInterestWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohInterestWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohInterestSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohInterestSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecZenohInterestView` is what a decode of `CodecZenohInterest` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecZenohInterest` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecZenohInterest::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
pub struct CodecZenohInterestExtensionsWalk;

impl ListWalk<0> for CodecZenohInterestExtensionsWalk {
    type Entry<'b> = CodecZenohExtEntryView<'b>;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 0]) -> Result<Self::Entry<'b>, CodecError> {
        CodecZenohExtEntryView::decode(_c)
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 0]) {  }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohInterestView<'a> {
    pub header: u8,
    pub id: u64,
    pub header_current_or_future: Option<CodecZenohInterestHeaderCurrentOrFuture<CodecZenohInterestBodyView<'a>>>,
    pub extensions: Option<Bounded<ListView<'a, CodecZenohInterestExtensionsWalk, 0>, { usize::MAX }, 1>>,
}

impl<'a> Default for CodecZenohInterestView<'a> {
    fn default() -> Self {
        Self {
            header: 0x19u8,
            id: Default::default(),
            header_current_or_future: Default::default(),
            extensions: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohInterestView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecZenohInterestIn::decode`], held as a view. On success the
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
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let id = cursor.read_vle_u64()?;
        let mut body: Option<CodecZenohInterestBodyView<'a>> = None;
        if (header & 0x20u8) != 0 || (header & 0x40u8) != 0 {
            CodecZenohInterestBodyView::decode_into(cursor, body.insert(CodecZenohInterestBodyView::default()))?;
        } else {
            body = None;
        }
        if (header & 0x80u8) != 0 {
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            let mut _more = false;
            loop {
                let _entry = CodecZenohExtEntryView::decode(cursor)?;
                _more = _entry.z();
                _count += 1;
                if !_more { break; }
            }
            if _more && cursor.remaining() == 0 {
                return Err(CodecError::NeedMoreBytes);
            }
            _out.extensions = Some(Bounded::new(ListView::new(_rest, _count, [])).ok_or(CodecError::TooManyElements)?);
        } else {
            _out.extensions = None;
        }
        _out.header = header;
        _out.id = id;
        _out.header_current_or_future = match ((header & 0x20u8) != 0, (header & 0x40u8) != 0) { (true, false) => Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body: body.unwrap_or_default() }), (false, true) => Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body: body.unwrap_or_default() }), (true, true) => Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body: body.unwrap_or_default() }), _ => None, };
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecZenohInterest`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder<D>(&self, _out: &mut CodecZenohInterestIn<::sce_forge_runtime::codec::Lent<'a, D>>) -> Result<(), CodecError> where D: CodecZenohInterestLentStorage {
        _out.header = self.header;
        _out.id = self.id;
        match &self.header_current_or_future {
            Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body }) => {
                let mut _o_body: CodecZenohInterestBodyOwned<::sce_forge_runtime::codec::Lent<'a, D>> = Default::default();
                body.into_builder(&mut _o_body)?;
                _out.header_current_or_future = Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body: _o_body });
            }
            Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body }) => {
                let mut _o_body: CodecZenohInterestBodyOwned<::sce_forge_runtime::codec::Lent<'a, D>> = Default::default();
                body.into_builder(&mut _o_body)?;
                _out.header_current_or_future = Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body: _o_body });
            }
            Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body }) => {
                let mut _o_body: CodecZenohInterestBodyOwned<::sce_forge_runtime::codec::Lent<'a, D>> = Default::default();
                body.into_builder(&mut _o_body)?;
                _out.header_current_or_future = Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body: _o_body });
            }
            None => _out.header_current_or_future = None,
        }
        match &self.extensions {
            Some(_l) => _out.extensions.insert(Default::default()).refill(|_vec| {
                    for _e in _l.iter() {
                        ::sce_forge_runtime::codec::SceList::try_push(_vec, Default::default())?;
                        if let Some(_slot) = ::sce_forge_runtime::codec::SceList::last_mut(_vec) {
                            _e.into_builder(_slot)?;
                        }
                    }
                    Ok(())
                })?,
            None => _out.extensions = None,
        }
        Ok(())
    }


    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecZenohInterest::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecZenohInterestWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::new();
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }

    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn current(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn future(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }

}

// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohInterest<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohInterestOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohInterestOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohInterestOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohInterestOwned` alone is the build's
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
// `let v: CodecZenohInterestOwned = CodecZenohInterestOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohInterest<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohInterestIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohInterestLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohInterestOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohInterestFrom<'a>>(self) -> Result<CodecZenohInterestOwned<S>, CodecError> {
        Ok(CodecZenohInterestOwned {
            header: self.header,
            id: self.id,
            header_current_or_future: match self.header_current_or_future { Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body }) => Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body: body.try_into_owned_in::<S>()? }), Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body }) => Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body: body.try_into_owned_in::<S>()? }), Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body }) => Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body: body.try_into_owned_in::<S>()? }), None => None },
            extensions: self.extensions.map(|_v| ::sce_forge_runtime::codec::try_collect_bounded(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
        })
    }

    // No projection at the build's default profile: on the heap-free tier
    // that is `Inline`, which holds no field whose protocol states no bound
    // (docs/adr/0077 §7). Name the profile, `try_into_owned_in::<S>()`.
}

#[allow(dead_code)]
impl<S: CodecZenohInterestStorage> CodecZenohInterestOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohInterestIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohInterestIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        _out.header = self.header;
        _out.id = self.id;
        match &self.header_current_or_future {
            Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body }) => {
                let mut _o_body: CodecZenohInterestBodyOwned<::sce_forge_runtime::codec::Lent<'o, S>> = Default::default();
                body.as_borrowed_into(&mut _o_body);
                _out.header_current_or_future = Some(CodecZenohInterestHeaderCurrentOrFuture::Current { body: _o_body });
            }
            Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body }) => {
                let mut _o_body: CodecZenohInterestBodyOwned<::sce_forge_runtime::codec::Lent<'o, S>> = Default::default();
                body.as_borrowed_into(&mut _o_body);
                _out.header_current_or_future = Some(CodecZenohInterestHeaderCurrentOrFuture::Future { body: _o_body });
            }
            Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body }) => {
                let mut _o_body: CodecZenohInterestBodyOwned<::sce_forge_runtime::codec::Lent<'o, S>> = Default::default();
                body.as_borrowed_into(&mut _o_body);
                _out.header_current_or_future = Some(CodecZenohInterestHeaderCurrentOrFuture::CurrentFuture { body: _o_body });
            }
            None => _out.header_current_or_future = None,
        }
        match &self.extensions {
            Some(_l) => _out.extensions.insert(Default::default()).refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, _l.inner(), |_e| Ok(_e.as_borrowed())))?,
            None => _out.extensions = None,
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
    pub fn transcode_in<D: for<'x> CodecZenohInterestFrom<'x>>(&self) -> Result<CodecZenohInterestOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
