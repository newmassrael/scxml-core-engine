// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Chains refined by the kinds of entry the gates on their entries name (docs/adr/0065 §6 R6).
//
// A field gated on `chain.has(k)` is on the wire exactly when an entry of the kind `k` is in the
// chain. The gates on one chain's entries, and the flags joined with them, decide its fields
// together, so a printed codec holds one arm of a gate group for each assignment of them, and the
// arm says of each kind whether the chain has an entry of it. The arm holds the chain in a type that
// says the same, so the fields the gates decide cannot disagree with the chain:
//
// - `SceChain` is the chain whole, within its count bound, with an entry of each kind of one set
//   and of none of another (`SceKinds`): the arm's, for entries with content, which the value holds
//   where they stand (R6b).
// - `ScePlaced` is the other entries and the place of the first entry of a kind among them, where
//   the one gate on a bare entry holds (R6a): the entry's identifier is all it is, so the codec
//   writes it from its kind and the value holds its place. The place is part of the value, since
//   the protocols these chains carry do not order their entries.
//
// Each is checked once, where it is made, as `BoundedList` is, and holds its entries as constants,
// so it stays what it was checked to be. A decode makes the arm's chain of the chain it read
// through `init(decoding:)`, which a chain that is not what the arm says refuses with
// `presentIfMismatch`.
//
// What kind an entry is, is the entry codec's to answer: a printed codec declares a `SceKind` for
// each kind a gate names, through the chain's identifier of an entry, generic over the entry's
// source, as it declares one for a law (`Law.swift`).
//
// Siblings: `Chain`, `Placed` and `Kinds` (Rust, `backends/rust/forge-runtime`, `membership.rs`),
// `SCE::Forge::Chain` and `Placed` (C++), and the classes each Kotlin codec declares for its arms.

/// A set of kinds of entry: what a `SceChain` holds an entry of each of, or of none. A set of no
/// kind is `SceNoKinds`; a set of more nests them, the first and the rest (`SceKindsOf`), as two
/// laws run together nest (`SceLaws`).
public protocol SceKinds {
    /// The entries: a source of the entry codec.
    associatedtype Entry
    /// Whether `entries` hold an entry of each kind of the set.
    static func eachIn<Entries: Collection>(_ entries: Entries) -> Bool
    where Entries.Element == Entry
    /// Whether `entries` hold an entry of none of the set's kinds.
    static func noneIn<Entries: Collection>(_ entries: Entries) -> Bool
    where Entries.Element == Entry
}

/// The set of no kind: every chain holds an entry of each of its kinds, and of none of them.
public enum SceNoKinds<Entry>: SceKinds {
    public static func eachIn<Entries: Collection>(_ entries: Entries) -> Bool
    where Entries.Element == Entry {
        true
    }

    public static func noneIn<Entries: Collection>(_ entries: Entries) -> Bool
    where Entries.Element == Entry {
        true
    }
}

/// The set of the kind `First` and the kinds of `Rest`.
public enum SceKindsOf<First: SceKind, Rest: SceKinds>: SceKinds where First.Entry == Rest.Entry {
    public typealias Entry = First.Entry

    public static func eachIn<Entries: Collection>(_ entries: Entries) -> Bool
    where Entries.Element == Entry {
        entries.contains(where: { First.matches($0) }) && Rest.eachIn(entries)
    }

    public static func noneIn<Entries: Collection>(_ entries: Entries) -> Bool
    where Entries.Element == Entry {
        !entries.contains(where: { First.matches($0) }) && Rest.noneIn(entries)
    }
}

/// A chain within the count bound `Bound`, held whole, that holds an entry of each kind of `Has`
/// and an entry of none of `Lacks` (docs/adr/0065 §6 R6b): the chain one arm of the gates on its
/// entries holds. It reads in order as its entries.
public struct SceChain<Element, Bound: SceCountBound, Has: SceKinds, Lacks: SceKinds>
where Has.Entry == Element, Lacks.Entry == Element {
    /// The chain, within its bound.
    public let list: BoundedList<Element, Bound>

    /// `list` as the chain of the arm, or `nil` where it lacks a kind of `Has` or holds one of
    /// `Lacks`.
    public init?(_ list: BoundedList<Element, Bound>) {
        guard Has.eachIn(list), Lacks.noneIn(list) else { return nil }
        self.list = list
    }

    /// The chain a decode read, as the arm its kinds select holds it, which a chain that is not
    /// what the arm says refuses with `presentIfMismatch`.
    public init(decoding list: BoundedList<Element, Bound>) throws(CodecError) {
        guard Has.eachIn(list), Lacks.noneIn(list) else { throw CodecError.presentIfMismatch }
        self.list = list
    }

    /// `list` as a value that has read no frame holds it, the fewest entries its bound admits,
    /// each a value of a frame of zeros. A chain the arm does not admit is no value of the type, and
    /// the program stops.
    public init(defaultOf list: BoundedList<Element, Bound>) {
        precondition(Has.eachIn(list) && Lacks.noneIn(list), "the default chain is of the arm's kinds")
        self.list = list
    }

    /// The entries, in wire order.
    public var entries: [Element] { list.entries }
}

extension SceChain: RandomAccessCollection {
    public var startIndex: Int { list.startIndex }
    public var endIndex: Int { list.endIndex }

    public subscript(position: Int) -> Element {
        list[position]
    }
}

extension SceChain: Equatable where Element: Equatable {}

extension SceChain: Hashable where Element: Hashable {}

extension SceChain: Sendable where Element: Sendable {}

/// A chain within the count bound `Bound` that holds an entry of the kind `Kind`, whose identifier
/// is all it is (docs/adr/0065 §6 R6a): the other entries, and the place of the first entry of the
/// kind among them, no entry before the place being of the kind. The codec writes the entry of the
/// kind at its place.
public struct ScePlaced<Element, Bound: SceCountBound, Kind: SceKind> where Kind.Entry == Element {
    /// The entries other than the one of the kind, in wire order.
    public let rest: [Element]
    /// Where the entry of the kind stands among the others: how many come before it.
    public let place: Int

    /// The entries `rest` with the entry of the kind at `place` among them, or `nil` where the
    /// chain they make is past the bound, `place` is past the end of `rest`, or an entry before
    /// `place` is of the kind (the entry of the kind is then that one, not the one at `place`).
    public init?(rest: [Element], place: Int) {
        guard Bound.holds(rest.count + 1), place >= 0, place <= rest.count,
            !rest.prefix(place).contains(where: { Kind.matches($0) })
        else { return nil }
        self.rest = rest
        self.place = place
    }

    /// The chain a decode read, which holds an entry of the kind, as the others and the place of
    /// the first of the kind: a chain that holds none refuses with `presentIfMismatch`.
    public init(decoding chain: BoundedList<Element, Bound>) throws(CodecError) {
        guard let place = chain.firstIndex(where: { Kind.matches($0) }) else {
            throw CodecError.presentIfMismatch
        }
        var rest = chain.entries
        rest.remove(at: place)
        self.rest = rest
        self.place = place
    }

    /// How many entries the chain holds, the one of the kind among them.
    public var count: Int { rest.count + 1 }
}

extension ScePlaced: Equatable where Element: Equatable {}

extension ScePlaced: Hashable where Element: Hashable {}

extension ScePlaced: Sendable where Element: Sendable {}
