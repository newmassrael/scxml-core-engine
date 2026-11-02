// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The containers of a heap node's policy limit (docs/adr/0077 §7).
//!
//! A deployment on a node with an allocator still states how much it holds of
//! a field: a policy limit, which the ADR counts as a capacity like an inline
//! node's array. These containers grow on the heap as a frame is read and
//! refuse past their limit `N` with [`CodecError::CapacityExceeded`], as
//! `Inline`'s refuse past theirs, so a storing decode reads the frame on past
//! the limit and answers the capacity last on either kind of node.
//!
//! They are apart from `HeapStr<N>` and `HeapBytes<N>`, whose `N` is
//! advisory: the algorithm kind holds its record fields in those and relies on
//! that.

use crate::bounded::{Entries, Fewest};
use crate::codec::CodecError;
use crate::storage::{GrowBytes, GrowStr, HeldList, SceByteBuf, SceList, SceStr};

/// A list on the heap of at most `N` entries.
#[derive(Debug, Clone, PartialEq)]
pub struct CappedVec<T, const N: usize>(alloc::vec::Vec<T>);

impl<T, const N: usize> Default for CappedVec<T, N> {
    fn default() -> Self {
        Self(alloc::vec::Vec::new())
    }
}

impl<T, const N: usize> core::ops::Deref for CappedVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<T, const N: usize> SceList<T> for CappedVec<T, N> {
    fn empty() -> Self {
        Self::default()
    }

    fn try_push(&mut self, value: T) -> Result<(), CodecError> {
        if self.0.len() == N {
            return Err(CodecError::CapacityExceeded);
        }
        self.0.push(value);
        Ok(())
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn as_slice(&self) -> &[T] {
        &self.0
    }

    fn last_mut(&mut self) -> Option<&mut T> {
        self.0.last_mut()
    }

    fn remove_at(&mut self, at: usize) {
        self.0.remove(at);
    }
}

impl<T, const N: usize> HeldList for CappedVec<T, N> {
    type Entry = T;
}

impl<T, const N: usize> Entries for CappedVec<T, N> {
    type Iter<'s>
        = core::slice::Iter<'s, T>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.0.len()
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.0.iter()
    }
}

impl<T: Default, const N: usize> Fewest for CappedVec<T, N> {
    fn fewest<const MIN: usize>() -> Self {
        const { assert!(MIN <= N, "the list cannot hold its fewest entries") };
        Self((0..MIN).map(|_| T::default()).collect())
    }
}

impl<T, const N: usize> IntoIterator for CappedVec<T, N> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A text on the heap of at most `N` bytes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CappedStr<const N: usize>(GrowStr);

impl<const N: usize> core::ops::Deref for CappedStr<N> {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

impl<const N: usize> PartialEq<&str> for CappedStr<N> {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_str() == *other
    }
}

impl<const N: usize> SceStr for CappedStr<N> {
    fn from_view(s: &str) -> Result<Self, CodecError> {
        if s.len() > N {
            return Err(CodecError::CapacityExceeded);
        }
        Ok(Self(GrowStr::from_view(s)?))
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn try_push_str(&mut self, s: &str) -> Result<(), CodecError> {
        if s.len() > N - self.0.as_str().len() {
            return Err(CodecError::CapacityExceeded);
        }
        <GrowStr as SceStr>::try_push_str(&mut self.0, s)
    }
}

impl<const N: usize> Fewest for CappedStr<N> {
    /// `MIN` NULs, as every text a profile owns.
    fn fewest<const MIN: usize>() -> Self {
        const { assert!(MIN <= N, "a field's capacity holds its fewest bytes") };
        Self(GrowStr::fewest::<MIN>())
    }
}

/// Bytes on the heap, at most `N` of them.
#[derive(Debug, Clone, PartialEq)]
pub struct CappedBytes<const N: usize>(GrowBytes);

impl<const N: usize> core::ops::Deref for CappedBytes<N> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const N: usize> SceByteBuf for CappedBytes<N> {
    fn from_slice(b: &[u8]) -> Result<Self, CodecError> {
        if b.len() > N {
            return Err(CodecError::CapacityExceeded);
        }
        Ok(Self(<GrowBytes as SceByteBuf>::from_slice(b)?))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const N: usize> Fewest for CappedBytes<N> {
    /// `MIN` zeros, as every byte buffer a profile owns.
    fn fewest<const MIN: usize>() -> Self {
        const { assert!(MIN <= N, "a field's capacity holds its fewest bytes") };
        Self(GrowBytes::fewest::<MIN>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capped_list_grows_to_its_limit_and_refuses_past_it() {
        let mut list = CappedVec::<u8, 3>::empty();
        for n in 0..3 {
            assert_eq!(list.try_push(n), Ok(()));
        }
        assert_eq!(list.try_push(3), Err(CodecError::CapacityExceeded));
        assert_eq!(list.as_slice(), &[0, 1, 2]);
    }

    #[test]
    fn a_capped_text_holds_its_limit_and_is_as_it_was_past_it() {
        assert_eq!(CappedStr::<4>::from_view("abcd").map(|s| s.len()), Ok(4));
        assert_eq!(
            CappedStr::<4>::from_view("abcde"),
            Err(CodecError::CapacityExceeded)
        );
        let mut text = CappedStr::<4>::from_view("ab").expect("two bytes");
        assert_eq!(text.try_push_str("cde"), Err(CodecError::CapacityExceeded));
        assert_eq!(text, "ab");
        assert_eq!(text.try_push_str("cd"), Ok(()));
        assert_eq!(text, "abcd");
    }

    #[test]
    fn capped_bytes_hold_their_limit_and_refuse_past_it() {
        assert_eq!(
            CappedBytes::<2>::from_slice(&[1, 2]).map(|b| b.len()),
            Ok(2)
        );
        assert_eq!(
            CappedBytes::<2>::from_slice(&[1, 2, 3]),
            Err(CodecError::CapacityExceeded)
        );
    }

    #[test]
    fn a_limit_past_any_inline_size_is_held_on_the_heap() {
        // 1 MiB fits no array on an MCU stack and is a policy a heap node may
        // state: the container reserves nothing up front.
        let big = CappedStr::<{ 1 << 20 }>::from_view(&"x".repeat(300)).expect("held");
        assert_eq!(big.len(), 300);
    }
}
