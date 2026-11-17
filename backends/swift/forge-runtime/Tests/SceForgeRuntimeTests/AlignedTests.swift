// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// Two lists of one count are held as one value where they are made (docs/adr/0065 §6 R5): the one
// door answers nil for two different counts, and a decode's door throws `inconsistentSource`.

private enum UpToFour: SceCountBound {
    static let least = 0
    static let most: Int? = 4
}

@Test func twoListsOfOneCountAreOneValue() throws {
    let links = try #require(BoundedList<UInt16, UpToFour>([1, 2]))
    let weights = try #require(BoundedList<UInt8, UpToFour>([7, 8]))
    let both = try #require(Aligned(links, weights))
    #expect(both.count == 2)
    #expect(both.first == links && both.second == weights)
    let fewer = try #require(BoundedList<UInt8, UpToFour>([7]))
    #expect(Aligned(links, fewer) == nil, "two counts are no pair of one count")
}

@Test func aDecodeRefusesTwoCountsAsAnInconsistentSource() throws {
    #expect(throws: CodecError.inconsistentSource) {
        try Aligned(decoding: [UInt16(1), 2], [UInt8(7)])
    }
    #expect(try Aligned(decoding: [UInt16](), [UInt8]()).count == 0)
}

@Test func theDefaultIsTheFewestOfEach() {
    let none = Aligned<BoundedList<UInt16, UpToFour>, BoundedList<UInt8, UpToFour>>(
        defaultOf: .fewest(of: 0), .fewest(of: 0))
    #expect(none.count == 0)
}
