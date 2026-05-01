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

/// Typed decode error. `NeedMoreBytes` is the only reachable variant
/// while every codec field is fixed-width; B1-α adds `VleWidthOverflow`,
/// B1-β adds `UnknownVariantTag`. The enum is `#[non_exhaustive]` to
/// keep additive variants from breaking downstream `match` arms before
/// SCE 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecError {
    /// The cursor's remaining buffer is shorter than the codec's
    /// declared minimum frame. Caller should resume after appending
    /// more bytes.
    NeedMoreBytes,
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
}
