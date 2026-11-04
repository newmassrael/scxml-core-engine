// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Lists refined by the kind of entry a chain's gate names (docs/adr/0065 §6 R6).
//!
//! A field gated on `chain.has(k)` is on the wire exactly when an entry of the
//! kind `k` is in the chain. The gates on one chain's entries, and the flags
//! joined with them, decide its fields by their values together, so a value holds
//! one arm for each assignment of them, and the arm says of each kind whether the
//! chain has an entry of it. A value made of these types says which arm it is by
//! the type it holds, so the fields the gates decide cannot disagree with the
//! chain:
//!
//! - [`Chain`] is the chain whole with an entry of each kind of one set and of
//!   none of another ([`Kinds`]): the arm's, for entries with content, which the
//!   value holds where they stand.
//! - [`Placed`] is the other entries and the place of the first entry of the kind
//!   among them, where the one gate on a bare entry holds: the entry's identifier
//!   is all it is, so the codec derives it and the value holds its place. The
//!   place is part of the value, since the protocols these chains carry do not
//!   order their entries: a frame is written back with its entries where they
//!   came.
//!
//! Both hold the list `L` the same way, so the arms of a value hold one list
//! type, and both count it against the chain's bounds. Each is made by a
//! constructor that checks it once, as [`crate::bounded::Bounded`] is, and has no
//! mutable access, so it stays what it was checked to be. The encode of a bare
//! entry's chain walks either as [`Slots`].
//!
//! A decode builds the chain in the arm that will hold it (`decode_chain_in`,
//! `decode_placed_in`), since a list moved into an arm copies its whole
//! capacity. Those two are the only places a chain is filled after it is made,
//! and each checks it before it returns, so no value outside them holds a chain
//! that is not what its type says. They are a generated decode's, and hidden
//! from the documentation (docs/adr/0065 §6 R6, amendment of 2026-11-05).

use core::marker::PhantomData;

use crate::bounded::{BoundedIter, Entries};
use crate::codec::CodecError;
use crate::storage::{HeldList, SceList};

/// A kind of entry: which entries of a chain a gate names. Generated, one for
/// each identifier a gate looks for, over every form of the chain's entry.
pub trait Key<E: ?Sized> {
    /// Whether `entry` is of the kind.
    fn matches(entry: &E) -> bool;
}

/// A list whose entries can each be asked whether it is of the kind `K`: what
/// the constructors of [`Chain`] and [`Placed`] check a list by.
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

impl<L: HeldList, K: Key<L::Entry>> OfKind<K> for L {
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.as_slice().iter().map(|e| K::matches(e))
    }
}

impl<'a, W: crate::view::ListWalk<C>, const C: usize, K: Key<W::Entry<'a>>> OfKind<K>
    for crate::view::ListView<'a, W, C>
{
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter().map(|e| K::matches(&e))
    }
}

impl<'a, W: crate::view::ListWalk<C>, const C: usize, K: Key<W::Entry<'a>>> OfKind<K>
    for crate::view::NonEmptyListView<'a, W, C>
where
    W::Entry<'a>: Clone,
{
    fn kinds(&self) -> impl Iterator<Item = bool> + '_ {
        self.entries().map(|e| K::matches(&e))
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

/// A set of kinds of entry, written as a tuple of [`Key`]s (`()`, `(K,)`,
/// `(K1, K2)`, up to eight): what a [`Chain`] holds an entry of each of, or of
/// none. A set is asked of a list through [`OfKind`], so it holds for every list
/// each of its kinds can be asked of.
pub trait Kinds<L> {
    /// Whether `list` holds an entry of each kind of the set.
    fn each_in(list: &L) -> bool;

    /// Whether `list` holds an entry of none of the set's kinds.
    fn none_in(list: &L) -> bool;
}

impl<L> Kinds<L> for () {
    fn each_in(_: &L) -> bool {
        true
    }

    fn none_in(_: &L) -> bool {
        true
    }
}

macro_rules! kinds_of_tuple {
    ($($kind:ident),+) => {
        impl<L, $($kind),+> Kinds<L> for ($($kind,)+)
        where
            $(L: OfKind<$kind>,)+
        {
            fn each_in(list: &L) -> bool {
                $(<L as OfKind<$kind>>::first_of_kind(list).is_some())&&+
            }

            fn none_in(list: &L) -> bool {
                $(<L as OfKind<$kind>>::first_of_kind(list).is_none())&&+
            }
        }
    };
}

kinds_of_tuple!(K1);
kinds_of_tuple!(K1, K2);
kinds_of_tuple!(K1, K2, K3);
kinds_of_tuple!(K1, K2, K3, K4);
kinds_of_tuple!(K1, K2, K3, K4, K5);
kinds_of_tuple!(K1, K2, K3, K4, K5, K6);
kinds_of_tuple!(K1, K2, K3, K4, K5, K6, K7);
kinds_of_tuple!(K1, K2, K3, K4, K5, K6, K7, K8);

/// A chain of at most `MAX` entries and at least `MIN`, held whole as the list
/// `L`, that holds an entry of each kind of the set `H` and an entry of none of
/// the set `A` (docs/adr/0065 §6 R6b): the chain one arm of the gates on its
/// entries holds, which says of each kind those gates ask for whether the chain
/// has one. The value holds each entry where it stands, so the first entry of a
/// kind is where the list has it, and nothing apart from the list says so.
///
/// A chain with no gate of its own holds one entry or more, and the arm where
/// every gate is clear holds such a chain with none of the kinds; the arms
/// where some gate holds hold an entry of the kind whatever the chain's fewest.
pub struct Chain<L, H, A, const MAX: usize, const MIN: usize = 0> {
    list: L,
    kinds: PhantomData<fn() -> (H, A)>,
}

impl<L: Entries, H, A, const MAX: usize, const MIN: usize> Chain<L, H, A, MAX, MIN> {
    /// `list` as a chain inside the bounds with an entry of each kind of `H`
    /// and of none of `A`, or `None` when it holds more or fewer entries, lacks
    /// a kind of `H`, or holds one of `A`.
    pub fn new(list: L) -> Option<Self>
    where
        H: Kinds<L>,
        A: Kinds<L>,
    {
        if !(MIN..=MAX).contains(&list.entry_count()) || !H::each_in(&list) || !A::none_in(&list) {
            return None;
        }
        Some(Self {
            list,
            kinds: PhantomData,
        })
    }

    /// The chain as its walk: the same entries with their count, the same
    /// kinds held and not. The walk states the chain's most entries alone, as
    /// the walk of every other arm does, so the arms' walks are one type; the
    /// fewest stay this type's.
    pub fn iter(&self) -> Chain<BoundedIter<L::Iter<'_>, MAX>, H, A, MAX, MIN> {
        Chain {
            list: BoundedIter::counted(self.list.entries(), self.list.entry_count()),
            kinds: PhantomData,
        }
    }

    /// The entries in order, with no named entry among them: the walk of a
    /// chain that has no entry of the kind of a bare entry a gate names
    /// (docs/adr/0065 §6 R6a), which the arm where the gate holds walks as
    /// [`Placed::slots`].
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

impl<L, H, A, const MAX: usize, const MIN: usize> Chain<L, H, A, MAX, MIN> {
    /// The list.
    pub fn inner(&self) -> &L {
        &self.list
    }

    /// The list, without its refinement.
    pub fn into_inner(self) -> L {
        self.list
    }

    /// A storage profile's list as a chain inside the bounds with an entry of
    /// each kind of `H` and of none of `A`, read through its slice, or `None`
    /// when it is not one.
    pub fn from_list<T>(list: L) -> Option<Self>
    where
        L: crate::storage::SceList<T>,
        H: for<'s> Kinds<&'s [T]>,
        A: for<'s> Kinds<&'s [T]>,
    {
        let entries = list.as_slice();
        if !(MIN..=MAX).contains(&entries.len()) || !H::each_in(&entries) || !A::none_in(&entries) {
            return None;
        }
        Some(Self {
            list,
            kinds: PhantomData,
        })
    }

    /// A storage profile's chain as its walk, read through its slice.
    pub fn list_iter<T>(&self) -> Chain<BoundedIter<core::slice::Iter<'_, T>, MAX>, H, A, MAX, MIN>
    where
        L: crate::storage::SceList<T>,
    {
        let entries = self.list.as_slice();
        Chain {
            list: BoundedIter::counted(entries.iter(), entries.len()),
            kinds: PhantomData,
        }
    }
}

impl<L: Clone, H, A, const MAX: usize, const MIN: usize> Clone for Chain<L, H, A, MAX, MIN> {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            kinds: PhantomData,
        }
    }
}

impl<L: Copy, H, A, const MAX: usize, const MIN: usize> Copy for Chain<L, H, A, MAX, MIN> {}

impl<L: PartialEq, H, A, const MAX: usize, const MIN: usize> PartialEq
    for Chain<L, H, A, MAX, MIN>
{
    fn eq(&self, other: &Self) -> bool {
        self.list == other.list
    }
}

impl<L: core::fmt::Debug, H, A, const MAX: usize, const MIN: usize> core::fmt::Debug
    for Chain<L, H, A, MAX, MIN>
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

/// Install in `group` the arm `install` makes of a chain, and read the chain's
/// entries into it where the arm holds it (docs/adr/0065 §6 R6b, docs/adr/0077
/// §7): `fill` is handed the arm's list empty, as [`crate::bounded::Bounded::refill`]
/// hands a list, and `held` finds the chain again in the arm `install` made.
///
/// A value whose arms are decided by the kinds of entry a chain holds cannot
/// know its arm before the chain is read, and a chain read into a local and
/// then moved into its arm copies the list's whole capacity at each move. So
/// the decode installs the arm it expects and reads the chain there. It answers
/// two things: whether the chain holds an entry of each kind of `H`, of none of
/// `A`, and as many entries as its bounds admit (when it holds other kinds,
/// they name another arm, which the decode reads the chain into again), and
/// whether the chain was past what the profile holds. `fill` says the second
/// as an entry decode does: it reads the chain whole, holding what fits
/// ([`crate::codec::push_keeping`]), and then answers `CapacityExceeded`
/// (docs/adr/0077 §7). Fewer entries than `MIN` are a frame cut short
/// (`NeedMoreBytes`), more than `MAX` a chain past its bound
/// (`TooManyElements`); a profile holds at least `MIN`, so a chain past its
/// capacity holds them.
///
/// Unless the chain is what the arm says, `group` is left at its default, so
/// it holds no chain that its type does not describe, whatever `fill` did. A
/// chain past its capacity that still holds an entry of every kind it read
/// keeps its arm, so the decode reads the arm's other elements on.
///
/// The chain `install` is handed has not been read yet, and is checked before
/// this returns. It is the one place a chain is made unchecked, so it is a
/// decode's alone (docs/adr/0065 §6 R6, amendment of 2026-11-05): hidden from
/// the documentation, and `install` and `held` capture nothing, which the build
/// asserts (each is zero-sized), so they hold no state the chain could be kept
/// in. They are called directly, never through a pointer.
#[doc(hidden)]
pub fn decode_chain_in<G, L, H, A, I, F, const MAX: usize, const MIN: usize>(
    group: &mut G,
    install: I,
    held: F,
    fill: impl FnOnce(&mut L) -> Result<(), CodecError>,
) -> Result<(bool, bool), CodecError>
where
    G: Default,
    L: HeldList,
    H: for<'s> Kinds<&'s [L::Entry]>,
    A: for<'s> Kinds<&'s [L::Entry]>,
    I: FnOnce(Chain<L, H, A, MAX, MIN>) -> G,
    F: FnOnce(&mut G) -> Option<&mut Chain<L, H, A, MAX, MIN>>,
{
    const {
        assert!(
            stateless::<I>() && stateless::<F>(),
            "a decode door's install and held capture nothing"
        )
    };
    *group = install(Chain {
        list: <L as SceList<L::Entry>>::empty(),
        kinds: PhantomData,
    });
    let read = held(group).map(|chain| -> Result<(bool, bool), CodecError> {
        let over = read_whole(fill(&mut chain.list))?;
        let entries = SceList::as_slice(&chain.list);
        match entries.len() {
            n if n < MIN => Err(CodecError::NeedMoreBytes),
            n if n > MAX => Err(CodecError::TooManyElements),
            _ => Ok((H::each_in(&entries) && A::none_in(&entries), over)),
        }
    });
    settle(group, read)
}

/// Whether a fill that ended `filled` read its chain past the capacity: `Ok` of
/// that, or the fill's own refusal of the frame.
fn read_whole(filled: Result<(), CodecError>) -> Result<bool, CodecError> {
    match filled {
        Ok(()) => Ok(false),
        Err(CodecError::CapacityExceeded) => Ok(true),
        Err(refused) => Err(refused),
    }
}

/// Whether a closure captures nothing: a zero-sized one holds no value, so no
/// chain a decode door hands it can be kept in it.
const fn stateless<T>() -> bool {
    core::mem::size_of::<T>() == 0
}

/// What a decode door returns, `group` left at its default unless the chain it
/// read is the arm's. `None` is an arm `install` made that `held` does not find
/// the chain in: a caller's defect, refused after the value is put back to its
/// default, so that it holds no unread chain.
fn settle<G: Default>(
    group: &mut G,
    read: Option<Result<(bool, bool), CodecError>>,
) -> Result<(bool, bool), CodecError> {
    let settled = read.unwrap_or_else(|| {
        *group = G::default();
        unreachable!("the arm `install` makes holds the chain `held` finds")
    });
    if !matches!(settled, Ok((true, _))) {
        *group = G::default();
    }
    settled
}

/// [`decode_chain_in`] for the arm where the one gate on a bare entry holds
/// (docs/adr/0065 §6 R6a): the arm is installed holding the named entry alone,
/// the chain's entries are read into its others, and the first entry of the
/// kind `K` is taken out where it stood and held as the place. It answers as
/// `decode_chain_in` does: the first is false when the chain holds no entry of
/// the kind, the second whether it was past the capacity, and it refuses with
/// `TooManyElements` a chain of more than `MAX` entries, the named one among
/// them.
///
/// The arm `install` makes holds the named entry alone, which a chain of the
/// kind is, so `group` holds what its type describes throughout; it is left at
/// its default unless the chain is the arm's. A decode's alone, as
/// `decode_chain_in` is.
#[doc(hidden)]
pub fn decode_placed_in<G, L, K, I, F, const MAX: usize>(
    group: &mut G,
    install: I,
    held: F,
    fill: impl FnOnce(&mut L) -> Result<(), CodecError>,
) -> Result<(bool, bool), CodecError>
where
    G: Default,
    L: HeldList,
    K: Key<L::Entry>,
    I: FnOnce(Placed<L, K, MAX>) -> G,
    F: FnOnce(&mut G) -> Option<&mut Placed<L, K, MAX>>,
{
    const {
        assert!(
            stateless::<I>() && stateless::<F>(),
            "a decode door's install and held capture nothing"
        )
    };
    *group = install(Placed {
        rest: <L as SceList<L::Entry>>::empty(),
        place: 0,
        kind: PhantomData,
    });
    let read = held(group).map(|placed| -> Result<(bool, bool), CodecError> {
        let over = read_whole(fill(&mut placed.rest))?;
        let entries = SceList::as_slice(&placed.rest);
        if entries.len() > MAX {
            return Err(CodecError::TooManyElements);
        }
        match entries.iter().position(|e| K::matches(e)) {
            Some(at) => {
                SceList::remove_at(&mut placed.rest, at);
                placed.place = at;
                Ok((true, over))
            }
            None => Ok((false, over)),
        }
    });
    settle(group, read)
}

/// A decoded chain that holds an entry of the kind `K` as the others and the
/// place of the first entry of the kind, the entry taken out where it stood;
/// `Err` gives the chain back when it holds no entry of the kind, or when the
/// others are past the bound.
pub fn place_first<L: HeldList, K: Key<L::Entry>, const MAX: usize>(
    mut list: L,
) -> Result<Placed<L, K, MAX>, L> {
    let Some(place) = list.as_slice().iter().position(|e| K::matches(e)) else {
        return Err(list);
    };
    list.remove_at(place);
    if list.as_slice().len() >= MAX {
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

    /// The kind of a number below 3, beside [`Even`] as a second kind one
    /// chain is asked for.
    struct Small;

    impl Key<u8> for Small {
        fn matches(entry: &u8) -> bool {
            *entry < 3
        }
    }

    impl Key<&u8> for Small {
        fn matches(entry: &&u8) -> bool {
            **entry < 3
        }
    }

    #[test]
    fn a_chain_with_no_entry_of_the_kind_is_held_inside_its_bounds() {
        assert!(Chain::<&[u8], (), (Even,), 3>::new(&[1, 3, 5][..]).is_some());
        assert!(Chain::<&[u8], (), (Even,), 3>::new(&[1, 4][..]).is_none());
        assert!(Chain::<&[u8], (), (Even,), 2>::new(&[1, 3, 5][..]).is_none());
        let empty: &[u8] = &[];
        assert!(Chain::<&[u8], (), (Even,), 3>::new(empty).is_some());
        assert!(Chain::<&[u8], (), (Even,), 3, 1>::new(empty).is_none());
        let chain = Chain::<&[u8], (), (Even,), 3>::new(&[1, 3][..]).expect("a chain");
        assert!(chain.slots().eq([Slot::Entry(&1), Slot::Entry(&3)]));
        assert_eq!(chain.iter().len(), 2);
    }

    #[test]
    fn a_chain_holds_an_entry_of_each_kind_it_has_and_of_none_it_has_not() {
        // 4 is even and not small; 1 is small and not even; 2 is both.
        assert!(Chain::<&[u8], (Even, Small), (), 3>::new(&[4, 1][..]).is_some());
        assert!(Chain::<&[u8], (Even, Small), (), 3>::new(&[2][..]).is_some());
        assert!(Chain::<&[u8], (Even, Small), (), 3>::new(&[4, 5][..]).is_none());
        assert!(Chain::<&[u8], (Even,), (Small,), 3>::new(&[4, 5][..]).is_some());
        assert!(Chain::<&[u8], (Even,), (Small,), 3>::new(&[4, 1][..]).is_none());
        assert!(Chain::<&[u8], (), (Even, Small), 3>::new(&[3, 5][..]).is_some());
        assert!(Chain::<&[u8], (), (Even, Small), 3>::new(&[3, 1][..]).is_none());
        // The walk keeps the entries and their kinds.
        let chain = Chain::<&[u8], (Even,), (Small,), 3>::new(&[5, 4][..]).expect("a chain");
        assert!(chain.iter().into_inner().eq([&5, &4]));
        let list = heapless::Vec::<u8, 4>::from_slice(&[3, 4]).expect("two");
        assert!(Chain::<_, (Even,), (Small,), 4>::from_list(list).is_some());
        let list = heapless::Vec::<u8, 4>::from_slice(&[3, 4]).expect("two");
        assert!(Chain::<_, (Small,), (), 4>::from_list(list).is_none());
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
        let placed = place_first::<_, Even, 4>(list).expect("an even entry");
        assert_eq!(placed.place(), 1);
        assert_eq!(placed.rest().as_slice(), &[1u8, 3, 4][..]);
        let odd = heapless::Vec::<u8, 4>::from_slice(&[1, 3]).expect("two");
        assert!(place_first::<_, Even, 4>(odd).is_err());
    }

    /// The list a growable profile holds is placed as the inline one is.
    #[cfg(feature = "alloc")]
    #[test]
    fn a_list_on_the_heap_is_placed_as_an_inline_one_is() {
        let placed = place_first::<_, Even, 4>(alloc::vec![1u8, 2, 3, 4]).expect("an even entry");
        assert_eq!(placed.place(), 1);
        assert_eq!(placed.rest().as_slice(), &[1u8, 3, 4][..]);
        assert!(place_first::<_, Even, 4>(alloc::vec![1u8, 3]).is_err());
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
    fn a_chain_held_whole_holds_an_entry_of_the_kind_inside_its_bound() {
        let having = Chain::<&[u8], (Even,), (), 3>::new(&[1, 4, 3][..]).expect("an even entry");
        assert_eq!(having.len(), 3);
        assert_eq!(having.iter().into_inner().len(), 3);
        assert!(Chain::<&[u8], (Even,), (), 3>::new(&[1, 3][..]).is_none());
        assert!(Chain::<&[u8], (Even,), (), 2>::new(&[1, 4, 3][..]).is_none());
        let list = heapless::Vec::<u8, 4>::from_slice(&[3, 2]).expect("two");
        assert!(Chain::<_, (Even,), (), 4>::from_list(list).is_some());
    }

    #[test]
    fn the_default_placed_chain_is_the_named_entry_alone() {
        let alone = Placed::<&[u8], Even, 3>::default();
        assert!(alone.slots().eq([Slot::<&u8>::Named]));
    }

    type Entries4 = heapless::Vec<u8, 4>;

    /// A value whose arms the chain's even entries decide, as a generated
    /// group of one membership is: the chain whole with an even entry, or the
    /// chain with none, absent where its own gate is clear.
    #[derive(Debug, PartialEq)]
    enum ByEven {
        Even(Chain<Entries4, (Even,), (), 4, 1>),
        NotEven(Option<Chain<Entries4, (), (Even,), 4, 1>>),
    }

    impl Default for ByEven {
        fn default() -> Self {
            ByEven::NotEven(None)
        }
    }

    fn even_arm(group: &mut ByEven) -> Option<&mut Chain<Entries4, (Even,), (), 4, 1>> {
        match group {
            ByEven::Even(chain) => Some(chain),
            ByEven::NotEven(_) => None,
        }
    }

    fn not_even_arm(group: &mut ByEven) -> Option<&mut Chain<Entries4, (), (Even,), 4, 1>> {
        match group {
            ByEven::NotEven(Some(chain)) => Some(chain),
            _ => None,
        }
    }

    /// A fill that pushes `entries`, as a generated chain decode pushes each
    /// entry it reads.
    fn pushing(entries: &[u8]) -> impl FnOnce(&mut Entries4) -> Result<(), CodecError> + '_ {
        move |list| {
            for entry in entries {
                SceList::try_push(list, *entry)?;
            }
            Ok(())
        }
    }

    #[test]
    fn a_chain_read_into_the_arm_its_kinds_name_is_held_there() {
        let mut group = ByEven::default();
        assert_eq!(
            decode_chain_in(&mut group, ByEven::Even, even_arm, pushing(&[1, 2])),
            Ok((true, false))
        );
        let list = Entries4::from_slice(&[1, 2]).expect("two");
        assert_eq!(
            group,
            ByEven::Even(Chain::from_list(list).expect("an even entry"))
        );

        let mut group = ByEven::default();
        assert_eq!(
            decode_chain_in(
                &mut group,
                |c| ByEven::NotEven(Some(c)),
                not_even_arm,
                pushing(&[1, 3])
            ),
            Ok((true, false))
        );
        let list = Entries4::from_slice(&[1, 3]).expect("two");
        assert_eq!(
            group,
            ByEven::NotEven(Some(Chain::from_list(list).expect("no even entry")))
        );
    }

    #[test]
    fn a_chain_of_other_kinds_than_its_arm_leaves_the_value_at_its_default() {
        // The arm expects an even entry and the chain holds none.
        let mut group = ByEven::NotEven(None);
        assert_eq!(
            decode_chain_in(&mut group, ByEven::Even, even_arm, pushing(&[1, 3])),
            Ok((false, false))
        );
        assert_eq!(group, ByEven::default());
        // The arm expects none and the chain holds one.
        assert_eq!(
            decode_chain_in(
                &mut group,
                |c| ByEven::NotEven(Some(c)),
                not_even_arm,
                pushing(&[2])
            ),
            Ok((false, false))
        );
        assert_eq!(group, ByEven::default());
    }

    #[test]
    fn a_chain_outside_its_bounds_or_cut_short_is_refused_and_not_held() {
        let mut group = ByEven::default();
        // Fewer entries than the chain's fewest is a frame cut short.
        assert_eq!(
            decode_chain_in(
                &mut group,
                |c| ByEven::NotEven(Some(c)),
                not_even_arm,
                pushing(&[])
            ),
            Err(CodecError::NeedMoreBytes)
        );
        assert_eq!(group, ByEven::default());
        // An error of the fill is the decode's, and the arm is not left half read.
        assert_eq!(
            decode_chain_in(&mut group, ByEven::Even, even_arm, |list| {
                SceList::try_push(list, 2u8)?;
                Err(CodecError::NeedMoreBytes)
            }),
            Err(CodecError::NeedMoreBytes)
        );
        assert_eq!(group, ByEven::default());
    }

    /// A list that holds more entries than the chain's bound, so the bound is
    /// what refuses.
    type Entries8 = heapless::Vec<u8, 8>;

    #[derive(Debug, PartialEq, Default)]
    enum Narrow {
        #[default]
        Empty,
        Held(Chain<Entries8, (), (), 2>),
    }

    fn narrow_arm(group: &mut Narrow) -> Option<&mut Chain<Entries8, (), (), 2>> {
        match group {
            Narrow::Held(chain) => Some(chain),
            Narrow::Empty => None,
        }
    }

    #[test]
    fn a_chain_past_its_bound_is_refused_where_the_list_holds_more() {
        let mut group = Narrow::default();
        let three = |list: &mut Entries8| -> Result<(), CodecError> {
            for entry in [1u8, 3, 5] {
                SceList::try_push(list, entry)?;
            }
            Ok(())
        };
        assert_eq!(
            decode_chain_in(&mut group, Narrow::Held, narrow_arm, three),
            Err(CodecError::TooManyElements)
        );
        assert_eq!(group, Narrow::Empty);
    }

    /// The arms of a bare entry's one membership: placed where it holds, the
    /// chain with none of the kind where it does not.
    #[derive(Debug, PartialEq)]
    enum ByMarker {
        Marked(Placed<Entries4, Even, 4>),
        Unmarked(Chain<Entries4, (), (Even,), 4, 1>),
    }

    impl Default for ByMarker {
        fn default() -> Self {
            ByMarker::Marked(Placed::default())
        }
    }

    fn marked_arm(group: &mut ByMarker) -> Option<&mut Placed<Entries4, Even, 4>> {
        match group {
            ByMarker::Marked(placed) => Some(placed),
            ByMarker::Unmarked(_) => None,
        }
    }

    #[test]
    fn a_bare_entry_read_in_place_is_taken_out_and_held_as_its_place() {
        let mut group = ByMarker::Unmarked(
            Chain::from_list(Entries4::from_slice(&[1]).expect("one")).expect("odd"),
        );
        assert_eq!(
            decode_placed_in(
                &mut group,
                ByMarker::Marked,
                marked_arm,
                pushing(&[1, 4, 3, 6])
            ),
            Ok((true, false))
        );
        let ByMarker::Marked(placed) = &group else {
            panic!("the marked arm is installed: {group:?}");
        };
        assert_eq!(placed.place(), 1);
        assert_eq!(placed.rest().as_slice(), &[1u8, 3, 6][..]);
        // A chain with no entry of the kind is not this arm's.
        assert_eq!(
            decode_placed_in(&mut group, ByMarker::Marked, marked_arm, pushing(&[1, 3])),
            Ok((false, false))
        );
        assert_eq!(group, ByMarker::default());
    }

    /// A list of two entries, below the chain's bound of four: a profile that
    /// holds less than the protocol does.
    type Entries2 = heapless::Vec<u8, 2>;

    #[derive(Debug, PartialEq)]
    enum ByEvenScant {
        Even(Chain<Entries2, (Even,), (), 4, 1>),
        NotEven(Chain<Entries2, (), (Even,), 4, 1>),
        Both(Chain<Entries2, (Even, Small), (), 4, 1>),
    }

    impl Default for ByEvenScant {
        fn default() -> Self {
            ByEvenScant::NotEven(
                Chain::from_list(Entries2::from_slice(&[1]).expect("one")).expect("odd"),
            )
        }
    }

    fn scant_even_arm(group: &mut ByEvenScant) -> Option<&mut Chain<Entries2, (Even,), (), 4, 1>> {
        match group {
            ByEvenScant::Even(chain) => Some(chain),
            _ => None,
        }
    }

    fn scant_both_arm(
        group: &mut ByEvenScant,
    ) -> Option<&mut Chain<Entries2, (Even, Small), (), 4, 1>> {
        match group {
            ByEvenScant::Both(chain) => Some(chain),
            _ => None,
        }
    }

    fn even(entry: &u8) -> bool {
        *entry % 2 == 0
    }

    fn small(entry: &u8) -> bool {
        *entry < 3
    }

    /// A fill that reads `entries` as a generated chain decode reads each one:
    /// held where it fits, the kinds the gates ask for kept past the capacity,
    /// and the chain past its capacity said once it is read whole.
    fn keeping<'e, L: HeldList<Entry = u8> + 'e>(
        entries: &'e [u8],
        kinds: &'e [fn(&u8) -> bool],
    ) -> impl FnOnce(&mut L) -> Result<(), CodecError> + 'e {
        move |list| {
            let mut full = false;
            for entry in entries {
                crate::codec::push_keeping(list, *entry, kinds, &mut full);
            }
            if full {
                return Err(CodecError::CapacityExceeded);
            }
            Ok(())
        }
    }

    #[test]
    fn a_chain_past_its_capacity_keeps_an_entry_of_each_kind_it_read_and_its_arm() {
        // Four entries, two held: 4 comes past the capacity, of a kind the list
        // holds none of, and takes the place of 3, whose kinds 1 holds too.
        let mut group = ByEvenScant::default();
        assert_eq!(
            decode_chain_in(
                &mut group,
                ByEvenScant::Even,
                scant_even_arm,
                keeping(&[1, 3, 5, 4], &[even])
            ),
            Ok((true, true))
        );
        let ByEvenScant::Even(chain) = &group else {
            panic!("the even arm is held: {group:?}");
        };
        assert_eq!(chain.inner().as_slice(), &[1u8, 4][..]);
        // An entry past the capacity of no new kind is read and not held.
        let mut group = ByEvenScant::default();
        assert_eq!(
            decode_chain_in(
                &mut group,
                ByEvenScant::Even,
                scant_even_arm,
                keeping(&[2, 4, 6], &[even])
            ),
            Ok((true, true))
        );
        let ByEvenScant::Even(chain) = &group else {
            panic!("the even arm is held: {group:?}");
        };
        assert_eq!(chain.inner().as_slice(), &[2u8, 4][..]);
    }

    #[test]
    fn a_chain_past_its_capacity_keeps_each_kind_while_it_has_room_for_one_of_each() {
        // 4 and 5 are held; 1 is of the small kind, which neither holds, and
        // takes the place of 5, which is of neither kind.
        let mut group = ByEvenScant::default();
        assert_eq!(
            decode_chain_in(
                &mut group,
                ByEvenScant::Both,
                scant_both_arm,
                keeping(&[4, 5, 1], &[even, small])
            ),
            Ok((true, true))
        );
        let ByEvenScant::Both(chain) = &group else {
            panic!("the arm of both kinds is held: {group:?}");
        };
        assert_eq!(chain.inner().as_slice(), &[4u8, 1][..]);
        // A third kind finds no entry to replace: 4 alone is even and 1 alone
        // small, so 7 is read and not held, and the two kinds stay.
        let mut group = ByEvenScant::default();
        assert_eq!(
            decode_chain_in(
                &mut group,
                ByEvenScant::Both,
                scant_both_arm,
                keeping(&[4, 1, 7], &[even, small, |e: &u8| *e == 7])
            ),
            Ok((true, true))
        );
        let ByEvenScant::Both(chain) = &group else {
            panic!("the arm of both kinds is held: {group:?}");
        };
        assert_eq!(chain.inner().as_slice(), &[4u8, 1][..]);
    }

    /// A list of one entry, below the number of kinds an arm asks for.
    type Entries1 = heapless::Vec<u8, 1>;

    #[derive(Debug, PartialEq)]
    enum ByBothInOne {
        Both(Chain<Entries1, (Even, Small), (), 4, 1>),
        Neither(Chain<Entries1, (), (Even, Small), 4, 1>),
    }

    impl Default for ByBothInOne {
        fn default() -> Self {
            ByBothInOne::Neither(
                Chain::from_list(Entries1::from_slice(&[3]).expect("one")).expect("neither"),
            )
        }
    }

    fn both_in_one_arm(
        group: &mut ByBothInOne,
    ) -> Option<&mut Chain<Entries1, (Even, Small), (), 4, 1>> {
        match group {
            ByBothInOne::Both(chain) => Some(chain),
            ByBothInOne::Neither(_) => None,
        }
    }

    #[test]
    fn a_chain_whose_kinds_outnumber_its_capacity_leaves_the_value_at_its_default() {
        // One entry held: 4, the one even entry; 1, the one small entry, has no
        // entry to replace, so the arm that asks for both kinds is not held.
        let mut group = ByBothInOne::default();
        assert_eq!(
            decode_chain_in(
                &mut group,
                ByBothInOne::Both,
                both_in_one_arm,
                keeping(&[4, 1], &[even, small])
            ),
            Ok((false, true))
        );
        assert_eq!(group, ByBothInOne::default());
    }
}
