// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The queue runtime on real threads (SCE Protocol-Synthesis RFC
//! §synth-5-P, verification layer 5).
//!
//! These are ordinary tests, and they run in the ordinary suite. Their job is
//! to be the program a memory-safety tool watches: Miri runs them under an
//! interpreter that checks every access for undefined behaviour and, with a
//! seed, explores weak-memory outcomes; ThreadSanitizer runs them natively
//! and reports a data race on any pair of accesses not ordered by the
//! program's own synchronisation. Neither can say the queue is correct, and
//! these tests do not rely on them to: each asserts what a queue must
//! deliver, so a run without a tool still checks the contract, and a run with
//! one checks that the contract is kept without an access the memory model
//! does not allow.
//!
//! What the tools reach that the loom models do not: the models are small by
//! necessity (two or three threads, one or two elements); these use as many
//! elements as a tool can afford, element types with a destructor, and the
//! queue's whole API from several handles at once. What loom reaches that
//! these do not: every interleaving within its bound, rather than the
//! schedules a machine happens to give.
//!
//! The counts are small under Miri, which is about a hundred times slower
//! than native execution; `cfg(miri)` picks them.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use sce_forge_runtime::queue::scq::Scq;
use sce_forge_runtime::queue::spsc::Spsc;
use sce_forge_runtime::queue::PushError;

/// How many values each producer pushes.
const PER_PRODUCER: u64 = if cfg!(miri) { 12 } else { 4_000 };

/// How many times a dropped-queue run is repeated.
const REPEATS: usize = if cfg!(miri) { 2 } else { 200 };

#[test]
fn spsc_delivers_every_value_in_order() {
    let mut queue = Spsc::<u64, 3>::new();
    let (mut producer, mut consumer) = queue.split();
    thread::scope(|scope| {
        scope.spawn(move || {
            for value in 0..PER_PRODUCER {
                let mut value = value;
                loop {
                    match producer.try_push(value) {
                        Ok(()) => break,
                        Err(PushError::Full(back)) => {
                            value = back;
                            thread::yield_now();
                        }
                        Err(PushError::OutOfMemory(_)) => panic!("bounded queue out of memory"),
                    }
                }
            }
        });
        scope.spawn(move || {
            let mut expected = 0;
            while expected < PER_PRODUCER {
                match consumer.try_pop() {
                    Some(value) => {
                        assert_eq!(value, expected, "values leave in the order they came");
                        expected += 1;
                    }
                    None => thread::yield_now(),
                }
            }
        });
    });
}

/// Producers and consumers of an SCQ queue, every value delivered once, and
/// each producer's values seen by each consumer in the order that producer
/// pushed them.
fn scq_delivers_every_value_once<const N: usize, const R: usize>(producers: u64, consumers: usize) {
    let queue = Scq::<u64, N, R>::new();
    let queue = &queue;
    let total = producers * PER_PRODUCER;
    let delivered = AtomicUsize::new(0);
    let delivered = &delivered;

    let seen: Vec<Vec<u64>> = thread::scope(|scope| {
        for who in 0..producers {
            scope.spawn(move || {
                let producer = queue.producer().unwrap();
                for k in 0..PER_PRODUCER {
                    let mut value = who * PER_PRODUCER + k;
                    loop {
                        match producer.try_push(value) {
                            Ok(()) => break,
                            Err(PushError::Full(back)) => {
                                value = back;
                                thread::yield_now();
                            }
                            Err(PushError::OutOfMemory(_)) => {
                                panic!("bounded queue out of memory")
                            }
                        }
                    }
                }
            });
        }
        let takers: Vec<_> = (0..consumers)
            .map(|_| {
                scope.spawn(move || {
                    let consumer = queue.consumer().unwrap();
                    let mut mine = Vec::new();
                    while delivered.load(Ordering::SeqCst) < total as usize {
                        match consumer.try_pop() {
                            Some(value) => {
                                delivered.fetch_add(1, Ordering::SeqCst);
                                mine.push(value);
                            }
                            None => thread::yield_now(),
                        }
                    }
                    mine
                })
            })
            .collect();
        takers.into_iter().map(|t| t.join().unwrap()).collect()
    });

    for (consumer, mine) in seen.iter().enumerate() {
        for who in 0..producers {
            let of_theirs: Vec<u64> = mine
                .iter()
                .copied()
                .filter(|v| v / PER_PRODUCER == who)
                .collect();
            assert!(
                of_theirs.windows(2).all(|pair| pair[0] < pair[1]),
                "consumer {consumer} saw producer {who}'s values out of order: {of_theirs:?}"
            );
        }
    }
    let mut every: Vec<u64> = seen.into_iter().flatten().collect();
    every.sort_unstable();
    assert_eq!(
        every,
        (0..total).collect::<Vec<_>>(),
        "every value comes out exactly once"
    );
}

#[test]
fn scq_delivers_every_value_once_to_several_consumers() {
    scq_delivers_every_value_once::<3, 4>(2, 2);
    // A ring of two for two producers and two consumers: a ring has to be at
    // least as large as the number of participants working it, so the
    // capacity of one rides on a ring of two.
    scq_delivers_every_value_once::<1, 2>(2, 2);
    scq_delivers_every_value_once::<5, 8>(3, 2);
}

#[test]
fn scq_with_one_producer_and_many_consumers_keeps_the_producers_order() {
    scq_delivers_every_value_once::<4, 4>(1, 3);
}

#[test]
fn scq_with_many_producers_and_one_consumer_delivers_each_value_once() {
    scq_delivers_every_value_once::<4, 4>(3, 1);
}

/// An element whose destruction is counted, so a run can say how many were
/// destroyed.
struct Counted(Arc<AtomicUsize>);

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// A queue dropped with elements still in it destroys each of them once, and
/// an element that was popped is destroyed by whoever popped it and not again
/// by the queue.
#[test]
fn scq_destroys_each_element_exactly_once_whether_popped_or_left() {
    for _ in 0..REPEATS {
        let destroyed = Arc::new(AtomicUsize::new(0));
        let pushed = AtomicUsize::new(0);
        let refused = AtomicUsize::new(0);
        let popped = AtomicUsize::new(0);
        {
            let queue = Scq::<Counted, 5, 8>::new();
            let queue = &queue;
            let (destroyed, pushed, refused, popped) = (&destroyed, &pushed, &refused, &popped);
            thread::scope(|scope| {
                for _ in 0..2 {
                    scope.spawn(move || {
                        let producer = queue.producer().unwrap();
                        for _ in 0..PER_PRODUCER.min(40) {
                            // A refused push hands its element back, and the
                            // result is dropped here, destroying it.
                            match producer.try_push(Counted(destroyed.clone())) {
                                Ok(()) => pushed.fetch_add(1, Ordering::SeqCst),
                                Err(_) => refused.fetch_add(1, Ordering::SeqCst),
                            };
                        }
                    });
                }
                scope.spawn(move || {
                    let consumer = queue.consumer().unwrap();
                    for _ in 0..PER_PRODUCER.min(30) {
                        if consumer.try_pop().is_some() {
                            popped.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                });
            });
            let pushed = pushed.load(Ordering::SeqCst);
            let popped = popped.load(Ordering::SeqCst);
            assert_eq!(
                destroyed.load(Ordering::SeqCst),
                popped + refused.load(Ordering::SeqCst),
                "while the queue is alive only popped and refused elements are destroyed"
            );
            assert!(popped <= pushed, "nothing comes out that did not go in");
        }
        assert_eq!(
            destroyed.load(Ordering::SeqCst),
            pushed.load(Ordering::SeqCst) + refused.load(Ordering::SeqCst),
            "dropping the queue destroys exactly what was left: every element once"
        );
    }
}

/// A ring of `R` slots hands out `R` producer places and `R` consumer places,
/// the `R + 1`th request of either is refused, a clone takes a place too, and
/// a place comes back when its handle is dropped. The two sides count
/// separately. The algorithm's empty test is justified for no more
/// participants on a side than the ring has slots, and loses a pushed element
/// beyond that (`scq.rs`), so the queue is what keeps the count.
#[test]
fn scq_hands_out_no_more_places_than_its_ring_has_slots() {
    let queue = Scq::<u8, 3, 4>::new();
    let mut producers: Vec<_> = (0..4).map(|_| queue.producer()).collect();
    assert!(
        producers.iter().all(Option::is_some),
        "four places on a ring of four"
    );
    assert!(
        queue.producer().is_none(),
        "a fifth producer on a ring of four"
    );
    assert!(
        producers[0].as_ref().unwrap().try_clone().is_none(),
        "a clone takes a place too"
    );
    let mut consumers: Vec<_> = (0..4).map(|_| queue.consumer()).collect();
    assert!(consumers.iter().all(Option::is_some));
    assert!(
        queue.consumer().is_none(),
        "a fifth consumer on a ring of four"
    );

    // A refused request leaves the count as it was: asking again and again
    // does not use up a place that was never given.
    for _ in 0..10 {
        assert!(queue.producer().is_none());
        assert!(queue.consumer().is_none());
    }

    producers.pop();
    assert!(
        queue.producer().is_some(),
        "a dropped handle gives its place back"
    );
    assert!(
        queue.consumer().is_none(),
        "the consumer side is unaffected by the producer side"
    );
    consumers.clear();
    assert!(queue.consumer().is_some());
    assert_eq!(producers.len(), 3);
}

// ─── The `segmented` queues ───

use std::alloc::Layout;
use std::ptr::NonNull;

use sce_forge_runtime::queue::allocator::{Progress, SegmentAllocator};
use sce_forge_runtime::queue::hazard::HazardDomain;
use sce_forge_runtime::queue::linked::LinkedLamport;
use sce_forge_runtime::queue::lscq::Lscq;

/// The system allocator, counting the blocks it has out, so a run can say
/// whether a queue gave every segment back. Under Miri a block that was never
/// given back is also reported as a leak, which is the same check from the
/// other side.
struct Counting {
    live: AtomicUsize,
}

impl Counting {
    const fn new() -> Self {
        Self {
            live: AtomicUsize::new(0),
        }
    }
}

// SAFETY: blocks come from the system allocator with the layout asked for and go
// back to it with the same; the count is atomic. The system allocator can block,
// which is what `PROGRESS` says.
unsafe impl SegmentAllocator for Counting {
    const PROGRESS: Progress = Progress::Blocking;

    fn allocate(&self, layout: Layout) -> Option<NonNull<u8>> {
        // SAFETY: a segment's layout is never zero-sized.
        let block = NonNull::new(unsafe { std::alloc::alloc(layout) })?;
        self.live.fetch_add(1, Ordering::SeqCst);
        Some(block)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        // SAFETY: the caller's contract: `ptr` came from `allocate` with `layout`.
        unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) };
        self.live.fetch_sub(1, Ordering::SeqCst);
    }
}

#[test]
fn linked_lamport_delivers_every_value_in_order_across_segments() {
    let allocator = Counting::new();
    {
        let queue = LinkedLamport::<u64, 3, Counting, 0>::new(&allocator).unwrap();
        let (producer, consumer) = (queue.producer().unwrap(), queue.consumer().unwrap());
        thread::scope(|scope| {
            scope.spawn(move || {
                for value in 0..PER_PRODUCER {
                    producer.try_push(value).unwrap();
                }
            });
            scope.spawn(move || {
                let mut expected = 0;
                while expected < PER_PRODUCER {
                    match consumer.try_pop() {
                        Some(value) => {
                            assert_eq!(value, expected, "values leave in the order they came");
                            expected += 1;
                        }
                        None => thread::yield_now(),
                    }
                }
            });
        });
    }
    assert_eq!(
        allocator.live.load(Ordering::SeqCst),
        0,
        "every segment went back"
    );
}

/// Producers and consumers of an LSCQ queue, every value delivered once, and
/// each producer's values seen by each consumer in the order that producer
/// pushed them. Segments of `N`, so a run is many segments, each closed, linked,
/// drained and retired while the others run.
fn lscq_delivers_every_value_once<const N: usize, const R: usize>(
    producers: u64,
    consumers: usize,
) {
    let allocator = Counting::new();
    let domain = HazardDomain::<8>::new();
    let total = producers * PER_PRODUCER;
    let delivered = AtomicUsize::new(0);
    let seen: Vec<Vec<u64>> = {
        let queue = Lscq::<u64, N, R, 8, Counting, 0>::new(&allocator, &domain).unwrap();
        let (queue, delivered) = (&queue, &delivered);
        thread::scope(|scope| {
            for who in 0..producers {
                scope.spawn(move || {
                    let producer = queue.producer().unwrap();
                    for k in 0..PER_PRODUCER {
                        producer.try_push(who * PER_PRODUCER + k).unwrap();
                    }
                });
            }
            let takers: Vec<_> = (0..consumers)
                .map(|_| {
                    scope.spawn(move || {
                        let consumer = queue.consumer().unwrap();
                        let mut mine = Vec::new();
                        while delivered.load(Ordering::SeqCst) < total as usize {
                            match consumer.try_pop() {
                                Some(value) => {
                                    delivered.fetch_add(1, Ordering::SeqCst);
                                    mine.push(value);
                                }
                                None => thread::yield_now(),
                            }
                        }
                        mine
                    })
                })
                .collect();
            takers.into_iter().map(|t| t.join().unwrap()).collect()
        })
    };

    for (consumer, mine) in seen.iter().enumerate() {
        for who in 0..producers {
            let of_theirs: Vec<u64> = mine
                .iter()
                .copied()
                .filter(|v| v / PER_PRODUCER == who)
                .collect();
            assert!(
                of_theirs.windows(2).all(|pair| pair[0] < pair[1]),
                "consumer {consumer} saw producer {who}'s values out of order: {of_theirs:?}"
            );
        }
    }
    let mut every: Vec<u64> = seen.into_iter().flatten().collect();
    every.sort_unstable();
    assert_eq!(
        every,
        (0..total).collect::<Vec<_>>(),
        "every value comes out exactly once"
    );
    assert_eq!(
        allocator.live.load(Ordering::SeqCst),
        0,
        "every segment went back, the retired ones included"
    );
    assert_eq!(
        domain.waiting(),
        0,
        "the domain freed every retired segment"
    );
}

#[test]
fn lscq_delivers_every_value_once_to_several_consumers() {
    lscq_delivers_every_value_once::<2, 4>(2, 2);
    lscq_delivers_every_value_once::<4, 4>(3, 3);
}

#[test]
fn lscq_with_one_producer_and_many_consumers_keeps_the_producers_order() {
    lscq_delivers_every_value_once::<2, 4>(1, 3);
}

#[test]
fn lscq_with_many_producers_and_one_consumer_delivers_each_value_once() {
    lscq_delivers_every_value_once::<2, 4>(3, 1);
}

/// A segmented queue dropped with elements still in it, across segments and
/// among the retired ones, destroys each of them once and gives every segment
/// back; an element that was popped is destroyed by whoever popped it and not
/// again by the queue.
#[test]
fn a_dropped_segmented_queue_destroys_each_element_once_and_frees_every_segment() {
    for _ in 0..REPEATS {
        let destroyed = Arc::new(AtomicUsize::new(0));
        let allocator = Counting::new();
        let domain = HazardDomain::<4>::new();
        let pushed = AtomicUsize::new(0);
        let popped = AtomicUsize::new(0);
        {
            let queue = Lscq::<Counted, 2, 2, 4, Counting, 0>::new(&allocator, &domain).unwrap();
            let queue = &queue;
            let (destroyed, pushed, popped) = (&destroyed, &pushed, &popped);
            thread::scope(|scope| {
                for _ in 0..2 {
                    scope.spawn(move || {
                        let producer = queue.producer().unwrap();
                        for _ in 0..PER_PRODUCER.min(40) {
                            assert!(
                                producer.try_push(Counted(destroyed.clone())).is_ok(),
                                "the allocator never refuses here"
                            );
                            pushed.fetch_add(1, Ordering::SeqCst);
                        }
                    });
                }
                scope.spawn(move || {
                    let consumer = queue.consumer().unwrap();
                    for _ in 0..PER_PRODUCER.min(30) {
                        if consumer.try_pop().is_some() {
                            popped.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                });
            });
            assert_eq!(
                destroyed.load(Ordering::SeqCst),
                popped.load(Ordering::SeqCst),
                "while the queue is alive only popped elements are destroyed"
            );
        }
        assert_eq!(
            destroyed.load(Ordering::SeqCst),
            pushed.load(Ordering::SeqCst),
            "dropping the queue destroys exactly what was left: every element once"
        );
        assert_eq!(
            allocator.live.load(Ordering::SeqCst),
            0,
            "every segment went back"
        );
        assert_eq!(domain.waiting(), 0);
    }
}

/// A segmented queue with many participants hands out `R` producer places and
/// `R` consumer places and no more than the domain has slots for, and a place and
/// a slot come back when a handle is dropped.
#[test]
fn lscq_hands_out_no_more_places_than_its_ring_and_its_domain_have() {
    let allocator = Counting::new();
    let domain = HazardDomain::<4>::new();
    let queue = Lscq::<u8, 2, 4, 4, Counting, 0>::new(&allocator, &domain).unwrap();
    let mut producers: Vec<_> = (0..3).map(|_| queue.producer()).collect();
    assert!(producers.iter().all(Option::is_some));
    let consumer = queue.consumer();
    assert!(
        consumer.is_some(),
        "three producers and one consumer: four slots"
    );
    assert!(
        queue.producer().is_none() && queue.consumer().is_none(),
        "the domain has four slots, all taken"
    );
    producers.pop();
    assert!(
        queue.consumer().is_some(),
        "a dropped handle gives its slot back"
    );
}
