// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A chain an arm of the gates on its entries holds is held to the kinds the arm says where it is
// made (docs/adr/0065 §6 R6): the one door answers nil for a chain that lacks a kind the arm holds
// or holds one it lacks, a decode's door throws `presentIfMismatch`, and a bare entry's chain holds
// the others and the place of the first of its kind.

/// An entry of a test chain: its kind.
private struct Entry: Equatable {
    let kind: UInt8
}

private enum OfSeven: SceKind {
    static func matches(_ entry: Entry) -> Bool { entry.kind == 7 }
}

private enum OfNine: SceKind {
    static func matches(_ entry: Entry) -> Bool { entry.kind == 9 }
}

private enum UpToThree: SceCountBound {
    static let least = 1
    static let most: Int? = 3
}

private typealias NoKind = SceNoKinds<Entry>
private typealias Seven = SceKindsOf<OfSeven, NoKind>
private typealias Nine = SceKindsOf<OfNine, NoKind>
private typealias SevenAndNine = SceKindsOf<OfSeven, Nine>

/// The chain of entries of `kinds`, within the bound.
private func chain(_ kinds: UInt8...) throws -> BoundedList<Entry, UpToThree> {
    try #require(BoundedList<Entry, UpToThree>(kinds.map { Entry(kind: $0) }))
}

@Test func aChainHoldsEachKindOfItsArmAndNoneItLacks() throws {
    let seven = try chain(1, 7)
    let held = try #require(SceChain<Entry, UpToThree, Seven, Nine>(seven))
    #expect(Array(held) == seven.entries, "it reads as its entries in order")
    #expect(held.entries == seven.entries)
    #expect(
        SceChain<Entry, UpToThree, SevenAndNine, NoKind>(seven) == nil,
        "the chain lacks a kind the arm holds"
    )
    #expect(
        SceChain<Entry, UpToThree, NoKind, Seven>(seven) == nil,
        "the chain holds a kind the arm lacks"
    )
    let both = try chain(9, 7)
    #expect(SceChain<Entry, UpToThree, SevenAndNine, NoKind>(both) != nil)
}

@Test func aDecodeRefusesAChainThatIsNotWhatItsArmSaysAsAPresentIfMismatch() throws {
    let plain = try chain(1)
    #expect(throws: CodecError.presentIfMismatch) {
        try SceChain<Entry, UpToThree, Seven, NoKind>(decoding: plain)
    }
    let read = try SceChain<Entry, UpToThree, NoKind, Seven>(decoding: try chain(1, 2))
    #expect(read.count == 2)
}

@Test func aBareEntrysChainHoldsTheOthersAndThePlaceOfTheFirstOfItsKind() throws {
    let placed = try #require(
        ScePlaced<Entry, UpToThree, OfSeven>(rest: [Entry(kind: 1), Entry(kind: 7)], place: 1))
    #expect(placed.rest == [Entry(kind: 1), Entry(kind: 7)])
    #expect(placed.place == 1)
    #expect(placed.count == 3)
    #expect(
        ScePlaced<Entry, UpToThree, OfSeven>(rest: [Entry(kind: 7)], place: 1) == nil,
        "an entry before the place is of the kind, so the entry of the kind is that one"
    )
    #expect(
        ScePlaced<Entry, UpToThree, OfSeven>(rest: [Entry(kind: 1)], place: 2) == nil,
        "the place is past the end of the others"
    )
    let three = [Entry(kind: 1), Entry(kind: 2), Entry(kind: 3)]
    #expect(
        ScePlaced<Entry, UpToThree, OfSeven>(rest: three, place: 0) == nil,
        "the others and the entry of the kind are past the bound"
    )
    #expect(ScePlaced<Entry, UpToThree, OfSeven>(rest: [], place: 0) != nil, "the entry alone")
}

@Test func aDecodeTakesTheFirstEntryOfTheKindOutOfTheChainItRead() throws {
    let placed = try ScePlaced<Entry, UpToThree, OfSeven>(decoding: try chain(1, 7, 7))
    #expect(placed.place == 1)
    #expect(placed.rest == [Entry(kind: 1), Entry(kind: 7)])
    let none = try chain(1, 2)
    #expect(throws: CodecError.presentIfMismatch) {
        try ScePlaced<Entry, UpToThree, OfSeven>(decoding: none)
    }
}
