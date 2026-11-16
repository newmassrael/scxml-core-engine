// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The bytes a frame carries after its fixed part (`tests/forge/resources/codec_tail.scxml` and
// `codec_length_ref.scxml`, and `codec_scout_zid_body.scxml` for the arithmetic on a length). The
// numerical oracle holds the round trips; what is held here is what the oracle cannot say: a frame
// that is too short or says more than it holds is refused and consumes nothing, and a frame that is
// longer than its field leaves the rest for the caller.
final class CodecVariableUnitTests: XCTestCase {
    func testATailTakesEverythingToTheEndOfTheInput() throws {
        var cursor = SceCursor([1, 2, 10, 20, 30])
        let frame = try CodecTail.decode(&cursor)
        XCTAssertEqual(frame.payload.bytes, [10, 20, 30])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testATailMayBeEmpty() throws {
        var cursor = SceCursor([1, 2])
        XCTAssertEqual(try CodecTail.decode(&cursor).payload.bytes, [])
    }

    func testAFrameShorterThanItsFixedPartIsRefusedAndConsumesNothing() {
        var cursor = SceCursor([1])
        XCTAssertThrowsError(try CodecTail.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 1)
    }

    func testALengthRefTakesExactlyTheBytesItsLengthSays() throws {
        var cursor = SceCursor([1, 3, 10, 20, 30, 99, 98])
        let frame = try CodecLengthRef.decode(&cursor)
        XCTAssertEqual(frame.msgId, 1)
        // The length is the encode's to derive (docs/adr/0081 §3): the value holds the bytes alone.
        XCTAssertEqual(frame.payload.bytes, [10, 20, 30])
        XCTAssertEqual(cursor.remaining, 2, "what follows the field is the caller's")
    }

    func testALengthRefThatSaysMoreThanTheFrameHoldsIsRefusedAndConsumesNothing() {
        var cursor = SceCursor([1, 5, 10, 20])
        XCTAssertThrowsError(try CodecLengthRef.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 4)
    }

    // The field is bounded by `sce:max-size="32"`. A length past the bound is not a frame of the
    // document, so it is refused for that and not waited on as a frame that is still arriving.
    func testALengthPastTheBoundIsRefusedForItAndNotWaitedOn() {
        var cursor = SceCursor([1, 255, 10])
        XCTAssertThrowsError(try CodecLengthRef.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .lengthOutOfRange)
        }
        XCTAssertEqual(cursor.remaining, 3, "a refused frame leaves the cursor where it was")
    }

    func testALengthAtTheBoundIsTaken() throws {
        var cursor = SceCursor([1, 32] + [UInt8](repeating: 7, count: 32))
        XCTAssertEqual(try CodecLengthRef.decode(&cursor).payload.byteCount, 32)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testATailPastTheBoundIsRefusedAndConsumesNothing() {
        var cursor = SceCursor([1, 2] + [UInt8](repeating: 7, count: 33))
        XCTAssertThrowsError(try CodecTail.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .lengthOutOfRange)
        }
        XCTAssertEqual(cursor.remaining, 35)
        var atTheBound = SceCursor([1, 2] + [UInt8](repeating: 7, count: 32))
        XCTAssertEqual(try CodecTail.decode(&atTheBound).payload.byteCount, 32)
    }

    // The bound is on the length the document derives: `zid_len_m1 + 1`, at most 16, so a carried
    // 16 says 17.
    func testTheBoundIsJudgedAfterTheArithmeticOfTheLength() throws {
        var past = SceCursor([16] + [UInt8](repeating: 1, count: 17))
        XCTAssertThrowsError(try CodecScoutZidBody.decode(&past)) { error in
            XCTAssertEqual(error as? CodecError, .lengthOutOfRange)
        }
        var atTheBound = SceCursor([15] + [UInt8](repeating: 1, count: 16))
        XCTAssertEqual(try CodecScoutZidBody.decode(&atTheBound).zid.byteCount, 16)
    }

    // A value past the bound is no value of the field's type (docs/adr/0065 §6 R1), so no encode is
    // ever handed one.
    func testAValuePastTheBoundIsNoValueOfTheField() {
        XCTAssertNil(BoundedBytes<CodecTail.PayloadBound>([UInt8](repeating: 7, count: 33)))
        XCTAssertNotNil(BoundedBytes<CodecTail.PayloadBound>([UInt8](repeating: 7, count: 32)))
    }

    // `zid_len_m1` states the length less one, and the document adds it back (`length-arith="+1"`).
    func testTheArithmeticOfALengthIsApplied() throws {
        var cursor = SceCursor([2, 0xAA, 0xBB, 0xCC, 0xDD])
        let frame = try CodecScoutZidBody.decode(&cursor)
        XCTAssertEqual(frame.zid.bytes, [0xAA, 0xBB, 0xCC])
        XCTAssertEqual(cursor.remaining, 1)
        // The encode derives the length less one from the bytes it writes.
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [2, 0xAA, 0xBB, 0xCC])
    }

    func testAnEncodeWritesTheFixedPartAndThenTheBytes() throws {
        let frame = CodecLengthRef(
            msgId: 7, payload: try XCTUnwrap(BoundedBytes<CodecLengthRef.PayloadBound>([0x10, 0x20])))
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [7, 2, 0x10, 0x20], "the length is derived from the bytes")
    }

    func testABoundedSinkTooSmallForTheWholeFrameHoldsNoneOfIt() throws {
        var frame = CodecTail()
        frame.payload = try XCTUnwrap(BoundedBytes<CodecTail.PayloadBound>([1, 2, 3, 4]))
        var sink = BoundedSink(capacity: 3)
        XCTAssertThrowsError(try frame.encode(into: &sink)) { error in
            XCTAssertEqual(error as? CodecError, .bufferOverflow)
        }
        XCTAssertEqual(sink.position, 0)
    }
}
