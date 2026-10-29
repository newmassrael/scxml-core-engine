// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The algorithms `sce-codegen generate -l swift` writes that take a byte string
// (`scripts/regen_static_datamodel_swift.sh`): the parameter is a `[UInt8]`, its
// length is `data.count` and a byte is read by its index, and a byte string passed
// on to another algorithm is the same bytes.
final class BytesAlgorithmTests: XCTestCase {
    private func bytes(_ text: String) -> [UInt8] {
        Array(text.utf8)
    }

    // The vectors of the reference (SMHasher; npm murmurhash 2.0.1 `v3`), which
    // stdlib/hash/murmur3_32.scxml states. A hash written at the wrong width or with
    // the blocks in the wrong order answers a different number for every one of them.
    func testMurmur3AnswersTheVectorsOfTheReference() {
        XCTAssertEqual(Murmur332.murmur332([], 0), 0)
        XCTAssertEqual(Murmur332.murmur332([], 1), 0x514E_28B7)
        XCTAssertEqual(Murmur332.murmur332(bytes("hello"), 0), 0x248B_FA47)
        XCTAssertEqual(
            Murmur332.murmur332(bytes("The quick brown fox jumps over the lazy dog"), 0),
            0x2E4F_F723)
    }

    // A length that is not a multiple of four ends in a block of one to three bytes,
    // mixed without the rotation of the state: the lengths around four are each
    // a different tail.
    func testMurmur3TakesEveryLengthOfTail() {
        let hashes = (0...8).map { Murmur332.murmur332(bytes(String(repeating: "a", count: $0)), 7) }
        XCTAssertEqual(Set(hashes).count, hashes.count)
    }

    // The count of code points is the count of Unicode scalar values of the same text.
    func testTheLengthIsTheCountOfCodePoints() throws {
        for text in ["", "abc", "h\u{E9}llo", "\u{AC00}\u{D55C}", "a\u{1F600}b", "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F466}"] {
            XCTAssertEqual(
                try Utf8Length.utf8Length(bytes(text)), UInt32(text.unicodeScalars.count), text)
        }
        // The family of three people is 18 bytes and 5 code points (utf8_length.scxml).
        let family = bytes("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F466}")
        XCTAssertEqual(family.count, 18)
        XCTAssertEqual(try Utf8Length.utf8Length(family), 5)
    }

    // What RFC 3629 section 4 does not accept has no count: an overlong form, a
    // surrogate, a value past U+10FFFF, a lead the grammar has no use for, a tail
    // that is not a tail and a sequence cut short.
    func testWhatTheGrammarRefusesHasNoCount() {
        let refused: [[UInt8]] = [
            [0xC0, 0x80],
            [0xC1, 0xBF],
            [0xE0, 0x80, 0x80],
            [0xED, 0xA0, 0x80],
            [0xF0, 0x80, 0x80, 0x80],
            [0xF4, 0x90, 0x80, 0x80],
            [0xF5, 0x80, 0x80, 0x80],
            [0x80],
            [0xC2, 0x41],
            [0xE2, 0x82],
        ]
        for data in refused {
            XCTAssertThrowsError(try Utf8Length.utf8Length(data), "\(data)") { error in
                XCTAssertEqual(error as? AlgorithmError, .precondition)
            }
        }
    }

    // The limit is in code points, whatever the bytes: three code points of two bytes
    // each are within three and not within two. The length the callee measured is
    // handed back as it is, and a refusal of the callee is passed on.
    func testACapIsHeldInCodePointsAndARefusalIsPassedOn() throws {
        let text = bytes("\u{E9}\u{E9}\u{E9}")
        XCTAssertEqual(text.count, 6)
        XCTAssertTrue(try Utf8Within.utf8Within(text, 3))
        XCTAssertFalse(try Utf8Within.utf8Within(text, 2))
        XCTAssertTrue(try Utf8Within.utf8Within([], 0))
        XCTAssertThrowsError(try Utf8Within.utf8Within([0xC0, 0x80], 10)) { error in
            XCTAssertEqual(error as? AlgorithmError, .precondition)
        }
    }
}
