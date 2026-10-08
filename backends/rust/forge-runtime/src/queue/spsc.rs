// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `bounded` row for one producer and one consumer (SCE
//! Protocol-Synthesis RFC §synth-5-P): a Lamport ring, wait-free on both
//! sides.
//!
//! **Lap indices, not counters.** Each side's index runs over `0..2N` and
//! the slot it names is the index modulo `N`. Two laps are what tell a full
//! ring from an empty one without spending a slot: the indices are equal
//! exactly when the ring is empty, and `N` apart exactly when it is full, so
//! the ring holds exactly the capacity declared. An index never exceeds
//! `2N`, so nothing wraps however long the queue runs, on any word width;
//! the only requirement is that `2N` fits in a `usize`, which is a
//! compile-time assertion.
//!
//! **Ordering.** The producer publishes a filled slot with a Release store
//! of its index, and the consumer reads that index with Acquire before it
//! reads the slot, so the write of an element happens-before its pop. The
//! consumer returns a slot the same way in the other direction, so its read
//! of an element happens-before the producer reuses the slot. Each side
//! keeps the other's index as last read, and reads the shared one again
//! only when the cached one says the ring is full (producer) or empty
//! (consumer).

use core::mem::MaybeUninit;

use super::sync::{AtomicUsize, Ordering, UnsafeCell};
use super::{Padded, PushError};

/// A bounded queue of exactly `N` elements for one producer and one
/// consumer.
///
/// The queue owns its storage and allocates nothing. Use it through the
/// two handles: [`Spsc::split`] borrows them from a queue the caller holds
/// mutably, and [`Spsc::producer`] / [`Spsc::consumer`] create them from a
/// shared reference, for a queue in a `static` shared with an interrupt
/// handler. Elements still queued when the queue is dropped are dropped
/// with it.
pub struct Spsc<T, const N: usize> {
    /// Lap index of the next slot the consumer pops. Written only by the
    /// consumer.
    head: Padded<AtomicUsize>,
    /// Lap index of the next slot the producer fills. Written only by the
    /// producer.
    tail: Padded<AtomicUsize>,
    slots: [UnsafeCell<MaybeUninit<T>>; N],
}

// SAFETY: the two handles touch disjoint slots — the producer only slots
// outside `[head, tail)`, the consumer only slots inside it — and every
// hand-over of a slot from one side to the other is ordered by the
// Release/Acquire pair on an index (module documentation). An element moves
// from the producer's context to the consumer's, so it must be `Send`.
unsafe impl<T: Send, const N: usize> Sync for Spsc<T, N> {}

impl<T, const N: usize> Spsc<T, N> {
    /// The capacity this queue holds, exactly.
    pub const CAPACITY: usize = N;

    /// The size of the lap-index space, evaluated wherever a queue of this
    /// `N` is constructed, so a capacity the ring cannot represent is a
    /// compile error rather than a runtime one.
    const LAPS: usize = {
        assert!(N > 0, "a queue's capacity must be at least one element");
        assert!(
            N <= usize::MAX / 2,
            "a queue's lap indices run over twice its capacity, which must fit in a usize"
        );
        2 * N
    };

    /// An empty queue.
    #[cfg(not(loom))]
    pub const fn new() -> Self {
        let _ = Self::LAPS;
        Self {
            head: Padded(AtomicUsize::new(0)),
            tail: Padded(AtomicUsize::new(0)),
            slots: [const { UnsafeCell::new(MaybeUninit::uninit()) }; N],
        }
    }

    /// An empty queue. Loom's cells and atomics have no `const`
    /// constructor, so the loom build's is an ordinary function.
    #[cfg(loom)]
    pub fn new() -> Self {
        let _ = Self::LAPS;
        Self {
            head: Padded(AtomicUsize::new(0)),
            tail: Padded(AtomicUsize::new(0)),
            slots: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
        }
    }

    /// The capacity this queue holds, exactly.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// The producer and the consumer of a queue the caller holds mutably.
    /// Both borrow the queue, so neither can outlive it and no second pair
    /// can exist while they do.
    pub fn split(&mut self) -> (Producer<'_, T, N>, Consumer<'_, T, N>) {
        let queue: &Self = self;
        // SAFETY: `queue` is reborrowed from `&mut self` for as long as the
        // two handles live, so these are the only producer and the only
        // consumer of this queue while they do.
        unsafe { (queue.producer(), queue.consumer()) }
    }

    /// The producer of a queue reached through a shared reference — a
    /// queue in a `static`, which [`Spsc::split`] cannot borrow mutably.
    ///
    /// # Safety
    ///
    /// No other [`Producer`] of this queue may exist while the returned one
    /// does, and if an earlier one existed on another execution context,
    /// its last push must happen-before this call.
    pub unsafe fn producer(&self) -> Producer<'_, T, N> {
        Producer {
            queue: self,
            tail: self.tail.0.load(Ordering::Acquire),
            cached_head: self.head.0.load(Ordering::Acquire),
        }
    }

    /// The consumer of a queue reached through a shared reference — a
    /// queue in a `static`, which [`Spsc::split`] cannot borrow mutably.
    ///
    /// # Safety
    ///
    /// No other [`Consumer`] of this queue may exist while the returned one
    /// does, and if an earlier one existed on another execution context,
    /// its last pop must happen-before this call.
    pub unsafe fn consumer(&self) -> Consumer<'_, T, N> {
        Consumer {
            queue: self,
            head: self.head.0.load(Ordering::Acquire),
            cached_tail: self.tail.0.load(Ordering::Acquire),
        }
    }

    /// The slot a lap index names.
    fn slot(&self, index: usize) -> &UnsafeCell<MaybeUninit<T>> {
        &self.slots[if index >= N { index - N } else { index }]
    }

    /// The lap index after `index`.
    fn next(index: usize) -> usize {
        if index + 1 == Self::LAPS {
            0
        } else {
            index + 1
        }
    }

    /// How many elements lie between a consumer index and a producer index.
    fn occupancy(head: usize, tail: usize) -> usize {
        if tail >= head {
            tail - head
        } else {
            tail + Self::LAPS - head
        }
    }
}

impl<T, const N: usize> Default for Spsc<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for Spsc<T, N> {
    fn drop(&mut self) {
        let tail = self.tail.0.load(Ordering::Acquire);
        let mut head = self.head.0.load(Ordering::Acquire);
        while head != tail {
            // SAFETY: `&mut self` means no handle is alive, and every slot in
            // `[head, tail)` holds an element that was pushed and not popped.
            self.slot(head)
                .with_mut(|slot| unsafe { (*slot).assume_init_drop() });
            head = Self::next(head);
        }
    }
}

/// The producing side of an [`Spsc`]. There is one at a time.
pub struct Producer<'a, T, const N: usize> {
    queue: &'a Spsc<T, N>,
    /// This side's own index; the shared one only publishes it.
    tail: usize,
    /// The consumer's index as this side last read it.
    cached_head: usize,
}

impl<T, const N: usize> Producer<'_, T, N> {
    /// Push `value`, or hand it back in [`PushError::Full`] when the queue
    /// holds its capacity. Wait-free: a bounded number of steps whatever the
    /// consumer is doing.
    pub fn try_push(&mut self, value: T) -> Result<(), PushError<T>> {
        let queue = self.queue;
        if Spsc::<T, N>::occupancy(self.cached_head, self.tail) == N {
            // Acquire pairs with the consumer's Release store of its index:
            // its read of the slot about to be reused happens-before the
            // write below.
            self.cached_head = queue.head.0.load(Ordering::Acquire);
            if Spsc::<T, N>::occupancy(self.cached_head, self.tail) == N {
                return Err(PushError::Full(value));
            }
        }
        // SAFETY: the slot at `tail` is outside `[head, tail)`, so the
        // consumer does not read it until the store below publishes it.
        queue
            .slot(self.tail)
            .with_mut(|slot| unsafe { slot.write(MaybeUninit::new(value)) });
        self.tail = Spsc::<T, N>::next(self.tail);
        // Release: the write of the slot happens-before the pop of any
        // consumer that acquires this index.
        queue.tail.0.store(self.tail, Ordering::Release);
        Ok(())
    }

    /// The capacity of the queue this side fills.
    pub const fn capacity(&self) -> usize {
        N
    }
}

/// The consuming side of an [`Spsc`]. There is one at a time.
pub struct Consumer<'a, T, const N: usize> {
    queue: &'a Spsc<T, N>,
    /// This side's own index; the shared one only publishes it.
    head: usize,
    /// The producer's index as this side last read it.
    cached_tail: usize,
}

impl<T, const N: usize> Consumer<'_, T, N> {
    /// Pop the oldest element, or `None` when the queue is empty.
    /// Wait-free: a bounded number of steps whatever the producer is doing.
    pub fn try_pop(&mut self) -> Option<T> {
        let queue = self.queue;
        if self.head == self.cached_tail {
            // Acquire pairs with the producer's Release store of its index:
            // its write of the slot happens-before the read below.
            self.cached_tail = queue.tail.0.load(Ordering::Acquire);
            if self.head == self.cached_tail {
                return None;
            }
        }
        // SAFETY: the slot at `head` is inside `[head, tail)`, so it holds a
        // pushed element, and the producer does not reuse it until the store
        // below returns it.
        let value = queue
            .slot(self.head)
            .with(|slot| unsafe { (*slot).assume_init_read() });
        self.head = Spsc::<T, N>::next(self.head);
        // Release: the read of the slot happens-before the producer's reuse
        // of it, by any producer that acquires this index.
        queue.head.0.store(self.head, Ordering::Release);
        Some(value)
    }

    /// The capacity of the queue this side drains.
    pub const fn capacity(&self) -> usize {
        N
    }
}
