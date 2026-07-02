// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Codec cursor + typed error contract for `sce:kind="codec"` decode bodies.
//!
//! RFC `watching-zenoh/docs/rfc-sce-protocol-synthesis.md` §synth-5-B L494-519
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
}

/// Project a borrowed slice of owned elements into the bounded inline
/// list (`heapless::Vec<U, N>`) the borrowed codec view spells, applying
/// `f` to each element by reference.
///
/// The owned mirror of a bounded list (`<sce:repeat>` / `<sce:tlv-chain>`)
/// is an unbounded `Vec`, so the owned→borrowed projection
/// (`{Codec}Owned::try_as_borrowed`) may carry more than `N` elements;
/// this raises [`CodecError::TooManyElements`] — the same bound and error
/// the decode path enforces, keeping the system invariant `<= N`
/// end-to-end (encode must never emit wire its own decoder would reject).
///
/// SSOT for the owned→borrowed bounded-list step: every generated
/// `try_as_borrowed` calls this rather than open-coding the capacity loop,
/// so the bound semantics live in exactly one place. `f` is fallible so a
/// nested fallible element projection (`_e.try_as_borrowed()`) threads its
/// own error through; an infallible element projection passes
/// `|_e| Ok(...)`.
pub fn try_project_bounded<'s, T, U, const N: usize>(
    src: &'s [T],
    mut f: impl FnMut(&'s T) -> Result<U, CodecError>,
) -> Result<crate::heapless::Vec<U, N>, CodecError> {
    // `&'s T` (not a fresh higher-ranked `&T`) ties each element's borrow
    // to the slice's lifetime, so a projected `U` that re-borrows the
    // element (`_e.as_borrowed() -> Body<'s>`) is nameable — the HRTB form
    // cannot express "return type borrows the closure argument".
    let mut out = crate::heapless::Vec::new();
    for item in src {
        out.push(f(item)?)
            .map_err(|_| CodecError::TooManyElements)?;
    }
    Ok(out)
}

// ── Portable owned scalar storage (the `{Codec}Owned` byte/string carrier) ──
//
// A codec's borrowed view decodes `bytes` / `string` fields as zero-copy
// `&'a [u8]` / `&'a str`; its lifetime-free owned mirror needs a container
// that holds those bytes by value, resolving to a growable `Vec` / `String`
// under `alloc` or a fixed `heapless` form (the C11 `char[N]` analog) on the
// heap-free MCU tier. `N` (the field's `sce:max-size`) rides on the type so
// a hand-assembled `{Codec}Owned` builder infers the cap from the field
// (`SceBytes::from_slice(&v)?`) rather than hardcoding it.
//
// `SceBytes<N>` is the SAME concept the statechart-emit runtime needs for
// typed `_event.data` byte payloads, so it lives in the shared
// `sce-portable-bytes` crate (one definition, both runtimes `pub use` it —
// the construction logic cannot drift). Re-exported here as
// `sce_forge_runtime::codec::SceBytes` so generated codec output spells the
// path unchanged. Its `from_slice` raises the crate-neutral
// [`sce_portable_bytes::CapacityExceeded`]; the `From` impl below maps that
// into `CodecError::TooManyElements` so a generated `try_into_owned` keeps
// threading one `?`. `SceString<N>` stays codec-local below — its emit-path
// sibling uses a *global* cap (not per-field `N`), so there is no shared
// definition to factor out.
pub use sce_portable_bytes::SceBytes;

impl From<sce_portable_bytes::CapacityExceeded> for CodecError {
    /// A no-alloc `SceBytes::from_slice` past its capacity is the same
    /// bounded-storage overflow the decode path reports — surface it as the
    /// existing typed variant so callers match one error.
    fn from(_: sce_portable_bytes::CapacityExceeded) -> Self {
        CodecError::TooManyElements
    }
}

/// Portable owned storage for a `string` codec field. Wraps `String` under
/// `alloc` (`N` advisory) or `heapless::String<N>` otherwise. `N` rides on
/// the type for the same downstream-inference reason as [`SceBytes`].
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq)]
pub struct SceString<const N: usize>(alloc::string::String, core::marker::PhantomData<[u8; N]>);
/// no-alloc variant of [`SceString`]: fixed-capacity inline storage capped at
/// `N`, heap-free for the MCU tier.
#[cfg(not(feature = "alloc"))]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq)]
pub struct SceString<const N: usize>(crate::heapless::String<N>);

impl<const N: usize> SceString<N> {
    /// Copy a borrowed `&str` decode view into the owned form. Profile
    /// semantics mirror [`SceBytes::from_slice`].
    #[cfg(feature = "alloc")]
    pub fn from_view(s: &str) -> Result<Self, CodecError> {
        Ok(Self(
            alloc::string::String::from(s),
            core::marker::PhantomData,
        ))
    }
    /// no-alloc counterpart: fixed-capacity copy, fallible past `N`.
    #[cfg(not(feature = "alloc"))]
    pub fn from_view(s: &str) -> Result<Self, CodecError> {
        crate::heapless::String::try_from(s)
            .map(Self)
            .map_err(|_| CodecError::TooManyElements)
    }
    /// Borrow the owned string back as `&str` — the projection
    /// `{Codec}Owned::as_borrowed` reuses.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<const N: usize> core::ops::Deref for SceString<N> {
    type Target = str;
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

// String-literal comparison parity with the wrapped `String` /
// `heapless::String`, so `assert_eq!(owned.locator, "abc")` works without an
// explicit `.as_str()` (consumed by the nested owned round-trip test). Only
// the `&str` form is kept — string literals are `&str`; the unsized `str`
// right-hand side has no consumer.
impl<const N: usize> PartialEq<&str> for SceString<N> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
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

impl<'a> SceSink for SliceSink<'a> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        if self.remaining() < bytes.len() {
            return Err(CodecError::BufferOverflow);
        }
        let end = self.pos + bytes.len();
        self.buf[self.pos..end].copy_from_slice(bytes);
        self.pos = end;
        Ok(())
    }

    fn position(&self) -> usize {
        self.pos
    }
}

// ── Heap-backed sink (alloc-only) ────────────────────────────────

#[cfg(feature = "alloc")]
extern crate alloc;
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
