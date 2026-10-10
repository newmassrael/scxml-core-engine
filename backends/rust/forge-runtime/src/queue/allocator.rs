// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The allocator a `segmented` queue is injected with (SCE Protocol-Synthesis
//! RFC §synth-5-P, *Storage modes*): the one thing that bounds how much such a
//! queue holds.
//!
//! The crate is `no_std` and does not link `alloc`, so the allocator is a trait
//! of its own rather than the standard library's. It hands out and takes back
//! blocks of a [`Layout`], which is all a segment needs, and it states the
//! progress its own operations give as an associated constant, because the
//! progress check of a queue document runs at build time and the allocator
//! arrives only at run time (RFC §synth-5-P, *Segment allocator progress is
//! declared in the document*). The generated queue holds the allocator to what
//! the document declared with a compile-time assertion on that constant.

use core::alloc::Layout;
use core::ptr::NonNull;

/// What an operation guarantees about how long it takes whatever the other
/// participants are doing. Ordered by strength: a stronger guarantee compares
/// greater, so "at least `LockFree`" is `>= LockFree`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Progress {
    /// May wait for another participant.
    Blocking = 0,
    /// Some participant's operation completes in a bounded number of steps.
    LockFree = 1,
    /// Every operation completes in a bounded number of steps.
    WaitFree = 2,
}

impl Progress {
    /// The rank as a `u8`, for a const generic parameter (an enum cannot be
    /// one on stable): `Blocking` 0, `LockFree` 1, `WaitFree` 2.
    pub const fn rank(self) -> u8 {
        self as u8
    }
}

/// The allocator refused a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocError;

/// An allocator a `segmented` queue takes its segments from.
///
/// # Safety
///
/// An implementation must return, for `allocate(layout)`, either `None` or a
/// block of at least `layout.size()` bytes aligned to `layout.align()` that no
/// other live block overlaps, valid until it is passed to `deallocate` with the
/// same layout; and `PROGRESS` must be what its operations give. Operations
/// are called from several threads at once, so the implementation is `Sync`
/// in effect.
pub unsafe trait SegmentAllocator {
    /// The progress `allocate` and `deallocate` give. The queue's push is no
    /// stronger than this.
    const PROGRESS: Progress;

    /// A block for `layout`, or `None` when none can be given. A refusal is
    /// the queue's `OutOfMemory`.
    fn allocate(&self, layout: Layout) -> Option<NonNull<u8>>;

    /// Give a block back.
    ///
    /// # Safety
    ///
    /// `ptr` came from `allocate` on this allocator with this `layout`, and has
    /// not been given back.
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);
}

/// A block from `allocator` holding `make()`, or `None` when it refuses. `make`
/// runs only once the block is there, so a refusal builds nothing.
pub(crate) fn place<A: SegmentAllocator, S>(
    allocator: &A,
    make: impl FnOnce() -> S,
) -> Option<NonNull<S>> {
    let block = allocator.allocate(Layout::new::<S>())?.cast::<S>();
    // SAFETY: the allocator's contract gives a block of `S`'s size and
    // alignment that nothing else uses.
    unsafe { block.as_ptr().write(make()) };
    Some(block)
}

/// Drop the `S` at `block` and give its block back.
///
/// # Safety
///
/// `block` came from [`place`] on this allocator and holds a live `S` nothing
/// else reaches; it is released once.
pub(crate) unsafe fn release<A: SegmentAllocator, S>(allocator: &A, block: NonNull<S>) {
    // SAFETY: the caller's contract.
    unsafe {
        core::ptr::drop_in_place(block.as_ptr());
        allocator.deallocate(block.cast(), Layout::new::<S>());
    }
}
