// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Codec cursor + typed error contract for `sce:kind="codec"` decode bodies.
//!
//! RFC `docs/spec/synth/rfc-sce-protocol-synthesis.md` §synth-5-B L494-519
//! pins a per-language cursor + Result/Option shape on decode so a
//! truncated input never aborts — it returns `NeedMoreBytes` and the
//! caller resumes after additional bytes arrive (DMA boundary,
//! fragmented network read).
//!
//! The cursor ships `peek_slice` (non-advancing), `advance`
//! (post-success), `remaining`, plus the `read_vle_*` stream-style
//! readers. Other stream-style readers (e.g. a dedicated `read_u8`,
//! `read_tag`, `skip_field`) are not provided until a consumer needs
//! them.
//!
//! Encode-side sink + `BufferOverflow` is the symmetric write half of
//! the cursor contract: `SceSink` is the object-safe trait that codec
//! `encode` bodies emit into; `VecSink` (heap-backed, infallible) and
//! `SliceSink` (caller-owned `&mut [u8]`, raises `BufferOverflow` at
//! the cap) are the two concrete impls shipped here. Caller owns the
//! destination storage, mirroring the borrow contract on the decode
//! cursor side.

/// Typed decode error. The enum is `#[non_exhaustive]` to keep
/// additive variants from breaking downstream `match` arms before
/// SCE 1.0.
///
/// The variant primitive intentionally does NOT need a typed
/// `UnknownVariantTag` variant — RFC §synth-5-B requires `<sce:default>` when
/// arms don't exhaust the tag domain (`codec/variant-arm-unreachable`
/// fires at build time otherwise), so the default arm catches every
/// unmatched tag at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecError {
    /// The cursor's remaining buffer is shorter than the codec's
    /// declared minimum frame. Caller should resume after appending
    /// more bytes.
    NeedMoreBytes,
    /// A `vle_u<N>` field's continuation chain implies a value wider
    /// than the declared type. Either the wire is corrupt or the
    /// author chose a too-narrow type. RFC §synth-5-B `codec/vle-width-overflow`.
    VleWidthOverflow,
    /// RFC §synth-5-B TLV chain primitive: the wire carried more entries
    /// than the codec author declared (`max-depth=N` exhausted while
    /// the cursor still had bytes) AND the codec declared
    /// `on-overflow="reject"`. Truncate-mode codecs never raise this
    /// — they silently drop the post-cap bytes. MCU-class symbol; the
    /// other 4 backends never construct it (their codec emit is
    /// rejected upfront by the codec-content MCU gate, RFC §synth-5-B "MCU-
    /// only codec sub-features").
    TlvChainOverflow,
    /// RFC §synth-5-B string primitive: a `sce:type="string"` length-prefixed
    /// field's payload bytes were not valid UTF-8. Forge-fail-fast
    /// contract — zenoh-pico itself aliases the bytes without
    /// validating, but SCE-side codecs reject malformed text early so
    /// downstream procedures never see a malformed `String` /
    /// `std::string`. The Rust + Go + Python runtimes construct this
    /// at decode; Cpp + Kotlin collapse to the truncation sentinel
    /// (`std::nullopt` / `null`) instead — those backends never
    /// construct typed `CodecError` variants at runtime, mirroring the
    /// existing VleWidthOverflow declaration-only convention.
    InvalidUtf8,
    /// Encode-side counterpart to `NeedMoreBytes`: the destination sink
    /// reported insufficient remaining capacity for the next write.
    /// Only the bounded `SliceSink` (caller-owned `&mut [u8]`) can
    /// raise this; the heap-backed `VecSink` grows on demand and is
    /// effectively infallible. Codec authors decide per call site
    /// whether the destination is bounded; a fixed-frame on-wire codec
    /// driving a DMA bounce buffer will run on `SliceSink` and surface
    /// overflow as a typed error rather than aborting.
    BufferOverflow,
    /// RFC §synth-5-B repeat / TLV chain primitive: the wire carried
    /// more elements than the codec's declared `sce:max-count` bound
    /// (the fixed-capacity `heapless::Vec<Body, MAX_COUNT>` backing the
    /// list field is full). The no-alloc list representation stores
    /// elements in bounded inline storage rather than a growable heap
    /// `Vec`, so an over-long wire run surfaces as this typed error
    /// instead of an allocation. Authors raise `sce:max-count` to admit
    /// longer runs (trading inline footprint for capacity). Decode-side
    /// symbol; the Cpp/Kotlin truncation backends never construct it
    /// (their list storage is the heap-backed host container).
    TooManyElements,
    /// An enum-typed field carried a value its declared set does not
    /// hold. `sce:strict-variants="true"` (the default) closes the set,
    /// so a carrier value outside it is not a value of that type and
    /// the frame does not decode. An open set (`sce:strict-variants=
    /// "false"`) carries every value of its carrier and never raises
    /// this. The Rust + Go + Python runtimes construct it at decode;
    /// Cpp + Kotlin collapse to the truncation sentinel
    /// (`std::nullopt` / `null`), as they do for `InvalidUtf8`.
    UndeclaredEnumValue,
    /// A `sce:encoding="cbor"` codec (SCE_FORGE.md §4.6.1) read bytes that
    /// are not the CBOR it expects: a head it cannot read (reserved
    /// additional information, an indefinite length), a value of another
    /// major type than the entry declares, a key that is not an unsigned
    /// integer, or a key given twice.
    CborMalformed,
    /// A CBOR codec's map lacked an entry declared `sce:required="true"`.
    CborRequiredKeyMissing,
    /// A CBOR codec's `bytes` entry declared `sce:length` held another
    /// number of bytes — on decode, or on encode.
    CborWrongLength,
    /// A CBOR codec skipped an unknown entry nested deeper than the bound
    /// (SCE_FORGE.md §4.6.1: 16) — the defence against a map built to
    /// exhaust the reader's stack.
    CborTooDeep,
    /// A CBOR codec read a value its entry's width cannot hold, or a
    /// `string` / `bytes` longer than its `sce:max-size`.
    CborOutOfRange,
    /// Encode-side: a field gated by a chain-membership `sce:present-if`
    /// (`<chain>.has(<value>)`) was given when the predicate says it does
    /// not belong on the wire, or left out when it does. The chain and the
    /// field are two descriptions of one wire, and they contradict each
    /// other; the encoder refuses before it writes the first byte, so the
    /// sink is untouched. Raised only by a codec some chain-membership
    /// predicate gates. The C11 backend never raises it: a C11 field carries
    /// no "given" marker, so there the predicate alone decides what is
    /// written.
    PresentIfMismatch,
    /// A `sce:encoding="content-line"` codec (SCE_FORGE.md §4.6.4) read a
    /// line, a parameter or an `END:` the grammar does not admit.
    LineMalformed,
    /// A content-line component lacked a property or parameter declared
    /// `sce:required="true"`; on encode, a parameter was given without the
    /// property it belongs to.
    LineRequiredMissing,
    /// A content-line property that holds one value occurred twice, a
    /// parameter was given twice in one line, or a list passed its
    /// `sce:max-count`.
    LineTooMany,
    /// A content-line value passed its `sce:max-size`.
    LineTooLong,
    /// A content-line TEXT carried an escape other than `\\`, `\;`, `\,`,
    /// `\n` and `\N`.
    LineBadEscape,
    /// A content-line value its entry cannot hold: a control character,
    /// invalid UTF-8, an integer or `bool` out of its type, a parameter of more
    /// values than one, a `"` in a parameter value.
    LineBadValue,
    /// Encode-side (docs/adr/0058): a length or a count the encoding derives from
    /// what a source presents — the bytes a `sce:length-field` counts, the
    /// entries of a `<sce:repeat count=>` — does not fit the field that carries
    /// it, or would be negative once the field's `sce:length-arith` is applied.
    /// Refused before the first byte is written.
    LengthOutOfRange,
    /// Encode-side (docs/adr/0058): two fields of a source that the codec
    /// derives one number from disagree in size — a count shared by two lists
    /// (`links` and the gated `weights` of a link-state), a length shared by two
    /// fields. The wire has room for one number, so no frame says both.
    /// Refused before the first byte is written.
    InconsistentSource,
}

/// Project an owned list into the bounded inline list
/// (`heapless::Vec<U, N>`) the borrowed codec view spells, applying `f` to
/// each element by reference.
///
/// The owned mirror of a bounded list (`<sce:repeat>` / `<sce:tlv-chain>`)
/// holds its elements in whatever container its storage profile chose, and a
/// growable profile can hold more than `N` of them; the owned→borrowed
/// projection (`{Codec}Owned::try_as_borrowed`) therefore raises
/// [`CodecError::TooManyElements`] past the bound — the same bound and error
/// the decode path enforces, keeping the system invariant `<= N` end-to-end
/// (encode must never emit wire its own decoder would reject). On a
/// fixed-capacity profile the source is already within `N`, so the check
/// costs a comparison and cannot fire.
///
/// SSOT for the owned→borrowed bounded-list step: every generated
/// `try_as_borrowed` calls this rather than open-coding the capacity loop,
/// so the bound semantics live in exactly one place. Taking the list itself
/// (rather than a slice) keeps reading the profile's container here too, so
/// the call site never names the container's read surface. `f` is fallible
/// so a nested fallible element projection (`_e.try_as_borrowed()`) threads
/// its own error through; an infallible element projection passes
/// `|_e| Ok(...)`.
///
/// The inverse direction (borrowed→owned) is
/// [`crate::storage::try_collect_list`], whose destination container is the
/// profile's rather than always inline.
pub fn try_project_bounded<'s, L, T, U, const N: usize>(
    src: &'s L,
    f: impl FnMut(&'s T) -> Result<U, CodecError>,
) -> Result<crate::heapless::Vec<U, N>, CodecError>
where
    L: crate::storage::SceList<T>,
    T: 's,
{
    let mut out = crate::heapless::Vec::new();
    try_project_bounded_into(&mut out, src, f)?;
    Ok(out)
}

/// [`try_project_bounded`] into a list the caller already holds, replacing
/// its contents.
///
/// A `heapless::Vec` moves by copying its whole declared capacity, so the
/// value-returning projector hands the list back through every layer between
/// the projection and the encode — the `Result`, the optional wrapper, the
/// borrowed struct, the variant — copying the declared maximum at each,
/// however few entries the list holds. Filling the destination in place is
/// what `try_as_borrowed_into` is built on, and the value-returning form is
/// this one over a fresh list, so the bound semantics stay in one place.
///
/// On `Err` the destination holds the entries projected so far.
pub fn try_project_bounded_into<'s, L, T, U, const N: usize>(
    out: &mut crate::heapless::Vec<U, N>,
    src: &'s L,
    mut f: impl FnMut(&'s T) -> Result<U, CodecError>,
) -> Result<(), CodecError>
where
    L: crate::storage::SceList<T>,
    T: 's,
{
    // `&'s T` (not a fresh higher-ranked `&T`) ties each element's borrow
    // to the list's lifetime, so a projected `U` that re-borrows the
    // element (`_e.as_borrowed() -> Body<'s>`) is nameable — the HRTB form
    // cannot express "return type borrows the closure argument".
    out.clear();
    for item in src.as_slice() {
        out.push(f(item)?)
            .map_err(|_| CodecError::TooManyElements)?;
    }
    Ok(())
}

/// The entries of an owned list, refused past the bound the borrowed view
/// declares for it.
///
/// An owned codec encodes straight from its own fields, without projecting
/// into the borrowed view first. That projection (`try_project_bounded_into`)
/// is where a list longer than the borrowed `heapless::Vec<_, N>` can hold was
/// refused with [`CodecError::TooManyElements`], and a growable profile can
/// hold more than `N` (an inline one is sized to its own advisory capacity,
/// which may exceed it). Encoding from the owned fields has to raise the
/// same error, or a value the borrowed view could not carry would be written
/// as a frame its own decoder would reject; this is that check, in one place
/// for every generated owned `encode`.
pub fn bounded_entries<L, T, const N: usize>(list: &L) -> Result<&[T], CodecError>
where
    L: crate::storage::SceList<T>,
{
    let entries = list.as_slice();
    if entries.len() > N {
        return Err(CodecError::TooManyElements);
    }
    Ok(entries)
}

// ── Owned storage profiles ───────────────────────────────────────
//
// A codec's borrowed view decodes `bytes` / `string` / list fields as
// zero-copy `&'a [u8]` / `&'a str` / `heapless::Vec<_, N>`; its lifetime-free
// owned mirror needs containers that hold those by value. *Which* containers
// is a policy the owner picks per value rather than a property of the build,
// so the mirror is `{Codec}Owned<S>` over a [`crate::storage::CodecStorage`]
// profile — see that module for why the profile is a type parameter and not a
// second generated type. Re-exported here so generated codec output spells
// one module path for the cursor, the sink and the storage policy alike.
pub use crate::storage::*;
// The list of a generated view, spelled from the same path as the rest.
pub use crate::view::{ListIter, ListView, ListWalk};

impl From<sce_portable_bytes::CapacityExceeded> for CodecError {
    /// A fixed-capacity `from_slice` past its bound is the same
    /// bounded-storage overflow the decode path reports — surface it as the
    /// existing typed variant so callers match one error.
    fn from(_: sce_portable_bytes::CapacityExceeded) -> Self {
        CodecError::TooManyElements
    }
}

/// Read-only cursor over a borrowed input slice. Decode bodies use
/// `peek_slice` to bounds-check + read fixed-offset bytes positionally,
/// then `advance` after the construction succeeds.
#[derive(Debug, Clone, Copy)]
pub struct SceCursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> SceCursor<'a> {
    /// Wrap `buf` with cursor position 0.
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Bytes remaining from current position to end of input.
    pub const fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Borrow the next `n` bytes without advancing. Returns
    /// `NeedMoreBytes` when the cursor's tail is shorter than `n`.
    pub fn peek_slice(&self, n: usize) -> Result<&'a [u8], CodecError> {
        if self.remaining() < n {
            Err(CodecError::NeedMoreBytes)
        } else {
            Ok(&self.buf[self.pos..self.pos + n])
        }
    }

    /// Advance the cursor by `n` bytes. Decode bodies call this after
    /// the per-field expressions on the peeked slice have succeeded.
    /// Returns `NeedMoreBytes` if `n` would overrun the buffer (caller
    /// programming error in normal use; surfaced as the typed wire
    /// error to keep the contract symmetric).
    pub fn advance(&mut self, n: usize) -> Result<(), CodecError> {
        if self.remaining() < n {
            Err(CodecError::NeedMoreBytes)
        } else {
            self.pos += n;
            Ok(())
        }
    }

    /// Read a base-128 variable-length encoded unsigned value of up to
    /// `max_bits` payload width. The leading bytes carry 7 data bits in
    /// their low 7 with bit 7 as the continuation flag (1 = more bytes
    /// follow); the final byte (at shift `7 * (VLE_LEN - 1)`) carries a
    /// full 8 data bits with no continuation flag. LSB-first byte order.
    /// This is the canonical Zenoh ZInt wire format (RFC §synth-5-B
    /// Appendix B): a W-bit value occupies at most `ceil((W-1)/7)` bytes,
    /// so a u64 caps at 9 bytes — bit 63 rides in the 9th byte's high
    /// bit rather than spilling into a 10th byte, matching `zenoh` /
    /// `zenoh-pico`.
    ///
    /// Returns `VleWidthOverflow` when the final byte of a sub-octet
    /// width (the u16 / u32 tail) carries more bits than `max_bits`
    /// allows (the wire is corrupt or a too-narrow `vle_u<N>` was used).
    fn read_vle_inner(&mut self, max_bits: u32) -> Result<u64, CodecError> {
        let vle_len = (max_bits - 1).div_ceil(7);
        let final_shift = 7 * (vle_len - 1);
        let mut value: u64 = 0;
        let mut shift: u32 = 0;
        for _ in 0..vle_len {
            let b = self.peek_slice(1)?[0];
            self.advance(1)?;
            if shift == final_shift {
                // Final byte: 8 data bits, the continuation bit reused as
                // data. For a width narrower than the byte (u16 / u32
                // tail) refuse a value that would overflow the remaining
                // bits.
                let allowed = max_bits - shift;
                if allowed < 8 && (b as u64) > (1u64 << allowed) - 1 {
                    return Err(CodecError::VleWidthOverflow);
                }
                value |= (b as u64) << shift;
                return Ok(value);
            }
            value |= ((b & 0x7F) as u64) << shift;
            if (b & 0x80) == 0 {
                return Ok(value);
            }
            shift += 7;
        }
        // The loop's final iteration always satisfies `shift ==
        // final_shift` and returns; defensive unreachable for the type.
        Err(CodecError::VleWidthOverflow)
    }

    /// Read a `vle_u16` field (1-3 wire bytes).
    pub fn read_vle_u16(&mut self) -> Result<u16, CodecError> {
        self.read_vle_inner(16).map(|v| v as u16)
    }

    /// Read a `vle_u32` field (1-5 wire bytes).
    pub fn read_vle_u32(&mut self) -> Result<u32, CodecError> {
        self.read_vle_inner(32).map(|v| v as u32)
    }

    /// Read a `vle_u64` field (1-9 wire bytes). Canonical Zenoh ZInt.
    pub fn read_vle_u64(&mut self) -> Result<u64, CodecError> {
        self.read_vle_inner(64)
    }
}

// ── Write-side sink ──────────────────────────────────────────────

/// Write-side cursor for codec emit. Object-safe write surface that
/// generated `encode` bodies accept.
///
/// `SceSink` mirrors the read-side `SceCursor` borrow contract: callers
/// own the destination storage, and codec bodies append bytes to it
/// positionally without owning the buffer. The required method writes
/// raw bytes; per-width helpers (`write_u8`, `write_u16_le`, …) are
/// provided as default methods so concrete sinks MAY override them for
/// efficiency without changing the call surface.
///
/// Object safety: no `Self` in return positions, no generics on trait
/// methods, no associated types. Callers may coerce a concrete sink to
/// `&mut dyn SceSink` when monomorphization cost is not desired.
pub trait SceSink {
    /// Append `bytes` to the underlying storage. Concrete sinks raise
    /// `CodecError::BufferOverflow` when the destination has
    /// insufficient remaining capacity; growable sinks return `Ok(())`.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError>;

    /// Bytes written by this sink instance since it wrapped the
    /// destination. Used by codec emit for offset-aware writes (DMA-
    /// aligned field padding, length-prefix back-patching). Distinct
    /// from "total bytes in destination" — a `VecSink` over a Vec
    /// that already had bytes returns the delta, not the absolute
    /// length, so codec emit positional math operates within its own
    /// encoding regardless of coalesced-send prefix state.
    fn position(&self) -> usize;

    /// Append a single byte. Default impl forwards to `write_bytes`.
    fn write_u8(&mut self, b: u8) -> Result<(), CodecError> {
        self.write_bytes(&[b])
    }

    /// Append a little-endian `u16` (2 bytes).
    fn write_u16_le(&mut self, v: u16) -> Result<(), CodecError> {
        self.write_bytes(&v.to_le_bytes())
    }

    /// Append a big-endian `u16` (2 bytes).
    fn write_u16_be(&mut self, v: u16) -> Result<(), CodecError> {
        self.write_bytes(&v.to_be_bytes())
    }

    /// Append a little-endian `u32` (4 bytes).
    fn write_u32_le(&mut self, v: u32) -> Result<(), CodecError> {
        self.write_bytes(&v.to_le_bytes())
    }

    /// Append a big-endian `u32` (4 bytes).
    fn write_u32_be(&mut self, v: u32) -> Result<(), CodecError> {
        self.write_bytes(&v.to_be_bytes())
    }

    /// Append a little-endian `u64` (8 bytes).
    fn write_u64_le(&mut self, v: u64) -> Result<(), CodecError> {
        self.write_bytes(&v.to_le_bytes())
    }

    /// Append a big-endian `u64` (8 bytes).
    fn write_u64_be(&mut self, v: u64) -> Result<(), CodecError> {
        self.write_bytes(&v.to_be_bytes())
    }

    /// Append `value` as a base-128 VLE of `max_bits` payload width.
    /// The write-side counterpart of [`SceCursor::read_vle_inner`]: the
    /// leading bytes carry 7 data bits with bit 7 as the continuation
    /// flag, and the final byte (after at most `VLE_LEN - 1`
    /// continuation bytes) carries a full 8 data bits with no flag, so a
    /// u64 caps at 9 bytes — the canonical Zenoh ZInt form (RFC
    /// §synth-5-B Appendix B). `VLE_LEN = ceil((max_bits - 1) / 7)`.
    fn write_vle_inner(&mut self, value: u64, max_bits: u32) -> Result<(), CodecError> {
        let cont_max = (max_bits - 1).div_ceil(7) - 1;
        let mut v = value;
        let mut n = 0u32;
        while v >= 0x80 && n < cont_max {
            self.write_u8((v as u8 & 0x7F) | 0x80)?;
            v >>= 7;
            n += 1;
        }
        self.write_u8(v as u8)
    }

    /// Append a `vle_u16` field (1-3 wire bytes).
    fn write_vle_u16(&mut self, v: u16) -> Result<(), CodecError> {
        self.write_vle_inner(v as u64, 16)
    }

    /// Append a `vle_u32` field (1-5 wire bytes).
    fn write_vle_u32(&mut self, v: u32) -> Result<(), CodecError> {
        self.write_vle_inner(v as u64, 32)
    }

    /// Append a `vle_u64` field (1-9 wire bytes). Canonical Zenoh ZInt.
    fn write_vle_u64(&mut self, v: u64) -> Result<(), CodecError> {
        self.write_vle_inner(v, 64)
    }
}

/// Bounded sink over a caller-owned `&mut [u8]`. Raises
/// `CodecError::BufferOverflow` when an append would exceed the slice
/// capacity. The natural sink for MCU / DMA / fixed-frame call sites.
///
/// Available on `no_std` without `alloc` — the underlying storage is a
/// borrowed slice the caller already owns.
pub struct SliceSink<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> SliceSink<'a> {
    /// Wrap `buf` with write position 0.
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Current write position (bytes written so far).
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Remaining capacity from current position to end of buffer.
    pub const fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Consume the sink and return the prefix of the original buffer
    /// containing the bytes written. Callers use this to recover the
    /// exact-size view for downstream syscalls (e.g. `send(2)`).
    pub fn into_written(self) -> &'a mut [u8] {
        &mut self.buf[..self.pos]
    }
}

impl<'a> SliceSink<'a> {
    /// Append `N` bytes whose count the compiler knows: one capacity test and
    /// stores, with no call to a copy routine. A byte, a fixed-width integer or
    /// a VLE is written this way; only a string or a payload is of a length the
    /// program learns at run time, and that one is [`SceSink::write_bytes`].
    #[inline(always)]
    fn put<const N: usize>(&mut self, bytes: [u8; N]) -> Result<(), CodecError> {
        // An array assignment, not `copy_from_slice`: at a small optimisation
        // level the slice copy is a call whatever its length, and the array
        // store is the stores themselves.
        match self
            .buf
            .get_mut(self.pos..)
            .and_then(<[u8]>::first_chunk_mut::<N>)
        {
            Some(dst) => {
                *dst = bytes;
                self.pos += N;
                Ok(())
            }
            None => Err(CodecError::BufferOverflow),
        }
    }
}

impl<'a> SceSink for SliceSink<'a> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        let Some(end) = self.pos.checked_add(bytes.len()) else {
            return Err(CodecError::BufferOverflow);
        };
        let Some(dst) = self.buf.get_mut(self.pos..end) else {
            return Err(CodecError::BufferOverflow);
        };
        dst.copy_from_slice(bytes);
        self.pos = end;
        Ok(())
    }

    #[inline]
    fn write_u8(&mut self, b: u8) -> Result<(), CodecError> {
        self.put([b])
    }

    #[inline]
    fn write_u16_le(&mut self, v: u16) -> Result<(), CodecError> {
        self.put(v.to_le_bytes())
    }

    #[inline]
    fn write_u16_be(&mut self, v: u16) -> Result<(), CodecError> {
        self.put(v.to_be_bytes())
    }

    #[inline]
    fn write_u32_le(&mut self, v: u32) -> Result<(), CodecError> {
        self.put(v.to_le_bytes())
    }

    #[inline]
    fn write_u32_be(&mut self, v: u32) -> Result<(), CodecError> {
        self.put(v.to_be_bytes())
    }

    #[inline]
    fn write_u64_le(&mut self, v: u64) -> Result<(), CodecError> {
        self.put(v.to_le_bytes())
    }

    #[inline]
    fn write_u64_be(&mut self, v: u64) -> Result<(), CodecError> {
        self.put(v.to_be_bytes())
    }

    /// The VLE as one capacity test and the bytes stored in place: its length
    /// is known from the value ([`vle_len`]), so no byte is tested on its own and
    /// a VLE that does not fit writes nothing. The bytes are those of the
    /// provided method (seven bits each with the continuation bit set, the last
    /// of a full-width value a full eight).
    #[inline]
    fn write_vle_inner(&mut self, value: u64, max_bits: u32) -> Result<(), CodecError> {
        // A value of seven bits or fewer is one byte of itself, which is most
        // of the fields a frame holds: stored in line, with the general case a
        // call.
        if value < 0x80 {
            return self.put([value as u8]);
        }
        self.write_vle_long(value, max_bits)
    }

    #[inline]
    fn write_vle_u16(&mut self, v: u16) -> Result<(), CodecError> {
        self.write_vle_inner(u64::from(v), 16)
    }

    #[inline]
    fn write_vle_u32(&mut self, v: u32) -> Result<(), CodecError> {
        self.write_vle_inner(u64::from(v), 32)
    }

    #[inline]
    fn write_vle_u64(&mut self, v: u64) -> Result<(), CodecError> {
        self.write_vle_inner(v, 64)
    }

    fn position(&self) -> usize {
        self.pos
    }
}

impl<'a> SliceSink<'a> {
    /// A VLE of two bytes or more: one capacity test and the bytes in place.
    #[inline(never)]
    fn write_vle_long(&mut self, value: u64, max_bits: u32) -> Result<(), CodecError> {
        let n = vle_len(value, max_bits);
        let Some(dst) = self.buf.get_mut(self.pos..self.pos + n) else {
            return Err(CodecError::BufferOverflow);
        };
        // `vle_len` is at least 1, so a slice that was found has a last byte.
        let Some((last, lead)) = dst.split_last_mut() else {
            return Err(CodecError::BufferOverflow);
        };
        let mut v = value;
        for slot in lead {
            *slot = (v as u8 & 0x7F) | 0x80;
            v >>= 7;
        }
        *last = v as u8;
        self.pos += n;
        Ok(())
    }
}

/// A sink that keeps nothing and counts what it is given: the size an encode
/// would write, found by running it (docs/adr/0058 §3).
///
/// A codec whose length or count precedes its content needs the size of what
/// follows before the first of it is written, and a streaming sink cannot be
/// written back to. Running the encode against this sink gives that size with
/// the encode's own rules and no second statement of them, and, run on a whole
/// value first, shows whether the encode refuses it before any real sink is
/// touched. Never raises `BufferOverflow`; available without `alloc`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CountSink {
    written: usize,
}

impl CountSink {
    /// A sink that has been given nothing.
    pub const fn new() -> Self {
        Self { written: 0 }
    }

    /// Bytes given so far.
    pub const fn len(&self) -> usize {
        self.written
    }

    /// Whether nothing has been given.
    pub const fn is_empty(&self) -> bool {
        self.written == 0
    }
}

impl SceSink for CountSink {
    #[inline]
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        self.written += bytes.len();
        Ok(())
    }

    #[inline]
    fn write_u8(&mut self, _b: u8) -> Result<(), CodecError> {
        self.written += 1;
        Ok(())
    }

    /// The length of the VLE and not a loop of byte writes: one count of the
    /// value's bits and a division by a constant (docs/adr/0061 §4).
    #[inline]
    fn write_vle_inner(&mut self, value: u64, max_bits: u32) -> Result<(), CodecError> {
        self.written += vle_len(value, max_bits);
        Ok(())
    }

    // The three widths state their cap as a constant, so the length of a VLE is
    // computed with nothing left to find out at run time.
    #[inline]
    fn write_vle_u16(&mut self, v: u16) -> Result<(), CodecError> {
        self.written += vle_len_u16(v);
        Ok(())
    }

    #[inline]
    fn write_vle_u32(&mut self, v: u32) -> Result<(), CodecError> {
        self.written += vle_len_u32(v);
        Ok(())
    }

    #[inline]
    fn write_vle_u64(&mut self, v: u64) -> Result<(), CodecError> {
        self.written += vle_len_u64(v);
        Ok(())
    }

    fn position(&self) -> usize {
        self.written
    }
}

// ── Length without a buffer (docs/adr/0061) ──────────────────────

/// The bytes [`SceSink::write_vle_inner`] writes for `value` at `max_bits`: seven
/// data bits per byte, and at most `ceil((max_bits - 1) / 7)` bytes, the last of
/// them carrying a full eight. No loop: the highest set bit says how many
/// groups of seven the value has.
///
/// Always inlined: with a width the caller states as a constant the cap is a
/// constant too, and a field that is a single byte (the common case) is a
/// compare and an add. As a call it was a division and a count of leading zeros
/// per field, on every pass.
#[inline(always)]
pub const fn vle_len(value: u64, max_bits: u32) -> usize {
    if value < 0x80 {
        return 1;
    }
    let groups = (64 - value.leading_zeros()).div_ceil(7);
    let cap = (max_bits - 1).div_ceil(7);
    (if groups < cap { groups } else { cap }) as usize
}

/// The bytes of a `vle_u16` field (1-3).
#[inline(always)]
pub const fn vle_len_u16(v: u16) -> usize {
    vle_len(v as u64, 16)
}

/// The bytes of a `vle_u32` field (1-5).
#[inline(always)]
pub const fn vle_len_u32(v: u32) -> usize {
    vle_len(v as u64, 32)
}

/// The bytes of a `vle_u64` field (1-9).
#[inline(always)]
pub const fn vle_len_u64(v: u64) -> usize {
    vle_len(v, 64)
}

/// A codec that writes a source of type `S`: the link between a generated
/// `{Codec}::write_source` and the witness [`Measured`]. Generated, one impl for
/// each codec that has a source and takes no flag from a parent; it forwards to
/// the codec's own skeleton and states no wire rule.
pub trait SourceWrite<S: ?Sized> {
    /// The skeleton of the codec, run into any sink.
    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError>;
}

/// A source that has been counted: it passed every refusal of the encode, and the
/// frame it writes is [`Self::len`] bytes (docs/adr/0061 §1).
///
/// The only way to hold one is [`measure`], so the type says "this value is
/// admissible and this long", and the step that needs that fact does not check
/// it again and has nothing left to report. It borrows the source, so a safe
/// caller cannot change the value between the count and the write.
pub struct Measured<'s, C: SourceWrite<S>, S: ?Sized> {
    src: &'s S,
    len: usize,
    codec: core::marker::PhantomData<fn() -> C>,
}

impl<'s, C: SourceWrite<S>, S: ?Sized> Measured<'s, C, S> {
    /// The exact size of the frame, in bytes.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the frame is empty (a codec with no field and no variant).
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Write the frame into `window`, which is exactly [`Self::len`] bytes.
    ///
    /// Panics if the window is another length, or if the source does not say
    /// the same thing the second time as the first (its accessors are reads of
    /// a value, and a source that changes under the encode has broken that, see
    /// docs/adr/0061 §3). It never writes outside the window.
    pub fn write_into(self, window: &mut [u8]) {
        assert!(
            window.len() == self.len,
            "the window is not the length Measured::len reported"
        );
        let mut sink = SliceSink::new(window);
        let written = C::write(self.src, &mut sink);
        assert!(
            written.is_ok() && sink.position() == self.len,
            "the source changed between measure and write_into"
        );
    }
}

/// Count `src` as codec `C` writes it: the size of the frame, or the refusal the
/// encode would give, with nothing written anywhere.
pub fn measure<C: SourceWrite<S>, S: ?Sized>(src: &S) -> Result<Measured<'_, C, S>, CodecError> {
    let mut count = CountSink::new();
    C::write(src, &mut count)?;
    Ok(Measured {
        src,
        len: count.len(),
        codec: core::marker::PhantomData,
    })
}

// ── Heap-backed sink (alloc-only) ────────────────────────────────

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Heap-backed sink over a caller-owned `&mut Vec<u8>`. The vector
/// grows on demand, so `write_bytes` never returns `BufferOverflow` —
/// the implementation is effectively infallible. The natural sink for
/// std / `no_std + alloc` consumers and the engine behind the
/// generated `encode_to_vec()` facade.
#[cfg(feature = "alloc")]
pub struct VecSink<'a> {
    buf: &'a mut Vec<u8>,
    start_len: usize,
}

#[cfg(feature = "alloc")]
impl<'a> VecSink<'a> {
    /// Wrap `buf` for append-only writes. Records the buffer's
    /// current length so `position()` returns the bytes written by
    /// this sink instance (not the absolute Vec length) — codec emit
    /// stays positionally consistent when the destination is shared
    /// with a coalesced-send prefix.
    pub fn new(buf: &'a mut Vec<u8>) -> Self {
        let start_len = buf.len();
        Self { buf, start_len }
    }
}

#[cfg(feature = "alloc")]
impl<'a> SceSink for VecSink<'a> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        self.buf.extend_from_slice(bytes);
        Ok(())
    }

    fn write_u8(&mut self, b: u8) -> Result<(), CodecError> {
        self.buf.push(b);
        Ok(())
    }

    fn position(&self) -> usize {
        self.buf.len() - self.start_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cursor_reports_zero_remaining() {
        let c = SceCursor::new(&[]);
        assert_eq!(c.remaining(), 0);
        assert_eq!(c.peek_slice(1), Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn peek_does_not_advance() {
        let buf = [1, 2, 3, 4];
        let c = SceCursor::new(&buf);
        assert_eq!(c.peek_slice(2), Ok(&buf[..2]));
        assert_eq!(c.peek_slice(2), Ok(&buf[..2]));
        assert_eq!(c.remaining(), 4);
    }

    #[test]
    fn peek_truncated_returns_need_more() {
        let buf = [1, 2];
        let c = SceCursor::new(&buf);
        assert_eq!(c.peek_slice(3), Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn advance_then_peek_uses_new_offset() {
        let buf = [1, 2, 3, 4];
        let mut c = SceCursor::new(&buf);
        c.advance(2).unwrap();
        assert_eq!(c.remaining(), 2);
        assert_eq!(c.peek_slice(2), Ok(&buf[2..]));
    }

    #[test]
    fn advance_past_end_errors() {
        let buf = [1, 2];
        let mut c = SceCursor::new(&buf);
        assert_eq!(c.advance(3), Err(CodecError::NeedMoreBytes));
    }

    // ── VLE round-trip oracle vectors (Zenoh ZInt) ───────────────

    #[test]
    fn vle_u64_zero_is_one_byte() {
        let mut c = SceCursor::new(&[0x00]);
        assert_eq!(c.read_vle_u64(), Ok(0));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn vle_u64_127_is_one_byte() {
        let mut c = SceCursor::new(&[0x7F]);
        assert_eq!(c.read_vle_u64(), Ok(127));
    }

    #[test]
    fn vle_u64_128_is_two_bytes() {
        let mut c = SceCursor::new(&[0x80, 0x01]);
        assert_eq!(c.read_vle_u64(), Ok(128));
    }

    #[test]
    fn vle_u64_max_is_ten_bytes() {
        // u64::MAX = 0xFFFF_FFFF_FFFF_FFFF
        // VLE: 9 bytes of 0xFF (each carrying 7 bits + cont) + 1 byte 0x01
        let buf = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
        let mut c = SceCursor::new(&buf);
        assert_eq!(c.read_vle_u64(), Ok(u64::MAX));
    }

    #[test]
    fn vle_u16_overflow_on_third_byte_with_high_payload() {
        // Third byte's payload can carry at most 2 bits (16 - 14).
        // Payload 0x04 = 0b100 exceeds → overflow.
        let buf = [0xFF, 0xFF, 0x04];
        let mut c = SceCursor::new(&buf);
        assert_eq!(c.read_vle_u16(), Err(CodecError::VleWidthOverflow));
    }

    #[test]
    fn vle_u16_overflow_on_continuation_past_max_bytes() {
        // Three bytes all with continuation set → 4th byte would be needed
        // → exceeds u16 (3 bytes max).
        let buf = [0xFF, 0xFF, 0xFF, 0x01];
        let mut c = SceCursor::new(&buf);
        assert_eq!(c.read_vle_u16(), Err(CodecError::VleWidthOverflow));
    }

    #[test]
    fn vle_truncated_returns_need_more() {
        // Continuation set but no next byte.
        let buf = [0x80];
        let mut c = SceCursor::new(&buf);
        assert_eq!(c.read_vle_u32(), Err(CodecError::NeedMoreBytes));
    }

    // ── RFC §synth-5-B string primitive — InvalidUtf8 variant exists and is
    // distinct from the other CodecError variants. The actual UTF-8
    // validation lives at the codec emit site (`core::str::from_utf8`
    // in `present_if_decode_string_length_ref`); this assertion just
    // pins the runtime symbol so a future enum reorder cannot silently
    // collapse it.

    #[test]
    fn invalid_utf8_is_distinct_codec_error() {
        let e = CodecError::InvalidUtf8;
        assert_ne!(e, CodecError::NeedMoreBytes);
        assert_ne!(e, CodecError::VleWidthOverflow);
        assert_ne!(e, CodecError::TlvChainOverflow);
    }

    // ── BufferOverflow variant pin ───────────────────────────────

    #[test]
    fn buffer_overflow_is_distinct_codec_error() {
        let e = CodecError::BufferOverflow;
        assert_ne!(e, CodecError::NeedMoreBytes);
        assert_ne!(e, CodecError::VleWidthOverflow);
        assert_ne!(e, CodecError::TlvChainOverflow);
        assert_ne!(e, CodecError::InvalidUtf8);
    }

    // ── SliceSink ─────────────────────────────────────────────────

    #[test]
    fn slice_sink_records_position_after_writes() {
        let mut buf = [0u8; 8];
        let mut s = SliceSink::new(&mut buf);
        s.write_u8(0x11).unwrap();
        s.write_u16_le(0x4433).unwrap();
        s.write_u32_be(0xDEAD_BEEF).unwrap();
        assert_eq!(s.position(), 7);
        assert_eq!(s.remaining(), 1);
        assert_eq!(buf[..7], [0x11, 0x33, 0x44, 0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn slice_sink_overflow_when_write_exceeds_cap() {
        let mut buf = [0u8; 3];
        let mut s = SliceSink::new(&mut buf);
        s.write_u16_le(0xCAFE).unwrap();
        assert_eq!(s.write_u16_le(0xBABE), Err(CodecError::BufferOverflow));
        assert_eq!(s.position(), 2);
    }

    #[test]
    fn slice_sink_into_written_returns_exact_prefix() {
        let mut buf = [0u8; 16];
        {
            let mut s = SliceSink::new(&mut buf);
            s.write_bytes(b"abcd").unwrap();
            let view = s.into_written();
            assert_eq!(view, b"abcd");
        }
        assert_eq!(&buf[..4], b"abcd");
    }

    #[test]
    fn slice_sink_zero_byte_write_succeeds_at_boundary() {
        // Zero-length write must not advance the cursor and must not
        // raise BufferOverflow even at exact saturation. Codec emit
        // sites pass empty slices for absent optional fields.
        let mut buf = [0u8; 2];
        let mut s = SliceSink::new(&mut buf);
        s.write_u16_le(0xAAAA).unwrap();
        s.write_bytes(&[]).unwrap();
        assert_eq!(s.position(), 2);
    }

    // ── VecSink (alloc-only) ──────────────────────────────────────

    #[cfg(feature = "alloc")]
    #[test]
    fn vec_sink_appends_to_caller_buffer() {
        let mut v: Vec<u8> = Vec::new();
        let mut s = VecSink::new(&mut v);
        s.write_u32_le(0x1234_5678).unwrap();
        s.write_u8(0xAB).unwrap();
        s.write_bytes(b"!").unwrap();
        assert_eq!(v, [0x78, 0x56, 0x34, 0x12, 0xAB, b'!']);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn vec_sink_position_is_delta_not_absolute() {
        // Critical for coalesced-send: VecSink wrapping a prefilled
        // Vec must report bytes written by THIS sink (delta), not the
        // absolute Vec length. Otherwise DMA-aligned codec padding
        // computes against the wrong baseline and corrupts the wire.
        let mut v: Vec<u8> = alloc::vec![0xAA, 0xBB, 0xCC];
        let mut s = VecSink::new(&mut v);
        assert_eq!(s.position(), 0, "fresh sink starts at delta 0");
        s.write_u32_be(0x11_22_33_44).unwrap();
        assert_eq!(s.position(), 4, "delta after 4-byte write");
        assert_eq!(v, [0xAA, 0xBB, 0xCC, 0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn slice_sink_position_matches_inherent_method() {
        // Pin that the trait-routed position() and the inherent
        // position() return the same value (both should — inherent
        // delegates to the same `pos` field).
        let mut buf = [0u8; 8];
        let mut s = SliceSink::new(&mut buf);
        s.write_u16_be(0xDEAD).unwrap();
        let trait_pos = SceSink::position(&s);
        assert_eq!(trait_pos, s.position());
        assert_eq!(trait_pos, 2);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn vec_sink_preserves_existing_prefix() {
        // VecSink appends — existing bytes in the destination remain.
        // Critical for coalesced-send paths that write multiple frames
        // into one wire buffer.
        let mut v: Vec<u8> = alloc::vec![0xDE, 0xAD];
        let mut s = VecSink::new(&mut v);
        s.write_u16_be(0xBEEF).unwrap();
        assert_eq!(v, [0xDE, 0xAD, 0xBE, 0xEF]);
    }

    // ── Length without a buffer (docs/adr/0061) ──────────────────

    #[test]
    fn vle_len_is_what_the_sink_writes_at_every_boundary() {
        // Every bit position and its neighbours, the top of each width, and the
        // ninth byte of a u64.
        for shift in 0..64u32 {
            for v in [
                (1u64 << shift) - 1,
                1u64 << shift,
                (1u64 << shift) + 1,
                u64::MAX >> shift,
            ] {
                for bits in [16u32, 32, 64] {
                    let masked = if bits == 64 {
                        v
                    } else {
                        v & ((1u64 << bits) - 1)
                    };
                    let mut buf = [0u8; 10];
                    let mut s = SliceSink::new(&mut buf);
                    s.write_vle_inner(masked, bits).unwrap();
                    assert_eq!(
                        vle_len(masked, bits),
                        s.position(),
                        "{masked:#x} at {bits} bits"
                    );
                    let mut c = CountSink::new();
                    c.write_vle_inner(masked, bits).unwrap();
                    assert_eq!(
                        c.len(),
                        s.position(),
                        "count sink, {masked:#x} at {bits} bits"
                    );
                }
            }
        }
    }

    #[test]
    fn the_slice_sink_writes_the_bytes_of_the_provided_methods() {
        // A sink that implements only the two required methods, so every
        // integer and VLE is written by the trait's own provided code: the
        // reference the slice sink's direct stores must equal.
        struct Reference {
            bytes: [u8; 64],
            len: usize,
        }
        impl SceSink for Reference {
            fn write_bytes(&mut self, b: &[u8]) -> Result<(), CodecError> {
                self.bytes[self.len..self.len + b.len()].copy_from_slice(b);
                self.len += b.len();
                Ok(())
            }
            fn position(&self) -> usize {
                self.len
            }
        }
        fn run<K: SceSink>(k: &mut K, vle: u64, bits: u32) {
            k.write_u8(0xA5).unwrap();
            k.write_u16_le(0x1234).unwrap();
            k.write_u16_be(0x1234).unwrap();
            k.write_u32_le(0x1234_5678).unwrap();
            k.write_u32_be(0x1234_5678).unwrap();
            k.write_u64_le(0x1234_5678_9ABC_DEF0).unwrap();
            k.write_u64_be(0x1234_5678_9ABC_DEF0).unwrap();
            k.write_vle_inner(vle, bits).unwrap();
            k.write_vle_u16(vle as u16).unwrap();
            k.write_vle_u32(vle as u32).unwrap();
            k.write_vle_u64(vle).unwrap();
            k.write_bytes(&[1, 2, 3]).unwrap();
        }
        for shift in 0..64u32 {
            for v in [
                (1u64 << shift) - 1,
                1u64 << shift,
                (1u64 << shift) + 1,
                u64::MAX >> shift,
            ] {
                for bits in [16u32, 32, 64] {
                    let masked = if bits == 64 {
                        v
                    } else {
                        v & ((1u64 << bits) - 1)
                    };
                    let mut want = Reference {
                        bytes: [0; 64],
                        len: 0,
                    };
                    run(&mut want, masked, bits);
                    let mut buf = [0u8; 64];
                    let mut got = SliceSink::new(&mut buf);
                    run(&mut got, masked, bits);
                    assert_eq!(got.position(), want.len, "{masked:#x} at {bits} bits");
                    assert_eq!(
                        &buf[..want.len],
                        &want.bytes[..want.len],
                        "{masked:#x} at {bits} bits"
                    );
                }
            }
        }
    }

    #[test]
    fn a_vle_that_does_not_fit_writes_nothing() {
        let mut buf = [0xEEu8; 3];
        let mut s = SliceSink::new(&mut buf);
        // 300 is two bytes; one byte of room holds neither of them.
        s.write_u8(1).unwrap();
        s.write_u8(2).unwrap();
        assert_eq!(s.write_vle_u64(300), Err(CodecError::BufferOverflow));
        assert_eq!(s.position(), 2, "a refused VLE moved the position");
        assert_eq!(buf, [1, 2, 0xEE], "a refused VLE left a byte");
    }

    #[test]
    fn vle_len_helpers_name_the_width() {
        assert_eq!(
            (vle_len_u16(0), vle_len_u16(128), vle_len_u16(u16::MAX)),
            (1, 2, 3)
        );
        assert_eq!(
            (vle_len_u32(0), vle_len_u32(1 << 28), vle_len_u32(u32::MAX)),
            (1, 5, 5)
        );
        assert_eq!(
            (
                vle_len_u64(127),
                vle_len_u64(1 << 56),
                vle_len_u64(u64::MAX)
            ),
            (1, 9, 9)
        );
    }

    /// A codec by hand: a `vle_u32`, then a byte string with a `vle_u16` length
    /// that is refused over 8 bytes.
    trait PairSource {
        fn number(&self) -> u32;
        fn bytes(&self) -> &[u8];
    }

    struct Pair;

    impl<S: PairSource + ?Sized> SourceWrite<S> for Pair {
        fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {
            if src.bytes().len() > 8 {
                return Err(CodecError::LengthOutOfRange);
            }
            w.write_vle_u32(src.number())?;
            w.write_vle_u16(src.bytes().len() as u16)?;
            w.write_bytes(src.bytes())
        }
    }

    struct Held(u32, &'static [u8]);

    impl PairSource for Held {
        fn number(&self) -> u32 {
            self.0
        }
        fn bytes(&self) -> &[u8] {
            self.1
        }
    }

    #[test]
    fn measured_len_is_the_bytes_the_window_receives() {
        for (n, b) in [(0u32, &b""[..]), (300, b"abc"), (u32::MAX, b"12345678")] {
            let held = Held(n, b);
            let m = measure::<Pair, _>(&held).unwrap();
            let mut expected = [0u8; 32];
            let mut s = SliceSink::new(&mut expected);
            Pair::write(&held, &mut s).unwrap();
            let want = s.into_written().len();
            assert_eq!(m.len(), want);
            let mut window = [0xEEu8; 32];
            m.write_into(&mut window[..want]);
            assert_eq!(window[..want], expected[..want]);
            assert!(
                window[want..].iter().all(|&x| x == 0xEE),
                "nothing past the window is written"
            );
        }
    }

    #[test]
    fn a_refused_source_has_no_measure() {
        let held = Held(1, b"123456789");
        assert!(matches!(
            measure::<Pair, _>(&held),
            Err(CodecError::LengthOutOfRange)
        ));
    }

    #[test]
    #[should_panic(expected = "not the length")]
    fn a_window_of_another_length_panics() {
        let held = Held(1, b"ab");
        let m = measure::<Pair, _>(&held).unwrap();
        let mut window = [0u8; 16];
        m.write_into(&mut window);
    }

    /// A source whose second answer differs from its first: it breaks the law
    /// that a read of a value is a read of the same value.
    struct Lying {
        reads: core::cell::Cell<u32>,
        first: u32,
        later: u32,
    }

    impl PairSource for Lying {
        fn number(&self) -> u32 {
            let n = self.reads.get();
            self.reads.set(n + 1);
            if n == 0 {
                self.first
            } else {
                self.later
            }
        }
        fn bytes(&self) -> &[u8] {
            b""
        }
    }

    #[test]
    #[should_panic(expected = "changed between")]
    fn a_source_that_grows_under_the_write_panics_and_never_overruns() {
        let lying = Lying {
            reads: core::cell::Cell::new(0),
            first: 1,
            later: u32::MAX,
        };
        let m = measure::<Pair, _>(&lying).unwrap();
        let mut window = [0u8; 2];
        m.write_into(&mut window);
    }

    #[test]
    #[should_panic(expected = "changed between")]
    fn a_source_that_shrinks_under_the_write_panics() {
        let lying = Lying {
            reads: core::cell::Cell::new(0),
            first: u32::MAX,
            later: 1,
        };
        let m = measure::<Pair, _>(&lying).unwrap();
        let mut window = [0u8; 6];
        m.write_into(&mut window);
    }

    // ── Object-safe trait pin ────────────────────────────────────

    #[test]
    fn sce_sink_is_object_safe_via_dyn() {
        // If SceSink ever gains a Self-returning method, generic param,
        // or associated type, this line stops compiling. The pin
        // protects the documented object-safety guarantee.
        let mut buf = [0u8; 4];
        let mut s = SliceSink::new(&mut buf);
        let d: &mut dyn SceSink = &mut s;
        d.write_u32_le(0x0102_0304).unwrap();
        assert_eq!(buf, [0x04, 0x03, 0x02, 0x01]);
    }
}
