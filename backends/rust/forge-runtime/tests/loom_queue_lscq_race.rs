// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The loom model of two LSCQ producers racing to link a successor segment (SCE
//! Protocol-Synthesis RFC §synth-5-P, verification layer 3), apart from the other
//! LSCQ models in `loom_queue_lscq.rs`.
//!
//! It has a file of its own for its cost. Three threads, two producers and a
//! consumer making two pops, give loom some six million executions within three
//! preemptions, which took 547 s on the build machine (measured 2026-10-10),
//! where the models of `loom_queue_lscq.rs` together take well under a second. A
//! mutation casefile runs the whole of a test binary as its baseline and bounds
//! one binary in wall-clock time, so the casefile about the other models does not
//! name this file, and the bound is not lowered to fit the clock.
//!
//! Two producers find the first segment full and race to link a successor, each
//! with its own element already in the segment it offers. One wins; the other
//! frees its segment and pushes behind the winner's. The model asserts what
//! `loom_queue_lscq.rs` does: an element comes out once and unchanged, none is
//! lost, the first element is first, and every segment, the loser's included,
//! goes back to the allocator. Safety, not progress, and bounded to three
//! preemptions, for the reasons that file gives.
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
    use loom::thread;

    use super::loom_support::{drain, leaked_allocator, leaked_domain, model, queue_over};

    /// Two producers find the first segment full and race to link a successor,
    /// each with its own element already in the segment it offers. One wins; the
    /// other frees its segment and pushes behind the winner's. Neither element is
    /// lost or duplicated, and every segment, the loser's included, goes back.
    #[test]
    fn lscq_two_producers_racing_to_link_lose_nothing() {
        model(|| {
            let allocator = leaked_allocator();
            let domain = leaked_domain::<4>();
            let queue = queue_over(allocator, domain);
            assert!(
                queue.producer().unwrap().try_push(0).is_ok(),
                "the first segment is now full"
            );
            let (first, second) = (queue.clone(), queue.clone());
            let a = thread::spawn(move || {
                assert!(first.producer().unwrap().try_push(1).is_ok());
            });
            let b = thread::spawn(move || {
                assert!(second.producer().unwrap().try_push(2).is_ok());
            });

            // Two pops: the first takes the element the segment held, and the
            // second can meet its close, its link and its retirement and go on to
            // the segment behind, which is what a producer still pushing into the
            // first must not be left behind by.
            let consumer = queue.consumer().unwrap();
            let mut seen: Vec<usize> = Vec::new();
            seen.extend(consumer.try_pop());
            seen.extend(consumer.try_pop());
            a.join().unwrap();
            b.join().unwrap();
            seen.extend(drain(&consumer));
            assert_eq!(seen.first(), Some(&0), "the first element is first");
            let mut rest = seen[1..].to_vec();
            rest.sort_unstable();
            assert_eq!(rest, [1, 2], "each of the others comes out once");
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
