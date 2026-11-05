// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The queue contract every language arm holds its runtime to (SCE Protocol-Synthesis RFC section synth-5-P,
// verification layer 1): `tests/forge/conformance/queue_contract.json`, a list of single-threaded
// scenarios, each a sequence of operations on one queue and the result each must give. The Swift arm runs
// every scenario whose storage row it implements, and says by name which it does not.
//
// - `bounded`, one producer and one consumer: the Lamport ring (`Spsc`). Any other cardinality: SCQ.
// - `segmented`, one and one: linked Lamport rings. Any other cardinality: a list of SCQ rings (`Lscq`).
// - `intrusive`: not a row Swift has. A node of a reference-counted object has no link to use in place,
//   and the generator refuses the storage for Swift by name.
//
// Swift has destructors, so the `destroy` step is held as the arms with destructors hold it: the queue ends
// and exactly the elements listed end with it.

/// The values of the elements that have ended.
private final class Ledger: Sendable {
    private let ended = Mutex<[Int]>([])

    func record(_ value: Int) {
        ended.withLock { $0.append(value) }
    }

    /// What has ended since the last call.
    func take() -> [Int] {
        ended.withLock { values in
            let taken = values
            values = []
            return taken
        }
    }
}

/// An element that tells the ledger when it ends.
private final class ContractElement: Sendable {
    let value: Int
    private let ledger: Ledger

    init(_ value: Int, _ ledger: Ledger) {
        self.value = value
        self.ledger = ledger
    }

    deinit {
        ledger.record(value)
    }
}

/// An allocator that gives `limit` segments in all, the first included, and never gives one more: a
/// segment handed back is not given again, so a scenario does not depend on when a retired segment is
/// returned, which differs between the algorithms.
private final class TotalAllocator: SegmentAllocator {
    let progress: QueueProgress = .waitFree
    private let limit: Int
    private let given = Atomic<Int>(0)

    init(limit: Int) {
        self.limit = limit
    }

    func allocate<Segment: AnyObject>(_ make: () -> Segment) -> Segment? {
        while true {
            let seen = given.load(ordering: .sequentiallyConsistent)
            if seen >= limit {
                return nil
            }
            if given.compareExchange(
                expected: seen, desired: seen + 1, ordering: .sequentiallyConsistent
            ).exchanged {
                return make()
            }
        }
    }

    func deallocate(_ segment: AnyObject) {}
}

/// A queue under test as the scenarios see it: what they push and pop, and its capacity where it has one.
/// The closures hold the queue and its two handles. A class, so that however many places refer to the
/// subject, `end()` is what ends the queue: a value would be copied, and every copy would keep it.
private final class Subject {
    private var capacityOf: (() -> Int)?
    private var pushOf: ((ContractElement) -> PushStatus)?
    private var popOf: (() -> ContractElement?)?

    init(
        capacity: (() -> Int)?, push: @escaping (ContractElement) -> PushStatus,
        pop: @escaping () -> ContractElement?
    ) {
        capacityOf = capacity
        pushOf = push
        popOf = pop
    }

    var hasEnded: Bool { pushOf == nil }

    /// The queue's capacity, or nil for a queue that has none.
    var capacity: Int? { capacityOf?() }

    func push(_ element: ContractElement) -> PushStatus {
        guard let pushOf else { preconditionFailure("the queue has ended") }
        return pushOf(element)
    }

    func pop() -> ContractElement? {
        guard let popOf else { preconditionFailure("the queue has ended") }
        return popOf()
    }

    /// Lets go of the queue and its handles: the queue ends, and what it held ends with it.
    func end() {
        capacityOf = nil
        pushOf = nil
        popOf = nil
    }
}

private func ceilPow2(_ n: Int) -> Int {
    var size = 1
    while size < n {
        size *= 2
    }
    return size
}

private func spscSubject(capacity: Int) -> Subject {
    let queue = Spsc<ContractElement>(capacity: capacity)
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    return Subject(
        capacity: { queue.capacity }, push: { producer.tryPush($0) }, pop: { consumer.tryPop() })
}

private func scqSubject(capacity: Int) -> Subject {
    let queue = Scq<ContractElement>(capacity: capacity, ringSlots: ceilPow2(capacity))
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    return Subject(
        capacity: { queue.capacity }, push: { producer.tryPush($0) }, pop: { consumer.tryPop() })
}

/// The queue a segmented scenario names, over an allocator that gives the scenario's `segments` in all
/// (every one it asks for when it names none). One producer and one consumer select linked Lamport rings,
/// and any other cardinality a list of SCQ rings (the RFC's selection table).
private func segmentedSubject(id: String, scenario: [String: Any]) -> Subject? {
    let segment = scenario["segment"] as! Int
    let allocator = TotalAllocator(limit: scenario["segments"] as? Int ?? Int.max)
    let one = scenario["producers"] as? String == "one" && scenario["consumers"] as? String == "one"
    if one {
        guard
            let queue = LinkedLamport<ContractElement>.create(
                segment: segment, allocator: allocator, required: .lockFree)
        else {
            Issue.record("\(id): the allocator refused the first segment")
            return nil
        }
        let producer = queue.producer()!
        let consumer = queue.consumer()!
        return Subject(capacity: nil, push: { producer.tryPush($0) }, pop: { consumer.tryPop() })
    }
    guard
        let queue = Lscq<ContractElement>.create(
            segment: segment, ringSlots: ceilPow2(segment), allocator: allocator,
            domain: HazardDomain(slots: 2), required: .lockFree)
    else {
        Issue.record("\(id): the allocator refused the first segment")
        return nil
    }
    let producer = queue.producer()!
    let consumer = queue.consumer()!
    return Subject(capacity: nil, push: { producer.tryPush($0) }, pop: { consumer.tryPop() })
}

/// The repository's checkout, found from this file: the package is built from the tree it is in.
private func contractFile() -> URL {
    var url = URL(fileURLWithPath: #filePath)
    // The file, then SceForgeRuntimeTests, Tests, forge-runtime, swift and backends.
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url.appendingPathComponent("tests/forge/conformance/queue_contract.json")
}

private func runScenario(_ subject: Subject, ledger: Ledger, id: String, steps: [[String: Any]]) {
    for (index, step) in steps.enumerated() {
        let context = "\(id) step \(index)"
        if subject.hasEnded {
            Issue.record("\(context): a step follows the destroy of the queue")
            return
        }
        let live = subject
        let op = step["op"] as? String ?? ""
        switch op {
        case "capacity":
            guard let capacity = live.capacity else {
                Issue.record("\(context): a segmented queue has no capacity to ask for")
                return
            }
            #expect(capacity == step["expect"] as? Int, "\(context): capacity")
        case "push":
            guard let value = step["value"] as? Int else {
                Issue.record("\(context): a push names its value")
                return
            }
            let want = step["expect"] as? String ?? ""
            let got = live.push(ContractElement(value, ledger))
            switch (got, want) {
            case (.ok, "ok"), (.full, "full"), (.outOfMemory, "out_of_memory"):
                break
            default:
                Issue.record("\(context): push gave \(got), want \(want)")
            }
        case "pop":
            let popped = live.pop()
            if let word = step["expect"] as? String {
                #expect(word == "empty", "\(context): a pop expects a number or \"empty\"")
                #expect(popped == nil, "\(context): expected the queue to be empty, popped \(popped?.value ?? -1)")
            } else {
                #expect(popped?.value == step["expect"] as? Int, "\(context): pop")
            }
        case "destroy":
            // What ended before this step is not what the queue held: only the queue's end is counted.
            _ = ledger.take()
            subject.end()
            let want = (step["expect"] as? [Int] ?? []).sorted()
            #expect(
                ledger.take().sorted() == want,
                "\(context): the queue's end ends exactly the elements it held")
        default:
            Issue.record("\(context): unknown op \(op)")
        }
    }
}

@Test func everyQueueContractScenarioHolds() throws {
    let data = try Data(contentsOf: contractFile())
    let contract = try #require(try JSONSerialization.jsonObject(with: data) as? [String: Any])
    #expect(contract["version"] as? Int == 1, "this arm reads version 1 of the contract")
    let scenarios = try #require(contract["scenarios"] as? [[String: Any]])
    #expect(!scenarios.isEmpty, "a contract with no scenarios checks nothing")

    var run = 0
    var refused: [String] = []
    for scenario in scenarios {
        let id = scenario["id"] as? String ?? "?"
        let storage = scenario["storage"] as? String ?? ""
        let one = scenario["producers"] as? String == "one" && scenario["consumers"] as? String == "one"
        let subject: Subject?
        switch storage {
        case "intrusive":
            // The generator refuses this storage for Swift by name, so there is no runtime to run.
            refused.append(id)
            continue
        case "segmented":
            subject = segmentedSubject(id: id, scenario: scenario)
        case "bounded" where one:
            subject = spscSubject(capacity: scenario["capacity"] as! Int)
        case "bounded":
            // Any other cardinality selects the SCQ row, so the three combinations share one runtime.
            subject = scqSubject(capacity: scenario["capacity"] as! Int)
        default:
            Issue.record("\(id): the Swift arm has no runtime for the storage \(storage)")
            continue
        }
        guard let subject else { continue }
        runScenario(subject, ledger: Ledger(), id: id, steps: scenario["steps"] as? [[String: Any]] ?? [])
        run += 1
    }
    #expect(run + refused.count == scenarios.count, "every scenario was run or refused by name")
    #expect(run >= 20, "the Swift arm ran \(run) scenarios: the bounded and the segmented rows")
    #expect(refused.allSatisfy { $0.hasPrefix("intrusive_") }, "only the intrusive rows are refused: \(refused)")
}
