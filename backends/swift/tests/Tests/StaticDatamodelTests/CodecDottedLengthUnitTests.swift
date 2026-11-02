// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A length held in the bits of a flag (`tests/forge/resources/codec_length_ref_dotted_basic.scxml`: a
// byte whose low four bits are a header and whose high four say how many bytes follow). The fixtures
// hold no numerical oracle, so this is where the behaviour is held: the length is the bits of the
// carrier shifted down and masked, the bits above and below them are not part of it, and a frame
// that is refused consumes nothing.
final class CodecDottedLengthUnitTests: XCTestCase {
    func testTheLengthIsTheBitsOfTheCarrierShiftedDownAndMasked() throws {
        var cursor = SceCursor([0x32, 0x01, 0x02, 0x03, 0x99])
        let frame = try CodecLengthRefDottedBasic.decode(&cursor)
        XCTAssertEqual(frame.carrier, 0x32)
        XCTAssertEqual(frame.payload, [0x01, 0x02, 0x03], "the high four bits say 3; the low four are the header")
        XCTAssertEqual(cursor.remaining, 1, "what follows the bytes is the caller's")
    }

    func testALengthOfZeroTakesNoBytes() throws {
        var cursor = SceCursor([0x05])
        let frame = try CodecLengthRefDottedBasic.decode(&cursor)
        XCTAssertEqual(frame.payload, [])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testTheLargestLengthTheBitsCanSayIsTaken() throws {
        var cursor = SceCursor([0xF0] + [UInt8](repeating: 0x7A, count: 15))
        XCTAssertEqual(try CodecLengthRefDottedBasic.decode(&cursor).payload.count, 15)
    }

    func testAFrameShorterThanItsLengthSaysIsRefusedAndConsumesNothing() {
        var cursor = SceCursor([0x40, 0x01, 0x02])
        XCTAssertThrowsError(try CodecLengthRefDottedBasic.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 3)
    }

    func testAnEncodeWritesTheCarrierAsItStandsAndThenTheBytes() throws {
        var sink = BytesSink()
        try CodecLengthRefDottedBasic(carrier: 0x30, payload: [0x01, 0x02, 0x03]).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x30, 0x01, 0x02, 0x03])
    }

    func testTheFlagPropertiesReadAndWriteTheSameBits() {
        var frame = CodecLengthRefDottedBasic()
        frame.payloadLen = 9
        XCTAssertEqual(frame.carrier, 0x90)
        frame.hdr = 0x5
        XCTAssertEqual(frame.carrier, 0x95)
        XCTAssertEqual(frame.payloadLen, 9)
    }
}
