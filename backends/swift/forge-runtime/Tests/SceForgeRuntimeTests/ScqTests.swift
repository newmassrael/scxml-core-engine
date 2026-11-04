// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The contract of the Swift bounded queue for any number of producers and consumers (SCE Protocol-Synthesis
// RFC section synth-5-P), Nikolaev's SCQ: a linearizable FIFO that holds exactly its capacity when nothing
// else runs, hands a side's place back when its handle is released or dropped, and keeps no copy of an
// element that has left it. Everything up to the threaded run is one thread, so each test states what the
// queue answers and not what a schedule gave; the threaded run then shows what one thread cannot, that
// with several producers and several consumers on real threads every element arrives exactly once, and
// each producer's elements arrive in the order that producer pushed them.

private func pair<Element: Sendable>(
    capacity: Int, ringSlots: Int, of: Element.Type
) -> (queue: Scq<Element>, producer: ScqProducer<Element>, consumer: ScqConsumer<Element>) {
    let queue = Scq<Element>(capacity: capacity, ringSlots: ringSlots)
    return (queue, queue.producer()!, queue.consumer()!)
}

@Test func aQueueHoldsExactlyItsCapacityWhenNothingElseRuns() {
    let (_, producer, consumer) = pair(capacity: 3, ringSlots: 4, of: Int.self)
    #expect(producer.capacity == 3)
    #expect(consumer.capacity == 3)
    #expect(producer.tryPush(1) == .ok)
    #expect(producer.tryPush(2) == .ok)
    #expect(producer.tryPush(3) == .ok)
    #expect(producer.tryPush(4) == .full)
    #expect(consumer.tryPop() == 1)
    #expect(producer.tryPush(4) == .ok)
    #expect(producer.tryPush(5) == .full)
}

@Test func aRingLargerThanTheCapacityStillHoldsExactlyTheCapacity() {
    let (_, producer, consumer) = pair(capacity: 5, ringSlots: 16, of: Int.self)
    for value in 0..<5 { #expect(producer.tryPush(value) == .ok) }
    #expect(producer.tryPush(99) == .full)
    for value in 0..<5 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

@Test func anScqElementComesOutInTheOrderItWentIn() {
    let (_, producer, consumer) = pair(capacity: 8, ringSlots: 8, of: Int.self)
    for value in 0..<8 { #expect(producer.tryPush(value) == .ok) }
    for value in 0..<8 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

@Test func anEmptyQueueAnswersNilEveryTime() {
    let (_, producer, consumer) = pair(capacity: 2, ringSlots: 2, of: String.self)
    for _ in 0..<50 { #expect(consumer.tryPop() == nil) }
    #expect(producer.tryPush("a") == .ok)
    #expect(consumer.tryPop() == "a")
    for _ in 0..<50 { #expect(consumer.tryPop() == nil) }
    // The head overshot an empty ring many times; the queue still takes and gives back its capacity.
    #expect(producer.tryPush("b") == .ok)
    #expect(producer.tryPush("c") == .ok)
    #expect(producer.tryPush("d") == .full)
    #expect(consumer.tryPop() == "b")
    #expect(consumer.tryPop() == "c")
}

@Test func theCyclesGoRoundManyTimesWithoutLosingTheOrder() {
    // Far more cycles than a ring of eight slots has entries, with the queue at a different fill each
    // time.
    let (_, producer, consumer) = pair(capacity: 5, ringSlots: 8, of: Int.self)
    var next = 0
    var expected = 0
    for round in 0..<5_000 {
        for _ in 0..<(round % 6) where producer.tryPush(next) == .ok { next += 1 }
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

@Test func anScqCapacityOfOneAlternatesPushAndPop() {
    let (_, producer, consumer) = pair(capacity: 1, ringSlots: 1, of: Int.self)
    for value in 0..<20 {
        #expect(producer.tryPush(value) == .ok)
        #expect(producer.tryPush(-1) == .full)
        #expect(consumer.tryPop() == value)
        #expect(consumer.tryPop() == nil)
    }
}

@Test func aSideHasAsManyPlacesAsTheRingHasSlotsAndDroppingAHandleGivesOneBack() {
    let queue = Scq<Int>(capacity: 2, ringSlots: 2)
    var first: ScqProducer<Int>? = queue.producer()
    let second = queue.producer()
    #expect(first != nil && second != nil)
    #expect(queue.producer() == nil, "every producer place is taken")
    first = nil
    #expect(queue.producer() != nil, "the place was given back when the handle was dropped")

    let consumer = queue.consumer()
    let other = queue.consumer()
    #expect(queue.consumer() == nil)
    other?.release()
    #expect(queue.consumer() != nil)
    _ = consumer
}

@Test func aClonedHandleTakesAPlaceOfItsOwn() {
    let queue = Scq<Int>(capacity: 2, ringSlots: 2)
    let producer = queue.producer()!
    let clone = producer.tryClone()
    #expect(clone != nil)
    #expect(producer.tryClone() == nil, "both producer places are taken")
    clone?.release()
    #expect(producer.tryClone() != nil)
}

/// An element that counts its own deallocations.
private final class Tracked: Sendable {
    let gone: Counter
    init(_ gone: Counter) { self.gone = gone }
    deinit { gone.increment() }
}

private final class Counter: Sendable {
    private let value = Atomic<Int>(0)
    func increment() { value.add(1, ordering: .relaxed) }
    var count: Int { value.load(ordering: .relaxed) }
}

@Test func anScqPoppedElementIsNotKeptByTheQueue() {
    let gone = Counter()
    let queue = Scq<Tracked>(capacity: 2, ringSlots: 2)
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

@Test func anScqReleasesTheElementsItStillHoldsWhenItEnds() {
    let gone = Counter()
    do {
        let queue = Scq<Tracked>(capacity: 3, ringSlots: 4)
        let producer = queue.producer()!
        #expect(producer.tryPush(Tracked(gone)) == .ok)
        #expect(producer.tryPush(Tracked(gone)) == .ok)
        producer.release()
    }
    #expect(gone.count == 2)
}

/// What the consuming threads received, handed to the test thread. A `Mutex` is noncopyable, so a
/// thread's closure holds it through a class.
private final class Received: Sendable {
    let seen = Mutex<[[Int]]>([])
}

@Test func severalProducersAndConsumersLoseNothingDuplicateNothingAndKeepEachProducersOrder() {
    let producers = 4
    let consumers = 4
    let perProducer = 40_000
    let total = producers * perProducer
    let queue = Scq<Int>(capacity: 64, ringSlots: 64)
    let popped = Counter()
    let done = DispatchSemaphore(value: 0)
    let received = Received()

    // An element is `producer * perProducer + sequence`, so a consumer can tell whose it is and where.
    for producerIndex in 0..<producers {
        let handle = queue.producer()!
        Thread {
            for sequence in 0..<perProducer {
                let value = producerIndex * perProducer + sequence
                while handle.tryPush(value) != .ok { Thread.sleep(forTimeInterval: 0.000_001) }
            }
            handle.release()
            done.signal()
        }.start()
    }
    for _ in 0..<consumers {
        let handle = queue.consumer()!
        Thread {
            var mine: [Int] = []
            while popped.count < total {
                if let value = handle.tryPop() {
                    mine.append(value)
                    popped.increment()
                } else {
                    Thread.sleep(forTimeInterval: 0.000_001)
                }
            }
            handle.release()
            received.seen.withLock { $0.append(mine) }
            done.signal()
        }.start()
    }
    for _ in 0..<(producers + consumers) { done.wait() }

    let perConsumer = received.seen.withLock { $0 }
    let everything = perConsumer.flatMap { $0 }
    #expect(everything.count == total)
    #expect(Set(everything).count == total, "no element arrived twice")
    #expect(everything.sorted() == Array(0..<total), "every element arrived")
    // Each consumer sees one producer's elements in the order that producer pushed them.
    for mine in perConsumer {
        var last = [Int: Int]()
        for value in mine {
            let producerIndex = value / perProducer
            if let before = last[producerIndex] {
                #expect(value > before, "producer \(producerIndex)'s order was kept")
            }
            last[producerIndex] = value
        }
    }
    #expect(queue.consumer()!.tryPop() == nil)
}
