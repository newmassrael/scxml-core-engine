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
//!
//! A codec's `bytes` and `string` fields are bounded the same way, counted in
//! bytes (docs/adr/0077 §3): [`Bytes`] and [`Text`] are names for a [`Bounded`]
//! slice or `str`, and an owned buffer is held through [`Bounded::from_buf`] or
//! [`Bounded::from_str_buf`].

use core::ops::Deref;

use crate::storage::SceList;
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

/// A list `L` of at most `MAX` entries and at least `MIN`.
///
/// The only ways to hold one are [`Self::new`], which counts the entries, and the
/// conversions whose source cannot hold more than `MAX` nor fewer than `MIN`,
/// which the build checks. It reads as the list it holds ([`Deref`]); it has no
/// mutable access, so its entries cannot leave the bounds once it is made.
///
/// `MIN` is none for every list but a chain closed by an entry flag, which is one
/// entry or more (docs/adr/0065 §6 R7), so a list type that does not name it
/// reads as before.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounded<L, const MAX: usize, const MIN: usize = 0>(L);

impl<L: core::fmt::Debug, const MAX: usize, const MIN: usize> core::fmt::Debug
    for Bounded<L, MAX, MIN>
{
    /// The list as it prints: the bound is a fact about it, not more of it, so a
    /// value holding a `Bounded` prints as one holding the list does.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<L: Entries, const MAX: usize, const MIN: usize> Bounded<L, MAX, MIN> {
    /// `list` as a list of at most `MAX` entries and at least `MIN`, or `None`
    /// when it holds more or fewer.
    pub fn new(list: L) -> Option<Self> {
        if (MIN..=MAX).contains(&list.entry_count()) {
            Some(Self(list))
        } else {
            None
        }
    }

    /// The entries in order, with their count.
    pub fn iter(&self) -> BoundedIter<L::Iter<'_>, MAX, MIN> {
        BoundedIter {
            iter: self.0.entries(),
            remaining: self.0.entry_count(),
        }
    }

    /// How many entries the list holds, at most `MAX` and at least `MIN`.
    pub fn len(&self) -> usize {
        self.0.entry_count()
    }

    /// Whether the list holds no entry.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<L: Entries, const MAX: usize, const MIN: usize> Bounded<L, MAX, MIN> {
    /// The entries of a storage profile's owned list, with their count.
    ///
    /// [`Self::iter`] over a profile's list `S::List<T, N>` names its entries as
    /// the profile's iterator yields them, which a generic owned form cannot read
    /// as `&T`; this reads them through [`crate::storage::SceList::as_slice`]. The
    /// count is still the one [`Self::new`] checked, so a list whose slice and
    /// count disagree is caught by the walk and never written past its bound.
    pub fn list_iter<T>(&self) -> BoundedIter<core::slice::Iter<'_, T>, MAX, MIN>
    where
        L: crate::storage::SceList<T>,
    {
        BoundedIter {
            iter: self.0.as_slice().iter(),
            remaining: self.0.entry_count(),
        }
    }
}

impl<L, const MAX: usize, const MIN: usize> Bounded<L, MAX, MIN> {
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

impl<L, const MAX: usize, const MIN: usize> Deref for Bounded<L, MAX, MIN> {
    type Target = L;

    fn deref(&self) -> &L {
        &self.0
    }
}

/// A list a bounded value's default is made of: the fewest entries it may
/// hold, each the entry's own default. A list of none is empty, a list of one
/// entry or more holds one (docs/adr/0065 §6 R7), and a byte string of an exact
/// length holds that many zeros (docs/adr/0077 §3), so the default value of a
/// codec that holds any of them is a frame its decode reads back.
pub trait Fewest: Sized {
    /// The list of `MIN` entries. A list that cannot hold `MIN` entries is
    /// refused when the build reaches it.
    fn fewest<const MIN: usize>() -> Self;
}

impl<L: Fewest, const MAX: usize, const MIN: usize> Default for Bounded<L, MAX, MIN> {
    /// The fewest entries the bounds admit ([`Fewest`]).
    fn default() -> Self {
        const { assert!(MIN <= MAX, "the bounds admit no list") };
        Self(L::fewest::<MIN>())
    }
}

impl<T: Default, const N: usize> Fewest for heapless::Vec<T, N> {
    fn fewest<const MIN: usize>() -> Self {
        const { assert!(MIN <= N, "the list cannot hold its fewest entries") };
        let mut list = heapless::Vec::new();
        for _ in 0..MIN {
            if list.push(T::default()).is_err() {
                unreachable!("a list whose capacity is its fewest holds them");
            }
        }
        list
    }
}

#[cfg(feature = "alloc")]
impl<T: Default> Fewest for alloc::vec::Vec<T> {
    fn fewest<const MIN: usize>() -> Self {
        (0..MIN).map(|_| T::default()).collect()
    }
}

impl<'a, W: ListWalk<K>, const K: usize> Fewest for ListView<'a, W, K> {
    /// A view over no bytes, which holds no entry.
    fn fewest<const MIN: usize>() -> Self {
        const { assert!(MIN == 0, "a list view over no bytes holds no entry") };
        ListView::default()
    }
}

impl<'a, W: ListWalk<K>, const K: usize> Fewest for crate::view::NonEmptyListView<'a, W, K>
where
    W::Entry<'a>: Default + Clone,
{
    /// One default entry, as the list's own default holds.
    fn fewest<const MIN: usize>() -> Self {
        const { assert!(MIN == 1, "a list of one entry or more holds one by default") };
        crate::view::NonEmptyListView::default()
    }
}

impl Fewest for &[u8] {
    /// `MIN` zero bytes, borrowed from a constant.
    fn fewest<const MIN: usize>() -> Self {
        &const { [0u8; MIN] }
    }
}

impl<'a, const N: usize> From<Bytes<'a, N, N>> for &'a [u8; N] {
    /// Bytes of an exact length as the array they are: the type holds the
    /// length, so the conversion checks nothing.
    fn from(bytes: Bytes<'a, N, N>) -> Self {
        match bytes.0.try_into() {
            Ok(array) => array,
            Err(_) => unreachable!("bytes of an exact length are that long"),
        }
    }
}

impl<'a, const N: usize> From<Bytes<'a, N, N>> for [u8; N] {
    /// A copy of bytes of an exact length, the array the reference conversion
    /// lends.
    fn from(bytes: Bytes<'a, N, N>) -> Self {
        *<&[u8; N]>::from(bytes)
    }
}

impl Fewest for &str {
    /// `MIN` NULs, the text of `MIN` zero bytes.
    fn fewest<const MIN: usize>() -> Self {
        match core::str::from_utf8(&const { [0u8; MIN] }) {
            Ok(text) => text,
            Err(_) => unreachable!("zero bytes are UTF-8"),
        }
    }
}

/// A byte buffer a profile owns holds `MIN` zeros by default, as [`Bytes`] does.
macro_rules! bytes_default_to_zeros {
    ($buffer:ident) => {
        impl<const N: usize> Fewest for crate::storage::$buffer<N> {
            fn fewest<const MIN: usize>() -> Self {
                match Self::from_slice(&[0u8; MIN]) {
                    Ok(buffer) => buffer,
                    Err(_) => unreachable!("a field's capacity holds its fewest bytes"),
                }
            }
        }
    };
}

/// A text buffer a profile owns holds `MIN` NULs by default, as [`Text`] does.
macro_rules! text_defaults_to_nuls {
    ($buffer:ident) => {
        impl<const N: usize> Fewest for crate::storage::$buffer<N> {
            fn fewest<const MIN: usize>() -> Self {
                match Self::from_text(<&str as Fewest>::fewest::<MIN>()) {
                    Ok(text) => text,
                    Err(_) => unreachable!("a field's capacity holds its fewest bytes"),
                }
            }
        }
    };
}

bytes_default_to_zeros!(InlineBytes);
text_defaults_to_nuls!(InlineStr);
#[cfg(feature = "alloc")]
bytes_default_to_zeros!(HeapBytes);
#[cfg(feature = "alloc")]
text_defaults_to_nuls!(HeapStr);

impl<L, const MAX: usize, const MIN: usize> Bounded<L, MAX, MIN> {
    /// Fill the list in place: `fill` is handed it cleared, and the count it
    /// leaves is checked against the bounds, fewer than `MIN` being a frame cut
    /// short. A decode builds a list here rather than in a local it then moves,
    /// since a `heapless::Vec` moves by copying its whole capacity.
    ///
    /// On `Err` the list holds `MIN` default entries, so it is still inside its
    /// bounds, whatever `fill` left; on `CapacityExceeded` it keeps the entries
    /// it held where they are inside them.
    ///
    /// A list past what its profile holds is read whole and not held past that
    /// capacity, and `fill` then answers `CapacityExceeded` (docs/adr/0077 §7).
    /// The count the minimum is judged by is the one held, and it is the count
    /// read wherever that is below the minimum: every profile holds at least
    /// `MIN` entries (the build asserts it of each list's fewest), so a list
    /// keeps fewer than `MIN` only when it read fewer.
    pub fn refill<T: Default>(
        &mut self,
        fill: impl FnOnce(&mut L) -> Result<(), crate::codec::CodecError>,
    ) -> Result<(), crate::codec::CodecError>
    where
        L: crate::storage::SceList<T>,
    {
        self.0.clear();
        let filled = fill(&mut self.0).and_then(|()| match self.0.as_slice().len() {
            n if n < MIN => Err(crate::codec::CodecError::NeedMoreBytes),
            n if n > MAX => Err(crate::codec::CodecError::TooManyElements),
            _ => Ok(()),
        });
        // A list read past its capacity keeps the entries it held, which are
        // inside its bounds, so what holds them can still be judged by them
        // (a list's law, docs/adr/0065 §6 R8). Any other refusal leaves the
        // fewest default entries.
        let kept = match filled {
            Ok(()) => true,
            Err(crate::codec::CodecError::CapacityExceeded) => {
                (MIN..=MAX).contains(&self.0.as_slice().len())
            }
            Err(_) => false,
        };
        if !kept {
            self.0.clear();
            for _ in 0..MIN {
                if self.0.try_push(T::default()).is_err() {
                    unreachable!("a list's capacity holds its fewest entries");
                }
            }
        }
        filled
    }
}

impl<'a, T, const N: usize, const MAX: usize, const MIN: usize> From<&'a [T; N]>
    for Bounded<&'a [T], MAX, MIN>
{
    /// An array of `N` entries, which the build admits only where `N` lies
    /// between `MIN` and `MAX`: a list of fixed length costs no check.
    fn from(entries: &'a [T; N]) -> Self {
        const {
            assert!(
                N <= MAX,
                "the array holds more entries than the list's bound"
            )
        };
        const {
            assert!(
                N >= MIN,
                "the array holds fewer entries than the list's bound"
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

impl<'a> Entries for &'a str {
    /// A text counts the UTF-8 bytes the wire carries, not its characters.
    type Iter<'s>
        = core::str::Bytes<'a>
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.len()
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.bytes()
    }
}

/// A byte string of at least `MIN` bytes and at most `MAX` (docs/adr/0077 §3):
/// a codec's `bytes` field, borrowed. The bound is the one a list has, counted
/// in bytes, so it is the same type: [`Bounded::new`] checks a slice once, and
/// an array whose length lies in the bounds converts without a check.
pub type Bytes<'a, const MAX: usize, const MIN: usize = 0> = Bounded<&'a [u8], MAX, MIN>;

/// A text of at least `MIN` and at most `MAX` UTF-8 bytes (docs/adr/0077 §3): a
/// codec's `string` field, borrowed, bounded as [`Bytes`] is.
pub type Text<'a, const MAX: usize, const MIN: usize = 0> = Bounded<&'a str, MAX, MIN>;

// Bytes and text compare with what they hold, as a `Vec` compares with a slice
// and a `String` with a `str`: the bounds are a fact about the value, not more
// of it, so `decoded.payload == b"abc"` reads as it did on a bare slice.
impl<T: PartialEq<U>, U, const MAX: usize, const MIN: usize> PartialEq<[U]>
    for Bounded<&[T], MAX, MIN>
{
    fn eq(&self, other: &[U]) -> bool {
        self.0 == other
    }
}

impl<T: PartialEq<U>, U, const MAX: usize, const MIN: usize> PartialEq<&[U]>
    for Bounded<&[T], MAX, MIN>
{
    fn eq(&self, other: &&[U]) -> bool {
        self.0 == *other
    }
}

impl<T: PartialEq<U>, U, const N: usize, const MAX: usize, const MIN: usize> PartialEq<[U; N]>
    for Bounded<&[T], MAX, MIN>
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0 == other
    }
}

impl<T: PartialEq<U>, U, const N: usize, const MAX: usize, const MIN: usize> PartialEq<&[U; N]>
    for Bounded<&[T], MAX, MIN>
{
    fn eq(&self, other: &&[U; N]) -> bool {
        self.0 == *other
    }
}

impl<const MAX: usize, const MIN: usize> PartialEq<str> for Bounded<&str, MAX, MIN> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl<const MAX: usize, const MIN: usize> PartialEq<&str> for Bounded<&str, MAX, MIN> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl<B: crate::storage::HeldBytes, const MAX: usize, const MIN: usize> Bounded<B, MAX, MIN> {
    /// The bytes, borrowed with their bounds: a value's source read, which
    /// needs no check since what holds them cannot change once it is held.
    pub fn as_bytes(&self) -> Bytes<'_, MAX, MIN> {
        Bounded(self.0.held_bytes())
    }

    /// The buffer a profile built for bytes inside the bounds, held once its
    /// length is checked: the container an [`crate::storage::OriginStorage`]
    /// profile holds the field in builds itself from the origin however it
    /// chooses ([`crate::storage::FromOrigin`]), so its length is the
    /// container's word, not the type's. A buffer of another length is refused as a length out of range.
    pub fn from_built(
        buf: Result<B, crate::codec::CodecError>,
    ) -> Result<Self, crate::codec::CodecError> {
        let buf = buf?;
        if (MIN..=MAX).contains(&buf.held_bytes().len()) {
            Ok(Self(buf))
        } else {
            Err(crate::codec::CodecError::LengthOutOfRange)
        }
    }
}

impl<'f, const MAX: usize, const MIN: usize> Bytes<'f, MAX, MIN> {
    /// These bytes of a frame as the profile `S` holds the field `Tag`: a read
    /// that has held them to their bounds, stored. The bounds carry over.
    pub fn store<S, Tag>(self) -> Result<Bounded<S::Bytes, MAX, MIN>, crate::codec::CodecError>
    where
        S: crate::storage::StoresBytes<'f, Tag>,
    {
        S::store_bytes(self.0).map(Bounded)
    }
}

impl<'f, const MAX: usize, const MIN: usize> Text<'f, MAX, MIN> {
    /// This text of a frame as the profile `S` holds the field `Tag`, as
    /// [`Bounded::store`] does for bytes.
    pub fn store<S, Tag>(self) -> Result<Bounded<S::Str, MAX, MIN>, crate::codec::CodecError>
    where
        S: crate::storage::StoresStr<'f, Tag>,
    {
        S::store_str(self.0).map(Bounded)
    }
}

impl<B: crate::storage::SceByteBuf, const MAX: usize, const MIN: usize> Bounded<B, MAX, MIN> {
    /// An owned byte buffer as bytes of at least `MIN` and at most `MAX`, or
    /// `None` when it holds fewer or more. A storage profile's buffer may grow
    /// past its declared capacity, so its length is checked here, once.
    pub fn from_buf(buf: B) -> Option<Self> {
        if (MIN..=MAX).contains(&buf.as_slice().len()) {
            Some(Self(buf))
        } else {
            None
        }
    }

    /// `bytes` copied into the profile's buffer, for an owned value built by
    /// hand: `LengthOutOfRange` where they lie outside the bounds, which is the
    /// error a decode of them gives, and the buffer's own capacity error on a
    /// profile whose capacity is below them. The buffer is the field's, so
    /// `payload: Bounded::from_slice(data)?` names no profile.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, crate::codec::CodecError> {
        let bytes =
            Bytes::<'_, MAX, MIN>::new(bytes).ok_or(crate::codec::CodecError::LengthOutOfRange)?;
        Self::copied(bytes)
    }

    /// Bytes already inside the bounds, copied into the profile's buffer: the
    /// owned projection of a decoded value. The bounds carry over without a
    /// check; the one error is the buffer's own capacity, on a profile whose
    /// capacity is below `MAX` ([`crate::storage::SceByteBuf::from_slice`]).
    pub fn copied(bytes: Bytes<'_, MAX, MIN>) -> Result<Self, crate::codec::CodecError> {
        B::from_slice(bytes.0).map(Self)
    }
}

impl<T: crate::storage::HeldStr, const MAX: usize, const MIN: usize> Bounded<T, MAX, MIN> {
    /// The text, borrowed with its bounds.
    pub fn as_text(&self) -> Text<'_, MAX, MIN> {
        Bounded(self.0.held_str())
    }

    /// The text buffer a profile built, held once its length is checked, as
    /// [`Bounded::from_built`] holds bytes.
    pub fn from_built_text(
        text: Result<T, crate::codec::CodecError>,
    ) -> Result<Self, crate::codec::CodecError> {
        let text = text?;
        if (MIN..=MAX).contains(&text.held_str().len()) {
            Ok(Self(text))
        } else {
            Err(crate::codec::CodecError::LengthOutOfRange)
        }
    }
}

impl<T: crate::storage::SceStr, const MAX: usize, const MIN: usize> Bounded<T, MAX, MIN> {
    /// An owned text buffer as text of at least `MIN` and at most `MAX` UTF-8
    /// bytes, or `None` when it holds fewer or more.
    pub fn from_str_buf(text: T) -> Option<Self> {
        if (MIN..=MAX).contains(&text.as_str().len()) {
            Some(Self(text))
        } else {
            None
        }
    }

    /// `text` copied into the profile's buffer, for an owned value built by
    /// hand, refused as [`Bounded::from_slice`] refuses bytes.
    pub fn from_text(text: &str) -> Result<Self, crate::codec::CodecError> {
        let text =
            Text::<'_, MAX, MIN>::new(text).ok_or(crate::codec::CodecError::LengthOutOfRange)?;
        Self::copied_text(text)
    }

    /// Text already inside the bounds, copied into the profile's buffer, as
    /// [`Bounded::copied`] copies bytes.
    pub fn copied_text(text: Text<'_, MAX, MIN>) -> Result<Self, crate::codec::CodecError> {
        T::from_view(text.0).map(Self)
    }
}

/// The entries of a list of at most `MAX` and at least `MIN`, in order, with how
/// many there are: what a codec's source interface returns for a list
/// (docs/adr/0065 §6 R2, R7).
///
/// A codec reads [`ExactSizeIterator::len`] before the first entry, for the count
/// it writes or the continuation bit of each entry, and then walks the entries
/// once. A source whose entries, walked again, are more or fewer than it counted
/// has broken what an accessor is (a read of a value, docs/adr/0061 §3), and the
/// walk panics rather than write a frame whose count disagrees with its entries.
#[derive(Debug, Clone)]
pub struct BoundedIter<I, const MAX: usize, const MIN: usize = 0> {
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
///
/// The decoded list is a `Bounded` of whatever list its profile holds it in, a
/// growable one or an inline one (docs/adr/0077 §7); the projection keeps its
/// count, so the owned list keeps both bounds.
pub fn try_collect_bounded<K, L, T, U, const MAX: usize, const MIN: usize>(
    src: Bounded<K, MAX, MIN>,
    f: impl FnMut(T) -> Result<U, crate::codec::CodecError>,
) -> Result<Bounded<L, MAX, MIN>, crate::codec::CodecError>
where
    K: IntoIterator<Item = T>,
    L: crate::storage::SceList<U>,
{
    crate::storage::try_collect_list(src.into_inner(), f).map(Bounded)
}

impl<I, const MAX: usize, const MIN: usize> BoundedIter<I, MAX, MIN> {
    /// `entries` with the count `remaining` that a list already inside the
    /// bounds was checked to hold: a refined list re-expressed as its walk.
    pub(crate) fn counted(entries: I, remaining: usize) -> Self {
        Self {
            iter: entries,
            remaining,
        }
    }
}

impl<I: Iterator + Clone, const MAX: usize, const MIN: usize> Entries for BoundedIter<I, MAX, MIN> {
    type Iter<'s>
        = Self
    where
        Self: 's;

    fn entry_count(&self) -> usize {
        self.remaining
    }

    fn entries(&self) -> Self::Iter<'_> {
        self.clone()
    }
}

impl<I: Iterator + Clone, const MAX: usize, const MIN: usize> BoundedIter<I, MAX, MIN> {
    /// `entries` as the entries of a list of at most `MAX` and at least `MIN`,
    /// counted once from a clone, or `None` when there are more or fewer.
    pub fn new(entries: I) -> Option<Self> {
        let count = entries.clone().count();
        if (MIN..=MAX).contains(&count) {
            Some(Self {
                iter: entries,
                remaining: count,
            })
        } else {
            None
        }
    }
}

impl<I: Iterator, const MAX: usize, const MIN: usize> BoundedIter<I, MAX, MIN> {
    /// Each entry through `f`: as many entries as before, so the same bounds.
    ///
    /// It stands in for [`Iterator::map`], which would drop the count, so that a
    /// source keeping its entries in its own type presents them as the codec's
    /// with the call it would write anyway: `self.list.iter().map(Entry)`.
    pub fn map<B, F: FnMut(I::Item) -> B>(
        self,
        f: F,
    ) -> BoundedIter<core::iter::Map<I, F>, MAX, MIN> {
        BoundedIter {
            iter: self.iter.map(f),
            remaining: self.remaining,
        }
    }
}

impl<I: Iterator, const MAX: usize, const MIN: usize> Iterator for BoundedIter<I, MAX, MIN> {
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

impl<I: Iterator, const MAX: usize, const MIN: usize> ExactSizeIterator
    for BoundedIter<I, MAX, MIN>
{
}

/// A list that knows how many entries it holds: what [`Aligned::new`] compares.
pub trait Counted {
    /// How many entries the list holds.
    fn count(&self) -> usize;
}

impl<L: Entries, const MAX: usize, const MIN: usize> Counted for Bounded<L, MAX, MIN> {
    fn count(&self) -> usize {
        self.len()
    }
}

impl<L: Entries> Counted for L {
    fn count(&self) -> usize {
        self.entry_count()
    }
}

/// Two lists of one count (docs/adr/0065 §6 R5): a codec that writes one number
/// for two lists holds both as this, so the number cannot disagree with either.
///
/// The two lists stay what the wire makes them, two runs of entries, so a view
/// holds two spans of bytes and nothing is copied to pair them. The value is made
/// by a constructor that compares them once, as [`Bounded`] is. Its only mutable
/// access is a generated decode's, which fills each list where the value holds
/// it (`refill_first`, `refill_second`, hidden from the documentation) and leaves
/// the two of one count after every call, so it stays what it was checked to be
/// (docs/adr/0065 §6 R5, amendment of 2026-11-05).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aligned<A, B> {
    first: A,
    second: B,
}

impl<A, B> Aligned<A, B> {
    /// `first` and `second` as two lists of one count, or `None` when their
    /// counts differ.
    pub fn new(first: A, second: B) -> Option<Self>
    where
        A: Counted,
        B: Counted,
    {
        (first.count() == second.count()).then_some(Self { first, second })
    }

    /// The first list.
    pub fn first(&self) -> &A {
        &self.first
    }

    /// The second list.
    pub fn second(&self) -> &B {
        &self.second
    }

    /// The two lists, without the fact that they are of one count.
    pub fn into_parts(self) -> (A, B) {
        (self.first, self.second)
    }

    /// How many entries each list holds.
    pub fn len(&self) -> usize
    where
        A: Counted,
    {
        self.first.count()
    }

    /// Whether both lists are empty.
    pub fn is_empty(&self) -> bool
    where
        A: Counted,
    {
        self.len() == 0
    }
}

impl<LA, LB, const MA: usize, const MB: usize> Aligned<Bounded<LA, MA>, Bounded<LB, MB>>
where
    LA: crate::storage::HeldList,
    LB: crate::storage::HeldList,
    LA::Entry: Default,
    LB::Entry: Default,
{
    /// Fill the first list in place, as [`Bounded::refill`] fills a list, where
    /// the value holds it: a decode reads the two lists of one count into the
    /// arm that holds them, since a list moved into an arm copies its whole
    /// capacity (docs/adr/0077 §7).
    ///
    /// The second list then holds as many default entries, so the two are of
    /// one count until [`Self::refill_second`] reads the second. A second list
    /// whose bound is below the first's count is refused as past its bound, and
    /// one whose profile holds fewer entries as past its capacity, which is
    /// what reading it would have answered. On `Err` both lists are empty.
    #[doc(hidden)]
    pub fn refill_first(
        &mut self,
        fill: impl FnOnce(&mut LA) -> Result<(), crate::codec::CodecError>,
    ) -> Result<(), crate::codec::CodecError> {
        let filled = self.first.refill::<LA::Entry>(fill).and_then(|()| {
            let count = SceList::as_slice(&self.first.0).len();
            SceList::clear(&mut self.second.0);
            if count > MB {
                return Err(crate::codec::CodecError::TooManyElements);
            }
            for _ in 0..count {
                SceList::try_push(&mut self.second.0, Default::default())?;
            }
            Ok(())
        });
        if filled.is_err() {
            self.empty_both();
        }
        filled
    }

    /// Fill the second list in place, as [`Self::refill_first`] fills the
    /// first, and hold it to the first's count. The decode reads both by the
    /// one count, each whole, so the two hold different counts only where one
    /// of them was past what its profile holds and kept fewer entries than it
    /// read (docs/adr/0077 §7): a count that differs is `CapacityExceeded`,
    /// said once the second list is read whole. On `Err` both lists are empty.
    #[doc(hidden)]
    pub fn refill_second(
        &mut self,
        fill: impl FnOnce(&mut LB) -> Result<(), crate::codec::CodecError>,
    ) -> Result<(), crate::codec::CodecError> {
        let filled = self.second.refill::<LB::Entry>(fill).and_then(|()| {
            let first = SceList::as_slice(&self.first.0).len();
            if SceList::as_slice(&self.second.0).len() == first {
                Ok(())
            } else {
                Err(crate::codec::CodecError::CapacityExceeded)
            }
        });
        if filled.is_err() {
            self.empty_both();
        }
        filled
    }

    /// Both lists empty, which are of one count: what a refill leaves on `Err`.
    fn empty_both(&mut self) {
        SceList::clear(&mut self.first.0);
        SceList::clear(&mut self.second.0);
    }
}

impl<A: Counted + Default, B: Counted + Default> Default for Aligned<A, B> {
    /// Two default lists. The lists a count is shared by have no fewest entries,
    /// so both start empty.
    fn default() -> Self {
        Self::new(A::default(), B::default()).expect("two lists with no fewest start empty")
    }
}

/// The one panic of a list walk, shared by every instance so that each costs a
/// call and not a message of its own.
#[cold]
#[inline(never)]
pub(crate) fn entries_changed() -> ! {
    panic!("a list's entries changed between its count and its walk (docs/adr/0061 §3)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_lists_of_one_count_are_held_and_two_of_different_counts_are_not() {
        let three = Bounded::<&[u8], 4>::new(&[1, 2, 3]).expect("within the bound");
        let also_three = Bounded::<&[u16], 8>::new(&[4, 5, 6]).expect("within the bound");
        let two = Bounded::<&[u16], 8>::new(&[4, 5]).expect("within the bound");
        assert!(Aligned::new(three, two).is_none());
        let aligned = Aligned::new(three, also_three).expect("one count");
        assert_eq!(aligned.len(), 3);
        // A list and a source's walks are held the same way.
        let walks = Aligned::new(aligned.first().iter(), aligned.second().iter());
        let (first, second) = walks.expect("a walk keeps its list's count").into_parts();
        assert!(first.eq([&1, &2, &3]));
        assert!(second.eq([&4, &5, &6]));
        assert!(Aligned::new(&[1u8][..], heapless::Vec::<u16, 2>::new()).is_none());
        // The default holds two empty lists, whose counts agree.
        let empty = Aligned::<heapless::Vec<u8, 4>, heapless::Vec<u16, 8>>::default();
        assert!(empty.is_empty());
    }

    type Pair = Aligned<Bounded<heapless::Vec<u8, 4>, 4>, Bounded<heapless::Vec<u16, 4>, 2>>;

    /// A fill that pushes `entries`, as a generated list decode pushes each
    /// entry it reads.
    fn pushing<T: Copy>(
        entries: &[T],
    ) -> impl FnOnce(&mut heapless::Vec<T, 4>) -> Result<(), crate::codec::CodecError> + '_ {
        move |list| {
            for entry in entries {
                crate::storage::SceList::try_push(list, *entry)?;
            }
            Ok(())
        }
    }

    #[test]
    fn two_lists_of_one_count_filled_in_place_stay_of_one_count_at_every_step() {
        use crate::codec::CodecError;
        let mut pair = Pair::default();
        assert_eq!(pair.refill_first(pushing(&[1u8, 2])), Ok(()));
        // The second holds as many default entries until it is read.
        assert_eq!(pair.first().as_slice(), &[1u8, 2][..]);
        assert_eq!(pair.second().as_slice(), &[0u16, 0][..]);
        assert_eq!(pair.refill_second(pushing(&[7u16, 8])), Ok(()));
        assert_eq!(pair.second().as_slice(), &[7u16, 8][..]);
        assert_eq!(pair.len(), 2);

        // A second list of another count is one whose list, or the first's,
        // held fewer entries than it read: past a capacity, and both emptied.
        assert_eq!(
            pair.refill_second(pushing(&[7u16])),
            Err(CodecError::CapacityExceeded)
        );
        assert!(pair.first().is_empty() && pair.second().is_empty());
        assert_eq!(pair.refill_first(pushing(&[1u8])), Ok(()));
        assert_eq!(
            pair.refill_second(pushing(&[7u16, 8])),
            Err(CodecError::CapacityExceeded)
        );
        assert!(pair.first().is_empty() && pair.second().is_empty());
        // A first list past its capacity leaves both empty, and the second,
        // read whole after it, is past a capacity too.
        assert_eq!(
            pair.refill_first(|_| Err(CodecError::CapacityExceeded)),
            Err(CodecError::CapacityExceeded)
        );
        assert!(pair.first().is_empty() && pair.second().is_empty());
        assert_eq!(
            pair.refill_second(pushing(&[7u16])),
            Err(CodecError::CapacityExceeded)
        );
        assert!(pair.first().is_empty() && pair.second().is_empty());

        // A first list past the second's bound is refused before the second
        // is read, and both are emptied.
        assert_eq!(
            pair.refill_first(pushing(&[1u8, 2, 3])),
            Err(CodecError::TooManyElements)
        );
        assert!(pair.first().is_empty() && pair.second().is_empty());
    }

    #[test]
    fn a_list_read_past_its_capacity_keeps_the_entries_it_held() {
        use crate::codec::CodecError;
        let mut list = Bounded::<heapless::Vec<u8, 4>, 4, 1>::default();
        assert_eq!(
            list.refill(|v| {
                crate::storage::SceList::try_push(v, 7u8)?;
                crate::storage::SceList::try_push(v, 8u8)?;
                Err(CodecError::CapacityExceeded)
            }),
            Err(CodecError::CapacityExceeded)
        );
        assert_eq!(list.as_slice(), &[7u8, 8][..]);
        // Held entries outside the bounds are not kept: none, below the fewest.
        assert_eq!(
            list.refill(|_| Err(CodecError::CapacityExceeded)),
            Err(CodecError::CapacityExceeded)
        );
        assert_eq!(list.as_slice(), &[0u8][..]);
    }

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
    fn bytes_are_held_between_their_bounds_and_an_array_inside_them_costs_no_check() {
        let zid = [1u8; 16];
        assert!(Bytes::<16, 1>::new(&zid[..]).is_some());
        assert!(Bytes::<15, 1>::new(&zid[..]).is_none());
        assert!(Bytes::<16, 1>::new(&[][..]).is_none());
        let fixed: Bytes<'_, 16, 1> = (&zid).into();
        assert_eq!(fixed.len(), 16);
        assert_eq!(*fixed, &zid[..]);
        // It compares with what it holds, as a slice or an array.
        assert_eq!(fixed, &zid[..]);
        assert_eq!(fixed, zid);
        assert_eq!(fixed, &zid);
        assert!(Text::<4>::new("ab").unwrap() == "ab");
    }

    #[test]
    fn a_text_is_counted_in_the_bytes_the_wire_carries() {
        // Two characters, four UTF-8 bytes.
        assert!(Text::<4>::new("\u{e9}\u{e9}").is_some());
        assert!(Text::<3>::new("\u{e9}\u{e9}").is_none());
        assert_eq!(Text::<4>::new("ab").map(|t| t.len()), Some(2));
    }

    #[test]
    fn the_default_of_bytes_is_their_fewest_zero_bytes() {
        assert_eq!(*Bytes::<'_, 16, 1>::default(), &[0u8][..]);
        assert_eq!(Text::<'_, 8, 1>::default().len(), 1);
        assert!(Bytes::<'_, 16>::default().is_empty());
        // An exact length: that many zeros, and the array it is.
        assert_eq!(*Bytes::<'_, 16, 16>::default(), &[0u8; 16][..]);
        let id = [7u8; 16];
        let exact: Bytes<'_, 16, 16> = (&id).into();
        assert_eq!(<[u8; 16]>::from(exact), id);
        assert_eq!(<&[u8; 16]>::from(exact), &id);
    }

    #[test]
    fn an_owned_buffer_is_held_once_checked_and_reads_back_without_a_check() {
        use crate::storage::{InlineBytes, InlineStr, SceStr};
        let buf = InlineBytes::<8>::from_slice(&[1, 2, 3]).unwrap();
        assert!(Bounded::<_, 2>::from_buf(buf.clone()).is_none());
        let held = Bounded::<_, 3, 1>::from_buf(buf).unwrap();
        assert_eq!(*held.as_bytes(), &[1u8, 2, 3][..]);
        let copied = Bounded::<InlineBytes<3>, 3, 1>::copied(held.as_bytes()).unwrap();
        assert_eq!(*copied.as_bytes(), *held.as_bytes());
        // A profile whose capacity is below the bound refuses for its capacity.
        assert!(Bounded::<InlineBytes<2>, 3, 1>::copied(held.as_bytes()).is_err());
        // Built by hand, the bounds are checked first and refused as a decode
        // of the same bytes would refuse them.
        assert_eq!(
            Bounded::<InlineBytes<8>, 2>::from_slice(&[1, 2, 3]),
            Err(crate::codec::CodecError::LengthOutOfRange)
        );
        assert_eq!(
            Bounded::<InlineBytes<8>, 3>::from_slice(&[1, 2, 3]).map(|b| b.as_bytes() == [1, 2, 3]),
            Ok(true)
        );
        assert!(Bounded::<InlineStr<8>, 3>::from_text("abcd").is_err());
        let text = InlineStr::<8>::from_view("abc").unwrap();
        assert!(Bounded::<_, 2>::from_str_buf(text.clone()).is_none());
        assert_eq!(
            *Bounded::<_, 3>::from_str_buf(text).unwrap().as_text(),
            "abc"
        );
    }

    #[test]
    fn a_heapless_list_converts_where_its_capacity_is_inside_the_bound() {
        let mut list = heapless::Vec::<u8, 4>::new();
        list.push(5).unwrap();
        let bounded: Bounded<heapless::Vec<u8, 4>, 4> = list.into();
        assert_eq!(bounded.iter().copied().next(), Some(5));
    }

    #[test]
    fn a_list_of_one_entry_or_more_holds_none_fewer() {
        let empty: &[u8] = &[];
        assert!(Bounded::<&[u8], 3, 1>::new(empty).is_none());
        assert_eq!(
            Bounded::<&[u8], 3, 1>::new(&[9u8][..]).map(|l| l.len()),
            Some(1)
        );
        let array: Bounded<&[u8], 3, 1> = (&[1u8, 2]).into();
        assert_eq!(array.iter().len(), 2);
        assert!(BoundedIter::<_, 3, 1>::new(empty.iter()).is_none());
        assert!(BoundedIter::<_, 3, 1>::new([4u8].iter()).is_some());
    }

    #[test]
    fn the_default_of_a_list_of_one_entry_or_more_is_one_default_entry() {
        let one = Bounded::<heapless::Vec<u8, 4>, 4, 1>::default();
        assert_eq!(one.as_slice(), &[0u8][..]);
    }

    #[test]
    fn a_refill_keeps_the_list_inside_its_bounds() {
        let mut list = Bounded::<heapless::Vec<u8, 4>, 4, 1>::default();
        list.refill(|v| {
            v.push(7)
                .and_then(|()| v.push(8))
                .map_err(|_| unreachable!("a list of four holds two"))
        })
        .expect("two entries lie inside the bounds");
        assert_eq!(list.as_slice(), &[7u8, 8][..]);
        // A fill that leaves none is a frame cut short, and the list holds its
        // fewest default entries again.
        assert_eq!(
            list.refill(|_| Ok(())),
            Err(crate::codec::CodecError::NeedMoreBytes)
        );
        assert_eq!(list.as_slice(), &[0u8][..]);
        // A fill that fails leaves the list inside its bounds too.
        assert_eq!(
            list.refill(|v| {
                v.clear();
                Err(crate::codec::CodecError::InvalidUtf8)
            }),
            Err(crate::codec::CodecError::InvalidUtf8)
        );
        assert_eq!(list.len(), 1);
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
