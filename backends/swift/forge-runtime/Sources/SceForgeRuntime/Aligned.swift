// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Two lists of one count (docs/adr/0065 §6 R5): a frame writes one count, and both lists hold that
// many entries. A printed codec holds them as one `Aligned` in the arm of the gate group that holds
// both, so the encode that writes the one count of both refuses nothing (docs/adr/0081 §2): the
// check runs once, where the value is made, and both lists are constants of it after.
//
// What a list answers its count by is `Collection.count`, which every list a printed codec holds
// (`BoundedList`, `Lawful`) has, so no protocol of the runtime's own stands for it.
//
// Siblings: `Aligned` (Rust, `backends/rust/forge-runtime`), `SCE::Forge::Aligned` (C++) and
// `Aligned` with `Counted` (Kotlin, `com.sce.forge.runtime`).

/// Two lists of one count.
public struct Aligned<First: Collection, Second: Collection> {
    /// The list the count is written for, which every arm of the group holds.
    public let first: First
    /// The list that shares the first's count.
    public let second: Second

    /// `first` and `second` as one value of one count, or `nil` where they hold different counts,
    /// which no frame carries.
    public init?(_ first: First, _ second: Second) {
        guard first.count == second.count else { return nil }
        self.first = first
        self.second = second
    }

    /// The two lists a decode read by the one count the frame states, which a pair of different
    /// counts refuses with `inconsistentSource`.
    public init(decoding first: First, _ second: Second) throws(CodecError) {
        guard first.count == second.count else { throw CodecError.inconsistentSource }
        self.first = first
        self.second = second
    }

    /// `first` and `second` as a value that has read no frame holds them, each the fewest entries
    /// its bound admits. Two lists of different counts are no value of the type, and the program
    /// stops.
    public init(defaultOf first: First, _ second: Second) {
        precondition(first.count == second.count, "the default lists are of one count")
        self.first = first
        self.second = second
    }

    /// How many entries each list holds: the one count the frame writes.
    public var count: Int { first.count }
}

extension Aligned: Equatable where First: Equatable, Second: Equatable {}

extension Aligned: Hashable where First: Hashable, Second: Hashable {}

extension Aligned: Sendable where First: Sendable, Second: Sendable {}
