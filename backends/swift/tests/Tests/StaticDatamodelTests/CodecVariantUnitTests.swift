// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A codec whose tail is one of several codecs, selected by a tag it has already read
// (`tests/forge/resources/codec_variant_dispatch.scxml`: the whole of a field; `codec_zenoh_push_body.scxml`:
// the low five bits of a flags carrier, whose other bits are not part of the tag). The fixtures hold no
// byte cases of the catalog (`numerical_reference.json` `sources`) hold the frames each arm and the
// catch-all write and read back, the tag derived from the arm (docs/adr/0081 §3); what is held here
// is what no case states: the decode leaves what follows the arm to its caller, a tag no arm names
// is kept by the catch-all within its bits, and a frame that is refused consumes nothing.
final class CodecVariantUnitTests: XCTestCase {
    func testTheTagPicksTheArmItNames() throws {
        var open = SceCursor([0x01, 0x00, 0x05])
        let first = try CodecVariantDispatch.decode(&open)
        XCTAssertEqual(first.body.tag, 1)
        XCTAssertEqual(first.body, .codecVariantSessionOpen(CodecVariantSessionOpen(version: 5)))
        var close = SceCursor([0x02, 0x07, 0xEE])
        let second = try CodecVariantDispatch.decode(&close)
        XCTAssertEqual(second.body, .codecVariantSessionClose(CodecVariantSessionClose(reason: 7)))
        XCTAssertEqual(close.remaining, 1, "what follows the arm is the caller's")
    }

    func testATagNoArmNamesTakesTheCatchAllAndIsKept() throws {
        var cursor = SceCursor([0x09, 0x03])
        let frame = try CodecVariantDispatch.decode(&cursor)
        XCTAssertEqual(frame.body.tag, 9)
        let tag = try XCTUnwrap(CodecVariantDispatchUnknownTag(9))
        XCTAssertEqual(frame.body, .default(tag: tag, body: CodecVariantSessionClose(reason: 3)))
    }

    // docs/adr/0065 §6 R3: the catch-all holds a tag no arm names, within the bits it is read from,
    // so a catch-all written is read back as the catch-all.
    func testTheCatchAllsTagAdmitsNoArmsTagAndNoTagPastItsBits() {
        XCTAssertNil(CodecVariantDispatchUnknownTag(0x01), "an arm names it")
        XCTAssertNil(CodecVariantDispatchUnknownTag(0x02), "an arm names it")
        XCTAssertEqual(CodecVariantDispatchUnknownTag(0xFF)?.value, 0xFF)
        XCTAssertNil(CodecZenohPushBodyUnknownTag(0x20), "past the five bits of the flag")
        XCTAssertEqual(CodecZenohPushBodyUnknownTag(0x1F)?.value, 0x1F)
    }

    func testAnArmCutShortRefusesTheFrameAndConsumesNothing() {
        var cursor = SceCursor([0x01, 0x00])
        XCTAssertThrowsError(try CodecVariantDispatch.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
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
        let tag = try XCTUnwrap(CodecZenohPushBodyUnknownTag(0x1F))
        XCTAssertEqual(frame.body, .default(tag: tag, body: CodecZenohPut(payload: 0x2A)))
    }
}
