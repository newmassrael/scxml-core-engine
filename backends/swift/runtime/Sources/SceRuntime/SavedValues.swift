// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// How each `sce:type` is written into a saved state and read back: only if it is one. A value of another
// kind, or one its width cannot hold, is refused rather than converted. These are the functions a generated
// machine's save and restore call, one per type, named as the type is.
//
// A number is written as a JSON number, except a 64-bit integer, which is written as a text: a reader that
// holds JSON numbers as doubles would lose its low bits, and a saved state outlives the backend that wrote it.
//
// Cross-language siblings: `com.sce.runtime.SavedValues` (Kotlin) and `sce_rust_runtime::saved_state` (Rust).

public enum SavedValues {
    // MARK: Writing

    public static func of(_ value: UInt8) -> JSONValue { .number(String(value)) }
    public static func of(_ value: UInt16) -> JSONValue { .number(String(value)) }
    public static func of(_ value: UInt32) -> JSONValue { .number(String(value)) }
    public static func of(_ value: Int8) -> JSONValue { .number(String(value)) }
    public static func of(_ value: Int16) -> JSONValue { .number(String(value)) }
    public static func of(_ value: Int32) -> JSONValue { .number(String(value)) }

    /// A 64-bit integer, written as a text.
    public static func of(_ value: UInt64) -> JSONValue { .text(String(value)) }
    public static func of(_ value: Int64) -> JSONValue { .text(String(value)) }

    /// A 32-bit real is written as the 64-bit real it widens to, which is exact: the number a reader that holds
    /// JSON numbers as doubles finds is the number the machine holds, `0.30000001192092896` and not the shorter
    /// `0.3` a single prints for it, which is another number to that reader.
    public static func of(_ value: Float) -> JSONValue { of(Double(value)) }

    /// A real. JSON has no spelling for a value that is not finite, so one is written as the text every
    /// backend reads it back from.
    public static func of(_ value: Double) -> JSONValue {
        if value.isFinite {
            return .number("\(value)")
        }
        if value.isNaN {
            return .text("NaN")
        }
        return .text(value > 0 ? "Infinity" : "-Infinity")
    }

    public static func of(_ value: Bool) -> JSONValue { .bool(value) }
    public static func of(_ value: String) -> JSONValue { .text(value) }

    // MARK: Reading

    /// The refusal of `value`, saved for `what`, which is not `kind` (`a number`, `a text`, ...). A `null` is
    /// named apart: it is an absent value (docs/adr/0055), which only a place the restoring build declares
    /// optional reads, so a state saved after a widening is refused at its first `null` by a build from before
    /// it.
    static func notA(_ value: JSONValue, _ what: String, _ kind: String) -> StateRefusal {
        if case .null = value {
            return StateRefusal("'\(what)' is absent (null), and its type is not optional")
        }
        return StateRefusal("'\(what)' is not \(kind)")
    }

    private static func number(_ value: JSONValue, _ what: String) throws -> String {
        guard case .number(let text) = value else { throw notA(value, what, "a number") }
        return text
    }

    private static func whole<T: FixedWidthInteger>(_ value: JSONValue, _ what: String, as type: T.Type) throws -> T {
        let text = try number(value, what)
        guard let found = T(text) else {
            throw StateRefusal("'\(what)' (\(text)) is not a whole number its type can hold")
        }
        return found
    }

    private static func wideText(_ value: JSONValue, _ what: String) throws -> String {
        guard case .text(let text) = value else {
            throw notA(value, what, "a 64-bit whole number written as a text")
        }
        return text
    }

    private static func real(_ value: JSONValue, _ what: String) throws -> Double {
        if case .text(let text) = value {
            switch text {
            case "NaN": return Double.nan
            case "Infinity": return Double.infinity
            case "-Infinity": return -Double.infinity
            default: break
            }
        }
        let text = try number(value, what)
        guard let found = Double(text) else { throw StateRefusal("'\(what)' (\(text)) is not a number") }
        return found
    }

    public static func uint8(_ value: JSONValue, _ what: String) throws -> UInt8 {
        try whole(value, what, as: UInt8.self)
    }

    public static func uint16(_ value: JSONValue, _ what: String) throws -> UInt16 {
        try whole(value, what, as: UInt16.self)
    }

    public static func uint32(_ value: JSONValue, _ what: String) throws -> UInt32 {
        try whole(value, what, as: UInt32.self)
    }

    public static func int8(_ value: JSONValue, _ what: String) throws -> Int8 {
        try whole(value, what, as: Int8.self)
    }

    public static func int16(_ value: JSONValue, _ what: String) throws -> Int16 {
        try whole(value, what, as: Int16.self)
    }

    public static func int32(_ value: JSONValue, _ what: String) throws -> Int32 {
        try whole(value, what, as: Int32.self)
    }

    public static func uint64(_ value: JSONValue, _ what: String) throws -> UInt64 {
        let text = try wideText(value, what)
        guard let found = UInt64(text) else {
            throw StateRefusal("'\(what)' (\(text)) is not a whole number its type can hold")
        }
        return found
    }

    public static func int64(_ value: JSONValue, _ what: String) throws -> Int64 {
        let text = try wideText(value, what)
        guard let found = Int64(text) else {
            throw StateRefusal("'\(what)' (\(text)) is not a whole number its type can hold")
        }
        return found
    }

    public static func float32(_ value: JSONValue, _ what: String) throws -> Float {
        Float(try real(value, what))
    }

    public static func float64(_ value: JSONValue, _ what: String) throws -> Double {
        try real(value, what)
    }

    public static func bool(_ value: JSONValue, _ what: String) throws -> Bool {
        guard case .bool(let found) = value else { throw notA(value, what, "a truth value") }
        return found
    }

    public static func string(_ value: JSONValue, _ what: String) throws -> String {
        guard case .text(let found) = value else { throw notA(value, what, "a text") }
        return found
    }

    /// A text read back only if it holds no more than the `capacity` UTF-8 bytes the machine bounds it by: a
    /// machine never holds more, and a restored one must not be the first to.
    public static func string(_ value: JSONValue, _ what: String, capacity: Int) throws -> String {
        let found = try string(value, what)
        let bytes = found.utf8.count
        if bytes > capacity {
            throw StateRefusal("'\(what)' holds \(bytes) UTF-8 bytes, past the \(capacity) it is bounded by")
        }
        return found
    }
}

// MARK: Composed values

/// An enum a machine holds, as its enum document names the variant: what a saved state writes for it, and the
/// only spelling it reads one back from, so a state outlives a build that renumbered the variants.
public protocol SavedEnumeration {
    /// What the machine's document calls the enum, which a refusal names.
    static var savedAlias: String { get }

    /// The name the enum document declares for the value.
    var declaredName: String { get }

    /// The variant the enum document gives `declaredName`, and nil for a name it does not declare.
    init?(declaredName: String)
}

extension SavedValues {
    /// An enum, by the name its document declares for the variant.
    public static func of<E: SavedEnumeration>(_ value: E) -> JSONValue { .text(value.declaredName) }

    /// An enum read back from the name of one of its variants, refused unless it is one.
    public static func enumeration<E: SavedEnumeration>(
        _ value: JSONValue, _ what: String, as type: E.Type
    ) throws -> E {
        let declared = try string(value, what)
        guard let found = E(declaredName: declared) else {
            throw StateRefusal("'\(what)' (\(declared)) is not a variant of \(E.savedAlias)")
        }
        return found
    }

    /// A byte string as its byte-exact Latin-1 text, each byte the character of that code point (docs/adr/0005,
    /// decision 2): the spelling it takes on the wire, so one value is one text on every backend, and a JSON
    /// reader that has no byte string reads it as a string.
    public static func of(_ value: [UInt8]) -> JSONValue { .text(bytesAsPayloadText(value)) }

    /// A byte string read back from its Latin-1 text (`of`), only if it holds no more than the `capacity`
    /// bytes the machine bounds it by: a machine never holds more, and a restored one must not be the first
    /// to. A character past U+00FF is no byte, and is refused rather than cut.
    public static func bytes(_ value: JSONValue, _ what: String, capacity: Int = .max) throws -> [UInt8] {
        let text = try string(value, what)
        var out: [UInt8] = []
        out.reserveCapacity(text.utf8.count)
        for scalar in text.unicodeScalars {
            guard scalar.value <= 0xFF else {
                let hex = String(scalar.value, radix: 16, uppercase: true)
                throw StateRefusal(
                    "'\(what)' holds U+\(String(repeating: "0", count: max(0, 4 - hex.count)) + hex), "
                        + "which is past U+00FF and so no byte")
            }
            out.append(UInt8(scalar.value))
        }
        if out.count > capacity {
            throw StateRefusal("'\(what)' holds \(out.count) bytes, past the \(capacity) it is bounded by")
        }
        return out
    }

    /// A value that may be absent (docs/adr/0055, section 4): `null` where it is absent, and otherwise the
    /// value `present` writes, as a value of its type is written anywhere else.
    public static func optional<T>(_ value: T?, _ present: (T) -> JSONValue) -> JSONValue {
        guard let value else { return .null }
        return present(value)
    }

    /// A value that may be absent read back: absent where the saved state holds `null`, and otherwise the value
    /// `present` reads, held to its type as any value is. Only a place the machine declares optional reads
    /// through this, so a `null` saved where the machine holds a value of `T` is refused by the reader of `T`,
    /// naming what holds it (`notA`).
    public static func optional<T>(
        _ value: JSONValue, _ what: String, _ present: (JSONValue, String) throws -> T
    ) rethrows -> T? {
        if case .null = value { return nil }
        return try present(value, what)
    }

    /// A list, each element written by `element`.
    public static func list<T>(_ value: [T], _ element: (T) -> JSONValue) -> JSONValue {
        .array(value.map(element))
    }

    /// A list read back element by element, only if it holds no more than the `capacity` the machine bounds
    /// it by: a machine never holds more, and a restored one must not be the first to.
    public static func list<T>(
        _ value: JSONValue, _ what: String, capacity: Int, _ element: (JSONValue, String) throws -> T
    ) throws -> [T] {
        guard case .array(let items) = value else { throw notA(value, what, "an array") }
        if items.count > capacity {
            throw StateRefusal("'\(what)' holds \(items.count) elements, past the \(capacity) it is bounded by")
        }
        return try items.enumerated().map { index, item in try element(item, "\(what)[\(index)]") }
    }

    /// The field `name` of a saved record `value`: what a generated record reads each field with.
    public static func field(_ value: JSONValue, _ record: String, _ name: String) throws -> JSONValue {
        guard case .object(let members) = value else { throw StateRefusal("'\(record)' is not an object") }
        guard let found = members.first(where: { $0.key.utf8.elementsEqual(name.utf8) }) else {
            throw StateRefusal("'\(record)' has no field '\(name)'")
        }
        return found.value
    }

    /// A record, its fields keyed by their ids in the schema's order.
    public static func record(_ fields: [(key: String, value: JSONValue)]) -> JSONValue { .object(fields) }

    /// A union (docs/adr/0092, section 6.1 item 7): one object, the tag under the key `tag` as the declared
    /// name `variant`, before the members of `record`, the record of the case as it writes itself, or nothing
    /// for an empty case (`nil`). What a generated union writes each case with.
    public static func union(tag: String, variant: String, _ record: JSONValue?) -> JSONValue {
        var members: [(key: String, value: JSONValue)] = [(key: tag, value: .text(variant))]
        if case .object(let fields)? = record {
            members.append(contentsOf: fields)
        }
        return .object(members)
    }

    /// The declared name of the case a saved union `value` is in: the text under its tag's key `tag`, read
    /// before any field of the case, since the tag says which fields follow (docs/adr/0092, section 4). A
    /// value that is no object, that has no tag, or whose tag is not a text is refused naming `what`; a name
    /// the union has no case for is the generated reader's to refuse.
    public static func unionTag(_ value: JSONValue, _ what: String, tag: String) throws -> String {
        try string(field(value, what, tag), "\(what).\(tag)")
    }
}
