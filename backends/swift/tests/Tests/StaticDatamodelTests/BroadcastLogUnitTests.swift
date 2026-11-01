// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The log `sce-codegen orchestrate -l swift` writes for `broadcast_log_swift_ring.scxml`
// (`scripts/regen_static_datamodel_swift.sh`) over the eleven-byte frame of
// `broadcast_log_swift_frame.scxml`: an enum that names the log's constants and gives typed
// `publish` and `read` over the codec's struct, on the runtime's 64-bit row. What is held here is
// what the generated module adds to the runtime, whose own contract `BroadcastLogTests` holds: the
// element is stored as its wire bytes, one length word and the bytes eight to a word,
// little-endian, and a read decodes them only when the runtime says the copy was whole.
final class BroadcastLogUnitTests: XCTestCase {
    private typealias Ring = BroadcastLogSwiftRing

    /// Bytes 0..3 are `sequence`, 4..7 `reading`, 8..9 `status` and 10 `level`, all big-endian.
    private func frame(_ n: UInt32) -> BroadcastLogSwiftFrame {
        BroadcastLogSwiftFrame(
            sequence: 0x0102_0300 &+ n, reading: 0xA0B0_C0D0, status: 0xBEEF, level: 0x7F)
    }

    func testTheConstantsAreTheDocumentsAndTheElementsSize() {
        XCTAssertEqual(Ring.retention, 3)
        XCTAssertEqual(Ring.slots, 4, "one slot more than the retention")
        XCTAssertEqual(Ring.maxEncodedBytes, 11)
        XCTAssertEqual(Ring.maxEncodedBytes, BroadcastLogSwiftFrame.frameLength)
        XCTAssertEqual(Ring.words, 3, "the length word and two words of bytes")
        XCTAssertEqual(Ring.declaredProgress, "wait-free")
        let log = Ring.makeLog()
        XCTAssertEqual(log.words, Ring.words)
        XCTAssertEqual(log.slots, Ring.slots)
        XCTAssertEqual(log.retention, 3)
    }

    func testAFrameIsPublishedAndReadBackAsItself() throws {
        let log = Ring.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        var reader = log.readerOldest()
        XCTAssertEqual(Ring.publish(publisher, frame(0)), .published(seq: 0))
        XCTAssertEqual(Ring.publish(publisher, frame(1)), .published(seq: 1))
        var answer = Ring.read(log, &reader)
        XCTAssertEqual(answer, .item(seq: 0, element: frame(0)))
        answer = Ring.read(log, &reader)
        XCTAssertEqual(answer, .item(seq: 1, element: frame(1)))
        answer = Ring.read(log, &reader)
        XCTAssertEqual(answer, .empty)
        XCTAssertEqual(reader.lost, 0)
    }

    func testAnEntryIsTheLengthWordAndTheBytesLittleEndianEightToAWord() throws {
        let log = Ring.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        XCTAssertEqual(Ring.publish(publisher, frame(0)), .published(seq: 0))
        // The runtime's own view of the entry: the typed module wrote nothing else.
        var raw = log.readerOldest()
        var words = [UInt64](repeating: 0, count: log.words)
        XCTAssertEqual(log.read(&raw, into: &words), .item(seq: 0))
        // The frame on the wire is 01 02 03 00 | A0 B0 C0 D0 | BE EF | 7F.
        XCTAssertEqual(words[0], 11, "the length of the frame")
        XCTAssertEqual(words[1], 0xD0C0_B0A0_0003_0201, "bytes 0 to 7, the first at the low end")
        XCTAssertEqual(words[2], 0x007F_EFBE, "bytes 8 to 10 and nothing above them")
    }

    func testAReaderTheLogLapsIsToldExactlyHowManyItMissed() throws {
        let log = Ring.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        for n in 0..<UInt32(5) {
            XCTAssertEqual(Ring.publish(publisher, frame(n)), .published(seq: UInt64(n)))
        }
        // Five frames in a log that retains three: the window is [2, 5), so a reader at 0
        // missed two and then reads the three that are left.
        var reader = log.readerAt(0)
        var answer = Ring.read(log, &reader)
        XCTAssertEqual(answer, .lagged(skipped: 2))
        XCTAssertEqual(reader.lost, 2)
        for n in 2..<UInt32(5) {
            answer = Ring.read(log, &reader)
            XCTAssertEqual(answer, .item(seq: UInt64(n), element: frame(n)))
        }
        answer = Ring.read(log, &reader)
        XCTAssertEqual(answer, .empty)
    }

    func testALengthAboveTheBoundIsRefusedAndTheReaderMovesPastIt() throws {
        let log = Ring.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        // An entry this module did not write: a stored length of twelve bytes in a log whose
        // element is at most eleven, which an index of the bytes would run off the end of.
        XCTAssertEqual(publisher.publish([12, UInt64.max, UInt64.max]), 0)
        XCTAssertEqual(Ring.publish(publisher, frame(1)), .published(seq: 1))
        var reader = log.readerOldest()
        let refused = Ring.read(log, &reader)
        guard case .unreadable(let seq, let why) = refused else {
            return XCTFail("a length above the bound was answered \(refused)")
        }
        XCTAssertEqual(seq, 0)
        XCTAssertTrue(why.contains("above the 11 bytes"), why)
        XCTAssertEqual(reader.cursor, 1, "the reader moved past the entry it could not read")
        let next = Ring.read(log, &reader)
        XCTAssertEqual(next, .item(seq: 1, element: frame(1)), "the entry after it reads")
    }

    func testAShorterFrameThanTheCodecReadsIsUnreadableAndNotAnIndexOutOfRange() throws {
        let log = Ring.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        // Three bytes are within the bound and are not a frame: the codec answers that it needs
        // more bytes, and the module says the entry does not decode.
        XCTAssertEqual(publisher.publish([3, 0x0003_0201, 0]), 0)
        var reader = log.readerOldest()
        let refused = Ring.read(log, &reader)
        guard case .unreadable(let seq, let why) = refused else {
            return XCTFail("a short frame was answered \(refused)")
        }
        XCTAssertEqual(seq, 0)
        XCTAssertTrue(why.contains("do not decode as a BroadcastLogSwiftFrame"), why)
        XCTAssertEqual(reader.cursor, 1)
    }

    func testTheLogGivesItsPublisherToOneHolderAtATime() throws {
        let log = Ring.makeLog()
        var publisher: BroadcastLogPublisher? = log.publisher()
        XCTAssertNotNil(publisher)
        let refused = log.publisher()
        XCTAssertNil(refused, "a second publisher is refused while the first is held")
        if let held = publisher {
            XCTAssertEqual(Ring.publish(held, frame(0)), .published(seq: 0))
        }
        publisher = nil
        let successor = try XCTUnwrap(log.publisher(), "dropping the publisher gave its place back")
        XCTAssertEqual(Ring.publish(successor, frame(1)), .published(seq: 1))
    }
}
