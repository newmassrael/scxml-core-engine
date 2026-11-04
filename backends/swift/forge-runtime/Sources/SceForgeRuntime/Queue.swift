// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The runtime half of `sce:kind="queue"` (SCE Protocol-Synthesis RFC section synth-5-P) for Swift: queues
// that hand elements from one thread to another.
//
// The document states a contract and names no algorithm; each algorithm the kind's selection table names
// lives here once. The contract every one of them keeps: a linearizable FIFO in which the push of an
// element happens-before the pop that returns it, never holding more than the capacity declared and
// holding exactly that when nothing is running.
//
// - `Spsc` is the `bounded` row for one producer and one consumer: a Lamport ring, wait-free on both
//   sides.
// - `Scq` is the `bounded` row for any other cardinality: Nikolaev's SCQ data queue (DISC 2019), lock-free
//   on both sides, in the form of the authors' single-width-CAS reference implementation (lfring_cas1.h,
//   dual 2-clause BSD / MIT). Swift's atomics have a fetch-or, which the reference's `fetch_or` is, so no
//   compare-and-swap loop stands in for it.
//
// Siblings, rule for rule: `Spsc` (Kotlin, `backends/kotlin/forge-runtime/.../queue/Spsc.kt`) and the
// Rust, C++ and Go runtimes. The atomics are `Synchronization`'s, with the orderings the algorithm needs
// and no stronger.
//
// What differs from the languages with destructors and constant generics:
//
// - The capacity is an argument of the initializer. The storage is one allocation, made by the
//   initializer and never again: no operation allocates.
// - A popped element's slot is cleared, so a queue holds no copy of an element that has left it.
// - A handle holds one place of its side until it is released. Releasing is `release()` or the end of the
//   handle's life (a handle is a class, and its deinitializer gives the place back), so a handle that is
//   dropped does not strand its side.
//
// A refused push needs no hand-back: `tryPush` takes its element by value, so the caller still holds it.

import Synchronization

/// Takes one of `limit` places of a side, if one is free. A compare-and-swap loop and not an add, so a
/// refused request leaves the count as it was and no concurrent request can be refused on account of it.
func takePlace(_ alive: borrowing Atomic<Int>, limit: Int) -> Bool {
    var seen = alive.load(ordering: .sequentiallyConsistent)
    while true {
        if seen >= limit {
            return false
        }
        let (exchanged, original) = alive.compareExchange(
            expected: seen, desired: seen + 1, ordering: .sequentiallyConsistent)
        if exchanged {
            return true
        }
        seen = original
    }
}

/// Whether a push took its element.
public enum PushStatus: Equatable, Sendable {
    /// The element is in the queue.
    case ok

    /// The queue held its capacity. The element was not taken.
    case full

    /// A segmented queue's allocator refused a segment. A `bounded` queue never reports it.
    case outOfMemory
}
