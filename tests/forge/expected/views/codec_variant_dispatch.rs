#![doc = "SCE-MAP: codec_variant_dispatch:8 :: _forge_body"]
// SCE-MAP: codec_variant_dispatch:8 :: _forge_body

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

use super::codec_variant_session_open::CodecVariantSessionOpen;
use super::codec_variant_session_close::CodecVariantSessionClose;

use super::codec_variant_session_open::CodecVariantSessionOpenView;
use super::codec_variant_session_close::CodecVariantSessionCloseView;

/// A tag `CodecVariantDispatch` names no arm for, which its default arm holds: a
/// value inside the 8 bits the tag is read from and outside
/// every arm's tag (docs/adr/0065 §6 R3). A default arm is written with the tag
/// it holds and read back as the default arm, so no tag it holds is refused.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodecVariantDispatchUnknownTag(u8);

#[allow(dead_code)]
impl CodecVariantDispatchUnknownTag {
    /// `tag` as a tag no arm names, or `None` when it is past the tag's bits or
    /// is an arm's.
    pub const fn new(tag: u8) -> Option<Self> {
        match tag {
            1..=2 => None,
            _ => Some(Self(tag)),
        }
    }

    /// The tag.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl ::sce_forge_runtime::codec::UnknownTag for CodecVariantDispatchUnknownTag {
    const SPACE: ::sce_forge_runtime::codec::TagSpace =
        ::sce_forge_runtime::codec::TagSpace::new(255, &[(1, 2)]);

    fn widened(self) -> u64 {
        u64::from(self.0)
    }

    fn from_admitted(tag: u64, _proof: ::sce_forge_runtime::codec::Admitted) -> Self {
        Self(tag as u8)
    }
}

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecVariantDispatchVariant {
    CodecVariantSessionOpen(CodecVariantSessionOpen),
    CodecVariantSessionClose(CodecVariantSessionClose),
    Default {
        tag: CodecVariantDispatchUnknownTag,
        body: CodecVariantSessionClose,
    },
}

impl Default for CodecVariantDispatchVariant {
    fn default() -> Self {
        // RFC variant-default-uniformity: pick the declared default
        // arm (`<sce:arm default="true"/>`) so a freshly-constructed
        // envelope round-trips byte-exactly through `encode() ->
        // decode()` — pairs with the inner codec's `<sce:flag value=>`
        // -baked `Default::default()` to close the dispatch loop.
        Self::CodecVariantSessionClose(CodecVariantSessionClose::default())
    }
}

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecVariantDispatch {
    pub msg_id: u8,
    pub body: CodecVariantDispatchVariant,
}

#[allow(dead_code)]
impl CodecVariantDispatch {
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
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
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
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
        // Decode fixed prefix (RFC §synth-5-B variant primitive: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let msg_id = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match msg_id {
            1u8 => {
                _out.body = CodecVariantDispatchVariant::CodecVariantSessionOpen(CodecVariantSessionOpen::default());
                if let CodecVariantDispatchVariant::CodecVariantSessionOpen(_b) = &mut _out.body {
                    CodecVariantSessionOpen::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecVariantDispatchVariant::CodecVariantSessionClose(CodecVariantSessionClose::default());
                if let CodecVariantDispatchVariant::CodecVariantSessionClose(_b) = &mut _out.body {
                    CodecVariantSessionClose::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecVariantDispatchVariant::Default {
                    tag: CodecVariantDispatchUnknownTag(other),
                    body: CodecVariantSessionClose::default(),
                };
                if let CodecVariantDispatchVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecVariantSessionClose::decode_into(cursor, _b)?;
                }
            }
        }
        _out.msg_id = msg_id;
        Ok(())
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 3;

    /// Encode `self` into the caller-owned sink. Every value has a frame
    /// (docs/adr/0065), so the only error is the sink's own:
    /// `CodecError::BufferOverflow` from a bounded sink when the destination
    /// has insufficient remaining capacity, and none from a sink that cannot
    /// refuse (`VecSink`, `CountSink`).
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        Self::encode_source(self, w)
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

/// The arm of `CodecVariantDispatch`'s variant a value holds, each arm's body as the
/// source it says. The tag is not among them: `CodecVariantDispatch` derives it from
/// the arm, as it derives a length, and a default arm presents the tag
/// it could not name.
// A default arm's tag is read by the codec that embeds this one and
// derives its own carrier bits from the arm, which not every build uses.
#[allow(dead_code)]
pub enum CodecVariantDispatchVariantSource<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> {
    CodecVariantSessionOpen(&'s A0),
    CodecVariantSessionClose(&'s A1),
    Default { tag: CodecVariantDispatchUnknownTag, body: &'s D },
}

impl<'s, A0: ?Sized, A1: ?Sized, D: ?Sized> CodecVariantDispatchVariantSource<'s, A0, A1, D> {
    /// The tag of the arm this is: the value the document gives a known
    /// arm, and the tag a default arm holds. It is the tag the encode
    /// writes, or the one a caller hands the decode for a variant whose
    /// wire form carries none.
    pub fn tag(&self) -> u8 {
        match self {
            Self::CodecVariantSessionOpen(_) => 1,
            Self::CodecVariantSessionClose(_) => 2,
            Self::Default { tag, .. } => tag.get(),
        }
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecVariantDispatch` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecVariantDispatch`
/// computes those from what is ([`CodecVariantDispatch::encode_source`]).
pub trait CodecVariantDispatchSource {
    /// The arm of the variant this value holds.
    fn body(&self) -> CodecVariantDispatchVariantSource<'_, impl super::codec_variant_session_open::CodecVariantSessionOpenSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource>;
}

impl<T: CodecVariantDispatchSource + ?Sized> CodecVariantDispatchSource for &T {
    fn body(&self) -> CodecVariantDispatchVariantSource<'_, impl super::codec_variant_session_open::CodecVariantSessionOpenSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource> {
        (**self).body()
    }
}

/// The arm of `CodecVariantDispatch`'s variant that CodecVariantDispatchParts holds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CodecVariantDispatchPartsBody<'a> {
    CodecVariantSessionOpen(&'a super::codec_variant_session_open::CodecVariantSessionOpenParts),
    CodecVariantSessionClose(&'a super::codec_variant_session_close::CodecVariantSessionCloseParts),
    /// The tag the grammar could not name, and the body it holds.
    Default { tag: CodecVariantDispatchUnknownTag, body: &'a super::codec_variant_session_close::CodecVariantSessionCloseParts },
}

/// A value of `CodecVariantDispatch` held in the types a caller already has: a source
/// ([`CodecVariantDispatchSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecVariantDispatchParts<'a> {
    /// The arm of the variant this value holds.
    pub body: CodecVariantDispatchPartsBody<'a>,
}

impl<'a> CodecVariantDispatchSource for CodecVariantDispatchParts<'a> {
    fn body(&self) -> CodecVariantDispatchVariantSource<'_, impl super::codec_variant_session_open::CodecVariantSessionOpenSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource> {
        match &self.body {
            CodecVariantDispatchPartsBody::CodecVariantSessionOpen(_b) => CodecVariantDispatchVariantSource::CodecVariantSessionOpen(*_b),
            CodecVariantDispatchPartsBody::CodecVariantSessionClose(_b) => CodecVariantDispatchVariantSource::CodecVariantSessionClose(*_b),
            CodecVariantDispatchPartsBody::Default { tag, body } => CodecVariantDispatchVariantSource::Default { tag: *tag, body: *body },
        }
    }
}

impl CodecVariantDispatchSource for CodecVariantDispatch {
    fn body(&self) -> CodecVariantDispatchVariantSource<'_, impl super::codec_variant_session_open::CodecVariantSessionOpenSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource> {
        match &self.body {
            CodecVariantDispatchVariant::CodecVariantSessionOpen(_b) => CodecVariantDispatchVariantSource::CodecVariantSessionOpen(_b),
            CodecVariantDispatchVariant::CodecVariantSessionClose(_b) => CodecVariantDispatchVariantSource::CodecVariantSessionClose(_b),
            CodecVariantDispatchVariant::Default { tag, body } => CodecVariantDispatchVariantSource::Default { tag: *tag, body },
        }
    }
}

impl CodecVariantDispatchSource for CodecVariantDispatchView {
    fn body(&self) -> CodecVariantDispatchVariantSource<'_, impl super::codec_variant_session_open::CodecVariantSessionOpenSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource, impl super::codec_variant_session_close::CodecVariantSessionCloseSource> {
        match &self.body {
            CodecVariantDispatchViewVariant::CodecVariantSessionOpen(_b) => CodecVariantDispatchVariantSource::CodecVariantSessionOpen(_b),
            CodecVariantDispatchViewVariant::CodecVariantSessionClose(_b) => CodecVariantDispatchVariantSource::CodecVariantSessionClose(_b),
            CodecVariantDispatchViewVariant::Default { tag, body } => CodecVariantDispatchVariantSource::Default { tag: *tag, body },
        }
    }
}

#[allow(dead_code)]
impl CodecVariantDispatch {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecVariantDispatchSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecVariantDispatchSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// The tag of the arm `src` holds ([`CodecVariantDispatchVariantSource::tag`]).
    pub fn tag_of<S: CodecVariantDispatchSource + ?Sized>(src: &S) -> u8 {
        src.body().tag()
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecVariantDispatchSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let _d_msg_id: u8 = src.body().tag();
        w.write_u8(_d_msg_id)?;
        match src.body() {
            CodecVariantDispatchVariantSource::CodecVariantSessionOpen(_b) => CodecVariantSessionOpen::write_source(_b, w)?,
            CodecVariantDispatchVariantSource::CodecVariantSessionClose(_b) => CodecVariantSessionClose::write_source(_b, w)?,
            CodecVariantDispatchVariantSource::Default { body: _b, .. } => CodecVariantSessionClose::write_source(_b, w)?,
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecVariantDispatchSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecVariantDispatchSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecVariantDispatch {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecVariantDispatch {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecVariantDispatchSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecVariantDispatchSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}


// ── View (ADR 0049) ──────────────────────────────────────────────────
// `CodecVariantDispatchView` is what a decode of `CodecVariantDispatch` learned, not a
// copy of what the wire repeated: a scalar, a string or a bytes field is held
// as `CodecVariantDispatch` holds it, an embedded body or a variant arm as its
// own view, and a bounded list as the frame it was read from, how many entries
// it holds and what a walk needs to read them again (`ListView`). Its size
// depends on the number of fields and not on any `sce:max-count`, so moving
// one copies a few words however long its lists may be. `decode` refuses the
// frames `CodecVariantDispatch::decode` refuses, with the same error: the entries
// are decoded once to check them, and a walk afterwards cannot fail.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecVariantDispatchViewVariant {
    CodecVariantSessionOpen(CodecVariantSessionOpenView),
    CodecVariantSessionClose(CodecVariantSessionCloseView),
    Default {
        tag: CodecVariantDispatchUnknownTag,
        body: CodecVariantSessionCloseView,
    },
}

impl Default for CodecVariantDispatchViewVariant {
    fn default() -> Self {
        Self::CodecVariantSessionClose(CodecVariantSessionCloseView::default())
    }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecVariantDispatchView {
    pub msg_id: u8,
    pub body: CodecVariantDispatchViewVariant,
}

#[allow(dead_code)]
impl CodecVariantDispatchView {
    /// A view with every field at its own type's [`Default`]: the destination
    /// [`Self::decode_into`] fills.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`: the bytes, the bounds and the
    /// errors of [`CodecVariantDispatch::decode`], held as a view. On success the
    /// cursor advances past the consumed bytes.
    pub fn decode(cursor: &mut SceCursor<'_>) -> Result<Self, CodecError> {
        let mut _out = Self::default();
        Self::decode_into(cursor, &mut _out)?;
        Ok(_out)
    }

    /// [`Self::decode`] into a destination the caller owns, so a view that
    /// embeds or dispatches to this one is filled where it will live. On
    /// `Err`, `_out` holds the fields decoded so far and is not to be read.
    pub fn decode_into(cursor: &mut SceCursor<'_>, _out: &mut Self) -> Result<(), CodecError> {
        // Decode fixed prefix (RFC §synth-5-B variant primitive: fields
        // sit before the variant suffix on the wire).
        let raw = cursor.peek_slice(1)?;
        let msg_id = raw[0];
        cursor.advance(1)?;
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        // Each arm is installed in `_out.body` holding its body's default and
        // that body is decoded where it sits, so no arm body is returned up
        // through the variant wrapper.
        match msg_id {
            1u8 => {
                _out.body = CodecVariantDispatchViewVariant::CodecVariantSessionOpen(CodecVariantSessionOpenView::default());
                if let CodecVariantDispatchViewVariant::CodecVariantSessionOpen(_b) = &mut _out.body {
                    CodecVariantSessionOpenView::decode_into(cursor, _b)?;
                }
            }
            2u8 => {
                _out.body = CodecVariantDispatchViewVariant::CodecVariantSessionClose(CodecVariantSessionCloseView::default());
                if let CodecVariantDispatchViewVariant::CodecVariantSessionClose(_b) = &mut _out.body {
                    CodecVariantSessionCloseView::decode_into(cursor, _b)?;
                }
            }
            other => {
                _out.body = CodecVariantDispatchViewVariant::Default {
                    tag: CodecVariantDispatchUnknownTag(other),
                    body: CodecVariantSessionCloseView::default(),
                };
                if let CodecVariantDispatchViewVariant::Default { body: _b, .. } = &mut _out.body {
                    CodecVariantSessionCloseView::decode_into(cursor, _b)?;
                }
            }
        }
        _out.msg_id = msg_id;
        Ok(())
    }

    /// Fill `_out`, the builder form [`CodecVariantDispatch`] a sender fills and
    /// encodes, with what this view says: every field, each embedded body, the
    /// variant arm, and each list entry as the walk yields it, built in the
    /// destination where it will live. Fails only where the destination cannot
    /// hold it, which a destination of the same declared bounds does.
    // The name every view's method has (ADR 0049), which borrows because a
    // view is read, not consumed. Clippy leaves an exported method's name
    // alone; a crate that includes this module privately is asked to rename
    // it, and the one name is what lets a codec reach every view it holds.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_builder(&self, _out: &mut CodecVariantDispatch) -> Result<(), CodecError> {
        _out.msg_id = self.msg_id;
        match &self.body {
            CodecVariantDispatchViewVariant::CodecVariantSessionOpen(_b) => {
                _out.body = CodecVariantDispatchVariant::CodecVariantSessionOpen(CodecVariantSessionOpen::default());
                if let CodecVariantDispatchVariant::CodecVariantSessionOpen(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecVariantDispatchViewVariant::CodecVariantSessionClose(_b) => {
                _out.body = CodecVariantDispatchVariant::CodecVariantSessionClose(CodecVariantSessionClose::default());
                if let CodecVariantDispatchVariant::CodecVariantSessionClose(_o) = &mut _out.body {
                    _b.into_builder(_o)?;
                }
            }
            CodecVariantDispatchViewVariant::Default { tag, body } => {
                _out.body = CodecVariantDispatchVariant::Default {
                    tag: *tag,
                    body: CodecVariantSessionClose::default(),
                };
                if let CodecVariantDispatchVariant::Default { body: _o, .. } = &mut _out.body {
                    body.into_builder(_o)?;
                }
            }
        }
        Ok(())
    }

    /// Worst-case encoded byte count for this codec, as
    /// [`CodecVariantDispatch::MAX_ENCODED_BYTES`].
    pub const MAX_ENCODED_BYTES: usize = 3;

    /// Encode what this view says into the caller-owned sink: the bytes
    /// [`CodecVariantDispatch::encode`] writes for the value [`Self::into_builder`]
    /// fills, written straight from the view, walking each list, without the
    /// builder. It writes the canonical wire, not the frame the view was decoded
    /// from, so a frame `decode` accepted in a form that is not canonical is
    /// not passed on as it came.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        CodecVariantDispatch::encode_source(self, w)
    }

    /// Heap-backed convenience facade over [`Self::encode`], which refuses no
    /// value (docs/adr/0065).
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let Ok(()) = self.encode(&mut VecSink::new(&mut _sce_v));
        _sce_v
    }
}
