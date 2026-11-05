// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The contract of the Swift segmented queue for any number of producers and consumers (SCE
// Protocol-Synthesis RFC section synth-5-P): a list of SCQ rings that grows by segments an allocator grants
// and hands each segment to a hazard domain once the consumers have left it. Everything up to the threaded
// run is one thread and states each rule of the list: the order across segments, the allocator's refusal,
// the segment's way back, the side places, the help a push gives a stalled one, and that the end of a queue
// gives back every segment and every element. The threaded run shows that with several producers and
// several consumers every element arrives once, each producer's in the order it pushed them, across many
// segments and many retirements.

private struct Fixture {
    let allocator: BudgetAllocator
    let domain: HazardDomain
    let queue: Lscq<Int>
}

private func fixture(segment: Int, ringSlots: Int, budget: Int, hazardSlots: Int = 8) -> Fixture {
    let allocator = BudgetAllocator(budget: budget)
    let domain = HazardDomain(slots: hazardSlots)
    let queue = Lscq<Int>.create(
        segment: segment, ringSlots: ringSlots, allocator: allocator, domain: domain, required: .waitFree)!
    return Fixture(allocator: allocator, domain: domain, queue: queue)
}

@Test func anLscqTakesElementsAcrossSegmentsInOrder() {
    let f = fixture(segment: 3, ringSlots: 4, budget: 8)
    let producer = f.queue.producer()!
    let consumer = f.queue.consumer()!
    #expect(producer.segment == 3)
    #expect(consumer.segment == 3)
    for value in 0..<10 { #expect(producer.tryPush(value) == .ok) }
    // Ten elements of three need four segments.
    #expect(f.allocator.grantedSegments == 4)
    for value in 0..<10 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

@Test func anLscqReportsOutOfMemoryWhenTheAllocatorRefusesAndKeepsWhatItHolds() {
    let f = fixture(segment: 2, ringSlots: 2, budget: 2)
    let producer = f.queue.producer()!
    let consumer = f.queue.consumer()!
    for value in 0..<4 { #expect(producer.tryPush(value) == .ok) }
    #expect(producer.tryPush(4) == .outOfMemory)
    #expect(producer.tryPush(4) == .outOfMemory, "a refusal changes nothing")
    for value in 0..<4 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

@Test func aSegmentTheConsumersHaveLeftGoesBackAndIsGrantedAgain() {
    let f = fixture(segment: 2, ringSlots: 2, budget: 2)
    let producer = f.queue.producer()!
    let consumer = f.queue.consumer()!
    for value in 0..<4 { #expect(producer.tryPush(value) == .ok) }
    #expect(producer.tryPush(4) == .outOfMemory)
    #expect(consumer.tryPop() == 0)
    #expect(consumer.tryPop() == 1)
    #expect(f.allocator.returnedSegments == 0)
    // The pop that leaves the first segment retires it. It goes back when the domain scans, and nothing
    // has asked for a scan: it holds the allocator's budget meanwhile.
    #expect(consumer.tryPop() == 2)
    #expect(f.domain.waiting == 1, "the left segment waits on the domain's list")
    #expect(f.allocator.returnedSegments == 0)
    // The push that the allocator refuses asks the domain to give back what it can, and is granted the
    // segment that went back. Otherwise a drained queue would refuse a push for as long as nobody pops.
    #expect(producer.tryPush(4) == .ok, "the refused push got the waiting segment back and was granted it")
    #expect(f.allocator.returnedSegments == 1)
    #expect(f.domain.waiting == 0)
    #expect(producer.tryPush(5) == .ok)
    #expect(consumer.tryPop() == 3)
    #expect(consumer.tryPop() == 4)
    #expect(consumer.tryPop() == 5)
    #expect(consumer.tryPop() == nil)
}

@Test func retirementsGiveSegmentsBackWithoutAnyoneAskingForAScan() {
    let f = fixture(segment: 1, ringSlots: 2, budget: 30, hazardSlots: 2)
    let producer = f.queue.producer()!
    let consumer = f.queue.consumer()!
    for value in 0..<20 { #expect(producer.tryPush(value) == .ok) }
    for value in 0..<20 { #expect(consumer.tryPop() == value) }
    // A scan runs when twice the slots are waiting, so what is held back is bounded by the participants.
    #expect(f.domain.waiting < 2 * f.domain.slots)
    #expect(f.allocator.liveSegments <= 1 + 2 * f.domain.slots)
}

@Test func anAllocatorThatRefusesTheFirstSegmentBuildsNoLscq() {
    let allocator = BudgetAllocator(budget: 0)
    let domain = HazardDomain(slots: 2)
    #expect(
        Lscq<Int>.create(segment: 4, ringSlots: 4, allocator: allocator, domain: domain, required: .waitFree)
            == nil)
}

@Test func aSideHasAPlacePerRingSlotAndTheDomainHasASlotPerHandle() {
    let f = fixture(segment: 2, ringSlots: 2, budget: 4)
    var first = f.queue.producer()
    let second = f.queue.producer()
    #expect(first != nil)
    #expect(second != nil)
    #expect(f.queue.producer() == nil, "a ring is not shared by more enqueuers than it has slots")
    first = nil
    #expect(f.queue.producer() != nil, "a place that came back is handed out again")

    let consumer = f.queue.consumer()
    #expect(consumer != nil)
    #expect(consumer?.tryClone() != nil)
    withExtendedLifetime(second) {}
}

@Test func aRefusedHandleLeavesNeitherAPlaceNorASlotTaken() {
    // Two domain slots, four places a side: the third handle is refused by the domain.
    let f = fixture(segment: 2, ringSlots: 4, budget: 4, hazardSlots: 2)
    var producer = f.queue.producer()
    let consumer = f.queue.consumer()
    #expect(producer != nil)
    #expect(consumer != nil)
    for _ in 0..<6 { #expect(f.queue.producer() == nil, "the domain has no slot left") }
    // Had each refusal kept a place, four places would be gone by now.
    producer = nil
    #expect(f.queue.producer() != nil)
    withExtendedLifetime(consumer) {}
}

@Test func aPushFinishesTheLinkAStalledProducerLeft() {
    let f = fixture(segment: 1, ringSlots: 2, budget: 4)
    let producer = f.queue.producer()!
    let consumer = f.queue.consumer()!
    let first = f.queue.head.load(ordering: .sequentiallyConsistent)
    #expect(producer.tryPush(0) == .ok)
    #expect(producer.tryPush(1) == .ok, "this push links the second segment")
    // A producer stopped between linking a successor and moving `tail` leaves `tail` on the first segment.
    f.queue.tail.store(first, ordering: .sequentiallyConsistent)
    #expect(producer.tryPush(2) == .ok)
    #expect(
        f.queue.tail.load(ordering: .sequentiallyConsistent) != first,
        "the push that found the link moved `tail` before it pushed")
    for value in 0..<3 { #expect(consumer.tryPop() == value) }
    #expect(consumer.tryPop() == nil)
}

/// A producer that has linked a successor and not yet moved `tail` leaves `tail` on the segment it linked
/// from. The consumer that then drains and retires that segment must take `tail` off it first, because a
/// producer validates its hazard against `tail` and a segment `tail` still names looks safe to a producer
/// that published after the scan read its slot (the type's documentation, *Where a hazard is validated*).
/// The state is made here by putting `tail` back on the first segment after a push linked the second, which
/// is what a producer stopped between its two compare-and-swaps leaves.
@Test func aConsumerTakesTailOffASegmentBeforeItRetiresIt() {
    let f = fixture(segment: 1, ringSlots: 2, budget: 4)
    let producer = f.queue.producer()!
    let consumer = f.queue.consumer()!
    let first = f.queue.head.load(ordering: .sequentiallyConsistent)

    #expect(producer.tryPush(0) == .ok, "the first segment takes one")
    #expect(producer.tryPush(1) == .ok, "the second push links a segment")
    let second = f.queue.tail.load(ordering: .sequentiallyConsistent)
    #expect(first != second, "the push moved `tail` to the new segment")
    f.queue.tail.store(first, ordering: .sequentiallyConsistent)

    #expect(consumer.tryPop() == 0, "the first segment's element")
    #expect(consumer.tryPop() == 1, "the next comes from the second")

    #expect(
        f.queue.head.load(ordering: .sequentiallyConsistent) == second,
        "the consumer moved `head` past the drained segment")
    #expect(
        f.queue.tail.load(ordering: .sequentiallyConsistent) == second,
        "`tail` does not name a segment `head` has left and the consumer retired")

    // Nothing names the retired segment now, so it can go back.
    f.domain.collect()
    #expect(f.allocator.liveSegments == 1, "the retired segment went back and the live one did not")
}

@Test func anLscqKeepsNoCopyOfAnElementThatLeftIt() {
    let gone = Counter()
    let allocator = BudgetAllocator(budget: 4)
    let domain = HazardDomain(slots: 4)
    let queue = Lscq<Tracked>.create(
        segment: 2, ringSlots: 2, allocator: allocator, domain: domain, required: .waitFree)!
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

@Test func anLscqThatEndsGivesBackEverySegmentAndEveryElementItHolds() {
    let gone = Counter()
    let allocator = BudgetAllocator(budget: 8)
    do {
        let domain = HazardDomain(slots: 4)
        let queue = Lscq<Tracked>.create(
            segment: 2, ringSlots: 2, allocator: allocator, domain: domain, required: .waitFree)!
        let producer = queue.producer()!
        let consumer = queue.consumer()!
        for _ in 0..<7 { #expect(producer.tryPush(Tracked(gone)) == .ok) }
        // Some segments are retired and still waiting on the domain's list when the queue ends.
        for _ in 0..<3 { _ = consumer.tryPop() }
        producer.release()
        consumer.release()
    }
    #expect(gone.count == 7, "the elements popped and the elements still in the queue all ended")
    #expect(allocator.liveSegments == 0, "every segment went back")
    #expect(allocator.grantedSegments == allocator.returnedSegments)
}

@Test func aLongLscqListOfSegmentsEndsWithoutOverflowingTheStack() {
    let gone = Counter()
    let allocator = BudgetAllocator(budget: 30_000)
    do {
        let domain = HazardDomain(slots: 2)
        let queue = Lscq<Tracked>.create(
            segment: 2, ringSlots: 2, allocator: allocator, domain: domain, required: .waitFree)!
        let producer = queue.producer()!
        for _ in 0..<40_000 { #expect(producer.tryPush(Tracked(gone)) == .ok) }
        producer.release()
    }
    #expect(gone.count == 40_000)
    #expect(allocator.liveSegments == 0)
}

/// What the consuming threads received, handed to the test thread. A `Mutex` is noncopyable, so a thread's
/// closure holds it through a class.
private final class Received: Sendable {
    let seen = Mutex<[[Int]]>([])
}

/// Told to the threads of a run that has stopped making progress, so that it ends instead of hanging.
private final class Stop: Sendable {
    private let flag = Atomic<Bool>(false)
    var raised: Bool { flag.load(ordering: .relaxed) }
    func raise() { flag.store(true, ordering: .relaxed) }
}

@Test func severalLscqProducersAndConsumersLoseNothingDuplicateNothingAndKeepEachProducersOrder() {
    let producers = 4
    let consumers = 4
    let perProducer = 20_000
    let total = producers * perProducer
    // Small segments and a small budget, so the run links, retires, gives back and is refused many times:
    // a producer that is refused waits for the consumers to hand a segment back.
    let f = fixture(segment: 8, ringSlots: 8, budget: 64, hazardSlots: producers + consumers)
    let pushed = Counter()
    let popped = Counter()
    let stop = Stop()
    let done = DispatchSemaphore(value: 0)
    let received = Received()

    // An element is `producer * perProducer + sequence`, so a consumer can tell whose it is and where.
    for producerIndex in 0..<producers {
        let handle = f.queue.producer()!
        Thread {
            producing: for sequence in 0..<perProducer {
                let value = producerIndex * perProducer + sequence
                while handle.tryPush(value) != .ok {
                    if stop.raised { break producing }
                    Thread.sleep(forTimeInterval: 0.000_001)
                }
                pushed.increment()
            }
            handle.release()
            done.signal()
        }.start()
    }
    for _ in 0..<consumers {
        let handle = f.queue.consumer()!
        Thread {
            var mine: [Int] = []
            while popped.count < total && !stop.raised {
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
    // A run that stops making progress fails with what it held, and does not hang the suite: the threads
    // are told to stop, and the test gives them a moment to say they have.
    let deadline = DispatchTime.now() + .seconds(120)
    var finished = 0
    while finished < producers + consumers {
        let limit = stop.raised ? DispatchTime.now() + .seconds(30) : deadline
        if done.wait(timeout: limit) == .success {
            finished += 1
        } else if stop.raised {
            Issue.record("\(producers + consumers - finished) thread(s) did not stop when told to")
            return
        } else {
            stop.raise()
            Issue.record(
                """
                no progress in 120 s: pushed \(pushed.count) and popped \(popped.count) of \(total); \
                allocator live \(f.allocator.liveSegments), granted \(f.allocator.grantedSegments), \
                returned \(f.allocator.returnedSegments); domain waiting \(f.domain.waiting)
                """)
        }
    }

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
    // This pop leaves every closed, empty segment behind, so one segment is left in the list.
    #expect(f.queue.consumer()!.tryPop() == nil)
    f.domain.flush()
    #expect(f.allocator.grantedSegments - f.allocator.returnedSegments == f.allocator.liveSegments)
    #expect(f.allocator.liveSegments == 1, "what the run left is the one segment the list still holds")
}
