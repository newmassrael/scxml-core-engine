// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

@testable import SceForgeRuntime

// What the queue tests share: an element that counts its own end, the counter it reports to, and an
// allocator that grants a fixed number of segments.

/// An element that counts its own deallocations.
final class Tracked: Sendable {
    let gone: Counter
    init(_ gone: Counter) { self.gone = gone }
    deinit { gone.increment() }
}

final class Counter: Sendable {
    private let value = Atomic<Int>(0)
    func increment() { value.add(1, ordering: .relaxed) }
    var count: Int { value.load(ordering: .relaxed) }
}

/// An allocator that grants `budget` segments at a time and counts what it was asked. A segment given
/// back is a segment that may be granted again.
final class BudgetAllocator: SegmentAllocator {
    let progress: QueueProgress
    private let budget: Int
    private let live = Atomic<Int>(0)
    private let granted = Atomic<Int>(0)
    private let returned = Atomic<Int>(0)

    init(budget: Int, progress: QueueProgress = .waitFree) {
        self.budget = budget
        self.progress = progress
    }

    func allocate<Segment: AnyObject>(_ make: () -> Segment) -> Segment? {
        while true {
            let seen = live.load(ordering: .sequentiallyConsistent)
            if seen >= budget {
                return nil
            }
            if live.compareExchange(
                expected: seen, desired: seen + 1, ordering: .sequentiallyConsistent
            ).exchanged {
                granted.add(1, ordering: .relaxed)
                return make()
            }
        }
    }

    func deallocate(_ segment: AnyObject) {
        live.subtract(1, ordering: .sequentiallyConsistent)
        returned.add(1, ordering: .relaxed)
    }

    var liveSegments: Int { live.load(ordering: .sequentiallyConsistent) }
    var grantedSegments: Int { granted.load(ordering: .relaxed) }
    var returnedSegments: Int { returned.load(ordering: .relaxed) }
}
