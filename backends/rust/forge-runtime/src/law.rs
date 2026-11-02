// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Lists whose entries say their inputs by a law (docs/adr/0065 §6 R8).
//!
//! An entry codec can say a flag-input itself (R4a) while the list that holds
//! it hands every entry a value from elsewhere: a flag of the parent bound to
//! every entry, or, in a chain, whether the entry before has an identifier. The
//! two agree or the value has no frame. Both are a language over the list that a
//! small automaton decides: a state, the value each entry must say in it, and
//! the state after the entry. A bound flag is the automaton whose state is the
//! flag and never changes ([`Uniform`]); the entry before is the one whose state
//! starts clear and becomes whether the entry has the identifier ([`AfterKey`]).
//! A list with both runs them together, as a tuple.
//!
//! [`Lawful`] holds a list with the state its automaton starts in, which is the
//! value's to give where the start is not fixed: the flag the parent writes is
//! read from it, so an empty list still holds one. It is made by
//! [`Lawful::new`], which runs the automaton once, as
//! [`crate::bounded::Bounded::new`] counts once, and it has no mutable access.
//!
//! What a codec's source interface returns is the walk of such a list. It runs
//! the automaton again as it is walked, and panics where an entry breaks the law,
//! as [`BoundedIter`] does where the entries are more or fewer than it counted: a
//! source whose entries say otherwise on a second read has broken what an
//! accessor is (docs/adr/0061 §3).

use core::marker::PhantomData;
use core::ops::Deref;

use crate::bounded::{Bounded, BoundedIter, Entries, Fewest};
use crate::codec::CodecError;
use crate::membership::Key;
use crate::storage::{HeldList, SceList};
use crate::view::{ListView, ListWalk, NonEmptyListView};

/// A flag-input an entry says. Generated for each input a list's entries say,
/// over every form of the entry.
pub trait Says<E: ?Sized> {
    /// The value `entry` says the input has, or `None` where it says nothing of
    /// it: an entry in an arm that does not read the input has no bit it could
    /// change (docs/adr/0065 §6 R4e), so it disagrees with no state (R8b).
    fn says(entry: &E) -> Option<bool>;
}

/// An automaton over a list's entries: the start a value gives it.
pub trait Law {
    /// What a value gives the automaton to start in: the flag, for the law of a
    /// bound flag; `()`, where the start is fixed.
    type Start: Copy + PartialEq + core::fmt::Debug;
}

/// The automaton run over entries of the form `E`.
pub trait LawOver<E: ?Sized>: Law {
    /// What the automaton knows between entries.
    type State: Copy;

    /// The state before the first entry.
    fn start(start: Self::Start) -> Self::State;

    /// The state after `entry`, or `None` when `entry` says a value other than
    /// the one `state` asks of it.
    fn step(state: Self::State, entry: &E) -> Option<Self::State>;

    /// The start a list that begins with `first` follows from, where the start
    /// is the value's: what a list built without one, a default, is given.
    fn start_of(first: Option<&E>) -> Self::Start;
}

/// The law of a flag bound to every entry: each entry says the input `S` as the
/// start says it, and the start is the flag.
pub struct Uniform<S>(PhantomData<fn() -> S>);

impl<S> Law for Uniform<S> {
    type Start = bool;
}

impl<S: Says<E>, E: ?Sized> LawOver<E> for Uniform<S> {
    type State = bool;

    fn start(start: bool) -> bool {
        start
    }

    fn step(state: bool, entry: &E) -> Option<bool> {
        S::says(entry)
            .is_none_or(|said| said == state)
            .then_some(state)
    }

    fn start_of(first: Option<&E>) -> bool {
        first.and_then(|entry| S::says(entry)).unwrap_or(false)
    }
}

/// The law of an input a chain supplies from the entry before
/// (`<sce:entry-flag-bind previous-entry-id>`): each entry says the input `S`
/// set exactly when the entry before is of the kind `K`, and the first says it
/// clear. The start is fixed, so a value gives `()`.
pub struct AfterKey<S, K>(PhantomData<fn() -> (S, K)>);

impl<S, K> Law for AfterKey<S, K> {
    type Start = ();
}

impl<S: Says<E>, K: Key<E>, E: ?Sized> LawOver<E> for AfterKey<S, K> {
    type State = bool;

    fn start((): ()) -> bool {
        false
    }

    fn step(state: bool, entry: &E) -> Option<bool> {
        S::says(entry)
            .is_none_or(|said| said == state)
            .then(|| K::matches(entry))
    }

    fn start_of(_: Option<&E>) {}
}

/// Several laws over one list, run together: each entry follows each of them,
/// and the start is each one's.
macro_rules! law_of_tuple {
    ($($law:ident $at:tt),+) => {
        impl<$($law: Law),+> Law for ($($law,)+) {
            type Start = ($(<$law as Law>::Start,)+);
        }

        impl<E: ?Sized, $($law: LawOver<E>),+> LawOver<E> for ($($law,)+) {
            type State = ($(<$law as LawOver<E>>::State,)+);

            fn start(start: Self::Start) -> Self::State {
                ($($law::start(start.$at),)+)
            }

            fn step(state: Self::State, entry: &E) -> Option<Self::State> {
                Some(($($law::step(state.$at, entry)?,)+))
            }

            fn start_of(first: Option<&E>) -> Self::Start {
                ($($law::start_of(first),)+)
            }
        }
    };
}

law_of_tuple!(A0 0);
law_of_tuple!(A0 0, A1 1);
law_of_tuple!(A0 0, A1 1, A2 2);
law_of_tuple!(A0 0, A1 1, A2 2, A3 3);

/// Whether `entries`, in order, follow the law `A` from `start`.
fn run<'e, A: LawOver<E>, E: ?Sized + 'e>(
    start: A::Start,
    entries: impl Iterator<Item = &'e E>,
) -> bool {
    let mut state = A::start(start);
    for entry in entries {
        match A::step(state, entry) {
            Some(next) => state = next,
            None => return false,
        }
    }
    true
}

/// [`run`] over entries a list yields by value, as a view's walk does.
fn run_owned<A: LawOver<E>, E>(start: A::Start, entries: impl Iterator<Item = E>) -> bool {
    let mut state = A::start(start);
    for entry in entries {
        match A::step(state, &entry) {
            Some(next) => state = next,
            None => return false,
        }
    }
    true
}

/// A list whose entries can be run through the law `A`: what [`Lawful::new`]
/// checks a list by.
///
/// Each list a codec holds implements it with the bound its own entries need, as
/// [`crate::membership::OfKind`] is implemented.
pub trait Follows<A: Law> {
    /// Whether the entries, in order, follow `A` from `start`.
    fn follows(&self, start: A::Start) -> bool;

    /// The start the list follows from where the start is the value's, read
    /// off its first entry ([`LawOver::start_of`]).
    fn start_of_first(&self) -> A::Start;
}

impl<T, A: LawOver<T>> Follows<A> for &[T] {
    fn follows(&self, start: A::Start) -> bool {
        run::<A, T>(start, self.iter())
    }

    fn start_of_first(&self) -> A::Start {
        A::start_of(self.first())
    }
}

impl<L: HeldList, A: LawOver<L::Entry>> Follows<A> for L {
    fn follows(&self, start: A::Start) -> bool {
        run::<A, L::Entry>(start, self.as_slice().iter())
    }

    fn start_of_first(&self) -> A::Start {
        A::start_of(self.as_slice().first())
    }
}

impl<'a, W: ListWalk<C>, const C: usize, A: LawOver<W::Entry<'a>>> Follows<A>
    for ListView<'a, W, C>
{
    fn follows(&self, start: A::Start) -> bool {
        run_owned::<A, W::Entry<'a>>(start, self.iter())
    }

    fn start_of_first(&self) -> A::Start {
        A::start_of(self.iter().next().as_ref())
    }
}

impl<'a, W: ListWalk<C>, const C: usize, A: LawOver<W::Entry<'a>>> Follows<A>
    for NonEmptyListView<'a, W, C>
where
    W::Entry<'a>: Clone,
{
    fn follows(&self, start: A::Start) -> bool {
        run_owned::<A, W::Entry<'a>>(start, self.entries())
    }

    fn start_of_first(&self) -> A::Start {
        A::start_of(self.entries().next().as_ref())
    }
}

impl<I: Iterator + Clone, const MAX: usize, const MIN: usize, A: LawOver<I::Item>> Follows<A>
    for BoundedIter<I, MAX, MIN>
{
    fn follows(&self, start: A::Start) -> bool {
        run_owned::<A, I::Item>(start, self.clone())
    }

    fn start_of_first(&self) -> A::Start {
        A::start_of(self.clone().next().as_ref())
    }
}

impl<L: Follows<A>, A: Law, const MAX: usize, const MIN: usize> Follows<A>
    for Bounded<L, MAX, MIN>
{
    fn follows(&self, start: A::Start) -> bool {
        self.inner().follows(start)
    }

    fn start_of_first(&self) -> A::Start {
        self.inner().start_of_first()
    }
}

/// A list `L` whose entries follow the law `A` from the start the value gives
/// (docs/adr/0065 §6 R8).
///
/// The only way to hold one is [`Self::new`], which runs the law over the
/// entries, and the walks and projections of one, which keep its entries and its
/// start. It reads as the list it holds ([`Deref`]); it has no mutable access, so
/// its entries cannot leave the law once it is made.
pub struct Lawful<L, A: Law> {
    start: A::Start,
    list: L,
    law: PhantomData<fn() -> A>,
}

impl<L, A: Law> Lawful<L, A> {
    /// `list` as a list whose entries follow `A` from `start`, or `None` when an
    /// entry says a value other than the one its place asks of it.
    pub fn new(start: A::Start, list: L) -> Option<Self>
    where
        L: Follows<A>,
    {
        list.follows(start).then_some(Self {
            start,
            list,
            law: PhantomData,
        })
    }

    /// The start the entries follow from: for the law of a bound flag, the flag.
    pub fn start(&self) -> A::Start {
        self.start
    }

    /// The list. Named so that it leaves the list's own `inner` reachable
    /// through [`Deref`].
    pub fn list(&self) -> &L {
        &self.list
    }

    /// The start and the list, without the law.
    pub fn into_parts(self) -> (A::Start, L) {
        (self.start, self.list)
    }
}

impl<L, A: Law> Deref for Lawful<L, A> {
    type Target = L;

    fn deref(&self) -> &L {
        &self.list
    }
}

impl<L: Clone, A: Law> Clone for Lawful<L, A> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            list: self.list.clone(),
            law: PhantomData,
        }
    }
}

impl<L: Copy, A: Law> Copy for Lawful<L, A> {}

impl<L: PartialEq, A: Law> PartialEq for Lawful<L, A> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.list == other.list
    }
}

impl<L: core::fmt::Debug, A: Law> core::fmt::Debug for Lawful<L, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Lawful")
            .field("start", &self.start)
            .field("list", &self.list)
            .finish()
    }
}

impl<L: Default + Follows<A>, A: Law> Default for Lawful<L, A> {
    /// The list's own default, from the start its first entry says. The default
    /// entries of a list whose start is fixed follow from it, which the
    /// generator holds the entry codec's default to.
    fn default() -> Self {
        let list = L::default();
        let start = list.start_of_first();
        match Self::new(start, list) {
            Some(lawful) => lawful,
            None => {
                panic!("the default entries of a list do not follow its law (docs/adr/0065 §6 R8)")
            }
        }
    }
}

impl<L: Entries, A: Law, const MAX: usize, const MIN: usize> Lawful<Bounded<L, MAX, MIN>, A> {
    /// The entries in order, with their count and the start they follow from:
    /// what a codec's source returns, and walks through the law. The entries
    /// alone are the list's own `iter`, reached through [`Deref`].
    pub fn walk(&self) -> Lawful<BoundedIter<L::Iter<'_>, MAX, MIN>, A> {
        Lawful {
            start: self.start,
            list: self.list.iter(),
            law: PhantomData,
        }
    }
}

impl<L, A: Law, const MAX: usize, const MIN: usize> Lawful<Bounded<L, MAX, MIN>, A> {
    /// The entries of a storage profile's owned list, read through its slice,
    /// with their count and the start they follow from: [`Self::walk`] for a
    /// profile's list. Its entries alone are the list's own `list_iter`.
    pub fn list_walk<T>(&self) -> Lawful<BoundedIter<core::slice::Iter<'_, T>, MAX, MIN>, A>
    where
        L: SceList<T> + Entries,
    {
        Lawful {
            start: self.start,
            list: self.list.list_iter(),
            law: PhantomData,
        }
    }

    /// Fill the list in place from `start`, as [`Bounded::refill`] fills it, and
    /// hold what `fill` left to the law: a decode reads each entry under the
    /// state the law is in, so the entries it reads follow it.
    ///
    /// On `Err` the list is its default, so it still follows its law, whatever
    /// `fill` left.
    pub fn refill_from<T: Default>(
        &mut self,
        start: A::Start,
        fill: impl FnOnce(&mut L) -> Result<(), CodecError>,
    ) -> Result<(), CodecError>
    where
        L: SceList<T> + Fewest + Follows<A>,
    {
        self.start = start;
        let filled = self.list.refill(fill).and_then(|()| {
            if self.list.follows(start) {
                Ok(())
            } else {
                Err(CodecError::PresentIfMismatch)
            }
        });
        if filled.is_err() {
            *self = Self::default();
        }
        filled
    }
}

impl<I: Iterator, A: Law, const MAX: usize, const MIN: usize> Lawful<BoundedIter<I, MAX, MIN>, A> {
    /// Each entry through `f`, from the same start: a source that keeps its
    /// entries in its own type presents them as the codec's with
    /// `self.list.iter().map(Entry)`. The walk holds the mapped entries to the
    /// law.
    pub fn map<B, F: FnMut(I::Item) -> B>(
        self,
        f: F,
    ) -> Lawful<BoundedIter<core::iter::Map<I, F>, MAX, MIN>, A> {
        Lawful {
            start: self.start,
            list: self.list.map(f),
            law: PhantomData,
        }
    }
}

impl<I: Iterator, A: LawOver<I::Item>, const MAX: usize, const MIN: usize> IntoIterator
    for Lawful<BoundedIter<I, MAX, MIN>, A>
{
    type Item = I::Item;
    type IntoIter = LawfulWalk<BoundedIter<I, MAX, MIN>, A, I::Item>;

    fn into_iter(self) -> Self::IntoIter {
        LawfulWalk {
            entries: self.list,
            state: A::start(self.start),
            law: PhantomData,
        }
    }
}

/// The walk of a [`Lawful`] list: its entries, each held to the law as it is
/// read.
pub struct LawfulWalk<I, A: LawOver<E>, E> {
    entries: I,
    state: A::State,
    law: PhantomData<fn() -> (A, E)>,
}

impl<I: Iterator<Item = E>, A: LawOver<E>, E> Iterator for LawfulWalk<I, A, E> {
    type Item = E;

    fn next(&mut self) -> Option<E> {
        let entry = self.entries.next()?;
        match A::step(self.state, &entry) {
            Some(next) => self.state = next,
            None => law_broken(),
        }
        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entries.size_hint()
    }
}

impl<I: ExactSizeIterator<Item = E>, A: LawOver<E>, E> ExactSizeIterator for LawfulWalk<I, A, E> {}

/// A source's entries said otherwise on their walk than when the list was made.
fn law_broken() -> ! {
    panic!("a list's entries changed between its law and its walk (docs/adr/0061 §3)")
}

/// Project each entry of a decoded list into an owned list of the profile `L`,
/// keeping its bound, as [`crate::bounded::try_collect_bounded`] does, and its
/// start. The projection keeps what each entry says, and the owned list is held
/// to the law again.
pub fn try_collect_lawful<L, T, U, A, const N: usize, const MAX: usize, const MIN: usize>(
    src: Lawful<Bounded<heapless::Vec<T, N>, MAX, MIN>, A>,
    f: impl FnMut(T) -> Result<U, CodecError>,
) -> Result<Lawful<Bounded<L, MAX, MIN>, A>, CodecError>
where
    L: SceList<U> + Follows<A>,
    A: Law,
{
    let (start, list) = src.into_parts();
    let list: Bounded<L, MAX, MIN> = crate::bounded::try_collect_bounded(list, f)?;
    Lawful::new(start, list).ok_or(CodecError::PresentIfMismatch)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An entry that says one input, and has an identifier.
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    struct Entry {
        wide: bool,
        id: u8,
    }

    // A generated input or kind is asked of every form of the entry, a walk's
    // `&Entry` among them, as these are. An entry whose identifier is 7 says
    // nothing of the input, as an arm that does not read it.
    struct Wide;
    impl Says<Entry> for Wide {
        fn says(entry: &Entry) -> Option<bool> {
            (entry.id != 7).then_some(entry.wide)
        }
    }
    impl Says<&Entry> for Wide {
        fn says(entry: &&Entry) -> Option<bool> {
            (entry.id != 7).then_some(entry.wide)
        }
    }

    struct Marker;
    impl Key<Entry> for Marker {
        fn matches(entry: &Entry) -> bool {
            entry.id == 4
        }
    }
    impl Key<&Entry> for Marker {
        fn matches(entry: &&Entry) -> bool {
            entry.id == 4
        }
    }

    const fn e(wide: bool, id: u8) -> Entry {
        Entry { wide, id }
    }

    type Every = Uniform<Wide>;
    type After = AfterKey<Wide, Marker>;

    #[test]
    fn a_bound_flag_holds_every_entry_to_the_start() {
        let all = [e(true, 1), e(true, 2)];
        assert!(Lawful::<&[Entry], Every>::new(true, &all[..]).is_some());
        assert!(Lawful::<&[Entry], Every>::new(false, &all[..]).is_none());
        let mixed = [e(true, 1), e(false, 2)];
        assert!(Lawful::<&[Entry], Every>::new(true, &mixed[..]).is_none());
        assert!(Lawful::<&[Entry], Every>::new(false, &mixed[..]).is_none());
    }

    #[test]
    fn an_empty_list_holds_either_start() {
        let none: [Entry; 0] = [];
        let set = Lawful::<&[Entry], Every>::new(true, &none[..]).expect("an empty list follows");
        assert!(set.start());
        let clear =
            Lawful::<&[Entry], Every>::new(false, &none[..]).expect("an empty list follows");
        assert!(!clear.start());
    }

    #[test]
    fn the_entry_before_says_what_the_next_entry_says() {
        // The first says clear; an entry after the marker says set; any other clear.
        let ok = [e(false, 4), e(true, 1), e(false, 4), e(true, 4), e(true, 2)];
        assert!(Lawful::<&[Entry], After>::new((), &ok[..]).is_some());
        let first_set = [e(true, 1)];
        assert!(Lawful::<&[Entry], After>::new((), &first_set[..]).is_none());
        let not_after = [e(false, 1), e(true, 2)];
        assert!(Lawful::<&[Entry], After>::new((), &not_after[..]).is_none());
        let missed = [e(false, 4), e(false, 2)];
        assert!(Lawful::<&[Entry], After>::new((), &missed[..]).is_none());
    }

    #[test]
    fn an_entry_that_says_nothing_disagrees_with_no_state() {
        // After the marker, an entry that says nothing; the state after it is
        // still whether it is the marker, so the entry after it is clear.
        let quiet = [e(false, 4), e(true, 7), e(false, 2)];
        assert!(Lawful::<&[Entry], After>::new((), &quiet[..]).is_some());
        let first_quiet = [e(true, 7), e(false, 1)];
        assert!(Lawful::<&[Entry], After>::new((), &first_quiet[..]).is_some());
        let set_after_quiet = [e(false, 4), e(true, 7), e(true, 2)];
        assert!(Lawful::<&[Entry], After>::new((), &set_after_quiet[..]).is_none());
        // A bound flag holds a quiet entry under either start.
        let mixed = [e(true, 1), e(false, 7)];
        assert!(Lawful::<&[Entry], Every>::new(true, &mixed[..]).is_some());
        assert!(Lawful::<&[Entry], Every>::new(false, &mixed[..]).is_none());
    }

    #[test]
    fn two_laws_hold_a_list_together() {
        // A second input, said by every entry whose identifier is not zero.
        struct Tagged;
        impl Says<Entry> for Tagged {
            fn says(entry: &Entry) -> Option<bool> {
                Some(entry.id != 0)
            }
        }
        type Both = (Uniform<Tagged>, After);
        let ok = [e(false, 9), e(false, 4), e(true, 8)];
        assert!(Lawful::<&[Entry], Both>::new((true, ()), &ok[..]).is_some());
        // Each law alone refuses: the bound flag, then the entry before.
        assert!(Lawful::<&[Entry], Both>::new((false, ()), &ok[..]).is_none());
        let untagged = [e(false, 9), e(false, 0)];
        assert!(Lawful::<&[Entry], Both>::new((true, ()), &untagged[..]).is_none());
        let first_set = [e(true, 9)];
        assert!(Lawful::<&[Entry], Both>::new((true, ()), &first_set[..]).is_none());
    }

    #[test]
    fn a_walk_yields_the_entries_and_keeps_the_start() {
        let all = [e(true, 1), e(true, 2)];
        let list = Bounded::<&[Entry], 4>::new(&all[..]).expect("within the bound");
        let lawful = Lawful::<_, Every>::new(true, list).expect("every entry says the start");
        let walk = lawful.walk();
        assert!(walk.start());
        assert_eq!(walk.len(), 2);
        let read: heapless::Vec<Entry, 4> = walk.into_iter().copied().collect();
        assert_eq!(read.as_slice(), &all);
    }

    #[test]
    #[should_panic(expected = "between its law and its walk")]
    fn a_walk_whose_entries_say_otherwise_panics() {
        let all = [e(true, 1), e(true, 2)];
        let list = Bounded::<&[Entry], 4>::new(&all[..]).expect("within the bound");
        let lawful = Lawful::<_, Every>::new(true, list).expect("every entry says the start");
        // A source whose entries read otherwise the second time.
        for _ in lawful.walk().map(|entry| e(!entry.wide, entry.id)) {}
    }

    #[test]
    fn a_default_follows_from_its_first_entry() {
        let set: Lawful<Bounded<heapless::Vec<Entry, 4>, 4, 1>, Every> = Lawful::default();
        assert!(!set.start());
        assert_eq!(set.len(), 1);
        let after: Lawful<Bounded<heapless::Vec<Entry, 4>, 4, 1>, After> = Lawful::default();
        assert_eq!(after.len(), 1);
    }

    #[test]
    fn a_refill_that_breaks_the_law_leaves_the_default() {
        let mut list: Lawful<Bounded<heapless::Vec<Entry, 4>, 4>, Every> = Lawful::default();
        let filled = list.refill_from(true, |v| {
            v.push(e(true, 1))
                .map_err(|_| CodecError::TooManyElements)?;
            v.push(e(false, 2)).map_err(|_| CodecError::TooManyElements)
        });
        assert_eq!(filled, Err(CodecError::PresentIfMismatch));
        assert_eq!(list, Lawful::default());
        let filled = list.refill_from(true, |v| {
            v.push(e(true, 1)).map_err(|_| CodecError::TooManyElements)
        });
        assert_eq!(filled, Ok(()));
        assert!(list.start());
        assert_eq!(list.len(), 1);
    }
}
