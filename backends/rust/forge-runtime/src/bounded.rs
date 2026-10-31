// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Lists of at most a bound (docs/adr/0065 §6 R2).
//!
//! A codec's list holds at most its `sce:max-count`, a chain at most its
//! `max-depth`: the count field or the chain's continuation bit cannot say more,
//! and the decoder refuses more. A list with more entries has no frame. These
//! types hold a list together with the fact that it has no more entries than its
//! bound, so a source made of them cannot present a list the encode would have to
//! refuse.
//!
//! [`Bounded`] holds a list in memory (a slice, a `heapless::Vec`, a profile's
//! owned list, a view's list over bytes) and is checked once, by [`Bounded::new`].
//! [`BoundedIter`] is what a codec's source interface returns: the entries in
//! order with their count, which the codec writes as the count field or reads
//! for each entry's continuation bit, and does not count again.
//!
//! The bound is a property of the wire, not of where the entries are kept, so it
//! is a type of its own and the storage profiles stay what they are: a heap
//! profile's list grows past its declared capacity, and the bound that keeps it
//! encodable is this type around it.

use core::ops::Deref;

use crate::view::{ListIter, ListView, ListWalk};

/// A list a [`Bounded`] can hold: its entries in order, and how many there are.
pub trait Entries {
    /// The entries in order. Walking a clone of it gives the same entries again.
    type Iter<'s>: Iterator + Clone
    where
        Self: 's;

    /// How many entries the list holds.
    fn entry_count(&self) -> usize;

    /// The entries, from the first.
    fn entries(&self) -> Self::Iter<'_>;
}

impl<'a, T> Entries for &'a [T] {
    type Iter<'s>
        = core::slice::Iter<'a, T>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.len()
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<T, const N: usize> Entries for heapless::Vec<T, N> {
    type Iter<'s>
        = core::slice::Iter<'s, T>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.len()
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

#[cfg(feature = "alloc")]
impl<T> Entries for alloc::vec::Vec<T> {
    type Iter<'s>
        = core::slice::Iter<'s, T>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.len()
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<'a, W: ListWalk<K>, const K: usize> Entries for ListView<'a, W, K> {
    type Iter<'s>
        = ListIter<'a, W, K>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.len()
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

/// A list `L` of at most `MAX` entries.
///
/// The only ways to hold one are [`Self::new`], which counts the entries, and the
/// conversions whose source cannot hold more than `MAX`, which the build checks.
/// It reads as the list it holds ([`Deref`]); it has no mutable access, so its
/// entries cannot grow past the bound once it is made.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounded<L, const MAX: usize>(L);

impl<L: core::fmt::Debug, const MAX: usize> core::fmt::Debug for Bounded<L, MAX> {
    /// The list as it prints: the bound is a fact about it, not more of it, so a
    /// value holding a `Bounded` prints as one holding the list does.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<L: Entries, const MAX: usize> Bounded<L, MAX> {
    /// `list` as a list of at most `MAX` entries, or `None` when it holds more.
    pub fn new(list: L) -> Option<Self> {
        if list.entry_count() <= MAX {
            Some(Self(list))
        } else {
            None
        }
    }

    /// The entries in order, with their count.
    pub fn iter(&self) -> BoundedIter<L::Iter<'_>, MAX> {
        BoundedIter {
            iter: self.0.entries(),
            remaining: self.0.entry_count(),
        }
    }

    /// How many entries the list holds, at most `MAX`.
    pub fn len(&self) -> usize {
        self.0.entry_count()
    }

    /// Whether the list holds no entry.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<L: Entries, const MAX: usize> Bounded<L, MAX> {
    /// The entries of a storage profile's owned list, with their count.
    ///
    /// [`Self::iter`] over a profile's list `S::List<T, N>` names its entries as
    /// the profile's iterator yields them, which a generic owned form cannot read
    /// as `&T`; this reads them through [`crate::storage::SceList::as_slice`]. The
    /// count is still the one [`Self::new`] checked, so a list whose slice and
    /// count disagree is caught by the walk and never written past its bound.
    pub fn list_iter<T>(&self) -> BoundedIter<core::slice::Iter<'_, T>, MAX>
    where
        L: crate::storage::SceList<T>,
    {
        BoundedIter {
            iter: self.0.as_slice().iter(),
            remaining: self.0.entry_count(),
        }
    }
}

impl<L, const MAX: usize> Bounded<L, MAX> {
    /// The list. Named so that it leaves the list's own `get(index)` reachable
    /// through [`Deref`].
    pub fn inner(&self) -> &L {
        &self.0
    }

    /// The list, without its bound.
    pub fn into_inner(self) -> L {
        self.0
    }
}

impl<L, const MAX: usize> Deref for Bounded<L, MAX> {
    type Target = L;

    fn deref(&self) -> &L {
        &self.0
    }
}

impl<L: Entries + Default, const MAX: usize> Default for Bounded<L, MAX> {
    /// The empty list, which every bound admits.
    fn default() -> Self {
        let empty = L::default();
        assert!(empty.entry_count() == 0, "a list's default holds entries");
        Self(empty)
    }
}

impl<'a, T, const N: usize, const MAX: usize> From<&'a [T; N]> for Bounded<&'a [T], MAX> {
    /// An array of `N` entries, which the build admits only where `N` is at most
    /// `MAX`: a list of fixed length costs no check.
    fn from(entries: &'a [T; N]) -> Self {
        const {
            assert!(
                N <= MAX,
                "the array holds more entries than the list's bound"
            )
        };
        Self(entries)
    }
}

impl<T, const N: usize, const MAX: usize> From<heapless::Vec<T, N>>
    for Bounded<heapless::Vec<T, N>, MAX>
{
    /// A `heapless::Vec` that cannot hold more than `MAX`, which the build checks
    /// from its capacity.
    fn from(list: heapless::Vec<T, N>) -> Self {
        const { assert!(N <= MAX, "the list's capacity is past the list's bound") };
        Self(list)
    }
}

/// The entries of a list of at most `MAX`, in order, with how many there are: what
/// a codec's source interface returns for a list (docs/adr/0065 §6 R2).
///
/// A codec reads [`ExactSizeIterator::len`] before the first entry, for the count
/// it writes or the continuation bit of each entry, and then walks the entries
/// once. A source whose entries, walked again, are more or fewer than it counted
/// has broken what an accessor is (a read of a value, docs/adr/0061 §3), and the
/// walk panics rather than write a frame whose count disagrees with its entries.
#[derive(Debug, Clone)]
pub struct BoundedIter<I, const MAX: usize> {
    iter: I,
    remaining: usize,
}

impl<'s, T, const N: usize, const MAX: usize> From<&'s heapless::Vec<T, N>>
    for BoundedIter<core::slice::Iter<'s, T>, MAX>
{
    /// The entries of a `heapless::Vec` that cannot hold more than `MAX`, which
    /// the build checks from its capacity: a decoded list, whose capacity is its
    /// bound.
    fn from(list: &'s heapless::Vec<T, N>) -> Self {
        const { assert!(N <= MAX, "the list's capacity is past the list's bound") };
        Self {
            iter: list.iter(),
            remaining: list.len(),
        }
    }
}

/// Project each entry of a decoded list into an owned list of the profile `L`,
/// keeping its bound: the projection writes as many entries as it reads, and the
/// decoded list cannot hold more than `MAX` (its capacity `N`, which the build
/// checks).
///
/// The owned list's own capacity may still refuse an entry, on a profile whose
/// capacity is below the decoded list's, which is
/// [`crate::storage::try_collect_list`]'s error.
pub fn try_collect_bounded<L, T, U, const N: usize, const MAX: usize>(
    src: heapless::Vec<T, N>,
    f: impl FnMut(T) -> Result<U, crate::codec::CodecError>,
) -> Result<Bounded<L, MAX>, crate::codec::CodecError>
where
    L: crate::storage::SceList<U>,
{
    const { assert!(N <= MAX, "the list's capacity is past the list's bound") };
    crate::storage::try_collect_list(src, f).map(Bounded)
}

impl<I: Iterator + Clone, const MAX: usize> BoundedIter<I, MAX> {
    /// `entries` as the entries of a list of at most `MAX`, counted once from a
    /// clone, or `None` when there are more.
    pub fn new(entries: I) -> Option<Self> {
        let count = entries.clone().count();
        if count <= MAX {
            Some(Self {
                iter: entries,
                remaining: count,
            })
        } else {
            None
        }
    }
}

impl<I: Iterator, const MAX: usize> BoundedIter<I, MAX> {
    /// Each entry through `f`: as many entries as before, so the same bound.
    ///
    /// It stands in for [`Iterator::map`], which would drop the count, so that a
    /// source keeping its entries in its own type presents them as the codec's
    /// with the call it would write anyway: `self.list.iter().map(Entry)`.
    pub fn map<B, F: FnMut(I::Item) -> B>(self, f: F) -> BoundedIter<core::iter::Map<I, F>, MAX> {
        BoundedIter {
            iter: self.iter.map(f),
            remaining: self.remaining,
        }
    }
}

impl<I: Iterator, const MAX: usize> Iterator for BoundedIter<I, MAX> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.remaining == 0 {
            if self.iter.next().is_some() {
                entries_changed();
            }
            return None;
        }
        self.remaining -= 1;
        match self.iter.next() {
            Some(entry) => Some(entry),
            None => entries_changed(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<I: Iterator, const MAX: usize> ExactSizeIterator for BoundedIter<I, MAX> {}

/// The one panic of a list walk, shared by every instance so that each costs a
/// call and not a message of its own.
#[cold]
#[inline(never)]
fn entries_changed() -> ! {
    panic!("a list's entries changed between its count and its walk (docs/adr/0061 §3)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_within_its_bound_is_held_and_one_past_it_is_not() {
        let entries = [1u8, 2, 3];
        assert_eq!(
            Bounded::<&[u8], 3>::new(&entries[..]).map(|l| l.len()),
            Some(3)
        );
        assert!(Bounded::<&[u8], 2>::new(&entries[..]).is_none());
        let empty: &[u8] = &[];
        assert!(Bounded::<&[u8], 0>::new(empty).is_some());
    }

    #[test]
    fn an_array_converts_without_a_check_and_reads_as_its_slice() {
        let list: Bounded<&[u8], 4> = (&[7u8, 8]).into();
        assert_eq!(list.len(), 2);
        assert_eq!(*list.inner(), &[7u8, 8][..]);
        assert_eq!(list.get(1), Some(&8));
        assert_eq!(list.first(), Some(&7));
    }

    #[test]
    fn the_walk_yields_the_entries_with_their_count() {
        let entries = [1u8, 2, 3];
        let list = Bounded::<&[u8], 8>::new(&entries[..]).unwrap();
        let walk = list.iter();
        assert_eq!(walk.len(), 3);
        assert!(walk.clone().copied().eq([1, 2, 3]));
        let counted = BoundedIter::<_, 3>::new(entries.iter()).unwrap();
        assert_eq!(counted.len(), 3);
        assert!(BoundedIter::<_, 2>::new(entries.iter()).is_none());
    }

    #[test]
    fn a_heapless_list_converts_where_its_capacity_is_inside_the_bound() {
        let mut list = heapless::Vec::<u8, 4>::new();
        list.push(5).unwrap();
        let bounded: Bounded<heapless::Vec<u8, 4>, 4> = list.into();
        assert_eq!(bounded.iter().copied().next(), Some(5));
    }

    /// A source whose entries are not the same on each walk: one entry on the
    /// count, the number of walks so far on the walk.
    #[derive(Clone)]
    struct Growing<'c> {
        walks: &'c core::cell::Cell<usize>,
        left: usize,
        started: bool,
    }

    impl Iterator for Growing<'_> {
        type Item = ();
        fn next(&mut self) -> Option<()> {
            if !self.started {
                self.started = true;
                self.walks.set(self.walks.get() + 1);
                self.left = self.walks.get();
            }
            if self.left == 0 {
                return None;
            }
            self.left -= 1;
            Some(())
        }
    }

    #[test]
    #[should_panic(expected = "a list's entries changed between its count and its walk")]
    fn a_walk_longer_than_its_count_panics() {
        let walks = core::cell::Cell::new(0);
        let source = Growing {
            walks: &walks,
            left: 0,
            started: false,
        };
        let entries = BoundedIter::<_, 8>::new(source).unwrap();
        assert_eq!(entries.len(), 1);
        for _ in entries {}
    }

    #[test]
    #[should_panic(expected = "a list's entries changed between its count and its walk")]
    fn a_walk_shorter_than_its_count_panics() {
        let entries = BoundedIter::<_, 8> {
            iter: [1u8].iter(),
            remaining: 2,
        };
        for _ in entries {}
    }
}
