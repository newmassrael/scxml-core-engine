// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A codec laid in place inside another and bounded by a length an earlier field states
// (`tests/forge/resources/codec_zenoh_timestamp_ext.scxml`: a variable-length size, then a
// `CodecZenohTimestamp` of a variable-length time, a variable-length id size and that many bytes).
// The fixtures hold no numerical oracle, so this is where the behaviour is held: the embedded codec
// is read from the bytes the length gives and from no others, it need not take all of them, the frame
// moves past all of them, and a frame that is refused consumes nothing.
final class CodecEmbedUnitTests: XCTestCase {
    func testAnEmbeddedCodecIsReadFromTheBytesTheLengthGives() throws {
        var cursor = SceCursor([0x04, 0x05, 0x02, 0xAA, 0xBB, 0x77])
        let frame = try CodecZenohTimestampExt.decode(&cursor)
        XCTAssertEqual(frame.ext_size, 4)
        XCTAssertEqual(frame.ts.time, 5)
        XCTAssertEqual(frame.ts.zid, [0xAA, 0xBB])
        XCTAssertEqual(cursor.remaining, 1, "what follows the bounded bytes is the caller's")
    }

    func testTheBodyNeedNotTakeAllTheBytesAndTheFrameMovesPastThemAll() throws {
        var cursor = SceCursor([0x05, 0x05, 0x02, 0xAA, 0xBB, 0xEE])
        let frame = try CodecZenohTimestampExt.decode(&cursor)
        XCTAssertEqual(frame.ts.zid, [0xAA, 0xBB])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testTheBodyIsNotReadPastTheBytesTheLengthGives() {
        // The length says 2 bytes, and the body needs 4: the two bytes after them are not its.
        var cursor = SceCursor([0x02, 0x05, 0x02, 0xAA, 0xBB])
        XCTAssertThrowsError(try CodecZenohTimestampExt.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 5, "a refused frame leaves the cursor where it was")
    }

    func testALengthPastTheFrameIsRefusedAndConsumesNothing() {
        var cursor = SceCursor([0x09, 0x05])
        XCTAssertThrowsError(try CodecZenohTimestampExt.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testAnEncodeWritesTheLengthAndThenTheEmbeddedBytes() throws {
        let frame = CodecZenohTimestampExt(
            ext_size: 4, ts: CodecZenohTimestamp(time: 5, zid_len: 2, zid: [0xAA, 0xBB]))
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x04, 0x05, 0x02, 0xAA, 0xBB])
    }

    func testTheFewestBytesAreTheLengthAndTheEmbeddedCodecsOwn() {
        XCTAssertEqual(
            CodecZenohTimestampExt.frameLength, 1 + CodecZenohTimestamp.frameLength)
    }
}
