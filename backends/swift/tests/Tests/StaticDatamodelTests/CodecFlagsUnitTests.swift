// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The flags a codec's carrier names (`sce:flags`), written by `sce-codegen generate -l swift`
// (`scripts/regen_static_datamodel_swift.sh`) as properties of the struct that read and write bits
// of the carrier. The units are the documents `tests/forge/resources/codec_flags_basic.scxml`
// (four single-bit flags of a `uint8` header) and `codec_qos_byte.scxml` (flags of one, two and
// three bits in one byte). The numerical oracle holds neither (`oracle_eligible` is false), so the
// flags are held here.
final class CodecFlagsUnitTests: XCTestCase {
    // `reliable` is bit 7, `more` bit 6, `drop` bit 5, `first` bit 4.
    func testAFlagOfOneBitReadsAndWritesItsBitAndNoOther() {
        var frame = CodecFlagsBasic()
        XCTAssertFalse(frame.reliable)
        frame.reliable = true
        XCTAssertEqual(frame.header, 0x80)
        frame.first = true
        XCTAssertEqual(frame.header, 0x90)
        frame.reliable = false
        XCTAssertEqual(frame.header, 0x10)
        XCTAssertTrue(frame.first)
        XCTAssertFalse(frame.more)
        XCTAssertFalse(frame.drop)
    }

    func testTheFlagsOfADecodedFrameAreTheBitsOfItsCarrier() throws {
        var cursor = SceCursor([0xB0])
        let frame = try CodecFlagsBasic.decode(&cursor)
        XCTAssertTrue(frame.reliable)
        XCTAssertFalse(frame.more)
        XCTAssertTrue(frame.drop)
        XCTAssertTrue(frame.first)
    }

    func testTheFlagsDoNotChangeTheWire() throws {
        var frame = CodecFlagsBasic()
        frame.more = true
        frame.drop = true
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x60])
    }

    // `priority` is bits 0..2, `reliable` bit 3, `congestion` bits 4..5, `express` bit 6 and
    // `reserved` bit 7.
    func testAFlagOfSeveralBitsReadsAndWritesItsRangeAndNoOther() {
        var frame = CodecQosByte()
        frame.priority = 5
        frame.congestion = 2
        XCTAssertEqual(frame.qos, 0x25)
        XCTAssertEqual(frame.priority, 5)
        XCTAssertEqual(frame.congestion, 2)
        frame.priority = 0
        XCTAssertEqual(frame.qos, 0x20, "writing a range clears that range and no other")
    }

    func testAValueWiderThanTheFlagIsCutToIt() {
        var frame = CodecQosByte()
        frame.reliable = true
        frame.priority = 0xFF
        XCTAssertEqual(frame.priority, 7)
        XCTAssertEqual(frame.qos, 0x0F, "the bits past the three of priority are not written")
        frame.congestion = 0xFF
        XCTAssertEqual(frame.congestion, 3)
        XCTAssertEqual(frame.qos, 0x3F)
        XCTAssertTrue(frame.reliable)
        XCTAssertFalse(frame.express)
    }

    func testTheFlagsOfADecodedByteAreItsRanges() throws {
        var cursor = SceCursor([0b0110_1011])
        let frame = try CodecQosByte.decode(&cursor)
        XCTAssertEqual(frame.priority, 3)
        XCTAssertTrue(frame.reliable)
        XCTAssertEqual(frame.congestion, 2)
        XCTAssertTrue(frame.express)
        XCTAssertFalse(frame.reserved)
    }
}
