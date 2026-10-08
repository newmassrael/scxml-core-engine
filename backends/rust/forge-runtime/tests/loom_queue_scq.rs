// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Loom models of the SCQ data queue, the `bounded` row for any cardinality
//! but one producer and one consumer (SCE Protocol-Synthesis RFC
//! §synth-5-P, verification layer 3).
//!
//! Loom runs each model under every interleaving of its threads and every
//! outcome the C11 memory model allows their atomics, and fails the model
//! when an access to an element's slot is not ordered after the access that
//! last wrote it. The queue's rings are the algorithm under test; the slots
//! are what proves a ring handed an index over correctly, because an index
//! handed over without the right ordering shows up as a slot read or written
//! with no happens-before edge to the other side's access. The casefile
//! `sce-build/tests/mutations/an_scq_queue_orders_every_index_hand_over.cases`
//! weakens the queue's orderings one at a time and requires these models to
//! turn red.
//!
//! **Safety, not progress.** No model waits for another thread. A consumer
//! that polls an empty queue until something arrives is a model whose
//! schedules include one in which the producer is never given the step it
//! needs, because SCQ is lock-free, not wait-free: some operation finishes,
//! not every one. Loom explores that schedule for ever. So each thread makes
//! its attempts once, whatever they return is an outcome the model accounts
//! for, and what is left in the queue after the threads have joined is
//! drained and counted. What a model asserts is therefore what holds under
//! every schedule: an element comes out at most once and never changes,
//! none is lost, the order a consumer sees is the order the elements went
//! in, a full queue refuses and nothing else does. That a waiting thread is
//! eventually served is layer 4's question.
//!
//! **Bounded, and said so.** An SCQ operation is a dozen atomic steps, so
//! three participants with an operation each already have more interleavings
//! than a model can enumerate. Each model runs under a bound of three
//! preemptions (the bound the context-bounding literature found enough for
//! almost every concurrency bug), exhaustively within it. What a passing
//! model establishes is therefore "no violation within three preemptions",
//! not "no violation"; layers 2 and 5 (recorded histories, ThreadSanitizer)
//! cover the rest, and the casefile is what shows that the bound still sees
//! each ordering.
//!
//! The target has `test = false` and builds only under `--cfg loom`
//! (`backends/rust/forge-runtime/Cargo.toml` gives the command). Built
//! without it, it refuses to compile: a model file that compiled to nothing
//! would pass, and a pass from a file that checked nothing is the one result
//! this layer must never give.

#[cfg(not(loom))]
compile_error!(
    "the loom models run only under `--cfg loom`; see backends/rust/forge-runtime/Cargo.toml"
);

#[cfg(loom)]
mod models {
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::sync::Arc;
    use loom::thread;
    use sce_forge_runtime::queue::scq::{Consumer, Scq};
    use sce_forge_runtime::queue::PushError;

    /// Run `body` under every interleaving within the preemption bound
    /// the module documentation states.
    fn model(body: impl Fn() + Sync + Send + 'static) {
        let mut builder = loom::model::Builder::new();
        builder.preemption_bound = Some(3);
        builder.check(body);
    }

    /// Whatever is left in the queue, oldest first. Run after every thread
    /// has joined, so nothing else touches the queue and the loop ends.
    fn drain<T, const N: usize, const R: usize>(consumer: &Consumer<'_, T, N, R>) -> Vec<T> {
        let mut left = Vec::new();
        while let Some(element) = consumer.try_pop() {
            left.push(element);
        }
        left
    }

    /// A queue of one that has already carried an element through it: the
    /// rings' cycles have advanced and their thresholds are set. The first
    /// element's hand-over is ordered by the threshold's sequentially
    /// consistent store and load, which every later one skips, so a model that
    /// starts from a fresh queue does not see the entries' own orderings and
    /// one that starts from this does.
    fn warmed_up_queue_of_one() -> Arc<Scq<usize, 1, 1>> {
        let queue = Arc::new(Scq::<usize, 1, 1>::new());
        assert!(queue.producer().unwrap().try_push(0).is_ok());
        assert_eq!(queue.consumer().unwrap().try_pop(), Some(0));
        queue
    }

    /// One push meets one pop on a warmed-up queue. The pop may find the
    /// queue empty or find the element; if it finds it, the element it reads
    /// is the one the push wrote, and the hand-over of the slot through the
    /// allocated ring is what orders the two accesses.
    #[test]
    fn a_pushed_element_reaches_a_pop() {
        model(|| {
            let queue = warmed_up_queue_of_one();
            let far = queue.clone();
            let pushing = thread::spawn(move || far.producer().unwrap().try_push(7).is_ok());

            let consumer = queue.consumer().unwrap();
            let popped = consumer.try_pop();
            assert!(pushing.join().unwrap(), "a push to an empty queue succeeds");

            let mut seen: Vec<usize> = popped.into_iter().collect();
            seen.extend(drain(&consumer));
            assert_eq!(seen, [7], "the element comes out once, and unchanged");
        });
    }

    /// A pop and a push meet over the one slot of a full queue of one. The
    /// push finds the queue still full, or finds the slot the pop returned
    /// through the free ring, and then its write into the slot is ordered after
    /// the pop's read of it by that ring, as the first use was by the other.
    #[test]
    fn a_popped_slot_is_reused_by_a_push() {
        model(|| {
            let queue = warmed_up_queue_of_one();
            assert!(queue.producer().unwrap().try_push(1).is_ok());
            let far = queue.clone();
            let pushing = thread::spawn(move || match far.producer().unwrap().try_push(2) {
                Ok(()) => true,
                Err(PushError::Full(back)) => {
                    assert_eq!(back, 2, "a refused push hands back its own element");
                    false
                }
                Err(PushError::OutOfMemory(_)) => panic!("a bounded queue ran out of memory"),
            });

            let consumer = queue.consumer().unwrap();
            let popped = consumer.try_pop();
            assert_eq!(popped, Some(1), "an element pushed before the pop is found");
            let pushed = pushing.join().unwrap();

            let left = drain(&consumer);
            assert_eq!(
                left,
                if pushed { vec![2] } else { Vec::new() },
                "what was pushed comes out once, and only what was pushed"
            );
        });
    }

    /// Two producers fill one queue of two. Both succeed whatever the
    /// order, and each element is delivered once.
    #[test]
    fn two_producers_deliver_each_element_once() {
        model(|| {
            let queue = Arc::new(Scq::<usize, 2, 2>::new());
            let first = {
                let far = queue.clone();
                thread::spawn(move || far.producer().unwrap().try_push(1).is_ok())
            };
            let second = {
                let far = queue.clone();
                thread::spawn(move || far.producer().unwrap().try_push(2).is_ok())
            };

            let consumer = queue.consumer().unwrap();
            let mut seen: Vec<usize> = consumer.try_pop().into_iter().collect();
            assert!(first.join().unwrap(), "the queue holds both elements");
            assert!(second.join().unwrap(), "the queue holds both elements");

            seen.extend(drain(&consumer));
            seen.sort_unstable();
            assert_eq!(seen, [1, 2], "each pushed element comes out exactly once");
        });
    }

    /// Two consumers share what one producer pushes: each element goes to
    /// one of them, none twice, and what each consumer sees is in the order
    /// the producer pushed it.
    #[test]
    fn two_consumers_split_the_elements_without_overlap() {
        model(|| {
            let queue = Arc::new(Scq::<usize, 2, 2>::new());
            let far = queue.clone();
            let pushing = thread::spawn(move || {
                let producer = far.producer().unwrap();
                let first = producer.try_push(1).is_ok();
                let second = producer.try_push(2).is_ok();
                first && second
            });

            let other = {
                let far = queue.clone();
                thread::spawn(move || {
                    let consumer = far.consumer().unwrap();
                    let mut seen: Vec<usize> = consumer.try_pop().into_iter().collect();
                    seen.extend(consumer.try_pop());
                    seen
                })
            };
            let consumer = queue.consumer().unwrap();
            let mut mine: Vec<usize> = consumer.try_pop().into_iter().collect();
            mine.extend(consumer.try_pop());
            let theirs = other.join().unwrap();
            assert!(pushing.join().unwrap(), "the queue holds both elements");

            for seen in [&mine, &theirs] {
                assert!(
                    seen.windows(2).all(|pair| pair[0] < pair[1]),
                    "a consumer sees the producer's order: {seen:?}"
                );
            }
            let mut every: Vec<usize> = mine;
            every.extend(theirs);
            every.extend(drain(&consumer));
            every.sort_unstable();
            assert_eq!(every, [1, 2], "each element reaches exactly one consumer");
        });
    }

    /// Two producers race for the one slot of a queue of one and nobody
    /// pops: exactly one wins and the other is told the queue is full, and
    /// the element the winner pushed is the one that comes out.
    #[test]
    fn a_queue_of_one_takes_exactly_one_of_two_racing_pushes() {
        model(|| {
            // Two producers, so a ring of two: a ring has to be at least as
            // large as the number of participants working it (`scq.rs`).
            let queue = Arc::new(Scq::<usize, 1, 2>::new());
            let rival = {
                let far = queue.clone();
                thread::spawn(move || far.producer().unwrap().try_push(2).is_ok())
            };
            let mine = queue.producer().unwrap().try_push(1).is_ok();
            let theirs = rival.join().unwrap();
            assert!(
                mine != theirs,
                "exactly one push takes the only slot (mine: {mine}, theirs: {theirs})"
            );

            let consumer = queue.consumer().unwrap();
            assert_eq!(
                drain(&consumer),
                [if mine { 1 } else { 2 }],
                "the element that came out is the one whose push succeeded"
            );
        });
    }

    /// An element whose drop is counted, so a model can say how many were
    /// destroyed.
    struct Counted(Arc<AtomicUsize>);

    impl Drop for Counted {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Elements still queued when the queue is dropped are dropped with it,
    /// exactly once, and the drop reads slots a producer wrote on another
    /// thread.
    #[test]
    fn a_dropped_queue_drops_what_it_still_holds() {
        model(|| {
            let dropped = Arc::new(AtomicUsize::new(0));
            let queue = Arc::new(Scq::<Counted, 2, 2>::new());
            let far = queue.clone();
            let far_dropped = dropped.clone();
            let pushing = thread::spawn(move || {
                let producer = far.producer().unwrap();
                let first = producer.try_push(Counted(far_dropped.clone())).is_ok();
                let second = producer.try_push(Counted(far_dropped)).is_ok();
                first && second
            });

            // A pop that finds an element drops it here; one that finds the
            // queue empty drops nothing.
            let popped = usize::from(queue.consumer().unwrap().try_pop().is_some());
            assert!(pushing.join().unwrap(), "the queue holds both elements");
            assert_eq!(
                dropped.load(Ordering::Relaxed),
                popped,
                "an element popped and dropped is dropped once"
            );

            let queue = Arc::try_unwrap(queue)
                .unwrap_or_else(|_| panic!("both handles are gone, so the queue has one owner"));
            drop(queue);
            assert_eq!(
                dropped.load(Ordering::Relaxed),
                2,
                "the elements still queued are dropped with the queue, and only once"
            );
        });
    }

    /// What one participant of a lapped model does, once.
    #[derive(Clone, Copy)]
    enum Role {
        Push(usize),
        Pop,
    }

    /// What it got: a push says whether the queue took the element, a pop
    /// what it found.
    enum Got {
        Pushed(usize, bool),
        Popped(Option<usize>),
    }

    fn play<const N: usize, const R: usize>(queue: &Scq<usize, N, R>, role: Role) -> Got {
        match role {
            Role::Push(value) => {
                Got::Pushed(value, queue.producer().unwrap().try_push(value).is_ok())
            }
            Role::Pop => Got::Popped(queue.consumer().unwrap().try_pop()),
        }
    }

    /// A queue that has already been through `laps` rounds of one push and one
    /// pop, so both of its rings have advanced their cycles and carry the marks
    /// earlier cycles left, then `held` elements pushed and kept, then
    /// `pushers` participants that each push one element and `poppers` that
    /// each pop once, all at the same time.
    ///
    /// The fresh queues above reach a ring's first cycle only. The states the
    /// algorithm's cycle rules exist for are the ones where an entry holds
    /// something from an earlier cycle, or a ticket's holder was overtaken by
    /// a whole lap of others, and those need a lap to have happened and more
    /// than one participant working one ring: two pushers are two dequeues of
    /// the free ring, two poppers two of the allocated ring.
    ///
    /// Safety only, as above: every element that went in and was not refused
    /// comes out exactly once, whoever took it, whether in a pop or in the
    /// drain after the threads have joined.
    fn lapped<const N: usize, const R: usize>(
        laps: usize,
        held: usize,
        pushers: usize,
        poppers: usize,
    ) {
        // The precondition of the algorithm (`scq.rs`): no more enqueuers or
        // dequeuers on a ring than it has slots. A shape that breaks it is
        // not a model of this queue, and the queue would refuse the handles.
        assert!(
            pushers <= R && poppers <= R,
            "{pushers} pushers and {poppers} poppers on a ring of {R}"
        );
        let queue = Arc::new(Scq::<usize, N, R>::new());
        for _ in 0..laps {
            assert!(queue.producer().unwrap().try_push(7_000).is_ok());
            assert_eq!(queue.consumer().unwrap().try_pop(), Some(7_000));
        }
        let mut expected: Vec<usize> = (0..held).map(|k| 1_000 + k).collect();
        for value in &expected {
            assert!(queue.producer().unwrap().try_push(*value).is_ok());
        }

        let mut roles: Vec<Role> = (0..pushers).map(|k| Role::Push(2_000 + k)).collect();
        roles.extend((0..poppers).map(|_| Role::Pop));
        let mine = roles.pop().expect("a model has a participant");
        let spawned: Vec<_> = roles
            .into_iter()
            .map(|role| {
                let far = queue.clone();
                thread::spawn(move || play(&far, role))
            })
            .collect();
        let mut gots = vec![play(&queue, mine)];
        gots.extend(spawned.into_iter().map(|t| t.join().unwrap()));

        let mut seen = Vec::new();
        for got in gots {
            match got {
                Got::Pushed(value, true) => expected.push(value),
                Got::Pushed(_, false) => {}
                Got::Popped(found) => seen.extend(found),
            }
        }
        seen.extend(drain(&queue.consumer().unwrap()));
        seen.sort_unstable();
        expected.sort_unstable();
        assert_eq!(
            seen, expected,
            "what went in and was not refused comes out once each, and nothing else does"
        );
    }

    /// One `#[test]` per shape: `laps` rounds first, `held` elements kept,
    /// `push` pushers and `pop` poppers.
    macro_rules! lapped_models {
        ($($name:ident: $n:literal, $r:literal, laps $laps:literal, held $held:literal,
           push $push:literal, pop $pop:literal;)*) => {$(
            #[test]
            fn $name() {
                model(|| lapped::<$n, $r>($laps, $held, $push, $pop));
            }
        )*};
    }

    // Every shape keeps its pushers and its poppers within its ring, which
    // `lapped` asserts. Three poppers on a ring of two, and two on a ring of
    // one, are the shapes where the element is lost (`scq.rs`, measured
    // 2026-10-09); the queue refuses the handles to build them.
    lapped_models! {
        lapped_n1_r1_laps1_held0_push1_pop1: 1, 1, laps 1, held 0, push 1, pop 1;
        lapped_n1_r1_laps3_held0_push1_pop1: 1, 1, laps 3, held 0, push 1, pop 1;
        lapped_n1_r1_laps2_held1_push1_pop1: 1, 1, laps 2, held 1, push 1, pop 1;
        lapped_n1_r2_laps1_held0_push2_pop1: 1, 2, laps 1, held 0, push 2, pop 1;
        lapped_n1_r2_laps2_held0_push1_pop2: 1, 2, laps 2, held 0, push 1, pop 2;
        lapped_n1_r2_laps2_held1_push2_pop1: 1, 2, laps 2, held 1, push 2, pop 1;
        lapped_n2_r2_laps1_held1_push2_pop1: 2, 2, laps 1, held 1, push 2, pop 1;
        lapped_n2_r2_laps2_held1_push2_pop1: 2, 2, laps 2, held 1, push 2, pop 1;
        lapped_n2_r2_laps2_held2_push1_pop2: 2, 2, laps 2, held 2, push 1, pop 2;
        lapped_n2_r2_laps1_held0_push1_pop2: 2, 2, laps 1, held 0, push 1, pop 2;
        lapped_n2_r2_laps2_held0_push1_pop2: 2, 2, laps 2, held 0, push 1, pop 2;
        lapped_n2_r2_laps3_held0_push1_pop2: 2, 2, laps 3, held 0, push 1, pop 2;
        lapped_n4_r4_laps2_held0_push1_pop2: 4, 4, laps 2, held 0, push 1, pop 2;
    }
}
