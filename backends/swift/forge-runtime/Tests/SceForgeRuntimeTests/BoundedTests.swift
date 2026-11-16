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

// A list of entries is held to its count bound the same way (docs/adr/0065 §6 R2), and a chain
// closed by its entries' flag holds one entry or more (§6 R7).

private enum UpToThree: SceCountBound {
    static let least = 0
    static let most: Int? = 3
}

private enum OneOrMore: SceCountBound {
    static let least = 1
    static let most: Int? = nil
}

@Test func aListIsMadeOnlyWithinItsCountBound() throws {
    #expect(BoundedList<UInt16, UpToThree>([1, 2, 3, 4]) == nil, "more than the most")
    #expect(BoundedList<UInt16, OneOrMore>([]) == nil, "fewer than the least")
    let held = try #require(BoundedList<UInt16, UpToThree>([7, 8]))
    #expect(held.entries == [7, 8])
    #expect(held.count == 2)
    #expect(Array(held) == [7, 8], "it reads as its entries in order")
    #expect(held[1] == 8)
    #expect(BoundedList<UInt16, UpToThree>([]) != nil, "an empty list is a value")
    #expect(BoundedList<UInt16, OneOrMore>([UInt16](repeating: 1, count: 1000)) != nil, "no most")
}

@Test func aDecodeRefusesEntriesPastTheBoundAsTooManyElements() throws {
    #expect(throws: CodecError.tooManyElements) {
        try BoundedList<UInt16, UpToThree>(decoding: [1, 2, 3, 4])
    }
    #expect(try BoundedList<UInt16, UpToThree>(decoding: [1, 2, 3]).entries == [1, 2, 3])
}

@Test func twoListsOfTheSameEntriesAreEqual() throws {
    let a = try #require(BoundedList<UInt16, UpToThree>([1, 2]))
    let b = try #require(BoundedList<UInt16, UpToThree>([1, 2]))
    let c = try #require(BoundedList<UInt16, UpToThree>([2, 1]))
    #expect(a == b)
    #expect(a != c, "the order of the entries is part of the value")
}

@Test func theFewestEntriesAreTheLeastTheBoundHolds() {
    #expect(BoundedList<UInt16, UpToThree>.fewest(of: 9).entries == [])
    #expect(BoundedList<UInt16, OneOrMore>.fewest(of: 9).entries == [9])
}
