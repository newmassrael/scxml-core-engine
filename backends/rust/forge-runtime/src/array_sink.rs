// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A write the build proves fits an array, and the bytes it wrote
//! (docs/adr/0065 §4, amendment of 2026-11-24).
//!
//! [`SliceSink`](crate::codec::SliceSink) writes into storage its caller holds
//! and refuses with `BufferOverflow` once the storage is full: whether a frame
//! fits is found at run time, so every write through it can fail. A write whose
//! largest size is known when the code is built has no such case once the
//! storage is at least that large, and its result says so here.
//!
//! [`BoundedWrite`] is such a write. It states the most and the fewest bytes it
//! writes as constants, and every constant is derived from the write's parts: a
//! byte is one, a VLE ([`Vle`]) is [`vle_len`](crate::codec::vle_len) of the
//! largest value of its width, an array is its length times its entry, a tuple
//! the sum of its members. The trait is sealed, so no write states a bound its
//! bytes do not keep.
//!
//! [`ArraySink`] is an array of `N` bytes a bounded write is written into. The
//! build admits [`ArraySink::written`] only where the write's most is at most
//! `N`, so the sink the write goes through cannot refuse (its error is
//! `Infallible`), and the bytes it holds lie between the write's fewest and `N`.
//! A bounded field is made of them with no check: borrowed, as
//! [`Bytes`](crate::bounded::Bytes) through `From<&ArraySink>`, and owned, as
//! [`ArrayBytes`](crate::storage::ArrayBytes) through
//! [`Bounded::from_sink`](crate::bounded::Bounded::from_sink).

use core::convert::Infallible;

use crate::codec::{vle_len_u16, vle_len_u32, vle_len_u64, SceSink};

mod sealed {
    /// How a bounded write puts its bytes into its array. This crate's alone,
    /// so the constants of every [`super::BoundedWrite`] are derived from bytes
    /// this crate writes.
    pub trait Sealed {
        /// Write the bytes into `out`, which has room for at least the most
        /// bytes the write states.
        fn put(&self, out: &mut Fill<'_>);
    }

    /// The array a bounded write is put into, and how many of its bytes are
    /// written.
    pub struct Fill<'a> {
        pub(super) buf: &'a mut [u8],
        pub(super) len: usize,
    }
}

/// A write that states the most and the fewest bytes it writes, as constants
/// derived from its parts: a `u8`, a [`Vle`], an array of bounded writes, a
/// tuple of up to four, and a reference to one.
///
/// Sealed: a write outside this crate is made of these parts, so its constants
/// are derived and never claimed, and the array [`ArraySink::written`] writes
/// it into always has room for it.
///
/// ```
/// use sce_forge_runtime::codec::{BoundedWrite, Vle};
/// // Eight pairs of 64-bit VLEs: at most nine bytes each, at least one.
/// type Pairs = [[Vle<u64>; 2]; 8];
/// assert_eq!(<Pairs as BoundedWrite>::MAX_BYTES, 144);
/// assert_eq!(<Pairs as BoundedWrite>::MIN_BYTES, 16);
/// ```
pub trait BoundedWrite: sealed::Sealed {
    /// The most bytes the write puts, whatever the values it holds.
    const MAX_BYTES: usize;
    /// The fewest bytes the write puts, whatever the values it holds.
    const MIN_BYTES: usize;
}

/// A value written as a VLE of its type's width, as
/// [`SceSink::write_vle_u16`] and its kin write it: one to three bytes for a
/// `u16`, one to five for a `u32`, one to nine for a `u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vle<T>(pub T);

/// A VLE of one width: written through the sink's own method, and bounded by
/// the length that method writes for the width's largest value.
macro_rules! vle_write {
    ($ty:ty, $write:ident, $len:ident) => {
        impl sealed::Sealed for Vle<$ty> {
            fn put(&self, out: &mut sealed::Fill<'_>) {
                let Ok(()) = out.$write(self.0);
            }
        }

        impl BoundedWrite for Vle<$ty> {
            const MAX_BYTES: usize = $len(<$ty>::MAX);
            const MIN_BYTES: usize = 1;
        }
    };
}

vle_write!(u16, write_vle_u16, vle_len_u16);
vle_write!(u32, write_vle_u32, vle_len_u32);
vle_write!(u64, write_vle_u64, vle_len_u64);

impl sealed::Sealed for u8 {
    fn put(&self, out: &mut sealed::Fill<'_>) {
        let Ok(()) = out.write_u8(*self);
    }
}

impl BoundedWrite for u8 {
    const MAX_BYTES: usize = 1;
    const MIN_BYTES: usize = 1;
}

impl<W: BoundedWrite, const K: usize> sealed::Sealed for [W; K] {
    fn put(&self, out: &mut sealed::Fill<'_>) {
        for part in self {
            sealed::Sealed::put(part, out);
        }
    }
}

impl<W: BoundedWrite, const K: usize> BoundedWrite for [W; K] {
    const MAX_BYTES: usize = K * W::MAX_BYTES;
    const MIN_BYTES: usize = K * W::MIN_BYTES;
}

impl<W: BoundedWrite> sealed::Sealed for &W {
    fn put(&self, out: &mut sealed::Fill<'_>) {
        sealed::Sealed::put(*self, out);
    }
}

impl<W: BoundedWrite> BoundedWrite for &W {
    const MAX_BYTES: usize = W::MAX_BYTES;
    const MIN_BYTES: usize = W::MIN_BYTES;
}

/// A tuple of bounded writes, written member by member and bounded by their
/// sum.
macro_rules! tuple_write {
    ($first:ident . $first_at:tt $(, $part:ident . $at:tt)+) => {
        impl<$first: BoundedWrite $(, $part: BoundedWrite)+> sealed::Sealed
            for ($first $(, $part)+)
        {
            fn put(&self, out: &mut sealed::Fill<'_>) {
                sealed::Sealed::put(&self.$first_at, out);
                $(sealed::Sealed::put(&self.$at, out);)+
            }
        }

        impl<$first: BoundedWrite $(, $part: BoundedWrite)+> BoundedWrite
            for ($first $(, $part)+)
        {
            const MAX_BYTES: usize = $first::MAX_BYTES $(+ $part::MAX_BYTES)+;
            const MIN_BYTES: usize = $first::MIN_BYTES $(+ $part::MIN_BYTES)+;
        }
    };
}

tuple_write!(A.0, B.1);
tuple_write!(A.0, B.1, C.2);
tuple_write!(A.0, B.1, C.2, D.3);

impl SceSink for sealed::Fill<'_> {
    /// The array has room for the write's most, which the build checked, so
    /// there is no room to run out of.
    type Error = Infallible;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Infallible> {
        // Every byte a bounded write puts has its place: the write's most is at
        // most the array's length, which `ArraySink::written` asserts when the
        // build reaches it, and each part puts no more than its constant says.
        // The range is still spelled so that no write can leave the array.
        // A part whose constant understated what it puts would lose bytes
        // here, so a build with debug assertions (every test of this crate)
        // stops on it rather than keep a short write.
        let start = self.len;
        let dst = start
            .checked_add(bytes.len())
            .and_then(|end| self.buf.get_mut(start..end));
        debug_assert!(
            dst.is_some(),
            "a bounded write put more bytes than its MAX_BYTES"
        );
        if let Some(dst) = dst {
            dst.copy_from_slice(bytes);
            self.len = start + bytes.len();
        }
        Ok(())
    }

    fn position(&self) -> usize {
        self.len
    }
}

/// An array of `N` bytes and the bounded write written into it, which holds at
/// least `MIN` of them: the sink of a write that cannot refuse.
///
/// The only way to hold one is [`Self::written`], which the build admits only
/// where the write's most is at most `N` and its fewest at least `MIN`, so the
/// value says "these bytes are the write's, and their length lies between
/// `MIN` and `N`". A bounded field is made of them with no check: borrowed
/// through `From<&ArraySink>` for [`Bytes`](crate::bounded::Bytes), owned
/// through [`Bounded::from_sink`](crate::bounded::Bounded::from_sink).
///
/// ```
/// use sce_forge_runtime::codec::{ArraySink, Bytes, Vle};
/// let sns: [[Vle<u64>; 2]; 2] = [[Vle(1), Vle(2)], [Vle(300), Vle(4)]];
/// let body: ArraySink<36, 4> = ArraySink::written(&sns);
/// assert_eq!(body.as_slice(), &[1u8, 2, 0xAC, 0x02, 4][..]);
/// let field: Bytes<'_, 64, 1> = (&body).into();
/// assert_eq!(field, [1u8, 2, 0xAC, 0x02, 4]);
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ArraySink<const N: usize, const MIN: usize = 0> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize, const MIN: usize> ArraySink<N, MIN> {
    /// The bytes `write` writes, in an array of `N`. The build admits it only
    /// where the array has room for the write's most and the write's fewest is
    /// at least `MIN`, so nothing is checked and nothing can be refused.
    ///
    /// An array without room for the write's most does not build:
    ///
    /// ```compile_fail,E0080
    /// use sce_forge_runtime::codec::{ArraySink, Vle};
    /// let _: ArraySink<8> = ArraySink::written(&Vle(1u64));
    /// ```
    ///
    /// nor does a write whose fewest is below the sink's:
    ///
    /// ```compile_fail,E0080
    /// use sce_forge_runtime::codec::{ArraySink, Vle};
    /// let _: ArraySink<9, 2> = ArraySink::written(&Vle(1u64));
    /// ```
    pub fn written<W: BoundedWrite>(write: &W) -> Self {
        const {
            assert!(
                W::MAX_BYTES <= N,
                "the array has no room for the write's most bytes"
            )
        };
        const {
            assert!(
                W::MIN_BYTES >= MIN,
                "the write's fewest bytes are below the sink's"
            )
        };
        let mut bytes = [0u8; N];
        let mut fill = sealed::Fill {
            buf: &mut bytes,
            len: 0,
        };
        sealed::Sealed::put(write, &mut fill);
        let len = fill.len;
        Self { bytes, len }
    }

    /// The bytes written.
    pub fn as_slice(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or_default()
    }

    /// How many bytes were written, at least `MIN` and at most `N`.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the write put no byte.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize, const MIN: usize> core::fmt::Debug for ArraySink<N, MIN> {
    /// The bytes written, as a slice prints: the array's size is the type's.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_slice(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounded::{Bounded, Bytes};
    use crate::codec::SliceSink;
    use crate::storage::ArrayBytes;

    #[test]
    fn a_vle_states_the_most_and_fewest_bytes_its_width_writes() {
        assert_eq!(<Vle<u16> as BoundedWrite>::MAX_BYTES, 3);
        assert_eq!(<Vle<u32> as BoundedWrite>::MAX_BYTES, 5);
        assert_eq!(<Vle<u64> as BoundedWrite>::MAX_BYTES, 9);
        assert_eq!(<Vle<u16> as BoundedWrite>::MIN_BYTES, 1);
        assert_eq!(<Vle<u64> as BoundedWrite>::MIN_BYTES, 1);
        // The most is reached: the largest value of each width fills it.
        let wide: ArraySink<3> = ArraySink::written(&Vle(u16::MAX));
        assert_eq!(wide.len(), 3);
        let wide: ArraySink<5> = ArraySink::written(&Vle(u32::MAX));
        assert_eq!(wide.len(), 5);
        let wide: ArraySink<9> = ArraySink::written(&Vle(u64::MAX));
        assert_eq!(wide.len(), 9);
        // And the fewest: a value of seven bits is one byte of itself.
        let one: ArraySink<9, 1> = ArraySink::written(&Vle(0x7Fu64));
        assert_eq!(one.as_slice(), &[0x7Fu8][..]);
    }

    #[test]
    fn the_bytes_are_those_the_codecs_bounded_sink_writes() {
        let values = [
            0u64,
            0x7F,
            0x80,
            0x3FFF,
            0x4000,
            u64::from(u32::MAX),
            u64::MAX - 1,
            u64::MAX,
        ];
        let sink: ArraySink<72, 8> = ArraySink::written(&values.map(Vle));
        let mut expected = [0u8; 72];
        let mut through = SliceSink::new(&mut expected);
        for value in values {
            through
                .write_vle_u64(value)
                .expect("an array of the most bytes holds them");
        }
        let written = through.into_written();
        assert_eq!(sink.as_slice(), &written[..]);
        assert_eq!(sink.len(), written.len());
    }

    #[test]
    fn an_array_of_pairs_is_bounded_by_its_count_and_written_in_order() {
        type Pairs = [[Vle<u64>; 2]; 8];
        assert_eq!(<Pairs as BoundedWrite>::MAX_BYTES, 8 * 2 * 9);
        assert_eq!(<Pairs as BoundedWrite>::MIN_BYTES, 8 * 2);
        // Every value at its widest fills the array exactly.
        let widest: Pairs = [[Vle(u64::MAX); 2]; 8];
        let full: ArraySink<144, 16> = ArraySink::written(&widest);
        assert_eq!(full.len(), 144);
        // Small values take a byte each, the first of each pair first.
        let small: Pairs = core::array::from_fn(|at| {
            let at = at as u64;
            [Vle(at), Vle(100 + at)]
        });
        let sink: ArraySink<144, 16> = ArraySink::written(&small);
        assert_eq!(sink.len(), 16);
        assert_eq!(sink.as_slice().get(..4), Some(&[0u8, 100, 1, 101][..]));
    }

    #[test]
    fn a_tuple_is_bounded_by_the_sum_of_its_members() {
        type Header = (u8, Vle<u16>, [u8; 2]);
        assert_eq!(<Header as BoundedWrite>::MAX_BYTES, 1 + 3 + 2);
        assert_eq!(<Header as BoundedWrite>::MIN_BYTES, 1 + 1 + 2);
        let header: Header = (0x51, Vle(300), [0xAA, 0xBB]);
        let sink: ArraySink<6, 4> = ArraySink::written(&header);
        assert_eq!(sink.as_slice(), &[0x51u8, 0xAC, 0x02, 0xAA, 0xBB][..]);
        // A reference writes, and is bounded as, what it refers to.
        let by_ref: ArraySink<6, 4> = ArraySink::written(&(&header.0, &header.1, &header.2));
        assert_eq!(by_ref, sink);
        let four: ArraySink<4, 4> = ArraySink::written(&(1u8, 2u8, 3u8, 4u8));
        assert_eq!(four.as_slice(), &[1u8, 2, 3, 4][..]);
    }

    #[test]
    fn a_bounded_field_is_made_of_the_written_bytes_without_a_check() {
        let sink: ArraySink<9, 1> = ArraySink::written(&Vle(300u64));
        // Borrowed, as a codec's source presents a `bytes` field.
        let borrowed: Bytes<'_, 16, 1> = (&sink).into();
        assert_eq!(borrowed, [0xACu8, 0x02]);
        // Owned, as a value holds it, and equal to the same bytes held by a check.
        let owned = Bounded::<ArrayBytes<9>, 16, 1>::from_sink(&sink);
        assert_eq!(owned.as_bytes(), borrowed);
        assert_eq!(
            Bounded::<ArrayBytes<9>, 16, 1>::from_slice(&[0xAC, 0x02]),
            Ok(owned)
        );
        // A write of no byte leaves the array, and the field, empty.
        let none: ArraySink<4> = ArraySink::written(&[0u8; 0]);
        assert!(none.is_empty());
        assert!(Bounded::<ArrayBytes<4>, 4>::from_sink(&none)
            .as_bytes()
            .is_empty());
    }
}
