// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reader a generated decode reads one frame through (docs/adr/0128).
//!
//! Every read of a [`SceReader`] is total: it returns a value, and where the
//! frame cannot give one it returns the zero of the value's type and records
//! the refusal. The first refusal recorded is the frame's; once one is, every
//! later read is a no-op that returns zero and consumes nothing, so a decode
//! reads its frame as one straight line of reads and says its verdict once, at
//! the end ([`SceReader::verdict`]). This is the sticky error of a decoder's
//! state (upb's parse state, Go's `bufio.Scanner`), and it is what removes the
//! branch and the early return each field of a `?`-threaded decode costs.
//!
//! A decode answers exactly what a decode that returns at its first refusal
//! answers: `Ok` for the same frames, the same first error for every other,
//! and the same bytes consumed on success. Two rules keep it so:
//!
//! - A loop tests the failure at its head ([`SceReader::failed`]; a loop to
//!   the end of the frame reads [`SceReader::remaining`], which is zero once
//!   the frame has failed), so a count read before a failure cannot make a
//!   long loop of no-op reads.
//! - A refusal that is not a read (a count past its bound, a value outside its
//!   type, an arm the build leaves out) is recorded through the same reader
//!   ([`SceReader::fail`], [`SceReader::settle`]).
//!
//! A value past what a storage profile holds is not a refusal of the frame
//! (docs/adr/0077 §7): it is noted ([`SceReader::note_capacity`]), the reads
//! go on, and the verdict says `CapacityExceeded` only where the frame is
//! read whole with no refusal of its own.
//!
//! A [`SceCursor`] is the caller's place in its input and is untouched by a
//! frame that fails: [`SceCursor::read_frame`] runs a decode over a reader
//! that starts where the cursor stands and moves the cursor past the frame
//! only when the frame was read whole.

use crate::codec::{zint_byte, zint_tail, CodecError, SceCursor};

/// `N` zero bytes that live for the whole program: what a read of a fixed
/// count of bytes returns once the frame has failed, so that its length is
/// still `N` and an index of a constant below `N` needs no check.
const fn zeros<const N: usize>() -> &'static [u8; N] {
    const { &[0; N] }
}

/// The reader one frame is decoded through: the bytes the frame is read from,
/// how far the decode has read, the first refusal it recorded and whether a
/// value was past a capacity.
///
/// Built by [`SceCursor::read_frame`] for a frame read from a cursor, or by
/// [`SceReader::new`] over bytes a caller holds (a list's span, the bytes a
/// length says a body occupies). It is not `Copy`: a decode that reads a span
/// twice marks where it stands ([`SceReader::mark`]) and goes back
/// ([`SceReader::rewind`]), and keeps what it recorded.
#[derive(Debug, Clone)]
pub struct SceReader<'a> {
    buf: &'a [u8],
    pos: usize,
    fault: Option<CodecError>,
    over: bool,
}

impl<'a> SceReader<'a> {
    /// A reader at the start of `buf`, with nothing recorded.
    pub const fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            fault: None,
            over: false,
        }
    }

    /// The bytes from where the reader stands to the end of its input: zero
    /// once the frame has failed, so a loop to the end of the frame stops.
    pub const fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// How many bytes of its input the reader has consumed.
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Whether a refusal is recorded: the frame does not decode, and every read
    /// from here on is a no-op.
    pub const fn failed(&self) -> bool {
        self.fault.is_some()
    }

    /// Whether a value of the frame was past what its storage profile holds
    /// (docs/adr/0077 §7).
    pub const fn exceeded(&self) -> bool {
        self.over
    }

    /// Whether the frame has been read with no refusal and no value past a
    /// capacity: the values the decode built hold what their types say.
    pub const fn clean(&self) -> bool {
        self.fault.is_none() && !self.over
    }

    /// The frame's verdict: the first refusal recorded, or `CapacityExceeded`
    /// where a value was past a capacity and nothing was refused, or `Ok`.
    pub fn verdict(&self) -> Result<(), CodecError> {
        match self.fault {
            Some(refused) => Err(refused),
            None if self.over => Err(CodecError::CapacityExceeded),
            None => Ok(()),
        }
    }

    /// Record `refusal` as the frame's, unless one is recorded already, and
    /// end the input, so that every later read is a no-op.
    #[cold]
    pub fn fail(&mut self, refusal: CodecError) {
        if self.fault.is_none() {
            self.fault = Some(refusal);
        }
        self.pos = self.buf.len();
    }

    /// Note that a value of the frame was past what its profile holds; the
    /// reads go on (docs/adr/0077 §7).
    pub fn note_capacity(&mut self) {
        self.over = true;
    }

    /// The value of `result`, or, where it is a refusal, the zero of its type
    /// with the refusal recorded: a step that can refuse, made total.
    #[inline]
    pub fn settle<T: Default>(&mut self, result: Result<T, CodecError>) -> T {
        match result {
            Ok(value) => value,
            Err(refused) => {
                self.fail(refused);
                T::default()
            }
        }
    }

    /// [`Self::settle`] of what `step` answers, `step` being handed this
    /// reader: a step whose own closures read the frame, which could not borrow
    /// the reader while the call that settles it does.
    #[inline]
    pub fn settle_with<T: Default>(
        &mut self,
        step: impl FnOnce(&mut Self) -> Result<T, CodecError>,
    ) -> T {
        let result = step(self);
        self.settle(result)
    }

    /// The value of `result` a storing decode holds: past a capacity, the zero
    /// of its type with the capacity noted and the reads going on; any other
    /// refusal recorded as [`Self::settle`] records it.
    #[inline]
    pub fn hold<T: Default>(&mut self, result: Result<T, CodecError>) -> T {
        match result {
            Ok(value) => value,
            Err(CodecError::CapacityExceeded) => {
                self.over = true;
                T::default()
            }
            Err(refused) => {
                self.fail(refused);
                T::default()
            }
        }
    }

    /// [`Self::hold`] of what `step` answers, `step` being handed this
    /// reader (see [`Self::settle_with`]).
    #[inline]
    pub fn hold_with<T: Default>(
        &mut self,
        step: impl FnOnce(&mut Self) -> Result<T, CodecError>,
    ) -> T {
        let result = step(self);
        self.hold(result)
    }

    /// The next `N` bytes as an array, consumed; `N` zero bytes and
    /// `NeedMoreBytes` recorded where fewer remain.
    ///
    /// The count is in the type, so a read `raw[k]` of a constant `k` below `N`
    /// has no bounds check and no panic path, on the frame's bytes and on the
    /// zeros alike.
    #[inline]
    pub fn take_array<const N: usize>(&mut self) -> &'a [u8; N] {
        let buf = self.buf;
        match buf.get(self.pos..).and_then(<[u8]>::first_chunk::<N>) {
            Some(bytes) => {
                self.pos += N;
                bytes
            }
            None => {
                self.fail(CodecError::NeedMoreBytes);
                zeros::<N>()
            }
        }
    }

    /// The next `n` bytes, consumed; no bytes and `NeedMoreBytes` recorded
    /// where fewer remain.
    #[inline]
    pub fn take_slice(&mut self, n: usize) -> &'a [u8] {
        let buf = self.buf;
        match buf.get(self.pos..).and_then(|rest| rest.get(..n)) {
            Some(bytes) => {
                self.pos += n;
                bytes
            }
            None => {
                self.fail(CodecError::NeedMoreBytes);
                &[]
            }
        }
    }

    /// Every byte that remains, consumed: a field that runs to the end of its
    /// frame.
    #[inline]
    pub fn take_rest(&mut self) -> &'a [u8] {
        let rest = self.rest();
        self.pos = self.buf.len();
        rest
    }

    /// Every byte that remains, not consumed: no bytes once the frame has
    /// failed.
    #[inline]
    pub fn rest(&self) -> &'a [u8] {
        let buf = self.buf;
        buf.get(self.pos..).unwrap_or(&[])
    }

    /// Every byte that remains, not consumed, where at least `N` do; `N` zero
    /// bytes and `NeedMoreBytes` recorded where fewer do. A frame whose fixed
    /// head is `N` bytes reads it by constant indexes below `N`, which hold on
    /// either.
    #[inline]
    pub fn peek_at_least<const N: usize>(&mut self) -> &'a [u8] {
        let rest = self.rest();
        if rest.len() < N {
            self.fail(CodecError::NeedMoreBytes);
            return zeros::<N>();
        }
        rest
    }

    /// The next byte, not consumed; zero and `NeedMoreBytes` recorded at the
    /// end of the input. The byte a variant dispatches on before its arm reads
    /// it as its own header.
    #[inline]
    pub fn peek_u8(&mut self) -> u8 {
        match self.buf.get(self.pos) {
            Some(&byte) => byte,
            None => {
                self.fail(CodecError::NeedMoreBytes);
                0
            }
        }
    }

    /// Consume `n` bytes; `NeedMoreBytes` recorded where fewer remain.
    #[inline]
    pub fn advance(&mut self, n: usize) {
        if self.remaining() < n {
            self.fail(CodecError::NeedMoreBytes);
        } else {
            self.pos += n;
        }
    }

    /// One ZInt held to `max_bits`: a value past `2^max_bits - 1` records
    /// `VleWidthOverflow` and reads as zero, a longer encoding of a value the
    /// width holds is that value, and a chain the input ends inside records
    /// `NeedMoreBytes` (see [`SceCursor::read_vle_u64`]). A ZInt of one byte is
    /// read where this is inlined; a longer one is the cursor's one cold body
    /// (`codec::zint_tail`), so the two readers of a ZInt run one decode
    /// (docs/adr/0130).
    #[inline(always)]
    fn read_vle_inner(&mut self, max_bits: u32) -> u64 {
        if let Some(b) = zint_byte(self.buf, self.pos) {
            self.pos += 1;
            return u64::from(b);
        }
        let (value, at) = zint_tail(self.buf, self.pos, max_bits);
        match value {
            Ok(value) => {
                self.pos = at;
                value
            }
            Err(refused) => {
                self.fail(refused);
                0
            }
        }
    }

    /// A `vle_u16` field: a ZInt whose value is at most `u16::MAX`.
    #[inline]
    pub fn read_vle_u16(&mut self) -> u16 {
        self.read_vle_inner(16) as u16
    }

    /// A `vle_u32` field: a ZInt whose value is at most `u32::MAX`.
    #[inline]
    pub fn read_vle_u32(&mut self) -> u32 {
        self.read_vle_inner(32) as u32
    }

    /// A `vle_u64` field: a ZInt.
    #[inline]
    pub fn read_vle_u64(&mut self) -> u64 {
        self.read_vle_inner(64)
    }

    /// Where the reader stands, to come back to with [`Self::rewind`]: how a
    /// decode reads a span again.
    pub const fn mark(&self) -> usize {
        self.pos
    }

    /// The bytes consumed since `at`, a [`Self::mark`] of this reader: what one
    /// entry occupies (docs/adr/0130). Empty once the frame has failed, whose
    /// bytes from there are not the frame's.
    pub fn since(&self, at: usize) -> &'a [u8] {
        if self.fault.is_some() {
            return &[];
        }
        let buf = self.buf;
        buf.get(at..self.pos).unwrap_or(&[])
    }

    /// Go back to `at`, a [`Self::mark`] of this reader. What the reader
    /// recorded stays: a failed frame stays failed, and its input ended.
    pub fn rewind(&mut self, at: usize) {
        if self.fault.is_none() && at <= self.buf.len() {
            self.pos = at;
        }
    }

    /// Read the next `n` bytes as a frame of their own through `read`, and
    /// consume them: the bytes a length says a body occupies. What `read`
    /// records is this frame's, the first refusal of the two counting first;
    /// where fewer than `n` bytes remain, `NeedMoreBytes` is recorded and
    /// `read` reads nothing.
    #[inline]
    pub fn within<T>(&mut self, n: usize, read: impl FnOnce(&mut SceReader<'a>) -> T) -> T {
        let bytes = self.take_slice(n);
        let mut inner = SceReader {
            buf: bytes,
            pos: 0,
            fault: self.fault,
            over: false,
        };
        if inner.fault.is_some() {
            inner.pos = inner.buf.len();
        }
        let value = read(&mut inner);
        if let Some(refused) = inner.fault {
            self.fail(refused);
        }
        self.over |= inner.over;
        value
    }
}

impl<'a> SceCursor<'a> {
    /// Decode one frame through `read`, over a [`SceReader`] that starts where
    /// this cursor stands, and answer the reader's verdict.
    ///
    /// The cursor moves past the frame where the frame was read whole: on
    /// `Ok`, and on `CapacityExceeded`, whose frame is well formed and read to
    /// its end (docs/adr/0077 §7). On any other refusal it does not move, so a
    /// caller that receives `NeedMoreBytes` reads the frame again from the same
    /// place once more bytes have arrived.
    #[inline]
    pub fn read_frame(&mut self, read: impl FnOnce(&mut SceReader<'a>)) -> Result<(), CodecError> {
        let mut reader = SceReader::new(self.peek_slice(self.remaining()).unwrap_or_default());
        read(&mut reader);
        match reader.fault {
            None => self.advance(reader.pos).and(reader.verdict()),
            Some(refused) => Err(refused),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_consume_the_bytes_they_return() {
        let bytes = [1u8, 2, 3, 4, 5, 6];
        let mut r = SceReader::new(&bytes);
        assert_eq!(r.take_array::<2>(), &[1, 2]);
        assert_eq!(r.peek_u8(), 3);
        assert_eq!(r.take_slice(3), &[3, 4, 5]);
        assert_eq!(r.position(), 5);
        assert_eq!(r.remaining(), 1);
        assert_eq!(r.rest(), &[6]);
        assert_eq!(r.take_rest(), &[6]);
        assert_eq!(r.remaining(), 0);
        assert!(r.clean());
        assert_eq!(r.verdict(), Ok(()));
    }

    #[test]
    fn a_read_past_the_end_records_need_more_bytes_and_reads_zero() {
        let bytes = [7u8];
        let mut r = SceReader::new(&bytes);
        assert_eq!(r.take_array::<2>(), &[0, 0]);
        assert!(r.failed());
        assert_eq!(r.verdict(), Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn the_first_refusal_is_the_frames() {
        let bytes = [0xFFu8, 0xFF, 0x7F, 9];
        let mut r = SceReader::new(&bytes);
        // A ZInt of 0x1FFFFF is past a 16-bit width.
        assert_eq!(r.read_vle_u16(), 0);
        r.fail(CodecError::TooManyElements);
        r.settle::<u8>(Err(CodecError::InvalidUtf8));
        assert_eq!(r.verdict(), Err(CodecError::VleWidthOverflow));
    }

    #[test]
    fn every_read_after_a_refusal_is_a_no_op_that_reads_zero() {
        let bytes = [1u8, 2, 3, 4];
        let mut r = SceReader::new(&bytes);
        r.fail(CodecError::UndeclaredEnumValue);
        assert_eq!(r.remaining(), 0);
        assert_eq!(r.rest(), &[] as &[u8]);
        assert_eq!(r.take_array::<3>(), &[0, 0, 0]);
        assert_eq!(r.take_slice(1), &[] as &[u8]);
        assert_eq!(r.peek_u8(), 0);
        assert_eq!(r.read_vle_u64(), 0);
        assert_eq!(r.peek_at_least::<2>(), &[0, 0]);
        r.advance(1);
        assert_eq!(r.verdict(), Err(CodecError::UndeclaredEnumValue));
    }

    #[test]
    fn a_rewind_after_a_refusal_does_not_bring_the_bytes_back() {
        let bytes = [1u8, 2, 3];
        let mut r = SceReader::new(&bytes);
        let at = r.mark();
        assert_eq!(r.take_array::<1>(), &[1]);
        r.rewind(at);
        assert_eq!(r.take_array::<1>(), &[1]);
        r.fail(CodecError::NeedMoreBytes);
        r.rewind(at);
        assert_eq!(r.remaining(), 0);
        assert_eq!(r.take_array::<1>(), &[0]);
    }

    #[test]
    fn a_capacity_is_noted_and_answered_only_where_nothing_was_refused() {
        let bytes = [5u8, 6];
        let mut r = SceReader::new(&bytes);
        assert_eq!(r.hold::<u8>(Err(CodecError::CapacityExceeded)), 0);
        assert!(!r.failed());
        assert!(r.exceeded());
        assert!(!r.clean());
        // The reads go on past a capacity.
        assert_eq!(r.take_array::<2>(), &[5, 6]);
        assert_eq!(r.verdict(), Err(CodecError::CapacityExceeded));
        r.fail(CodecError::LengthOutOfRange);
        assert_eq!(r.verdict(), Err(CodecError::LengthOutOfRange));
    }

    #[test]
    fn a_settled_step_is_its_value_or_the_zero_of_its_type() {
        let mut r = SceReader::new(&[]);
        assert_eq!(r.settle(Ok::<u16, CodecError>(9)), 9);
        assert!(r.clean());
        assert_eq!(
            r.settle_with(|_| Err::<u16, CodecError>(CodecError::InvalidUtf8)),
            0
        );
        assert_eq!(r.verdict(), Err(CodecError::InvalidUtf8));
    }

    #[test]
    fn a_frame_within_a_length_reads_its_bytes_and_shares_the_verdict() {
        let bytes = [1u8, 2, 3, 4];
        let mut r = SceReader::new(&bytes);
        let inner = r.within(2, |inner| {
            let head = inner.take_array::<1>()[0];
            (head, inner.remaining())
        });
        assert_eq!(inner, (1, 1));
        assert_eq!(r.position(), 2);
        // A body that reads past its length refuses the frame.
        r.within(1, |inner| {
            inner.take_array::<2>();
        });
        assert_eq!(r.verdict(), Err(CodecError::NeedMoreBytes));
        // A length past the input reads nothing and refuses with the first.
        let mut short = SceReader::new(&bytes);
        let read = short.within(9, |inner| inner.take_array::<1>()[0]);
        assert_eq!(read, 0);
        assert_eq!(short.verdict(), Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn a_vle_reads_as_the_cursor_reads_it() {
        let bytes = [0x80u8, 0x01, 0x05];
        let mut r = SceReader::new(&bytes);
        assert_eq!(r.read_vle_u16(), 128);
        assert_eq!(r.read_vle_u32(), 5);
        assert!(r.clean());
        // A chain the input ends inside.
        let mut cut = SceReader::new(&[0x80u8]);
        assert_eq!(cut.read_vle_u64(), 0);
        assert_eq!(cut.verdict(), Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn the_reader_and_the_cursor_read_every_zint_alike() {
        // Every length of chain, cut at every byte, at every width: the reader's
        // verdict and value are the cursor's, since both run one body.
        let chains: [&[u8]; 6] = [
            &[0x05],
            &[0x80, 0x01],
            &[0xFF, 0xFF, 0x03],
            &[0xFF, 0xFF, 0xFF, 0x0F],
            &[0xFF; 9],
            &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
        ];
        for chain in chains {
            for cut in 0..=chain.len() {
                let bytes = &chain[..cut];
                for bits in [16u32, 32, 64] {
                    let mut cursor = SceCursor::new(bytes);
                    let expected = match bits {
                        16 => cursor.read_vle_u16().map(u64::from),
                        32 => cursor.read_vle_u32().map(u64::from),
                        _ => cursor.read_vle_u64(),
                    };
                    let mut r = SceReader::new(bytes);
                    let value = match bits {
                        16 => u64::from(r.read_vle_u16()),
                        32 => u64::from(r.read_vle_u32()),
                        _ => r.read_vle_u64(),
                    };
                    match expected {
                        Ok(v) => {
                            assert_eq!((value, r.verdict()), (v, Ok(())), "{bytes:?} at {bits}");
                            assert_eq!(r.position(), bytes.len() - cursor.remaining());
                        }
                        Err(e) => {
                            assert_eq!((value, r.verdict()), (0, Err(e)), "{bytes:?} at {bits}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_cursor_moves_past_a_frame_read_whole_and_stays_on_a_refusal() {
        let bytes = [1u8, 2, 3];
        let mut cursor = SceCursor::new(&bytes);
        assert_eq!(
            cursor.read_frame(|r| {
                r.take_array::<2>();
            }),
            Ok(())
        );
        assert_eq!(cursor.remaining(), 1);
        assert_eq!(
            cursor.read_frame(|r| {
                r.take_array::<2>();
            }),
            Err(CodecError::NeedMoreBytes)
        );
        assert_eq!(cursor.remaining(), 1, "a refused frame leaves the cursor");
        assert_eq!(
            cursor.read_frame(|r| {
                r.take_array::<1>();
                r.note_capacity();
            }),
            Err(CodecError::CapacityExceeded)
        );
        assert_eq!(
            cursor.remaining(),
            0,
            "a frame past a capacity is read whole"
        );
    }
}
