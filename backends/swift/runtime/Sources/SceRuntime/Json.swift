// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The runtime's one JSON reader and writer.
//
// Two things read JSON here: an event's `_event.data` lifted to its typed payload
// (`EventPayload.swift`) and a machine's saved state read back. One reader serves
// both, so a text one accepts and the other refuses cannot exist; one writer
// serves the saved state and the inject seam's `data`. The test profile's test log
// is written in the JSON Canonicalization Scheme by `writeCanonicalJSON`, over the
// same escaping (docs/SCE_TEST_PROFILE.md §12.5).
//
// A number keeps the spelling it was written in (`JSONValue.number`), so a whole
// number stays exact until the one who reads it chooses its width: a 64-bit value
// never passes through a `Double`. An object keeps the order its members were
// written in, since the text one machine writes is every backend's.
//
// Why not `JSONSerialization` and `JSONDecoder`: they take a number through a
// `Double`, sort or lose the order of an object's members, and escape differently
// from the one form every engine writes (ARCHITECTURE.md, "JSON Text (Single
// Source of Truth)").
//
// Cross-language siblings: `sce_rust_runtime::json` (Rust), `com.sce.runtime.Json`
// (Kotlin) and `ParseJSON` and `WriteJSON` (Go).

/// A JSON value.
public indirect enum JSONValue: Equatable, Sendable {
    /// `null`.
    case null

    /// `true` or `false`.
    case bool(Bool)

    /// A number, in the spelling it was written in.
    case number(String)

    /// A text, its escapes already read.
    case text(String)

    /// An array's items in order.
    case array([JSONValue])

    /// An object's members in the order they were written.
    case object([(key: String, value: JSONValue)])

    public static func == (lhs: JSONValue, rhs: JSONValue) -> Bool {
        switch (lhs, rhs) {
        case (.null, .null): return true
        case (.bool(let a), .bool(let b)): return a == b
        case (.number(let a), .number(let b)): return a == b
        case (.text(let a), .text(let b)): return a.utf8.elementsEqual(b.utf8)
        case (.array(let a), .array(let b)): return a == b
        case (.object(let a), .object(let b)):
            return a.count == b.count
                && zip(a, b).allSatisfy { $0.key.utf8.elementsEqual($1.key.utf8) && $0.value == $1.value }
        default: return false
        }
    }

    /// The member `name` of an object; nil for a missing member or a value that is
    /// not an object.
    public func member(_ name: String) -> JSONValue? {
        guard case .object(let members) = self else {
            return nil
        }
        return members.first { $0.key.utf8.elementsEqual(name.utf8) }?.value
    }
}

/// Why a text is not one JSON value.
public enum JSONError: Error, Equatable, CustomStringConvertible {
    /// The text breaks the grammar; the sentence says where.
    case malformed(String)

    /// One value is followed by more text.
    case trailing

    public var description: String {
        switch self {
        case .malformed(let what): return "not JSON (\(what))"
        case .trailing: return "more than one JSON value"
        }
    }
}

/// Reads `text` as exactly one JSON value, surrounding whitespace aside.
public func parseJSON(_ text: String) throws -> JSONValue {
    var reader = JSONReader(bytes: Array(text.utf8))
    let value = try reader.readValue()
    reader.skipWhitespace()
    if !reader.atEnd {
        throw JSONError.trailing
    }
    return value
}

/// `value` as compact JSON text. A number is written in the spelling it carries,
/// so a value `parseJSON` read is written back unchanged.
public func writeJSON(_ value: JSONValue) -> String {
    var out = ""
    writeJSON(value, into: &out)
    return out
}

private func writeJSON(_ value: JSONValue, into out: inout String) {
    switch value {
    case .null:
        out += "null"
    case .bool(let b):
        out += b ? "true" : "false"
    case .number(let n):
        out += n
    case .text(let s):
        out += quoteJSON(s)
    case .array(let items):
        out += "["
        for (i, item) in items.enumerated() {
            if i > 0 {
                out += ","
            }
            writeJSON(item, into: &out)
        }
        out += "]"
    case .object(let members):
        out += "{"
        for (i, member) in members.enumerated() {
            if i > 0 {
                out += ","
            }
            out += quoteJSON(member.key)
            out += ":"
            writeJSON(member.value, into: &out)
        }
        out += "}"
    }
}

/// Why a value has no form in the JSON Canonicalization Scheme (`writeCanonicalJSON`).
public struct NotCanonical: Error, Equatable, CustomStringConvertible {
    /// Why.
    public let reason: String

    public init(_ reason: String) {
        self.reason = reason
    }

    public var description: String {
        reason
    }
}

/// `value` in the JSON Canonicalization Scheme (RFC 8785): no whitespace, the members of each object
/// sorted by their names as sequences of UTF-16 code units (section 3.2.3), each text in the one form
/// every engine writes (`quoteJSON`, which is ECMAScript's `JSON.stringify` of a text: section
/// 3.2.2.2), and each number as ECMAScript spells the double it reads as (`numberText`, section
/// 3.2.2.3). Two values equal under the scheme are then written as the same bytes, whatever order their
/// members were built in.
///
/// The order is not Swift's `String` order, which compares by canonical equivalence and by Unicode
/// scalars: two names are compared by their `utf16` views, unit by unit, and two names are the same
/// name only where those units are the same.
///
/// Refused, naming why: a number in no JSON spelling, one past the range of a double, which the scheme
/// has no spelling for, and an object that names one member twice, which has no single meaning
/// (section 3.1).
///
/// Cross-language siblings: `sce_rust_runtime::json::write_canonical` (Rust), `Json.writeJcs`
/// (Kotlin).
public func writeCanonicalJSON(_ value: JSONValue) throws(NotCanonical) -> String {
    var out = ""
    try writeCanonicalJSON(value, into: &out)
    return out
}

private func writeCanonicalJSON(_ value: JSONValue, into out: inout String) throws(NotCanonical) {
    switch value {
    case .null:
        out += "null"
    case .bool(let b):
        out += b ? "true" : "false"
    case .number(let spelling):
        guard isJSONNumber(spelling) else {
            throw NotCanonical("'\(spelling)' is not a number")
        }
        guard let number = Double(spelling), number.isFinite else {
            throw NotCanonical(
                "the number \(spelling) is past the range of a double, which RFC 8785 writes every number as")
        }
        out += numberText(number)
    case .text(let s):
        out += quoteJSON(s)
    case .array(let items):
        out += "["
        for (i, item) in items.enumerated() {
            if i > 0 {
                out += ","
            }
            try writeCanonicalJSON(item, into: &out)
        }
        out += "]"
    case .object(let members):
        let sorted = members.sorted { $0.key.utf16.lexicographicallyPrecedes($1.key.utf16) }
        for (left, right) in zip(sorted, sorted.dropFirst()) where left.key.utf16.elementsEqual(right.key.utf16) {
            throw NotCanonical("the member \"\(left.key)\" appears twice")
        }
        out += "{"
        for (i, member) in sorted.enumerated() {
            if i > 0 {
                out += ","
            }
            out += quoteJSON(member.key)
            out += ":"
            try writeCanonicalJSON(member.value, into: &out)
        }
        out += "}"
    }
}

/// Whether `spelling` is one number in RFC 8259's grammar (section 6), whole, with no whitespace
/// around it: the grammar `parseJSON` reads a number by. `Double`'s own reader takes more, a hex
/// float and `nan` among it, which no JSON text holds.
private func isJSONNumber(_ spelling: String) -> Bool {
    var reader = JSONReader(bytes: Array(spelling.utf8))
    guard let first = reader.bytes.first, first == UInt8(ascii: "-") || (0x30...0x39).contains(first) else {
        return false
    }
    guard (try? reader.readNumber()) != nil else {
        return false
    }
    return reader.atEnd
}

/// The JSON spelling of one text: escaped and quoted.
public func quoteJSON(_ text: String) -> String {
    "\"" + escapeJSONString(text) + "\""
}

/// `text` as the body of a JSON string: escaped, unquoted.
///
/// The one form SCE writes on every engine (ARCHITECTURE.md, "JSON Text (Single
/// Source of Truth)"): `"` and `\` escaped, the five short forms `\b \f \n \r \t`,
/// every other U+0000-U+001F as `\u00xx` in lowercase hex, and everything else as
/// it is. `tests/json_text/string_escape.json` holds the cases.
public func escapeJSONString(_ text: String) -> String {
    let hex = Array("0123456789abcdef".utf8)
    var out: [UInt8] = []
    out.reserveCapacity(text.utf8.count)
    for c in text.utf8 {
        switch c {
        case UInt8(ascii: "\""):
            out.append(contentsOf: [0x5C, 0x22])
        case UInt8(ascii: "\\"):
            out.append(contentsOf: [0x5C, 0x5C])
        case 0x08:
            out.append(contentsOf: [0x5C, UInt8(ascii: "b")])
        case 0x0C:
            out.append(contentsOf: [0x5C, UInt8(ascii: "f")])
        case 0x0A:
            out.append(contentsOf: [0x5C, UInt8(ascii: "n")])
        case 0x0D:
            out.append(contentsOf: [0x5C, UInt8(ascii: "r")])
        case 0x09:
            out.append(contentsOf: [0x5C, UInt8(ascii: "t")])
        case 0..<0x20:
            out.append(contentsOf: [0x5C, UInt8(ascii: "u"), 0x30, 0x30, hex[Int(c >> 4)], hex[Int(c & 0x0F)]])
        default:
            out.append(c)
        }
    }
    return String(decoding: out, as: UTF8.self)
}

private struct JSONReader {
    let bytes: [UInt8]
    var at = 0

    init(bytes: [UInt8]) {
        self.bytes = bytes
    }

    var atEnd: Bool { at >= bytes.count }

    mutating func skipWhitespace() {
        while at < bytes.count {
            switch bytes[at] {
            case 0x20, 0x09, 0x0A, 0x0D: at += 1
            default: return
            }
        }
    }

    mutating func readValue() throws -> JSONValue {
        skipWhitespace()
        if atEnd {
            throw JSONError.malformed("it ends where a value was expected")
        }
        let c = bytes[at]
        switch c {
        case UInt8(ascii: "{"): return try readObject()
        case UInt8(ascii: "["): return try readArray()
        case UInt8(ascii: "\""): return .text(try readString())
        case UInt8(ascii: "t"):
            try readKeyword("true")
            return .bool(true)
        case UInt8(ascii: "f"):
            try readKeyword("false")
            return .bool(false)
        case UInt8(ascii: "n"):
            try readKeyword("null")
            return .null
        case UInt8(ascii: "-"), 0x30...0x39: return try readNumber()
        default:
            throw JSONError.malformed("unexpected '\(String(decoding: [c], as: UTF8.self))'")
        }
    }

    mutating func readObject() throws -> JSONValue {
        at += 1  // '{'
        var members: [(key: String, value: JSONValue)] = []
        skipWhitespace()
        if !atEnd && bytes[at] == UInt8(ascii: "}") {
            at += 1
            return .object(members)
        }
        while true {
            skipWhitespace()
            if atEnd || bytes[at] != UInt8(ascii: "\"") {
                throw JSONError.malformed("a field name was expected")
            }
            let key = try readString()
            // An object whose names repeat has no single meaning: a reader that
            // takes the first and one that takes the last would read two values
            // from one text, so it is refused as the other backends refuse it.
            if members.contains(where: { $0.key.utf8.elementsEqual(key.utf8) }) {
                throw JSONError.malformed("the field \"\(key)\" appears twice")
            }
            skipWhitespace()
            if atEnd || bytes[at] != UInt8(ascii: ":") {
                throw JSONError.malformed("a ':' was expected")
            }
            at += 1
            members.append((key: key, value: try readValue()))
            skipWhitespace()
            if atEnd {
                throw JSONError.malformed("it ends unclosed")
            }
            switch bytes[at] {
            case UInt8(ascii: ","):
                at += 1
            case UInt8(ascii: "}"):
                at += 1
                return .object(members)
            default:
                throw JSONError.malformed("a ',' or '}' was expected")
            }
        }
    }

    mutating func readArray() throws -> JSONValue {
        at += 1  // '['
        var items: [JSONValue] = []
        skipWhitespace()
        if !atEnd && bytes[at] == UInt8(ascii: "]") {
            at += 1
            return .array(items)
        }
        while true {
            items.append(try readValue())
            skipWhitespace()
            if atEnd {
                throw JSONError.malformed("it ends unclosed")
            }
            switch bytes[at] {
            case UInt8(ascii: ","):
                at += 1
            case UInt8(ascii: "]"):
                at += 1
                return .array(items)
            default:
                throw JSONError.malformed("a ',' or ']' was expected")
            }
        }
    }

    mutating func readString() throws -> String {
        at += 1  // '"'
        var out: [UInt8] = []
        while true {
            if atEnd {
                throw JSONError.malformed("a text ends unclosed")
            }
            let c = bytes[at]
            at += 1
            switch c {
            case UInt8(ascii: "\""):
                return String(decoding: out, as: UTF8.self)
            case UInt8(ascii: "\\"):
                if atEnd {
                    throw JSONError.malformed("an escape ends the text")
                }
                let escape = bytes[at]
                at += 1
                switch escape {
                case UInt8(ascii: "\""): out.append(0x22)
                case UInt8(ascii: "\\"): out.append(0x5C)
                case UInt8(ascii: "/"): out.append(0x2F)
                case UInt8(ascii: "b"): out.append(0x08)
                case UInt8(ascii: "f"): out.append(0x0C)
                case UInt8(ascii: "n"): out.append(0x0A)
                case UInt8(ascii: "r"): out.append(0x0D)
                case UInt8(ascii: "t"): out.append(0x09)
                case UInt8(ascii: "u"):
                    let unit = try readUnit()
                    // A high surrogate followed by a `\u` low one is one character,
                    // as it is where a text is held as UTF-16; a half on its own
                    // names no character, so the text is no JSON text any runtime
                    // reads alike (RFC 8259, sections 7 and 8.2).
                    if (0xD800...0xDFFF).contains(unit) {
                        guard let low = readLowSurrogate(high: unit),
                            let scalar = Unicode.Scalar(0x10000 + ((UInt32(unit) - 0xD800) << 10) + (UInt32(low) - 0xDC00))
                        else {
                            throw JSONError.malformed("a \\u escape is half of a character")
                        }
                        out.append(contentsOf: Array(String(scalar).utf8))
                        continue
                    }
                    guard let scalar = Unicode.Scalar(UInt32(unit)) else {
                        throw JSONError.malformed("a \\u escape names no character")
                    }
                    out.append(contentsOf: Array(String(scalar).utf8))
                default:
                    throw JSONError.malformed("unknown escape '\\\(String(decoding: [escape], as: UTF8.self))'")
                }
            case 0..<0x20:
                // RFC 8259, section 7: a character below U+0020 stands in a text only
                // as an escape.
                throw JSONError.malformed("a control character stands in a text unescaped")
            default:
                out.append(c)
            }
        }
    }

    /// The four hex digits of a `\u` escape.
    mutating func readUnit() throws -> UInt16 {
        if at + 4 > bytes.count {
            throw JSONError.malformed("a \\u escape is short")
        }
        var unit: UInt16 = 0
        for digit in bytes[at..<at + 4] {
            let value: UInt8
            switch digit {
            case 0x30...0x39: value = digit - 0x30
            case 0x61...0x66: value = digit - 0x61 + 10
            case 0x41...0x46: value = digit - 0x41 + 10
            default: throw JSONError.malformed("a \\u escape is not hex")
            }
            unit = unit << 4 | UInt16(value)
        }
        at += 4
        return unit
    }

    /// The `\u` low surrogate that completes the high one `high`, consumed only
    /// when it does.
    mutating func readLowSurrogate(high: UInt16) -> UInt16? {
        if high >= 0xDC00 || at + 2 > bytes.count || bytes[at] != 0x5C || bytes[at + 1] != UInt8(ascii: "u") {
            return nil
        }
        let saved = at
        at += 2
        guard let low = try? readUnit(), (0xDC00...0xDFFF).contains(low) else {
            at = saved
            return nil
        }
        return low
    }

    /// One number in RFC 8259's grammar (section 6): `-`, then `0` or a digit that
    /// is not `0` and more digits, then a fraction and an exponent, each with at
    /// least one digit. A leading zero, a fraction or an exponent with no digit,
    /// and a sign but `-` are no JSON number.
    mutating func readNumber() throws -> JSONValue {
        let start = at
        func isDigit(_ reader: JSONReader) -> Bool {
            !reader.atEnd && reader.bytes[reader.at] >= 0x30 && reader.bytes[reader.at] <= 0x39
        }
        func digits(_ reader: inout JSONReader) {
            while isDigit(reader) {
                reader.at += 1
            }
        }
        if !atEnd && bytes[at] == UInt8(ascii: "-") {
            at += 1
        }
        if !atEnd && bytes[at] == UInt8(ascii: "0") {
            at += 1
        } else if isDigit(self) {
            digits(&self)
        } else {
            throw JSONError.malformed("a number has no digits")
        }
        if !atEnd && bytes[at] == UInt8(ascii: ".") {
            at += 1
            if !isDigit(self) {
                throw JSONError.malformed("a fraction has no digits")
            }
            digits(&self)
        }
        if !atEnd && (bytes[at] == UInt8(ascii: "e") || bytes[at] == UInt8(ascii: "E")) {
            at += 1
            if !atEnd && (bytes[at] == UInt8(ascii: "+") || bytes[at] == UInt8(ascii: "-")) {
                at += 1
            }
            if !isDigit(self) {
                throw JSONError.malformed("an exponent has no digits")
            }
            digits(&self)
        }
        // A digit right after the number is a number with a leading zero (`01`),
        // which no runtime reads alike.
        if isDigit(self) {
            throw JSONError.malformed("a number has a leading zero")
        }
        let slice = bytes[start..<at]
        return .number(String(decoding: slice, as: UTF8.self))
    }

    mutating func readKeyword(_ word: String) throws {
        let letters = Array(word.utf8)
        if at + letters.count > bytes.count || !bytes[at..<at + letters.count].elementsEqual(letters) {
            throw JSONError.malformed("expected '\(word)'")
        }
        at += letters.count
    }
}
