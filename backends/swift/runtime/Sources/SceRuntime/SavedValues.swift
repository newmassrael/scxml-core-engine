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
