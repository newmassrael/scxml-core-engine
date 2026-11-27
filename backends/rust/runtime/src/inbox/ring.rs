// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The queue of a bounded inbox (`BoundedInbox`): Vyukov's bounded queue of
//! many producers, drained by the session's owner, with a take that never
//! stops in front of an input still being written while an input posted after
//! it waits behind it (docs/adr/0147, D1).
//!
//! A post reserves the next cell, by moving the enqueue position past it,
//! writes its input into the cell, and publishes the cell, by stamping its
//! sequence. Posts finish in any order, and the cells are taken in the order
//! they were reserved. Vyukov's take, and the `heapless::mpmc` queue that held
//! the inbox before this one, answers that the queue is empty whenever the
//! oldest cell reserved is not yet published, even when a post that reserved a
//! later cell has published it and returned. The drain before an event of the
//! owner's own then applies nothing, and the own event enters ahead of a post
//! taken before it was raised, which the promise does not allow
//! (`docs/formal/InboxService.tla`: an own event enters behind every input
//! taken before it).
//!
//! So a take that finds the oldest cell reserved and not published looks at the
//! cells reserved after it. When one of them is published, a post behind the
//! oldest has written its input and may have returned, and the take waits for
//! the oldest to be published. When none is, every input reserved and not taken
//! belongs to a post still under way, which may linearize after whatever the
//! owner does next, and the take answers that the queue holds nothing it can
//! reach.
//!
//! What the wait asks of a deployment: between its reservation and its
//! publication a post runs a few instructions and nothing that waits, so the
//! owner waits at most for a poster to run them. An interrupt handler, another
//! core, and a thread a preemptive scheduler runs beside the owner always do. A
//! poster that the owner itself preempts on one core, a task of lower priority,
//! cannot run them while the owner waits, so a deployment whose posters include
//! such a task and a context that posts behind it (an interrupt handler) can
//! stop its owner there; the order the promise asks cannot be kept by a ring
//! that is taken in the order its cells were reserved without that wait.

use core::cell::UnsafeCell;
use core::cmp::Ordering as Turn;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

/// A queue of at most `N` inputs, `N` a power of two from 2 to 128.
pub(crate) struct Ring<T, const N: usize> {
    cells: [Cell<T>; N],
    /// The position the next post reserves.
    enqueue_pos: AtomicU8,
    /// The position the next take reads.
    dequeue_pos: AtomicU8,
}

/// One cell: an input, and the sequence that says whose turn the cell is.
/// For the position `p` the cell stands at, its sequence is `p` while it is
/// free for the post that reserves `p`, `p + 1` once that post has published
/// its input, and `p + N` once the input is taken, which frees the cell for
/// the position `N` on.
struct Cell<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    sequence: AtomicU8,
}

impl<T> Cell<T> {
    const fn new(sequence: u8) -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            sequence: AtomicU8::new(sequence),
        }
    }
}

// SAFETY: a cell's value is written only by the post that reserved the cell,
// before it publishes the cell with a release store, and read only by the take
// that moved the dequeue position past the cell, after an acquire load saw it
// published; the take frees the cell with a release store that the next post's
// acquire load sees before it writes. So no two contexts reach one value at
// once, and a value is handed from the poster's context to the owner's, which
// `T: Send` allows.
unsafe impl<T: Send, const N: usize> Sync for Ring<T, N> {}

impl<T, const N: usize> Ring<T, N> {
    const MASK: u8 = (N - 1) as u8;

    /// The shapes the queue holds, refused where the queue is declared.
    const SHAPE: () = assert!(
        N >= 2 && N <= 128 && N.is_power_of_two(),
        "a bounded inbox holds a power of two of inputs, from 2 to 128"
    );

    const EMPTY_CELL: Cell<T> = Cell::new(0);

    /// An empty queue, for a `static`.
    pub(crate) const fn new() -> Self {
        let () = Self::SHAPE;
        let mut cells = [Self::EMPTY_CELL; N];
        let mut at = 0;
        while at < N {
            cells[at] = Cell::new(at as u8);
            at += 1;
        }
        Self {
            cells,
            enqueue_pos: AtomicU8::new(0),
            dequeue_pos: AtomicU8::new(0),
        }
    }

    fn cell(&self, pos: u8) -> &Cell<T> {
        &self.cells[usize::from(pos & Self::MASK)]
    }

    /// Put `item` at the back; `item` back when the queue holds `N` inputs,
    /// counted with those a take is reading out. Any context's.
    pub(crate) fn enqueue(&self, item: T) -> Result<(), T> {
        let mut pos = self.enqueue_pos.load(Ordering::Relaxed);
        let (cell, reserved) = loop {
            let cell = self.cell(pos);
            let sequence = cell.sequence.load(Ordering::Acquire);
            let lead = (sequence as i8).wrapping_sub(pos as i8);
            match lead.cmp(&0) {
                Turn::Equal => match self.enqueue_pos.compare_exchange_weak(
                    pos,
                    pos.wrapping_add(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break (cell, pos),
                    Err(now) => pos = now,
                },
                // The cell still holds an input of the lap before: full.
                Turn::Less => return Err(item),
                // Another post reserved the position since it was read.
                Turn::Greater => pos = self.enqueue_pos.load(Ordering::Relaxed),
            }
        };
        #[cfg(all(test, feature = "std"))]
        hook::reserved();
        // SAFETY: the reservation above made the cell this post's alone: no
        // other post reserves its position again before a take frees it, and
        // no take reads it before the store below publishes it.
        unsafe { (*cell.value.get()).as_mut_ptr().write(item) };
        cell.sequence
            .store(reserved.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Take the input at the front; `None` when no input is published there
    /// and none is published behind it. The owner's.
    pub(crate) fn dequeue(&self) -> Option<T> {
        loop {
            let pos = self.dequeue_pos.load(Ordering::Relaxed);
            let cell = self.cell(pos);
            let sequence = cell.sequence.load(Ordering::Acquire);
            let lead = (sequence as i8).wrapping_sub(pos.wrapping_add(1) as i8);
            match lead.cmp(&0) {
                Turn::Equal => {
                    if self
                        .dequeue_pos
                        .compare_exchange_weak(
                            pos,
                            pos.wrapping_add(1),
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        // SAFETY: the cell's post published its input, which
                        // the acquire load above saw, and moving the dequeue
                        // position past the cell made it this take's alone; the
                        // store below frees it only once the input is read out.
                        let item = unsafe { (*cell.value.get()).as_ptr().read() };
                        cell.sequence.store(
                            pos.wrapping_add(Self::MASK).wrapping_add(1),
                            Ordering::Release,
                        );
                        return Some(item);
                    }
                }
                Turn::Less => {
                    // The front is free, or reserved and not yet published.
                    // Behind a published cell it is waited for; otherwise every
                    // input not taken is a post's still under way.
                    if !self.published_after(pos) {
                        return None;
                    }
                    #[cfg(all(test, feature = "std"))]
                    hook::waiting();
                    core::hint::spin_loop();
                }
                // The front was taken since it was read.
                Turn::Greater => {}
            }
        }
    }

    /// Whether a cell reserved after the position `front` is published.
    fn published_after(&self, front: u8) -> bool {
        let reserved = self.enqueue_pos.load(Ordering::Acquire).wrapping_sub(front);
        (1..reserved).any(|behind| {
            let pos = front.wrapping_add(behind);
            self.cell(pos).sequence.load(Ordering::Acquire) == pos.wrapping_add(1)
        })
    }
}

impl<T, const N: usize> Drop for Ring<T, N> {
    // What the queue still holds is dropped with it. No post is under way: the
    // queue is borrowed alone.
    fn drop(&mut self) {
        while self.dequeue().is_some() {}
    }
}

/// What the tests below park a post or a take at: the point between a post's
/// reservation and its publication, and a take that waits.
#[cfg(all(test, feature = "std"))]
mod hook {
    use std::cell::RefCell;

    type Hook = Box<dyn FnMut()>;

    std::thread_local! {
        static RESERVED: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static WAITING: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }

    /// Call `hook` on this thread between each reservation and its
    /// publication.
    pub(super) fn on_reserved(hook: Hook) {
        RESERVED.with(|slot| *slot.borrow_mut() = Some(hook));
    }

    /// Call `hook` on this thread each time a take waits.
    pub(super) fn on_waiting(hook: Hook) {
        WAITING.with(|slot| *slot.borrow_mut() = Some(hook));
    }

    pub(super) fn reserved() {
        RESERVED.with(|slot| {
            if let Some(hook) = slot.borrow_mut().as_mut() {
                hook();
            }
        });
    }

    pub(super) fn waiting() {
        WAITING.with(|slot| {
            if let Some(hook) = slot.borrow_mut().as_mut() {
                hook();
            }
        });
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{hook, Ring};
    use std::sync::{mpsc, Arc};
    use std::thread;

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

    // The positions are eight bits wide and wrap: a ring that went round them
    // many times still takes in order and answers empty when it is.
    #[test]
    fn a_ring_keeps_its_order_across_the_wrap_of_its_positions() {
        let ring: Ring<u32, 4> = Ring::new();
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

    // docs/adr/0147, D1: one post reserves the front and is parked before it
    // publishes; a second post, made after, publishes behind it and returns. A
    // take now must not answer that the ring is empty, since the second post
    // was taken and returned: it waits for the first to be published, and then
    // gives both, in the order they were reserved. Without the wait it answers
    // at once, and the drain before an own event lets that event in ahead of a
    // post that returned before it.
    #[test]
    fn a_take_waits_for_a_reserved_input_that_a_published_one_is_behind() {
        let ring: Arc<Ring<u32, 4>> = Arc::new(Ring::new());
        let (parked, is_parked) = mpsc::channel::<()>();
        let (release, released) = mpsc::channel::<()>();
        let first = {
            let ring = Arc::clone(&ring);
            thread::spawn(move || {
                hook::on_reserved(Box::new(move || {
                    parked.send(()).expect("the test listens");
                    released.recv().expect("the test releases the post");
                }));
                ring.enqueue(1)
            })
        };
        is_parked.recv().expect("the first post reserved its cell");
        assert_eq!(ring.enqueue(2), Ok(()), "the second post was refused");

        let (waits, is_waiting) = mpsc::channel::<()>();
        let owner = {
            let ring = Arc::clone(&ring);
            thread::spawn(move || {
                let mut told = false;
                hook::on_waiting(Box::new(move || {
                    if !told {
                        told = true;
                        let _ = waits.send(());
                    }
                }));
                [ring.dequeue(), ring.dequeue(), ring.dequeue()]
            })
        };
        let waited = is_waiting.recv();
        release
            .send(())
            .expect("the first post waits to be released");
        assert_eq!(first.join().expect("the first post"), Ok(()));
        let taken = owner.join().expect("the owner");
        assert!(
            waited.is_ok(),
            "the take did not wait for the reserved input a published one was behind: it gave \
             {taken:?}"
        );
        assert_eq!(taken, [Some(1), Some(2), None]);
    }

    // A reserved input with nothing published behind it is a post still under
    // way, which may linearize after the take: the take answers at once.
    #[test]
    fn a_take_does_not_wait_for_a_reserved_input_with_nothing_published_behind_it() {
        let ring: Arc<Ring<u32, 4>> = Arc::new(Ring::new());
        let (parked, is_parked) = mpsc::channel::<()>();
        let (release, released) = mpsc::channel::<()>();
        let first = {
            let ring = Arc::clone(&ring);
            thread::spawn(move || {
                hook::on_reserved(Box::new(move || {
                    parked.send(()).expect("the test listens");
                    released.recv().expect("the test releases the post");
                }));
                ring.enqueue(1)
            })
        };
        is_parked.recv().expect("the post reserved its cell");
        assert_eq!(ring.dequeue(), None);
        release.send(()).expect("the post waits to be released");
        assert_eq!(first.join().expect("the post"), Ok(()));
        assert_eq!(ring.dequeue(), Some(1));
    }
}
