// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

@Test func theQueueIsFirstInFirstOut() {
    var queue = EventQueueManager<Int>()
    #expect(queue.isEmpty && !queue.hasEvents && queue.count == 0)
    #expect(queue.pop() == nil)
    #expect(queue.front() == nil)
    for n in 1...3 {
        queue.raise(n)
    }
    #expect(queue.count == 3)
    #expect(queue.front() == 1, "front reads the head and leaves it")
    #expect(queue.count == 3)
    #expect(queue.pop() == 1)
    #expect(queue.pop() == 2)
    #expect(queue.pop() == 3)
    #expect(queue.pop() == nil)
}

@Test func aSnapshotIsTheQueueFrontFirstAndNoWayToChangeIt() {
    var queue = EventQueueManager<Int>()
    queue.raise(1)
    queue.raise(2)
    _ = queue.pop()
    queue.raise(3)
    var snapshot = queue.snapshot()
    #expect(snapshot == [2, 3])
    snapshot.append(99)
    #expect(queue.snapshot() == [2, 3])
}

@Test func theOrderHoldsAcrossTheCompactionOfSpentEntries() {
    var queue = EventQueueManager<Int>()
    var next = 0
    var popped = 0
    // Interleaved raises and pops long enough to cross the compaction threshold
    // several times, and short enough that a mistake in it shows as a wrong order.
    for round in 0..<500 {
        for _ in 0..<(round % 4 + 1) {
            queue.raise(next)
            next += 1
        }
        for _ in 0..<(round % 3 + 1) {
            if let value = queue.pop() {
                #expect(value == popped)
                popped += 1
            }
        }
        #expect(queue.count == next - popped)
    }
    while let value = queue.pop() {
        #expect(value == popped)
        popped += 1
    }
    #expect(popped == next)
}

@Test func clearEmptiesTheQueue() {
    var queue = EventQueueManager<String>()
    queue.raise("a")
    queue.raise("b")
    queue.clear()
    #expect(queue.isEmpty)
    #expect(queue.pop() == nil)
    queue.raise("c")
    #expect(queue.pop() == "c")
}
