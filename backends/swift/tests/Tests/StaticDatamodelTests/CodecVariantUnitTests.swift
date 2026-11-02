// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A codec whose tail is one of several codecs, selected by a tag it has already read
// (`tests/forge/resources/codec_variant_dispatch.scxml`: the whole of a field; `codec_zenoh_push_body.scxml`:
// the low five bits of a flags carrier, whose other bits are not part of the tag). The fixtures hold no
// numerical oracle, so this is where the behaviour is held: the tag picks the arm, a tag no arm names
// takes the catch-all and is kept, an encode writes the tag field as it stands and then the arm
// held, and a frame that is refused consumes nothing.
final class CodecVariantUnitTests: XCTestCase {
    func testTheTagPicksTheArmItNames() throws {
        var open = SceCursor([0x01, 0x00, 0x05])
        let first = try CodecVariantDispatch.decode(&open)
        XCTAssertEqual(first.msg_id, 1)
        XCTAssertEqual(first.body, .codecVariantSessionOpen(CodecVariantSessionOpen(version: 5)))
        var close = SceCursor([0x02, 0x07, 0xEE])
        let second = try CodecVariantDispatch.decode(&close)
        XCTAssertEqual(second.body, .codecVariantSessionClose(CodecVariantSessionClose(reason: 7)))
        XCTAssertEqual(close.remaining, 1, "what follows the arm is the caller's")
    }

    func testATagNoArmNamesTakesTheCatchAllAndIsKept() throws {
        var cursor = SceCursor([0x09, 0x03])
        let frame = try CodecVariantDispatch.decode(&cursor)
        XCTAssertEqual(frame.msg_id, 9)
        XCTAssertEqual(
            frame.body, .default(tag: 9, body: CodecVariantSessionClose(reason: 3)))
    }

    func testAnArmCutShortRefusesTheFrameAndConsumesNothing() {
        var cursor = SceCursor([0x01, 0x00])
        XCTAssertThrowsError(try CodecVariantDispatch.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testAnEncodeWritesTheTagFieldAsItStandsAndThenTheArm() throws {
        var sink = BytesSink()
        try CodecVariantDispatch(
            msg_id: 1, body: .codecVariantSessionOpen(CodecVariantSessionOpen(version: 5))
        ).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x01, 0x00, 0x05])
        var caught = BytesSink()
        try CodecVariantDispatch(
            msg_id: 9, body: .default(tag: 9, body: CodecVariantSessionClose(reason: 3))
        ).encode(into: &caught)
        XCTAssertEqual(caught.bytes, [0x09, 0x03])
    }

    func testAValueThatHasReadNoFrameHoldsTheArmTheDocumentMarksAsItsDefault() {
        XCTAssertEqual(
            CodecVariantDispatch().body, .codecVariantSessionClose(CodecVariantSessionClose()))
    }

    func testTheFewestBytesAreTheTagAndTheShortestArm() {
        XCTAssertEqual(
            CodecVariantDispatch.frameLength,
            1 + min(CodecVariantSessionOpen.frameLength, CodecVariantSessionClose.frameLength))
    }

    // The tag is the low five bits of `header`; the three above them are another flag and no part of it.
    func testATagInTheBitsOfAFlagIsReadShiftedDownAndMasked() throws {
        var put = SceCursor([0x01, 0x2A])
        let first = try CodecZenohPushBody.decode(&put)
        XCTAssertEqual(first.body, .codecZenohPut(CodecZenohPut(payload: 0x2A)))
        var withOtherBits = SceCursor([0x41, 0x2A])
        XCTAssertEqual(
            try CodecZenohPushBody.decode(&withOtherBits).body,
            .codecZenohPut(CodecZenohPut(payload: 0x2A)),
            "bits above the tag do not change the arm")
        var del = SceCursor([0x02])
        XCTAssertEqual(try CodecZenohPushBody.decode(&del).body, .codecZenohDel(CodecZenohDel()))
    }

    func testTheCatchAllOfAFlagTagKeepsTheBitsAndNotTheWholeCarrier() throws {
        var cursor = SceCursor([0xFF, 0x2A])
        let frame = try CodecZenohPushBody.decode(&cursor)
        XCTAssertEqual(frame.header, 0xFF)
        XCTAssertEqual(frame.body, .default(tag: 0x1F, body: CodecZenohPut(payload: 0x2A)))
    }
}
