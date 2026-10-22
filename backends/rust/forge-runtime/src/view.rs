// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The list of a generated codec view (ADR 0049).
//!
//! A decoded `{Codec}<'a>` holds a bounded list as a `heapless::Vec<E, N>`: a
//! length and an uninitialised array of `N` entries, so the record is as large
//! as its list can be whatever the wire held, and moving it copies all of it.
//! A [`ListView`] holds the same list as the bytes its entries occupy, how many
//! there are, and what a walk needs to read them again, so its size depends on
//! neither `N` nor the entry. The generated scan that builds one decodes every
//! entry once and refuses what the eager decode refuses (`TooManyElements`, a
//! chain cut short or too long), so a walk afterwards cannot fail.
//!
//! The walk is the work `decode` did once, done again per walk, and `get` walks
//! from the start: that is the price of a record that does not hold its entries.

use core::fmt;
use core::marker::PhantomData;

use crate::codec::{CodecError, SceCursor};

/// How the entries of one list are read: what a walk decodes each entry with, and
/// how it updates the values it carries from one entry to the next.
///
/// Generated, one marker type for each list of each codec, so that a list names
/// them by type and holds no function. The first form held them as function
/// pointers over the lifetime of the bytes (`fn(&mut SceCursor<'a>, ..)`), which
/// made the list invariant in that lifetime: a view decoded from a buffer that
/// lives long could not be lent where a shorter lifetime is asked for, the
/// one thing a borrowed view exists to allow. An entry is [`Self::Entry`]
/// of whatever lifetime the cursor it is decoded from has, which is what a
/// generic associated type says and a function pointer's return type cannot.
///
/// `K` is the number of flag inputs an entry's decode takes, each a `u8`.
pub trait ListWalk<const K: usize> {
    /// An entry, borrowing from the bytes of the cursor it was read from.
    type Entry<'b>;

    /// Read one entry from `cursor` given the current values `args`: what the
    /// parent threads to every entry, and what a chain carried from the entry
    /// before.
    fn decode<'b>(
        cursor: &mut SceCursor<'b>,
        args: &[u8; K],
    ) -> Result<Self::Entry<'b>, CodecError>;

    /// Update the carried values from the entry just read, so that a walk
    /// rebuilds the values the scan saw.
    fn advance(entry: &Self::Entry<'_>, args: &mut [u8; K]);
}

/// A bounded list as the bytes its entries occupy.
///
/// `span` runs from the first entry to the end of the frame the list was read
/// from, not to the end of its last entry, so that each entry is decoded from a
/// cursor that holds what the scan's cursor held at that entry: a decode that
/// looks at how many bytes remain sees the same number in a walk as in the
/// scan. `count` says where the list stops.
///
/// `args` are the values the walk starts with: what the parent threads to every
/// entry, and the initial value of what a chain carries from one entry to the
/// next. `W` says how an entry is read ([`ListWalk`]); it is a type and not a
/// value, so a list is a span, a count and the carried values, and is covariant
/// in `'a`.
pub struct ListView<'a, W: ListWalk<K>, const K: usize> {
    span: &'a [u8],
    count: u32,
    args: [u8; K],
    walk: PhantomData<fn() -> W>,
}

impl<'a, W: ListWalk<K>, const K: usize> ListView<'a, W, K> {
    /// A list of `count` entries read from `span`. The generated scan is the
    /// only caller: it has just decoded exactly these entries.
    pub const fn new(span: &'a [u8], count: u32, args: [u8; K]) -> Self {
        Self {
            span,
            count,
            args,
            walk: PhantomData,
        }
    }

    /// How many entries the list holds.
    pub const fn len(&self) -> usize {
        self.count as usize
    }

    /// Whether the list holds no entry.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The entries in order, each decoded as it is reached.
    pub fn iter(&self) -> ListIter<'a, W, K> {
        ListIter {
            cursor: SceCursor::new(self.span),
            left: self.count,
            args: self.args,
            walk: PhantomData,
        }
    }

    /// The entry at `index`, found by walking from the start.
    pub fn get(&self, index: usize) -> Option<W::Entry<'a>> {
        self.iter().nth(index)
    }
}

impl<'a, W: ListWalk<K>, const K: usize> Default for ListView<'a, W, K> {
    /// The empty list.
    fn default() -> Self {
        Self::new(&[], 0, [0; K])
    }
}

impl<'a, W: ListWalk<K>, const K: usize> Clone for ListView<'a, W, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, W: ListWalk<K>, const K: usize> Copy for ListView<'a, W, K> {}

impl<'a, W: ListWalk<K>, const K: usize> fmt::Debug for ListView<'a, W, K>
where
    W::Entry<'a>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// Two lists are equal when they hold equal entries in the same order,
/// whatever bytes each was read from.
impl<'a, W: ListWalk<K>, const K: usize> PartialEq for ListView<'a, W, K>
where
    W::Entry<'a>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<'a, W: ListWalk<K>, const K: usize> IntoIterator for &ListView<'a, W, K> {
    type Item = W::Entry<'a>;
    type IntoIter = ListIter<'a, W, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The walk of a [`ListView`].
pub struct ListIter<'a, W: ListWalk<K>, const K: usize> {
    cursor: SceCursor<'a>,
    left: u32,
    args: [u8; K],
    walk: PhantomData<fn() -> W>,
}

impl<'a, W: ListWalk<K>, const K: usize> Iterator for ListIter<'a, W, K> {
    type Item = W::Entry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        // The scan decoded this entry from these bytes with these values, so
        // it decodes again; the arm is there because the type cannot say so,
        // and ends the walk rather than panicking.
        let entry = W::decode(&mut self.cursor, &self.args).ok()?;
        W::advance(&entry, &mut self.args);
        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.left as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // An entry is one byte, plus one more when the byte is odd; the previous
    // entry's evenness is the one value a walk carries.
    #[derive(Debug, PartialEq)]
    struct Entry {
        head: u8,
        extra: Option<u8>,
        after_odd: u8,
    }

    struct Walk;

    impl ListWalk<1> for Walk {
        type Entry<'b> = Entry;

        fn decode<'b>(c: &mut SceCursor<'b>, a: &[u8; 1]) -> Result<Entry, CodecError> {
            let head = c.peek_slice(1)?[0];
            c.advance(1)?;
            let extra = if head & 1 == 1 {
                let e = c.peek_slice(1)?[0];
                c.advance(1)?;
                Some(e)
            } else {
                None
            };
            Ok(Entry {
                head,
                extra,
                after_odd: a[0],
            })
        }

        fn advance(e: &Entry, a: &mut [u8; 1]) {
            a[0] = u8::from(e.head & 1 == 1);
        }
    }

    static BYTES: [u8; 5] = [1, 9, 2, 3, 7];

    fn list() -> ListView<'static, Walk, 1> {
        ListView::new(&BYTES, 3, [0])
    }

    // An entry that holds a slice of the bytes it was read from, as a view of a
    // codec with a bytes field does: the lifetime of the entry is the cursor's.
    struct BorrowedWalk;

    impl ListWalk<0> for BorrowedWalk {
        type Entry<'b> = &'b [u8];

        fn decode<'b>(c: &mut SceCursor<'b>, _: &[u8; 0]) -> Result<&'b [u8], CodecError> {
            let one = c.peek_slice(1)?;
            c.advance(1)?;
            Ok(one)
        }

        fn advance(_: &&[u8], _: &mut [u8; 0]) {}
    }

    #[test]
    fn a_walk_reads_the_entries_in_order_with_the_carried_value() {
        let entries: [Option<Entry>; 4] = {
            let mut it = list().iter();
            [it.next(), it.next(), it.next(), it.next()]
        };
        assert_eq!(
            entries[0],
            Some(Entry {
                head: 1,
                extra: Some(9),
                after_odd: 0
            })
        );
        assert_eq!(
            entries[1],
            Some(Entry {
                head: 2,
                extra: None,
                after_odd: 1
            })
        );
        assert_eq!(
            entries[2],
            Some(Entry {
                head: 3,
                extra: Some(7),
                after_odd: 0
            })
        );
        assert_eq!(entries[3], None);
    }

    #[test]
    fn a_list_knows_its_length_and_walks_again_from_the_start() {
        let l = list();
        assert_eq!(l.len(), 3);
        assert!(!l.is_empty());
        assert_eq!(l.iter().count(), 3);
        assert_eq!(l.iter().count(), 3);
        assert_eq!(
            l.get(2),
            Some(Entry {
                head: 3,
                extra: Some(7),
                after_odd: 0
            })
        );
        assert_eq!(l.get(3), None);
    }

    #[test]
    fn the_default_list_is_empty() {
        let l: ListView<'static, Walk, 1> = ListView::default();
        assert!(l.is_empty());
        assert_eq!(l.iter().count(), 0);
        assert_eq!(l.get(0), None);
    }

    #[test]
    fn lists_are_equal_by_their_entries() {
        let other: ListView<'static, Walk, 1> = ListView::new(&BYTES, 3, [0]);
        assert_eq!(list(), other);
        let shorter: ListView<'static, Walk, 1> = ListView::new(&BYTES, 2, [0]);
        assert_ne!(list(), shorter);
    }

    #[test]
    fn a_copy_walks_as_the_original_does() {
        let l = list();
        let c = l;
        assert_eq!(l.iter().count(), c.iter().count());
    }

    // An entry of a different size, to show that no entry is held.
    struct WideWalk;

    impl ListWalk<1> for WideWalk {
        type Entry<'b> = [u8; 512];

        fn decode<'b>(_: &mut SceCursor<'b>, _: &[u8; 1]) -> Result<[u8; 512], CodecError> {
            Ok([0; 512])
        }

        fn advance(_: &[u8; 512], _: &mut [u8; 1]) {}
    }

    #[test]
    fn the_size_of_a_list_does_not_depend_on_what_it_holds() {
        // The whole point: a span, a count and the carried values. No entry
        // storage, whatever the entry type, and no function pointer: how an entry
        // is read is a type.
        assert_eq!(
            core::mem::size_of::<ListView<'static, WideWalk, 1>>(),
            core::mem::size_of::<ListView<'static, Walk, 1>>()
        );
        assert!(
            core::mem::size_of::<ListView<'static, Walk, 1>>()
                <= core::mem::size_of::<&[u8]>() + 2 * core::mem::size_of::<u32>()
        );
    }

    /// A list is lent for less than it lives: the regression of the defect the
    /// function pointers had, which made `ListView` invariant in `'a` and this
    /// function not compile. It is a compile-time test; calling it is not the point.
    fn lend<'long: 'short, 'short>(
        list: ListView<'long, BorrowedWalk, 0>,
    ) -> ListView<'short, BorrowedWalk, 0> {
        list
    }

    #[test]
    fn a_list_read_from_a_long_lived_buffer_is_lent_for_a_shorter_time() {
        static LONG: [u8; 3] = [10, 20, 30];
        let long: ListView<'static, BorrowedWalk, 0> = ListView::new(&LONG, 3, []);
        // A shorter lifetime than the buffer's, as a caller that keeps the view
        // in a struct of its own, or hands two views of different lifetimes to
        // one function, asks for.
        let local = [1u8, 2, 3];
        let short: ListView<'_, BorrowedWalk, 0> = ListView::new(&local, 3, []);
        fn same_lifetime<'x>(
            a: ListView<'x, BorrowedWalk, 0>,
            b: ListView<'x, BorrowedWalk, 0>,
        ) -> usize {
            a.iter().count() + b.iter().count()
        }
        assert_eq!(same_lifetime(lend(long), short), 6);
        // The entries it yields borrow from the buffer the list was read from.
        assert_eq!(long.get(1), Some(&LONG[1..2]));
    }
}
