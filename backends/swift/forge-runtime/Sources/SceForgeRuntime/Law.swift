// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The lists of a codec printed from its encode plan whose entries say their inputs by a law
// (docs/adr/0065 §6 R8).
//
// An entry codec can say a flag input itself (R4a) while the list that holds it hands every entry
// a value from elsewhere: a flag of the parent bound to every entry, or, in a chain, whether the
// entry before is of a kind. The two agree or the value has no frame. Both are a language over the
// list that a small automaton decides: a state, the value each entry must say in it, and the state
// after the entry. A flag bound to every entry is the automaton whose state is the flag and never
// changes (`SceUniform`); the entry before is the one whose state starts clear and becomes whether
// the entry is of the kind (`SceAfterKey`). A list with both runs them together (`SceLaws`).
//
// `Lawful` holds a list with the state its automaton starts in, which the value gives where the
// start is not fixed: the flag the parent writes is read from it, so an empty list still holds one.
// It is checked once, where it is made, as `BoundedList` is, and it holds its list as a constant,
// so its entries cannot leave the law once it is made.
//
// What an entry says, and what kind it is, are the entry codec's to answer: a printed codec
// declares a `SceSaid` and a `SceKind` for each list it holds to a law, through the entry codec's
// reader of the input and the chain's identifier of an entry, generic over the entry's source.
//
// Siblings: `Lawful` (Rust, `backends/rust/forge-runtime`, `law.rs`) and `SCE::Forge::Lawful`
// (C++, `sce/forge/law.h`), so the languages' codecs hold the same refinement.

/// What a value gives a law whose start is fixed: nothing.
public struct SceFixedStart: Equatable, Hashable, Sendable {
    public init() {}
}

/// What an entry of a list says of one flag input: set or clear, or `nil` where it says nothing of
/// it (docs/adr/0065 §6 R4e), which follows any state (§6 R8b).
public protocol SceSaid {
    /// The entries: a source of the entry codec.
    associatedtype Entry
    /// What `entry` says of the input.
    static func says(_ entry: Entry) -> Bool?
}

/// Whether an entry of a chain is of one kind: its identifier is the kind's.
public protocol SceKind {
    /// The entries: a source of the entry codec.
    associatedtype Entry
    /// Whether `entry` is of the kind.
    static func matches(_ entry: Entry) -> Bool
}

/// The automaton the entries of a list follow (docs/adr/0065 §6 R8).
public protocol SceLaw {
    /// The entries: a source of the entry codec.
    associatedtype Entry
    /// What a value gives the law to start in.
    associatedtype Start: Equatable
    /// What the automaton holds between two entries.
    associatedtype State
    /// The state before the first entry.
    static func start(_ start: Start) -> State
    /// The state after `entry`, or `nil` where `entry` says a value other than the one `state`
    /// asks of it.
    static func step(_ state: State, _ entry: Entry) -> State?
    /// The start a list that begins with `first` (`nil` for an empty list) follows from: what the
    /// first entry says, else clear.
    static func start(of first: Entry?) -> Start
}

extension SceLaw {
    /// Whether `entries`, in order, follow the law from `start`.
    public static func follows<Entries: Sequence>(_ entries: Entries, from start: Start) -> Bool
    where Entries.Element == Entry {
        var state = Self.start(start)
        for entry in entries {
            guard let next = step(state, entry) else { return false }
            state = next
        }
        return true
    }
}

/// The law of a flag bound to every entry: each entry says the input `Said` reads as the start
/// says it, and the start is the flag.
public enum SceUniform<Said: SceSaid>: SceLaw {
    public typealias Entry = Said.Entry
    public typealias Start = Bool
    public typealias State = Bool

    public static func start(_ start: Bool) -> Bool {
        start
    }

    public static func step(_ state: Bool, _ entry: Said.Entry) -> Bool? {
        if let said = Said.says(entry), said != state {
            return nil
        }
        return state
    }

    public static func start(of first: Said.Entry?) -> Bool {
        first.flatMap { Said.says($0) } ?? false
    }
}

/// The law of an input a chain supplies from the entry before (`<sce:entry-flag-bind
/// previous-entry-id>`): each entry says the input `Said` reads set exactly where the entry before
/// is of the kind `Kind` matches, and the first says it clear. The start is fixed.
public enum SceAfterKey<Said: SceSaid, Kind: SceKind>: SceLaw where Said.Entry == Kind.Entry {
    public typealias Entry = Said.Entry
    public typealias Start = SceFixedStart
    public typealias State = Bool

    public static func start(_ start: SceFixedStart) -> Bool {
        false
    }

    public static func step(_ state: Bool, _ entry: Said.Entry) -> Bool? {
        if let said = Said.says(entry), said != state {
            return nil
        }
        return Kind.matches(entry)
    }

    public static func start(of first: Said.Entry?) -> SceFixedStart {
        SceFixedStart()
    }
}

/// The starts of two laws run together, in order.
public struct SceStarts<First: Equatable, Second: Equatable>: Equatable {
    /// The first law's start.
    public let first: First
    /// The second law's start.
    public let second: Second

    public init(_ first: First, _ second: Second) {
        self.first = first
        self.second = second
    }
}

extension SceStarts: Hashable where First: Hashable, Second: Hashable {}

extension SceStarts: Sendable where First: Sendable, Second: Sendable {}

/// Two laws over one list, run together: each entry follows both, and the start is each one's, in
/// order. A list of more laws nests them, the first and the rest.
public enum SceLaws<First: SceLaw, Second: SceLaw>: SceLaw where First.Entry == Second.Entry {
    public typealias Entry = First.Entry
    public typealias Start = SceStarts<First.Start, Second.Start>
    public typealias State = (First.State, Second.State)

    public static func start(_ start: Start) -> State {
        (First.start(start.first), Second.start(start.second))
    }

    public static func step(_ state: State, _ entry: First.Entry) -> State? {
        guard let first = First.step(state.0, entry), let second = Second.step(state.1, entry) else {
            return nil
        }
        return (first, second)
    }

    public static func start(of first: First.Entry?) -> Start {
        SceStarts(First.start(of: first), Second.start(of: first))
    }
}

/// A list whose entries follow the law `Law` from the start the value gives (docs/adr/0065 §6 R8),
/// read in order as the list it holds.
public struct Lawful<List: Collection, Law: SceLaw> where List.Element == Law.Entry {
    /// The list.
    public let list: List
    /// What the value gives the law to start in.
    public let start: Law.Start

    /// `list` from `start`, or `nil` where an entry says a value other than the one its place asks
    /// of it.
    public init?(start: Law.Start, list: List) {
        guard Law.follows(list, from: start) else { return nil }
        self.list = list
        self.start = start
    }

    /// The list a decode read from the start the frame gave, which a frame whose entries say other
    /// than their places ask of them refuses with `presentIfMismatch`.
    public init(decoding list: List, start: Law.Start) throws(CodecError) {
        guard Law.follows(list, from: start) else { throw CodecError.presentIfMismatch }
        self.list = list
        self.start = start
    }

    /// `list` from the start its first entry says: what a value that has read no frame holds, its
    /// list the fewest entries its bound admits, each a value of a frame of zeros. A list whose
    /// entries do not follow the law from there is no value of the type, and the program stops.
    public init(defaultOf list: List) {
        let start = Law.start(of: list.first)
        precondition(Law.follows(list, from: start), "the default entries follow the list's law")
        self.list = list
        self.start = start
    }
}

extension Lawful: Collection {
    public var startIndex: List.Index { list.startIndex }
    public var endIndex: List.Index { list.endIndex }

    public subscript(position: List.Index) -> List.Element {
        list[position]
    }

    public func index(after position: List.Index) -> List.Index {
        list.index(after: position)
    }
}

extension Lawful: Equatable where List: Equatable {}

extension Lawful: Hashable where List: Hashable, Law.Start: Hashable {}

extension Lawful: Sendable where List: Sendable, Law.Start: Sendable {}
