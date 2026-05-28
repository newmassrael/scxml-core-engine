#![doc = "SCE-MAP: codec_zenoh_oam:56"]
// SCE-MAP: codec_zenoh_oam:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// RFC §5.B B1-α: `VecSink` and the heap-backed `encode_to_vec` facade
// are gated on the `alloc` feature (see
// `sce-forge-runtime/rust/src/codec.rs`). MCU / `no_std` consumers see
// only the sink-based primary `encode` + `SliceSink` paths.
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use sce_forge_runtime::codec::VecSink;
// RFC §5.B B2/B3: bounded inline list storage for repeat / tlv-chain
// fields — heap-free `heapless::Vec<T, N>` (re-exported by the runtime),
// the Rust mirror of the C11 `T elems[MAX]; len` representation. Always
// available (no `alloc` gate) so list-bearing codecs compile on the
// pure no_std no-alloc MCU tier.
use sce_forge_runtime::heapless::Vec as HeaplessVec;

use super::codec_zenoh_ext_entry::CodecZenohExtEntry;
use super::codec_zenoh_ext_unit::CodecZenohExtUnit;
use super::codec_zenoh_ext_zint::CodecZenohExtZint;
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbuf;

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
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

// RFC variant-default-uniformity Atomic β: at least one field's
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
    /// Construct an instance with every field zero-initialized via
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519).
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
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
        let extensions = if (header & 0x80u8) != 0 {
            let mut _vec: HeaplessVec<CodecZenohExtEntry<'a>, 4> = HeaplessVec::new();
            for _ in 0..4u32 {
                    if cursor.remaining() == 0 { break; }
                    let _entry = CodecZenohExtEntry::decode(cursor)?;
                    let _continue = _entry.z();
                    _vec.push(_entry).map_err(|_| CodecError::TooManyElements)?;
                    if !_continue { break; }
                }
            Some(_vec)
        } else {
            None
        };
        // Dispatch on the tag field; each arm decodes its body codec
        // from the cursor. The default arm (when declared) carries the
        // runtime tag value so encode can round-trip it back onto the
        // wire.
        let body = match ((header >> 5) & (0x03 as u8)) as u8 {
            0u8 => CodecZenohOamVariant::CodecZenohExtUnit(CodecZenohExtUnit::decode(cursor)?),
            1u8 => CodecZenohOamVariant::CodecZenohExtZint(CodecZenohExtZint::decode(cursor)?),
            2u8 => CodecZenohOamVariant::CodecZenohExtZbuf(CodecZenohExtZbuf::decode(cursor)?),
            other => CodecZenohOamVariant::Default {
                tag: other,
                body: CodecZenohExtUnit::decode(cursor)?,
            },
        };
        Ok(Self {
            header,
            id,
            extensions,
            body,
        })
    }

    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn mid(&self) -> u8 {
        (((self.header >> 0) & (0x1F as u8))) as u8
    }

    pub fn set_mid(&mut self, v: u8) {
        let _mask: u8 = (0x1F as u8) << 0;
        let _val: u8 = ((v as u8) & (0x1F as u8)) << 0;
        self.header = (self.header & !_mask) | _val;
    }

    pub fn enc(&self) -> u8 {
        (((self.header >> 5) & (0x03 as u8))) as u8
    }

    pub fn set_enc(&mut self, v: u8) {
        let _mask: u8 = (0x03 as u8) << 5;
        let _val: u8 = ((v as u8) & (0x03 as u8)) << 5;
        self.header = (self.header & !_mask) | _val;
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
    pub const MAX_ENCODED_BYTES: usize = 46;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `VecSink`) are effectively infallible.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode (per-field present_if/tlv-chain/embed/
        // repeat helpers). Peek-byte mode: the arm body's encode prepends
        // its own header byte (which the decoder peeked); no separate
        // tag byte is emitted here. Streaming-prefix mode (own-field
        // variant): the carrier is part of the prefix fields and emits
        // through the same per-field path.
        w.write_u8(self.header)?;
        {
            let mut _vle = self.id as u64;
            while _vle >= 0x80 {
                w.write_u8((_vle as u8 & 0x7F) | 0x80)?;
                _vle >>= 7;
            }
            w.write_u8(_vle as u8)?;
        }
        if let Some(_list) = &self.extensions {
            for _e in _list {
                _e.encode(w)?;
            }
        }
        // Append the active arm's encoded bytes.
        match &self.body {
            CodecZenohOamVariant::CodecZenohExtUnit(b) => {
                b.encode(w)?;
            }
            CodecZenohOamVariant::CodecZenohExtZint(b) => {
                b.encode(w)?;
            }
            CodecZenohOamVariant::CodecZenohExtZbuf(b) => {
                b.encode(w)?;
            }
            CodecZenohOamVariant::Default { body, .. } => {
                body.encode(w)?;
            }
        }
        Ok(())
    }

    /// Heap-backed convenience facade. Pre-reserves
    /// `MAX_ENCODED_BYTES` so the worst-case write path performs at
    /// most one allocation, then delegates to `encode` over a
    /// `VecSink`. Returns the freshly-encoded byte vector. Callers
    /// targeting zero-alloc hot paths should call `encode` directly
    /// against a caller-owned sink.
    ///
    /// Gated on the `alloc` feature — `VecSink` lives behind the
    /// same gate (see `sce-forge-runtime/rust/src/codec.rs`). MCU /
    /// `no_std` builds without `alloc` only see the sink-based
    /// primary `encode`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut _sce_v: Vec<u8> = Vec::with_capacity(Self::MAX_ENCODED_BYTES);
        let mut _sce_sink = VecSink::new(&mut _sce_v);
        self.encode(&mut _sce_sink)
            .expect("VecSink is infallible");
        _sce_v
    }
}

// ── Owned projection (consumer-requested; alloc-gated) ────────────────
// `CodecZenohOam<'a>` above is a zero-copy view borrowing the decode
// buffer. AP / async consumers that persist a decoded message beyond the
// buffer's lifetime call `.into_owned()` for this lifetime-free
// `CodecZenohOamOwned`. The rkyv-style Archived(borrowed) ↔ native
// (owned) split — both generated from the one SCXML source (SSOT). `Vec`
// / `String` are alloc, so the whole projection is gated; the no-alloc
// borrowed path above is untouched.
#[cfg(feature = "alloc")]
use super::codec_zenoh_ext_entry::CodecZenohExtEntryOwned;
#[cfg(feature = "alloc")]
use super::codec_zenoh_ext_zbuf::CodecZenohExtZbufOwned;
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct CodecZenohOamOwned {
    pub header: u8,
    pub id: u16,
    pub extensions: Option<Vec<CodecZenohExtEntryOwned>>,
    pub body: CodecZenohOamOwnedVariant,
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub enum CodecZenohOamOwnedVariant {
    CodecZenohExtUnit(CodecZenohExtUnit),
    CodecZenohExtZint(CodecZenohExtZint),
    CodecZenohExtZbuf(CodecZenohExtZbufOwned),
    Default {
        tag: u8,
        body: CodecZenohExtUnit,
    },
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohOamVariant<'a> {
    /// Deep-copy this borrowed variant body into its owned mirror.
    pub fn into_owned(self) -> CodecZenohOamOwnedVariant {
        match self {
            CodecZenohOamVariant::CodecZenohExtUnit(_b) => CodecZenohOamOwnedVariant::CodecZenohExtUnit(_b),
            CodecZenohOamVariant::CodecZenohExtZint(_b) => CodecZenohOamOwnedVariant::CodecZenohExtZint(_b),
            CodecZenohOamVariant::CodecZenohExtZbuf(_b) => CodecZenohOamOwnedVariant::CodecZenohExtZbuf(_b.into_owned()),
            CodecZenohOamVariant::Default { tag, body } => CodecZenohOamOwnedVariant::Default { tag, body },
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> CodecZenohOam<'a> {
    /// Deep-copy this borrowed zero-copy view into an owned, lifetime-free
    /// [`CodecZenohOamOwned`] (alloc). Call at a decode boundary when
    /// the decoded value must outlive the input buffer — e.g. stored in a
    /// long-lived enum or moved across an async task. The no-alloc
    /// borrowed path is unaffected; this method exists only under
    /// `feature = "alloc"`.
    pub fn into_owned(self) -> CodecZenohOamOwned {
        CodecZenohOamOwned {
            header: self.header,
            id: self.id,
            extensions: self.extensions.map(|_v| _v.into_iter().map(|_e| _e.into_owned()).collect()),
            body: self.body.into_owned(),
        }
    }
}
