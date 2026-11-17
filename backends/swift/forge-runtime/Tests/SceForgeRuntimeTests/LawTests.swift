// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A list whose entries say their inputs by a law is held to it where it is made (docs/adr/0065 §6
// R8): the one door answers nil for entries that say other than their places ask, a decode's door
// throws `presentIfMismatch`, and an entry that says nothing follows any state (R8b).

/// An entry of a test list: what it says of its one input, and its kind.
private struct Entry: Equatable {
    let says: Bool?
    let kind: UInt8
}

private enum EntrySays: SceSaid {
    static func says(_ entry: Entry) -> Bool? { entry.says }
}

private enum EntryIsMarker: SceKind {
    static func matches(_ entry: Entry) -> Bool { entry.kind == 4 }
}

private enum UpToFour: SceCountBound {
    static let least = 0
    static let most: Int? = 4
}

private enum OneOrMore: SceCountBound {
    static let least = 1
    static let most: Int? = 4
}

private typealias Uniform = SceUniform<EntrySays>
private typealias AfterMarker = SceAfterKey<EntrySays, EntryIsMarker>
private typealias Both = SceLaws<Uniform, AfterMarker>

@Test func aUniformLawHoldsEveryEntryToTheStart() throws {
    let set = [Entry(says: true, kind: 1), Entry(says: nil, kind: 2), Entry(says: true, kind: 3)]
    let held = try #require(Lawful<[Entry], Uniform>(start: true, list: set))
    #expect(held.start == true)
    #expect(Array(held) == set, "it reads as its list in order")
    #expect(held.count == 3)
    #expect(Lawful<[Entry], Uniform>(start: false, list: set) == nil, "an entry says set")
    #expect(
        Lawful<[Entry], Uniform>(start: false, list: []) != nil,
        "an empty list follows any start"
    )
}

@Test func anAfterKeyLawHoldsEachEntryToTheOneBefore() throws {
    let marked = [Entry(says: false, kind: 4), Entry(says: true, kind: 1), Entry(says: false, kind: 2)]
    #expect(Lawful<[Entry], AfterMarker>(start: SceFixedStart(), list: marked) != nil)
    let first = [Entry(says: true, kind: 1)]
    #expect(
        Lawful<[Entry], AfterMarker>(start: SceFixedStart(), list: first) == nil,
        "the first entry has no entry before it"
    )
    let missed = [Entry(says: false, kind: 4), Entry(says: false, kind: 1)]
    #expect(
        Lawful<[Entry], AfterMarker>(start: SceFixedStart(), list: missed) == nil,
        "the entry after the marker says it is after one"
    )
}

@Test func twoLawsRunTogether() throws {
    // Each entry says one input for both laws here, so only a list both accept is a value.
    let list = [Entry(says: false, kind: 1)]
    #expect(Lawful<[Entry], Both>(start: SceStarts(false, SceFixedStart()), list: list) != nil)
    #expect(
        Lawful<[Entry], Both>(start: SceStarts(true, SceFixedStart()), list: list) == nil,
        "the uniform law refuses it"
    )
}

@Test func aDecodeRefusesEntriesThatBreakTheLawAsAPresentIfMismatch() throws {
    #expect(throws: CodecError.presentIfMismatch) {
        try Lawful<[Entry], Uniform>(decoding: [Entry(says: true, kind: 1)], start: false)
    }
    #expect(try Lawful<[Entry], Uniform>(decoding: [], start: true).start == true)
}

@Test func theDefaultStartsWhereItsFirstEntrySays() throws {
    let none = Lawful<BoundedList<Entry, UpToFour>, Uniform>(
        defaultOf: .fewest(of: Entry(says: true, kind: 0)))
    #expect(none.isEmpty && none.start == false, "an empty list starts clear")
    let one = Lawful<BoundedList<Entry, OneOrMore>, Uniform>(
        defaultOf: .fewest(of: Entry(says: true, kind: 0)))
    #expect(one.count == 1 && one.start == true, "the one default entry says the start")
}
