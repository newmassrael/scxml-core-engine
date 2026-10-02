// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Loom models of the one-producer, one-consumer bounded queue (SCE
//! Protocol-Synthesis RFC §synth-5-P, verification layer 3).
//!
//! Loom runs each model under every interleaving of the two threads and
//! every outcome the C11 memory model allows their atomics, and fails the
//! model when an access to a slot is not ordered after the access that
//! last wrote it. So these models do not sample the ordering the module
//! documents — they check it: weaken one of the queue's Release stores or
//! Acquire loads and a model here fails (the casefile
//! `sce-build/tests/mutations/an_spsc_queue_orders_every_slot_hand_over.cases`
//! does exactly that, one ordering at a time).
//!
//! The target has `test = false` and builds only under `--cfg loom`
//! (`backends/rust/forge-runtime/Cargo.toml` gives the command). Built
//! without it, it refuses to compile: a model file that compiled to
//! nothing would pass, and a pass from a file that checked nothing is the
//! one result this layer must never give.

#[cfg(not(loom))]
compile_error!(
    "the loom models run only under `--cfg loom`; see backends/rust/forge-runtime/Cargo.toml"
);

#[cfg(loom)]
mod models {
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::sync::Arc;
    use loom::thread;
    use sce_forge_runtime::queue::spsc::Spsc;
    use sce_forge_runtime::queue::PushError;

    /// Push `value`, yielding to the other thread while the ring is full.
    fn push_until_taken<T, const N: usize>(
        producer: &mut sce_forge_runtime::queue::spsc::Producer<'_, T, N>,
        mut value: T,
    ) {
        loop {
            match producer.try_push(value) {
                Ok(()) => return,
                Err(PushError::Full(back)) => {
                    value = back;
                    thread::yield_now();
                }
                Err(PushError::OutOfMemory(_)) => {
                    panic!("a bounded queue reported OutOfMemory, which only segmented storage may")
                }
            }
        }
    }

    /// Three elements through a ring of one: every push after the first
    /// waits for the pop before it, and the lap index wraps, so both
    /// hand-over directions and the wrap run under every interleaving.
    #[test]
    fn elements_arrive_in_order_through_a_ring_of_one() {
        loom::model(|| {
            let queue = Arc::new(Spsc::<usize, 1>::new());
            let far = queue.clone();
            let producer = thread::spawn(move || {
                // SAFETY: this is the only producer of the queue.
                let mut producer = unsafe { far.producer() };
                for value in 0..3 {
                    push_until_taken(&mut producer, value);
                }
            });

            // SAFETY: this is the only consumer of the queue.
            let mut consumer = unsafe { queue.consumer() };
            let mut expected = 0;
            while expected < 3 {
                match consumer.try_pop() {
                    Some(value) => {
                        assert_eq!(
                            value, expected,
                            "elements must leave in the order they came"
                        );
                        expected += 1;
                    }
                    None => thread::yield_now(),
                }
            }
            assert_eq!(
                consumer.try_pop(),
                None,
                "nothing comes out that was not pushed"
            );
            producer.join().unwrap();
        });
    }

    /// An element whose drop is counted, so a model can say how many were
    /// destroyed and by whom.
    struct Counted(Arc<AtomicUsize>);

    impl Drop for Counted {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Elements still queued when the queue is dropped are dropped with it,
    /// exactly once, and the drop reads slots the producer wrote on another
    /// thread.
    #[test]
    fn a_dropped_queue_drops_what_it_still_holds() {
        loom::model(|| {
            let dropped = Arc::new(AtomicUsize::new(0));
            let queue = Arc::new(Spsc::<Counted, 2>::new());
            let far = queue.clone();
            let far_dropped = dropped.clone();
            let producer = thread::spawn(move || {
                // SAFETY: this is the only producer of the queue.
                let mut producer = unsafe { far.producer() };
                push_until_taken(&mut producer, Counted(far_dropped.clone()));
                push_until_taken(&mut producer, Counted(far_dropped));
            });

            {
                // SAFETY: this is the only consumer of the queue.
                let mut consumer = unsafe { queue.consumer() };
                loop {
                    if let Some(element) = consumer.try_pop() {
                        drop(element);
                        break;
                    }
                    thread::yield_now();
                }
            }
            producer.join().unwrap();
            assert_eq!(
                dropped.load(Ordering::Relaxed),
                1,
                "one element was popped and dropped"
            );

            let queue = Arc::try_unwrap(queue)
                .unwrap_or_else(|_| panic!("both handles are gone, so the queue has one owner"));
            drop(queue);
            assert_eq!(
                dropped.load(Ordering::Relaxed),
                2,
                "the element still queued is dropped with the queue, and only once"
            );
        });
    }
}
