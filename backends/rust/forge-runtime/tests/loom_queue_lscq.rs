// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Loom models of the `segmented` queues (SCE Protocol-Synthesis RFC
//! §synth-5-P, verification layer 3): the linked Lamport rings for one producer
//! and one consumer, and LSCQ, the list of SCQ rings, with its hazard-pointer
//! domain.
//!
//! Loom runs each model under every interleaving of its threads and every
//! outcome the C11 memory model allows their atomics, and fails the model when
//! an access to an element's slot is not ordered after the access that last
//! wrote it. What these models add to the SCQ ones (`loom_queue_scq.rs`) is the
//! list around the rings: that a segment is linked behind another with its first
//! element already in it and no element is lost across the link; that a closed
//! segment is drained before it is retired, so an element still arriving in it is
//! found; and that a segment a consumer retires while a producer is using it is
//! not freed under the producer. The race of two producers to link a successor
//! is in `loom_queue_lscq_race.rs`, which has a file of its own because exploring
//! it takes minutes where these models take well under a second.
//!
//! **Safety, not progress**, for the reason `loom_queue_scq.rs` gives: no model
//! waits for another thread, each thread makes its attempts once, and what is
//! left in the queue after the threads have joined is drained and counted. What a
//! model asserts therefore holds under every schedule: an element comes out at
//! most once and never changes, none is lost, the order each consumer sees is the
//! order the elements went in, and every segment goes back to the allocator.
//!
//! **Bounded, and said so.** Each model runs under three preemptions, exhaustively
//! within them. What a passing model establishes is "no violation within three
//! preemptions", not "no violation"; the recorded histories and the sanitizers
//! cover the rest, and the casefile
//! `sce-build/tests/mutations/a_segmented_queue_orders_every_segment_hand_over.cases`
//! weakens the hand-overs one at a time and requires these models to turn red.
//!
//! The segment holds one element, so every second push links a segment and the
//! models reach the list's code in a few operations. The domain's `collect` is
//! called where a retirement should be freed: the scan that frees would otherwise
//! run only after twice the slots are waiting, which a model this small never
//! reaches.
//!
//! The target has `test = false` and builds only under `--cfg loom`
//! (`backends/rust/forge-runtime/Cargo.toml` gives the command). Built without it,
//! it refuses to compile, so a file that checked nothing cannot pass.

#[cfg(not(loom))]
compile_error!(
    "the loom models run only under `--cfg loom`; see backends/rust/forge-runtime/Cargo.toml"
);

#[cfg(loom)]
mod loom_support;

#[cfg(loom)]
mod models {
    use loom::sync::atomic::Ordering;
    use loom::sync::Arc;
    use loom::thread;
    use sce_forge_runtime::queue::linked::LinkedLamport;

    use super::loom_support::{
        drain, leaked_allocator, leaked_domain, model, offer_switch, queue_over, Counting,
    };

    // ─── Linked Lamport rings ───

    /// One producer pushes two elements through segments of one and one consumer
    /// pops. The second push links a segment behind the first, and the pop may meet
    /// the link at any step of it: it finds the first element, or the first and the
    /// second, or neither yet, and an element it finds is the one written.
    #[test]
    fn linked_a_pushed_element_crosses_a_segment_to_a_pop() {
        model(|| {
            let allocator = leaked_allocator();
            let queue = Arc::new(
                LinkedLamport::<usize, 1, Counting, 0>::new(allocator).expect("first segment"),
            );
            let far = queue.clone();
            let pushing = thread::spawn(move || {
                let producer = far.producer().unwrap();
                assert!(producer.try_push(1).is_ok());
                assert!(producer.try_push(2).is_ok());
            });

            let consumer = queue.consumer().unwrap();
            offer_switch();
            let mut seen: Vec<usize> = Vec::new();
            seen.extend(consumer.try_pop());
            seen.extend(consumer.try_pop());
            pushing.join().unwrap();
            while let Some(element) = consumer.try_pop() {
                seen.push(element);
            }
            assert_eq!(seen, [1, 2], "both come out once, in order, unchanged");
            drop(consumer);
            drop(queue);
            assert_eq!(allocator.live.load(Ordering::SeqCst), 0);
        });
    }

    // ─── LSCQ ───

    /// One producer pushes two elements through segments of one and one consumer
    /// pops. The second push closes the first segment and links a successor holding
    /// the element, and the pop meets the close, the link and the retirement at any
    /// step: it may find nothing, the first element, or both, and the order and the
    /// content hold under every schedule.
    #[test]
    fn lscq_a_pushed_element_crosses_a_segment_to_a_pop() {
        model(|| {
            let allocator = leaked_allocator();
            let domain = leaked_domain::<4>();
            let queue = queue_over(allocator, domain);
            let far = queue.clone();
            let pushing = thread::spawn(move || {
                let producer = far.producer().unwrap();
                assert!(producer.try_push(1).is_ok());
                assert!(producer.try_push(2).is_ok());
            });

            let consumer = queue.consumer().unwrap();
            let mut seen: Vec<usize> = Vec::new();
            seen.extend(consumer.try_pop());
            seen.extend(consumer.try_pop());
            domain.collect();
            pushing.join().unwrap();
            seen.extend(drain(&consumer));
            assert_eq!(seen, [1, 2], "both come out once, in order, unchanged");
            drop(consumer);
            drop(queue);
            assert_eq!(
                allocator.live.load(Ordering::SeqCst),
                0,
                "every segment went back"
            );
        });
    }

    /// A consumer drains the first segment, moves `head` past it and retires it,
    /// and frees what it can, while a producer is pushing into the segment behind.
    /// The producer holds a hazard on what it uses, so a segment it is still in is
    /// not freed under it, and what it pushes is found.
    #[test]
    fn lscq_a_retired_segment_is_not_freed_under_a_producer() {
        model(|| {
            let allocator = leaked_allocator();
            let domain = leaked_domain::<4>();
            let queue = queue_over(allocator, domain);
            assert!(queue.producer().unwrap().try_push(0).is_ok());
            assert!(queue.producer().unwrap().try_push(1).is_ok());
            let far = queue.clone();
            let pushing = thread::spawn(move || {
                assert!(far.producer().unwrap().try_push(2).is_ok());
            });

            let consumer = queue.consumer().unwrap();
            let mut seen: Vec<usize> = Vec::new();
            seen.extend(consumer.try_pop());
            seen.extend(consumer.try_pop());
            domain.collect();
            pushing.join().unwrap();
            seen.extend(drain(&consumer));
            assert_eq!(seen, [0, 1, 2], "all three come out once, in order");
            drop(consumer);
            drop(queue);
            assert_eq!(
                allocator.live.load(Ordering::SeqCst),
                0,
                "every segment went back"
            );
        });
    }
}
