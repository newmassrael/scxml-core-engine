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

use crate::codec::{CodecError, SceCursor};

/// A bounded list as the bytes its entries occupy.
///
/// `span` runs from the first entry to the end of the frame the list was read
/// from, not to the end of its last entry, so that each entry is decoded from a
/// cursor that holds what the scan's cursor held at that entry: a decode that
/// looks at how many bytes remain sees the same number in a walk as in the
/// scan. `count` says where the list stops.
///
/// `K` is the number of flag inputs an entry's decode takes, each a `u8`. `args`
/// are the values the walk starts with: what the parent threads to every
/// entry, and the initial value of what a chain carries from one entry to the
/// next. `decode` reads one entry given the current values and `advance`
/// updates the carried ones from the entry just read, both generated for the
/// list so that a walk rebuilds the values the scan saw.
pub struct ListView<'a, E, const K: usize> {
    span: &'a [u8],
    count: u32,
    args: [u8; K],
    decode: fn(&mut SceCursor<'a>, &[u8; K]) -> Result<E, CodecError>,
    advance: fn(&E, &mut [u8; K]),
}

impl<'a, E, const K: usize> ListView<'a, E, K> {
    /// A list of `count` entries read from `span`. The generated scan is the
    /// only caller: it has just decoded exactly these entries.
    pub const fn new(
        span: &'a [u8],
        count: u32,
        args: [u8; K],
        decode: fn(&mut SceCursor<'a>, &[u8; K]) -> Result<E, CodecError>,
        advance: fn(&E, &mut [u8; K]),
    ) -> Self {
        Self {
            span,
            count,
            args,
            decode,
            advance,
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
    pub fn iter(&self) -> ListIter<'a, E, K> {
        ListIter {
            cursor: SceCursor::new(self.span),
            left: self.count,
            args: self.args,
            decode: self.decode,
            advance: self.advance,
        }
    }

    /// The entry at `index`, found by walking from the start.
    pub fn get(&self, index: usize) -> Option<E> {
        self.iter().nth(index)
    }
}

fn no_entry<'a, E, const K: usize>(_: &mut SceCursor<'a>, _: &[u8; K]) -> Result<E, CodecError> {
    Err(CodecError::NeedMoreBytes)
}

fn no_advance<E, const K: usize>(_: &E, _: &mut [u8; K]) {}

impl<'a, E, const K: usize> Default for ListView<'a, E, K> {
    /// The empty list.
    fn default() -> Self {
        Self::new(&[], 0, [0; K], no_entry::<E, K>, no_advance::<E, K>)
    }
}

impl<'a, E, const K: usize> Clone for ListView<'a, E, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, E, const K: usize> Copy for ListView<'a, E, K> {}

impl<'a, E: fmt::Debug, const K: usize> fmt::Debug for ListView<'a, E, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// Two lists are equal when they hold equal entries in the same order,
/// whatever bytes each was read from.
impl<'a, E: PartialEq, const K: usize> PartialEq for ListView<'a, E, K> {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<'a, E, const K: usize> IntoIterator for &ListView<'a, E, K> {
    type Item = E;
    type IntoIter = ListIter<'a, E, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The walk of a [`ListView`].
pub struct ListIter<'a, E, const K: usize> {
    cursor: SceCursor<'a>,
    left: u32,
    args: [u8; K],
    decode: fn(&mut SceCursor<'a>, &[u8; K]) -> Result<E, CodecError>,
    advance: fn(&E, &mut [u8; K]),
}

impl<'a, E, const K: usize> Iterator for ListIter<'a, E, K> {
    type Item = E;

    fn next(&mut self) -> Option<E> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        // The scan decoded this entry from these bytes with these values, so
        // it decodes again; the arm is there because the type cannot say so,
        // and ends the walk rather than panicking.
        let entry = (self.decode)(&mut self.cursor, &self.args).ok()?;
        (self.advance)(&entry, &mut self.args);
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

    fn decode(c: &mut SceCursor<'static>, a: &[u8; 1]) -> Result<Entry, CodecError> {
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

    static BYTES: [u8; 5] = [1, 9, 2, 3, 7];

    fn list() -> ListView<'static, Entry, 1> {
        ListView::new(&BYTES, 3, [0], decode, advance)
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
        let l: ListView<'static, Entry, 1> = ListView::default();
        assert!(l.is_empty());
        assert_eq!(l.iter().count(), 0);
        assert_eq!(l.get(0), None);
    }

    #[test]
    fn lists_are_equal_by_their_entries() {
        let other = ListView::new(&BYTES, 3, [0], decode, advance);
        assert_eq!(list(), other);
        let shorter = ListView::new(&BYTES, 2, [0], decode, advance);
        assert_ne!(list(), shorter);
    }

    #[test]
    fn a_copy_walks_as_the_original_does() {
        let l = list();
        let c = l;
        assert_eq!(l.iter().count(), c.iter().count());
    }

    #[test]
    fn the_size_of_a_list_does_not_depend_on_what_it_holds() {
        // The whole point: a span, a count, the carried values, two function
        // pointers. No entry storage, whatever the entry type.
        assert_eq!(
            core::mem::size_of::<ListView<'static, [u8; 512], 1>>(),
            core::mem::size_of::<ListView<'static, u8, 1>>()
        );
    }
}
