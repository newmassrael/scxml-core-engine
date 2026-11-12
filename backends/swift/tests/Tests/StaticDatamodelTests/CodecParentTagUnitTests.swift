// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A codec whose header carries, in a flag, the tag of the variant it embeds
// (`tests/forge/resources/codec_source_parent_tag.scxml`: `hdr.kind`, one bit, then
// the arm's body; arm 0 is a 16-bit version, arm 1 a one-byte reason). A decode hands the variant the
// flag as its tag. An encode is what the dispatch drives the other way: the bit of the flag is the tag
// of the arm the value holds, whatever the stored header says, and every other bit of the header is
// kept.
final class CodecParentTagUnitTests: XCTestCase {
    private func decode(_ bytes: [UInt8]) throws -> CodecSourceParentTag {
        var cursor = SceCursor(bytes)
        let frame = try CodecSourceParentTag.decode(&cursor)
        XCTAssertEqual(cursor.remaining, 0)
        return frame
    }

    private func encoded(_ frame: CodecSourceParentTag) throws -> [UInt8] {
        var sink = BytesSink()
        try frame.encode(into: &sink)
        return sink.bytes
    }

    func testTheFlagSelectsTheArmThatIsRead() throws {
        XCTAssertEqual(try decode([0x80, 0x12, 0x34]).body.tag, 0)
        XCTAssertEqual(try decode([0x81, 0x07]).body.tag, 1)
    }

    func testTheFlagIsTheTagOfTheArmTheValueHoldsWhateverTheHeaderSays() throws {
        // The value holds the close arm; its header names the open arm and has a bit outside the flag.
        var frame = try decode([0x81, 0x07])
        frame.hdr = 0xFE
        XCTAssertEqual(try encoded(frame), [0xFF, 0x07])
        // And the other way: the open arm, and a header that names the close arm.
        frame = try decode([0x80, 0x12, 0x34])
        frame.hdr = 0x01
        XCTAssertEqual(try encoded(frame), [0x00, 0x12, 0x34])
    }

    func testAFrameRoundTripsThroughItsOwnEncode() throws {
        for wire: [UInt8] in [[0x80, 0x12, 0x34], [0x81, 0x07], [0x01, 0x09], [0x00, 0xAB, 0xCD]] {
            XCTAssertEqual(try encoded(try decode(wire)), wire)
        }
    }
}
