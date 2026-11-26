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

use crate::codec::{read_zint_short, CodecError, SceCursor};

/// `N` zero bytes that live for the whole program: what a read of a fixed
/// count of bytes returns once the frame has failed, so that its length is
/// still `N` and an index of a constant below `N` needs no check.
const fn zeros<const N: usize>() -> &'static [u8; N] {
    const { &[0; N] }
}

/// A length as a codec's field holds it, in the field's own integer type: the
/// count of bytes it names, or `None` where it names none (a negative value).
/// What [`SceReader::within`] takes, so a length reaches the reader whole
/// whatever the field's width and sign, and the reader alone decides what the
/// target can hold.
pub trait WireLength: Copy {
    /// The count of bytes this length names, or `None` for a negative one.
    fn byte_count(self) -> Option<u64>;
}

macro_rules! wire_length_unsigned {
    ($($t:ty),*) => {$(
        impl WireLength for $t {
            #[inline(always)]
            fn byte_count(self) -> Option<u64> {
                Some(u64::from(self))
            }
        }
    )*};
}

macro_rules! wire_length_signed {
    ($($t:ty),*) => {$(
        impl WireLength for $t {
            #[inline(always)]
            fn byte_count(self) -> Option<u64> {
                u64::try_from(self).ok()
            }
        }
    )*};
}

wire_length_unsigned!(u8, u16, u32, u64);
wire_length_signed!(i8, i16, i32, i64);

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
    /// The reader's place in its input: a cursor, so that a read the cursor
    /// and the reader share is one function that takes the place alone
    /// (`SceCursor::read_vle_inner`, docs/adr/0138 fifth amendment).
    place: SceCursor<'a>,
    fault: Option<CodecError>,
    over: bool,
}

impl<'a> SceReader<'a> {
    /// A reader at the start of `buf`, with nothing recorded.
    pub const fn new(buf: &'a [u8]) -> Self {
        Self {
            place: SceCursor::new(buf),
            fault: None,
            over: false,
        }
    }

    /// The bytes from where the reader stands to the end of its input: zero
    /// once the frame has failed, so a loop to the end of the frame stops.
    pub const fn remaining(&self) -> usize {
        self.place.remaining()
    }

    /// How many bytes of its input the reader has consumed.
    pub const fn position(&self) -> usize {
        self.place.position()
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
    ///
    /// One function, out of line, for every read of every codec: the refusal of
    /// each read is a call, not the record written again at each read
    /// (docs/adr/0137).
    #[cold]
    #[inline(never)]
    pub fn fail(&mut self, refusal: CodecError) {
        if self.fault.is_none() {
            self.fault = Some(refusal);
        }
        self.place.rest = &[];
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
            Err(refused) => {
                self.refuse_held(refused);
                T::default()
            }
        }
    }

    /// What [`Self::hold`] records of a value it does not hold: a capacity
    /// noted, any other refusal recorded. One function, out of line, for every
    /// held field of every codec, as [`Self::fail`] is (docs/adr/0137).
    #[cold]
    #[inline(never)]
    fn refuse_held(&mut self, refused: CodecError) {
        if matches!(refused, CodecError::CapacityExceeded) {
            self.over = true;
        } else {
            self.fail(refused);
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
        match self.place.rest.split_first_chunk::<N>() {
            Some((bytes, after)) => {
                self.place.rest = after;
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
        match self.place.rest.split_at_checked(n) {
            Some((bytes, after)) => {
                self.place.rest = after;
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
        let rest = self.place.rest;
        self.place.rest = &[];
        rest
    }

    /// Every byte that remains, not consumed: no bytes once the frame has
    /// failed.
    #[inline]
    pub fn rest(&self) -> &'a [u8] {
        self.place.rest
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
        match self.place.rest.first() {
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
        match self.place.rest.get(n..) {
            Some(after) => self.place.rest = after,
            None => self.fail(CodecError::NeedMoreBytes),
        }
    }

    /// One ZInt held to `max_bits`, consumed, and its refusal recorded: a value
    /// past `2^max_bits - 1` records `VleWidthOverflow` and reads as zero, a
    /// longer encoding of a value the width holds is that value, and a chain the
    /// input ends inside records `NeedMoreBytes` (see
    /// [`SceCursor::read_vle_u64`]).
    ///
    /// The reader's one read of a ZInt, out of line, for every width and every
    /// length. A ZInt of one or two bytes, what nearly every field holds, is
    /// answered here by `codec::read_zint_short`; a longer one is a call of
    /// the cursor's one ZInt read over the reader's place in its input
    /// ([`SceCursor::read_vle_inner`]), the program's one copy of the decode,
    /// so the two readers of a ZInt run one decode (docs/adr/0130, docs/adr/0138
    /// fifth amendment). A field is a call that answers the value, and none
    /// holds the body's answer or its record.
    #[inline(never)]
    fn read_vle_inner(&mut self, max_bits: u32) -> u64 {
        if let Some(value) = read_zint_short(&mut self.place.rest) {
            return value;
        }
        match self.place.read_vle_inner(max_bits) {
            (value, None) => value,
            (_, Some(refused)) => {
                self.fail(refused);
                0
            }
        }
    }

    /// The reader's ZInt read held in place where it is called: the `_inline`
    /// reads, which a body on the build's hot path holds so that a ZInt of one
    /// or two bytes is no call (docs/adr/0140). A longer one is a call of
    /// [`Self::read_vle_inner`], so no site holds the decode or the record of
    /// its refusal.
    #[inline(always)]
    fn read_vle_held(&mut self, max_bits: u32) -> u64 {
        match read_zint_short(&mut self.place.rest) {
            Some(value) => value,
            None => self.read_vle_inner(max_bits),
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

    /// [`Self::read_vle_u16`] held in place: the read a body on the build's
    /// hot path makes, with no call (docs/adr/0140). Every other body calls
    /// [`Self::read_vle_u16`], so its ZInts share one function.
    #[inline(always)]
    pub fn read_vle_u16_inline(&mut self) -> u16 {
        self.read_vle_held(16) as u16
    }

    /// [`Self::read_vle_u32`] held in place (see [`Self::read_vle_u16_inline`]).
    #[inline(always)]
    pub fn read_vle_u32_inline(&mut self) -> u32 {
        self.read_vle_held(32) as u32
    }

    /// [`Self::read_vle_u64`] held in place (see [`Self::read_vle_u16_inline`]).
    #[inline(always)]
    pub fn read_vle_u64_inline(&mut self) -> u64 {
        self.read_vle_held(64)
    }

    /// Where the reader stands, to come back to with [`Self::rewind`]: how a
    /// decode reads a span again.
    pub const fn mark(&self) -> usize {
        self.place.position()
    }

    /// The bytes consumed since `at`, a [`Self::mark`] of this reader: what one
    /// entry occupies (docs/adr/0130). Empty once the frame has failed, whose
    /// bytes from there are not the frame's.
    pub fn since(&self, at: usize) -> &'a [u8] {
        if self.fault.is_some() {
            return &[];
        }
        let buf = self.place.buf;
        buf.get(at..self.place.position()).unwrap_or(&[])
    }

    /// Go back to `at`, a [`Self::mark`] of this reader. What the reader
    /// recorded stays: a failed frame stays failed, and its input ended.
    pub fn rewind(&mut self, at: usize) {
        if self.fault.is_none() {
            if let Some(rest) = self.place.buf.get(at..) {
                self.place.rest = rest;
            }
        }
    }

    /// Read the next `n` bytes as a frame of their own through `read`, and
    /// consume them: the bytes a length says a body occupies. What `read`
    /// records is this frame's, the first refusal of the two counting first;
    /// where fewer than `n` bytes remain, `NeedMoreBytes` is recorded and
    /// `read` reads nothing. `n` is the length as its field holds it, of the
    /// field's own integer type ([`WireLength`]): a negative one, or one past
    /// what the target's `usize` holds, names no bytes the input can hold and
    /// is `LengthOutOfRange` ([`Self::begin_within`]), never cut to its low
    /// bits.
    ///
    /// It is generic over the read, so each body read within a length has its
    /// own copy, and that copy is the call of the read alone: the start of the
    /// inner frame is [`Self::begin_within`] and what it recorded is made this
    /// frame's by [`Self::end_within`], each one function for every codec of a
    /// program (docs/adr/0138).
    #[inline]
    pub fn within<T>(
        &mut self,
        n: impl WireLength,
        read: impl FnOnce(&mut SceReader<'a>) -> T,
    ) -> T {
        let mut inner = self.begin_within(n.byte_count());
        let value = read(&mut inner);
        self.end_within(&inner);
        value
    }

    /// The reader of the next `n` bytes as a frame of their own, consumed from
    /// this one ([`Self::within`]): it starts with this frame's refusal, its
    /// input ended, where this frame has failed or fewer than `n` bytes remain.
    /// Not generic over anything a decode reads, so it is one function, out of
    /// line, for every codec (docs/adr/0138).
    ///
    /// `n` is the count of bytes the length names, `None` where it names
    /// none (a negative length). A count past what the target's `usize` holds
    /// names more bytes than any input of the target holds; either is refused
    /// as out of range rather than cut to its low bits, which would read a
    /// different, shorter body. Where `usize` is 64 bits wide and the field
    /// unsigned, the test is never true and is no code.
    #[inline(never)]
    fn begin_within(&mut self, n: Option<u64>) -> SceReader<'a> {
        let bytes = match n.and_then(|n| usize::try_from(n).ok()) {
            Some(n) => self.take_slice(n),
            None => {
                self.fail(CodecError::LengthOutOfRange);
                &[]
            }
        };
        let mut inner = SceReader {
            place: SceCursor::new(bytes),
            fault: self.fault,
            over: false,
        };
        if inner.fault.is_some() {
            inner.place.rest = &[];
        }
        inner
    }

    /// What `inner`, begun by [`Self::begin_within`], recorded, made this
    /// frame's: its refusal, the first of the two counting first, and a value
    /// past a capacity. One function, out of line, for every codec, as
    /// [`Self::begin_within`] is (docs/adr/0138).
    #[inline(never)]
    fn end_within(&mut self, inner: &SceReader<'a>) {
        if let Some(refused) = inner.fault {
            self.fail(refused);
        }
        self.over |= inner.over;
    }
}

impl<'a> SceCursor<'a> {
    /// Decode one frame through `read`, over a [`SceReader`] that starts where
    /// this cursor stands, and answer what `read` returns under the reader's
    /// verdict: the value a view's read builds, or `()` for a read into a
    /// destination.
    ///
    /// The cursor moves past the frame where the frame was read whole: on
    /// `Ok`, and on `CapacityExceeded`, whose frame is well formed and read to
    /// its end (docs/adr/0077 §7). On any other refusal it does not move, so a
    /// caller that receives `NeedMoreBytes` reads the frame again from the same
    /// place once more bytes have arrived.
    ///
    /// It is generic over the read, so each decode has its own copy, inlined
    /// where the decode is, and that copy is the call of the read alone: the
    /// reader's start is [`Self::begin_frame`] (docs/adr/0138), and the verdict
    /// and the move of the cursor are [`Self::end_frame`] (docs/adr/0137), each
    /// one function for every decode of a program.
    #[inline]
    pub fn read_frame<T>(
        &mut self,
        read: impl FnOnce(&mut SceReader<'a>) -> T,
    ) -> Result<T, CodecError> {
        let mut reader = self.begin_frame();
        let value = read(&mut reader);
        self.end_frame(&reader).map(|()| value)
    }

    /// A reader over the bytes from where this cursor stands, with nothing
    /// recorded: the start of the frame [`Self::read_frame`] reads. Not generic
    /// over anything a decode reads, so it is one function, out of line, for
    /// every codec, as [`Self::end_frame`] is (docs/adr/0138).
    #[inline(never)]
    fn begin_frame(&self) -> SceReader<'a> {
        SceReader::new(self.rest)
    }

    /// The verdict of the frame `reader` read from where this cursor stands,
    /// and the cursor moved past it where it was read whole
    /// ([`Self::read_frame`]). Not generic over anything a decode reads, so it
    /// is one function, out of line, for every codec (docs/adr/0137).
    #[inline(never)]
    fn end_frame(&mut self, reader: &SceReader<'a>) -> Result<(), CodecError> {
        if let Some(refused) = reader.fault {
            return Err(refused);
        }
        // The reader was begun over this cursor's unread bytes
        // ([`Self::begin_frame`]), and a reader that refused nothing holds a
        // suffix of what it was begun over: what it left unread is what this
        // cursor leaves, with no position counted and no length tested again
        // (docs/adr/0143).
        self.rest = reader.place.rest;
        if reader.over {
            Err(CodecError::CapacityExceeded)
        } else {
            Ok(())
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
    fn a_frame_within_a_length_shares_its_capacity_and_starts_failed_after_a_refusal() {
        let bytes = [1u8, 2, 3, 4];
        // A value past a capacity inside the length is the outer frame's, and
        // the reads go on.
        let mut r = SceReader::new(&bytes);
        let head = r.within(2, |inner| {
            inner.note_capacity();
            inner.take_array::<1>()[0]
        });
        assert_eq!(head, 1);
        assert_eq!(r.position(), 2);
        assert!(r.exceeded() && !r.failed());
        assert_eq!(r.verdict(), Err(CodecError::CapacityExceeded));
        // After a refusal the inner frame starts with it: it has nothing to
        // read, reads zeros, and the outer verdict stays the first refusal.
        let mut failed = SceReader::new(&bytes);
        failed.fail(CodecError::InvalidUtf8);
        let seen = failed.within(2, |inner| {
            (
                inner.failed(),
                inner.remaining(),
                inner.take_array::<1>()[0],
            )
        });
        assert_eq!(seen, (true, 0, 0));
        assert_eq!(failed.verdict(), Err(CodecError::InvalidUtf8));
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
    fn a_zint_read_in_place_reads_as_its_call_reads_it() {
        // The reads a body on the build's hot path holds in place run the
        // reader's one decode, so they answer what the reads every other body
        // calls answer: the value, the verdict and the bytes consumed, at every
        // length of chain, cut at every byte, at every width (docs/adr/0140).
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
                    let mut called = SceReader::new(bytes);
                    let mut held = SceReader::new(bytes);
                    let (by_call, in_place) = match bits {
                        16 => (
                            u64::from(called.read_vle_u16()),
                            u64::from(held.read_vle_u16_inline()),
                        ),
                        32 => (
                            u64::from(called.read_vle_u32()),
                            u64::from(held.read_vle_u32_inline()),
                        ),
                        _ => (called.read_vle_u64(), held.read_vle_u64_inline()),
                    };
                    assert_eq!(
                        (in_place, held.verdict(), held.position()),
                        (by_call, called.verdict(), called.position()),
                        "{bytes:?} at {bits}"
                    );
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

    #[test]
    fn a_frame_answers_the_value_its_read_returns_under_the_verdict() {
        let bytes = [4u8, 5, 6];
        let mut cursor = SceCursor::new(&bytes);
        assert_eq!(cursor.read_frame(|r| r.take_array::<2>()[1]), Ok(5));
        assert_eq!(cursor.remaining(), 1);
        // A refused frame answers its refusal, not the zeros its read returned,
        // and leaves the cursor where it was.
        assert_eq!(
            cursor.read_frame(|r| r.take_array::<2>()[0]),
            Err(CodecError::NeedMoreBytes)
        );
        assert_eq!(cursor.remaining(), 1);
        // A frame past a capacity answers the capacity and is read whole.
        assert_eq!(
            cursor.read_frame(|r| {
                r.note_capacity();
                r.take_array::<1>()[0]
            }),
            Err(CodecError::CapacityExceeded)
        );
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn a_held_value_past_its_capacity_is_noted_and_any_other_refusal_recorded() {
        let mut r = SceReader::new(&[]);
        assert_eq!(r.hold::<u8>(Err(CodecError::CapacityExceeded)), 0);
        assert!(r.exceeded() && !r.failed());
        assert_eq!(r.hold::<u8>(Err(CodecError::InvalidUtf8)), 0);
        assert!(r.failed());
        assert_eq!(r.hold(Ok::<u8, CodecError>(3)), 3);
        assert_eq!(r.verdict(), Err(CodecError::InvalidUtf8));
    }
    #[test]
    fn a_negative_length_is_out_of_range_and_reads_nothing() {
        let bytes = [1u8, 2, 3];
        let mut r = SceReader::new(&bytes);
        let read = r.within(-1i8, |inner| inner.take_array::<1>()[0]);
        assert_eq!(read, 0);
        assert_eq!(r.verdict(), Err(CodecError::LengthOutOfRange));
    }

    #[test]
    fn a_length_is_read_whole_whatever_its_type() {
        let bytes = [7u8, 8, 9];
        for n in [2u64.byte_count(), 2u8.byte_count(), 2i64.byte_count()] {
            let mut r = SceReader::new(&bytes);
            let head = r.within(n.unwrap(), |inner| inner.take_array::<2>()[1]);
            assert_eq!((head, r.position(), r.verdict()), (8, 2, Ok(())));
        }
    }
}
