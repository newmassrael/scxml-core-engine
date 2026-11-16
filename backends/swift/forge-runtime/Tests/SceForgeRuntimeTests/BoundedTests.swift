// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A byte string and a text of a printed codec are held to their bound where they are made
// (docs/adr/0065 §6 R1): the one door answers nil outside it, a decode's door throws
// `lengthOutOfRange`, and a text is counted in its UTF-8 bytes.

private enum TwoToFour: SceByteBound {
    static let least = 2
    static let most: Int? = 4
}

private enum AtLeastOne: SceByteBound {
    static let least = 1
    static let most: Int? = nil
}

@Test func aByteStringIsMadeOnlyWithinItsBound() throws {
    #expect(BoundedBytes<TwoToFour>([1]) == nil, "fewer than the least")
    #expect(BoundedBytes<TwoToFour>([1, 2, 3, 4, 5]) == nil, "more than the most")
    let held = try #require(BoundedBytes<TwoToFour>([1, 2, 3]))
    #expect(held.bytes == [1, 2, 3])
    #expect(held.byteCount == 3)
    var frame: [UInt8] = [9]
    held.append(to: &frame)
    #expect(frame == [9, 1, 2, 3])
    #expect(BoundedBytes<AtLeastOne>([UInt8](repeating: 7, count: 1000)) != nil, "no most")
}

@Test func aDecodeRefusesBytesOutsideTheBoundAsALengthOutOfRange() throws {
    #expect(throws: CodecError.lengthOutOfRange) { try BoundedBytes<TwoToFour>(decoding: [1]) }
    #expect(throws: CodecError.lengthOutOfRange) { try BoundedText<TwoToFour>(decoding: "abcde") }
    #expect(try BoundedBytes<TwoToFour>(decoding: [1, 2]).bytes == [1, 2])
}

@Test func aTextIsCountedInItsUtf8Bytes() throws {
    // Two characters of three bytes each are six bytes, past the most of four.
    #expect(BoundedText<TwoToFour>("\u{D55C}\u{AE00}") == nil)
    let held = try #require(BoundedText<TwoToFour>("\u{E9}a"))
    #expect(held.byteCount == 3)
    var frame: [UInt8] = []
    held.append(to: &frame)
    #expect(frame == [0xC3, 0xA9, 0x61])
}

@Test func theFewestIsTheLeastOfZeros() {
    #expect(BoundedBytes<TwoToFour>.fewest.bytes == [0, 0])
    #expect(BoundedText<TwoToFour>.fewest.byteCount == 2)
}
