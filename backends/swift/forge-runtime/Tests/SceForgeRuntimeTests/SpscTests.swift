// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The contract of the Swift bounded queue for one producer and one consumer (SCE Protocol-Synthesis RFC
// section synth-5-P): a Lamport ring that holds exactly its capacity, hands elements out in the order
// they went in, gives a side's place back when its handle is released or dropped, and keeps no copy of an
// element that has left it. Everything up to the threaded run is one thread, so each test states what the
// queue answers and not what a schedule gave; the threaded run then shows what one thread cannot, that
// with a producer and a consumer on real threads every element arrives once, in order.

private func pair<Element: Sendable>(
    _ capacity: Int, of: Element.Type = Element.self
) -> (queue: Spsc<Element>, producer: SpscProducer<Element>, consumer: SpscConsumer<Element>) {
    let queue = Spsc<Element>(capacity: capacity)
    return (queue, queue.producer()!, queue.consumer()!)
}

@Test func aQueueHoldsExactlyItsCapacity() {
    let (_, producer, consumer) = pair(3, of: Int.self)
    #expect(producer.capacity == 3)
    #expect(consumer.capacity == 3)
    #expect(producer.tryPush(1) == .ok)
    #expect(producer.tryPush(2) == .ok)
    #expect(producer.tryPush(3) == .ok)
    // A ring that spent a slot to tell full from empty would have refused the third.
    #expect(producer.tryPush(4) == .full)
    #expect(consumer.tryPop() == 1)
    #expect(producer.tryPush(4) == .ok)
    #expect(producer.tryPush(5) == .full)
}

@Test func anElementComesOutInTheOrderItWentIn() {
    let (_, producer, consumer) = pair(4, of: Int.self)
    for value in 0..<4 { #expect(producer.tryPush(value) == .ok) }
    for value in 0..<4 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

@Test func anEmptyQueueAnswersNilAndKeepsAnsweringIt() {
    let (_, producer, consumer) = pair(2, of: String.self)
    #expect(consumer.tryPop() == nil)
    #expect(consumer.tryPop() == nil)
    #expect(producer.tryPush("a") == .ok)
    #expect(consumer.tryPop() == "a")
    #expect(consumer.tryPop() == nil)
}

@Test func theIndicesGoRoundManyTimesWithoutLosingTheOrder() {
    // Far more laps than the 2N index space holds, with the ring at a different fill each time.
    let (_, producer, consumer) = pair(5, of: Int.self)
    var next = 0
    var expected = 0
    for round in 0..<2_000 {
        let burst = round % 6
        for _ in 0..<burst where producer.tryPush(next) == .ok { next += 1 }
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
}

@Test func aCapacityOfOneAlternatesPushAndPop() {
    let (_, producer, consumer) = pair(1, of: Int.self)
    for value in 0..<10 {
        #expect(producer.tryPush(value) == .ok)
        #expect(producer.tryPush(-1) == .full)
        #expect(consumer.tryPop() == value)
        #expect(consumer.tryPop() == nil)
    }
}

@Test func aSideHasOnePlaceAtATimeAndDroppingItsHandleGivesItBack() {
    let queue = Spsc<Int>(capacity: 2)
    var producer = queue.producer()
    #expect(producer != nil)
    #expect(queue.producer() == nil, "a ring shared by two producers is not this algorithm")
    producer = nil
    #expect(queue.producer() != nil, "the place was given back when the handle was dropped")

    let consumer = queue.consumer()
    #expect(queue.consumer() == nil)
    consumer?.release()
    #expect(queue.consumer() != nil)
}

@Test func theSuccessorOfAHandleContinuesWhereItStopped() {
    let queue = Spsc<Int>(capacity: 3)
    let first = queue.producer()!
    #expect(first.tryPush(1) == .ok)
    #expect(first.tryPush(2) == .ok)
    first.release()
    let second = queue.producer()!
    #expect(second.tryPush(3) == .ok)
    #expect(second.tryPush(4) == .full)
    let consumer = queue.consumer()!
    #expect([consumer.tryPop(), consumer.tryPop(), consumer.tryPop()] == [1, 2, 3])
}

@Test func aPoppedElementIsNotKeptByTheQueue() {
    let gone = Counter()
    let queue = Spsc<Tracked>(capacity: 2)
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    #expect(producer.tryPush(Tracked(gone)) == .ok)
    do {
        let popped = consumer.tryPop()
        #expect(popped != nil)
        // The popped element is still held here, so it has not ended; its life is kept to this point
        // so that a compiler that ends it early does not make the check true by accident.
        withExtendedLifetime(popped) {
            #expect(gone.count == 0)
        }
    }
    // The only reference left was the popped one, and it has ended.
    #expect(gone.count == 1)
}

@Test func aQueueReleasesTheElementsItStillHoldsWhenItEnds() {
    let gone = Counter()
    do {
        let queue = Spsc<Tracked>(capacity: 3)
        let producer = queue.producer()!
        #expect(producer.tryPush(Tracked(gone)) == .ok)
        #expect(producer.tryPush(Tracked(gone)) == .ok)
        producer.release()
    }
    #expect(gone.count == 2)
}

/// What the consuming thread received, handed to the test thread. A `Mutex` is noncopyable, so a thread's
/// closure holds it through a class.
private final class Received: Sendable {
    let seen = Mutex<[Int]>([])
}

@Test func aProducerAndAConsumerOnTheirOwnThreadsLoseNothingAndKeepTheOrder() {
    let total = 200_000
    let queue = Spsc<Int>(capacity: 64)
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    let done = DispatchSemaphore(value: 0)
    let received = Received()

    let pushing = Thread {
        for value in 0..<total {
            while producer.tryPush(value) != .ok { Thread.sleep(forTimeInterval: 0.000_001) }
        }
        done.signal()
    }
    let popping = Thread {
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
    }
    pushing.start()
    popping.start()
    done.wait()
    done.wait()

    let seen = received.seen.withLock { $0 }
    #expect(seen.count == total)
    #expect(seen == Array(0..<total), "every element arrived once, in the order it was pushed")
    #expect(consumer.tryPop() == nil)
}
