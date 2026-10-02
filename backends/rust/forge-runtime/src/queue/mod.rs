// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Queues that hand elements from one execution context to another — the
//! runtime half of `sce:kind="queue"` (SCE Protocol-Synthesis RFC
//! §synth-5-P).
//!
//! The document states a contract and names no algorithm; each algorithm
//! the kind's selection table names lives here once. The contract every one
//! of them keeps: linearizable FIFO in which the push of an element
//! happens-before the pop that returns it, at the exact capacity declared.
//! Memory ordering is not a parameter — it is part of each algorithm.
//!
//! Modules:
//! - [`spsc`] — the `bounded` row for one producer and one consumer: a
//!   Lamport ring, wait-free on both sides.
//!
//! No allocation, no global state, no threads (SCE_FORGE.md §2.1, C1/C2):
//! a queue owns its storage, and where it lives is the caller's choice.

pub mod spsc;

mod sync;

/// Why a push did not take its element. The element comes back, so the
/// caller decides what happens to it.
///
/// One type for every storage mode, so generated code handles one error
/// whatever the document chose. A `bounded` queue only ever reports
/// [`PushError::Full`]; [`PushError::OutOfMemory`] is the `segmented`
/// storage mode's, reported when its injected allocator refuses a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushError<T> {
    /// The queue held its declared capacity.
    Full(T),
    /// A `segmented` queue's allocator refused a new segment.
    OutOfMemory(T),
}

impl<T> PushError<T> {
    /// The element the push did not take.
    pub fn into_inner(self) -> T {
        match self {
            PushError::Full(value) | PushError::OutOfMemory(value) => value,
        }
    }
}
