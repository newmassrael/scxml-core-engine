#![doc = "SCE-MAP: codec_zenoh_oam:56 :: _forge_body"]
// SCE-MAP: codec_zenoh_oam:56 :: _forge_body

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
use super::codec_zenoh_ext_unit::CodecZenohExtUnit;
use super::codec_zenoh_ext_zint::CodecZenohExtZint;
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbuf;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohOamVariant<'a> {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohExtZbuf(CodecZenohExtZbuf<'a>),
    Default {
        tag: u8,
        body: CodecZenohExtUnit,
    },
}

impl<'a> Default for CodecZenohOamVariant<'a> {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecZenohExtUnit(CodecZenohExtUnit::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohOam<'a> {
    pub header: u8,
    pub id: u16,
    pub extensions: Option<HeaplessVec<CodecZenohExtEntry<'a>, 4>>,
    pub body: CodecZenohOamVariant<'a>,
}

// RFC variant-default-uniformity: at least one field's
// `<sce:flags>` carrier declares a wire-MID constant via
// `<sce:flag value="N"/>`. Manual `impl Default` bakes the OR of
// every declared `(value & mask) << bit` into that carrier so a
// freshly-constructed instance carries the wire-MID for its own
// dispatch tag. Fields without declared values fall through to
// `Default::default()` (preserving derive(Default) semantics).
impl<'a> Default for CodecZenohOam<'a> {
    fn default() -> Self {
        Self {
            header: 0x1fu8,
            id: Default::default(),
            extensions: Default::default(),
            body: CodecZenohOamVariant::default(),
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohOam<'a> {
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
        let id = cursor.read_vle_u16()?;
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
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match (header >> 5) & 0x03 {
            0u8 => {
                _out.body = CodecZenohOamVariant::CodecZenohExtUnit(CodecZenohExtUnit::default());
                if let CodecZenohOamVariant::CodecZenohExtUnit(_b) = &mut _out.body {
                    CodecZenohExtUnit::decode_into(cursor, _b)?;
                }
            }
            1u8 => {
                _out.body = CodecZenohOamVariant::CodecZenohExtZint(CodecZenohExtZint::default());
                if let CodecZenohOamVariant::CodecZenohExtZint(_b) = &mut _out.body {
                    CodecZenohExtZint::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecZenohOamVariant::CodecZenohExtZbuf(CodecZenohExtZbuf::default());
                if let CodecZenohOamVariant::CodecZenohExtZbuf(_b) = &mut _out.body {
                    CodecZenohExtZbuf::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohOamVariant::Default {
                    tag: other,
                    body: CodecZenohExtUnit::default(),
                };
                if let CodecZenohOamVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohExtUnit::decode_into(cursor, _b)?;
                }
            }
        }
        _out.header = header;
        _out.id = id;
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

    pub fn enc(&self) -> u8 {
        (self.header >> 5) & 0x03
    }

    pub fn set_enc(&mut self, v: u8) {
        self.header = (self.header & !0x60) | ((v & 0x03) << 5);
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
    pub const MAX_ENCODED_BYTES: usize = 45;

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

/// The arm of `CodecZenohOam`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohOam` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohOamVariantSource<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, D: ?Sized> {
    CodecZenohExtUnit(&'s A0),
    CodecZenohExtZint(&'s A1),
    CodecZenohExtZbuf(&'s A2),
    Default { tag: u8, body: &'s D },
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohOam` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohOam`
/// computes those from what is ([`CodecZenohOam::encode_source`]).
pub trait CodecZenohOamSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The `id` field.
    fn id(&self) -> u16;
    /// The `extensions` field.
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone>;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohOamVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource>;
}

impl<T: CodecZenohOamSource + ?Sized> CodecZenohOamSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn id(&self) -> u16 {
        (**self).id()
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        (**self).extensions()
    }
    fn body(&self) -> CodecZenohOamVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohOam`'s variant that CodecZenohOamParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohOamPartsBody<'a> {
    CodecZenohExtUnit(&'a super::codec_zenoh_ext_unit::CodecZenohExtUnitParts),
    CodecZenohExtZint(&'a super::codec_zenoh_ext_zint::CodecZenohExtZintParts),
    CodecZenohExtZbuf(&'a super::codec_zenoh_ext_zbuf::CodecZenohExtZbufParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: u8, body: &'a super::codec_zenoh_ext_unit::CodecZenohExtUnitParts },
}

/// A value of `CodecZenohOam` held in the types a caller already has: a source
/// ([`CodecZenohOamSource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohOamParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The `id` field.
    pub id: u16,
    /// The `extensions` field.
    pub extensions: Option<&'a [super::codec_zenoh_ext_entry::CodecZenohExtEntryParts<'a>]>,
    /// The arm of the variant this value holds.
    pub body: CodecZenohOamPartsBody<'a>,
}

impl<'a> CodecZenohOamSource for CodecZenohOamParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn id(&self) -> u16 {
        self.id
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.map(<[_]>::iter)
    }
    fn body(&self) -> CodecZenohOamVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohOamPartsBody::CodecZenohExtUnit(_b) => CodecZenohOamVariantSource::CodecZenohExtUnit(*_b),
            CodecZenohOamPartsBody::CodecZenohExtZint(_b) => CodecZenohOamVariantSource::CodecZenohExtZint(*_b),
            CodecZenohOamPartsBody::CodecZenohExtZbuf(_b) => CodecZenohOamVariantSource::CodecZenohExtZbuf(*_b),
            CodecZenohOamPartsBody::Default { tag, body } => CodecZenohOamVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<'a> CodecZenohOamSource for CodecZenohOam<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn id(&self) -> u16 {
        self.id
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.as_ref().map(|_l| _l.iter())
    }
    fn body(&self) -> CodecZenohOamVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohOamVariant::CodecZenohExtUnit(_b) => CodecZenohOamVariantSource::CodecZenohExtUnit(_b),
            CodecZenohOamVariant::CodecZenohExtZint(_b) => CodecZenohOamVariantSource::CodecZenohExtZint(_b),
            CodecZenohOamVariant::CodecZenohExtZbuf(_b) => CodecZenohOamVariantSource::CodecZenohExtZbuf(_b),
            CodecZenohOamVariant::Default { tag, body } => CodecZenohOamVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohOamSource for CodecZenohOamOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn id(&self) -> u16 {
        self.id
    }
    fn extensions(&self) -> Option<impl Iterator<Item = impl super::codec_zenoh_ext_entry::CodecZenohExtEntrySource> + Clone> {
        self.extensions.as_ref().map(|_l| ::sce_forge_runtime::codec::SceList::as_slice(_l).iter())
    }
    fn body(&self) -> CodecZenohOamVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohOamOwnedVariant::CodecZenohExtUnit(_b) => CodecZenohOamVariantSource::CodecZenohExtUnit(_b),
            CodecZenohOamOwnedVariant::CodecZenohExtZint(_b) => CodecZenohOamVariantSource::CodecZenohExtZint(_b),
            CodecZenohOamOwnedVariant::CodecZenohExtZbuf(_b) => CodecZenohOamVariantSource::CodecZenohExtZbuf(_b),
            CodecZenohOamOwnedVariant::Default { tag, body } => CodecZenohOamVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohOam<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohOamSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecZenohOamSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new())?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohOamSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        let _k_extensions: usize = src.extensions().map_or(0, |_l| _l.count());
        if _k_extensions > 4 {
            return Err(CodecError::TlvChainOverflow);
        }
        let _d_header_enc: u8 = match src.body() {
            CodecZenohOamVariantSource::CodecZenohExtUnit(_) => 0,
            CodecZenohOamVariantSource::CodecZenohExtZint(_) => 1,
            CodecZenohOamVariantSource::CodecZenohExtZbuf(_) => 2,
            CodecZenohOamVariantSource::Default { tag, .. } => tag,
        };
        let _p_header_z_0: bool = src.extensions().is_some();
        if _d_header_enc > 0x3 {
            return Err(CodecError::LengthOutOfRange);
        }
        let _c_header: u8 = (((src.header() & !0x60) | (_d_header_enc << 5)) & !0x80) | (u8::from(_p_header_z_0) << 7);
        w.write_u8(_c_header)?;
        w.write_vle_u16(src.id())?;
        if let Some(_v) = src.extensions() {
            for (_i, _e) in _v.enumerate() {
                CodecZenohExtEntry::write_source_in(&_e, w, &super::codec_zenoh_ext_entry::CodecZenohExtEntryOverrides::header_z(_i + 1 < _k_extensions))?;
            }
        }
        match src.body() {
            CodecZenohOamVariantSource::CodecZenohExtUnit(_b) => CodecZenohExtUnit::write_source(_b, w)?,
            CodecZenohOamVariantSource::CodecZenohExtZint(_b) => CodecZenohExtZint::write_source(_b, w)?,
            CodecZenohOamVariantSource::CodecZenohExtZbuf(_b) => CodecZenohExtZbuf::write_source(_b, w)?,
            CodecZenohOamVariantSource::Default { body: _b, .. } => CodecZenohExtUnit::write_source(_b, w)?,
        }
        Ok(())
    }
}

impl<'a, S: CodecZenohOamSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohOam<'a> {
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohOam<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecZenohOamSource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohOamSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        ::sce_forge_runtime::codec::write_window::<Self, S>(src, window)
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohOam<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohOamOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohOamOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohOamOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohOamOwned` alone is the build's
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
// `let v: CodecZenohOamOwned = CodecZenohOamOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbufOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohOamOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub header: u8,
    pub id: u16,
    pub extensions: Option<S::List<CodecZenohExtEntryOwned<S>, 16>>,
    pub body: CodecZenohOamOwnedVariant<S>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohOamOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn mid(&self) -> u8 {
        self.header & 0x1F
    }

    pub fn enc(&self) -> u8 {
        (self.header >> 5) & 0x03
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
pub enum CodecZenohOamOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohExtZbuf(CodecZenohExtZbufOwned<S>),
    Default {
        tag: u8,
        body: CodecZenohExtUnit,
    },
}

#[allow(dead_code)]
impl<'a> CodecZenohOamVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohOamOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecZenohOamVariant::CodecZenohExtUnit(_b) => CodecZenohOamOwnedVariant::CodecZenohExtUnit(_b),
            CodecZenohOamVariant::CodecZenohExtZint(_b) => CodecZenohOamOwnedVariant::CodecZenohExtZint(_b),
            CodecZenohOamVariant::CodecZenohExtZbuf(_b) => CodecZenohOamOwnedVariant::CodecZenohExtZbuf(_b.try_into_owned_in::<S>()?),
            CodecZenohOamVariant::Default { tag, body } => CodecZenohOamOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohOamOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohOamOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn as_borrowed(&self) -> CodecZenohOamVariant<'_> {
        let mut _out = CodecZenohOamVariant::default();
        self.as_borrowed_into(&mut _out);
        _out
    }

    /// [`Self::as_borrowed`] into a destination the caller
    /// owns: the active arm is installed in `_out` holding its body's default
    /// and that body is projected where it sits, so the arm body is never
    /// returned up through the variant wrapper.
    ///
    /// A bounded list moves by copying its whole declared capacity, so a
    /// projection that returns one copies the declared maximum at every layer
    /// it passes through. A caller on a path where that shows projects into a
    /// destination it owns. On `Err`, `_out` holds what was projected so far.
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohOamVariant<'o>) {
        match self {
            CodecZenohOamOwnedVariant::CodecZenohExtUnit(_b) => {
                *_out = CodecZenohOamVariant::CodecZenohExtUnit(Default::default());
                if let CodecZenohOamVariant::CodecZenohExtUnit(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohOamOwnedVariant::CodecZenohExtZint(_b) => {
                *_out = CodecZenohOamVariant::CodecZenohExtZint(Default::default());
                if let CodecZenohOamVariant::CodecZenohExtZint(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohOamOwnedVariant::CodecZenohExtZbuf(_b) => {
                *_out = CodecZenohOamVariant::CodecZenohExtZbuf(Default::default());
                if let CodecZenohOamVariant::CodecZenohExtZbuf(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohOamOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohOamVariant::Default { tag: *tag, body: Default::default() };
                if let CodecZenohOamVariant::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohOam<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohOamOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohOamOwned<S>, CodecError> {
        Ok(CodecZenohOamOwned {
            header: self.header,
            id: self.id,
            extensions: self.extensions.map(|_v| ::sce_forge_runtime::codec::try_collect_list(_v, |_e| _e.try_into_owned_in::<S>())).transpose()?,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohOamOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohOamOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `try_as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    /// Fallible: a bounded `<sce:repeat>` / `<sce:tlv-chain>` list holding
    /// more than its declared `N` raises `CodecError::TooManyElements` — the
    /// same bound decode enforces. Only a growable profile can hold such a
    /// list; on the inline profile the source is already within bounds.
    pub fn try_as_borrowed(&self) -> Result<CodecZenohOam<'_>, CodecError> {
        let mut _out = CodecZenohOam::default();
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
    pub fn try_as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohOam<'o>) -> Result<(), CodecError> {
        _out.header = self.header;
        _out.id = self.id;
        match &self.extensions {
            Some(_l) => ::sce_forge_runtime::codec::try_project_bounded_into(_out.extensions.insert(Default::default()), _l, |_e| Ok(_e.as_borrowed()))?,
            None => _out.extensions = None,
        }
        self.body.as_borrowed_into(&mut _out.body);
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
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohOamOwned<D>, CodecError> {
        self.try_as_borrowed()?.try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohOamOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `try_as_borrowed` followed by
    /// [`CodecZenohOam::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here. The same
    /// refusals as the projection's: a list longer than the borrowed view
    /// declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> {
        CodecZenohOam::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecZenohOam::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
