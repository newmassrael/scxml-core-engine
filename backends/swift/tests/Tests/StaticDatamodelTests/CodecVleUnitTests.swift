// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A variable-length integer (`tests/forge/resources/codec_vle_zint_u64.scxml`, and
// `codec_ext_timestamp.scxml` where the fields after it have no byte of their own). The numerical
// oracle holds the round trips; what is held here is what it cannot say: the canonical chain (a
// 64-bit value takes at most 9 bytes), that a frame cut short or a chain wider than its type is
// refused with the cursor where it was, and that a field after a chain begins where the chain ended.
final class CodecVleUnitTests: XCTestCase {
    func testAValueIsTheFewestBytesThatHoldIt() throws {
        func bytes(_ value: UInt64) throws -> [UInt8] {
            var sink = BytesSink()
            try CodecVleZintU64(value: value).encode(into: &sink)
            return sink.bytes
        }
        XCTAssertEqual(try bytes(0), [0x00])
        XCTAssertEqual(try bytes(127), [0x7F])
        XCTAssertEqual(try bytes(300), [0xAC, 0x02])
        XCTAssertEqual(try bytes(UInt64.max), [UInt8](repeating: 0xFF, count: 9))
    }

    func testAChainIsReadBackAsTheValueItWas() throws {
        for value: UInt64 in [0, 1, 127, 128, 300, 1 << 35, 1 << 63, .max] {
            var sink = BytesSink()
            try CodecVleZintU64(value: value).encode(into: &sink)
            var cursor = SceCursor(sink.bytes + [0xEE])
            XCTAssertEqual(try CodecVleZintU64.decode(&cursor).value, value)
            XCTAssertEqual(cursor.remaining, 1, "the read stops at the end of the chain")
        }
    }

    func testAChainCutShortIsRefusedAndConsumesNothing() {
        var cursor = SceCursor([0x80, 0x80])
        XCTAssertThrowsError(try CodecVleZintU64.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testAFieldAfterAChainBeginsWhereTheChainEnded() throws {
        var frame = CodecExtTimestamp()
        frame.time = 300
        frame.zid_size = 3
        frame.zid = [1, 2, 3]
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0xAC, 0x02, 3, 1, 2, 3])
        var cursor = SceCursor(sink.bytes)
        let decoded = try CodecExtTimestamp.decode(&cursor)
        XCTAssertEqual(decoded, frame)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAFrameWhoseBytesAreCutShortConsumesNothingOfItsChain() {
        // The chain and the size are whole, and the three bytes the size counts are two.
        var cursor = SceCursor([0xAC, 0x02, 3, 1, 2])
        XCTAssertThrowsError(try CodecExtTimestamp.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 5, "a refused frame leaves the cursor where it was")
    }
}
