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
// RFC §synth-5-B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_zenoh_locator::CodecZenohLocator;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohHello<'a> {
    pub version: u8,
    pub cbyte: u8,
    pub zid: &'a [u8],
    pub num_locators: Option<u64>,
    pub locators: Option<HeaplessVec<CodecZenohLocator<'a>, 64>>,
}

#[allow(dead_code)]
impl<'a> CodecZenohHello<'a> {
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
    pub fn decode(cursor: &mut SceCursor<'a>, l: u8) -> Result<Self, CodecError> {
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
            let _n = (((cbyte >> 4) & 0xF) as usize).wrapping_add(1);
            let raw = cursor.peek_slice(_n)?;
            let _v = raw;
            cursor.advance(_n)?;
            _v
        };
        let num_locators = if (l & 0x01u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        if (l & 0x01u8) != 0 {
            let _n = num_locators.expect("co-gating: count present-if matches repeat");
            let _vec = _out.locators.insert(HeaplessVec::<CodecZenohLocator<'a>, 64>::new());
            for _ in 0.._n {
                _vec.push(CodecZenohLocator::decode(cursor)?)
                    .map_err(|_| CodecError::TooManyElements)?;
            }
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 8795;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S, l: u8) -> Result<(), CodecError> {
        Self::encode_source(self, w, l)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector, or the
    /// `CodecError::PresentIfMismatch` `encode` refuses a message with
    /// when a field's presence disagrees with the `sce:present-if`
    /// that gates it. Callers targeting zero-alloc hot paths should
    /// call `encode` directly against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `backends/rust/forge-runtime/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self, l: u8) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink, l)?;
        Ok(_sce_v)
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohHello` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohHello`
/// computes those from what is ([`CodecZenohHello::encode_source`]).
pub trait CodecZenohHelloSource {
    /// The `version` field.
    fn version(&self) -> u8;
    /// The `cbyte` field.
    fn cbyte(&self) -> u8;
    /// The `zid` field.
    fn zid(&self) -> &[u8];
    /// The `locators` field.
    fn locators(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone>;
}

impl<T: CodecZenohHelloSource + ?Sized> CodecZenohHelloSource for &T {
    fn version(&self) -> u8 {
        (**self).version()
    }
    fn cbyte(&self) -> u8 {
        (**self).cbyte()
    }
    fn zid(&self) -> &[u8] {
        (**self).zid()
    }
    fn locators(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone> {
        (**self).locators()
    }
}

impl<'a> CodecZenohHelloSource for CodecZenohHello<'a> {
    fn version(&self) -> u8 {
        self.version
    }
    fn cbyte(&self) -> u8 {
        self.cbyte
    }
    fn zid(&self) -> &[u8] {
        self.zid
    }
    fn locators(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone> {
        self.locators.as_ref().map(|_l| _l.iter())
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohHelloSource for CodecZenohHelloOwned<S> {
    fn version(&self) -> u8 {
        self.version
    }
    fn cbyte(&self) -> u8 {
        self.cbyte
    }
    fn zid(&self) -> &[u8] {
        ::sce_forge_runtime::codec::SceByteBuf::as_slice(&self.zid)
    }
    fn locators(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_locator::CodecZenohLocatorSource> + Clone> {
        self.locators.as_ref().map(|_l| ::sce_forge_runtime::codec::SceList::as_slice(_l).iter())
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohHello<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohHelloSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecZenohHelloSource + ?Sized, K: SceSink>(src: &S, w: &mut K, l: u8) -> Result<(), CodecError> {
        Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new(), l)?;
        Self::write_source(src, w, l)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohHelloSource + ?Sized, K: SceSink>(src: &S, w: &mut K, l: u8) -> Result<(), CodecError> {
        let _k_locators: usize = src.locators().map_or(0, |_l| _l.count());
        if _k_locators > 64 {
            return Err(CodecError::TooManyElements);
        }
        let _n_d_cbyte_zid_len_m1_0: usize = src.zid().len().checked_sub(1).ok_or(CodecError::LengthOutOfRange)?;
        let _f_d_num_locators: Option<usize> = if src.locators().is_some() { Some(_k_locators) } else { None };
        let _d_num_locators: u64 = u64::try_from(_f_d_num_locators.unwrap_or(0)).map_err(|_| CodecError::LengthOutOfRange)?;
        if _n_d_cbyte_zid_len_m1_0 > 0xF {
            return Err(CodecError::LengthOutOfRange);
        }
        let _v_cbyte_zid_len_m1: u8 = u8::try_from(_n_d_cbyte_zid_len_m1_0).map_err(|_| CodecError::LengthOutOfRange)?;
        let _c_cbyte: u8 = (src.cbyte() & !0xF0) | (_v_cbyte_zid_len_m1 << 4);
        if ((l & 0x1) != 0) != src.locators().is_some() {
            return Err(CodecError::PresentIfMismatch);
        }
        let _ = l;
        w.write_u8(src.version())?;
        w.write_u8(_c_cbyte)?;
        w.write_bytes(src.zid())?;
        if (l & 0x1) != 0 {
            w.write_vle_u64(_d_num_locators)?;
        }
        if let Some(_v) = src.locators() {
            for _e in _v {
                CodecZenohLocator::write_source(&_e, w)?;
            }
        }
        Ok(())
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
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_locator::CodecZenohLocatorOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohHelloOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub version: u8,
    pub cbyte: u8,
    pub zid: S::Bytes<16>,
    pub num_locators: Option<u64>,
    pub locators: Option<S::List<CodecZenohLocatorOwned<S>, 64>>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohHelloOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn whatami(&self) -> u8 {
        self.cbyte & 0x03
    }

    pub fn zid_len_m1(&self) -> u8 {
        (self.cbyte >> 4) & 0x0F
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohHello<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohHelloOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohHelloOwned<S>, CodecError> {
        Ok(CodecZenohHelloOwned {
            version: self.version,
            cbyte: self.cbyte,
            zid: <S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice(self.zid)?,
            num_locators: self.num_locators,
            locators: self.locators.map(|_v| ::sce_forge_runtime::codec::try_collect_list(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohHelloOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohHelloOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohHello<'_>, CodecError> {
        let mut _out = CodecZenohHello::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohHello<'o>) -> Result<(), CodecError> {
        _out.version = self.version;
        _out.cbyte = self.cbyte;
        _out.zid = ::sce_forge_runtime::codec::SceByteBuf::as_slice(&self.zid);
        _out.num_locators = self.num_locators;
        match &self.locators {
            Some(_l) => ::sce_forge_runtime::codec::try_project_bounded_into(_out.locators.insert(Default::default()), _l, |_e| Ok(_e.as_borrowed()))?,
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
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohHelloOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohHelloOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `try_as_borrowed` followed by
    /// [`CodecZenohHello::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here. The same
    /// refusals as the projection's: a list longer than the borrowed view
    /// declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K, l: u8) -> Result<(), CodecError> {
        CodecZenohHello::encode_source(self, w, l)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, and so is a
    /// presence the chain predicates disagree with.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self, l: u8) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecZenohHello::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink, l)?;
        Ok(_sce_v)
    }
}
