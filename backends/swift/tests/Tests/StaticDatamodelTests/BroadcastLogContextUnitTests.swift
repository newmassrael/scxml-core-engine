// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The logs `sce-codegen orchestrate -l swift` writes for the shared conformance documents
// `tests/forge/conformance/broadcast_log_context` (`scripts/regen_static_datamodel_swift.sh`): logs over
// an element whose codec asks its caller for what its wire form does not carry (ADR 0062 section 4).
// An entry is the length word, the context words, then the bytes, so that a reader decodes the entry
// under what it was published under: `Chosen` stores the tag of the arm a value holds, `Flagged` the
// flag input the value was encoded under.
final class BroadcastLogContextUnitTests: XCTestCase {
    // MARK: - A variant whose tag its caller supplies

    private typealias Chosen = BroadcastLogConformanceChosen
    private typealias Choice = BroadcastLogConformanceChoice

    private func small(_ payload: UInt8) -> Choice {
        Choice(body: .broadcastLogConformanceChoiceSmall(BroadcastLogConformanceChoiceSmall(payload: payload)))
    }

    private func large(_ payload: UInt16) -> Choice {
        Choice(body: .broadcastLogConformanceChoiceLarge(BroadcastLogConformanceChoiceLarge(payload: payload)))
    }

    func testAnEntryOfAVariantHoldsTheLengthTheTagAndTheBytes() {
        XCTAssertEqual(Chosen.contextWords, 1, "one word that holds the variant's tag")
        XCTAssertEqual(Chosen.maxEncodedBytes, 2, "the large arm is the longest")
        XCTAssertEqual(Chosen.words, 1 + 1 + 1, "the length word, the context word and a word of bytes")
    }

    func testTheTagOfAValueIsTheTagOfTheArmItHolds() {
        XCTAssertEqual(small(7).tag, 1, "tag 1 selects the small arm")
        XCTAssertEqual(large(7).tag, 0, "any other tag, the large arm")
    }

    func testEachArmIsReadBackUnderTheTagItWasPublishedAs() throws {
        let log = Chosen.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        var reader = log.readerOldest()
        XCTAssertEqual(Chosen.publish(publisher, small(0x5A)), .published(seq: 0))
        XCTAssertEqual(Chosen.publish(publisher, large(0xBEEF)), .published(seq: 1))
        XCTAssertEqual(Chosen.read(log, &reader), .item(seq: 0, element: small(0x5A)))
        XCTAssertEqual(Chosen.read(log, &reader), .item(seq: 1, element: large(0xBEEF)))
        XCTAssertEqual(Chosen.read(log, &reader), .empty)
    }

    func testTheEntryHoldsTheTagInItsContextWordAndTheBytesAfterIt() throws {
        let log = Chosen.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        XCTAssertEqual(Chosen.publish(publisher, small(0x5A)), .published(seq: 0))
        var raw = log.readerOldest()
        var words = [UInt64](repeating: 0, count: log.words)
        XCTAssertEqual(log.read(&raw, into: &words), .item(seq: 0))
        XCTAssertEqual(words[0], 1, "the small arm is one byte")
        XCTAssertEqual(words[1], 1, "the context word is the tag of the small arm")
        XCTAssertEqual(words[2], 0x5A, "the bytes follow the context")
    }

    // MARK: - A flag input its caller supplies

    private typealias Flagged = BroadcastLogConformanceFlagged
    private typealias Event = BroadcastLogConformanceFlaggedEvent

    /// The one flag input the element's codec takes, as the log names it.
    private func flags(_ hasValue: Bool) -> Flagged.Flags {
        Flagged.Flags(hasValue: hasValue)
    }

    func testAnEntryOfAFlaggedElementHoldsTheLengthTheFlagWordAndTheBytes() {
        XCTAssertEqual(Flagged.contextWords, 1, "one word holds the flag input")
        XCTAssertEqual(Flagged.maxEncodedBytes, 3, "the id, and the value under the flag")
        XCTAssertEqual(Flagged.words, 1 + 1 + 1)
    }

    func testAValueWithItsFlagIsReadBackWithTheFlagItWasPublishedUnder() throws {
        let log = Flagged.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        var reader = log.readerOldest()
        let element = Event(sensor_id: 9, value: 0x1234)
        XCTAssertEqual(Flagged.publish(publisher, element, flags: flags(true)), .published(seq: 0))
        XCTAssertEqual(Flagged.read(log, &reader), .item(seq: 0, element: element, flags: flags(true)))
    }

    func testAValueWithoutItsFlagIsOneByteAndReadBackWithoutIt() throws {
        let log = Flagged.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        var reader = log.readerOldest()
        let element = Event(sensor_id: 9, value: nil)
        XCTAssertEqual(Flagged.publish(publisher, element, flags: flags(false)), .published(seq: 0))
        XCTAssertEqual(Flagged.read(log, &reader), .item(seq: 0, element: element, flags: flags(false)))
    }

    func testTheFlagIsABitOfTheContextWordAndTheBytesFollowIt() throws {
        let log = Flagged.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        let element = Event(sensor_id: 9, value: 0x1234)
        XCTAssertEqual(Flagged.publish(publisher, element, flags: flags(true)), .published(seq: 0))
        var raw = log.readerOldest()
        var words = [UInt64](repeating: 0, count: log.words)
        XCTAssertEqual(log.read(&raw, into: &words), .item(seq: 0))
        XCTAssertEqual(words[0], 3, "the id and the two bytes of the value")
        XCTAssertEqual(words[1], 1, "flag 0 is bit 0 of the first context word")
        // The id is byte 0 and the value follows it, little-endian: 0x34 then 0x12.
        XCTAssertEqual(words[2], 0x0012_3409, "the bytes, little-endian, eight to a word")
    }

    func testTwoFlagsOfOneLogAreKeptApartBySlot() throws {
        let log = Flagged.makeLog()
        let publisher = try XCTUnwrap(log.publisher())
        var reader = log.readerOldest()
        let with = Event(sensor_id: 1, value: 2)
        let without = Event(sensor_id: 3, value: nil)
        _ = Flagged.publish(publisher, with, flags: flags(true))
        _ = Flagged.publish(publisher, without, flags: flags(false))
        XCTAssertEqual(Flagged.read(log, &reader), .item(seq: 0, element: with, flags: flags(true)))
        XCTAssertEqual(Flagged.read(log, &reader), .item(seq: 1, element: without, flags: flags(false)))
    }
}
