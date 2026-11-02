// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A field that is on the wire only on a flag of an earlier carrier
// (`tests/forge/resources/codec_present_if_*.scxml`). The fixtures hold no numerical oracle, so
// this is where the behaviour is held: the field is `nil` when its gate is closed, a decode reads
// it only when the gate the carrier it has read opens, and an encode writes it when it is given.
final class CodecPresentIfUnitTests: XCTestCase {
    func testAFieldWhoseFlagIsClearIsNilAndNotOnTheWire() throws {
        var cursor = SceCursor([0x00, 0xEE])
        let frame = try CodecPresentIfBasic.decode(&cursor)
        XCTAssertNil(frame.seq)
        XCTAssertEqual(cursor.remaining, 1, "the byte after the carrier is not the field's")
    }

    func testAFieldWhoseFlagIsSetIsReadAfterTheCarrier() throws {
        var cursor = SceCursor([0x01, 0x12, 0x34])
        let frame = try CodecPresentIfBasic.decode(&cursor)
        XCTAssertEqual(frame.seq, 0x1234)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAGivenFieldIsWrittenAndAnAbsentOneIsNot() throws {
        var sink = BytesSink()
        try CodecPresentIfBasic(flags: 0x01, seq: 0x1234).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x01, 0x12, 0x34])
        var bare = BytesSink()
        try CodecPresentIfBasic(flags: 0x00, seq: nil).encode(into: &bare)
        XCTAssertEqual(bare.bytes, [0x00])
    }

    func testAFrameWhoseGateIsOpenAndWhoseFieldIsShortConsumesNothing() {
        var cursor = SceCursor([0x01, 0x12])
        XCTAssertThrowsError(try CodecPresentIfBasic.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2, "a refused frame leaves the cursor where it was")
    }

    func testANegatedGateOpensOnAClearBit() throws {
        var open = SceCursor([0x00, 0x00, 0x2A])
        XCTAssertEqual(try CodecPresentIfNegation.decode(&open).seq, 0x002A)
        var closed = SceCursor([0x01])
        XCTAssertNil(try CodecPresentIfNegation.decode(&closed).seq)
    }

    func testADisjunctionOpensOnEitherFlag() throws {
        for carrier: UInt8 in [0x01, 0x02, 0x03] {
            var cursor = SceCursor([carrier, 0x00, 0x07])
            XCTAssertEqual(try CodecPresentIfDisjunction.decode(&cursor).seq, 7)
        }
        var cursor = SceCursor([0x00])
        XCTAssertNil(try CodecPresentIfDisjunction.decode(&cursor).seq)
    }

    func testAGatedTailIsTheRestOfTheFrameWhenOpen() throws {
        var open = SceCursor([0x01, 0xAA, 0xBB])
        XCTAssertEqual(try CodecPresentIfTail.decode(&open).payload, [0xAA, 0xBB])
        var closed = SceCursor([0x00, 0xAA])
        let frame = try CodecPresentIfTail.decode(&closed)
        XCTAssertNil(frame.payload)
        XCTAssertEqual(closed.remaining, 1)
    }

    func testAGatedVariableLengthIntegerIsReadAsAChain() throws {
        var open = SceCursor([0x01, 0xAC, 0x02])
        XCTAssertEqual(try CodecPresentIfVle.decode(&open).optional_id, 300)
        var sink = BytesSink()
        try CodecPresentIfVle(flags: 0x01, optional_id: 300).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x01, 0xAC, 0x02])
    }
}
