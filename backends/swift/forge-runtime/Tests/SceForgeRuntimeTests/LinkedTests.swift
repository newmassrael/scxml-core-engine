// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The contract of the Swift segmented queue for one producer and one consumer (SCE Protocol-Synthesis RFC
// section synth-5-P): linked Lamport rings that grow by segments an allocator grants and give each
// segment back once the consumer has left it. The allocator is the only bound on what such a queue
// holds, so the tests state it: an element is taken while the allocator grants segments and reported
// `.outOfMemory` when it does not, the element staying with the caller. Everything up to the threaded run
// is one thread; the threaded run shows that with a producer and a consumer on real threads every element
// arrives once, in order, across many segments.

private func queue(
    segment: Int, budget: Int
) -> (allocator: BudgetAllocator, queue: LinkedLamport<Int>, producer: LinkedLamportProducer<Int>, consumer: LinkedLamportConsumer<Int>) {
    let allocator = BudgetAllocator(budget: budget)
    let queue = LinkedLamport<Int>.create(segment: segment, allocator: allocator, required: .waitFree)!
    return (allocator, queue, queue.producer()!, queue.consumer()!)
}

@Test func aLinkedQueueTakesElementsAcrossSegmentsInOrder() {
    let (allocator, _, producer, consumer) = queue(segment: 3, budget: 8)
    #expect(producer.segment == 3)
    #expect(consumer.segment == 3)
    for value in 0..<10 { #expect(producer.tryPush(value) == .ok) }
    // Ten elements of three need four segments.
    #expect(allocator.grantedSegments == 4)
    for value in 0..<10 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

@Test func aLinkedQueueReportsOutOfMemoryWhenTheAllocatorRefusesAndKeepsWhatItHolds() {
    let (_, _, producer, consumer) = queue(segment: 2, budget: 2)
    for value in 0..<4 { #expect(producer.tryPush(value) == .ok) }
    #expect(producer.tryPush(4) == .outOfMemory)
    #expect(producer.tryPush(4) == .outOfMemory, "a refusal changes nothing")
    for value in 0..<4 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

@Test func aSegmentTheConsumerHasLeftIsGivenBackAndGrantedAgain() {
    let (allocator, _, producer, consumer) = queue(segment: 2, budget: 2)
    for value in 0..<4 { #expect(producer.tryPush(value) == .ok) }
    #expect(producer.tryPush(4) == .outOfMemory)
    #expect(allocator.returnedSegments == 0)
    // Reading the first segment does not leave it: the consumer leaves it on its next pop.
    #expect(consumer.tryPop() == 0)
    #expect(consumer.tryPop() == 1)
    #expect(allocator.returnedSegments == 0)
    #expect(consumer.tryPop() == 2)
    #expect(allocator.returnedSegments == 1)
    #expect(allocator.liveSegments == 1)
    #expect(producer.tryPush(4) == .ok)
    #expect(producer.tryPush(5) == .ok)
    #expect(consumer.tryPop() == 3)
    #expect(consumer.tryPop() == 4)
    #expect(consumer.tryPop() == 5)
    #expect(consumer.tryPop() == nil)
}

@Test func anAllocatorThatRefusesTheFirstSegmentBuildsNoQueue() {
    let allocator = BudgetAllocator(budget: 0)
    #expect(LinkedLamport<Int>.create(segment: 4, allocator: allocator, required: .waitFree) == nil)
}

@Test func aLinkedQueueGoesRoundManyTimesWithoutLosingTheOrder() {
    let (allocator, _, producer, consumer) = queue(segment: 3, budget: 3)
    var next = 0
    var expected = 0
    for round in 0..<3_000 {
        for _ in 0..<(round % 7) where producer.tryPush(next) == .ok { next += 1 }
        for _ in 0..<(round % 5 + 1) {
            if let value = consumer.tryPop() {
                #expect(value == expected)
                expected += 1
            }
        }
    }
    while let value = consumer.tryPop() {
        #expect(value == expected)
        expected += 1
    }
    #expect(expected == next)
    #expect(allocator.liveSegments <= 3)
}

@Test func aLinkedSideHasOnePlaceAtATimeAndDroppingItsHandleGivesItBack() {
    let allocator = BudgetAllocator(budget: 2)
    let queue = LinkedLamport<Int>.create(segment: 2, allocator: allocator, required: .waitFree)!
    var producer = queue.producer()
    #expect(producer != nil)
    #expect(queue.producer() == nil, "a list shared by two producers is not this algorithm")
    producer = nil
    #expect(queue.producer() != nil)
    let consumer = queue.consumer()
    #expect(queue.consumer() == nil)
    consumer?.release()
    #expect(queue.consumer() != nil)
}

@Test func theSuccessorOfALinkedHandleContinuesWhereItStopped() {
    let allocator = BudgetAllocator(budget: 4)
    let queue = LinkedLamport<Int>.create(segment: 2, allocator: allocator, required: .waitFree)!
    let first = queue.producer()!
    for value in 0..<3 { #expect(first.tryPush(value) == .ok) }
    first.release()
    let second = queue.producer()!
    #expect(second.tryPush(3) == .ok)
    #expect(second.tryPush(4) == .ok)
    let consumer = queue.consumer()!
    for value in 0..<5 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
    consumer.release()
    let again = queue.consumer()!
    #expect(second.tryPush(5) == .ok)
    #expect(again.tryPop() == 5)
}

@Test func aLinkedQueueKeepsNoCopyOfAnElementThatLeftIt() {
    let gone = Counter()
    let allocator = BudgetAllocator(budget: 4)
    let queue = LinkedLamport<Tracked>.create(segment: 2, allocator: allocator, required: .waitFree)!
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    #expect(producer.tryPush(Tracked(gone)) == .ok)
    do {
        let popped = consumer.tryPop()
        #expect(popped != nil)
        withExtendedLifetime(popped) {
            #expect(gone.count == 0)
        }
    }
    #expect(gone.count == 1)
}

@Test func aLinkedQueueReleasesTheElementsInEverySegmentWhenItEnds() {
    let gone = Counter()
    do {
        let allocator = BudgetAllocator(budget: 8)
        let queue = LinkedLamport<Tracked>.create(segment: 2, allocator: allocator, required: .waitFree)!
        let producer = queue.producer()!
        for _ in 0..<7 { #expect(producer.tryPush(Tracked(gone)) == .ok) }
        producer.release()
    }
    #expect(gone.count == 7)
}

@Test func aLongListOfSegmentsEndsWithoutOverflowingTheStack() {
    let gone = Counter()
    do {
        let allocator = BudgetAllocator(budget: 200_000)
        let queue = LinkedLamport<Tracked>.create(segment: 1, allocator: allocator, required: .waitFree)!
        let producer = queue.producer()!
        for _ in 0..<150_000 { #expect(producer.tryPush(Tracked(gone)) == .ok) }
        producer.release()
    }
    #expect(gone.count == 150_000)
}

private final class Received: Sendable {
    let seen = Mutex<[Int]>([])
}

@Test func aProducerAndAConsumerAcrossManySegmentsLoseNothingAndKeepTheOrder() {
    let total = 200_000
    let allocator = BudgetAllocator(budget: 16)
    let queue = LinkedLamport<Int>.create(segment: 7, allocator: allocator, required: .waitFree)!
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    let done = DispatchSemaphore(value: 0)
    let received = Received()

    Thread {
        for value in 0..<total {
            while producer.tryPush(value) != .ok { Thread.sleep(forTimeInterval: 0.000_001) }
        }
        done.signal()
    }.start()
    Thread {
        var seen: [Int] = []
        seen.reserveCapacity(total)
        while seen.count < total {
            if let value = consumer.tryPop() {
                seen.append(value)
            } else {
                Thread.sleep(forTimeInterval: 0.000_001)
            }
        }
        received.seen.withLock { $0 = seen }
        done.signal()
    }.start()
    done.wait()
    done.wait()

    let seen = received.seen.withLock { $0 }
    #expect(seen == Array(0..<total), "every element arrived once, in the order it was pushed")
    #expect(allocator.liveSegments <= 16)
    #expect(allocator.grantedSegments - allocator.returnedSegments == allocator.liveSegments)
}
