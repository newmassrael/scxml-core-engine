#![doc = "SCE-MAP: codec_chain_has_tagged:11 :: _forge_body"]
// SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

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

use super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntry;

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CodecChainHasTagged {
    pub header: u8,
    pub entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<HeaplessVec<CodecChainHasTaggedEntry, 3>>,
}

#[allow(dead_code)]
impl CodecChainHasTagged {
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
        let mut entries: ::sce_forge_runtime::codec::Bounded<HeaplessVec<CodecChainHasTaggedEntry, 3>, 3, 1> = Default::default();
        entries.refill(|_vec| {
            let mut _more = false;
            for _ in 0..3u32 {
                let _entry = CodecChainHasTaggedEntry::decode(cursor)?;
                _more = _entry.more();
                // The loop walks the chain's depth and `_vec` holds the capacity, so
                // this push fails only where the deployment holds fewer entries than
                // the frame says. An over-long chain is refused after the loop.
                _vec.push(_entry).map_err(|_| CodecError::CapacityExceeded)?;
                if !_more { break; }
            }
            if _more && cursor.remaining() == 0 {
                return Err(CodecError::NeedMoreBytes);
            }
            if _more {
                return Err(CodecError::TlvChainOverflow);
            }
            Ok(())
        })?;
        let _has_entries_7 = entries.iter().any(|_e| u64::from(_e.entry_type) == 7u64);
        let _has_entries_9 = entries.iter().any(|_e| u64::from(_e.entry_type) == 9u64);
        let priority = if _has_entries_7 || (header & 0x01u8) != 0 {
            let raw = cursor.peek_slice(1)?;
            let _v = raw[0];
            cursor.advance(1)?;
            Some(_v)
        } else {
            None
        };
        let checksum = if !_has_entries_9 {
            let raw = cursor.peek_slice(2)?;
            let _v = ((raw[0] as u16) << 8) | raw[1] as u16;
            cursor.advance(2)?;
            Some(_v)
        } else {
            None
        };
        _out.header = header;
        _out.entries_has_0x7_has_0x9_header_wide = match (_has_entries_7, _has_entries_9, (header & 0x01u8) != 0) { (true, true, true) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9Wide { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says"), priority: priority.unwrap_or_default() }, (true, true, false) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9 { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says"), priority: priority.unwrap_or_default() }, (true, false, true) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Wide { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says"), priority: priority.unwrap_or_default(), checksum: checksum.unwrap_or_default() }, (false, true, true) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9Wide { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says"), priority: priority.unwrap_or_default() }, (true, false, false) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7 { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says"), priority: priority.unwrap_or_default(), checksum: checksum.unwrap_or_default() }, (false, true, false) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9 { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says") }, (false, false, true) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Wide { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says"), priority: priority.unwrap_or_default(), checksum: checksum.unwrap_or_default() }, (false, false, false) => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Neither { entries: ::sce_forge_runtime::codec::Chain::new(entries.into_inner()).expect("the gates found the chain's kinds as the arm says"), checksum: checksum.unwrap_or_default() }, };
        Ok(())
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as bool; multi-bit
    // (width>=2) reads as the smallest unsigned integer that fits the
    // range. Setters mask + shift on the way in so out-of-range
    // callers can't corrupt sibling bits. Wire layout is unchanged —
    // the carrier still occupies its declared bytes.
    pub fn wide(&self) -> bool {
        (self.header & 0x01) != 0
    }

    pub fn set_wide(&mut self, v: bool) {
        if v {
            self.header |= 0x01;
        } else {
            self.header &= !0x01;
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VecSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SliceSink` allocations.
    pub const MAX_ENCODED_BYTES: usize = 14;

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

/// The fields `entries.has(0x7)`, `entries.has(0x9)`, `header.wide` decide (docs/adr/0065 §6 R4b, R4c): a value holds the
/// ones of one arm, and `CodecChainHasTagged` derives the gates from which.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<Entries> {
    /// While `entries.has(0x7)` holds and `entries.has(0x9)` holds and `header.wide` holds.
    Has0x7Has0x9Wide {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key), (), 3, 1>,
        /// The `priority` field.
        priority: u8,
    },
    /// While `entries.has(0x7)` holds and `entries.has(0x9)` holds and `header.wide` does not hold.
    Has0x7Has0x9 {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key), (), 3, 1>,
        /// The `priority` field.
        priority: u8,
    },
    /// While `entries.has(0x7)` holds and `entries.has(0x9)` does not hold and `header.wide` holds.
    Has0x7Wide {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (CodecChainHasTaggedEntriesHas0x7Key,), (CodecChainHasTaggedEntriesHas0x9Key,), 3, 1>,
        /// The `priority` field.
        priority: u8,
        /// The `checksum` field.
        checksum: u16,
    },
    /// While `entries.has(0x7)` does not hold and `entries.has(0x9)` holds and `header.wide` holds.
    Has0x9Wide {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (CodecChainHasTaggedEntriesHas0x9Key,), (CodecChainHasTaggedEntriesHas0x7Key,), 3, 1>,
        /// The `priority` field.
        priority: u8,
    },
    /// While `entries.has(0x7)` holds and `entries.has(0x9)` does not hold and `header.wide` does not hold.
    Has0x7 {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (CodecChainHasTaggedEntriesHas0x7Key,), (CodecChainHasTaggedEntriesHas0x9Key,), 3, 1>,
        /// The `priority` field.
        priority: u8,
        /// The `checksum` field.
        checksum: u16,
    },
    /// While `entries.has(0x7)` does not hold and `entries.has(0x9)` holds and `header.wide` does not hold.
    Has0x9 {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (CodecChainHasTaggedEntriesHas0x9Key,), (CodecChainHasTaggedEntriesHas0x7Key,), 3, 1>,
    },
    /// While `entries.has(0x7)` does not hold and `entries.has(0x9)` does not hold and `header.wide` holds.
    Wide {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (), (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key), 3, 1>,
        /// The `priority` field.
        priority: u8,
        /// The `checksum` field.
        checksum: u16,
    },
    /// While `entries.has(0x7)` does not hold and `entries.has(0x9)` does not hold and `header.wide` does not hold.
    Neither {
        /// The `entries` field.
        entries: ::sce_forge_runtime::codec::Chain<Entries, (), (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key), 3, 1>,
        /// The `checksum` field.
        checksum: u16,
    },
}

impl<Entries> Default for CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<Entries>
where
    ::sce_forge_runtime::codec::Bounded<Entries, 3, 1>: Default,
    Entries: ::sce_forge_runtime::codec::Entries + ::sce_forge_runtime::codec::OfKind<CodecChainHasTaggedEntriesHas0x7Key> + ::sce_forge_runtime::codec::OfKind<CodecChainHasTaggedEntriesHas0x9Key>,
{
    /// The chain's default, in the arm its entries' kinds select with every
    /// other gate clear, each field at its type's default.
    fn default() -> Self {
        let _list = <::sce_forge_runtime::codec::Bounded<Entries, 3, 1> as Default>::default().into_inner();
        match (<Entries as ::sce_forge_runtime::codec::OfKind<CodecChainHasTaggedEntriesHas0x7Key>>::first_of_kind(&_list).is_some(), <Entries as ::sce_forge_runtime::codec::OfKind<CodecChainHasTaggedEntriesHas0x9Key>>::first_of_kind(&_list).is_some(),) {
            (true, true,) => Self::Has0x7Has0x9 { entries: ::sce_forge_runtime::codec::Chain::new(_list).expect("the default entries are of the kinds found"), priority: Default::default() },
            (true, false,) => Self::Has0x7 { entries: ::sce_forge_runtime::codec::Chain::new(_list).expect("the default entries are of the kinds found"), priority: Default::default(), checksum: Default::default() },
            (false, true,) => Self::Has0x9 { entries: ::sce_forge_runtime::codec::Chain::new(_list).expect("the default entries are of the kinds found") },
            (false, false,) => Self::Neither { entries: ::sce_forge_runtime::codec::Chain::new(_list).expect("the default entries are of the kinds found"), checksum: Default::default() },
        }
    }
}

/// The kind of entry `entries.has(0x7)` names: an entry whose identifier is
/// 0x7, which a value holds where it stands (docs/adr/0065 §6 R6b).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecChainHasTaggedEntriesHas0x7Key;

impl<E: super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> ::sce_forge_runtime::codec::Key<E> for CodecChainHasTaggedEntriesHas0x7Key {
    fn matches(entry: &E) -> bool {
        u64::from(super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource::entry_type(entry)) == 7u64
    }
}

/// The kind of entry `entries.has(0x9)` names: an entry whose identifier is
/// 0x9, which a value holds where it stands (docs/adr/0065 §6 R6b).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecChainHasTaggedEntriesHas0x9Key;

impl<E: super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> ::sce_forge_runtime::codec::Key<E> for CodecChainHasTaggedEntriesHas0x9Key {
    fn matches(entry: &E) -> bool {
        u64::from(super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource::entry_type(entry)) == 9u64
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `CodecChainHasTagged` says: its presented fields. A length, a count
/// or the flag a gated field is announced by is not among them; `CodecChainHasTagged`
/// computes those from what is ([`CodecChainHasTagged::encode_source`]).
pub trait CodecChainHasTaggedSource {
    /// The `header` field.
    fn header(&self) -> u8;
    /// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides, held together (docs/adr/0065 §6 R4).
    fn entries_has_0x7_has_0x9_header_wide(&self) -> CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3>>;
}

impl<T: CodecChainHasTaggedSource + ?Sized> CodecChainHasTaggedSource for &T {
    fn header(&self) -> u8 {
        (**self).header()
    }
    fn entries_has_0x7_has_0x9_header_wide(&self) -> CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3>> {
        (**self).entries_has_0x7_has_0x9_header_wide()
    }
}

/// A value of `CodecChainHasTagged` held in the types a caller already has: a source
/// ([`CodecChainHasTaggedSource`]) whose lists are slices within their bounds and whose
/// bodies are references.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodecChainHasTaggedParts<'a> {
    /// The `header` field.
    pub header: u8,
    /// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides, held together (docs/adr/0065 §6 R4).
    pub entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<&'a [super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntryParts]>,
}

impl<'a> CodecChainHasTaggedSource for CodecChainHasTaggedParts<'a> {
    fn entries_has_0x7_has_0x9_header_wide(&self) -> CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3>> {
        match &self.entries_has_0x7_has_0x9_header_wide {
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9Wide { entries, priority } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9Wide { entries: entries.iter(), priority: *priority },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9 { entries, priority } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9 { entries: entries.iter(), priority: *priority },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Wide { entries, priority, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Wide { entries: entries.iter(), priority: *priority, checksum: *checksum },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9Wide { entries, priority } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9Wide { entries: entries.iter(), priority: *priority },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7 { entries, priority, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7 { entries: entries.iter(), priority: *priority, checksum: *checksum },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9 { entries } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9 { entries: entries.iter() },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Wide { entries, priority, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Wide { entries: entries.iter(), priority: *priority, checksum: *checksum },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Neither { entries, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Neither { entries: entries.iter(), checksum: *checksum },
        }
    }
    fn header(&self) -> u8 {
        self.header
    }
}

impl CodecChainHasTaggedSource for CodecChainHasTagged {
    fn entries_has_0x7_has_0x9_header_wide(&self) -> CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<::sce_forge_runtime::codec::BoundedIter<impl Iterator<Item = impl super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntrySource> + Clone, 3>> {
        match &self.entries_has_0x7_has_0x9_header_wide {
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9Wide { entries, priority } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9Wide { entries: entries.iter(), priority: *priority },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9 { entries, priority } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9 { entries: entries.iter(), priority: *priority },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Wide { entries, priority, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Wide { entries: entries.iter(), priority: *priority, checksum: *checksum },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9Wide { entries, priority } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9Wide { entries: entries.iter(), priority: *priority },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7 { entries, priority, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7 { entries: entries.iter(), priority: *priority, checksum: *checksum },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9 { entries } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9 { entries: entries.iter() },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Wide { entries, priority, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Wide { entries: entries.iter(), priority: *priority, checksum: *checksum },
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Neither { entries, checksum } => CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Neither { entries: entries.iter(), checksum: *checksum },
        }
    }
    fn header(&self) -> u8 {
        self.header
    }
}

#[allow(dead_code)]
impl CodecChainHasTagged {
    /// Encode what `src` says into the caller-owned sink: the bytes
    /// [`Self::encode`] writes for the value it holds, read through
    /// [`CodecChainHasTaggedSource`]. A length, a count or a gate flag is computed from what
    /// `src` presents. Every value `src` can present has a frame, so the
    /// only error is the sink's (docs/adr/0065).
    pub fn encode_source<S: CodecChainHasTaggedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the guard a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: CodecChainHasTaggedSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> {
        let (_m_entries, _m_priority, _m_checksum) = match src.entries_has_0x7_has_0x9_header_wide() {
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9Wide { entries, priority } => (Some(entries.into_inner()), Some(priority), None),
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9 { entries, priority } => (Some(entries.into_inner()), Some(priority), None),
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Wide { entries, priority, checksum } => (Some(entries.into_inner()), Some(priority), Some(checksum)),
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9Wide { entries, priority } => (Some(entries.into_inner()), Some(priority), None),
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7 { entries, priority, checksum } => (Some(entries.into_inner()), Some(priority), Some(checksum)),
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9 { entries } => (Some(entries.into_inner()), None, None),
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Wide { entries, priority, checksum } => (Some(entries.into_inner()), Some(priority), Some(checksum)),
            CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Neither { entries, checksum } => (Some(entries.into_inner()), None, Some(checksum)),
        };
        let _k_entries: usize = _m_entries.clone().map_or(0, |_l| _l.len());
        let _d_header_wide: bool = matches!(src.entries_has_0x7_has_0x9_header_wide(), CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Has0x9Wide { .. } | CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x7Wide { .. } | CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Has0x9Wide { .. } | CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide::Wide { .. });
        let _c_header: u8 = (src.header() & !0x1) | u8::from(_d_header_wide);
        w.write_u8(_c_header)?;
        if let Some(_v) = _m_entries.clone() {
            for (_i, _e) in _v.enumerate() {
                CodecChainHasTaggedEntry::write_source_in(&_e, w, &super::codec_chain_has_tagged_entry::CodecChainHasTaggedEntryOverrides::ctl_more(_i + 1 < _k_entries))?;
            }
        }
        if let Some(_v) = _m_priority {
            w.write_u8(_v)?;
        }
        if let Some(_v) = _m_checksum {
            w.write_u8((_v >> 8 & 0xFF) as u8)?;
            w.write_u8((_v & 0xFF) as u8)?;
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: CodecChainHasTaggedSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<S: CodecChainHasTaggedSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for CodecChainHasTagged {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error> where CodecError: From<K::Error> {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl CodecChainHasTagged {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: CodecChainHasTaggedSource + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: CodecChainHasTaggedSource + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
