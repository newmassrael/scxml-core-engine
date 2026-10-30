#![doc = "SCE-MAP: codec_zenoh_ext_entry:52 :: _forge_body"]
// SCE-MAP: codec_zenoh_ext_entry:52 :: _forge_body

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

use super::codec_zenoh_ext_unit::CodecZenohExtUnit;
use super::codec_zenoh_ext_zint::CodecZenohExtZint;
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbuf;

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohExtEntryVariant<'a> {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohExtZbuf(CodecZenohExtZbuf<'a>),
    Default {
        tag: u8,
        body: CodecZenohExtUnit,
    },
}

impl<'a> Default for CodecZenohExtEntryVariant<'a> {
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
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecZenohExtEntry<'a> {
    pub header: u8,
    pub body: CodecZenohExtEntryVariant<'a>,
}

#[allow(dead_code)]
impl<'a> CodecZenohExtEntry<'a> {
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
        // Decode fixed prefix (RFC §synth-5-B variant primitive: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let header = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match (header >> 5) & 0x03 {
            0u8 => {
                _out.body = CodecZenohExtEntryVariant::CodecZenohExtUnit(CodecZenohExtUnit::default());
                if let CodecZenohExtEntryVariant::CodecZenohExtUnit(_b) = &mut _out.body {
                    CodecZenohExtUnit::decode_into(cursor, _b)?;
                }
            }
            1u8 => {
                _out.body = CodecZenohExtEntryVariant::CodecZenohExtZint(CodecZenohExtZint::default());
                if let CodecZenohExtEntryVariant::CodecZenohExtZint(_b) = &mut _out.body {
                    CodecZenohExtZint::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecZenohExtEntryVariant::CodecZenohExtZbuf(CodecZenohExtZbuf::default());
                if let CodecZenohExtEntryVariant::CodecZenohExtZbuf(_b) = &mut _out.body {
                    CodecZenohExtZbuf::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecZenohExtEntryVariant::Default {
                    tag: other,
                    body: CodecZenohExtUnit::default(),
                };
                if let CodecZenohExtEntryVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecZenohExtUnit::decode_into(cursor, _b)?;
                }
            }
        }
        _out.header = header;
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn ext_id(&self) -> u8 {
        self.header & 0x0F
    }

    pub fn set_ext_id(&mut self, v: u8) {
        self.header = (self.header & !0x0F) | (v & 0x0F);
    }

    pub fn m(&self) -> bool {
        (self.header & 0x10) != 0
    }

    pub fn set_m(&mut self, v: bool) {
        if v {
            self.header |= 0x10;
        } else {
            self.header &= !0x10;
        }
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
    pub const MAX_ENCODED_BYTES: usize = 42;

    /// Encode `self` into the caller-owned sink. Returns the value's refusal
    /// when the encoding cannot carry it (a length past its type, a list past
    /// its bound, a gate that disagrees with a presence; the document declares
    /// `sce:encode-may-fail`), or the sink's own error:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> where CodecError: From<S::Error> {
        Self::encode_source(self, w)
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

/// The arm of `CodecZenohExtEntry`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecZenohExtEntry` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecZenohExtEntryVariantSource<'s, A0: ?Sized, A1: ?Sized, A2: ?Sized, D: ?Sized> {
    CodecZenohExtUnit(&'s A0),
    CodecZenohExtZint(&'s A1),
    CodecZenohExtZbuf(&'s A2),
    Default { tag: u8, body: &'s D },
}

/// Flags of `CodecZenohExtEntry` a parent sets in place of the ones a value presents:
/// the end flag of a chain whose entries are `CodecZenohExtEntry`, which the chain
/// derives from where an entry stands.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodecZenohExtEntryOverrides {
    /// The `M` flag of `header`, when a parent sets it.
    pub header_m: Option<bool>,
    /// The `Z` flag of `header`, when a parent sets it.
    pub header_z: Option<bool>,
}

#[allow(dead_code)]
impl CodecZenohExtEntryOverrides {
    /// Sets `M` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_m(value: bool) -> Self {
        Self { header_m: Some(value), ..Self::default() }
    }

    /// Sets `Z` of `header` and nothing else.
    #[allow(clippy::needless_update)]
    pub fn header_z(value: bool) -> Self {
        Self { header_z: Some(value), ..Self::default() }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecZenohExtEntry` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecZenohExtEntry`
/// computes those from what is ([`CodecZenohExtEntry::encode_source`]).
pub trait CodecZenohExtEntrySource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecZenohExtEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource>;
}

impl<T: CodecZenohExtEntrySource + ?Sized> CodecZenohExtEntrySource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn body(&self) -> CodecZenohExtEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        (**self).body()
    }
}

/// The arm of `CodecZenohExtEntry`'s variant that CodecZenohExtEntryParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecZenohExtEntryPartsBody<'a> {
    CodecZenohExtUnit(&'a super::codec_zenoh_ext_unit::CodecZenohExtUnitParts),
    CodecZenohExtZint(&'a super::codec_zenoh_ext_zint::CodecZenohExtZintParts),
    CodecZenohExtZbuf(&'a super::codec_zenoh_ext_zbuf::CodecZenohExtZbufParts<'a>),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: u8, body: &'a super::codec_zenoh_ext_unit::CodecZenohExtUnitParts },
}

/// A value of `CodecZenohExtEntry` held in the types a caller already has: a source
/// ([`CodecZenohExtEntrySource`]) whose lists are slices and whose bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecZenohExtEntryParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The arm of the variant this value holds.
    pub body: CodecZenohExtEntryPartsBody<'a>,
}

impl<'a> CodecZenohExtEntrySource for CodecZenohExtEntryParts<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohExtEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohExtEntryPartsBody::CodecZenohExtUnit(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtUnit(*_b),
            CodecZenohExtEntryPartsBody::CodecZenohExtZint(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtZint(*_b),
            CodecZenohExtEntryPartsBody::CodecZenohExtZbuf(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtZbuf(*_b),
            CodecZenohExtEntryPartsBody::Default { tag, body } => CodecZenohExtEntryVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl<'a> CodecZenohExtEntrySource for CodecZenohExtEntry<'a> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohExtEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohExtEntryVariant::CodecZenohExtUnit(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtUnit(_b),
            CodecZenohExtEntryVariant::CodecZenohExtZint(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtZint(_b),
            CodecZenohExtEntryVariant::CodecZenohExtZbuf(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtZbuf(_b),
            CodecZenohExtEntryVariant::Default { tag, body } => CodecZenohExtEntryVariantSource::Default { tag: *tag, body },
        }
    }
}

impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohExtEntrySource for CodecZenohExtEntryOwned<S> {
    fn header(&self) -> u8 {
        self.header
    }
    fn body(&self) -> CodecZenohExtEntryVariantSource<'_, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource, impl super::codec_zenoh_ext_zint::CodecZenohExtZintSource, impl super::codec_zenoh_ext_zbuf::CodecZenohExtZbufSource, impl super::codec_zenoh_ext_unit::CodecZenohExtUnitSource> {
        match &self.body {
            CodecZenohExtEntryOwnedVariant::CodecZenohExtUnit(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtUnit(_b),
            CodecZenohExtEntryOwnedVariant::CodecZenohExtZint(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtZint(_b),
            CodecZenohExtEntryOwnedVariant::CodecZenohExtZbuf(_b) => CodecZenohExtEntryVariantSource::CodecZenohExtZbuf(_b),
            CodecZenohExtEntryOwnedVariant::Default { tag, body } => CodecZenohExtEntryVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohExtEntry<'a> {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecZenohExtEntrySource`]. A length, a count or a gate flag is computed from what
    /// `src` presents, and a value the encoding cannot carry is refused
    /// before the first byte is written.
    pub fn encode_source<S: CodecZenohExtEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecZenohExtEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source_in(src, w, &CodecZenohExtEntryOverrides::default())
    }

    /// [`Self::write_source`] with the flags a parent sets in place of the
    /// ones `src` presents.
    #[doc(hidden)]
    pub fn write_source_in<S: CodecZenohExtEntrySource + ?Sized, K: SceSink>(src: &S, w: &mut K, over: &CodecZenohExtEntryOverrides) -> Result<(), CodecError> where CodecError: From<K::Error> {
        let _d_header_enc: u8 = match src.body() {
            CodecZenohExtEntryVariantSource::CodecZenohExtUnit(_) => 0,
            CodecZenohExtEntryVariantSource::CodecZenohExtZint(_) => 1,
            CodecZenohExtEntryVariantSource::CodecZenohExtZbuf(_) => 2,
            CodecZenohExtEntryVariantSource::Default { tag, .. } => tag,
        };
        if _d_header_enc > 0x3 {
            return Err(CodecError::LengthOutOfRange);
        }
        let mut _c_header: u8 = (src.header() & !0x60) | (_d_header_enc << 5);
        if let Some(_o) = over.header_m {
            _c_header = (_c_header & !0x10) | (u8::from(_o) << 4);
        }
        if let Some(_o) = over.header_z {
            _c_header = (_c_header & !0x80) | (u8::from(_o) << 7);
        }
        w.write_u8(_c_header)?;
        match src.body() {
            CodecZenohExtEntryVariantSource::CodecZenohExtUnit(_b) => CodecZenohExtUnit::write_source(_b, w)?,
            CodecZenohExtEntryVariantSource::CodecZenohExtZint(_b) => CodecZenohExtZint::write_source(_b, w)?,
            CodecZenohExtEntryVariantSource::CodecZenohExtZbuf(_b) => CodecZenohExtZbuf::write_source(_b, w)?,
            CodecZenohExtEntryVariantSource::Default { body: _b, .. } => CodecZenohExtUnit::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecZenohExtEntrySource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<'a, S: CodecZenohExtEntrySource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecZenohExtEntry<'a> {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohExtEntry<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: CodecZenohExtEntrySource + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecZenohExtEntrySource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── Owned projection (storage-parameterised native form) ─────────────
// `CodecZenohExtEntry<'a>` above is a zero-copy view borrowing the decode
// buffer. Consumers that persist a decoded value beyond the buffer's
// lifetime — including the self-contained bounded-collection that stores
// elements by value — call `.try_into_owned()` for this lifetime-free
// `CodecZenohExtEntryOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split, both generated from the one SCXML source (SSOT).
//
// The owned form is parameterised by a storage profile rather than fixed at
// build-configuration time: `CodecZenohExtEntryOwned<Heap>` holds growable
// containers with the declared capacities advisory, `CodecZenohExtEntryOwned<Inline>`
// holds every field inline at its declared capacity and never allocates, and
// both exist in the same binary. `CodecZenohExtEntryOwned` alone is the build's
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
// `let v: CodecZenohExtEntryOwned = CodecZenohExtEntryOwned { .. };`
// — because the fields reach the profile through its associated container
// types, which cannot be run backwards to recover the profile from a value.
// Naming it once is also what lets each field's declared capacity infer,
// so no call site repeats a `sce:max-size` / `sce:max-count` constant.
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbufOwned;
// Same pub-API policy as the borrowed view above: the owned mirror and its
// projections are cross-crate surface, and which of them a given in-repo
// fixture happens to call says nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohExtEntryOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    pub header: u8,
    pub body: CodecZenohExtEntryOwnedVariant<S>,
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohExtEntryOwned<S> {
    // RFC §synth-5-B read-accessor parity with the borrowed view: pure
    // bit getters over the copied carrier (rkyv Archived↔native getter
    // parity), so owned consumers read `{Codec}Owned` with the same API as
    // the borrowed view and never re-derive the SCE wire bit layout (SSOT).
    // Read-only — write accessors belong with an owned-encode path, which
    // does not exist yet.
    pub fn ext_id(&self) -> u8 {
        self.header & 0x0F
    }

    pub fn m(&self) -> bool {
        (self.header & 0x10) != 0
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
pub enum CodecZenohExtEntryOwnedVariant<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohExtZbuf(CodecZenohExtZbufOwned<S>),
    Default {
        tag: u8,
        body: CodecZenohExtUnit,
    },
}

#[allow(dead_code)]
impl<'a> CodecZenohExtEntryVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror at the
    /// given storage profile. Fallible because a borrowed arm body's own
    /// projection re-checks its declared capacities (the same bounds decode
    /// enforces) — on the growable profile no check can fire.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohExtEntryOwnedVariant<S>, CodecError> {
        Ok(match self {
            CodecZenohExtEntryVariant::CodecZenohExtUnit(_b) => CodecZenohExtEntryOwnedVariant::CodecZenohExtUnit(_b),
            CodecZenohExtEntryVariant::CodecZenohExtZint(_b) => CodecZenohExtEntryOwnedVariant::CodecZenohExtZint(_b),
            CodecZenohExtEntryVariant::CodecZenohExtZbuf(_b) => CodecZenohExtEntryOwnedVariant::CodecZenohExtZbuf(_b.try_into_owned_in::<S>()?),
            CodecZenohExtEntryVariant::Default { tag, body } => CodecZenohExtEntryOwnedVariant::Default { tag, body },
        })
    }

    /// The same projection at the build's default storage profile.
    pub fn try_into_owned(self) -> Result<CodecZenohExtEntryOwnedVariant, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohExtEntryOwnedVariant<S> {
    /// Re-borrow this owned variant body back into its borrowed mirror —
    /// the inverse of the projection above. Reuses the borrowed view's
    /// single `encode`; the owned form deliberately carries no encode of
    /// its own.
    pub fn as_borrowed(&self) -> CodecZenohExtEntryVariant<'_> {
        let mut _out = CodecZenohExtEntryVariant::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohExtEntryVariant<'o>) {
        match self {
            CodecZenohExtEntryOwnedVariant::CodecZenohExtUnit(_b) => {
                *_out = CodecZenohExtEntryVariant::CodecZenohExtUnit(Default::default());
                if let CodecZenohExtEntryVariant::CodecZenohExtUnit(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohExtEntryOwnedVariant::CodecZenohExtZint(_b) => {
                *_out = CodecZenohExtEntryVariant::CodecZenohExtZint(Default::default());
                if let CodecZenohExtEntryVariant::CodecZenohExtZint(_o) = _out {
                    *_o = _b.clone();
                }
            }
            CodecZenohExtEntryOwnedVariant::CodecZenohExtZbuf(_b) => {
                *_out = CodecZenohExtEntryVariant::CodecZenohExtZbuf(Default::default());
                if let CodecZenohExtEntryVariant::CodecZenohExtZbuf(_o) = _out {
                    _b.as_borrowed_into(_o);
                }
            }
            CodecZenohExtEntryOwnedVariant::Default { tag, body } => {
                *_out = CodecZenohExtEntryVariant::Default { tag: *tag, body: Default::default() };
                if let CodecZenohExtEntryVariant::Default { body: _o, .. } = _out {
                    *_o = body.clone();
                }
            }
        }
        
    }
}

#[allow(dead_code)]
impl<'a> CodecZenohExtEntry<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohExtEntryOwned`] held in the given storage profile. Call at
    /// a decode boundary when the decoded value must outlive the input
    /// buffer — stored in a long-lived enum, moved across an async task, or
    /// inserted by value into a bounded-collection.
    ///
    /// Fallible for profile uniformity: on the inline profile a field longer
    /// than its declared capacity raises `CodecError::TooManyElements` (the
    /// same bound and error decode enforces), on the growable profile the
    /// copy cannot fail. The borrowed zero-copy path is unaffected either
    /// way.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<CodecZenohExtEntryOwned<S>, CodecError> {
        Ok(CodecZenohExtEntryOwned {
            header: self.header,
            body: self.body.try_into_owned_in::<S>()?,
        })
    }

    /// The same projection at the build's default storage profile — growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<CodecZenohExtEntryOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohExtEntryOwned<S> {
    /// Re-borrow this owned value back into the zero-copy borrowed view —
    /// the inverse of `try_into_owned_in`. `encode` lives only on the
    /// borrowed view (the owned form is read-only), so an owned consumer
    /// reaches it via `as_borrowed` then `encode` / `encode_to_vec`.
    /// Each field is projected by reference — a cheap re-borrow, not a copy.
    pub fn as_borrowed(&self) -> CodecZenohExtEntry<'_> {
        let mut _out = CodecZenohExtEntry::default();
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
    pub fn as_borrowed_into<'o>(&'o self, _out: &mut CodecZenohExtEntry<'o>) {
        _out.header = self.header;
        self.body.as_borrowed_into(&mut _out.body);
        
    }

    /// Move this value to a different storage profile — growable to inline
    /// when handing it to a path that must not allocate, or inline to
    /// growable when it is leaving that path.
    ///
    /// A checked projection through the borrowed view, not a re-decode: the
    /// bytes are copied once and every destination capacity is enforced, so
    /// a value that cannot fit the target profile is rejected here rather
    /// than truncated.
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<CodecZenohExtEntryOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> CodecZenohExtEntryOwned<S> {
    /// Encode `self` into the caller-owned sink, straight from the owned
    /// fields: the bytes `as_borrowed` followed by
    /// [`CodecZenohExtEntry::encode`] writes, without the projection.
    ///
    /// The projection builds the whole borrowed record (every list and
    /// embedded body in it, by value) only for `encode` to read it once. A
    /// sender that holds an owned value encodes it from here.
    /// The same refusals as the projection's: a list longer than the borrowed
    /// view declares raises `CodecError::TooManyElements`, so an owned value the
    /// borrowed form could not carry is an error and never a frame its own
    /// decoder would reject.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {
        CodecZenohExtEntry::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`]. Fallible: a list
    /// past the bound the borrowed view declares is refused, as is any value
    /// the encoding cannot carry.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(CodecZenohExtEntry::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)?;
        Ok(_sce_v)
    }
}
