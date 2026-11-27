// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The queue of a bounded inbox (`BoundedInbox`): many producers, drained by
//! the session's owner, whose take never waits for a post (docs/adr/0147, D1
//! and D5).
//!
//! **Two rings make the queue**, as in the data queue of Nikolaev's SCQ, which
//! the C11 inbox stands on. The inputs live in an array of `N` slots. The
//! *free* ring holds the indices of the slots nobody uses and starts with all
//! `N`; the *filled* ring holds the indices of the slots that hold an input, in
//! the order the posts linearized, and starts empty. A post takes an index off
//! the free ring (none: the inbox is full), writes its input into that slot,
//! which is its own until it lets it go, and puts the index on the filled ring.
//! A take does the reverse: it takes an index off the filled ring, reads the
//! input out and puts the index back on the free ring.
//!
//! **An index ring claims and fills a cell in one step.** Each cell is one
//! atomic word: a stamp and the index the cell carries. For the position `p` a
//! cell stands at, the stamp is `p` while the cell is empty for the put at `p`,
//! `p + 1` once that put has filled it, and `p + N` once a take has taken it,
//! which empties it for the put at `p + N` (the sequence of Vyukov's cells). A
//! put compare-and-swaps the empty cell at the tail to filled with its index, and
//! a take compare-and-swaps the filled cell at the head to taken; then either
//! moves its end past the cell, and a context that finds the cell at an end
//! already done with moves the end for it (Tsigas and Zhang's queue). So a cell
//! is never reserved and left unwritten: the tail passes a cell only once it is
//! filled, and a put fills only the cell at the tail, so every position before
//! the tail is filled, and a take that finds the cell at the head empty finds
//! the ring empty.
//!
//! **Why the take never waits.** A post that is stopped, by an interrupt or by
//! the owner itself on one core, holds a slot and no place in the order: its
//! input has no position in the filled ring until the step that publishes it.
//! The owner's take is then a few loads and stores on the filled ring and the
//! free ring, whatever any poster is doing; a long-stopped post costs its one
//! slot, and the other `N - 1` go round as before.
//!
//! **The promise (docs/formal/InboxService.tla).** A post linearizes where it
//! puts its index on the filled ring, or, refused, where it finds the free ring
//! empty; a take where it takes an index off the filled ring, or, answering
//! that there is none, where it reads the empty cell at the head.
//!
//! - A poster's own posts keep its order: it begins its next only once its
//!   previous one has returned, so the next one's index is put at a later
//!   position, and the positions are taken in order.
//! - A post that returned before the drain before an event of the owner's own
//!   began is taken by that drain: it is filled at a position before the tail,
//!   every position before the tail is filled, and the drain takes until it
//!   reads an empty cell. A post still under way when the drain reads the
//!   inbox overlaps the own event, and may linearize after it.
//! - At most once: one compare-and-swap takes an index off the filled ring, and
//!   the slot goes back to the free ring only after its input is read out.
//! - Never lost: an index on the filled ring stays there until a take takes it.
//! - Full: a post is refused only when every slot holds an input the filled ring
//!   holds, the input a take is reading out, or the input of a post still under
//!   way, which the promise counts with what the session holds (docs/adr/0147,
//!   D1). No post is refused while a slot is free.
//!
//! **Ordering.** A cell is loaded with acquire and swapped with
//! acquire-release, and the ends move with relaxed swaps, which only say where
//! to look: the cells say what is there. An input's write into its slot
//! happens-before the take that reads it, through the release of the swap that
//! fills the index's cell and the acquire of the load the take reads it with;
//! the read happens-before the slot's next write, through the same pair on the
//! free ring.
//!
//! **Positions are 24 bits.** A cell's word is 32 bits, the index in the low 8,
//! and the stamps and positions count modulo 2^24. A context that read a cell
//! and is stopped before its compare-and-swap is misled only if, meanwhile,
//! the ring's positions run through all 2^24 values and bring the same word
//! back: at a million posts a second, sixteen seconds of one stop between two
//! instructions. 32-bit atomics are what every target with compare-and-swap of
//! a byte has.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, Ordering};

/// The low bits of a cell's word, which hold the index the cell carries; the
/// rest hold its stamp.
const INDEX_BITS: u32 = 8;

/// The positions and the stamps count modulo 2^24.
const POSITION_MASK: u32 = (1 << (32 - INDEX_BITS)) - 1;

/// The position `by` places after `position`.
const fn ahead(position: u32, by: u32) -> u32 {
    position.wrapping_add(by) & POSITION_MASK
}

/// The position `by` places before `position`.
const fn behind(position: u32, by: u32) -> u32 {
    position.wrapping_sub(by) & POSITION_MASK
}

/// The word of a cell stamped `stamp` that carries `index`.
const fn word(stamp: u32, index: u32) -> u32 {
    (stamp << INDEX_BITS) | index
}

/// The stamp of a cell's word.
fn stamp_of(word: u32) -> u32 {
    word >> INDEX_BITS
}

/// The index a cell's word carries.
fn index_of(word: u32) -> u8 {
    (word & ((1 << INDEX_BITS) - 1)) as u8
}

/// A ring of `N` cells that carries slot indices, any context's at either
/// end.
struct IndexRing<const N: usize> {
    cells: [AtomicU32; N],
    /// The position of the next take, or one a take has done with and not yet
    /// moved past.
    head: AtomicU32,
    /// The position of the next put, or one a put has filled and not yet moved
    /// past.
    tail: AtomicU32,
}

impl<const N: usize> IndexRing<N> {
    const LEN: u32 = N as u32;
    const MASK: u32 = Self::LEN - 1;

    /// The value an array of cells is first filled with, only for that: an
    /// array of a type that is not Copy is repeated from a constant before Rust
    /// 1.79, the workspace's floor is 1.75, and every cell is set after.
    #[allow(clippy::declare_interior_mutable_const)]
    const UNSET: AtomicU32 = AtomicU32::new(0);

    /// A ring whose positions start at `start`, holding the indices from 0 to
    /// `held - 1` at its first `held` positions.
    const fn starting_at(start: u32, held: u32) -> Self {
        let mut cells = [Self::UNSET; N];
        let mut k = 0;
        while k < Self::LEN {
            let position = ahead(start, k);
            let cell = if k < held {
                word(ahead(position, 1), k)
            } else {
                word(position, 0)
            };
            cells[(position & Self::MASK) as usize] = AtomicU32::new(cell);
            k += 1;
        }
        Self {
            cells,
            head: AtomicU32::new(start),
            tail: AtomicU32::new(ahead(start, held)),
        }
    }

    fn cell(&self, position: u32) -> &AtomicU32 {
        &self.cells[(position & Self::MASK) as usize]
    }

    /// Move `end` past `position`, whose cell is done with, unless another
    /// context has.
    fn pass(end: &AtomicU32, position: u32) {
        let _ = end.compare_exchange(
            position,
            ahead(position, 1),
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
    }

    /// Put `index` at the back; false when the ring holds `N` indices.
    fn put(&self, index: u8) -> bool {
        loop {
            let position = self.tail.load(Ordering::Relaxed);
            let cell = self.cell(position);
            let seen = cell.load(Ordering::Acquire);
            let stamp = stamp_of(seen);
            if stamp == position {
                // Empty for the put at the tail: claimed and filled in one step.
                let filled = word(ahead(position, 1), u32::from(index));
                if cell
                    .compare_exchange(seen, filled, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    Self::pass(&self.tail, position);
                    return true;
                }
            } else if stamp == ahead(position, 1) {
                // Filled by a put stopped before it moved the tail past it.
                Self::pass(&self.tail, position);
            } else if stamp == behind(ahead(position, 1), Self::LEN) {
                // Still filled from the lap before: the ring holds N.
                return false;
            }
            // Otherwise the tail read was behind: it is read again.
        }
    }

    /// Take the index at the front; `None` when the ring holds none.
    fn take(&self) -> Option<u8> {
        loop {
            let position = self.head.load(Ordering::Relaxed);
            let cell = self.cell(position);
            let seen = cell.load(Ordering::Acquire);
            let stamp = stamp_of(seen);
            if stamp == ahead(position, 1) {
                // Filled at the head: taken in one step.
                let taken = word(ahead(position, Self::LEN), 0);
                if cell
                    .compare_exchange(seen, taken, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    Self::pass(&self.head, position);
                    return Some(index_of(seen));
                }
            } else if stamp == position {
                // Empty for the put at the head: every position before the
                // tail is filled, so nothing is filled past it either.
                return None;
            } else if stamp == ahead(position, Self::LEN) {
                // Taken by a take stopped before it moved the head past it.
                Self::pass(&self.head, position);
            }
            // Otherwise the head read was behind: it is read again.
        }
    }
}

/// A queue of at most `N` inputs, `N` a power of two from 2 to 128.
pub(crate) struct Ring<T, const N: usize> {
    slots: [Slot<T>; N],
    /// The indices of the slots no post holds and no input fills.
    free: IndexRing<N>,
    /// The indices of the slots that hold an input, in the order the posts
    /// linearized.
    filled: IndexRing<N>,
}

/// One slot: an input, or nothing.
struct Slot<T>(UnsafeCell<MaybeUninit<T>>);

// SAFETY: a slot is written only by the post that took its index off the free
// ring, before it puts the index on the filled ring with a release swap, and read
// only by the take that took the index off the filled ring, after an acquire load
// saw it there; the take puts the index back on the free ring with a release swap
// that the next post's acquire load sees before it writes. An index is on one
// ring or held by one context at a time, so no two contexts reach one slot at
// once, and an input is handed from the poster's context to the owner's, which
// `T: Send` allows.
unsafe impl<T: Send, const N: usize> Sync for Ring<T, N> {}

impl<T, const N: usize> Ring<T, N> {
    /// The shapes the queue holds, refused where the queue is declared.
    const SHAPE: () = assert!(
        N >= 2 && N <= 128 && N.is_power_of_two(),
        "a bounded inbox holds a power of two of inputs, from 2 to 128"
    );

    /// The value an array of slots is filled with: empty.
    #[allow(clippy::declare_interior_mutable_const)]
    const EMPTY_SLOT: Slot<T> = Slot(UnsafeCell::new(MaybeUninit::uninit()));

    /// An empty queue, for a `static`.
    pub(crate) const fn new() -> Self {
        Self::starting_at(0)
    }

    /// An empty queue whose rings' positions start at `start`.
    const fn starting_at(start: u32) -> Self {
        let () = Self::SHAPE;
        Self {
            slots: [Self::EMPTY_SLOT; N],
            free: IndexRing::starting_at(start, N as u32),
            filled: IndexRing::starting_at(start, 0),
        }
    }

    /// The input slot `index` holds, or will.
    fn slot(&self, index: u8) -> *mut T {
        self.slots[usize::from(index)].0.get().cast()
    }

    /// Put `item` at the back; `item` back when every slot is held, by an
    /// input the queue holds, one a take is reading out or one a post is
    /// writing. Any context's.
    pub(crate) fn enqueue(&self, item: T) -> Result<(), T> {
        let Some(index) = self.free.take() else {
            return Err(item);
        };
        let slot = self.slot(index);
        // SAFETY: the index came off the free ring, so the slot is this post's
        // alone until the index is put on a ring again.
        unsafe { slot.write(item) };
        #[cfg(any(all(test, feature = "std"), feature = "inbox-preemption"))]
        preemption::point();
        if self.filled.put(index) {
            return Ok(());
        }
        // The filled ring has room for every index, and this post holds one
        // that is on neither ring, so the put above is never refused. Were it,
        // the input goes back to its poster rather than stay in a slot no take
        // reaches.
        // SAFETY: the slot is still this post's, and holds the input written
        // above.
        let item = unsafe { slot.read() };
        let returned = self.free.put(index);
        debug_assert!(returned, "the free ring has room for an index taken off it");
        Err(item)
    }

    /// Take the input at the front; `None` when the queue holds none. The
    /// owner's: never waits, whatever a post under way is doing.
    pub(crate) fn dequeue(&self) -> Option<T> {
        let index = self.filled.take()?;
        // SAFETY: the index came off the filled ring, whose acquire load saw
        // the release that published the input written into the slot; the slot
        // is this take's alone until the index is put back on the free ring.
        let item = unsafe { self.slot(index).read() };
        let returned = self.free.put(index);
        debug_assert!(returned, "the free ring has room for an index taken off it");
        Some(item)
    }
}

impl<T, const N: usize> Drop for Ring<T, N> {
    // What the queue still holds is dropped with it. No post is under way: the
    // queue is borrowed alone.
    fn drop(&mut self) {
        while self.dequeue().is_some() {}
    }
}

/// The point between a post's write of its input into its slot and the step
/// that publishes it, where a test preempts the post: as an interrupt handler,
/// or the owner itself on one core, may stop a poster there.
#[cfg(any(all(test, feature = "std"), feature = "inbox-preemption"))]
pub(super) mod preemption {
    use std::cell::RefCell;

    type Preemption = Box<dyn FnOnce()>;

    std::thread_local! {
        static NEXT: RefCell<Option<Preemption>> = const { RefCell::new(None) };
    }

    /// Run `preempt` on this thread, once, where the next post this thread
    /// makes to a [`BoundedInbox`](crate::BoundedInbox) has written its input into
    /// the slot it holds and has not yet published it: what an interrupt
    /// handler, or the owner on the poster's core, runs while it stops the
    /// post there. `preempt` may post and drain as those may, the same inbox
    /// among them. With the crate's `inbox-preemption` feature, which a test
    /// names; no release build has it.
    pub fn preempt_next_post(preempt: impl FnOnce() + 'static) {
        NEXT.with(|next| *next.borrow_mut() = Some(Box::new(preempt)));
    }

    /// Run the preemption set for this thread, if any. It is taken out before
    /// it runs, so a post it makes is not preempted again.
    pub(super) fn point() {
        let preempt = NEXT.with(|next| next.borrow_mut().take());
        if let Some(preempt) = preempt {
            preempt();
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{ahead, preemption, word, IndexRing, Ring, POSITION_MASK};
    use core::sync::atomic::Ordering;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    #[test]
    fn a_ring_takes_in_order_and_refuses_past_its_capacity() {
        let ring: Ring<u32, 2> = Ring::new();
        assert_eq!(ring.enqueue(1), Ok(()));
        assert_eq!(ring.enqueue(2), Ok(()));
        assert_eq!(ring.enqueue(3), Err(3), "a ring of two took a third input");
        assert_eq!(ring.dequeue(), Some(1));
        assert_eq!(
            ring.enqueue(4),
            Ok(()),
            "a ring took no input after one was taken"
        );
        assert_eq!(ring.dequeue(), Some(2));
        assert_eq!(ring.dequeue(), Some(4));
        assert_eq!(ring.dequeue(), None);
    }

    // The positions are 24 bits wide and wrap: a ring that starts just short of
    // the wrap and goes past it still takes in order and answers empty when it
    // is.
    #[test]
    fn a_ring_keeps_its_order_across_the_wrap_of_its_positions() {
        let ring: Ring<u32, 4> = Ring::starting_at(POSITION_MASK - 5);
        for round in 0..300u32 {
            assert_eq!(ring.enqueue(round), Ok(()));
            assert_eq!(ring.enqueue(round + 1000), Ok(()));
            assert_eq!(ring.dequeue(), Some(round));
            assert_eq!(ring.dequeue(), Some(round + 1000));
            assert_eq!(ring.dequeue(), None);
        }
    }

    // What a ring still holds is dropped with it.
    #[test]
    fn a_ring_drops_what_it_still_holds() {
        let held = Arc::new(());
        let ring: Ring<Arc<()>, 4> = Ring::new();
        assert!(ring.enqueue(Arc::clone(&held)).is_ok());
        assert!(ring.enqueue(Arc::clone(&held)).is_ok());
        assert_eq!(Arc::strong_count(&held), 3);
        drop(ring);
        assert_eq!(
            Arc::strong_count(&held),
            1,
            "a dropped ring kept what it held"
        );
    }

    // docs/adr/0147, D5: a post is preempted on its own thread between the
    // write of its input and its publication, as an interrupt handler or the
    // owner on the poster's core preempts it. What runs there posts behind it,
    // and drains: the take passes the post under way, takes what was posted
    // behind it, and answers that the ring holds nothing more, all on the
    // thread the stopped post cannot run on until the take returns. A take that
    // waited for the stopped post would never return here; one that stopped in
    // front of it would give nothing. Once the post publishes, the next take
    // gives it.
    #[test]
    fn a_take_passes_a_post_under_way_and_takes_what_was_posted_behind_it() {
        let ring: Rc<Ring<u32, 4>> = Rc::new(Ring::new());
        let taken: Rc<RefCell<Vec<Option<u32>>>> = Rc::default();
        {
            let (ring, taken) = (Rc::clone(&ring), Rc::clone(&taken));
            preemption::preempt_next_post(move || {
                assert_eq!(
                    ring.enqueue(2),
                    Ok(()),
                    "a post behind one under way was refused"
                );
                taken.borrow_mut().extend([ring.dequeue(), ring.dequeue()]);
            });
        }
        assert_eq!(ring.enqueue(1), Ok(()), "the preempted post was refused");
        assert_eq!(
            *taken.borrow(),
            [Some(2), None],
            "the drain did not take what was posted behind a post under way, past it"
        );
        assert_eq!(
            [ring.dequeue(), ring.dequeue()],
            [Some(1), None],
            "the post that was under way was not taken once it was published"
        );
    }

    // docs/adr/0147, D5: a post stopped for as long as the rest of a run costs
    // the slot it holds and nothing more. Behind it the other slots go round
    // lap after lap, the ring refuses a post only when they are all held, and
    // the stopped post is taken once it publishes.
    #[test]
    fn a_post_under_way_holds_one_slot_and_never_stops_the_rest() {
        let ring: Rc<Ring<u32, 4>> = Rc::new(Ring::new());
        {
            let ring = Rc::clone(&ring);
            preemption::preempt_next_post(move || {
                for lap in 0..100u32 {
                    let first = 10 + 3 * lap;
                    for input in first..first + 3 {
                        assert_eq!(
                            ring.enqueue(input),
                            Ok(()),
                            "lap {lap}: a post was refused with a slot free beside the one under way"
                        );
                    }
                    assert_eq!(
                        ring.enqueue(9),
                        Err(9),
                        "lap {lap}: a ring of four took a fourth input beside a post under way"
                    );
                    for input in first..first + 3 {
                        assert_eq!(ring.dequeue(), Some(input), "lap {lap}");
                    }
                    assert_eq!(ring.dequeue(), None, "lap {lap}");
                }
            });
        }
        assert_eq!(ring.enqueue(1), Ok(()), "the preempted post was refused");
        assert_eq!([ring.dequeue(), ring.dequeue()], [Some(1), None]);
    }

    // A context stopped after it filled or took a cell and before it moved the
    // end past it leaves the end there; the next context at that end moves it
    // and goes on, rather than refusing a put or answering that a ring that
    // holds an index is empty.
    #[test]
    fn an_index_ring_moves_an_end_a_stopped_context_left_behind() {
        let ring: IndexRing<4> = IndexRing::starting_at(0, 0);
        // A put that filled position 0 with index 3 and did not move the tail.
        ring.cell(0).store(word(ahead(0, 1), 3), Ordering::Release);
        assert!(
            ring.put(1),
            "a put behind a filled cell the tail had not passed was refused"
        );
        // A take that took position 0 and did not move the head.
        ring.cell(0).store(word(ahead(0, 4), 0), Ordering::Release);
        assert_eq!(
            ring.take(),
            Some(1),
            "a take behind a taken cell the head had not passed did not take the next"
        );
        assert_eq!(ring.take(), None);
    }

    #[test]
    fn an_index_ring_refuses_a_put_when_it_holds_its_length() {
        let ring: IndexRing<2> = IndexRing::starting_at(0, 2);
        assert!(!ring.put(0), "a ring of two holding two took a third index");
        assert_eq!(ring.take(), Some(0));
        assert!(ring.put(0));
        assert_eq!(
            [ring.take(), ring.take(), ring.take()],
            [Some(1), Some(0), None]
        );
    }
}
