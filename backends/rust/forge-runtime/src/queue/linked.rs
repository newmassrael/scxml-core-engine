// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `segmented` row for one producer and one consumer (SCE Protocol-Synthesis
//! RFC §synth-5-P): linked Lamport rings, wait-free on both sides, bounded only
//! by the allocator it is given.
//!
//! **The shape.** The queue is a list of segments. A segment holds `N` slots
//! that are filled once, in order, and read once, in order: it is a Lamport ring
//! that is never lapped, so it needs neither the second lap that tells a full
//! ring from an empty one nor a free slot. The producer fills the newest segment
//! and, when it is full, takes a new one from the allocator and links it behind;
//! the consumer reads the oldest segment and, when it has read all `N`, follows
//! the link and gives the segment back. Each side keeps its own position and
//! shares it with the other only through the two things the other must see:
//! how many slots of a segment are filled, and the link to the next segment.
//!
//! **No reclamation domain.** The producer never touches a segment again after it
//! has linked its successor, and the consumer gives a segment back only after it
//! has read that link, so the consumer frees it directly. Nothing else can hold
//! a pointer to it: with one producer and one consumer no third party exists
//! (RFC §synth-5-P, *Reclamation domain*).
//!
//! **Progress.** Pop is wait-free. Push is wait-free but for the one step that
//! can take a segment: the allocator. The queue is therefore as strong as the
//! allocator the document declares ([`SegmentAllocator::PROGRESS`], held to the
//! declared rank by a compile-time assertion in [`LinkedLamport::new`]), and a
//! push the allocator refuses hands the element back as
//! [`PushError::OutOfMemory`].
//!
//! **The allocator is borrowed.** The queue holds `&'d A`, so it can move and the
//! allocator is the caller's, with no global default (SCE_FORGE.md §2.1, C2).
//!
//! **Ordering.** The producer publishes a filled slot by storing the count of
//! filled slots with Release, and the consumer reads that count with Acquire
//! before it reads the slot, so the write of an element happens-before the pop
//! that returns it. The link is stored with Release and read with Acquire for the
//! same reason: everything the producer did to the segment before linking it
//! happens-before the consumer's use of the next one. Each position is private
//! to one side, so it needs no ordering of its own.

use core::mem::MaybeUninit;
use core::ptr::{self, NonNull};

use super::allocator::{place, release, AllocError, Progress, SegmentAllocator};
use super::sync::{AtomicBool, AtomicPtr, AtomicUsize, Ordering, UnsafeCell};
use super::{Padded, PushError};

/// One segment: `N` slots filled once.
struct Segment<T, const N: usize> {
    /// How many slots, from the first, hold an element the producer published.
    /// Written only by the producer.
    written: Padded<AtomicUsize>,
    /// The segment after this one, or null while this is the newest. Written
    /// once, by the producer, when this segment is full and it takes the next.
    next: AtomicPtr<Segment<T, N>>,
    slots: [UnsafeCell<MaybeUninit<T>>; N],
}

impl<T, const N: usize> Segment<T, N> {
    fn new() -> Self {
        Self {
            written: Padded(AtomicUsize::new(0)),
            next: AtomicPtr::new(ptr::null_mut()),
            slots: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
        }
    }
}

/// A queue for one producer and one consumer that grows by segments of `N`
/// elements taken from the allocator `A`.
///
/// `REQUIRED` is the rank (`Progress::rank`) of the allocator progress the
/// document declared: an allocator that gives less does not build. The queue
/// gives its segments back when it is dropped, with the elements still in them.
pub struct LinkedLamport<'d, T, const N: usize, A: SegmentAllocator, const REQUIRED: u8> {
    /// The segment the consumer reads. Written only by the consumer (and by
    /// `new`).
    head: Padded<AtomicPtr<Segment<T, N>>>,
    /// How many elements of the head segment the consumer has taken. Written
    /// only by the consumer.
    head_read: Padded<AtomicUsize>,
    /// The segment the producer fills. Written only by the producer (and by
    /// `new`).
    tail: Padded<AtomicPtr<Segment<T, N>>>,
    /// Whether the producer / consumer handle is alive: there is one of each.
    producer_taken: AtomicBool,
    consumer_taken: AtomicBool,
    allocator: &'d A,
}

// SAFETY: a segment is written by the producer and read by the consumer, in the
// order the Release/Acquire pairs of the module documentation give; the heads and
// tails are each written by one side. Elements move from one context to the
// other, so they must be `Send`, and the allocator is called from both.
unsafe impl<T: Send, const N: usize, A: SegmentAllocator + Sync, const REQUIRED: u8> Sync
    for LinkedLamport<'_, T, N, A, REQUIRED>
{
}
unsafe impl<T: Send, const N: usize, A: SegmentAllocator + Sync, const REQUIRED: u8> Send
    for LinkedLamport<'_, T, N, A, REQUIRED>
{
}

impl<'d, T, const N: usize, A: SegmentAllocator, const REQUIRED: u8>
    LinkedLamport<'d, T, N, A, REQUIRED>
{
    /// What the shape must satisfy, evaluated wherever a queue is built: a
    /// segment of at least one element, and an allocator whose progress is at
    /// least the declared one, so a queue that would claim more than its allocator
    /// gives does not build.
    const SHAPE: () = {
        assert!(N > 0, "a segment holds at least one element");
        assert!(
            A::PROGRESS.rank() >= REQUIRED,
            "the allocator gives less progress than the document declared for it"
        );
    };

    /// The elements one segment holds.
    pub const SEGMENT: usize = N;

    /// The progress the allocator gives, and so the most push can.
    pub const ALLOCATOR_PROGRESS: Progress = A::PROGRESS;

    /// An empty queue over `allocator`, or [`AllocError`] when the allocator will
    /// not give the first segment.
    pub fn new(allocator: &'d A) -> Result<Self, AllocError> {
        let () = Self::SHAPE;
        let first = place(allocator, Segment::<T, N>::new).ok_or(AllocError)?;
        Ok(Self {
            head: Padded(AtomicPtr::new(first.as_ptr())),
            head_read: Padded(AtomicUsize::new(0)),
            tail: Padded(AtomicPtr::new(first.as_ptr())),
            producer_taken: AtomicBool::new(false),
            consumer_taken: AtomicBool::new(false),
            allocator,
        })
    }

    /// The producer, or `None` when it is already taken. There is one, and it is
    /// not `Clone`. Dropping it gives it back.
    pub fn producer(&self) -> Option<Producer<'_, 'd, T, N, A, REQUIRED>> {
        self.producer_taken
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
            .then(|| Producer { queue: self })
    }

    /// The consumer, or `None` when it is already taken. There is one, and it is
    /// not `Clone`. Dropping it gives it back.
    pub fn consumer(&self) -> Option<Consumer<'_, 'd, T, N, A, REQUIRED>> {
        self.consumer_taken
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
            .then(|| Consumer { queue: self })
    }
}

impl<T, const N: usize, A: SegmentAllocator, const REQUIRED: u8> Drop
    for LinkedLamport<'_, T, N, A, REQUIRED>
{
    fn drop(&mut self) {
        // `&mut self` means neither handle is alive: every segment from the head
        // is ours, and its slots from `read` up to `written` hold elements that
        // were pushed and not popped.
        let mut segment = self.head.0.load(Ordering::Acquire);
        let mut read = self.head_read.0.load(Ordering::Acquire);
        while let Some(block) = NonNull::new(segment) {
            // SAFETY: a live segment of this queue, reached once.
            let (written, next) = unsafe {
                let s = block.as_ref();
                (
                    s.written.0.load(Ordering::Acquire),
                    s.next.load(Ordering::Acquire),
                )
            };
            for index in read..written {
                // SAFETY: the slot holds a pushed element nobody popped.
                unsafe {
                    block.as_ref().slots[index].with_mut(|slot| (*slot).assume_init_drop());
                }
            }
            // SAFETY: the segment came from `place` on this allocator, holds no
            // element any more, and is released once.
            unsafe { release(self.allocator, block) };
            segment = next;
            read = 0;
        }
    }
}

/// The producing side of a [`LinkedLamport`]. There is one at a time.
pub struct Producer<'q, 'd, T, const N: usize, A: SegmentAllocator, const REQUIRED: u8> {
    queue: &'q LinkedLamport<'d, T, N, A, REQUIRED>,
}

impl<T, const N: usize, A: SegmentAllocator, const REQUIRED: u8> Drop
    for Producer<'_, '_, T, N, A, REQUIRED>
{
    fn drop(&mut self) {
        self.queue.producer_taken.store(false, Ordering::Release);
    }
}

impl<T, const N: usize, A: SegmentAllocator, const REQUIRED: u8>
    Producer<'_, '_, T, N, A, REQUIRED>
{
    /// Push `value`, or hand it back in [`PushError::OutOfMemory`] when the
    /// newest segment is full and the allocator will not give another. Wait-free
    /// when the allocator is.
    pub fn try_push(&self, value: T) -> Result<(), PushError<T>> {
        let queue = self.queue;
        // The tail and the filled count are this side's: only the producer
        // writes them.
        let mut tail = queue.tail.0.load(Ordering::Relaxed);
        // SAFETY: the tail is a live segment: the consumer frees a segment only
        // after reading its link, and the link is stored only once the producer
        // has moved past it.
        let mut written = unsafe { (*tail).written.0.load(Ordering::Relaxed) };
        if written == N {
            let Some(fresh) = place(queue.allocator, Segment::<T, N>::new) else {
                return Err(PushError::OutOfMemory(value));
            };
            // SAFETY: `tail` is live (above); this is the only store of its
            // link, and the producer never touches the segment again after it.
            unsafe { (*tail).next.store(fresh.as_ptr(), Ordering::Release) };
            queue.tail.0.store(fresh.as_ptr(), Ordering::Relaxed);
            tail = fresh.as_ptr();
            written = 0;
        }
        // SAFETY: slot `written` is past every filled slot, so the consumer
        // does not read it until the store below publishes it.
        unsafe {
            (*tail).slots[written].with_mut(|slot| slot.write(MaybeUninit::new(value)));
            (*tail).written.0.store(written + 1, Ordering::Release);
        }
        Ok(())
    }

    /// The elements one segment holds.
    pub const fn segment(&self) -> usize {
        N
    }
}

/// The consuming side of a [`LinkedLamport`]. There is one at a time.
pub struct Consumer<'q, 'd, T, const N: usize, A: SegmentAllocator, const REQUIRED: u8> {
    queue: &'q LinkedLamport<'d, T, N, A, REQUIRED>,
}

impl<T, const N: usize, A: SegmentAllocator, const REQUIRED: u8> Drop
    for Consumer<'_, '_, T, N, A, REQUIRED>
{
    fn drop(&mut self) {
        self.queue.consumer_taken.store(false, Ordering::Release);
    }
}

impl<T, const N: usize, A: SegmentAllocator, const REQUIRED: u8>
    Consumer<'_, '_, T, N, A, REQUIRED>
{
    /// Pop the oldest element, or `None` when the queue is empty. Wait-free.
    pub fn try_pop(&self) -> Option<T> {
        let queue = self.queue;
        let mut head = queue.head.0.load(Ordering::Relaxed);
        let mut read = queue.head_read.0.load(Ordering::Relaxed);
        if read == N {
            // Every element of the head segment is taken. Its successor exists
            // once the producer has linked it, and the producer never touches
            // this segment after that, so it is ours to give back.
            // SAFETY: `head` is a live segment: only this side frees one.
            let next = unsafe { (*head).next.load(Ordering::Acquire) };
            let next = NonNull::new(next)?;
            // SAFETY: the segment came from `place` on this allocator, every
            // element in it was popped, and the producer is past it.
            unsafe { release(queue.allocator, NonNull::new_unchecked(head)) };
            head = next.as_ptr();
            read = 0;
            queue.head.0.store(head, Ordering::Relaxed);
            queue.head_read.0.store(0, Ordering::Relaxed);
        }
        // SAFETY: `head` is a live segment (above).
        let written = unsafe { (*head).written.0.load(Ordering::Acquire) };
        if read >= written {
            return None;
        }
        // SAFETY: slot `read` is below the published count, so it holds an
        // element, and only this side reads it.
        let value = unsafe { (*head).slots[read].with(|slot| (*slot).assume_init_read()) };
        queue.head_read.0.store(read + 1, Ordering::Relaxed);
        Some(value)
    }

    /// The elements one segment holds.
    pub const fn segment(&self) -> usize {
        N
    }
}
