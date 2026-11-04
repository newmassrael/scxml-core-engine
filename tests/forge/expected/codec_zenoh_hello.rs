#![doc = "SCE-MAP: codec_zenoh_hello:41 :: _forge_body"]
// SCE-MAP: codec_zenoh_hello:41 :: _forge_body

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
use super::codec_zenoh_locator::CodecZenohLocatorView;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
use super::codec_zenoh_locator::CodecZenohLocatorOwned;
/// The field `zid` of `CodecZenohHello`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohHelloZidField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecZenohHelloZidField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16>;
}

/// The field `locators` of `CodecZenohHello`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohHelloLocatorsField;

// `Inline` does not answer for `locators`: the protocol states no bound of
// it, so it is held only by a profile whose container grows, or by a deployment
// that gives it a capacity (docs/adr/0077 §7).

/// What `CodecZenohHelloIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohHelloStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohHelloZidField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohHelloLocatorsField> + super::codec_zenoh_locator::CodecZenohLocatorStorage {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohHelloZidField> + ::sce_forge_runtime::codec::HoldsList<CodecZenohHelloLocatorsField> + super::codec_zenoh_locator::CodecZenohLocatorStorage> CodecZenohHelloStorage for S {}

/// What `CodecZenohHelloIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `CodecZenohHello` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait CodecZenohHelloLentStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohHelloLocatorsField> + super::codec_zenoh_locator::CodecZenohLocatorLentStorage {}

impl<D: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsList<CodecZenohHelloLocatorsField> + super::codec_zenoh_locator::CodecZenohLocatorLentStorage> CodecZenohHelloLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohHelloStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohHelloFrom<'f>: CodecZenohHelloStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohHelloZidField> + super::codec_zenoh_locator::CodecZenohLocatorFrom<'f> {}

impl<'f, S: CodecZenohHelloStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohHelloZidField> + super::codec_zenoh_locator::CodecZenohLocatorFrom<'f>> CodecZenohHelloFrom<'f> for S {}

// `CodecZenohHelloIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohHello<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohHelloOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohHelloIn<S: CodecZenohHelloStorage> {
    pub version: u8,
    pub cbyte: u8,
    pub zid: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecZenohHelloZidField>>::Bytes, 16, 1>,
    pub num_locators: Option<u64>,
    pub locators: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsList<CodecZenohHelloLocatorsField>>::List<CodecZenohLocatorOwned<S>>, { usize::MAX }>>,
}

// No eager form at the default profile: `Lent<'a>` holds each list as `Inline`
// does, and `Inline` holds no list whose protocol states no bound
// (docs/adr/0077 §7). Name the eager form at a profile that holds its lists,
// `CodecZenohHelloIn<Lent<'a, D>>`.

/// The value at a profile that holds its bytes and text, named
/// with the profile: the build's default on the heap-free tier is `Inline`, which
/// holds no field whose protocol states no bound (docs/adr/0077 §7).
#[allow(dead_code)]
pub type CodecZenohHelloOwned<S> = CodecZenohHelloIn<S>;

impl<S: CodecZenohHelloStorage> Default for CodecZenohHelloIn<S> {
    fn default() -> Self {
        Self {
            version: Default::default(),
            cbyte: Default::default(),
            zid: Default::default(),
            num_locators: Default::default(),
            locators: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohHelloStorage> CodecZenohHelloIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>, l: u8) -> Result<Self, CodecError> where S: CodecZenohHelloFrom<'f> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, l)?;
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, l: u8) -> Result<(), CodecError> where S: CodecZenohHelloFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let _frame = *cursor;
        match Self::store_into(cursor, _out, l) {
            Err(CodecError::CapacityExceeded) => Err(Self::past_capacity(_frame, l)),
            stored => stored,
        }
    }

    /// What a frame is that stopped the decode past a capacity (docs/adr/0077
    /// §7): past the capacity when a form that cannot run out reads it, and
    /// what that form refuses it as when it does not. Kept out of the decode
    /// and never inlined into it, so the value the form reads takes no room in
    /// the decode's frame; it runs only after the decode has stopped.
    #[cold]
    #[inline(never)]
    fn past_capacity<'f>(mut _frame: SceCursor<'f>, l: u8) -> CodecError where S: CodecZenohHelloFrom<'f> {
        match CodecZenohHelloView::decode(&mut _frame, l) {
            Ok(_) => CodecError::CapacityExceeded,
            Err(refused) => refused,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, l: u8) -> Result<(), CodecError> where S: CodecZenohHelloFrom<'f> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = l;
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
        let cbyte = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let zid = {
            let _len = (cbyte >> 4) & 0xF;
            let _n = (_len as usize) + 1;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            _v
        };
        let zid = ::sce_forge_runtime::codec::Bounded::new(zid).ok_or(CodecError::LengthOutOfRange)?;
        let zid = zid.store::<S, CodecZenohHelloZidField>()?;
        let num_locators = if (l & 0x01u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        if (l & 0x01u8) != 0 {
            let _n = num_locators.expect("co-gating: count present-if matches repeat");
            _out.locators.insert(Default::default()).refill(|_vec| {
                for _ in 0.._n {
                    ::sce_forge_runtime::codec::SceList::try_push(_vec, CodecZenohLocatorOwned::<S>::decode(cursor)?)?;
                }
                Ok(())
            })?;
        } else {
            _out.locators = None;
        }
        _out.version = version;
        _out.cbyte = cbyte;
        _out.zid = zid;
        _out.num_locators = num_locators;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn whatami(&self) -> u8 {
        self.cbyte & 0x03
    }

    pub fn set_whatami(&mut self, v: u8) {
        self.cbyte = (self.cbyte & !0x03) | (v & 0x03);
    }

    pub fn zid_len_m1(&self) -> u8 {
        (self.cbyte >> 4) & 0x0F
    }

    pub fn set_zid_len_m1(&mut self, v: u8) {
        self.cbyte = (self.cbyte & !0xF0) | ((v & 0x0F) << 4);
    }


    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohHelloWire::encode_source(self, w)
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

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohHello` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; the
/// encode computes those from what is ([`CodecZenohHelloWire::encode_source`]).
pub trait CodecZenohHelloSource {
    /// The `version` field.
    fn version(&self) -> u8;
    /// The `cbyte` field.
    fn cbyte(&self) -> u8;
    /// The `zid` field.
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1>;
    /// The `locators` field.
    fn locators(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone, { usize::MAX }>>;
}

impl<T: CodecZenohHelloSource + ?Sized> CodecZenohHelloSource for &T {
    fn version(&self) -> u8 {
        (**self).version()
    }
    fn cbyte(&self) -> u8 {
        (**self).cbyte()
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        (**self).zid()
    }
    fn locators(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone, { usize::MAX }>> {
        (**self).locators()
    }
}

/// A value of `CodecZenohHello` held in the types a caller already has: a source
/// ([`CodecZenohHelloSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohHelloParts<'a> {
    /// The `version` field.
    pub version: u8,
    /// The `cbyte` field.
    pub cbyte: u8,
    /// The `zid` field.
    pub zid: ::sce_forge_runtime::codec::Bytes<'a, 16, 1>,
    /// The `locators` field.
    pub locators: Option<::sce_forge_runtime::codec::Bounded<&'a [super::codec_zenoh_locator::CodecZenohLocatorParts<'a>], { usize::MAX }>>,
}

impl<'a> CodecZenohHelloSource for CodecZenohHelloParts<'a> {
    fn version(&self) -> u8 {
        self.version
    }
    fn cbyte(&self) -> u8 {
        self.cbyte
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        self.zid
    }
    fn locators(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone, { usize::MAX }>> {
        self.locators.as_ref().map(|_l| _l.iter())
    }
}

impl<S: CodecZenohHelloStorage> CodecZenohHelloSource for CodecZenohHelloOwned<S> {
    fn version(&self) -> u8 {
        self.version
    }
    fn cbyte(&self) -> u8 {
        self.cbyte
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        self.zid.as_bytes()
    }
    fn locators(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone, { usize::MAX }>> {
        self.locators.as_ref().map(|_l| _l.list_iter())
    }
}

impl<'a> CodecZenohHelloSource for CodecZenohHelloView<'a> {
    fn version(&self) -> u8 {
        self.version
    }
    fn cbyte(&self) -> u8 {
        self.cbyte
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        self.zid
    }
    fn locators(&self) -> Option<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone, { usize::MAX }>> {
        self.locators.as_ref().map(|_l| _l.iter())
    }
}

/// The codec `CodecZenohHello` itself: what writes a value read through
/// [`CodecZenohHelloSource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohHelloWire;

#[allow(dead_code)]
impl CodecZenohHelloWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohHello::encode` writes for the value it holds, read through
    /// [`CodecZenohHelloSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohHelloSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The flag-input `L` that `src` says: set exactly when `locators` is present.
    /// A parent hands the decode this bit and derives it from here for the
    /// encode.
    pub fn input_l<S: CodecZenohHelloSource + ?Sized>(src: &S) -> bool {
        CodecZenohHelloSource::locators(src).is_some()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohHelloSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let l: u8 = u8::from(CodecZenohHelloSource::locators(src).is_some());
        let _k_locators: usize = src.locators().map_or(0, |_l| _l.len());
        let _n_d_cbyte_zid_len_m1_0: usize = src.zid().len() - 1;
        let _f_d_num_locators: Option<usize> = if src.locators().is_some() { Some(_k_locators) } else { None };
        let _d_num_locators: u64 = ::sce_forge_runtime::codec::widen_usize(_f_d_num_locators.unwrap_or(0));
        let _v_cbyte_zid_len_m1: u8 = _n_d_cbyte_zid_len_m1_0 as u8;
        let _c_cbyte: u8 = (src.cbyte() & !0xF0) | (_v_cbyte_zid_len_m1 << 4);
        w.write_u8(src.version())?;
        w.write_u8(_c_cbyte)?;
        w.write_bytes(src.zid().into_inner())?;
        if (l & 0x1) != 0 {
            w.write_vle_u64(_d_num_locators)?;
        }
        if let Some(_v) = src.locators() {
            for _e in _v {
                super::codec_zenoh_locator::CodecZenohLocatorWire::write_source(&_e, w)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohHelloSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohHelloSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohHelloWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohHelloWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohHelloSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohHelloSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecZenohHelloView` is what a decode of `CodecZenohHello` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecZenohHello` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecZenohHello::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
pub struct CodecZenohHelloLocatorsWalk;

impl ListWalk<0> for CodecZenohHelloLocatorsWalk {
    type Entry<'b> = CodecZenohLocatorView<'b>;

    fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; 0]) -> Result<Self::Entry<'b>, CodecError> {
        CodecZenohLocatorView::decode(_c)
    }

    fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; 0]) {  }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohHelloView<'a> {
    pub version: u8,
    pub cbyte: u8,
    pub zid: ::sce_forge_runtime::codec::Bytes<'a, 16, 1>,
    pub num_locators: Option<u64>,
    pub locators: Option<Bounded<ListView<'a, CodecZenohHelloLocatorsWalk, 0>, { usize::MAX }>>,
}

#[allow(dead_code)]
impl<'a> CodecZenohHelloView<'a> {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecZenohHelloIn::decode`], held as a view. On success the
    /// cursor advances past the consumed bytes.
    pub fn decode(cursor: &mut SceCursor<'a>, l: u8) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, l)?;
        Ok(_out)
    }

    /// [`Self::decode`] into a destination the caller owns, so a view that
    /// embeds or dispatches to this one is filled where it will live. On
    /// `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self, l: u8) -> Result<(), CodecError> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = l;
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
        let cbyte = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let zid = {
            let _len = (cbyte >> 4) & 0xF;
            let _n = (_len as usize) + 1;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            _v
        };
        let zid = ::sce_forge_runtime::codec::Bounded::new(zid).ok_or(CodecError::LengthOutOfRange)?;
        let num_locators = if (l & 0x01u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        if (l & 0x01u8) != 0 {
            let _n = num_locators.expect("co-gating: count present-if matches repeat");
            let _rest = cursor.peek_slice(cursor.remaining())?;
            let mut _count: u32 = 0;
            for _ in 0.._n {
                CodecZenohLocatorView::decode(cursor)?;
                _count += 1;
            }
            _out.locators = Some(Bounded::new(ListView::new(_rest, _count, [])).ok_or(CodecError::TooManyElements)?);
        } else {
            _out.locators = None;
        }
        _out.version = version;
        _out.cbyte = cbyte;
        _out.zid = zid;
        _out.num_locators = num_locators;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecZenohHello`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder<D>(&self, _out: &mut CodecZenohHelloIn<::sce_forge_runtime::codec::Lent<'a, D>>) -> Result<(), CodecError> where D: CodecZenohHelloLentStorage {
        _out.version = self.version;
        _out.cbyte = self.cbyte;
        _out.zid = self.zid;
        _out.num_locators = self.num_locators;
        match &self.locators {
            Some(_l) => _out.locators.insert(Default::default()).refill(|_vec| {
                    for _e in _l.iter() {
                        ::sce_forge_runtime::codec::SceList::try_push(_vec, Default::default())?;
                        if let Some(_slot) = ::sce_forge_runtime::codec::SceList::last_mut(_vec) {
                            _e.into_builder(_slot)?;
                        }
                    }
                    Ok(())
                })?,
            None => _out.locators = None,
        }
        Ok(())
    }


    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecZenohHello::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecZenohHelloWire::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::new();
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }

    pub fn whatami(&self) -> u8 {
        self.cbyte & 0x03
    }

    pub fn zid_len_m1(&self) -> u8 {
        (self.cbyte >> 4) & 0x0F
    }

}

// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohHello<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohHelloOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohHelloOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohHelloOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohHelloOwned` alone is the build's
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
// `let v: CodecZenohHelloOwned = CodecZenohHelloOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohHello<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohHelloIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohHelloLentStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohHelloOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohHelloFrom<'a>>(self) -> Result<CodecZenohHelloOwned<S>, CodecError> {
        Ok(CodecZenohHelloOwned {
            version: self.version,
            cbyte: self.cbyte,
            zid: self.zid.store::<S, CodecZenohHelloZidField>()?,
            num_locators: self.num_locators,
            locators: self.locators.map(|_v| ::sce_forge_runtime::codec::try_collect_bounded(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
        })
    }

    // No projection at the build's default profile: on the heap-free tier
    // that is `Inline`, which holds no field whose protocol states no bound
    // (docs/adr/0077 §7). Name the profile, `try_into_owned_in::<S>()`.
}

#[allow(dead_code)]
impl<S: CodecZenohHelloStorage> CodecZenohHelloOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohHelloIn<::sce_forge_runtime::codec::Lent<'_, S>>, CodecError> {
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohHelloIn<::sce_forge_runtime::codec::Lent<'o, S>>) -> Result<(), CodecError> {
        _out.version = self.version;
        _out.cbyte = self.cbyte;
        _out.zid = self.zid.as_bytes();
        _out.num_locators = self.num_locators;
        match &self.locators {
            Some(_l) => _out.locators.insert(Default::default()).refill(|_d| ::sce_forge_runtime::codec::try_project_bounded_into(_d, _l.inner(), |_e| Ok(_e.as_borrowed())))?,
            None => _out.locators = None,
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
    pub fn transcode_in<D: for<'x> CodecZenohHelloFrom<'x>>(&self) -> Result<CodecZenohHelloOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}
