#![doc = "SCE-MAP: codec_zenoh_init_body:42 :: _forge_body"]
// SCE-MAP: codec_zenoh_init_body:42 :: _forge_body

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
/// The field `zid` of `CodecZenohInitBody`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohInitBodyZidField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyZidField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16>;
}

/// The field `cookie` of `CodecZenohInitBody`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct CodecZenohInitBodyCookieField;

impl ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyCookieField> for ::sce_forge_runtime::codec::Inline {
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<128>;
}

/// What `CodecZenohInitBodyIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait CodecZenohInitBodyStorage: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyZidField> + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyCookieField> {}

impl<S: ::core::fmt::Debug + Clone + PartialEq + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyZidField> + ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyCookieField>> CodecZenohInitBodyStorage for S {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside CodecZenohInitBodyStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait CodecZenohInitBodyFrom<'f>: CodecZenohInitBodyStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohInitBodyZidField> + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohInitBodyCookieField> {}

impl<'f, S: CodecZenohInitBodyStorage + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohInitBodyZidField> + ::sce_forge_runtime::codec::StoresBytes<'f, CodecZenohInitBodyCookieField>> CodecZenohInitBodyFrom<'f> for S {}

// `CodecZenohInitBodyIn<S>` is the one value this codec has, at the storage
// profile `S` (docs/adr/0077 §7). `CodecZenohInitBody<'a>` is it on `Lent`, which
// borrows bytes and text from the frame, and `CodecZenohInitBodyOwned<S>` names it
// at a profile that holds them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohInitBodyIn<S: CodecZenohInitBodyStorage> {
    pub version: u8,
    pub cbyte: u8,
    pub zid: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyZidField>>::Bytes, 16, 1>,
    pub s: Option<CodecZenohInitBodyS>,
    pub cookie_len: Option<u64>,
    pub cookie: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<CodecZenohInitBodyCookieField>>::Bytes, 128>>,
}

/// The eager form: bytes and text borrowed from the frame, lists held inline.
#[allow(dead_code)]
pub type CodecZenohInitBody<'a> = CodecZenohInitBodyIn<::sce_forge_runtime::codec::Lent<'a>>;

/// The value at a profile that holds its bytes and text.
#[allow(dead_code)]
pub type CodecZenohInitBodyOwned<S = ::sce_forge_runtime::codec::DefaultStorage> = CodecZenohInitBodyIn<S>;

impl<S: CodecZenohInitBodyStorage> Default for CodecZenohInitBodyIn<S> {
    fn default() -> Self {
        Self {
            version: Default::default(),
            cbyte: Default::default(),
            zid: Default::default(),
            s: Default::default(),
            cookie_len: Default::default(),
            cookie: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<S: CodecZenohInitBodyStorage> CodecZenohInitBodyIn<S> {
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
    pub fn decode<'f>(cursor: &mut SceCursor<'f>, s: u8, a: u8) -> Result<Self, CodecError> where S: CodecZenohInitBodyFrom<'f> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out, s, a)?;
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
    pub fn decode_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, s: u8, a: u8) -> Result<(), CodecError> where S: CodecZenohInitBodyFrom<'f> {
        // A form past its capacity stopped where it could not store, so the
        // frame is judged by what cannot run out, and only a frame that parses
        // is one this node cannot hold (docs/adr/0077 §7). Any profile may hold
        // less than a bound, so this stands on every codec that takes one.
        let mut _frame = *cursor;
        match Self::store_into(cursor, _out, s, a) {
            Err(CodecError::CapacityExceeded) => {
                CodecZenohInitBody::store_into(&mut _frame, &mut CodecZenohInitBody::default(), s, a)?;
                Err(CodecError::CapacityExceeded)
            }
            stored => stored,
        }
    }

    /// The reading of [`Self::decode_into`], which stops where a field is past
    /// its capacity.
    fn store_into<'f>(cursor: &mut SceCursor<'f>, _out: &mut Self, s: u8, a: u8) -> Result<(), CodecError> where S: CodecZenohInitBodyFrom<'f> {
        // Declared-but-unconsumed flag inputs: defensive suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly. The validator enforces
        // declaration; consumption is a per-codec design choice.
        let _ = s;
        let _ = a;
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
        let zid = zid.store::<S, CodecZenohInitBodyZidField>()?;
        let sn_res = if (s & 0x01u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let batch_size = if (s & 0x01u8) != 0 {
            let raw = cursor.peek_slice(2)?;
            let _v = raw[0] as u16 | ((raw[1] as u16) << 8);
            cursor.advance(2)?;
            Some(_v)
        } else {
            None
        };
        let cookie_len = if (a & 0x01u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let cookie = if (a & 0x01u8) != 0 {
            let _len = cookie_len.unwrap();
            if _len > 128 {
                return Err(CodecError::LengthOutOfRange);
            }
            let _n = _len as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        let cookie = cookie.map(|_v| ::sce_forge_runtime::codec::Bounded::new(_v).ok_or(CodecError::LengthOutOfRange)).transpose()?;
        let cookie = cookie.map(|_v| _v.store::<S, CodecZenohInitBodyCookieField>()).transpose()?;
        _out.version = version;
        _out.cbyte = cbyte;
        _out.zid = zid;
        _out.s = if (s & 0x01u8) != 0 { Some(CodecZenohInitBodyS { sn_res: sn_res.unwrap_or_default(), batch_size: batch_size.unwrap_or_default() }) } else { None };
        _out.cookie_len = cookie_len;
        _out.cookie = cookie;
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 159;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        CodecZenohInitBodyWire::encode_source(self, w)
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

/// The fields `S` decide (docs/adr/0065 §6 R4b), present together
/// while `S` holds: a value holds all of them or none, and `CodecZenohInitBody`
/// derives the gate from which.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CodecZenohInitBodyS {
    /// The `sn_res` field.
    pub sn_res: u8,
    /// The `batch_size` field.
    pub batch_size: u16,
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohInitBody` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohInitBody`
/// computes those from what is ([`CodecZenohInitBody::encode_source`]).
pub trait CodecZenohInitBodySource {
    /// The `version` field.
    fn version(&self) -> u8;
    /// The `cbyte` field.
    fn cbyte(&self) -> u8;
    /// The `zid` field.
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1>;
    /// The `cookie` field.
    fn cookie(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 128>>;
    /// The fields `S` decides, held together (docs/adr/0065 §6 R4).
    fn s(&self) -> Option<CodecZenohInitBodyS>;
}

impl<T: CodecZenohInitBodySource + ?Sized> CodecZenohInitBodySource for &T {
    fn version(&self) -> u8 {
        (**self).version()
    }
    fn cbyte(&self) -> u8 {
        (**self).cbyte()
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        (**self).zid()
    }
    fn cookie(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 128>> {
        (**self).cookie()
    }
    fn s(&self) -> Option<CodecZenohInitBodyS> {
        (**self).s()
    }
}

/// A value of `CodecZenohInitBody` held in the types a caller already has: a source
/// ([`CodecZenohInitBodySource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohInitBodyParts<'a> {
    /// The `version` field.
    pub version: u8,
    /// The `cbyte` field.
    pub cbyte: u8,
    /// The `zid` field.
    pub zid: ::sce_forge_runtime::codec::Bytes<'a, 16, 1>,
    /// The `cookie` field.
    pub cookie: Option<::sce_forge_runtime::codec::Bytes<'a, 128>>,
    /// The fields `S` decides, held together (docs/adr/0065 §6 R4).
    pub s: Option<CodecZenohInitBodyS>,
}

impl<'a> CodecZenohInitBodySource for CodecZenohInitBodyParts<'a> {
    fn s(&self) -> Option<CodecZenohInitBodyS> {
        self.s
    }
    fn version(&self) -> u8 {
        self.version
    }
    fn cbyte(&self) -> u8 {
        self.cbyte
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        self.zid
    }
    fn cookie(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 128>> {
        self.cookie
    }
}

impl<S: CodecZenohInitBodyStorage> CodecZenohInitBodySource for CodecZenohInitBodyOwned<S> {
    fn s(&self) -> Option<CodecZenohInitBodyS> {
        self.s
    }
    fn version(&self) -> u8 {
        self.version
    }
    fn cbyte(&self) -> u8 {
        self.cbyte
    }
    fn zid(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 1> {
        self.zid.as_bytes()
    }
    fn cookie(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 128>> {
        self.cookie.as_ref().map(::sce_forge_runtime::codec::Bounded::as_bytes)
    }
}

/// The codec `CodecZenohInitBody` itself: what writes a value read through
/// [`CodecZenohInitBodySource`], holding no value and naming no storage profile. Every
/// form of the value, a parent and a view write through it, so a profile
/// that cannot hold some field is never asked for here (docs/adr/0077 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecZenohInitBodyWire;

#[allow(dead_code)]
impl CodecZenohInitBodyWire {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// `CodecZenohInitBody::encode` writes for the value it holds, read through
    /// [`CodecZenohInitBodySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecZenohInitBodySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The flag-input `S` that `src` says: set exactly when the value holds the side of `S` present while it holds.
    /// A parent hands the decode this bit and derives it from here for the
    /// encode.
    pub fn input_s<S: CodecZenohInitBodySource + ?Sized>(src: &S) -> bool {
        CodecZenohInitBodySource::s(src).is_some()
    }

    /// The flag-input `A` that `src` says: set exactly when `cookie` is present.
    /// A parent hands the decode this bit and derives it from here for the
    /// encode.
    pub fn input_a<S: CodecZenohInitBodySource + ?Sized>(src: &S) -> bool {
        CodecZenohInitBodySource::cookie(src).is_some()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohInitBodySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let (_m_sn_res, _m_batch_size) = match src.s() {
            Some(CodecZenohInitBodyS { sn_res, batch_size }) => (Some(sn_res), Some(batch_size)),
            None => (None, None),
        };
        let a: u8 = u8::from(CodecZenohInitBodySource::cookie(src).is_some());
        let _n_d_cbyte_zid_len_m1_0: usize = src.zid().len() - 1;
        let _n_d_cookie_len_0: usize = match src.cookie() {
            Some(_v) => _v.len(),
            None => 0,
        };
        let _f_d_cookie_len: Option<usize> = if src.cookie().is_some() { Some(_n_d_cookie_len_0) } else { None };
        let _d_cookie_len: u64 = ::sce_forge_runtime::codec::widen_usize(_f_d_cookie_len.unwrap_or(0));
        let _v_cbyte_zid_len_m1: u8 = _n_d_cbyte_zid_len_m1_0 as u8;
        let _c_cbyte: u8 = (src.cbyte() & !0xF0) | (_v_cbyte_zid_len_m1 << 4);
        w.write_u8(src.version())?;
        w.write_u8(_c_cbyte)?;
        w.write_bytes(src.zid().into_inner())?;
        if let Some(_v) = _m_sn_res {
            w.write_u8(_v)?;
        }
        if let Some(_v) = _m_batch_size {
            w.write_u8((_v & 0xFF) as u8)?;
            w.write_u8((_v >> 8 & 0xFF) as u8)?;
        }
        if (a & 0x1) != 0 {
            w.write_vle_u64(_d_cookie_len)?;
        }
        if let Some(_v) = src.cookie() {
            w.write_bytes(_v.into_inner())?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohInitBodySource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecZenohInitBodySource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohInitBodyWire {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecZenohInitBodyWire {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecZenohInitBodySource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohInitBodySource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohInitBody<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohInitBodyOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohInitBodyOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohInitBodyOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohInitBodyOwned` alone is the build's
// default profile. Because the non-allocating profile is a *type*, this mirror
// carries no `alloc` gate at all — a list- or embed-bearing codec has an owned
// form on the heap-free tier too, and a heap-capable consumer can still pin a
// value to storage that is guaranteed not to allocate.
//
// `try_into_owned` is the fallible direction (one `?` per profile: on the
// growable profile the copy cannot fail, on the inline profile it enforces
// each declared bound); `as_borrowed` re-borrows any profile back
// into the single borrowed view that owns `encode`; `transcode_in` moves a
// value between profiles as a checked projection rather than a re-decode.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling one instead names it on the value or its binding —
// `let v: CodecZenohInitBodyOwned = CodecZenohInitBodyOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's container infer, so no call
// site repeats a capacity. A capacity is not in this file at all except as
// what `Inline` holds: a deployment answers for the same tags from its own.

// The lent value at any profile `D` that holds its lists: what a decode of a
// frame gives where bytes and text borrow it, `CodecZenohInitBody<'a>` being it
// at `Inline` (docs/adr/0077 §7).
#[allow(dead_code)]
impl<'a, D> CodecZenohInitBodyIn<::sce_forge_runtime::codec::Lent<'a, D>> where D: CodecZenohInitBodyStorage {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohInitBodyOwned`] held in the given storage profile. Call at
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
    pub fn try_into_owned_in<S: CodecZenohInitBodyFrom<'a>>(self) -> Result<CodecZenohInitBodyOwned<S>, CodecError> {
        Ok(CodecZenohInitBodyOwned {
            version: self.version,
            cbyte: self.cbyte,
            zid: self.zid.store::<S, CodecZenohInitBodyZidField>()?,
            s: self.s,
            cookie_len: self.cookie_len,
            cookie: self.cookie.map(|_v| _v.store::<S, CodecZenohInitBodyCookieField>()).transpose()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohInitBodyOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: CodecZenohInitBodyStorage> CodecZenohInitBodyOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohInitBodyIn<::sce_forge_runtime::codec::Lent<'_, S>> {
        let mut _out = Default::default();
        self.as_borrowed_into(&mut _out);
        _out
    }

    /// [`Self::as_borrowed`] into a destination the caller owns, each
    /// bounded list and embedded body projected in the field where it sits.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through (the optional wrapper, the borrowed struct, the
    /// variant, the caller's binding), however few entries the list holds. A
    /// caller on a path where that shows projects into a destination it owns,
    /// then encodes from it. On `Err`, `_out` holds what was projected so far
    /// and is not to be read.
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohInitBodyIn<::sce_forge_runtime::codec::Lent<'o, S>>) {
        _out.version = self.version;
        _out.cbyte = self.cbyte;
        _out.zid = self.zid.as_bytes();
        _out.s = self.s;
        _out.cookie_len = self.cookie_len;
        _out.cookie = self.cookie.as_ref().map(::sce_forge_runtime::codec::Bounded::as_bytes);
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: for<'x> CodecZenohInitBodyFrom<'x>>(&self) -> Result<CodecZenohInitBodyOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}
