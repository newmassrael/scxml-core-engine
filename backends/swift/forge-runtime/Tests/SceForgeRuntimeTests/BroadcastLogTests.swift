// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The contract of the Swift broadcast log (SCE Protocol-Synthesis RFC §synth-5-S; ADR 0062
// sections 2, 4 and 5), held against the cases of the Kotlin runtime's own tests
// (`BroadcastLogContractTest`, `BroadcastLogRuntimeTest`): everything up to the threaded run is one
// thread, so each test states what the log answers and not what a schedule gave; the threaded run
// then shows what one thread cannot, that with one publisher and several readers on real threads
// every entry a reader is handed is whole, in order, and that what it was handed plus what it was
// told it missed is exactly what was published.

/// Three slots retain two entries; an entry is two words.
private func smallLog() -> BroadcastLog {
    BroadcastLog(words: 2, slots: 3)
}

private func entryOf(_ n: UInt64) -> [UInt64] {
    [n, n]
}

/// One read into a buffer that holds a value no entry has, so an untouched buffer shows.
private func readOnce(
    _ log: BroadcastLog, _ reader: inout BroadcastLogReader
) -> (read: BroadcastLogRead, out: [UInt64]) {
    var out = [UInt64](repeating: UInt64.max, count: log.words)
    let result = log.read(&reader, into: &out)
    return (result, out)
}

private func expectRead(
    _ log: BroadcastLog, _ reader: inout BroadcastLogReader, _ expected: BroadcastLogRead
) {
    let answer = readOnce(log, &reader)
    #expect(answer.read == expected, "the read answers \(expected)")
}

private func expectItem(
    _ log: BroadcastLog, _ reader: inout BroadcastLogReader, _ seq: UInt64, _ expected: [UInt64]
) {
    let answer = readOnce(log, &reader)
    #expect(answer.read == .item(seq: seq), "the read answers the entry \(seq)")
    #expect(answer.out == expected, "the entry \(seq) holds the words it was published with")
}

@Test func anEmptyLogIsEmptyToEveryReader() {
    let log = smallLog()
    var reader = log.readerOldest()
    expectRead(log, &reader, .empty)
    #expect(reader.cursor == 0)
    #expect(log.head == 0)
}

@Test func aLogReportsTheShapeItWasMadeWith() {
    let log = BroadcastLog(words: 5, slots: 9)
    #expect(log.words == 5)
    #expect(log.slots == 9)
    #expect(log.retention == 8, "the slot being overwritten is in flux, so one slot is not retained")
    #expect(BroadcastLog.horizon == (UInt64.max - 2) / 2)
}

@Test func entriesComeBackInOrderWithTheirWords() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    var reader = log.readerOldest()
    #expect(publisher.publish(entryOf(10)) == 0)
    #expect(publisher.publish(entryOf(11)) == 1)
    expectItem(log, &reader, 0, entryOf(10))
    expectItem(log, &reader, 1, entryOf(11))
    expectRead(log, &reader, .empty)
    #expect(reader.lost == 0)
}

@Test func theWindowIsExactlyTheRetention() throws {
    // 3 slots retain 2 entries.
    let log = smallLog()
    #expect(log.retention == 2)
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(3) {
        publisher.publish(entryOf(n))
    }
    // h = 3: the window is [1, 3), so a reader at 0 is one behind it.
    var behind = log.readerAt(0)
    expectRead(log, &behind, .lagged(skipped: 1))
    #expect(behind.cursor == 1)
    #expect(behind.lost == 1)
    expectItem(log, &behind, 1, entryOf(1))
    // A reader at the oldest retained entry lost nothing.
    var oldest = log.readerOldest()
    #expect(oldest.cursor == 1)
    expectItem(log, &oldest, 1, entryOf(1))
    #expect(oldest.lost == 0)
}

@Test func aReaderManyLapsBehindIsToldExactlyHowManyItMissed() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(100) {
        publisher.publish(entryOf(n))
    }
    // h = 100: the window is [98, 100) and a reader at 0 is told 98, not "a lot".
    var reader = log.readerAt(0)
    expectRead(log, &reader, .lagged(skipped: 98))
    #expect(reader.cursor == 98)
    #expect(reader.lost == 98)
    expectItem(log, &reader, 98, entryOf(98))
    expectItem(log, &reader, 99, entryOf(99))
    expectRead(log, &reader, .empty)
}

@Test func readersDoNotShareACursorOrALoss() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(5) {
        publisher.publish(entryOf(n))
    }
    var a = log.readerAt(0)
    var b = log.readerAt(3)
    expectRead(log, &a, .lagged(skipped: 3))
    expectItem(log, &b, 3, entryOf(3))
    #expect(a.lost == 3)
    #expect(b.lost == 0)
    #expect(b.cursor == 4)
}

@Test func aReaderAtTheHeadSeesOnlyWhatComesAfter() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    publisher.publish(entryOf(0))
    var reader = log.readerHead()
    expectRead(log, &reader, .empty)
    publisher.publish(entryOf(1))
    expectItem(log, &reader, 1, entryOf(1))
}

@Test func aReaderAtASequenceNumberIsClampedToTheHeadWhenAheadAndLagsWhenBehind() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(5) {
        publisher.publish(entryOf(n))
    }
    // Ahead of the head: the head.
    #expect(log.readerAt(99).cursor == 5)
    #expect(log.readerAt(5).cursor == 5)
    // Inside the window: as asked.
    #expect(log.readerAt(4).cursor == 4)
    #expect(log.readerAt(3).cursor == 3)
    // Behind the window the reader is at the sequence number it asked for, and its first read
    // resumes at the oldest retained entry and says how many it missed.
    var behind = log.readerAt(1)
    #expect(behind.cursor == 1)
    expectRead(log, &behind, .lagged(skipped: 2))
    #expect(behind.cursor == 3)
}

@Test func aDeferredReaderStartsAtTheOldestOfItsFirstReadAndIsToldOfNothing() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    // Both readers open on the empty log, one at the oldest of that moment and one deferred; then
    // five entries (0 to 4) are published. Three slots retain two, so the oldest retained is
    // entry 3.
    var eager = log.readerOldest()
    var deferred = log.readerDeferred()
    #expect(eager.isBound)
    #expect(!deferred.isBound)
    for n in 0..<UInt64(5) {
        publisher.publish(entryOf(n))
    }
    // The eager reader holds 0, which is outside the window [3, 5): it is told it missed 3.
    expectRead(log, &eager, .lagged(skipped: 3))
    // The deferred reader binds at this read, at the oldest retained entry, so its first answer
    // is that entry and it has lost nothing.
    expectItem(log, &deferred, 3, entryOf(3))
    #expect(deferred.isBound)
    #expect(deferred.lost == 0)
    #expect(deferred.cursor == 4)
    expectItem(log, &deferred, 4, entryOf(4))
    expectRead(log, &deferred, .empty)
}

@Test func aDeferredReaderOnALogWithNothingPublishedIsEmptyAndThenReadsFromTheStart() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    var reader = log.readerDeferred()
    expectRead(log, &reader, .empty)
    #expect(reader.isBound, "the first read binds even when it finds nothing")
    #expect(reader.cursor == 0)
    publisher.publish(entryOf(0))
    expectItem(log, &reader, 0, entryOf(0))
    #expect(reader.lost == 0)
}

@Test func anUnboundReaderReportsCursorZeroAndNotBoundUntilItsFirstRead() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(3) {
        publisher.publish(entryOf(n))
    }
    // Opening loads nothing: the head is 3 and the oldest retained is 1, and the reader holds
    // neither.
    var deferred = log.readerDeferred()
    #expect(!deferred.isBound)
    #expect(deferred.cursor == 0)
    #expect(deferred.lost == 0)
    // The other three ways to open are bound at once.
    #expect(log.readerOldest().isBound)
    #expect(log.readerHead().isBound)
    #expect(log.readerAt(0).isBound)
    // The first read binds to the oldest retained entry, 1, and reads it.
    expectItem(log, &deferred, 1, entryOf(1))
    #expect(deferred.isBound)
    #expect(deferred.cursor == 2)
}

@Test func aDeferredReaderThatFallsBehindLaterIsToldAsAnyReaderIs() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(10) {
        publisher.publish(entryOf(n))
    }
    var reader = log.readerDeferred()
    // h = 10: the window is [8, 10), and the bind took 8 with no loss.
    expectItem(log, &reader, 8, entryOf(8))
    #expect(reader.lost == 0)
    for n in 10..<UInt64(13) {
        publisher.publish(entryOf(n))
    }
    // h = 13: the window is [11, 13) and the reader holds 9, so it missed 2.
    expectRead(log, &reader, .lagged(skipped: 2))
    #expect(reader.lost == 2)
    #expect(reader.cursor == 11)
    expectItem(log, &reader, 11, entryOf(11))
}

@Test func aDeferredReaderIsTheRingsLateReaderWhichIsToldOfNothingHoweverMuchWasWritten() throws {
    // A ring of four retained entries: three are published, a reader is opened deferred, ten more
    // are published, and 13 have been published in all.
    let log = BroadcastLog(words: 2, slots: 5)
    #expect(log.retention == 4)
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(3) {
        publisher.publish(entryOf(n))
    }
    var late = log.readerDeferred()
    for n in 3..<UInt64(13) {
        publisher.publish(entryOf(n))
    }
    // The oldest retained is 9, and the reader is told of nothing.
    var seen: [UInt64] = []
    while true {
        let answer = readOnce(log, &late).read
        guard case .item(let seq) = answer else {
            #expect(answer == .empty)
            break
        }
        seen.append(seq)
    }
    #expect(seen == [9, 10, 11, 12])
    #expect(late.lost == 0)
}

@Test func thereIsOnePublisherAndItsSuccessorContinuesTheSequence() throws {
    let log = smallLog()
    let first = try #require(log.publisher())
    let refused = log.publisher()
    #expect(refused == nil, "a second publisher is refused")
    first.publish(entryOf(0))
    first.release()
    let second = try #require(log.publisher(), "the place was freed")
    #expect(second.publish(entryOf(1)) == 1)
    #expect(log.head == 2)
}

@Test func droppingThePublisherFreesItsPlace() throws {
    let log = smallLog()
    var first: BroadcastLogPublisher? = log.publisher()
    #expect(first != nil)
    let refused = log.publisher()
    #expect(refused == nil, "the place is held while the publisher exists")
    #expect(first?.publish(entryOf(0)) == 0)
    first = nil
    let second = try #require(log.publisher(), "the last reference to a publisher gave its place back")
    #expect(second.publish(entryOf(1)) == 1, "the successor continues the sequence")
}

@Test func aReleasedPublisherGivesItsPlaceBackOnceAndASuccessorIsNotDisturbed() throws {
    let log = smallLog()
    let first = try #require(log.publisher())
    first.release()
    let second = try #require(log.publisher(), "the place is free after a release")
    // Releasing the first again, and dropping it later, must not give back the place the second
    // holds.
    first.release()
    let third = log.publisher()
    #expect(third == nil, "a second release of the first publisher left the second's place alone")
    #expect(second.publish(entryOf(0)) == 0)
}

@Test func publishingPastTheWindowKeepsTheNewestEntries() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(100) {
        publisher.publish(entryOf(n))
    }
    var reader = log.readerOldest()
    expectItem(log, &reader, 98, entryOf(98))
    expectItem(log, &reader, 99, entryOf(99))
    expectRead(log, &reader, .empty)
}

@Test func anEmptyReadLeavesTheCursorWhereItWas() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    publisher.publish(entryOf(0))
    var reader = log.readerHead()
    for _ in 0..<3 {
        expectRead(log, &reader, .empty)
        #expect(reader.cursor == 1)
    }
}

@Test func theLossOfASlowReaderIsTheSumOfWhatItWasToldItMissed() throws {
    let log = smallLog()
    let publisher = try #require(log.publisher())
    for n in 0..<UInt64(5) {
        publisher.publish(entryOf(n))
    }
    var reader = log.readerAt(0)
    // h = 5: the window is [3, 5).
    expectRead(log, &reader, .lagged(skipped: 3))
    expectItem(log, &reader, 3, entryOf(3))
    for n in 5..<UInt64(8) {
        publisher.publish(entryOf(n))
    }
    // h = 8: the window is [6, 8) and the reader holds 4.
    expectRead(log, &reader, .lagged(skipped: 2))
    #expect(reader.lost == 5)
    #expect(reader.cursor == 6)
}

@Test func aLogOfOneRetainedEntryKeepsTheLatestAndNothingOlder() throws {
    let log = BroadcastLog(words: 1, slots: 2)
    #expect(log.retention == 1)
    let publisher = try #require(log.publisher())
    var reader = log.readerOldest()
    publisher.publish([7])
    publisher.publish([8])
    expectRead(log, &reader, .lagged(skipped: 1))
    expectItem(log, &reader, 1, [8])
    expectRead(log, &reader, .empty)
}

@Test func anEntryOfManyWordsAndOneOfNoneRoundTrip() throws {
    let wide = BroadcastLog(words: 5, slots: 3)
    let publisher = try #require(wide.publisher())
    var reader = wide.readerOldest()
    let entry: [UInt64] = [UInt64.min, UInt64.max, 0, 1, UInt64.max - 1]
    publisher.publish(entry)
    expectItem(wide, &reader, 0, entry)

    let bare = BroadcastLog(words: 0, slots: 3)
    let barePublisher = try #require(bare.publisher())
    var bareReader = bare.readerOldest()
    #expect(barePublisher.publish([]) == 0)
    expectItem(bare, &bareReader, 0, [])
}

@Test func theSequenceStopsAtTheHorizonAndTheLastEntryThatFitsReadsBack() throws {
    let log = smallLog()
    log.startAt(published: BroadcastLog.horizon - 1)
    let publisher = try #require(log.publisher())
    var reader = log.readerHead()
    // The last entry that fits: its stamp, 2n + 2, is still below 2^64.
    #expect(publisher.publish(entryOf(5)) == BroadcastLog.horizon - 1)
    #expect(log.head == BroadcastLog.horizon)
    expectItem(log, &reader, BroadcastLog.horizon - 1, entryOf(5))
    expectRead(log, &reader, .empty)
    // The stamp of the entry at the horizon, 2 * horizon + 2, is the last even value a UInt64
    // holds, and the entry after it would be the first to wrap: that is why the horizon is where
    // the publisher stops.
    #expect(2 &* BroadcastLog.horizon &+ 2 == UInt64.max &- 1)
}

// MARK: the threaded run

/// The word `index` of the entry `seq`. Every word is a function of the sequence number and of its
/// own position, so a copy that mixes two entries, or a word of one entry in the place of another,
/// disagrees with the entry it claims to be.
private func wordOf(_ seq: UInt64, _ index: Int) -> UInt64 {
    seq &* 1_000_003 &+ UInt64(index) &* 7 &+ 11
}

private func stressEntry(_ seq: UInt64, words: Int) -> [UInt64] {
    (0..<words).map { wordOf(seq, $0) }
}

/// One thread's account of a run, written by that thread alone and read by the test after the
/// thread has signalled it is done.
private final class Tally: @unchecked Sendable {
    var items: UInt64 = 0
    var lagReports: UInt64 = 0
    var cursor: UInt64 = 0
    var lost: UInt64 = 0
    var failure: String?
}

private final class Finished: Sendable {
    let flag = Atomic<Bool>(false)
}

/// One publisher publishing `total` entries and `readerCount` readers, each opened at the first
/// entry, reading until the publisher is finished and they have reached the end.
private func runShape(words: Int, slots: Int, readerCount: Int, total: UInt64) throws {
    let log = BroadcastLog(words: words, slots: slots)
    let publisher = try #require(log.publisher())
    let finished = Finished()
    let publisherTally = Tally()
    let tallies = (0..<readerCount).map { _ in Tally() }
    let done = DispatchSemaphore(value: 0)
    let deadline = Date().addingTimeInterval(60)

    var threads: [Thread] = []
    threads.append(
        Thread {
            for seq in 0..<total {
                let taken = publisher.publish(stressEntry(seq, words: words))
                if taken != seq {
                    publisherTally.failure = "publish returned \(taken), wanted \(seq)"
                    break
                }
                // Give the readers a turn now and then, so a run is not the publisher alone
                // followed by the readers alone.
                if seq % 61 == 0 {
                    Thread.sleep(forTimeInterval: 0.000_001)
                }
            }
            finished.flag.store(true, ordering: .releasing)
            publisher.release()
            done.signal()
        })
    for index in 0..<readerCount {
        let tally = tallies[index]
        threads.append(
            Thread {
                var reader = log.readerAt(0)
                var out = [UInt64](repeating: 0, count: words)
                var last: UInt64?
                readLoop: while true {
                    if Date() > deadline {
                        tally.failure = "reader \(index) did not reach the end before the deadline"
                        break readLoop
                    }
                    // Read before the read: if the publisher had finished by then, an empty answer
                    // is the end of the log and not a pause.
                    let wasFinished = finished.flag.load(ordering: .acquiring)
                    let before = reader.cursor
                    switch log.read(&reader, into: &out) {
                    case .item(let seq):
                        tally.items += 1
                        if seq != before {
                            tally.failure = "reader \(index): item \(seq) at cursor \(before)"
                            break readLoop
                        }
                        if let previous = last, seq <= previous {
                            tally.failure = "reader \(index): item \(seq) after \(previous)"
                            break readLoop
                        }
                        last = seq
                        for word in 0..<words where out[word] != wordOf(seq, word) {
                            tally.failure = "reader \(index): item \(seq) word \(word) is \(out[word]), torn"
                            break readLoop
                        }
                        if reader.cursor != seq + 1 {
                            tally.failure = "reader \(index): cursor \(reader.cursor) after item \(seq)"
                            break readLoop
                        }
                    case .lagged(let skipped):
                        tally.lagReports += 1
                        if skipped < 1 {
                            tally.failure = "reader \(index): a lag that skipped \(skipped)"
                            break readLoop
                        }
                        if reader.cursor != before + skipped {
                            tally.failure =
                                "reader \(index): cursor \(reader.cursor) after skipping \(skipped) from \(before)"
                            break readLoop
                        }
                    case .empty:
                        if reader.cursor != before {
                            tally.failure = "reader \(index): an empty read moved the cursor"
                            break readLoop
                        }
                        if wasFinished {
                            break readLoop
                        }
                        Thread.sleep(forTimeInterval: 0.000_01)
                    }
                }
                tally.cursor = reader.cursor
                tally.lost = reader.lost
                done.signal()
            })
    }
    for thread in threads {
        thread.start()
    }
    for _ in threads {
        done.wait()
    }

    #expect(publisherTally.failure == nil, "\(publisherTally.failure ?? "")")
    for index in 0..<readerCount {
        let tally = tallies[index]
        #expect(tally.failure == nil, "\(tally.failure ?? "")")
        #expect(tally.cursor == total, "reader \(index) ended at the head")
        #expect(
            tally.items + tally.lost == total,
            "reader \(index) was handed or told of every entry once")
        #expect(tally.items >= 1, "reader \(index) was handed at least the last entry")
    }
    #expect(log.head == total)
}

@Test func readersOfASmallRingNeverSeeATornEntry() throws {
    try runShape(words: 4, slots: 2, readerCount: 2, total: 100_000)
}

@Test func readersOfAThreeSlotLogNeverSeeATornEntry() throws {
    try runShape(words: 3, slots: 3, readerCount: 2, total: 100_000)
}

@Test func readersOfAWideEntryNeverSeeATornEntry() throws {
    try runShape(words: 16, slots: 5, readerCount: 2, total: 60_000)
}

@Test func aReaderThatKeepsUpLosesNothing() throws {
    // The log is larger than the run, so no reader can fall out of the window.
    let total: UInt64 = 500
    let log = BroadcastLog(words: 2, slots: Int(total) + 1)
    let publisher = try #require(log.publisher())
    var reader = log.readerOldest()
    for seq in 0..<total {
        publisher.publish(stressEntry(seq, words: 2))
        expectItem(log, &reader, seq, stressEntry(seq, words: 2))
    }
    #expect(reader.lost == 0)
}
