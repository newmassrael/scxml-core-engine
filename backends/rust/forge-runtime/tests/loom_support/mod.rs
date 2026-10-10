// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the loom models of the `segmented` queues share: the bound every model
//! runs under, an allocator that counts the segments it has out, and the LSCQ
//! shape the models build over it.
//!
//! The models live in more than one file because their costs differ by orders of
//! magnitude, and a mutation casefile runs the whole of a file as its baseline
//! (`scripts/mutate` bounds one test binary in wall-clock time). Keeping what
//! they share here means the bound is stated once and a file that is split off
//! for its cost does not carry a second copy of it.
//!
//! Included only under `--cfg loom`, by the model files, which refuse to compile
//! without it.

use std::alloc::Layout;
use std::ptr::NonNull;

use loom::sync::atomic::{AtomicUsize, Ordering};
use sce_forge_runtime::queue::allocator::{Progress, SegmentAllocator};
use sce_forge_runtime::queue::hazard::HazardDomain;
use sce_forge_runtime::queue::lscq::{Consumer, Lscq};

/// Run `body` under every interleaving within three preemptions, and every
/// outcome the memory model allows the atomics in it. A passing model
/// establishes "no violation within three preemptions", which each model file
/// says in its own module documentation.
pub fn model(body: impl Fn() + Sync + Send + 'static) {
    let mut builder = loom::model::Builder::new();
    builder.preemption_bound = Some(3);
    builder.check(body);
}

/// Offer loom a thread switch here, which it would otherwise not try.
///
/// Loom decides which interleavings to run by noting, per atomic, the last
/// access to it, and queues a switch only where another thread's pending access
/// conflicts with that one. A thread that loads an atomic and then stores it
/// leaves its own load as the last access, which hides every access another
/// thread made before it, and the switch that would have put the store ahead of
/// those accesses is never queued. The linked ring's producer does exactly
/// that with its filled count. Measured 2026-10-10 on loom 0.7.2, a model of one
/// push and one pop ran one execution and passed with the count published
/// `Relaxed`, where a yield in the consumer ran several and failed it.
///
/// So a model whose threads race on such an atomic yields where the threads
/// begin to race, once at the head of the side that only reads. That adds the
/// switches without changing what the model asserts.
pub fn offer_switch() {
    loom::thread::yield_now();
}

/// The system allocator, counting the blocks it has out so a model can say
/// that the queue gave every segment back.
pub struct Counting {
    pub live: AtomicUsize,
}

impl Counting {
    fn new() -> Self {
        Self {
            live: AtomicUsize::new(0),
        }
    }
}

// SAFETY: blocks come from the system allocator with the layout asked for and
// go back to it with the same; the count is atomic. The system allocator can
// block, which is what `PROGRESS` says.
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

/// Loom builds the queue afresh in every execution, and a queue borrows its
/// allocator and its domain; both are leaked for the one execution, which is
/// a few words, so that the queue can be shared by `Arc` between threads.
pub fn leaked_allocator() -> &'static Counting {
    Box::leak(Box::new(Counting::new()))
}

pub fn leaked_domain<const H: usize>() -> &'static HazardDomain<H> {
    Box::leak(Box::new(HazardDomain::<H>::new()))
}

/// Segments of one element over a ring of two, a hazard domain of four slots,
/// a producer and a consumer place on each side.
pub type Queue = Lscq<'static, usize, 1, 2, 4, Counting, 0>;

pub fn queue_over(
    allocator: &'static Counting,
    domain: &'static HazardDomain<4>,
) -> loom::sync::Arc<Queue> {
    loom::sync::Arc::new(Queue::new(allocator, domain).expect("first segment"))
}

/// Whatever is left in the queue, oldest first. Run after every thread has
/// joined, so nothing else touches the queue and the loop ends.
pub fn drain(consumer: &Consumer<'_, '_, usize, 1, 2, 4, Counting, 0>) -> Vec<usize> {
    let mut left = Vec::new();
    while let Some(element) = consumer.try_pop() {
        left.push(element);
    }
    left
}
