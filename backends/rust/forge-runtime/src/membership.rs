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
//! - [`Except`] is a chain none of whose entries is of the kind, where the gate
//!   does not hold.
//! - [`Placed`] is the other entries and the place of the first entry of the kind
//!   among them, where the gate holds. The place is part of the value, since the
//!   protocols these chains carry do not order their entries: a frame is written
//!   back with its entries where they came.
//!
//! Both hold the list `L` the same way, so the two arms of a value hold one list
//! type, and both count it against the chain's bounds. Each is made by a
//! constructor that checks it once, as [`crate::bounded::Bounded`] is, and has no
//! mutable access, so it stays what it was checked to be. The encode walks
//! either as [`Slots`].

use core::marker::PhantomData;

use crate::bounded::{BoundedIter, Entries};

/// A kind of entry: which entries of a chain a gate names. Generated, one for
/// each identifier a gate looks for, over every form of the chain's entry.
pub trait Key<E: ?Sized> {
    /// Whether `entry` is of the kind.
    fn matches(entry: &E) -> bool;
}

/// A list whose entries can each be asked whether it is of the kind `K`: what
/// the constructors of [`Except`] and [`Placed`] check a list by.
///
/// Each list a codec holds implements it with the bound its own entries need
/// (`K: Key<T>` for a list of `T`, `K: Key<W::Entry<'a>>` for a view's). One
/// bound over every lifetime a walk could borrow its list for would hold only
/// for entries that borrow nothing, which a decoded entry does.
pub trait OfKind<K> {
    /// Whether each entry, in order, is of the kind.
    fn kinds(&self) -> impl Iterator<Item = bool> + '_;

    /// The place of the first entry of the kind, or `None` when there is none.
    fn first_of_kind(&self) -> Option<usize> {
        self.kinds().position(|of_kind| of_kind)
    }
}

impl<T, K: Key<T>> OfKind<K> for &[T] {
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter().map(|e| K::matches(e))
    }
}

impl<T, const N: usize, K: Key<T>> OfKind<K> for heapless::Vec<T, N> {
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter().map(|e| K::matches(e))
    }
}

#[cfg(feature = "alloc")]
impl<T, K: Key<T>> OfKind<K> for alloc::vec::Vec<T> {
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter().map(|e| K::matches(e))
    }
}

impl<'a, W: crate::view::ListWalk<C>, const C: usize, K: Key<W::Entry<'a>>> OfKind<K>
    for crate::view::ListView<'a, W, C>
{
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter().map(|e| K::matches(&e))
    }
}

impl<I: Iterator + Clone, const MAX: usize, const MIN: usize, K: Key<I::Item>> OfKind<K>
    for BoundedIter<I, MAX, MIN>
{
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.clone().map(|e| K::matches(&e))
    }
}

impl<L: OfKind<K>, K> OfKind<K> for Without<L> {
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        let skip = self.skip;
        self.list
            .kinds()
            .enumerate()
            .filter(move |(at, _)| Some(*at) != skip)
            .map(|(_, of_kind)| of_kind)
    }
}

/// A chain of at most `MAX` entries and at least `MIN`, held as the list `L`,
/// none of whose entries is of the kind `K`.
pub struct Except<L, K, const MAX: usize, const MIN: usize = 0> {
    list: L,
    kind: PhantomData<fn() -> K>,
}

impl<L: Entries, K, const MAX: usize, const MIN: usize> Except<L, K, MAX, MIN> {
    /// `list` as a chain with no entry of the kind `K` inside the bounds, or
    /// `None` when it holds more or fewer entries or one of the kind.
    pub fn new(list: L) -> Option<Self>
    where
        L: OfKind<K>,
    {
        if !(MIN..=MAX).contains(&list.entry_count()) || list.first_of_kind().is_some() {
            return None;
        }
        Some(Self {
            list,
            kind: PhantomData,
        })
    }

    /// The chain as its walk: the same entries with their count, still none of
    /// the kind. The walk states the chain's most entries alone, as a walk of
    /// the other arm does, so both arms' walks are one type; the fewest stay
    /// this type's.
    pub fn iter(&self) -> Except<BoundedIter<L::Iter<'_>, MAX>, K, MAX, MIN> {
        Except {
            list: BoundedIter::counted(self.list.entries(), self.list.entry_count()),
            kind: PhantomData,
        }
    }

    /// The entries in order, with no named entry among them.
    pub fn slots(&self) -> Slots<L::Iter<'_>> {
        Slots {
            rest: self.list.entries(),
            place: None,
            at: 0,
            remaining: self.list.entry_count(),
        }
    }

    /// How many entries the chain holds.
    pub fn len(&self) -> usize {
        self.list.entry_count()
    }

    /// Whether the chain holds no entry.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<L, K, const MAX: usize, const MIN: usize> Except<L, K, MAX, MIN> {
    /// The list.
    pub fn inner(&self) -> &L {
        &self.list
    }

    /// The list, without its refinement.
    pub fn into_inner(self) -> L {
        self.list
    }

    /// A storage profile's list as a chain with no entry of the kind `K` inside
    /// the bounds, read through its slice, or `None` when it is not one.
    pub fn from_list<T>(list: L) -> Option<Self>
    where
        L: crate::storage::SceList<T>,
        K: Key<T>,
    {
        let entries = list.as_slice();
        if !(MIN..=MAX).contains(&entries.len()) || entries.iter().any(|e| K::matches(e)) {
            return None;
        }
        Some(Self {
            list,
            kind: PhantomData,
        })
    }

    /// A storage profile's chain as its walk, read through its slice.
    pub fn list_iter<T>(&self) -> Except<BoundedIter<core::slice::Iter<'_, T>, MAX>, K, MAX, MIN>
    where
        L: crate::storage::SceList<T>,
    {
        let entries = self.list.as_slice();
        Except {
            list: BoundedIter::counted(entries.iter(), entries.len()),
            kind: PhantomData,
        }
    }
}

impl<L: Clone, K, const MAX: usize, const MIN: usize> Clone for Except<L, K, MAX, MIN> {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            kind: PhantomData,
        }
    }
}

impl<L: Copy, K, const MAX: usize, const MIN: usize> Copy for Except<L, K, MAX, MIN> {}

impl<L: PartialEq, K, const MAX: usize, const MIN: usize> PartialEq for Except<L, K, MAX, MIN> {
    fn eq(&self, other: &Self) -> bool {
        self.list == other.list
    }
}

impl<L: core::fmt::Debug, K, const MAX: usize, const MIN: usize> core::fmt::Debug
    for Except<L, K, MAX, MIN>
{
    /// The list as it prints: the refinement is a fact about it.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.list.fmt(f)
    }
}

/// What the walk of a chain yields: an entry of the list, or the place of the
/// entry of the kind the chain names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot<T> {
    /// An entry of the list.
    Entry(T),
    /// The entry of the kind the chain names.
    Named,
}

/// A chain of at most `MAX` entries that holds an entry of the kind `K`: the
/// others, held as the list `L`, at most `MAX - 1`, and the place of the first
/// entry of the kind among them, no entry before the place being of the kind.
pub struct Placed<L, K, const MAX: usize> {
    rest: L,
    place: usize,
    kind: PhantomData<fn() -> K>,
}

impl<L: Entries, K, const MAX: usize> Placed<L, K, MAX> {
    /// The entries `rest` with the named entry at `place` among them, or `None`
    /// when the chain they make holds more than `MAX`, `place` is past the end
    /// of `rest`, or an entry before `place` is of the kind (the named entry is
    /// then that one, not the one at `place`).
    pub fn new(rest: L, place: usize) -> Option<Self>
    where
        L: OfKind<K>,
    {
        let count = rest.entry_count();
        if count >= MAX || place > count || rest.first_of_kind().is_some_and(|at| at < place) {
            return None;
        }
        Some(Self {
            rest,
            place,
            kind: PhantomData,
        })
    }

    /// The chain as its walk: the same others with their count and the same
    /// place.
    pub fn iter(&self) -> Placed<BoundedIter<L::Iter<'_>, MAX>, K, MAX> {
        Placed {
            rest: BoundedIter::counted(self.rest.entries(), self.rest.entry_count()),
            place: self.place,
            kind: PhantomData,
        }
    }

    /// The entries in order with the named entry at its place, and how many.
    pub fn slots(&self) -> Slots<L::Iter<'_>> {
        Slots {
            rest: self.rest.entries(),
            place: Some(self.place),
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

    /// The others and the place, without the refinement.
    pub fn into_parts(self) -> (L, usize) {
        (self.rest, self.place)
    }

    /// A storage profile's list of the others with the named entry at `place`,
    /// read through its slice, or `None` when they do not make such a chain
    /// ([`Self::new`]).
    pub fn from_list<T>(rest: L, place: usize) -> Option<Self>
    where
        L: crate::storage::SceList<T>,
        K: Key<T>,
    {
        let entries = rest.as_slice();
        if entries.len() >= MAX
            || place > entries.len()
            || entries[..place].iter().any(|e| K::matches(e))
        {
            return None;
        }
        Some(Self {
            rest,
            place,
            kind: PhantomData,
        })
    }

    /// A storage profile's chain as its walk, read through its slice.
    pub fn list_iter<T>(&self) -> Placed<BoundedIter<core::slice::Iter<'_, T>, MAX>, K, MAX>
    where
        L: crate::storage::SceList<T>,
    {
        let entries = self.rest.as_slice();
        Placed {
            rest: BoundedIter::counted(entries.iter(), entries.len()),
            place: self.place,
            kind: PhantomData,
        }
    }
}

/// A list a view holds whole as the others of a chain that holds an entry of
/// the kind `K`, read without its first entry of the kind, which stands at the
/// place; `Err` gives the list back when it holds none, or when the others are
/// past the bound.
pub fn place_first_in<L: Entries + OfKind<K>, K, const MAX: usize>(
    list: L,
) -> Result<Placed<Without<L>, K, MAX>, L> {
    let Some(place) = list.first_of_kind() else {
        return Err(list);
    };
    if list.entry_count() > MAX {
        return Err(list);
    }
    Ok(Placed {
        rest: Without {
            list,
            skip: Some(place),
        },
        place,
        kind: PhantomData,
    })
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
        const {
            assert!(
                MAX >= 1,
                "the chain cannot hold its named entry inside its bound"
            )
        };
        let empty = L::default();
        assert!(empty.entry_count() == 0, "a list's default holds entries");
        Self {
            rest: empty,
            place: 0,
            kind: PhantomData,
        }
    }
}

/// The walk of a chain: each entry, and the named one at its place where the
/// chain holds one.
#[derive(Debug, Clone)]
pub struct Slots<I> {
    rest: I,
    place: Option<usize>,
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
        let slot = if self.place == Some(self.at) {
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

/// A decoded chain that holds an entry of the kind `K` as the others and the
/// place of the first entry of the kind, the entry taken out where it stood;
/// `Err` gives the chain back when it holds no entry of the kind, or when the
/// others are past the bound.
pub fn place_first<T, K: Key<T>, const N: usize, const MAX: usize>(
    mut list: heapless::Vec<T, N>,
) -> Result<Placed<heapless::Vec<T, N>, K, MAX>, heapless::Vec<T, N>> {
    let Some(place) = list.iter().position(|e| K::matches(e)) else {
        return Err(list);
    };
    list.remove(place);
    if list.len() >= MAX {
        return Err(list);
    }
    Ok(Placed {
        rest: list,
        place,
        kind: PhantomData,
    })
}

/// A list `L` read without its entry at `skip`, where there is one: how a view,
/// which holds a chain as the bytes its entries occupy, holds the others of a
/// chain whose named entry stands among them. Both arms of a view hold this
/// one type, with nothing to skip where the chain names no entry.
pub struct Without<L> {
    list: L,
    skip: Option<usize>,
}

impl<L: Entries> Without<L> {
    /// `list` read whole.
    pub fn whole(list: L) -> Self {
        Self { list, skip: None }
    }

    /// `list` read without its entry at `skip`, or `None` when `skip` is past
    /// its end.
    pub fn skipping(list: L, skip: usize) -> Option<Self> {
        (skip < list.entry_count()).then_some(Self {
            list,
            skip: Some(skip),
        })
    }
}

impl<L: Entries> Entries for Without<L> {
    type Iter<'s>
        = SkipAt<L::Iter<'s>>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.list.entry_count() - usize::from(self.skip.is_some())
    }

    fn entries(&self) -> Self::Iter<'_> {
        SkipAt {
            inner: self.list.entries(),
            skip: self.skip,
            at: 0,
        }
    }
}

impl<L: Clone> Clone for Without<L> {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            skip: self.skip,
        }
    }
}

impl<L: Copy> Copy for Without<L> {}

impl<L: Entries + Default> Default for Without<L> {
    fn default() -> Self {
        Self {
            list: L::default(),
            skip: None,
        }
    }
}

impl<L: PartialEq> PartialEq for Without<L> {
    /// The same list read without the same entry.
    fn eq(&self, other: &Self) -> bool {
        self.skip == other.skip && self.list == other.list
    }
}

impl<L: core::fmt::Debug> core::fmt::Debug for Without<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Without")
            .field("list", &self.list)
            .field("skip", &self.skip)
            .finish()
    }
}

/// The walk of a [`Without`]: the inner walk with the entry at `skip` passed over.
#[derive(Debug, Clone)]
pub struct SkipAt<I> {
    inner: I,
    skip: Option<usize>,
    at: usize,
}

impl<I: Iterator> Iterator for SkipAt<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.skip == Some(self.at) {
            self.at += 1;
            self.inner.next()?;
        }
        self.at += 1;
        self.inner.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kind of an even number, over a number and over a reference to one,
    /// as a generated kind is over every form of its entry.
    struct Even;

    impl Key<u8> for Even {
        fn matches(entry: &u8) -> bool {
            *entry % 2 == 0
        }
    }

    impl Key<&u8> for Even {
        fn matches(entry: &&u8) -> bool {
            **entry % 2 == 0
        }
    }

    #[test]
    fn a_chain_with_no_entry_of_the_kind_is_held_inside_its_bounds() {
        assert!(Except::<&[u8], Even, 3>::new(&[1, 3, 5][..]).is_some());
        assert!(Except::<&[u8], Even, 3>::new(&[1, 4][..]).is_none());
        assert!(Except::<&[u8], Even, 2>::new(&[1, 3, 5][..]).is_none());
        let empty: &[u8] = &[];
        assert!(Except::<&[u8], Even, 3>::new(empty).is_some());
        assert!(Except::<&[u8], Even, 3, 1>::new(empty).is_none());
        let chain = Except::<&[u8], Even, 3>::new(&[1, 3][..]).expect("a chain");
        assert!(chain.slots().eq([Slot::Entry(&1), Slot::Entry(&3)]));
        assert_eq!(chain.iter().len(), 2);
    }

    #[test]
    fn the_named_entry_stands_at_its_place_and_nothing_before_it_is_of_its_kind() {
        let placed = Placed::<&[u8], Even, 4>::new(&[1, 3, 4][..], 2).expect("a place");
        assert_eq!(placed.len(), 4);
        assert!(placed.slots().eq([
            Slot::Entry(&1),
            Slot::Entry(&3),
            Slot::Named,
            Slot::Entry(&4)
        ]));
        assert_eq!(placed.slots().len(), 4);
        // The walk keeps the place and the others.
        assert!(placed.iter().slots().eq(placed.slots()));
        // An entry of the kind before the place is the first of the kind, so the
        // place would not be the first entry's.
        assert!(Placed::<&[u8], Even, 4>::new(&[2, 3][..], 1).is_none());
        // After the place, entries of the kind are entries like any other.
        assert!(Placed::<&[u8], Even, 4>::new(&[1, 2][..], 1).is_some());
        // The place lies within the entries, at their end at most.
        assert!(Placed::<&[u8], Even, 4>::new(&[1][..], 2).is_none());
        assert!(Placed::<&[u8], Even, 4>::new(&[1][..], 1).is_some());
        // The chain, the named entry with the others, is within the bound.
        assert!(Placed::<&[u8], Even, 2>::new(&[1, 3][..], 0).is_none());
        assert!(Placed::<&[u8], Even, 3>::new(&[1, 3][..], 0).is_some());
    }

    #[test]
    fn a_decoded_chain_is_placed_at_its_first_entry_of_the_kind() {
        let list = heapless::Vec::<u8, 4>::from_slice(&[1, 2, 3, 4]).expect("four");
        let placed = place_first::<u8, Even, 4, 4>(list).expect("an even entry");
        assert_eq!(placed.place(), 1);
        assert_eq!(placed.rest().as_slice(), &[1u8, 3, 4][..]);
        let odd = heapless::Vec::<u8, 4>::from_slice(&[1, 3]).expect("two");
        assert!(place_first::<u8, Even, 4, 4>(odd).is_err());
    }

    #[test]
    fn a_list_read_without_one_entry_counts_and_walks_the_others() {
        let whole: &[u8] = &[1, 2, 3];
        let without = Without::skipping(whole, 1).expect("an entry at 1");
        assert_eq!(without.entry_count(), 2);
        assert!(without.entries().eq([&1, &3]));
        assert_eq!(Without::whole(whole).entry_count(), 3);
        assert!(Without::skipping(whole, 3).is_none());
        let last = Without::skipping(whole, 2).expect("an entry at 2");
        assert!(last.entries().eq([&1, &2]));
    }

    #[test]
    fn the_default_placed_chain_is_the_named_entry_alone() {
        let alone = Placed::<&[u8], Even, 3>::default();
        assert!(alone.slots().eq([Slot::<&u8>::Named]));
    }
}
