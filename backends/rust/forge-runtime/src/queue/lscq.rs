// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `segmented` row for any cardinality other than one producer and one
//! consumer (SCE Protocol-Synthesis RFC §synth-5-P): LSCQ, a list of SCQ rings,
//! lock-free on both sides and bounded only by the allocator it is given.
//!
//! **The algorithm** is Nikolaev's LSCQ (DISC 2019, section 6): a Michael-Scott
//! list whose nodes are SCQ rings instead of single elements. Producers push into
//! the newest ring (`tail`); consumers pop from the oldest (`head`). A ring that
//! cannot take another element is *closed*: nothing is ever pushed into it again.
//! The producer that found it closed takes a segment from the allocator, puts its
//! element in it before anyone can see it, and links it behind the closed one with
//! a compare-and-swap on that ring's `next`; a producer that loses that race frees
//! its segment and pushes into the winner's. A consumer that finds the oldest ring
//! empty and closed moves `head` to its successor and retires it.
//!
//! **Each ring is the SCQ data queue of [`super::scq`]**, which keeps its elements
//! in a slot array and moves their indices through two rings. A segment is
//! therefore allocated whole, and an element is written into its slot once and
//! read out of it once, so an element is never moved after it is pushed.
//!
//! **Closing.** The close is a bit in the allocated ring's tail, set with a
//! fetch-and-or, and an enqueue takes its ticket with a fetch-and-add that returns
//! the bit: a ticket granted before the close can still be used, one after it is
//! refused. That is what makes the order across segments exact. Without it a
//! producer could pass a "not closed" check, stall, and push into a segment a
//! consumer had already emptied and retired, and the element would be lost.
//!
//! **Retiring a ring waits for the elements in flight.** A consumer sees a ring
//! empty through SCQ's threshold, which does not look at a ticket an enqueue holds
//! and has not yet filled. So the consumer that finds the oldest ring empty and
//! closed pops it once more with `Scq::pop_drained`, which walks the head to the
//! closed tail and either takes the element such an enqueue filled or makes its
//! entry unusable, and only when that answers empty does it move `head`.
//!
//! **Reclamation.** A segment can be read by a participant after another has
//! moved `head` past it, so it is freed through a hazard-pointer domain the caller
//! builds ([`HazardDomain`]); each handle holds one slot of it and protects the
//! one segment it is touching. A segment is retired only by the consumer whose
//! compare-and-swap moved `head` past it; a stalled producer that still holds the
//! segment keeps only that segment alive (RFC §synth-5-P, *Reclamation domain*).
//! `tail` never points at a retired segment: the producer that links a successor
//! holds its hazard on the old one until it has tried to move `tail`, and either it
//! or a helper has by then.
//!
//! **Progress.** Push and pop are lock-free: a retry means another participant
//! moved `head`, `tail` or a ring, and the helping step lets a push finish what a
//! stalled one started. A push is as strong as the allocator it calls
//! ([`SegmentAllocator::PROGRESS`], held to the declared rank by a compile-time
//! assertion in [`Lscq::new`]); a refusal hands the element back as
//! [`PushError::OutOfMemory`].
//!
//! **Sizes.** Each ring is an `Scq<T, N, R>`: `N` elements a segment, `R` slots
//! in its index rings, a power of two at least `N` and at least the most producers
//! or consumers that work the queue at once (module `scq`). The domain has `H`
//! slots, one per live handle, both sides together.
//!
//! The module exists only on a target with 64-bit atomics, like [`super::scq`].

use core::ptr::{self, NonNull};

use super::allocator::{place, release, AllocError, Progress, SegmentAllocator};
use super::hazard::{Hazard, HazardDomain, Retired};
use super::scq::Scq;
use super::sync::{AtomicPtr, AtomicUsize, Ordering};
use super::{Padded, PushError};

/// One segment: a ring and the link to the next.
///
/// `repr(C)` with the retirement header first: the domain names a segment by the
/// address of its header, which is the segment's.
#[repr(C)]
struct Segment<T, const N: usize, const R: usize> {
    retired: Retired,
    /// The segment after this one, or null while this is the newest. Written
    /// once, by the producer whose compare-and-swap links a successor.
    next: AtomicPtr<Segment<T, N, R>>,
    ring: Scq<T, N, R>,
}

impl<T, const N: usize, const R: usize> Segment<T, N, R> {
    fn new() -> Self {
        Self {
            retired: Retired::new(),
            next: AtomicPtr::new(ptr::null_mut()),
            ring: Scq::new(),
        }
    }
}

/// Free a retired segment: the function a domain calls once no hazard names it.
///
/// # Safety
///
/// `node` is the header of a `Segment<T, N, R>` that came from `place` on the
/// allocator `context` points to, and nothing reaches it.
unsafe fn reclaim<T, const N: usize, const R: usize, A: SegmentAllocator>(
    node: *mut Retired,
    context: *const (),
) {
    // SAFETY: the caller's contract, and `retire`'s: the allocator outlives the
    // domain's hold on the segment, which the queue guarantees by borrowing both
    // for `'d` and flushing the domain when it is dropped.
    unsafe {
        let allocator = &*context.cast::<A>();
        release(
            allocator,
            NonNull::new_unchecked(node.cast::<Segment<T, N, R>>()),
        );
    }
}

/// A queue for any number of producers and any number of consumers that grows by
/// segments of `N` elements taken from the allocator `A`.
///
/// `R` is the ring size, a power of two at least `N` and at least the most
/// producers or consumers at once; `H` the slots of the domain, one per live
/// handle; `REQUIRED` the rank (`Progress::rank`) of the allocator progress the
/// document declared. The queue borrows the allocator and the domain for `'d`, so
/// it can move, and gives its segments back when it is dropped, with the elements
/// still in them.
pub struct Lscq<
    'd,
    T,
    const N: usize,
    const R: usize,
    const H: usize,
    A: SegmentAllocator,
    const REQUIRED: u8,
> {
    /// The oldest segment. Moved forward by the consumer that finds it empty and
    /// closed.
    head: Padded<AtomicPtr<Segment<T, N, R>>>,
    /// The newest segment, or the one before it while a producer is between
    /// linking a successor and moving `tail`.
    tail: Padded<AtomicPtr<Segment<T, N, R>>>,
    /// How many producer and consumer handles are alive.
    producers: AtomicUsize,
    consumers: AtomicUsize,
    allocator: &'d A,
    domain: &'d HazardDomain<H>,
}

// SAFETY: a segment's ring is touched by many participants, each through its own
// atomic operations (module `scq`); its memory is kept alive by the hazard slots
// the handles hold and freed only once none names it. Elements move from one
// context to another, so they must be `Send`, and the allocator is called from
// every participant.
unsafe impl<T: Send, const N: usize, const R: usize, const H: usize, A, const REQ: u8> Sync
    for Lscq<'_, T, N, R, H, A, REQ>
where
    A: SegmentAllocator + Sync,
{
}
unsafe impl<T: Send, const N: usize, const R: usize, const H: usize, A, const REQ: u8> Send
    for Lscq<'_, T, N, R, H, A, REQ>
where
    A: SegmentAllocator + Sync,
{
}

impl<'d, T, const N: usize, const R: usize, const H: usize, A, const REQUIRED: u8>
    Lscq<'d, T, N, R, H, A, REQUIRED>
where
    A: SegmentAllocator,
{
    /// What the shape must satisfy, evaluated wherever a queue is built: the
    /// rings' own layout (a segment of at least one element, a ring a power of
    /// two at least that large), and an allocator whose progress is at least the
    /// declared one, so a queue that would claim more than its allocator gives
    /// does not build.
    const SHAPE: () = {
        let () = Scq::<T, N, R>::LAYOUT;
        assert!(
            A::PROGRESS.rank() >= REQUIRED,
            "the allocator gives less progress than the document declared for it"
        );
    };

    /// The elements one segment holds.
    pub const SEGMENT: usize = N;

    /// The progress the allocator gives, and so the most push can.
    pub const ALLOCATOR_PROGRESS: Progress = A::PROGRESS;

    /// An empty queue over `allocator` and `domain`, or [`AllocError`] when the
    /// allocator will not give the first segment.
    pub fn new(allocator: &'d A, domain: &'d HazardDomain<H>) -> Result<Self, AllocError> {
        let () = Self::SHAPE;
        let first = place(allocator, Segment::<T, N, R>::new).ok_or(AllocError)?;
        Ok(Self {
            head: Padded(AtomicPtr::new(first.as_ptr())),
            tail: Padded(AtomicPtr::new(first.as_ptr())),
            producers: AtomicUsize::new(0),
            consumers: AtomicUsize::new(0),
            allocator,
            domain,
        })
    }

    /// A producer handle, or `None` when `R` producers are already alive or the
    /// domain has no free slot. Dropping a handle gives its place back.
    pub fn producer(&self) -> Option<Producer<'_, 'd, T, N, R, H, A, REQUIRED>> {
        let place = Place::take(&self.producers, R)?;
        let hazard = self.domain.acquire()?;
        Some(Producer {
            queue: self,
            hazard,
            _place: place,
        })
    }

    /// A consumer handle, or `None` when `R` consumers are already alive or the
    /// domain has no free slot. Dropping a handle gives its place back.
    pub fn consumer(&self) -> Option<Consumer<'_, 'd, T, N, R, H, A, REQUIRED>> {
        let place = Place::take(&self.consumers, R)?;
        let hazard = self.domain.acquire()?;
        Some(Consumer {
            queue: self,
            hazard,
            _place: place,
        })
    }

    /// The context a retired segment is freed with: the allocator.
    fn context(&self) -> *const () {
        (self.allocator as *const A).cast()
    }
}

impl<T, const N: usize, const R: usize, const H: usize, A, const REQUIRED: u8> Drop
    for Lscq<'_, T, N, R, H, A, REQUIRED>
where
    A: SegmentAllocator,
{
    fn drop(&mut self) {
        // `&mut self` means no handle is alive: every segment from the head is
        // ours alone, and dropping one drops the elements still in it.
        let mut segment = self.head.0.load(Ordering::Acquire);
        while let Some(block) = NonNull::new(segment) {
            // SAFETY: a live segment of this queue, reached once.
            let next = unsafe { block.as_ref().next.load(Ordering::Acquire) };
            // SAFETY: it came from `place` on this allocator and is released
            // once.
            unsafe { release(self.allocator, block) };
            segment = next;
        }
        // The segments consumers retired are on the domain's list and may still
        // be waiting for a scan; they must be freed before the allocator they name
        // can go.
        self.domain.flush();
    }
}

/// A place on one side of the queue, given back when it is dropped.
struct Place<'a>(&'a AtomicUsize);

impl<'a> Place<'a> {
    /// Take one of `limit` places, if one is free. A compare-and-swap loop and
    /// not a `fetch_add`, so a refused request leaves the count as it was.
    fn take(alive: &'a AtomicUsize, limit: usize) -> Option<Self> {
        let mut seen = alive.load(Ordering::Relaxed);
        loop {
            if seen >= limit {
                return None;
            }
            match alive.compare_exchange_weak(seen, seen + 1, Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => return Some(Self(alive)),
                Err(now) => seen = now,
            }
        }
    }
}

impl Drop for Place<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

/// A producing side of an [`Lscq`]. It holds one producer place and one slot of
/// the domain until it is dropped.
pub struct Producer<
    'q,
    'd,
    T,
    const N: usize,
    const R: usize,
    const H: usize,
    A: SegmentAllocator,
    const REQUIRED: u8,
> {
    queue: &'q Lscq<'d, T, N, R, H, A, REQUIRED>,
    hazard: Hazard<'d, H>,
    _place: Place<'q>,
}

impl<T, const N: usize, const R: usize, const H: usize, A, const REQUIRED: u8>
    Producer<'_, '_, T, N, R, H, A, REQUIRED>
where
    A: SegmentAllocator,
{
    /// Another producer of the same queue, or `None` when every place or slot is
    /// taken.
    pub fn try_clone(&self) -> Option<Self> {
        let queue = self.queue;
        let place = Place::take(&queue.producers, R)?;
        let hazard = queue.domain.acquire()?;
        Some(Self {
            queue,
            hazard,
            _place: place,
        })
    }

    /// Push `value`, or hand it back in [`PushError::OutOfMemory`] when the
    /// newest segment is closed and the allocator will not give another. Lock-free
    /// when the allocator is.
    pub fn try_push(&self, mut value: T) -> Result<(), PushError<T>> {
        let queue = self.queue;
        loop {
            let tail = self.hazard.protect(&queue.tail.0);
            // SAFETY: protected, and never null: `tail` always names a segment.
            let segment = unsafe { &*tail };
            let next = segment.next.load(Ordering::Acquire);
            if !next.is_null() {
                // A producer linked a successor and has not moved `tail`: finish
                // its step.
                let _ =
                    queue
                        .tail
                        .0
                        .compare_exchange(tail, next, Ordering::AcqRel, Ordering::Relaxed);
                continue;
            }
            match segment.ring.push_or_close(value) {
                Ok(()) => {
                    self.hazard.clear();
                    return Ok(());
                }
                Err(back) => value = back,
            }
            // The segment is closed. Put the element in a segment of our own
            // before anyone can see it, then try to link it behind this one.
            let Some(fresh) = place(queue.allocator, Segment::<T, N, R>::new) else {
                self.hazard.clear();
                return Err(PushError::OutOfMemory(value));
            };
            // SAFETY: `fresh` is ours alone until it is linked below.
            let pushed = unsafe { fresh.as_ref() }.ring.push_or_close(value);
            if pushed.is_err() {
                unreachable!("a new ring is open and has a free slot, so it takes an element");
            }
            match segment.next.compare_exchange(
                ptr::null_mut(),
                fresh.as_ptr(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    let _ = queue.tail.0.compare_exchange(
                        tail,
                        fresh.as_ptr(),
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    );
                    self.hazard.clear();
                    return Ok(());
                }
                Err(_) => {
                    // Another producer linked first. Take our element back out
                    // of the segment nobody saw, free it, and push behind theirs.
                    // SAFETY: never published, so nobody else reaches it.
                    let back = unsafe { fresh.as_ref() }.ring.pop_any();
                    // SAFETY: as above; the segment is empty now.
                    unsafe { release(queue.allocator, fresh) };
                    value = back.expect("a segment holding one element gives it back");
                }
            }
        }
    }
}

/// A consuming side of an [`Lscq`]. It holds one consumer place and one slot of
/// the domain until it is dropped.
pub struct Consumer<
    'q,
    'd,
    T,
    const N: usize,
    const R: usize,
    const H: usize,
    A: SegmentAllocator,
    const REQUIRED: u8,
> {
    queue: &'q Lscq<'d, T, N, R, H, A, REQUIRED>,
    hazard: Hazard<'d, H>,
    _place: Place<'q>,
}

impl<T, const N: usize, const R: usize, const H: usize, A, const REQUIRED: u8>
    Consumer<'_, '_, T, N, R, H, A, REQUIRED>
where
    A: SegmentAllocator,
{
    /// Another consumer of the same queue, or `None` when every place or slot is
    /// taken.
    pub fn try_clone(&self) -> Option<Self> {
        let queue = self.queue;
        let place = Place::take(&queue.consumers, R)?;
        let hazard = queue.domain.acquire()?;
        Some(Self {
            queue,
            hazard,
            _place: place,
        })
    }

    /// Pop the oldest element, or `None` when the queue is empty. Lock-free.
    pub fn try_pop(&self) -> Option<T> {
        let queue = self.queue;
        loop {
            let head = self.hazard.protect(&queue.head.0);
            // SAFETY: protected, and never null: `head` always names a segment.
            let segment = unsafe { &*head };
            if let Some(value) = segment.ring.pop_any() {
                self.hazard.clear();
                return Some(value);
            }
            let next = segment.next.load(Ordering::Acquire);
            if next.is_null() {
                self.hazard.clear();
                return None;
            }
            // The segment is closed. An enqueue that took a ticket before the
            // close may still be filling its entry, so the empty answer above
            // does not yet show the segment holds nothing and never will.
            if let Some(value) = segment.ring.pop_drained() {
                self.hazard.clear();
                return Some(value);
            }
            if queue
                .head
                .0
                .compare_exchange(head, next, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                // This consumer moved `head` past the segment, so it alone
                // retires it. Its own hazard goes first, or the scan the
                // retirement may start would keep the segment for it.
                self.hazard.clear();
                // SAFETY: `head` is the header of a segment of this queue,
                // unlinked from `head` by the compare-and-swap above; `tail` is
                // not left on it (module documentation); it is retired once; and
                // `reclaim` frees exactly such a segment with the allocator,
                // which the queue borrows for as long as the domain holds it.
                unsafe {
                    queue.domain.retire(
                        head.cast::<Retired>(),
                        reclaim::<T, N, R, A>,
                        queue.context(),
                    );
                }
            }
        }
    }
}
