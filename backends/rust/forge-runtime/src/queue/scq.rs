// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `bounded` row for any cardinality other than one producer and one
//! consumer (SCE Protocol-Synthesis RFC §synth-5-P): the SCQ data queue,
//! lock-free on both sides.
//!
//! **The algorithm** is Nikolaev's SCQ ("A Scalable, Portable, and
//! Memory-Efficient Lock-Free FIFO Queue", DISC 2019), in the form of the
//! authors' single-width-CAS reference implementation (`lfring_cas1.h`,
//! dual 2-clause BSD / MIT). A ring of `R` slots keeps `2R` entries, each one
//! 64-bit word holding the entry's cycle, an `IsSafe` bit and the index it
//! carries. A ticket from a fetch-and-add on `head` or `tail` names an entry
//! and a cycle; an entry is written only by an enqueue of a later cycle and
//! consumed only by a dequeue of its own, so a stale operation fails its
//! compare-and-swap instead of corrupting the ring (ABA safety without a
//! reclamation scheme).
//!
//! **Two rings make a data queue.** The elements live in an array of `N`
//! slots. The *free* ring holds the indices of the slots nobody is using and
//! starts with all `N`; the *allocated* ring holds the indices of the slots
//! that hold an element, in the order they were filled, and starts empty. A
//! push takes an index from the free ring (none: the queue is full), writes
//! its element into that slot and puts the index on the allocated ring; a pop
//! does the reverse. The rings carry only indices, so the structure holds no
//! pointer: it is position-independent, which is what lets a mesh channel
//! place it in shared memory.
//!
//! **Capacity is exact at rest.** The rings have `R` slots, a power of two at
//! least `N` (checked when the queue is built), but only `N` indices ever
//! circulate, so never more than `N` elements fit, and a queue nothing is
//! running on refuses a push exactly when it holds `N`. Both are parameters
//! because Rust cannot size an array from an expression of another parameter;
//! the generator states both, and [`Scq::LAYOUT`] refuses a pair that cannot
//! run.
//!
//! **A ring needs at least as many slots as it has participants.** The
//! algorithm's empty test, the `threshold`, is justified for at most `R`
//! enqueuers and at most `R` dequeuers working one ring at once ("the number
//! of concurrent enqueuers or dequeuers never exceeds n", Nikolaev 2019,
//! section 5.1). Beyond that a completed push can leave the threshold at -1
//! with its element in the ring, and every pop then reports the queue empty
//! until another push completes. Measured 2026-10-09 with loom, one pusher
//! and a lap already run: two poppers on `R = 1` and three on `R = 2` lose
//! the element, two poppers on `R = 2` and on `R = 4` do not (up to five
//! preemptions). So a queue hands out at most `R` producer handles and at
//! most `R` consumer handles at a time, and the generator sizes `R` as the
//! capacity or the declared participants, whichever is larger.
//!
//! **A push may be refused while others hold slots.** A slot's index goes
//! back to the free ring only after the pop that took the element has read
//! it, and a push holds the index it took until it has published its element.
//! A push that looks for an index in that window finds none and reports the
//! queue full, though fewer than `N` elements are in it: waiting for the
//! operation that holds the slot would not be lock-free, since that
//! participant may be stopped. The shortfall is at most one slot per other
//! participant. Measured 2026-10-08, about one recorded run in 250 shows it
//! with two producers and two consumers on a loaded machine, and a history
//! checker that required the strict count refused those runs
//! (`queue_history::Refusal`).
//!
//! **Ordering.** The reference's: acquire loads, acquire-release
//! read-modify-writes, and sequentially consistent accesses to the
//! threshold. An element's write into its slot happens-before the pop that
//! reads it through the Release half of the compare-and-swap that put its
//! index on the allocated ring and the Acquire load that took it off; the
//! read happens-before the slot's reuse through the same pair on the free
//! ring. The loom models in `tests/loom_queue_scq.rs` check that, and the
//! casefile `an_scq_queue_orders_every_index_hand_over.cases` weakens each
//! hand-over in turn.
//!
//! **What no mutation case defends.** Four things here are kept as the
//! reference implementation has them and are not defended by a case, because
//! the models cannot reach the state that would show them wrong: the two
//! ticket fetch-and-adds' ordering, an enqueue's first read of the entry it
//! fills (both are covered by the compare-and-swap that follows), an
//! enqueue's check that the head has not passed an unsafe entry, and a
//! dequeue's marking of an entry that holds another cycle's index unsafe (the
//! last two matter only on a second lap with a dequeue ahead of the enqueue,
//! which takes more operations than three preemptions of two or three
//! threads give). Weaken one and the models stay green. Measured
//! 2026-10-09 for the last two (the head check removed; the unsafe marking
//! removed): the recorded histories, the thread test and the lapped loom
//! models (a lap already run, up to two participants a side on rings of two
//! and four) stay green too, so nothing run in this tree defends them. Miri and ThreadSanitizer look for
//! undefined behaviour and data races, not lost elements, and were not run on
//! mutants of these four. They are the reference implementation's, kept as
//! written, and a change to one needs a targeted schedule written for it
//! first.
//!
//! **Counters never wrap in practice.** Head and tail are 64-bit and the
//! algorithm tolerates a stalled participant for [`WRAP_BOUND_OPS`]
//! operations (RFC §synth-5-P, *Counter width*): 2^62, which at 10^9
//! operations a second is 146 years.
//!
//! The module exists only on a target with 64-bit atomics. On another it is
//! absent, and the generator refuses the row there instead.

use core::mem::MaybeUninit;

use super::sync::{AtomicI64, AtomicU64, AtomicUsize, Ordering, UnsafeCell};
use super::{Padded, PushError};

/// How many operations a participant may be delayed between reading an entry
/// and acting on it before it could be misled: `2^(w-2)` for a word of `w`
/// bits, independent of the capacity.
///
/// An entry's cycle is compared with a ticket's modulo `2^c`, where
/// `c = w - 1 - log2(2R)`, so a delay is safe until the entry's cycle has
/// moved by half that range, `2^(c-1)`; each cycle takes `2R` operations;
/// the product is `2^(w-2)`.
pub const WRAP_BOUND_OPS: u64 = 1 << 62;

/// `x` is before `y` in the order that wraps: the signed difference. Tickets
/// and cycles are compared this way throughout.
fn before(x: u64, y: u64) -> bool {
    (x.wrapping_sub(y) as i64) < 0
}

/// `x` is `y` or before it, in the same order.
fn at_or_before(x: u64, y: u64) -> bool {
    (x.wrapping_sub(y) as i64) <= 0
}

/// Two entries. A ring of `R` slots keeps `2R` of them, and an array length
/// cannot be written `2 * R`, so the array holds pairs.
struct Pair {
    even: AtomicU64,
    odd: AtomicU64,
}

/// One SCQ ring of `R` slots (`R` a power of two) and `2R` entries.
///
/// An entry is `cycle | IsSafe | index`: the index in the low `log2(2R)`
/// bits, the `IsSafe` bit above it, the cycle in the rest. An entry whose
/// index bits are all ones holds nothing.
struct Ring<const R: usize> {
    /// Next ticket a dequeue takes.
    head: Padded<AtomicU64>,
    /// How many failed dequeues remain before the ring may be called empty;
    /// negative means it is empty.
    threshold: Padded<AtomicI64>,
    /// Next ticket an enqueue takes.
    tail: Padded<AtomicU64>,
    entries: [Pair; R],
}

impl<const R: usize> Ring<R> {
    /// The number of entries, `2R`; also the value of the `IsSafe` bit.
    const ENTRIES: u64 = 2 * R as u64;
    /// The bits an index occupies.
    const INDEX_MASK: u64 = Self::ENTRIES - 1;
    /// The index bits and the `IsSafe` bit: everything below the cycle.
    const LOW_MASK: u64 = 2 * Self::ENTRIES - 1;
    /// `log2(2R)`.
    const ENTRY_ORDER: u32 = R.trailing_zeros() + 1;
    /// Threshold after an enqueue: `3R - 1` failed dequeues are enough to
    /// show that an empty-looking ring is empty.
    const THRESHOLD: i64 = 3 * R as i64 - 1;
    /// How many entries one 64-byte cache line holds, as a power of two.
    /// Consecutive tickets are spread over lines by rotating the ticket's
    /// bits by this much; a ring smaller than a line does not rotate.
    const LINE_SHIFT: u32 = 3;
    /// How often a dequeue that found an entry still empty looks again before
    /// it makes the entry unusable for the enqueue that owns it. Bounded, so
    /// the operation stays lock-free; loom, which cannot explore a long spin,
    /// looks once.
    const SPINS: u32 = if cfg!(loom) { 1 } else { 10_000 };

    /// The entry a ticket names: the ticket rotated so that consecutive
    /// tickets land on different cache lines.
    const fn map(ticket: u64) -> usize {
        let mask = Self::INDEX_MASK;
        if Self::ENTRY_ORDER >= Self::LINE_SHIFT {
            let up = Self::LINE_SHIFT;
            (((ticket & mask) >> (Self::ENTRY_ORDER - up)) | ((ticket << up) & mask)) as usize
        } else {
            (ticket & mask) as usize
        }
    }

    /// The ticket `map` sends to this entry.
    const fn unmap(position: u64) -> u64 {
        let mask = Self::INDEX_MASK;
        if Self::ENTRY_ORDER >= Self::LINE_SHIFT {
            let up = Self::LINE_SHIFT;
            ((position & mask) >> up) | ((position << (Self::ENTRY_ORDER - up)) & mask)
        } else {
            position & mask
        }
    }

    /// What the entry at `position` holds in a ring built with `filled`
    /// indices: the first `filled` tickets' entries hold index `ticket` in
    /// cycle 0, safe; the rest hold nothing, in the cycle before it.
    const fn initial(position: usize, filled: usize) -> u64 {
        let ticket = Self::unmap(position as u64);
        if (ticket as usize) < filled {
            Self::ENTRIES + ticket
        } else {
            u64::MAX
        }
    }

    /// A ring holding the indices `0..filled`, oldest first.
    #[cfg(not(loom))]
    const fn new(filled: usize) -> Self {
        let mut pairs: [MaybeUninit<Pair>; R] = [const { MaybeUninit::uninit() }; R];
        let mut k = 0;
        while k < R {
            pairs[k] = MaybeUninit::new(Pair {
                even: AtomicU64::new(Self::initial(2 * k, filled)),
                odd: AtomicU64::new(Self::initial(2 * k + 1, filled)),
            });
            k += 1;
        }
        // SAFETY: the loop wrote every element, and `MaybeUninit<Pair>` has
        // the layout of `Pair`, so the array is an initialised `[Pair; R]`.
        let entries = unsafe { core::ptr::read(&pairs as *const _ as *const [Pair; R]) };
        Self {
            head: Padded(AtomicU64::new(0)),
            threshold: Padded(AtomicI64::new(if filled == 0 {
                -1
            } else {
                Self::THRESHOLD
            })),
            tail: Padded(AtomicU64::new(filled as u64)),
            entries,
        }
    }

    /// A ring holding the indices `0..filled`, oldest first. Loom's atomics
    /// have no `const` constructor, so the loom build's is an ordinary
    /// function with the same contents.
    #[cfg(loom)]
    fn new(filled: usize) -> Self {
        Self {
            head: Padded(AtomicU64::new(0)),
            threshold: Padded(AtomicI64::new(if filled == 0 {
                -1
            } else {
                Self::THRESHOLD
            })),
            tail: Padded(AtomicU64::new(filled as u64)),
            entries: core::array::from_fn(|k| Pair {
                even: AtomicU64::new(Self::initial(2 * k, filled)),
                odd: AtomicU64::new(Self::initial(2 * k + 1, filled)),
            }),
        }
    }

    fn entry(&self, position: usize) -> &AtomicU64 {
        let pair = &self.entries[position >> 1];
        if position & 1 == 0 {
            &pair.even
        } else {
            &pair.odd
        }
    }

    /// Put `index` on the ring. The ring never refuses: at most `R` indices
    /// circulate through it, and an enqueue that finds its entry unusable
    /// takes the next ticket.
    fn enqueue(&self, index: usize) {
        let n = Self::ENTRIES;
        let stored = index as u64 ^ Self::INDEX_MASK;
        loop {
            let tail = self.tail.0.fetch_add(1, Ordering::AcqRel);
            let ticket_cycle = (tail << 1) | Self::LOW_MASK;
            let slot = self.entry(Self::map(tail));
            let mut entry = slot.load(Ordering::Acquire);
            // The entry is ours to fill when it is empty, from an earlier
            // cycle, and either safe or not yet passed by the head.
            loop {
                let entry_cycle = entry | Self::LOW_MASK;
                let usable = before(entry_cycle, ticket_cycle)
                    && (entry == entry_cycle
                        || (entry == (entry_cycle ^ n)
                            && at_or_before(self.head.0.load(Ordering::Acquire), tail)));
                if !usable {
                    break;
                }
                match slot.compare_exchange_weak(
                    entry,
                    ticket_cycle ^ stored,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        if self.threshold.0.load(Ordering::SeqCst) != Self::THRESHOLD {
                            self.threshold.0.store(Self::THRESHOLD, Ordering::SeqCst);
                        }
                        return;
                    }
                    Err(seen) => entry = seen,
                }
            }
        }
    }

    /// Move `tail` back to `head` after dequeues overshot an empty ring, so
    /// that the next enqueue does not start a cycle ahead of what is read.
    fn catch_up(&self, mut tail: u64, mut head: u64) {
        while self
            .tail
            .0
            .compare_exchange_weak(tail, head, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            head = self.head.0.load(Ordering::Acquire);
            tail = self.tail.0.load(Ordering::Acquire);
            if !before(tail, head) {
                break;
            }
        }
    }

    /// Take the oldest index off the ring, or `None` when it is empty.
    fn dequeue(&self) -> Option<usize> {
        if self.threshold.0.load(Ordering::SeqCst) < 0 {
            return None;
        }
        let n = Self::ENTRIES;
        loop {
            let head = self.head.0.fetch_add(1, Ordering::AcqRel);
            let ticket_cycle = (head << 1) | Self::LOW_MASK;
            let slot = self.entry(Self::map(head));
            let mut spins = 0;
            'look: loop {
                let mut entry = slot.load(Ordering::Acquire);
                loop {
                    let entry_cycle = entry | Self::LOW_MASK;
                    if entry_cycle == ticket_cycle {
                        // The entry is this ticket's: take its index and
                        // leave the entry empty in the same cycle.
                        slot.fetch_or(Self::INDEX_MASK, Ordering::AcqRel);
                        return Some((entry & Self::INDEX_MASK) as usize);
                    }
                    let replacement;
                    if (entry | n) != entry_cycle {
                        // It holds an index of another cycle: mark it unsafe,
                        // so no enqueue of this cycle's ticket fills the
                        // entry behind the head. Already unsafe: nothing to do.
                        replacement = entry & !n;
                        if entry == replacement {
                            break 'look;
                        }
                    } else {
                        // It is empty. An enqueue holding this cycle's ticket
                        // may be about to fill it; look a few times, then move
                        // the entry to this cycle so that enqueue fails.
                        spins += 1;
                        if spins <= Self::SPINS {
                            continue 'look;
                        }
                        replacement = ticket_cycle ^ ((!entry) & n);
                    }
                    if !before(entry_cycle, ticket_cycle) {
                        break 'look;
                    }
                    match slot.compare_exchange_weak(
                        entry,
                        replacement,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => break 'look,
                        Err(seen) => entry = seen,
                    }
                }
            }

            let tail = self.tail.0.load(Ordering::Acquire);
            if at_or_before(tail, head.wrapping_add(1)) {
                self.catch_up(tail, head.wrapping_add(1));
                self.threshold.0.fetch_sub(1, Ordering::AcqRel);
                return None;
            }
            if self.threshold.0.fetch_sub(1, Ordering::AcqRel) <= 0 {
                return None;
            }
        }
    }
}

/// A bounded queue of at most `N` elements for any number of producers and
/// any number of consumers, lock-free. Exactly `N` fit when nothing else is
/// running.
///
/// `R` is the ring size: a power of two, at least `N`, and at least the most
/// producers or consumers that will work the queue at once; [`Scq::LAYOUT`]
/// checks the first two and [`Scq::producer`] and [`Scq::consumer`] hold the
/// queue to the third. The queue owns its storage and allocates nothing, so it
/// may be a `static`. Elements still queued when the queue is dropped are
/// dropped with it.
///
/// Use it through [`Producer`] and [`Consumer`] handles, all from `&Scq`.
pub struct Scq<T, const N: usize, const R: usize> {
    /// The indices of the slots that hold an element, oldest first.
    allocated: Ring<R>,
    /// The indices of the slots nobody is using.
    free: Ring<R>,
    /// How many producer handles are alive.
    producers: AtomicUsize,
    /// How many consumer handles are alive.
    consumers: AtomicUsize,
    slots: [UnsafeCell<MaybeUninit<T>>; N],
}

// SAFETY: a slot is touched by one participant at a time. Its index is on
// exactly one of the two rings, or held by the one push or pop that took it
// off one and has not yet put it on the other; the rings hand an index from
// one participant to the next through a Release compare-and-swap and an
// Acquire load (module documentation), which orders the accesses to the
// slot. An element moves from a producer's context to a consumer's, so it
// must be `Send`.
unsafe impl<T: Send, const N: usize, const R: usize> Sync for Scq<T, N, R> {}

impl<T, const N: usize, const R: usize> Scq<T, N, R> {
    /// The capacity this queue holds, exactly.
    pub const CAPACITY: usize = N;

    /// The bytes of storage the queue occupies, for a target that budgets
    /// them.
    pub const STORAGE_BYTES: usize = core::mem::size_of::<Self>();

    /// What the pair `(N, R)` must satisfy, evaluated wherever a queue of
    /// this shape is built, so a pair the algorithm cannot run is a compile
    /// error rather than a runtime one.
    pub const LAYOUT: () = {
        assert!(N > 0, "a queue's capacity must be at least one element");
        assert!(R.is_power_of_two(), "the ring size must be a power of two");
        assert!(
            R >= N,
            "the ring must have at least as many slots as the capacity"
        );
        assert!(
            R <= 1 << 30,
            "a ring this large leaves its entries too few bits for the cycle"
        );
    };

    /// An empty queue.
    #[cfg(not(loom))]
    pub const fn new() -> Self {
        let () = Self::LAYOUT;
        Self {
            allocated: Ring::new(0),
            free: Ring::new(N),
            producers: AtomicUsize::new(0),
            consumers: AtomicUsize::new(0),
            slots: [const { UnsafeCell::new(MaybeUninit::uninit()) }; N],
        }
    }

    /// An empty queue. Loom's cells and atomics have no `const`
    /// constructor, so the loom build's is an ordinary function.
    #[cfg(loom)]
    pub fn new() -> Self {
        let () = Self::LAYOUT;
        Self {
            allocated: Ring::new(0),
            free: Ring::new(N),
            producers: AtomicUsize::new(0),
            consumers: AtomicUsize::new(0),
            slots: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
        }
    }

    /// The capacity this queue holds, exactly.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// A producer handle, or `None` when `R` producer handles are already
    /// alive: the ring cannot be shared by more enqueuers than it has slots
    /// (module documentation). Dropping a handle gives its place back.
    pub fn producer(&self) -> Option<Producer<'_, T, N, R>> {
        // `then`, not `then_some`: a handle built for a refused request would
        // be dropped at once, and dropping one gives a place back.
        Self::take_place(&self.producers).then(|| Producer { queue: self })
    }

    /// A consumer handle, or `None` when `R` consumer handles are already
    /// alive. Dropping a handle gives its place back.
    pub fn consumer(&self) -> Option<Consumer<'_, T, N, R>> {
        Self::take_place(&self.consumers).then(|| Consumer { queue: self })
    }

    /// Take one of the `R` places of a side, if one is free. A compare-and-swap
    /// loop and not a `fetch_add`, so a refused request leaves the count as it
    /// was and no concurrent request can be refused on account of it.
    fn take_place(alive: &AtomicUsize) -> bool {
        let mut seen = alive.load(Ordering::Relaxed);
        loop {
            if seen >= R {
                return false;
            }
            match alive.compare_exchange_weak(seen, seen + 1, Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => return true,
                Err(now) => seen = now,
            }
        }
    }
}

impl<T, const N: usize, const R: usize> Default for Scq<T, N, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize, const R: usize> Drop for Scq<T, N, R> {
    fn drop(&mut self) {
        // `&mut self` means no handle is alive, so every index on the
        // allocated ring names a slot that holds a pushed element nobody
        // popped.
        while let Some(index) = self.allocated.dequeue() {
            // SAFETY: see above; the index leaves the ring as it is dropped,
            // so no slot is dropped twice.
            self.slots[index].with_mut(|slot| unsafe { (*slot).assume_init_drop() });
        }
    }
}

/// A producing side of an [`Scq`]. It holds one of the queue's `R` producer
/// places until it is dropped.
pub struct Producer<'a, T, const N: usize, const R: usize> {
    queue: &'a Scq<T, N, R>,
}

impl<T, const N: usize, const R: usize> Drop for Producer<'_, T, N, R> {
    fn drop(&mut self) {
        self.queue.producers.fetch_sub(1, Ordering::Release);
    }
}

impl<T, const N: usize, const R: usize> Producer<'_, T, N, R> {
    /// Another producer of the same queue, or `None` when all `R` places are
    /// taken.
    pub fn try_clone(&self) -> Option<Self> {
        self.queue.producer()
    }

    /// Push `value`, or hand it back in [`PushError::Full`] when the queue
    /// holds its capacity, or while other participants' operations hold the
    /// slots it is short of (module documentation). Lock-free: some operation
    /// completes in a bounded number of steps whatever the other participants
    /// are doing.
    pub fn try_push(&self, value: T) -> Result<(), PushError<T>> {
        let queue = self.queue;
        let Some(index) = queue.free.dequeue() else {
            return Err(PushError::Full(value));
        };
        // SAFETY: the index came off the free ring, so no other participant
        // holds this slot until the enqueue below hands it on.
        queue.slots[index].with_mut(|slot| unsafe { slot.write(MaybeUninit::new(value)) });
        queue.allocated.enqueue(index);
        Ok(())
    }

    /// The capacity of the queue this side fills.
    pub const fn capacity(&self) -> usize {
        N
    }
}

/// A consuming side of an [`Scq`]. It holds one of the queue's `R` consumer
/// places until it is dropped.
pub struct Consumer<'a, T, const N: usize, const R: usize> {
    queue: &'a Scq<T, N, R>,
}

impl<T, const N: usize, const R: usize> Drop for Consumer<'_, T, N, R> {
    fn drop(&mut self) {
        self.queue.consumers.fetch_sub(1, Ordering::Release);
    }
}

impl<T, const N: usize, const R: usize> Consumer<'_, T, N, R> {
    /// Another consumer of the same queue, or `None` when all `R` places are
    /// taken.
    pub fn try_clone(&self) -> Option<Self> {
        self.queue.consumer()
    }

    /// Pop the oldest element, or `None` when the queue is empty. Lock-free.
    pub fn try_pop(&self) -> Option<T> {
        let queue = self.queue;
        let index = queue.allocated.dequeue()?;
        // SAFETY: the index came off the allocated ring, so the slot holds a
        // pushed element and no other participant holds the slot until the
        // enqueue below hands it on.
        let value = queue.slots[index].with(|slot| unsafe { (*slot).assume_init_read() });
        queue.free.enqueue(index);
        Some(value)
    }

    /// The capacity of the queue this side drains.
    pub const fn capacity(&self) -> usize {
        N
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    /// `map` and `unmap` are inverse bijections on the entries, for rings
    /// below and above the size at which the rotation starts.
    fn map_is_a_bijection<const R: usize>() {
        let entries = 2 * R as u64;
        let mut seen = [false; 128];
        assert!(entries as usize <= seen.len(), "R={R} outgrew the test");
        for ticket in 0..entries {
            let position = Ring::<R>::map(ticket);
            assert!(position < entries as usize, "R={R} ticket {ticket}");
            assert!(!seen[position], "R={R}: two tickets share entry {position}");
            seen[position] = true;
            assert_eq!(Ring::<R>::unmap(position as u64), ticket, "R={R}");
        }
    }

    #[test]
    fn the_entry_map_is_a_bijection() {
        map_is_a_bijection::<1>();
        map_is_a_bijection::<2>();
        map_is_a_bijection::<4>();
        map_is_a_bijection::<8>();
        map_is_a_bijection::<64>();
    }

    /// A single-threaded run of `rounds` fills and drains of a queue of
    /// capacity `N`: exact capacity, FIFO order, and the same again after the
    /// ring's cycles have moved many times.
    fn fills_and_drains<const N: usize, const R: usize>(rounds: usize) {
        let queue = Scq::<u64, N, R>::new();
        let (producer, consumer) = (queue.producer().unwrap(), queue.consumer().unwrap());
        let mut next = 0u64;
        let mut expected = 0u64;
        for _ in 0..rounds {
            for _ in 0..N {
                assert!(producer.try_push(next).is_ok(), "N={N}");
                next += 1;
            }
            assert_eq!(producer.try_push(next), Err(PushError::Full(next)), "N={N}");
            for _ in 0..N {
                assert_eq!(consumer.try_pop(), Some(expected), "N={N}");
                expected += 1;
            }
            assert_eq!(consumer.try_pop(), None, "N={N}");
        }
    }

    /// How many times the single-threaded runs below repeat. Miri interprets
    /// every step and is about a hundred times slower than native execution,
    /// so under it the runs are a few rounds: enough to cross a cycle of the
    /// smallest ring, which is what it is there to check for undefined
    /// behaviour, while the native runs are what take the cycles far.
    const ROUNDS: usize = if cfg!(miri) { 6 } else { 300 };

    #[test]
    fn capacity_is_exact_and_order_holds_over_many_cycles() {
        fills_and_drains::<1, 1>(ROUNDS);
        fills_and_drains::<2, 2>(ROUNDS);
        fills_and_drains::<3, 4>(ROUNDS);
        fills_and_drains::<5, 8>(ROUNDS);
        fills_and_drains::<8, 8>(ROUNDS);
        fills_and_drains::<13, 16>(ROUNDS * 2 / 3);
    }

    /// Many pops of an empty queue run the threshold down; an element pushed
    /// after them must still come out.
    #[test]
    fn an_element_pushed_after_many_empty_pops_is_seen() {
        let queue = Scq::<u64, 3, 4>::new();
        let (producer, consumer) = (queue.producer().unwrap(), queue.consumer().unwrap());
        for _ in 0..ROUNDS * 2 / 3 {
            assert_eq!(consumer.try_pop(), None);
        }
        for value in 1..=3 {
            assert!(producer.try_push(value).is_ok());
        }
        for value in 1..=3 {
            assert_eq!(consumer.try_pop(), Some(value));
        }
        assert_eq!(consumer.try_pop(), None);
    }

    #[test]
    fn a_queue_may_be_a_static() {
        static QUEUE: Scq<u8, 3, 4> = Scq::new();
        assert!(QUEUE.producer().unwrap().try_push(7).is_ok());
        assert_eq!(QUEUE.consumer().unwrap().try_pop(), Some(7));
    }

    /// A ring larger than the capacity, for participants the capacity alone
    /// would not allow, still holds exactly the capacity.
    #[test]
    fn a_ring_larger_than_the_capacity_still_holds_exactly_the_capacity() {
        fills_and_drains::<1, 4>(ROUNDS);
        fills_and_drains::<3, 8>(ROUNDS);
        fills_and_drains::<5, 16>(ROUNDS);
    }
}
