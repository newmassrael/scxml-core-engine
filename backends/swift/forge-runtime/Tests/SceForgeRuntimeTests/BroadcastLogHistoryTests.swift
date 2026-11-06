// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The Swift arm's stress runs of the `broadcast-log` kind, written as histories (SCE Protocol-Synthesis RFC
// §synth-5-S; ADR 0062 section 5 and section 6, layer 3).
//
// These tests run the log of this package on real threads, record every call the publisher and each reader
// made, and write one history per run in the JSON form every backend writes
// (`tests/forge/conformance/broadcast_history.schema.json`). They judge nothing about linearizability:
// `sce-codegen check-broadcast-history` does, for this backend as for the others, so no backend carries a
// checker of its own. The Swift gate (`scripts/gates/forge-swift.sh`) sets `SCE_BROADCAST_HISTORY_DIR`, runs
// these, then runs the command over what they wrote. Without the variable there is nowhere to write, and the
// test is disabled by name.
//
// The recording follows the queue arm's: one counter that every thread reads immediately before and
// immediately after each call, so a read-modify-write chain on one atomic orders the readings with the calls
// between them, and "returned before invoked" in the record means it in the run. Every call is recorded, the
// empty reads and the lagged ones too, because those are results the checker must account for.
//
// The values the publisher publishes are a function of the sequence number, and the words of an entry are a
// function of its value, so a copy that mixes two entries fails the run here, before the checker sees it, as
// well as showing in the history as a value no publish wrote. A run is a few hundred entries, and each
// reader's record is capped at what a run costs, so the checker's specialised procedure decides it without a
// search.

/// Words in an entry; more than one, so a copy can be torn between them.
private let entryWords = 3

/// The publisher reads the head after every this many publishes.
private let headEvery: UInt64 = 6

/// A reader reads the head after every this many reads.
private let readerHeadEvery = 7

/// Operations a reader records at most, its open and its reads together. A reader that has not reached the
/// end of the log by then stops, and its history is a prefix of what it would have done, which is a history
/// all the same.
private let maxReaderOperations = 900

/// Where the reader opened `at` asks to start; past the head when the log is young, which clamps.
private let openAt: UInt64 = 3

/// Runs of each shape that are recorded: short runs, many of them, as the queue arm's.
private let runsPerShape = 25

/// How long a run may take before it is called stuck: nanoseconds.
private let deadlineNanos: UInt64 = 120_000_000_000

/// The value the publish of sequence number `seq` carries.
private func valueOf(_ seq: UInt64) -> UInt64 {
    seq + 100
}

/// The entry of value `value`: each word is the value and a function of its place.
private func entryOfValue(_ value: UInt64) -> [UInt64] {
    (0..<entryWords).map { value + UInt64($0) * 1_000 }
}

/// One observed operation, as the JSON the checker reads: `fields` holds what the call names beyond its
/// kind and its two clock readings.
private struct Observed {
    let call: String
    let fields: String
    let invoked: Int
    let returned: Int

    var json: String {
        "{\"call\":\"\(call)\"\(fields),\"invoked\":\(invoked),\"returned\":\(returned)}"
    }
}

private struct BroadcastHistory {
    let retention: Int
    let publisher: [Observed]
    let readers: [[Observed]]

    var json: String {
        let published = "[" + publisher.map(\.json).joined(separator: ",") + "]"
        let read = readers.map { "[" + $0.map(\.json).joined(separator: ",") + "]" }.joined(separator: ",")
        return "{\"version\":1,\"retention\":\(retention),\"publisher\":\(published),\"readers\":[\(read)]}"
    }
}

/// The clock every thread reads before and after each call.
private final class SharedClock: Sendable {
    private let ticks = Atomic<Int>(0)

    func tick() -> Int {
        ticks.wrappingAdd(1, ordering: .sequentiallyConsistent).newValue
    }
}

/// Whether the publisher has published everything and let go of its place.
private final class Finished: Sendable {
    private let flag = Atomic<Bool>(false)

    func set() {
        flag.store(true, ordering: .sequentiallyConsistent)
    }

    var isSet: Bool {
        flag.load(ordering: .sequentiallyConsistent)
    }
}

/// What the threads of a run recorded and said went wrong.
private final class Recording: Sendable {
    private let published = Mutex<[Observed]>([])
    private let read: Mutex<[[Observed]]>
    private let problems = Mutex<[String]>([])

    init(readers: Int) {
        read = Mutex(Array(repeating: [], count: readers))
    }

    func handPublisher(_ ops: [Observed]) {
        published.withLock { $0 = ops }
    }

    func hand(_ ops: [Observed], ofReader index: Int) {
        read.withLock { $0[index] = ops }
    }

    func fail(_ reason: String) {
        problems.withLock { $0.append(reason) }
    }

    var publisher: [Observed] { published.withLock { $0 } }
    var readers: [[Observed]] { read.withLock { $0 } }
    var failures: [String] { problems.withLock { $0 } }
}

/// Where the histories are written, or nil when nowhere is named.
private func broadcastHistoryDirectory() -> URL? {
    guard let named = ProcessInfo.processInfo.environment["SCE_BROADCAST_HISTORY_DIR"], !named.isEmpty else {
        return nil
    }
    return URL(fileURLWithPath: named, isDirectory: true)
}

private func write(_ history: BroadcastHistory, named name: String, in directory: URL) throws {
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try (history.json + "\n").write(
        to: directory.appendingPathComponent("\(name).json"), atomically: true, encoding: .utf8)
}

/// Gives the other threads the processor for a moment.
private func yieldProcessor() {
    Thread.sleep(forTimeInterval: 0.000_001)
}

/// The head a call returned, as the checker reads it.
private func headObservation(_ log: BroadcastLog, _ clock: SharedClock) -> Observed {
    let invoked = clock.tick()
    let head = log.head
    let returned = clock.tick()
    return Observed(call: "head", fields: ",\"head\":\(head)", invoked: invoked, returned: returned)
}

/// One publisher publishing `total` entries and `readerCount` readers on real threads through a log of
/// `retention` retained entries. The readers open in turn at the oldest entry, at the head and at `openAt`;
/// each reads until the publisher is finished and it has met the end, or until its record is full. Nil, with
/// the reasons recorded as an issue, when a thread saw the log do what it must not.
private func recordBroadcastRun(retention: Int, readerCount: Int, total: UInt64) -> BroadcastHistory? {
    // The ring holds one slot more than the retention a reader may rely on.
    let log = BroadcastLog(words: entryWords, slots: retention + 1)
    let publisher = log.publisher()!
    let clock = SharedClock()
    let finished = Finished()
    let recording = Recording(readers: readerCount)
    let started = DispatchTime.now().uptimeNanoseconds
    let done = DispatchSemaphore(value: 0)

    Thread {
        var ops: [Observed] = []
        publishing: for seq in 0..<total {
            let value = valueOf(seq)
            let entry = entryOfValue(value)
            let invoked = clock.tick()
            let taken = publisher.publish(entry)
            let returned = clock.tick()
            ops.append(
                Observed(
                    call: "publish", fields: ",\"value\":\(value),\"seq\":\(taken)", invoked: invoked,
                    returned: returned))
            if taken != seq {
                recording.fail("publish returned \(taken), wanted \(seq)")
                break publishing
            }
            if seq % headEvery == headEvery - 1 {
                ops.append(headObservation(log, clock))
            }
            if seq % 3 == 0 {
                yieldProcessor()
            }
        }
        finished.set()
        publisher.release()
        recording.handPublisher(ops)
        done.signal()
    }.start()

    for index in 0..<readerCount {
        Thread {
            var ops: [Observed] = []
            defer {
                recording.hand(ops, ofReader: index)
                done.signal()
            }
            var out = [UInt64](repeating: 0, count: entryWords)
            for _ in 0..<index { yieldProcessor() }

            let openInvoked = clock.tick()
            var reader: BroadcastLogReader
            let opened: String
            switch index % 3 {
            case 0:
                reader = log.readerOldest()
                opened = ",\"from\":\"oldest\""
            case 1:
                reader = log.readerHead()
                opened = ",\"from\":\"head\""
            default:
                reader = log.readerAt(openAt)
                opened = ",\"from\":\"at\",\"at\":\(openAt)"
            }
            let openReturned = clock.tick()
            ops.append(
                Observed(
                    call: "open", fields: opened + ",\"cursor\":\(reader.cursor)", invoked: openInvoked,
                    returned: openReturned))

            var last: UInt64?
            var reads = 0
            reading: while ops.count < maxReaderOperations {
                if DispatchTime.now().uptimeNanoseconds - started > deadlineNanos {
                    recording.fail("reader \(index) did not reach the end before the deadline")
                    break reading
                }
                // Taken before the read: if the publisher had finished by then, an empty answer is the end
                // of the log and not a pause.
                let wasFinished = finished.isSet
                let before = reader.cursor
                let invoked = clock.tick()
                let result = log.read(&reader, into: &out)
                let returned = clock.tick()
                reads += 1
                switch result {
                case .item(let seq):
                    if seq != before || (last.map { seq <= $0 } ?? false) {
                        recording.fail("reader \(index): item \(seq) at cursor \(before) after \(String(describing: last))")
                        break reading
                    }
                    last = seq
                    for k in 0..<entryWords where out[k] != out[0] + UInt64(k) * 1_000 {
                        recording.fail("reader \(index): item \(seq) is torn: \(out)")
                        break reading
                    }
                    ops.append(
                        Observed(
                            call: "read", fields: ",\"outcome\":\"item\",\"seq\":\(seq),\"value\":\(out[0])",
                            invoked: invoked, returned: returned))
                case .lagged(let skipped):
                    ops.append(
                        Observed(
                            call: "read", fields: ",\"outcome\":\"lagged\",\"skipped\":\(skipped)",
                            invoked: invoked, returned: returned))
                case .empty:
                    ops.append(
                        Observed(call: "read", fields: ",\"outcome\":\"empty\"", invoked: invoked, returned: returned))
                    if wasFinished {
                        break reading
                    }
                    yieldProcessor()
                }
                if reads % readerHeadEvery == 0 && ops.count < maxReaderOperations {
                    ops.append(headObservation(log, clock))
                }
            }
        }.start()
    }

    for _ in 0..<(1 + readerCount) { done.wait() }
    let failures = recording.failures
    if !failures.isEmpty {
        Issue.record("\(failures.joined(separator: "; "))")
        return nil
    }
    return BroadcastHistory(retention: retention, publisher: recording.publisher, readers: recording.readers)
}

@Test(.enabled(if: broadcastHistoryDirectory() != nil, "SCE_BROADCAST_HISTORY_DIR names where the histories are written"))
func theBroadcastLogRunsAreWrittenAsHistories() throws {
    let directory = try #require(broadcastHistoryDirectory())
    // The retentions run from the smallest log, in which every publish after the first laps a reader, to one
    // a reader can follow.
    let shapes: [(retention: Int, readers: Int, total: UInt64)] = [
        (1, 2, 500),
        (2, 2, 500),
        (2, 3, 400),
        (3, 3, 500),
    ]
    for shape in shapes {
        for run in 0..<runsPerShape {
            let history = try #require(
                recordBroadcastRun(retention: shape.retention, readerCount: shape.readers, total: shape.total))
            try write(
                history,
                named: "swift_broadcast_r\(shape.retention)_c\(shape.readers)_p\(shape.total)_\(run)",
                in: directory)
        }
    }
}
