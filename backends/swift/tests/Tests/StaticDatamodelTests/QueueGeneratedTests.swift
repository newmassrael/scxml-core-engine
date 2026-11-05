// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import Synchronization
import XCTest

@testable import StaticDatamodelMachines

// The queue kind's generated units (SCE Protocol-Synthesis RFC section synth-5-P), compiled and used.
//
// sce-build's tests read what the generator writes for a queue and look for text in it, which shows the
// text is what the template says and nothing about whether the names in it are the names the runtime
// has. Here the unit the generator writes for each row Swift has is compiled over the element codec's
// struct it names, and the queue it defines is used: a rename in the runtime, or a constant the template
// computes wrongly, stops the build or fails an assertion. The runtime's own properties are
// `forge-runtime`'s tests; this file holds what the generated unit adds.
//
// The units come from `tests/forge/resources/queue_conformance_*.scxml`, written by
// `scripts/regen_static_datamodel_swift.sh`. The two `intrusive` fixtures are not among them: Swift
// refuses that storage by name.

/// An allocator that grants `limit` segments at a time and counts what it was asked. A segment given
/// back is a segment that may be granted again.
private final class GrantingAllocator: SegmentAllocator {
    let progress: QueueProgress
    private let limit: Int
    private let live = Atomic<Int>(0)
    private let granted = Atomic<Int>(0)

    init(limit: Int = Int.max, progress: QueueProgress = .waitFree) {
        self.limit = limit
        self.progress = progress
    }

    func allocate<Segment: AnyObject>(_ make: () -> Segment) -> Segment? {
        while true {
            let seen = live.load(ordering: .sequentiallyConsistent)
            if seen >= limit {
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
    }

    var liveSegments: Int { live.load(ordering: .sequentiallyConsistent) }
    var grantedSegments: Int { granted.load(ordering: .relaxed) }
}

private func event(_ sensor: UInt8, _ value: UInt16) -> QueueConformanceEvent {
    QueueConformanceEvent(sensor_id: sensor, value: value)
}

// The generated alias is the runtime's queue over the element's struct: these accept the alias and
// hand back the runtime's type, and do not compile otherwise.
private func runtimeQueue(_ queue: QueueConformanceSpsc) -> Spsc<QueueConformanceEvent> { queue }
private func runtimeQueue(_ queue: QueueConformanceScq) -> Scq<QueueConformanceEvent> { queue }
private func runtimeQueue(_ queue: QueueConformanceSegmented) -> LinkedLamport<QueueConformanceEvent> { queue }
private func runtimeQueue(_ queue: QueueConformanceSegmentedMany) -> Lscq<QueueConformanceEvent> { queue }

final class QueueGeneratedTests: XCTestCase {
    func testTheLamportRingUnitStatesWhatTheDocumentRequiredAndWhatItGives() {
        typealias Contract = QueueConformanceSpscContract
        XCTAssertEqual(Contract.capacity, 4, "the capacity is the document's: four")
        for progress in [Contract.declaredProgress, Contract.pushProgress, Contract.popProgress] {
            XCTAssertEqual(progress, "wait-free")
        }
        XCTAssertTrue(
            Contract.algorithm.contains("Lamport"),
            "one producer and one consumer select the Lamport ring: \(Contract.algorithm)")
        XCTAssertEqual(runtimeQueue(makeQueueConformanceSpsc()).capacity, Contract.capacity)
    }

    func testTheLamportRingUnitHandsElementsOverInOrderAndRefusesWhenFull() throws {
        let capacity = QueueConformanceSpscContract.capacity
        let queue = makeQueueConformanceSpsc()
        let producer = try XCTUnwrap(queue.producer())
        let consumer = try XCTUnwrap(queue.consumer())

        XCTAssertNil(consumer.tryPop(), "a new queue is empty")
        for value in 0..<capacity {
            XCTAssertEqual(producer.tryPush(event(1, UInt16(value))), .ok, "push \(value) of \(capacity) must fit")
        }
        XCTAssertEqual(producer.tryPush(event(9, 99)), .full, "a full queue refuses the push")
        for value in 0..<capacity {
            let got = try XCTUnwrap(consumer.tryPop(), "pop \(value)")
            XCTAssertEqual(got.sensor_id, 1)
            XCTAssertEqual(got.value, UInt16(value), "pop \(value): first in, first out")
        }
        XCTAssertNil(consumer.tryPop(), "drained")
    }

    func testTheScqUnitStatesTheRingItNeedsAndWhatItGives() {
        typealias Contract = QueueConformanceScqContract
        XCTAssertEqual(Contract.capacity, 6, "the capacity is the document's: six")
        XCTAssertEqual(Contract.participants, 3, "the participants are the document's: three")
        XCTAssertEqual(Contract.ringSlots, 8, "the next power of two at or above both numbers")
        XCTAssertEqual(Contract.wrapBoundOps, scqWrapBoundOps, "the bound is the runtime's")
        for progress in [Contract.declaredProgress, Contract.pushProgress, Contract.popProgress] {
            XCTAssertEqual(progress, "lock-free")
        }
        XCTAssertTrue(
            Contract.algorithm.contains("SCQ"), "many producers and consumers select SCQ: \(Contract.algorithm)")
        let queue = runtimeQueue(makeQueueConformanceScq())
        XCTAssertEqual(queue.capacity, Contract.capacity)
        XCTAssertEqual(queue.ringSlots, Contract.ringSlots)
    }

    func testTheScqUnitFitsExactlyItsCapacityWhenNothingElseRuns() throws {
        let capacity = QueueConformanceScqContract.capacity
        let queue = makeQueueConformanceScq()
        let producer = try XCTUnwrap(queue.producer())
        let consumer = try XCTUnwrap(queue.consumer())
        for value in 0..<capacity {
            XCTAssertEqual(producer.tryPush(event(2, UInt16(value))), .ok, "push \(value) of \(capacity) must fit")
        }
        XCTAssertEqual(producer.tryPush(event(9, 99)), .full, "a full queue refuses the push")
        for value in 0..<capacity {
            let got = try XCTUnwrap(consumer.tryPop(), "pop \(value)")
            XCTAssertEqual(got.value, UInt16(value), "pop \(value): first in, first out")
        }
        XCTAssertNil(consumer.tryPop(), "drained")
    }

    func testTheLinkedLamportUnitStatesWhatTheDocumentRequiredAndWhatItGives() throws {
        typealias Contract = QueueConformanceSegmentedContract
        XCTAssertEqual(Contract.segment, 3, "the segment is the document's: three")
        XCTAssertEqual(Contract.declaredProgress, "lock-free")
        XCTAssertEqual(Contract.pushProgress, "lock-free", "a push no stronger than its allocator")
        XCTAssertEqual(Contract.popProgress, "wait-free")
        XCTAssertTrue(
            Contract.algorithm.contains("Lamport"),
            "one and one select linked Lamport rings: \(Contract.algorithm)")
        XCTAssertEqual(Contract.allocatorProgress, .lockFree, "the allocator progress is the document's")
        XCTAssertEqual(Contract.allocatorProgressWord, "lock-free")
        let queue = try XCTUnwrap(makeQueueConformanceSegmented(allocator: GrantingAllocator()))
        XCTAssertEqual(runtimeQueue(queue).segment, Contract.segment)
    }

    func testTheLinkedLamportUnitHandsElementsOverAcrossSegmentsAndReportsARefusedSegment() throws {
        let segment = QueueConformanceSegmentedContract.segment
        // Two segments of three: six elements, then the allocator refuses.
        let allocator = GrantingAllocator(limit: 2)
        let queue = try XCTUnwrap(makeQueueConformanceSegmented(allocator: allocator))
        let producer = try XCTUnwrap(queue.producer())
        let consumer = try XCTUnwrap(queue.consumer())

        XCTAssertNil(consumer.tryPop(), "a new queue is empty")
        for value in 0..<(2 * segment) {
            XCTAssertEqual(producer.tryPush(event(1, UInt16(value))), .ok, "push \(value) fits the two segments")
        }
        XCTAssertEqual(producer.tryPush(event(9, 99)), .outOfMemory, "the allocator refuses a third segment")
        for value in 0..<(2 * segment) {
            let got = try XCTUnwrap(consumer.tryPop(), "pop \(value)")
            XCTAssertEqual(got.value, UInt16(value), "pop \(value): first in, first out")
        }
        XCTAssertNil(consumer.tryPop(), "drained")
        XCTAssertEqual(producer.tryPush(event(1, 100)), .ok, "a segment the consumer gave back is room again")
    }

    func testTheLinkedLamportUnitIsNotBuiltWhenTheAllocatorGivesNoSegment() {
        XCTAssertNil(
            makeQueueConformanceSegmented(allocator: GrantingAllocator(limit: 0)),
            "an allocator with no segment to give")
    }

    func testTheLscqUnitStatesTheRingItNeedsAndWhatItGives() throws {
        typealias Contract = QueueConformanceSegmentedManyContract
        XCTAssertEqual(Contract.segment, 3, "the segment is the document's: three")
        XCTAssertEqual(Contract.participants, 2, "the participants are the document's: two")
        XCTAssertEqual(Contract.ringSlots, 4, "the next power of two at or above max(segment, participants)")
        XCTAssertEqual(Contract.hazardSlots, 4, "one hazard slot for every handle that can be alive")
        for progress in [Contract.declaredProgress, Contract.pushProgress, Contract.popProgress] {
            XCTAssertEqual(progress, "lock-free")
        }
        XCTAssertTrue(
            Contract.algorithm.contains("LSCQ"), "many and many select LSCQ: \(Contract.algorithm)")
        XCTAssertEqual(Contract.allocatorProgress, .lockFree)
        let queue = try XCTUnwrap(makeQueueConformanceSegmentedMany(allocator: GrantingAllocator()))
        XCTAssertEqual(runtimeQueue(queue).segment, Contract.segment)
        XCTAssertEqual(runtimeQueue(queue).ringSlots, Contract.ringSlots)
    }

    func testTheLscqUnitHandsElementsOverAcrossSegmentsAndGivesEachSegmentBack() throws {
        let segment = QueueConformanceSegmentedManyContract.segment
        let hazardSlots = QueueConformanceSegmentedManyContract.hazardSlots
        let allocator = GrantingAllocator()
        let total = 4 * segment + 1
        do {
            let queue = try XCTUnwrap(makeQueueConformanceSegmentedMany(allocator: allocator))
            let producer = try XCTUnwrap(queue.producer())
            let consumer = try XCTUnwrap(queue.consumer())

            XCTAssertNil(consumer.tryPop(), "a new queue is empty")
            for value in 0..<total {
                XCTAssertEqual(producer.tryPush(event(2, UInt16(value))), .ok, "push \(value)")
            }
            XCTAssertGreaterThanOrEqual(
                allocator.grantedSegments, 5,
                "thirteen elements of three need at least five segments: \(allocator.grantedSegments)")
            for value in 0..<total {
                let got = try XCTUnwrap(consumer.tryPop(), "pop \(value)")
                XCTAssertEqual(got.value, UInt16(value), "pop \(value): first in, first out")
            }
            XCTAssertNil(consumer.tryPop(), "drained")
            // The segments the consumer left wait on the unit's hazard domain until it scans, which is
            // bounded by the slots: the queue's own segment and at most twice the slots more.
            XCTAssertLessThanOrEqual(allocator.liveSegments, 1 + 2 * hazardSlots)
        }
        XCTAssertEqual(allocator.liveSegments, 0, "the end of the queue gave every segment back")
    }

    func testTheLscqUnitIsNotBuiltWhenTheAllocatorGivesNoSegment() {
        XCTAssertNil(
            makeQueueConformanceSegmentedMany(allocator: GrantingAllocator(limit: 0)),
            "an allocator with no segment to give")
    }
}
