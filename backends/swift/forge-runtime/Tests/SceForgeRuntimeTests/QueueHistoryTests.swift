// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The Swift arm's stress runs of the `queue` kind, written as histories (SCE Protocol-Synthesis RFC section
// synth-5-P, verification layer 2).
//
// These tests run the queues of this package on real threads, record every attempt each thread made, and
// write one history per run in the JSON form every backend writes
// (`tests/forge/conformance/queue_history.schema.json`). They judge nothing about linearizability:
// `sce-codegen check-queue-history` does, for this backend as for the others, so no backend carries a
// checker of its own. The Swift gate (`scripts/gates/forge-swift.sh`) sets `SCE_QUEUE_HISTORY_DIR`, runs
// these, then runs the command over what they wrote. Without the variable there is nowhere to write, and the
// tests are disabled by name.
//
// The recording follows the other arms': one counter that every thread reads immediately before and
// immediately after each call, so a read-modify-write chain on one atomic orders the readings with the
// calls between them, and "returned before invoked" in the record means it in the run. Every attempt is
// recorded, the refused pushes and the empty pops too, because those are results the checker must account
// for.

/// One observed operation. A pop that found the queue empty names no value.
private struct Operation {
    let call: String
    let value: Int?
    let outcome: String
    let invoked: Int
    let returned: Int

    var json: String {
        let named = value.map { ",\"value\":\($0)" } ?? ""
        return "{\"call\":\"\(call)\"\(named),\"outcome\":\"\(outcome)\",\"invoked\":\(invoked),\"returned\":\(returned)}"
    }
}

private struct History {
    let capacity: Int
    let refusal: String
    let participants: [[Operation]]

    /// A participant that made no attempt is the empty list, which the format takes and `null` it does not.
    var json: String {
        let logs = participants.map { "[" + $0.map(\.json).joined(separator: ",") + "]" }
        return "{\"version\":1,\"capacity\":\(capacity),\"refusal\":\"\(refusal)\",\"participants\":[\(logs.joined(separator: ","))]}"
    }
}

private let refusalAtCapacity = "at-capacity"
private let refusalWhileSlotsAreHeld = "while-slots-are-held"

/// Runs of each Lamport capacity are recorded. A Lamport run pushes thousands of values and its history is
/// megabytes, and the ring has no schedule a second run reaches that the first does not.
private let runsPerSpscCapacity = 3

/// Values that go through each Lamport run.
private let valuesPerSpscRun = 2000

/// Runs of each SCQ shape are recorded: short runs, many of them, because the schedules that matter are a
/// small fraction of those a machine gives.
private let runsPerScqShape = 25

/// Runs of each segmented shape are recorded, for the same reason as the SCQ shapes'.
private let runsPerSegmentedShape = 25

/// How long a run may take before it is called stuck: nanoseconds.
private let deadlineNanos: UInt64 = 120_000_000_000

/// Where the histories are written, or nil when nowhere is named.
private func historyDirectory() -> URL? {
    guard let named = ProcessInfo.processInfo.environment["SCE_QUEUE_HISTORY_DIR"], !named.isEmpty else {
        return nil
    }
    return URL(fileURLWithPath: named, isDirectory: true)
}

private func write(_ history: History, named name: String, in directory: URL) throws {
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try (history.json + "\n").write(
        to: directory.appendingPathComponent("\(name).json"), atomically: true, encoding: .utf8)
}

/// The clock every thread reads before and after each call.
private final class TickClock: Sendable {
    private let ticks = Atomic<Int>(0)

    func tick() -> Int {
        ticks.wrappingAdd(1, ordering: .sequentiallyConsistent).newValue
    }
}

/// What the threads of a run recorded and said went wrong.
private final class Recording: Sendable {
    private let logs: Mutex<[[Operation]]>
    private let problems = Mutex<[String]>([])

    init(participants: Int) {
        logs = Mutex(Array(repeating: [], count: participants))
    }

    func hand(_ log: [Operation], of participant: Int) {
        logs.withLock { $0[participant] = log }
    }

    func fail(_ reason: String) {
        problems.withLock { $0.append(reason) }
    }

    var collected: [[Operation]] { logs.withLock { $0 } }
    var failures: [String] { problems.withLock { $0 } }
}

/// A producing handle of whichever queue a run is on: what it pushes with and how it gives its place back.
/// The closures are `@Sendable`: each is handed to the thread that works that handle.
private struct Producing: Sendable {
    let push: @Sendable (Int) -> PushStatus
    let release: @Sendable () -> Void
}

/// A consuming handle of whichever queue a run is on.
private struct Consuming: Sendable {
    let pop: @Sendable () -> Int?
    let release: @Sendable () -> Void
}

/// Gives the other threads the processor for a moment.
private func backOff() {
    Thread.sleep(forTimeInterval: 0.000_001)
}

/// One producer thread for each of `producers` and one consumer thread for each of `consumers` on real
/// threads, each producer pushing `perProducer` distinct values, judged against a sequential queue of
/// `capacity` under `refusal`. A push the queue does not take is recorded with the outcome `full` and tried
/// again; a status other than `refused` or `.ok` fails the run.
private func recordRun(
    capacity: Int, refusal: String, refused: PushStatus, producers: [Producing], consumers: [Consuming],
    perProducer: Int
) -> History? {
    let clock = TickClock()
    let started = DispatchTime.now().uptimeNanoseconds
    let total = producers.count * perProducer
    let delivered = Counter()
    let recording = Recording(participants: producers.count + consumers.count)
    let done = DispatchSemaphore(value: 0)

    for (p, handle) in producers.enumerated() {
        Thread {
            var log: [Operation] = []
            producing: for i in 0..<perProducer {
                let value = p * perProducer + i + 1
                while true {
                    if DispatchTime.now().uptimeNanoseconds - started > deadlineNanos {
                        recording.fail("the elements did not all arrive before the deadline")
                        break producing
                    }
                    let invoked = clock.tick()
                    let status = handle.push(value)
                    let returned = clock.tick()
                    if status == .ok {
                        log.append(Operation(call: "push", value: value, outcome: "pushed", invoked: invoked, returned: returned))
                        break
                    }
                    if status != refused {
                        recording.fail("a push reported \(status) where only \(refused) is a refusal of this queue")
                        break producing
                    }
                    log.append(Operation(call: "push", value: value, outcome: "full", invoked: invoked, returned: returned))
                    backOff()
                }
            }
            handle.release()
            recording.hand(log, of: p)
            done.signal()
        }.start()
    }
    // The delivered count is shared, so a consumer that has seen every element go out stops.
    for (c, handle) in consumers.enumerated() {
        Thread {
            var log: [Operation] = []
            while delivered.count < total {
                if DispatchTime.now().uptimeNanoseconds - started > deadlineNanos {
                    recording.fail("the elements did not all arrive before the deadline")
                    break
                }
                let invoked = clock.tick()
                let value = handle.pop()
                let returned = clock.tick()
                if let value {
                    log.append(Operation(call: "pop", value: value, outcome: "popped", invoked: invoked, returned: returned))
                    delivered.increment()
                } else {
                    log.append(Operation(call: "pop", value: nil, outcome: "empty", invoked: invoked, returned: returned))
                    backOff()
                }
            }
            handle.release()
            recording.hand(log, of: producers.count + c)
            done.signal()
        }.start()
    }
    for _ in 0..<(producers.count + consumers.count) { done.wait() }
    let failures = recording.failures
    if !failures.isEmpty {
        Issue.record("\(failures.joined(separator: "; "))")
        return nil
    }
    return History(capacity: capacity, refusal: refusal, participants: recording.collected)
}

/// One producer and one consumer on two threads through the Lamport ring.
private func recordSpscRun(capacity: Int) -> History? {
    let queue = Spsc<Int>(capacity: capacity)
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    return recordRun(
        capacity: capacity, refusal: refusalAtCapacity, refused: .full,
        producers: [Producing(push: { producer.tryPush($0) }, release: { producer.release() })],
        consumers: [Consuming(pop: { consumer.tryPop() }, release: { consumer.release() })],
        perProducer: valuesPerSpscRun)
}

/// `producers` producers and `consumers` consumers on real threads through the SCQ queue, each producer
/// pushing `perProducer` distinct values.
private func recordScqRun(capacity: Int, ringSlots: Int, producers: Int, consumers: Int, perProducer: Int) -> History? {
    let queue = Scq<Int>(capacity: capacity, ringSlots: ringSlots)
    let pushing = (0..<producers).map { _ -> Producing in
        let handle = queue.producer()!
        return Producing(push: { handle.tryPush($0) }, release: { handle.release() })
    }
    let popping = (0..<consumers).map { _ -> Consuming in
        let handle = queue.consumer()!
        return Consuming(pop: { handle.tryPop() }, release: { handle.release() })
    }
    return recordRun(
        capacity: capacity, refusal: refusalWhileSlotsAreHeld, refused: .full, producers: pushing, consumers: popping,
        perProducer: perProducer)
}

/// The segmented queues through an allocator that gives as many segments as are asked for: one producer and
/// one consumer through linked Lamport rings of four, and otherwise `producers` and `consumers` through a
/// list of SCQ rings of segments of two over rings of four. A segmented queue has no capacity, and the
/// recording gives none that could matter: the history is judged against a queue that holds every value, so a
/// push is never refused, and an empty pop is judged exactly as for any other queue. The queue's segments
/// must all be back with the allocator but the newest one, once the domain has given back what it holds, or
/// the run fails.
private func recordSegmentedRun(many: Bool, producers: Int, consumers: Int, perProducer: Int) -> History? {
    let allocator = BudgetAllocator(budget: Int.max)
    let domain = HazardDomain(slots: producers + consumers)
    let pushing: [Producing]
    let popping: [Consuming]
    // The queue is held until the run is judged, so that its segments are still the allocator's.
    let keep: AnyObject
    if many {
        let queue = Lscq<Int>.create(
            segment: 2, ringSlots: 4, allocator: allocator, domain: domain, required: .lockFree)!
        keep = queue
        pushing = (0..<producers).map { _ -> Producing in
            let handle = queue.producer()!
            return Producing(push: { handle.tryPush($0) }, release: { handle.release() })
        }
        popping = (0..<consumers).map { _ -> Consuming in
            let handle = queue.consumer()!
            return Consuming(pop: { handle.tryPop() }, release: { handle.release() })
        }
    } else {
        let queue = LinkedLamport<Int>.create(segment: 4, allocator: allocator, required: .lockFree)!
        keep = queue
        let producer = queue.producer()!
        let consumer = queue.consumer()!
        pushing = [Producing(push: { producer.tryPush($0) }, release: { producer.release() })]
        popping = [Consuming(pop: { consumer.tryPop() }, release: { consumer.release() })]
    }
    let history = recordRun(
        capacity: producers * perProducer, refusal: refusalAtCapacity, refused: .outOfMemory, producers: pushing,
        consumers: popping, perProducer: perProducer)
    domain.flush()
    #expect(
        allocator.liveSegments == 1,
        "the queue gave back every segment but its newest: \(allocator.liveSegments) still out")
    withExtendedLifetime(keep) {}
    return history
}

@Test(.enabled(if: historyDirectory() != nil, "SCE_QUEUE_HISTORY_DIR names where the histories are written"))
func theLamportRingRunsAreWrittenAsHistories() throws {
    let directory = try #require(historyDirectory())
    for capacity in [1, 2, 3, 8] {
        for run in 0..<runsPerSpscCapacity {
            let history = try #require(recordSpscRun(capacity: capacity))
            try write(history, named: "swift_spsc_n\(capacity)_\(run)", in: directory)
        }
    }
}

@Test(.enabled(if: historyDirectory() != nil, "SCE_QUEUE_HISTORY_DIR names where the histories are written"))
func theScqRunsAreWrittenAsHistories() throws {
    let directory = try #require(historyDirectory())
    // The shapes are the Rust arm's. A ring is at least as large as the number of participants working it,
    // so the small capacities ride on rings sized for their threads.
    let shapes: [(capacity: Int, ringSlots: Int, producers: Int, consumers: Int, perProducer: Int)] = [
        (1, 2, 2, 2, 150),
        (3, 4, 2, 2, 150),
        (8, 8, 2, 2, 150),
        (2, 4, 3, 1, 120),
        (4, 4, 1, 3, 120),
        (5, 8, 2, 2, 150),
    ]
    for shape in shapes {
        for run in 0..<runsPerScqShape {
            let history = try #require(
                recordScqRun(
                    capacity: shape.capacity, ringSlots: shape.ringSlots, producers: shape.producers,
                    consumers: shape.consumers, perProducer: shape.perProducer))
            try write(
                history,
                named: "swift_scq_n\(shape.capacity)_r\(shape.ringSlots)_p\(shape.producers)_c\(shape.consumers)_\(run)",
                in: directory)
        }
    }
}

@Test(.enabled(if: historyDirectory() != nil, "SCE_QUEUE_HISTORY_DIR names where the histories are written"))
func theSegmentedRunsAreWrittenAsHistories() throws {
    let directory = try #require(historyDirectory())
    // The shapes are the C arm's: linked Lamport rings for one and one, and a list of SCQ rings with few and
    // with several threads a side, short runs of many segments each.
    let shapes: [(many: Bool, producers: Int, consumers: Int, perProducer: Int)] = [
        (false, 1, 1, 600),
        (true, 2, 1, 60),
        (true, 1, 2, 60),
        (true, 2, 2, 50),
        (true, 3, 3, 30),
    ]
    for shape in shapes {
        for run in 0..<runsPerSegmentedShape {
            let history = try #require(
                recordSegmentedRun(
                    many: shape.many, producers: shape.producers, consumers: shape.consumers,
                    perProducer: shape.perProducer))
            let row = shape.many ? "lscq_n2_r4" : "linked_lamport_n4"
            try write(
                history, named: "swift_segmented_\(row)_p\(shape.producers)_c\(shape.consumers)_\(run)",
                in: directory)
        }
    }
}

/// A participant that made no attempt is written as the empty list: a consumer that started after the others
/// had taken every element made none, and `null` is what the format refuses. Run-dependent in the stress
/// runs, so it is pinned here without one.
@Test func aParticipantThatMadeNoAttemptIsWrittenAsAnEmptyListNotNull() {
    let history = History(
        capacity: 2, refusal: refusalWhileSlotsAreHeld,
        participants: [
            [Operation(call: "push", value: 1, outcome: "pushed", invoked: 1, returned: 2)],
            [],
            [Operation(call: "pop", value: 1, outcome: "popped", invoked: 3, returned: 4)],
        ])
    let text = history.json
    #expect(!text.contains("null"), "a participant with no attempt was written as null: \(text)")
    #expect(text.contains("}],[],[{"), "the middle participant is not the empty list: \(text)")
}
