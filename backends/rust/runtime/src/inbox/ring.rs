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

use core::mem::MaybeUninit;

use sync::{AtomicU32, Ordering, UnsafeCell};

/// The atomics, the cell and the spin hint the queue is written against.
///
/// A build gets `core`'s. The loom models (`loom_models` below, run through
/// `backends/rust/runtime/tests/loom_inbox_ring.rs`) build this file as a test
/// under `--cfg loom` and get loom's instrumented ones, which is how they
/// explore every interleaving, and every outcome the C11 memory model allows,
/// of the text that ships. The library is never built with loom's: under
/// `--cfg loom` it is built as the models' dependency, without `test`, and
/// keeps `core`'s, so `BoundedInbox::new` stays a constant function. A slot is
/// reached only through `with` and `with_mut`, loom's form, so that loom sees
/// every access to one.
mod sync {
    #[cfg(all(loom, test))]
    pub(super) use loom::cell::UnsafeCell;
    #[cfg(all(loom, test))]
    pub(super) use loom::hint::spin_loop;
    #[cfg(all(loom, test))]
    pub(super) use loom::sync::atomic::{AtomicU32, Ordering};

    #[cfg(not(all(loom, test)))]
    pub(super) use core::hint::spin_loop;
    #[cfg(not(all(loom, test)))]
    pub(super) use core::sync::atomic::{AtomicU32, Ordering};

    /// A cell whose contents are reached through a pointer handed to a
    /// closure, as loom's are.
    #[cfg(not(all(loom, test)))]
    #[repr(transparent)]
    pub(super) struct UnsafeCell<T>(core::cell::UnsafeCell<T>);

    #[cfg(not(all(loom, test)))]
    impl<T> UnsafeCell<T> {
        pub(super) const fn new(value: T) -> Self {
            Self(core::cell::UnsafeCell::new(value))
        }

        pub(super) fn with<R>(&self, f: impl FnOnce(*const T) -> R) -> R {
            f(self.0.get())
        }

        pub(super) fn with_mut<R>(&self, f: impl FnOnce(*mut T) -> R) -> R {
            f(self.0.get())
        }
    }
}

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
    #[cfg(not(all(loom, test)))]
    #[allow(clippy::declare_interior_mutable_const)]
    const UNSET: AtomicU32 = AtomicU32::new(0);

    /// The word cell `cell` of the array first holds, in a ring whose positions
    /// start at `start` and that holds the indices from 0 to `held - 1` at its
    /// first `held` positions: the cell stands at the position `k` places from
    /// `start` that it is the cell of.
    const fn first_word(start: u32, held: u32, cell: u32) -> u32 {
        let k = cell.wrapping_sub(start) & Self::MASK;
        let position = ahead(start, k);
        if k < held {
            word(ahead(position, 1), k)
        } else {
            word(position, 0)
        }
    }

    /// A ring whose positions start at `start`, holding the indices from 0 to
    /// `held - 1` at its first `held` positions.
    #[cfg(not(all(loom, test)))]
    const fn starting_at(start: u32, held: u32) -> Self {
        let mut cells = [Self::UNSET; N];
        let mut cell = 0;
        while cell < Self::LEN {
            cells[cell as usize] = AtomicU32::new(Self::first_word(start, held, cell));
            cell += 1;
        }
        Self {
            cells,
            head: AtomicU32::new(start),
            tail: AtomicU32::new(ahead(start, held)),
        }
    }

    /// The same ring, of loom's atomics, which no constant makes.
    #[cfg(all(loom, test))]
    fn starting_at(start: u32, held: u32) -> Self {
        Self {
            cells: core::array::from_fn(|cell| {
                AtomicU32::new(Self::first_word(start, held, cell as u32))
            }),
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
            // Otherwise the tail read was behind, or another context moved
            // first: read again. The hint is where a model checker runs the
            // others, so a stale read is not taken again for ever.
            sync::spin_loop();
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
            // Otherwise the head read was behind, or another context moved
            // first: read again.
            sync::spin_loop();
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
    #[cfg(not(all(loom, test)))]
    #[allow(clippy::declare_interior_mutable_const)]
    const EMPTY_SLOT: Slot<T> = Slot(UnsafeCell::new(MaybeUninit::uninit()));

    /// An empty queue, for a `static`.
    #[cfg(not(all(loom, test)))]
    pub(crate) const fn new() -> Self {
        Self::starting_at(0)
    }

    /// An empty queue whose rings' positions start at `start`.
    #[cfg(not(all(loom, test)))]
    const fn starting_at(start: u32) -> Self {
        let () = Self::SHAPE;
        Self {
            slots: [Self::EMPTY_SLOT; N],
            free: IndexRing::starting_at(start, N as u32),
            filled: IndexRing::starting_at(start, 0),
        }
    }

    /// An empty queue of loom's atomics and cells, which no constant makes.
    #[cfg(all(loom, test))]
    pub(crate) fn new() -> Self {
        let () = Self::SHAPE;
        Self {
            slots: core::array::from_fn(|_| Slot(UnsafeCell::new(MaybeUninit::uninit()))),
            free: IndexRing::starting_at(0, N as u32),
            filled: IndexRing::starting_at(0, 0),
        }
    }

    /// The cell of the slot `index`.
    fn slot(&self, index: u8) -> &UnsafeCell<MaybeUninit<T>> {
        &self.slots[usize::from(index)].0
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
        slot.with_mut(|slot| unsafe { slot.cast::<T>().write(item) });
        #[cfg(any(all(test, feature = "std", not(loom)), feature = "inbox-preemption"))]
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
        let item = slot.with(|slot| unsafe { slot.cast::<T>().read() });
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
        let item = self
            .slot(index)
            .with(|slot| unsafe { slot.cast::<T>().read() });
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
#[cfg(any(all(test, feature = "std", not(loom)), feature = "inbox-preemption"))]
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

#[cfg(all(test, feature = "std", not(loom)))]
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

/// The loom models of the queue (docs/adr/0147, D5): two posters and the owner,
/// under every interleaving within a bound of three preemptions and every
/// outcome the C11 memory model allows their atomics. Loom fails a model where
/// a slot is read or written with no happens-before edge to the access before
/// it, so a slot index handed over without the ordering it needs shows up as
/// a race on the slot. What each model asserts is what holds under every
/// schedule: each input taken comes out once, none is lost and none that was
/// refused comes out, each poster's inputs come out in its order, a post is
/// refused only while every slot is held, and a drain takes a post that
/// returned before it began whatever a post still under way holds.
///
/// No thread waits for another: each makes its attempts once, and what the
/// queue holds once the threads have joined is drained and counted. Bounded
/// at three preemptions, as the forge queues' models are
/// (`backends/rust/forge-runtime/tests/loom_queue_scq.rs` says why): a pass is
/// "no violation within three preemptions". Built only as a test under
/// `--cfg loom`, through `backends/rust/runtime/tests/loom_inbox_ring.rs`,
/// whose manifest entry gives the command.
#[cfg(all(loom, test))]
mod loom_models {
    use super::Ring;
    use loom::sync::atomic::{AtomicBool, Ordering};
    use loom::sync::Arc;
    use loom::thread;

    /// Run `body` under every interleaving within the preemption bound.
    fn model(body: impl Fn() + Sync + Send + 'static) {
        let mut builder = loom::model::Builder::new();
        builder.preemption_bound = Some(3);
        builder.check(body);
    }

    /// What `ring` still holds, oldest first: run where nothing else posts
    /// to it, so the loop ends.
    fn drain<const N: usize>(ring: &Ring<u32, N>) -> Vec<u32> {
        let mut left = Vec::new();
        while let Some(input) = ring.dequeue() {
            left.push(input);
        }
        left
    }

    /// The inputs a poster's posts in `inputs`, made one after another, had
    /// taken, in its order.
    fn post_each<const N: usize>(ring: &Ring<u32, N>, inputs: [u32; 2]) -> Vec<u32> {
        let mut taken = Vec::new();
        for input in inputs {
            match ring.enqueue(input) {
                Ok(()) => taken.push(input),
                Err(back) => assert_eq!(back, input, "a refused post handed back another input"),
            }
        }
        taken
    }

    /// `entered`, what came out in the order it came out, holds each input
    /// `posted` took once and nothing else, and each poster's in its order.
    fn assert_each_once_in_each_posters_order(entered: &[u32], posted: &[Vec<u32>]) {
        let mut taken: Vec<u32> = posted.iter().flatten().copied().collect();
        let mut came_out = entered.to_vec();
        taken.sort_unstable();
        came_out.sort_unstable();
        assert_eq!(
            came_out, taken,
            "what came out is not each input the posts took, once"
        );
        for posts in posted {
            let order: Vec<u32> = entered
                .iter()
                .copied()
                .filter(|input| posts.contains(input))
                .collect();
            assert_eq!(&order, posts, "a poster's inputs came out out of its order");
        }
    }

    /// Two posters post two inputs each into a queue of two while the owner
    /// takes twice. Posts may be refused, as the slots are held; every input
    /// a post took comes out once, in its poster's order, and no refused one
    /// comes out.
    #[test]
    fn two_posters_and_the_owner_keep_each_input_once_in_its_posters_order() {
        model(|| {
            let ring: Arc<Ring<u32, 2>> = Arc::new(Ring::new());
            let posters: Vec<_> = [[10, 11], [20, 21]]
                .into_iter()
                .map(|inputs| {
                    let ring = Arc::clone(&ring);
                    thread::spawn(move || post_each(&ring, inputs))
                })
                .collect();
            let mut entered: Vec<u32> = (0..2).filter_map(|_| ring.dequeue()).collect();
            let posted: Vec<Vec<u32>> = posters
                .into_iter()
                .map(|poster| poster.join().expect("a poster"))
                .collect();
            entered.extend(drain(&ring));
            assert_each_once_in_each_posters_order(&entered, &posted);
        });
    }

    /// A queue of two holds one input; a poster posts two more while the
    /// owner takes the first out. The poster's first post always finds a free
    /// slot. Its second may be refused, but only while both slots are held: a
    /// poster that has heard the owner put its slot back has that slot to post
    /// into. What came out is the inputs taken, in order.
    #[test]
    fn a_post_is_refused_only_while_every_slot_is_held() {
        model(|| {
            let ring: Arc<Ring<u32, 2>> = Arc::new(Ring::new());
            assert_eq!(ring.enqueue(1), Ok(()));
            let slot_back = Arc::new(AtomicBool::new(false));
            let poster = {
                let (ring, slot_back) = (Arc::clone(&ring), Arc::clone(&slot_back));
                thread::spawn(move || {
                    assert_eq!(
                        ring.enqueue(2),
                        Ok(()),
                        "a post was refused with a slot free"
                    );
                    let heard = slot_back.load(Ordering::SeqCst);
                    (heard, ring.enqueue(3))
                })
            };
            assert_eq!(
                ring.dequeue(),
                Some(1),
                "the owner did not take an input posted before it began"
            );
            slot_back.store(true, Ordering::SeqCst);
            let (heard, third) = poster.join().expect("the poster");
            if heard {
                assert_eq!(
                    third,
                    Ok(()),
                    "a post was refused after the owner had put a slot back"
                );
            }
            let mut entered = vec![1];
            entered.extend(drain(&ring));
            match third {
                Ok(()) => assert_eq!(entered, [1, 2, 3]),
                Err(back) => {
                    assert_eq!(back, 3, "a refused post handed back another input");
                    assert_eq!(entered, [1, 2]);
                }
            }
        });
    }

    /// The take past a post under way (docs/adr/0147, D1 and D5): the first
    /// poster posts one input, the second posts one and then says it has
    /// returned. The owner, having heard it, drains, as the drain before an
    /// event of its own does: that drain takes the second input, in every
    /// interleaving, the ones where the first post holds its slot and has not
    /// published among them, and the take never waits for the first.
    #[test]
    fn a_drain_takes_a_post_that_returned_whatever_a_post_under_way_holds() {
        model(|| {
            let ring: Arc<Ring<u32, 2>> = Arc::new(Ring::new());
            let returned = Arc::new(AtomicBool::new(false));
            let first = {
                let ring = Arc::clone(&ring);
                thread::spawn(move || ring.enqueue(1))
            };
            let second = {
                let (ring, returned) = (Arc::clone(&ring), Arc::clone(&returned));
                thread::spawn(move || {
                    let posted = ring.enqueue(2);
                    returned.store(true, Ordering::SeqCst);
                    posted
                })
            };
            let heard = returned.load(Ordering::SeqCst);
            let mut entered = drain(&ring);
            if heard {
                assert!(
                    entered.contains(&2),
                    "a drain begun after a post returned did not take it: {entered:?}"
                );
            }
            assert_eq!(first.join().expect("the first poster"), Ok(()));
            assert_eq!(second.join().expect("the second poster"), Ok(()));
            entered.extend(drain(&ring));
            entered.sort_unstable();
            assert_eq!(entered, [1, 2], "each input comes out once");
        });
    }
}
