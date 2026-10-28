#![doc = "SCE-MAP: codec_zenoh_response:75 :: _forge_body"]
// SCE-MAP: codec_zenoh_response:75 :: _forge_body

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

use super::codec_zenoh_ext_entry::CodecZenohExtEntry;
use super::codec_zenoh_reply::CodecZenohReply;
use super::codec_zenoh_err::CodecZenohErr;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohResponseVariant<'a> {
    CodecZenohReply(CodecZenohReply<'a>),
    CodecZenohErr(CodecZenohErr<'a>),
    Default {
        tag: u8,
        body: CodecZenohReply<'a>,
    },
}

impl<'a> Default for CodecZenohResponseVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohReply(CodecZenohReply::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohResponse<'a> {
    pub header: u8,
    pub request_id: u64,
    pub key_id: u32,
    pub suffix_len: Option<u64>,
    pub suffix: Option<&'a str>,
    pub extensions: Option<HeaplessVec<CodecZenohExtEntry<'a>, 4>>,
    pub body: CodecZenohResponseVariant<'a>,
}

// RFC variant-default-uniformity: at least one field's
// `<sce:flags>` carrier declares a wire-MID constant via
// `<sce:flag value="N"/>`. Manual `impl Default` bakes the OR of
// every declared `(value & mask) << bit` into that carrier so a
// freshly-constructed instance carries the wire-MID for its own
// dispatch tag. Fields without declared values fall through to
// `Default::default()` (preserving derive(Default) semantics).
impl<'a> Default for CodecZenohResponse<'a> {
    fn default() -> Self {
        Self {
            header: 0x1bu8,
            request_id: Default::default(),
            key_id: Default::default(),
            suffix_len: Default::default(),
            suffix: Default::default(),
            extensions: Default::default(),
            body: CodecZenohResponseVariant::default(),
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohResponse<'a> {
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
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
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
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self) -> Result<(), CodecError> {
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix decode (variable-length fields supported via
        // per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        // mode additionally peeks the cursor's next byte for the variant
        // tag without advancing — the arm body decoder reads the peeked
        // byte as its own header byte (Zenoh response/request body MID
        // dispatch shape per network.c:347-364 + 220-235).
        let header = {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            _v
        };
        let request_id = cursor.read_vle_u64()?;
        let key_id = cursor.read_vle_u32()?;
        let suffix_len = if (header & 0x20u8) != 0 {
            let _v = cursor.read_vle_u64()?;
            Some(_v)
        } else {
            None
        };
        let suffix = if (header & 0x20u8) != 0 {
            let _n = suffix_len.unwrap() as usize;
            let raw = cursor.peek_slice(_n)?;
            let _v = core::str::from_utf8(raw)
                .map_err(|_| CodecError::InvalidUtf8)?;
            cursor.advance(_n)?;
            Some(_v)
        } else {
            None
        };
        if (header & 0x80u8) != 0 {
            let _vec = _out.extensions.insert(HeaplessVec::<CodecZenohExtEntry<'a>, 4>::new());
            let mut _more = false;
            for _ in 0..4u32 {
                if cursor.remaining() == 0 { break; }
                let _entry = CodecZenohExtEntry::decode(cursor)?;
                _more = _entry.z();
                // Bounded by max-depth on both sides — loop count and `_vec`
                // capacity are the same literal — so this push cannot fail. An
                // over-long chain is refused by the guard after the loop.
                _vec.push(_entry).map_err(|_| CodecError::TooManyElements)?;
                if !_more { break; }
            }
            if _more && cursor.remaining() == 0 {
                return Err(CodecError::NeedMoreBytes);
            }
            if _more {
                return Err(CodecError::TlvChainOverflow);
            }
        } else {
            _out.extensions = None;
        }
        let _peek = cursor.peek_slice(1)?[0];
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match _peek & 0x1F {
            4u8 => {
                _out.body = CodecZenohResponseVariant::CodecZenohReply(CodecZenohReply::default());
                if let CodecZenohResponseVariant::CodecZenohReply(_b) = &mut _out.body {
                    CodecZenohReply::decode_into(cursor, _b)?;
                }
            }
            5u8 => {
                _out.body = CodecZenohResponseVariant::CodecZenohErr(CodecZenohErr::default());
                if let CodecZenohResponseVariant::CodecZenohErr(_b) = &mut _out.body {
                    CodecZenohErr::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohResponseVariant::Default {
                    tag: other,
                    body: CodecZenohReply::default(),
                };
                if let CodecZenohResponseVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohReply::decode_into(cursor, _b)?;
                }
            }
        }
        _out.header = header;
        _out.request_id = request_id;
        _out.key_id = key_id;
        _out.suffix_len = suffix_len;
        _out.suffix = suffix;
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

    pub fn n(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn set_n(&mut self, v: bool) {
        if v {
            self.header |= 0x20;
        } else {
            self.header &= !0x20;
        }
    }

    pub fn m(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn set_m(&mut self, v: bool) {
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 970;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        Self::encode_source(self, w)
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector, or the error
    /// `encode` refuses a value with: one the encoding cannot carry (a
    /// length past its type, a list past its bound) or one a gate
    /// disagrees with. It is a `Result` for every codec, so that a codec
    /// gaining a derived element, or a build that gates one in, does not
    /// change what a caller writes. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
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

/// The arm of `CodecZenohResponse`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohResponse` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohResponseVariantSource<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> {
    CodecZenohReply(&'s A0),
    CodecZenohErr(&'s A1),
    Default { tag: u8, body: &'s D },
}

/// Flags of `CodecZenohResponse` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecZenohResponse`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecZenohResponseOverrides {
    /// The `M` flag of `header`, when a parent sets it.
    pub header_m: Option<bool>,
}

#[allow(dead_code)]
impl CodecZenohResponseOverrides {
    /// Sets `M` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_m(value: bool) -> Self {
        Self { header_m: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohResponse` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohResponse`
/// computes those from what is ([`CodecZenohResponse::encode_source`]).
pub trait CodecZenohResponseSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `request_id` field.
    fn request_id(&self) -> u64;
    /// The `key_id` field.
    fn key_id(&self) -> u32;
    /// The `suffix` field.
    fn suffix(&self) -> Option<&str>;
    /// The `extensions` field.
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone>;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource>;
}

impl<T: CodecZenohResponseSource + ?Sized> CodecZenohResponseSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn request_id(&self) -> u64 {
        (**self).request_id()
    }
    fn key_id(&self) -> u32 {
        (**self).key_id()
    }
    fn suffix(&self) -> Option<&str> {
        (**self).suffix()
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        (**self).extensions()
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohResponse`'s variant that CodecZenohResponseParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohResponsePartsBody<'a> {
    CodecZenohReply(&'a super::codec_zenoh_reply::CodecZenohReplyParts<'a>),
    CodecZenohErr(&'a super::codec_zenoh_err::CodecZenohErrParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: u8, body: &'a super::codec_zenoh_reply::CodecZenohReplyParts<'a> },
}

/// A value of `CodecZenohResponse` held in the types a caller already has: a source
/// ([`CodecZenohResponseSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohResponseParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `request_id` field.
    pub request_id: u64,
    /// The `key_id` field.
    pub key_id: u32,
    /// The `suffix` field.
    pub suffix: Option<&'a str>,
    /// The `extensions` field.
    pub extensions: Option<&'a [super::codec_zenoh_ext_entry::CodecZenohExtEntryParts<'a>]>,
    /// The arm of the variant this value holds.
    pub body: CodecZenohResponsePartsBody<'a>,
}

impl<'a> CodecZenohResponseSource for CodecZenohResponseParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn request_id(&self) -> u64 {
        self.request_id
    }
    fn key_id(&self) -> u32 {
        self.key_id
    }
    fn suffix(&self) -> Option<&str> {
        self.suffix
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.map(<[_]>::iter)
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        match &self.body {
            CodecZenohResponsePartsBody::CodecZenohReply(_b) => CodecZenohResponseVariantSource::CodecZenohReply(*_b),
            CodecZenohResponsePartsBody::CodecZenohErr(_b) => CodecZenohResponseVariantSource::CodecZenohErr(*_b),
            CodecZenohResponsePartsBody::Default { tag, body } => CodecZenohResponseVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<'a> CodecZenohResponseSource for CodecZenohResponse<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn request_id(&self) -> u64 {
        self.request_id
    }
    fn key_id(&self) -> u32 {
        self.key_id
    }
    fn suffix(&self) -> Option<&str> {
        self.suffix
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        match &self.body {
            CodecZenohResponseVariant::CodecZenohReply(_b) => CodecZenohResponseVariantSource::CodecZenohReply(_b),
            CodecZenohResponseVariant::CodecZenohErr(_b) => CodecZenohResponseVariantSource::CodecZenohErr(_b),
            CodecZenohResponseVariant::Default { tag, body } => CodecZenohResponseVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohResponseSource for CodecZenohResponseOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn request_id(&self) -> u64 {
        self.request_id
    }
    fn key_id(&self) -> u32 {
        self.key_id
    }
    fn suffix(&self) -> Option<&str> {
        self.suffix.as_ref().map(::sce_forge_runtime::codec::SceStr::as_str)
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.as_ref().map(|_l| ::sce_forge_runtime::codec::SceList::as_slice(_l).iter())
    }
    fn body(&self) -> CodecZenohResponseVariantSource<'_, impl super::codec_zenoh_reply::CodecZenohReplySource, impl super::codec_zenoh_err::CodecZenohErrSource, impl super::codec_zenoh_reply::CodecZenohReplySource> {
        match &self.body {
            CodecZenohResponseOwnedVariant::CodecZenohReply(_b) => CodecZenohResponseVariantSource::CodecZenohReply(_b),
            CodecZenohResponseOwnedVariant::CodecZenohErr(_b) => CodecZenohResponseVariantSource::CodecZenohErr(_b),
            CodecZenohResponseOwnedVariant::Default { tag, body } => CodecZenohResponseVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohResponse<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohResponseSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecZenohResponseSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new())?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohResponseSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source_in(src, w, &CodecZenohResponseOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecZenohResponseSource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecZenohResponseOverrides) -> Result<(), CodecError> {
        let _k_extensions: usize = src.extensions().map_or(0, |_l| _l.count());
        if _k_extensions > 4 {
            return Err(CodecError::TlvChainOverflow);
        }
        let _p_header_n_0: bool = src.suffix().is_some();
        let _p_header_z_0: bool = src.extensions().is_some();
        let _n_d_suffix_len_0: usize = match src.suffix() {
            Some(_v) => _v.len(),
            None => 0,
        };
        let _f_d_suffix_len: Option<usize> = if src.suffix().is_some() { Some(_n_d_suffix_len_0) } else { None };
        let _d_suffix_len: u64 = u64::try_from(_f_d_suffix_len.unwrap_or(0)).map_err(|_| CodecError::LengthOutOfRange)?;
        let mut _c_header: u8 = (((src.header() & !0x20) | (u8::from(_p_header_n_0) << 5)) & !0x80) | (u8::from(_p_header_z_0) << 7);
        if let Some(_o) = over.header_m {
            _c_header = (_c_header & !0x40) | (u8::from(_o) << 6);
        }
        w.write_u8(_c_header)?;
        w.write_vle_u64(src.request_id())?;
        w.write_vle_u32(src.key_id())?;
        if (_c_header & 0x20) != 0 {
            w.write_vle_u64(_d_suffix_len)?;
        }
        if let Some(_v) = src.suffix() {
            w.write_bytes(_v.as_bytes())?;
        }
        if let Some(_v) = src.extensions() {
            for (_i, _e) in _v.enumerate() {
                CodecZenohExtEntry::write_source_in(&_e, w, &super::codec_zenoh_ext_entry::CodecZenohExtEntryOverrides::header_z(_i + 1 < _k_extensions))?;
            }
        }
        match src.body() {
            CodecZenohResponseVariantSource::CodecZenohReply(_b) => CodecZenohReply::write_source(_b, w)?,
            CodecZenohResponseVariantSource::CodecZenohErr(_b) => CodecZenohErr::write_source(_b, w)?,
            CodecZenohResponseVariantSource::Default { body: _b, .. } => CodecZenohReply::write_source(_b, w)?,
        }
        Ok(())
    }
}

impl<'a, S: CodecZenohResponseSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohResponse<'a> {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohResponse<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecZenohResponseSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohResponseSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        ::sce_forge_runtime::codec::write_window::<Self, S>(src, window)
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohResponse<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohResponseOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohResponseOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohResponseOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohResponseOwned` alone is the build's
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
// `let v: CodecZenohResponseOwned = CodecZenohResponseOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
use super::codec_zenoh_reply::CodecZenohReplyOwned;
use super::codec_zenoh_err::CodecZenohErrOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohResponseOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub header: u8,
    pub request_id: u64,
    pub key_id: u32,
    pub suffix_len: Option<u64>,
    pub suffix: Option<S::Str<256>>,
    pub extensions: Option<S::List<CodecZenohExtEntryOwned<S>, 16>>,
    pub body: CodecZenohResponseOwnedVariant<S>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohResponseOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn n(&self) -> bool {
        (self.header & 0x20) != 0
    }

    pub fn m(&self) -> bool {
        (self.header & 0x40) != 0
    }

    pub fn z(&self) -> bool {
        (self.header & 0x80) != 0
    }
}

// Variant arms wrap distinct body codecs whose owned mirrors differ in
// field count and size, so the tagged union is inherently size-disparate.
// The lint's only remedy is boxing the large arm, which adds an
// indirection (and allocation) the generated decode path does not need —
// and which the non-allocating storage profile could not take at all.
// The size spread is the deliberate tagged-union trade-off.
#[allow(clippy::large_enum_variant)]
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohResponseOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecZenohReply(CodecZenohReplyOwned<S>),
    CodecZenohErr(CodecZenohErrOwned<S>),
    Default {
        tag: u8,
        body: CodecZenohReplyOwned<S>,
    },
}

#[allow(dead_code)]
impl<'a> CodecZenohResponseVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohResponseOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecZenohResponseVariant::CodecZenohReply(_b) => CodecZenohResponseOwnedVariant::CodecZenohReply(_b.try_into_owned_in::<S>()?),
            CodecZenohResponseVariant::CodecZenohErr(_b) => CodecZenohResponseOwnedVariant::CodecZenohErr(_b.try_into_owned_in::<S>()?),
            CodecZenohResponseVariant::Default { tag, body } => CodecZenohResponseOwnedVariant::Default { tag, body: body.try_into_owned_in::<S>()? },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohResponseOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohResponseOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohResponseVariant<'_>, CodecError> {
        let mut _out = CodecZenohResponseVariant::default();
        self.try_as_borrowed_into(&mut _out)?;
        Ok(_out)
    }

    /// [`Self::try_as_borrowed`] into a destination the caller
    /// owns: the active arm is installed in `_out` holding its body's default
    /// and that body is projected where it sits, so the arm body is never
    /// returned up through the variant wrapper.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through. A caller on a path where that shows projects into a
    /// destination it owns. On `Err`, `_out` holds what was projected so far.
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohResponseVariant<'o>) -> Result<(), CodecError> {
        match self {
            CodecZenohResponseOwnedVariant::CodecZenohReply(_b) => {
                *_out = CodecZenohResponseVariant::CodecZenohReply(Default::default());
                if let CodecZenohResponseVariant::CodecZenohReply(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohResponseOwnedVariant::CodecZenohErr(_b) => {
                *_out = CodecZenohResponseVariant::CodecZenohErr(Default::default());
                if let CodecZenohResponseVariant::CodecZenohErr(_o) = _out {
                    _b.try_as_borrowed_into(_o)?;
                }
            }
            CodecZenohResponseOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohResponseVariant::Default { tag: *tag, body: Default::default() };
                if let CodecZenohResponseVariant::Default { body: _o, .. } = _out {
                    body.try_as_borrowed_into(_o)?;
                }
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohResponse<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohResponseOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohResponseOwned<S>, CodecError> {
        Ok(CodecZenohResponseOwned {
            header: self.header,
            request_id: self.request_id,
            key_id: self.key_id,
            suffix_len: self.suffix_len,
            suffix: self.suffix.map(<S::Str<256> as ::sce_forge_runtime::codec::SceStr>::from_view).transpose()?,
            extensions: self.extensions.map(|_v| ::sce_forge_runtime::codec::try_collect_list(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohResponseOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohResponseOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohResponse<'_>, CodecError> {
        let mut _out = CodecZenohResponse::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohResponse<'o>) -> Result<(), CodecError> {
        _out.header = self.header;
        _out.request_id = self.request_id;
        _out.key_id = self.key_id;
        _out.suffix_len = self.suffix_len;
        _out.suffix = self.suffix.as_ref().map(::sce_forge_runtime::codec::SceStr::as_str);
        match &self.extensions {
            Some(_l) => ::sce_forge_runtime::codec::try_project_bounded_into(_out.extensions.insert(Default::default()), _l, |_e| Ok(_e.as_borrowed()))?,
            None => _out.extensions = None,
        }
        self.body.try_as_borrowed_into(&mut _out.body)?;
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
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohResponseOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohResponseOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `try_as_borrowed` followed by
    /// [`CodecZenohResponse::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here. The same
    /// refusals as the projection's: a list longer than the borrowed view
    /// declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> {
        CodecZenohResponse::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecZenohResponse::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
