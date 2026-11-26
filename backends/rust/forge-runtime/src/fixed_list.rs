// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The fixed-capacity list a profile holds a codec's list in
//! (`heapless::Vec<T, N>`): its push and its removal are each one body for every
//! entry type and capacity, and its [`SceList`] impl is the casts around them
//! (docs/adr/0141).
//!
//! A generic container compiles each of its operations once for every pair of
//! entry type and capacity a program holds a list of. The two whose body is more
//! than a field access, the push of an entry and the removal of one, are here one
//! function each, generic over nothing and out of line, over the list as its raw
//! parts: the address of its first slot, its length, its capacity and the size of
//! an entry. They move an entry as its bytes, which is all a move of a Rust value
//! is. What stays typed is the impl: it takes the list apart, hands the entry over
//! as bytes it no longer drops, sets the length the body answers for, and drops
//! what the list gives up. It is the split `hashbrown` makes between
//! `RawTableInner`, one body for every key and value type, and `RawTable<T>`.
//!
//! # Why the bodies are sound for every `T`
//!
//! - A move is a copy of the value's bytes after which the source is not used.
//!   The push copies the entry's `size_of::<T>()` bytes into the slot at the
//!   length, and the impl holds the entry in `ManuallyDrop`, so the list is its
//!   one owner; where the list is full nothing is copied, and the impl drops the
//!   entry, once.
//! - The removal copies the entry out into a `MaybeUninit<T>` the impl owns and
//!   moves the entries after it down one slot; the impl sets the length before it
//!   drops the entry, so a `Drop` that panics leaves a list that counts exactly
//!   what it holds. An index past the end moves nothing and drops nothing.
//! - The slot at `index` is `index * size` bytes from the first, inside the
//!   buffer of `capacity * size` bytes, which a type's size keeps at most
//!   `isize::MAX`. A type's size is a multiple of its alignment, so every slot is
//!   aligned as the first is, for an entry of any alignment; the copies are of
//!   bytes and ask none of their own.
//! - A zero-sized entry is a copy of no bytes, and the length counts it as any
//!   other.
//! - `ptr::copy` and `ptr::copy_nonoverlapping` are untyped: padding,
//!   uninitialised bytes and the provenance of a reference an entry holds move
//!   with the entry.
//!
//! The unit tests below are run under Miri by `scripts/gates/forge-rust.sh`.

use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::ptr;

use crate::codec::CodecError;
use crate::storage::{HeldList, SceList};

/// Copy the entry of `size` bytes at `entry` into the slot at `len` of the
/// list whose first slot is at `slots`, of `capacity` slots of `size` bytes,
/// and answer whether it did. Where the list is full it copies nothing and
/// answers `false`.
///
/// On `true` the list holds the entry: the caller sets the length to `len + 1`
/// and does not drop the entry. On `false` the entry is still the caller's.
///
/// # Safety
///
/// - `slots` is the first of `capacity` slots of `size` bytes, valid for writes
///   of `capacity * size` bytes, which are at most `isize::MAX`;
/// - `len` is at most `capacity`;
/// - `entry` is valid for reads of `size` bytes, none of them in those slots.
#[inline(never)]
unsafe fn push(slots: *mut u8, len: usize, capacity: usize, size: usize, entry: *const u8) -> bool {
    if len >= capacity {
        return false;
    }
    // SAFETY: `len` is below `capacity`, so the slot at `len * size` and its
    // `size` bytes lie inside the `capacity * size` writable bytes the caller
    // gives, and the product does not overflow, being below that size. The entry
    // is `size` readable bytes outside them, so the two do not overlap.
    unsafe {
        ptr::copy_nonoverlapping(entry, slots.add(len * size), size);
    }
    true
}

/// Move the entry at `at` of the list whose first slot is at `slots`, `len`
/// slots of `size` bytes, out into `out`, and each entry after it down one
/// slot, closing the gap.
///
/// The caller sets the length to `len - 1` and owns the entry at `out`, and
/// does so only where this answers `true`.
///
/// Where `at` is not below `len` it moves nothing and answers `false`: the
/// removal is total, with no panic path in a bare-metal image (the runtime's
/// rule since the panic-free reads), and the list is as it was.
///
/// # Safety
///
/// - `slots` is the first of at least `len` slots of `size` bytes, the first
///   `len` of which each hold an entry, valid for reads and writes of
///   `len * size` bytes;
/// - `out` is valid for writes of `size` bytes, none of them in those slots.
#[inline(never)]
unsafe fn remove(slots: *mut u8, len: usize, size: usize, at: usize, out: *mut u8) -> bool {
    if at >= len {
        return false;
    }
    // SAFETY: `at` is below `len`, so the slot at `at * size` and its `size`
    // bytes lie inside the `len * size` bytes the caller gives, and `out` is
    // `size` writable bytes outside them. The entries after it start one slot
    // further, at most at `len * size` (the end of those bytes, which `add`
    // may reach), and are `len - at - 1` slots, so they end where the held
    // slots do; they and the slot they move into overlap, which `ptr::copy`
    // allows.
    unsafe {
        let slot = slots.add(at * size);
        ptr::copy_nonoverlapping(slot, out, size);
        ptr::copy(slot.add(size), slot, (len - at - 1) * size);
    }
    true
}

impl<T, const N: usize> SceList<T> for heapless::Vec<T, N> {
    fn empty() -> Self {
        heapless::Vec::new()
    }

    #[inline]
    fn try_push(&mut self, value: T) -> Result<(), CodecError> {
        let entry = ManuallyDrop::new(value);
        let len = self.len();
        // SAFETY: `as_mut_ptr` is the first of the vector's `N` slots of
        // `size_of::<T>()` bytes, which the vector owns and `&mut self` lends
        // whole: a buffer of a type the program holds, so at most `isize::MAX`
        // bytes. The vector's length is at most `N`. `entry` is a local of this
        // frame, `size_of::<T>()` bytes (`ManuallyDrop` is transparent), so
        // not in the vector.
        let pushed = unsafe {
            push(
                self.as_mut_ptr().cast::<u8>(),
                len,
                N,
                mem::size_of::<T>(),
                ptr::from_ref(&entry).cast::<u8>(),
            )
        };
        if !pushed {
            // Nothing was copied: the entry is still this function's, and is
            // dropped here, once.
            drop(ManuallyDrop::into_inner(entry));
            return Err(CodecError::CapacityExceeded);
        }
        // SAFETY: the slot at `len` now holds the entry's bytes, so each of the
        // first `len + 1` slots holds an entry, and `len + 1` is at most `N`.
        // `entry` is not dropped (`ManuallyDrop`), so the vector is the one
        // owner of what it now holds.
        unsafe { self.set_len(len + 1) };
        Ok(())
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }

    fn last_mut(&mut self) -> Option<&mut T> {
        self.as_mut_slice().last_mut()
    }

    #[inline]
    fn remove_at(&mut self, at: usize) {
        let len = self.len();
        let mut taken = MaybeUninit::<T>::uninit();
        // SAFETY: `as_mut_ptr` is the first of the vector's `N` slots of
        // `size_of::<T>()` bytes, at least `len` of them, the first `len` each
        // holding an entry, lent whole by `&mut self`. `taken` is
        // `size_of::<T>()` writable bytes of this frame, not in the vector.
        // `remove` moves nothing where `at` is not below `len`, and the
        // vector is then as it was.
        let removed = unsafe {
            remove(
                self.as_mut_ptr().cast::<u8>(),
                len,
                mem::size_of::<T>(),
                at,
                taken.as_mut_ptr().cast::<u8>(),
            )
        };
        if !removed {
            return;
        }
        // SAFETY: the entry at `at` was moved out and each one after it down one
        // slot, so each of the first `len - 1` slots holds an entry, none of them
        // twice; `len` is at least one, `at` being below it.
        unsafe { self.set_len(len - 1) };
        // SAFETY: `taken` holds the bytes of the entry the vector held at `at`,
        // which it no longer counts, so it is dropped once, here, after the
        // vector is whole again.
        unsafe { taken.assume_init_drop() };
    }

    fn is_full(&self) -> bool {
        heapless::Vec::is_full(self)
    }
}

impl<T, const N: usize> HeldList for heapless::Vec<T, N> {
    type Entry = T;
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use core::{mem, ptr};

    use crate::codec::{push_keeping, CodecError, EntryKinds, NoKinds};
    use crate::storage::SceList;

    /// An entry that counts its drops in a cell the test owns, and says its
    /// place and its kind.
    struct Counted<'d> {
        id: u8,
        kind: u8,
        drops: &'d Cell<usize>,
    }

    impl Drop for Counted<'_> {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    fn counted(id: u8, kind: u8, drops: &Cell<usize>) -> Counted<'_> {
        Counted { id, kind, drops }
    }

    /// The places of the entries a list holds, in order.
    fn ids<const N: usize>(list: &heapless::Vec<Counted<'_>, N>) -> heapless::Vec<u8, N> {
        list.iter().map(|entry| entry.id).collect()
    }

    /// The kinds the gates on a list of `Counted` ask for: `KINDS` of them,
    /// an entry being of the one its `kind` says.
    struct ByKind<const KINDS: usize>;

    impl<'d, const KINDS: usize> EntryKinds<Counted<'d>> for ByKind<KINDS> {
        const COUNT: usize = KINDS;

        fn is(entry: &Counted<'d>, kind: usize) -> bool {
            usize::from(entry.kind) == kind
        }
    }

    #[test]
    fn an_entry_is_held_in_the_slot_at_the_length_until_the_list_is_full() {
        let mut list: heapless::Vec<u32, 3> = heapless::Vec::new();
        for value in [10, 20, 30] {
            assert_eq!(SceList::try_push(&mut list, value), Ok(()));
        }
        assert!(SceList::is_full(&list));
        assert_eq!(
            SceList::try_push(&mut list, 40),
            Err(CodecError::CapacityExceeded)
        );
        assert_eq!(SceList::as_slice(&list), &[10, 20, 30]);
    }

    #[test]
    fn an_entry_refused_is_dropped_once_and_one_held_with_its_list() {
        let drops = Cell::new(0);
        let mut list: heapless::Vec<Counted<'_>, 2> = heapless::Vec::new();
        assert_eq!(SceList::try_push(&mut list, counted(0, 0, &drops)), Ok(()));
        assert_eq!(SceList::try_push(&mut list, counted(1, 0, &drops)), Ok(()));
        assert_eq!(drops.get(), 0, "a held entry is not dropped by its push");
        assert_eq!(
            SceList::try_push(&mut list, counted(2, 0, &drops)),
            Err(CodecError::CapacityExceeded)
        );
        assert_eq!(drops.get(), 1, "the refused entry is dropped, once");
        assert_eq!(ids(&list), [0, 1]);
        drop(list);
        assert_eq!(drops.get(), 3, "the list drops what it holds, once each");
    }

    #[test]
    fn an_entry_removed_is_dropped_once_and_the_rest_close_the_gap() {
        let drops = Cell::new(0);
        let mut list: heapless::Vec<Counted<'_>, 4> = heapless::Vec::new();
        for id in 0..4 {
            assert_eq!(SceList::try_push(&mut list, counted(id, 0, &drops)), Ok(()));
        }
        SceList::remove_at(&mut list, 1);
        assert_eq!(drops.get(), 1);
        assert_eq!(ids(&list), [0, 2, 3]);
        // The last entry: nothing after it moves.
        SceList::remove_at(&mut list, 2);
        assert_eq!(drops.get(), 2);
        assert_eq!(ids(&list), [0, 2]);
        // The first: every entry after it moves.
        SceList::remove_at(&mut list, 0);
        assert_eq!(drops.get(), 3);
        assert_eq!(ids(&list), [2]);
        // A slot a removal freed holds the next push.
        assert_eq!(SceList::try_push(&mut list, counted(9, 0, &drops)), Ok(()));
        assert_eq!(ids(&list), [2, 9]);
        drop(list);
        assert_eq!(drops.get(), 5, "every entry is dropped once");
    }

    /// A removal past the end moves nothing and drops nothing: the list is as
    /// it was, and the runtime keeps no panic path for it.
    #[test]
    fn a_removal_past_the_end_removes_nothing() {
        let drops = Cell::new(0);
        let mut list: heapless::Vec<Counted<'_>, 4> = heapless::Vec::new();
        assert_eq!(SceList::try_push(&mut list, counted(0, 0, &drops)), Ok(()));
        SceList::remove_at(&mut list, 1);
        SceList::remove_at(&mut list, usize::MAX);
        assert_eq!(ids(&list), [0]);
        assert_eq!(drops.get(), 0);
        drop(list);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn a_full_list_with_no_kind_asked_holds_what_it_holds_and_reads_on() {
        let drops = Cell::new(0);
        let mut list: heapless::Vec<Counted<'_>, 2> = heapless::Vec::new();
        let mut full = false;
        for id in 0..3 {
            push_keeping::<_, NoKinds>(&mut list, counted(id, 0, &drops), &mut full);
        }
        assert!(full, "the entry past the capacity is noted");
        assert_eq!(ids(&list), [0, 1]);
        assert_eq!(
            drops.get(),
            1,
            "the entry past the capacity is dropped, once"
        );
        drop(list);
        assert_eq!(drops.get(), 3);
    }

    #[test]
    fn an_entry_of_a_new_kind_past_the_capacity_takes_the_place_of_a_spare_one() {
        let drops = Cell::new(0);
        let mut list: heapless::Vec<Counted<'_>, 2> = heapless::Vec::new();
        let mut full = false;
        push_keeping::<_, ByKind<2>>(&mut list, counted(0, 0, &drops), &mut full);
        push_keeping::<_, ByKind<2>>(&mut list, counted(1, 0, &drops), &mut full);
        assert!(!full);
        // Of a kind the list holds none of: it takes the place of the last
        // entry whose kind another entry also holds, which is dropped once.
        push_keeping::<_, ByKind<2>>(&mut list, counted(2, 1, &drops), &mut full);
        assert!(full);
        assert_eq!(ids(&list), [0, 2]);
        assert_eq!(drops.get(), 1, "the entry given up is dropped, once");
        // Of a kind the list holds: it is read and not held.
        push_keeping::<_, ByKind<2>>(&mut list, counted(3, 0, &drops), &mut full);
        assert_eq!(ids(&list), [0, 2]);
        assert_eq!(drops.get(), 2, "the entry not held is dropped, once");
        drop(list);
        assert_eq!(drops.get(), 4);
    }

    #[test]
    fn a_full_list_whose_every_entry_is_the_one_of_its_kind_keeps_them_all() {
        let drops = Cell::new(0);
        let mut list: heapless::Vec<Counted<'_>, 2> = heapless::Vec::new();
        let mut full = false;
        push_keeping::<_, ByKind<3>>(&mut list, counted(0, 0, &drops), &mut full);
        push_keeping::<_, ByKind<3>>(&mut list, counted(1, 1, &drops), &mut full);
        // A new kind, and no entry another one stands in for: none is given up.
        push_keeping::<_, ByKind<3>>(&mut list, counted(2, 2, &drops), &mut full);
        assert!(full);
        assert_eq!(ids(&list), [0, 1]);
        assert_eq!(
            drops.get(),
            1,
            "only the entry past the capacity is dropped"
        );
        drop(list);
        assert_eq!(drops.get(), 3);
    }

    /// A zero-sized entry with drop glue, counted in a static only
    /// `zero_sized_entries_are_counted_pushed_removed_and_dropped` touches.
    struct Mark;

    static MARK_DROPS: AtomicUsize = AtomicUsize::new(0);

    impl Drop for Mark {
        fn drop(&mut self) {
            MARK_DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn zero_sized_entries_are_counted_pushed_removed_and_dropped() {
        assert_eq!(mem::size_of::<Mark>(), 0);
        let mut list: heapless::Vec<Mark, 3> = heapless::Vec::new();
        for _ in 0..3 {
            assert_eq!(SceList::try_push(&mut list, Mark), Ok(()));
        }
        assert_eq!(
            SceList::try_push(&mut list, Mark),
            Err(CodecError::CapacityExceeded)
        );
        assert_eq!(MARK_DROPS.load(Ordering::Relaxed), 1);
        SceList::remove_at(&mut list, 1);
        assert_eq!(MARK_DROPS.load(Ordering::Relaxed), 2);
        assert_eq!(SceList::as_slice(&list).len(), 2);
        drop(list);
        assert_eq!(MARK_DROPS.load(Ordering::Relaxed), 4);

        // And with no drop glue at all.
        let mut units: heapless::Vec<(), 2> = heapless::Vec::new();
        assert_eq!(SceList::try_push(&mut units, ()), Ok(()));
        assert_eq!(SceList::try_push(&mut units, ()), Ok(()));
        assert_eq!(
            SceList::try_push(&mut units, ()),
            Err(CodecError::CapacityExceeded)
        );
        SceList::remove_at(&mut units, 0);
        assert_eq!(SceList::as_slice(&units).len(), 1);
    }

    /// An entry aligned past its size's own need.
    #[derive(Debug, PartialEq)]
    #[repr(align(64))]
    struct Wide(u8);

    #[test]
    fn an_over_aligned_entry_is_held_aligned_in_every_slot() {
        let mut list: heapless::Vec<Wide, 3> = heapless::Vec::new();
        for value in 1..=3 {
            assert_eq!(SceList::try_push(&mut list, Wide(value)), Ok(()));
        }
        SceList::remove_at(&mut list, 0);
        assert_eq!(SceList::as_slice(&list), &[Wide(2), Wide(3)]);
        for entry in SceList::as_slice(&list) {
            assert!(ptr::from_ref(entry).is_aligned());
        }
    }

    #[test]
    fn an_entry_that_borrows_keeps_its_reference_and_its_padding_moves_with_it() {
        let frame = [1u8, 2, 3, 4, 5, 6];
        // A slice and a short integer: a reference whose provenance the copy
        // keeps, and padding bytes it copies uninitialised.
        let mut list: heapless::Vec<(&[u8], u16), 3> = heapless::Vec::new();
        assert_eq!(SceList::try_push(&mut list, (&frame[..2], 7)), Ok(()));
        assert_eq!(SceList::try_push(&mut list, (&frame[2..4], 8)), Ok(()));
        assert_eq!(SceList::try_push(&mut list, (&frame[4..], 9)), Ok(()));
        SceList::remove_at(&mut list, 0);
        let held = SceList::as_slice(&list);
        assert_eq!(held, &[(&frame[2..4], 8), (&frame[4..], 9)]);
        assert_eq!(
            held[0].0.as_ptr(),
            frame[2..].as_ptr(),
            "the frame's memory, not a copy"
        );
    }
}
