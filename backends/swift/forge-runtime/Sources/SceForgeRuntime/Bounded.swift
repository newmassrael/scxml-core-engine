// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The byte strings and texts of a codec printed from its encode plan, each held to the bound its
// document declares (docs/adr/0065 §6 R1, docs/adr/0077 §3): a value of `BoundedBytes` or
// `BoundedText` is within its bound for as long as it lives, so the encode over it refuses nothing
// the bound decides and derives the length it writes from it. The bound is a type of the codec's
// own (`SceByteBound`), so a value made for one field is no value of another.
//
// The one door is the failable initializer: it answers `nil` for a value outside the bound, as
// Rust's `Bounded::new` answers `None` and C++'s `make` an empty optional. A decode makes one by
// `init(decoding:)`, which throws the `CodecError` a frame that says too much is refused with.
//
// A list or a chain of entries is held the same way (docs/adr/0065 §6 R2): `BoundedList` holds as
// many entries as its count bound (`SceCountBound`) admits, so the encode over it refuses no count
// and derives the count it writes from it. A chain closed by its entries' flag holds one entry or
// more (§6 R7), which its bound's `least` states.
//
// Siblings: `Bytes`, `Text` and `Bounded` (Rust, `backends/rust/forge-runtime`), `Bounded` (C++)
// and `BoundedBytes`, `BoundedText` (Python, `sce_forge_runtime.bounded`).

/// How many bytes a byte string or a text of one field of a codec holds: a type the generated
/// codec declares for the field, so the bound is part of the field's type.
public protocol SceByteBound {
    /// The fewest bytes.
    static var least: Int { get }
    /// The most bytes, or `nil` where the protocol states none (docs/adr/0077 §7).
    static var most: Int? { get }
}

extension SceByteBound {
    /// Whether `count` bytes are within the bound.
    public static func holds(_ count: Int) -> Bool {
        count >= least && (most.map { count <= $0 } ?? true)
    }
}

/// A byte string within the bound `Bound`.
public struct BoundedBytes<Bound: SceByteBound>: Equatable, Hashable, Sendable {
    /// The bytes.
    public let bytes: [UInt8]

    /// The bytes `bytes`, or `nil` where they are outside the bound.
    public init?(_ bytes: [UInt8]) {
        guard Bound.holds(bytes.count) else { return nil }
        self.bytes = bytes
    }

    /// The bytes a decode read, which a frame that says more or fewer than the bound refuses with
    /// `lengthOutOfRange`.
    public init(decoding bytes: [UInt8]) throws(CodecError) {
        guard Bound.holds(bytes.count) else { throw CodecError.lengthOutOfRange }
        self.bytes = bytes
    }

    private init(within bytes: [UInt8]) {
        self.bytes = bytes
    }

    /// The fewest bytes the bound holds, each zero: what a value that has read no frame holds.
    public static var fewest: Self {
        Self(within: [UInt8](repeating: 0, count: Bound.least))
    }

    /// How many bytes the frame holds of it.
    public var byteCount: Int { bytes.count }

    /// Appends the bytes to `frame`.
    public func append(to frame: inout [UInt8]) {
        frame += bytes
    }
}

/// A text within the bound `Bound`, counted in the UTF-8 bytes the frame holds of it.
public struct BoundedText<Bound: SceByteBound>: Equatable, Hashable, Sendable {
    /// The text.
    public let text: String

    /// The text `text`, or `nil` where its UTF-8 bytes are outside the bound.
    public init?(_ text: String) {
        guard Bound.holds(text.utf8.count) else { return nil }
        self.text = text
    }

    /// The text a decode read, which a frame that says more or fewer bytes than the bound refuses
    /// with `lengthOutOfRange`.
    public init(decoding text: String) throws(CodecError) {
        guard Bound.holds(text.utf8.count) else { throw CodecError.lengthOutOfRange }
        self.text = text
    }

    private init(within text: String) {
        self.text = text
    }

    /// The fewest bytes the bound holds, each NUL: what a value that has read no frame holds.
    public static var fewest: Self {
        Self(within: String(repeating: "\u{0}", count: Bound.least))
    }

    /// How many bytes the frame holds of it: its UTF-8 bytes, not its characters.
    public var byteCount: Int { text.utf8.count }

    /// Appends the UTF-8 bytes of the text to `frame`.
    public func append(to frame: inout [UInt8]) {
        frame += Array(text.utf8)
    }
}

/// How many entries a list or a chain of one field of a codec holds: a type the generated codec
/// declares for the field, so the bound is part of the field's type. A count bound is no byte bound,
/// so a bound declared for one kind of element is no bound of the other.
public protocol SceCountBound {
    /// The fewest entries: one for a chain closed by its entries' flag, none for another list.
    static var least: Int { get }
    /// The most entries, or `nil` where the protocol states none (docs/adr/0077 §7).
    static var most: Int? { get }
}

extension SceCountBound {
    /// Whether `count` entries are within the bound.
    public static func holds(_ count: Int) -> Bool {
        count >= least && (most.map { count <= $0 } ?? true)
    }
}

/// A list of entries within the bound `Bound`, each a value `Element` its entry codec writes: the
/// value a printed codec presents a list or a chain as, read in order as a collection.
public struct BoundedList<Element, Bound: SceCountBound> {
    /// The entries, in wire order.
    public let entries: [Element]

    /// The entries `entries`, or `nil` where they are more or fewer than the bound holds.
    public init?(_ entries: [Element]) {
        guard Bound.holds(entries.count) else { return nil }
        self.entries = entries
    }

    /// The entries a decode read, which a frame that says more than the bound holds refuses with
    /// `tooManyElements`. A decode reads no fewer than the least: a chain reads one entry before it
    /// asks whether another follows.
    public init(decoding entries: [Element]) throws(CodecError) {
        guard Bound.holds(entries.count) else { throw CodecError.tooManyElements }
        self.entries = entries
    }

    private init(within entries: [Element]) {
        self.entries = entries
    }

    /// The fewest entries the bound holds, each `entry`: what a value that has read no frame holds.
    public static func fewest(of entry: @autoclosure () -> Element) -> Self {
        Self(within: (0..<Bound.least).map { _ in entry() })
    }
}

extension BoundedList: RandomAccessCollection {
    public var startIndex: Int { entries.startIndex }
    public var endIndex: Int { entries.endIndex }

    public subscript(position: Int) -> Element {
        entries[position]
    }
}

extension BoundedList: Equatable where Element: Equatable {}

extension BoundedList: Hashable where Element: Hashable {}

extension BoundedList: Sendable where Element: Sendable {}
