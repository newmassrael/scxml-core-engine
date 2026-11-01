// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Lists refined by the kind of entry a chain's gate names (docs/adr/0065 §6 R6).
//!
//! A field gated on `chain.has(k)` is on the wire exactly when an entry of the
//! kind `k` is in the chain. The chains that gate holds for factor in one way, at
//! the first entry of the kind: the entries before it, none of the kind, then
//! that entry, then any entries. The chains it does not hold for have no entry of
//! the kind. A value made of these types says which of the two it is by the type
//! it holds, so the fields the gate decides cannot disagree with the chain:
//!
//! - [`Except`] is a list none of whose entries is of the kind, the chain where
//!   the gate does not hold.
//! - [`Placed`] is the other entries and the place of the first entry of the kind
//!   among them, the chain where the gate holds. The place is part of the value,
//!   since the protocols these chains carry do not order their entries: a frame
//!   is written back with its entries where they came.
//!
//! Each is made by a constructor that checks it once, as [`crate::bounded::Bounded`]
//! is, and has no mutable access, so it stays what it was checked to be.

use core::marker::PhantomData;

use crate::bounded::Entries;

/// A kind of entry: which entries of a chain a gate names. Generated, one for
/// each identifier a gate looks for, over every form of the chain's entry.
pub trait Key<E: ?Sized> {
    /// Whether `entry` is of the kind.
    fn matches(entry: &E) -> bool;
}

/// A list `L` none of whose entries is of the kind `K`.
pub struct Except<L, K>(L, PhantomData<fn() -> K>);

impl<L: Entries, K> Except<L, K> {
    /// `list` as a list with no entry of the kind `K`, or `None` when it has one.
    pub fn new(list: L) -> Option<Self>
    where
        K: for<'s> Key<<L::Iter<'s> as Iterator>::Item>,
    {
        if list.entries().any(|e| K::matches(&e)) {
            None
        } else {
            Some(Self(list, PhantomData))
        }
    }
}

impl<L, K> Except<L, K> {
    /// The list.
    pub fn inner(&self) -> &L {
        &self.0
    }
}

impl<L: Entries, K> Entries for Except<L, K> {
    type Iter<'s>
        = L::Iter<'s>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.0.entry_count()
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.0.entries()
    }
}

impl<L: Clone, K> Clone for Except<L, K> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<L: Copy, K> Copy for Except<L, K> {}

impl<L: PartialEq, K> PartialEq for Except<L, K> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<L: core::fmt::Debug, K> core::fmt::Debug for Except<L, K> {
    /// The list as it prints: the refinement is a fact about it.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<L: Default + Entries, K> Default for Except<L, K> {
    /// The empty list, which has no entry of any kind.
    fn default() -> Self {
        let empty = L::default();
        assert!(empty.entry_count() == 0, "a list's default holds entries");
        Self(empty, PhantomData)
    }
}

/// What the walk of a [`Placed`] list yields: an entry of the list, or the place
/// of the entry of the kind it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot<T> {
    /// An entry of the list.
    Entry(T),
    /// The entry of the kind the list names.
    Named,
}

/// The entries of a chain that holds an entry of the kind `K`: the others, at most
/// `MAX`, and the place of the first entry of the kind among them, no entry
/// before the place being of the kind.
pub struct Placed<L, K, const MAX: usize> {
    rest: L,
    place: usize,
    kind: PhantomData<fn() -> K>,
}

impl<L: Entries, K, const MAX: usize> Placed<L, K, MAX> {
    /// The entries `rest` with the named entry at `place` among them, or `None`
    /// when `rest` holds more than `MAX`, `place` is past its end, or an entry
    /// before `place` is of the kind (the named entry is then that one, not the
    /// one at `place`).
    pub fn new(rest: L, place: usize) -> Option<Self>
    where
        K: for<'s> Key<<L::Iter<'s> as Iterator>::Item>,
    {
        let count = rest.entry_count();
        if count > MAX || place > count || rest.entries().take(place).any(|e| K::matches(&e)) {
            return None;
        }
        Some(Self {
            rest,
            place,
            kind: PhantomData,
        })
    }

    /// The entries in order with the named entry at its place, and how many.
    pub fn slots(&self) -> Slots<L::Iter<'_>> {
        Slots {
            rest: self.rest.entries(),
            place: self.place,
            at: 0,
            remaining: self.rest.entry_count() + 1,
        }
    }

    /// How many entries the chain holds, the named one included.
    pub fn len(&self) -> usize {
        self.rest.entry_count() + 1
    }

    /// Whether the chain holds no entry, which it never does.
    pub fn is_empty(&self) -> bool {
        false
    }
}

impl<L, K, const MAX: usize> Placed<L, K, MAX> {
    /// The entries other than the named one.
    pub fn rest(&self) -> &L {
        &self.rest
    }

    /// Where the named entry stands among the others: how many come before it.
    pub fn place(&self) -> usize {
        self.place
    }
}

impl<L: Clone, K, const MAX: usize> Clone for Placed<L, K, MAX> {
    fn clone(&self) -> Self {
        Self {
            rest: self.rest.clone(),
            place: self.place,
            kind: PhantomData,
        }
    }
}

impl<L: Copy, K, const MAX: usize> Copy for Placed<L, K, MAX> {}

impl<L: PartialEq, K, const MAX: usize> PartialEq for Placed<L, K, MAX> {
    fn eq(&self, other: &Self) -> bool {
        self.place == other.place && self.rest == other.rest
    }
}

impl<L: core::fmt::Debug, K, const MAX: usize> core::fmt::Debug for Placed<L, K, MAX> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Placed")
            .field("rest", &self.rest)
            .field("place", &self.place)
            .finish()
    }
}

impl<L: Default + Entries, K, const MAX: usize> Default for Placed<L, K, MAX> {
    /// The named entry alone.
    fn default() -> Self {
        let empty = L::default();
        assert!(empty.entry_count() == 0, "a list's default holds entries");
        Self {
            rest: empty,
            place: 0,
            kind: PhantomData,
        }
    }
}

/// The walk of a [`Placed`] list: each entry, and the named one at its place.
#[derive(Debug, Clone)]
pub struct Slots<I> {
    rest: I,
    place: usize,
    at: usize,
    remaining: usize,
}

impl<I: Iterator> Iterator for Slots<I> {
    type Item = Slot<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        // A list that walks longer or shorter than it counted has broken what an
        // accessor is (docs/adr/0061 §3), and the walk panics as a bounded walk
        // does rather than write a chain whose entries disagree with its count.
        if self.remaining == 0 {
            if self.rest.next().is_some() {
                crate::bounded::entries_changed();
            }
            return None;
        }
        self.remaining -= 1;
        let slot = if self.at == self.place {
            Slot::Named
        } else {
            match self.rest.next() {
                Some(entry) => Slot::Entry(entry),
                None => crate::bounded::entries_changed(),
            }
        };
        self.at += 1;
        Some(slot)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<I: Iterator> ExactSizeIterator for Slots<I> {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kind of an even number.
    struct Even;

    impl Key<&u8> for Even {
        fn matches(entry: &&u8) -> bool {
            **entry % 2 == 0
        }
    }

    #[test]
    fn a_list_with_no_entry_of_the_kind_is_held_and_one_with_it_is_not() {
        assert!(Except::<&[u8], Even>::new(&[1, 3, 5][..]).is_some());
        assert!(Except::<&[u8], Even>::new(&[1, 4][..]).is_none());
        let empty: &[u8] = &[];
        assert!(Except::<&[u8], Even>::new(empty).is_some());
    }

    #[test]
    fn the_named_entry_stands_at_its_place_and_nothing_before_it_is_of_its_kind() {
        let placed = Placed::<&[u8], Even, 3>::new(&[1, 3, 4][..], 2).expect("a place");
        assert_eq!(placed.len(), 4);
        assert!(placed.slots().eq([
            Slot::Entry(&1),
            Slot::Entry(&3),
            Slot::Named,
            Slot::Entry(&4)
        ]));
        assert_eq!(placed.slots().len(), 4);
        // An entry of the kind before the place is the first of the kind, so the
        // place would not be the first entry's.
        assert!(Placed::<&[u8], Even, 3>::new(&[2, 3][..], 1).is_none());
        // After the place, entries of the kind are entries like any other.
        assert!(Placed::<&[u8], Even, 3>::new(&[1, 2][..], 1).is_some());
        // The place lies within the entries, at their end at most.
        assert!(Placed::<&[u8], Even, 3>::new(&[1][..], 2).is_none());
        assert!(Placed::<&[u8], Even, 3>::new(&[1][..], 1).is_some());
        // The others are within the bound.
        assert!(Placed::<&[u8], Even, 1>::new(&[1, 3][..], 0).is_none());
    }

    #[test]
    fn the_default_placed_list_is_the_named_entry_alone() {
        let alone = Placed::<&[u8], Even, 3>::default();
        assert!(alone.slots().eq([Slot::<&u8>::Named]));
    }
}
