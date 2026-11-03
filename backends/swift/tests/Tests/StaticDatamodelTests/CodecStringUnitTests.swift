// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A text of a length the frame states (`tests/forge/resources/codec_zenoh_locator.scxml`: a
// variable-length size, then that many UTF-8 bytes, at most 128; `codec_present_if_string.scxml`
// where the size and the text are on the wire under one gate; `codec_embed_basic.scxml` where the
// locator is laid in place). The fixtures hold no numerical oracle, so this is where the behaviour is
// held: the bytes are read as UTF-8 and bytes that are not are refused and not repaired, the bound
// counts the bytes on the wire and not the characters, and a frame that is refused consumes nothing.
final class CodecStringUnitTests: XCTestCase {
    func testATextIsTheUtf8BytesItsLengthSays() throws {
        var cursor = SceCursor([0x03, 0x74, 0x63, 0x70, 0x77])
        let frame = try CodecZenohLocator.decode(&cursor)
        XCTAssertEqual(frame.locator, "tcp")
        XCTAssertEqual(cursor.remaining, 1, "what follows the text is the caller's")
    }

    func testAMultiByteCharacterIsTheBytesOfItsScalar() throws {
        var cursor = SceCursor([0x05, 0x61, 0xC3, 0xA9, 0xE2, 0x82])
        XCTAssertThrowsError(try CodecZenohLocator.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .invalidUtf8, "a scalar cut off is not text")
        }
        var whole = SceCursor([0x06, 0x61, 0xC3, 0xA9, 0xE2, 0x82, 0xAC])
        XCTAssertEqual(try CodecZenohLocator.decode(&whole).locator, "a\u{E9}\u{20AC}")
    }

    func testBytesThatAreNotUtf8AreRefusedAndConsumeNothing() {
        var cursor = SceCursor([0x02, 0xC3, 0x28])
        XCTAssertThrowsError(try CodecZenohLocator.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .invalidUtf8)
        }
        XCTAssertEqual(cursor.remaining, 3, "a refused frame leaves the cursor where it was")
    }

    func testATextShorterThanItsLengthSaysNeedsMoreBytes() {
        var cursor = SceCursor([0x04, 0x61, 0x62])
        XCTAssertThrowsError(try CodecZenohLocator.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 3)
    }

    func testAnEncodeWritesTheUtf8BytesOfTheText() throws {
        var sink = BytesSink()
        try CodecZenohLocator(locator_len: 2, locator: "\u{E9}").encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x02, 0xC3, 0xA9])
    }

    func testALengthPastTheBoundIsRefusedBeforeTheBytesAreCut() {
        // 256 as a variable-length integer: past the bound of 255, and the frame holds none of the
        // bytes it says.
        var cursor = SceCursor([0x80, 0x02])
        XCTAssertThrowsError(try CodecZenohLocator.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .lengthOutOfRange)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    // 128 characters of two bytes each are 256 bytes: past a bound of 255 bytes, though not of 255
    // characters.
    func testTheBoundCountsTheBytesOnTheWireAndNotTheCharacters() throws {
        let text = String(repeating: "\u{E9}", count: 128)
        var sink = BytesSink()
        XCTAssertThrowsError(try CodecZenohLocator(locator_len: 256, locator: text).encode(into: &sink)) {
            error in
            XCTAssertEqual(error as? CodecError, .lengthOutOfRange)
        }
        XCTAssertEqual(sink.bytes, [], "a refused encode writes nothing")
        let atTheBound = String(repeating: "a", count: 255)
        try CodecZenohLocator(locator_len: 255, locator: atTheBound).encode(into: &sink)
        XCTAssertEqual(sink.bytes.count, 2 + 255, "the size of 255 is two bytes of variable-length integer")
    }

    func testAGatedTextWithAGatedLengthIsAbsentWhenItsGateIsClosed() throws {
        var closed = SceCursor([0x00])
        let absent = try CodecPresentIfString.decode(&closed)
        XCTAssertNil(absent.text_len)
        XCTAssertNil(absent.text)
        var open = SceCursor([0x01, 0x02, 0x68, 0x69])
        let present = try CodecPresentIfString.decode(&open)
        XCTAssertEqual(present.text, "hi")
        var sink = BytesSink()
        try present.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x01, 0x02, 0x68, 0x69])
    }

    func testAnEmbeddedLocatorIsReadInPlaceAfterTheFieldsBeforeIt() throws {
        var cursor = SceCursor([0x07, 0x03, 0x74, 0x63, 0x70])
        let frame = try CodecEmbedBasic.decode(&cursor)
        XCTAssertEqual(frame.tag, 7)
        XCTAssertEqual(frame.locator.locator, "tcp")
        XCTAssertEqual(cursor.remaining, 0)
        XCTAssertEqual(CodecEmbedBasic.frameLength, 1 + CodecZenohLocator.frameLength)
    }
}
