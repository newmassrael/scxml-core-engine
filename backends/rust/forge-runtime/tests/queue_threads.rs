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
                let producer = queue.producer();
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
                    let consumer = queue.consumer();
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
    scq_delivers_every_value_once::<1, 1>(2, 2);
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
                        let producer = queue.producer();
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
                    let consumer = queue.consumer();
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
