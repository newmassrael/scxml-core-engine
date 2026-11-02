// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A variant whose tag is a bit of the next byte, which is peeked and not read
// (`tests/forge/resources/codec_variant_peek_basic.scxml`; its arms are `CodecPeekArmA`, a header
// whose `kind` bit is clear and a byte, and `CodecPeekArmB`, a header whose `kind` bit is set and two
// bytes). The fixtures hold no numerical oracle, so this is where the behaviour is held: the byte
// chooses the arm and stays for the arm to read as its own header, and a frame that is refused
// consumes nothing.
final class CodecVariantPeekUnitTests: XCTestCase {
    func testTheBitOfTheNextByteChoosesTheArmAndTheArmReadsThatByteAsItsHeader() throws {
        var a = SceCursor([0x00, 0x2A])
        XCTAssertEqual(
            try CodecVariantPeekBasic.decode(&a).body,
            .codecPeekArmA(CodecPeekArmA(header: 0x00, payload: 0x2A)))
        XCTAssertEqual(a.remaining, 0)
        var b = SceCursor([0x01, 0x12, 0x34, 0x99])
        XCTAssertEqual(
            try CodecVariantPeekBasic.decode(&b).body,
            .codecPeekArmB(CodecPeekArmB(header: 0x01, payload: 0x1234)))
        XCTAssertEqual(b.remaining, 1, "what follows the arm is the caller's")
    }

    func testTheBitsOfTheByteAboveTheTagAreTheArmsAndDoNotChangeTheChoice() throws {
        var cursor = SceCursor([0xFE, 0x07])
        XCTAssertEqual(
            try CodecVariantPeekBasic.decode(&cursor).body,
            .codecPeekArmA(CodecPeekArmA(header: 0xFE, payload: 0x07)))
    }

    func testAnEmptyCursorHasNoByteToPeekAndConsumesNothing() {
        var cursor = SceCursor([])
        XCTAssertThrowsError(try CodecVariantPeekBasic.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAnArmCutShortRefusesTheFrameAndConsumesNothing() {
        var cursor = SceCursor([0x01, 0x12])
        XCTAssertThrowsError(try CodecVariantPeekBasic.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testAnEncodeWritesTheArmAloneForThePeekedByteIsItsHeader() throws {
        var sink = BytesSink()
        try CodecVariantPeekBasic(
            body: .codecPeekArmB(CodecPeekArmB(header: 0x01, payload: 0x1234))
        ).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x01, 0x12, 0x34])
    }
}
