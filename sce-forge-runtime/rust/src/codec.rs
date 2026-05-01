// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Codec cursor + typed error contract for `sce:kind="codec"` decode bodies.
//!
//! RFC `watching-zenoh/docs/rfc-sce-protocol-synthesis.md` §5.B L494-519
//! pins a per-language cursor + Result/Option shape on decode so a
//! truncated input never aborts — it returns `NeedMoreBytes` and the
//! caller resumes after additional bytes arrive (DMA boundary,
//! fragmented network read).
//!
//! Phase B1-prep ships the minimum API the existing fixed-width codec
//! fixtures need: `peek_slice` (non-advancing), `advance` (post-success),
//! `remaining`. Stream-style readers (`read_u8`, `read_vle_*`, `read_tag`,
//! `skip_field`) land alongside their first consumer in B1-α/β/δ — VLE,
//! variant, present-if respectively.
//!
//! Encode-side cursor + `BufferOverflow` lands in B1-α (variable-length
//! VLE encode is the first reachable consumer).

/// Typed decode error. The enum is `#[non_exhaustive]` to keep
/// additive variants from breaking downstream `match` arms before
/// SCE 1.0. B1-β adds `UnknownVariantTag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecError {
    /// The cursor's remaining buffer is shorter than the codec's
    /// declared minimum frame. Caller should resume after appending
    /// more bytes.
    NeedMoreBytes,
    /// A `vle_u<N>` field's continuation chain implies a value wider
    /// than the declared type. Either the wire is corrupt or the
    /// author chose a too-narrow type. RFC §5.B `codec/vle-width-overflow`.
    VleWidthOverflow,
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
    /// `max_bits` payload width. Each byte carries 7 data bits in its
    /// low 7; bit 7 is the continuation flag (1 = more bytes follow).
    /// LSB-first byte order — the first byte's payload occupies the
    /// low 7 bits of the result. Mirrors the Zenoh ZInt wire format
    /// (RFC §5.B Appendix B).
    ///
    /// Returns `VleWidthOverflow` when the continuation chain implies
    /// a value wider than `max_bits` (either the wire is corrupt or
    /// the codec author chose a too-narrow `vle_u<N>` type).
    fn read_vle_inner(&mut self, max_bits: u32) -> Result<u64, CodecError> {
        let max_bytes = max_bits.div_ceil(7);
        let mut value: u64 = 0;
        let mut shift: u32 = 0;
        for _ in 0..max_bytes {
            let b = self.peek_slice(1)?[0];
            self.advance(1)?;
            let payload = (b & 0x7F) as u64;
            // On the final byte the type's max_bits may permit only a
            // partial 7 bits; refuse payloads that would overflow it.
            if shift + 7 > max_bits {
                let allowed = max_bits - shift;
                let max_payload = (1u64 << allowed) - 1;
                if payload > max_payload {
                    return Err(CodecError::VleWidthOverflow);
                }
            }
            value |= payload << shift;
            if (b & 0x80) == 0 {
                return Ok(value);
            }
            shift += 7;
        }
        // Read max_bytes bytes but the last byte still set the
        // continuation flag — value would not fit max_bits.
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

    /// Read a `vle_u64` field (1-10 wire bytes). Canonical Zenoh ZInt.
    pub fn read_vle_u64(&mut self) -> Result<u64, CodecError> {
        self.read_vle_inner(64)
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
}
