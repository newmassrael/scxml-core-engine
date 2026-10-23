// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Lifting an event's typed `_event.data` view out of the data it carries.
//
// NL-to-IR Item C1 Path A gives a schema'd event a typed payload that the natively
// lowered guards read. One producer fills it: the generated `<Machine>Raise<Event>`
// inject seam. Every other producer (`<send>` with `<param>`, namelist or
// `<content>`, an invoke forwarding an event either way, autoforward, BasicHTTP,
// mesh) fills `EventMetadata.data`, the wire §scxml-5.10 describes and
// §scxml-B-2-8-1 reads.
//
// Until this module the two never met, so the same guard answered differently
// depending on where its event came from, and a typed payload could not cross an
// invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE already
// refuses for the same guard, measured on the same document: no data, a missing
// field, or a value of another type each give `error.execution` and a guard that
// does not fire. A native lowering that answered differently would make the
// optimisation observable, which is the one thing it may not be.
//
// Cross-language siblings: `sce_rust_runtime::event_payload` (Rust),
// `sce_runtime.event_payload` (Python), `sce.LiftPayload` (Go) and
// `SCE::Common::EventPayloadFields` (C++).

/// Why an event's data cannot be read as its schema's fields.
///
/// A sentence, not a code: it rides in the `error.execution` event's data, where
/// the document's own handler can read it.
public struct PayloadRefusal: Error, Equatable, CustomStringConvertible {
    /// Why the payload was refused, in the words the document's handler reads.
    public var reason: String

    public init(_ reason: String) {
        self.reason = reason
    }

    public var description: String {
        reason
    }
}

/// The fields an event's data names, each in the spelling it was written in
/// (`Json.swift`), so a whole number stays exact until it is read at the width its
/// schema declares. A nested value this document's schema does not name is kept, so
/// a payload carrying more than the schema does still reads.
public struct PayloadFields {
    private var fields: [(key: String, value: JSONValue)]

    private init(fields: [(key: String, value: JSONValue)]) {
        self.fields = fields
    }

    /// Reads an event's data as the fields it names.
    ///
    /// The JSON read is §scxml-B-2-8-1's second rung, the same one the script engine
    /// takes for the same string, through the runtime's one JSON reader.
    public static func decode(_ data: String) throws -> PayloadFields {
        let trimmed = trimmingJSONWhitespace(data)
        if trimmed.isEmpty {
            throw PayloadRefusal("the event carries no data")
        }
        if trimmed.first != "{" {
            throw PayloadRefusal(
                "the event's data is a bare value, and this event's schema declares named fields"
            )
        }
        let value: JSONValue
        do {
            value = try parseJSON(trimmed)
        } catch JSONError.trailing {
            throw PayloadRefusal("the event's data carries more than one JSON value")
        } catch let error as JSONError {
            throw PayloadRefusal("the event's data is not JSON (\(error.malformedWhat))")
        }
        guard case .object(let members) = value else {
            preconditionFailure("a text that starts with '{' and parses is an object")
        }
        return PayloadFields(fields: members)
    }

    /// `decode` for a schema a later revision of which added fields: `defaults` is
    /// the JSON object of those fields' defaults, which the generator spells from
    /// the schema, and a field the delivery lacks is read out of it instead
    /// (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").
    ///
    /// A delivery with no data at all lacks every field, so it is read as one that
    /// names none: a field of the first revision is then refused by name, and a
    /// schema all of whose fields came later reads its defaults. A field the
    /// delivery does carry is read as it came, and held to its type as any other
    /// value is.
    public static func decode(_ data: String, defaults: String) throws -> PayloadFields {
        var decoded = trimmingJSONWhitespace(data).isEmpty ? PayloadFields(fields: []) : try decode(data)
        guard case .object(let fallback)? = try? parseJSON(trimmingJSONWhitespace(defaults)) else {
            throw PayloadRefusal(
                "the schema's defaults are not a JSON object, which the generator never writes"
            )
        }
        for (name, value) in fallback where !decoded.fields.contains(where: { $0.key.utf8.elementsEqual(name.utf8) }) {
            decoded.fields.append((key: name, value: value))
        }
        return decoded
    }

    /// A field of `optional<T>` (docs/adr/0055, section 4): absent where the
    /// delivery carries `null`, and otherwise the value `read` reads of it, held to
    /// `T` as a field of `T` is. A delivery that lacks the key is refused as it is
    /// for any field: `null` is the one spelling of absence, and a missing key means
    /// a sender whose revision does not have the field, which `decode(_:defaults:)`
    /// has already filled where a later revision added it.
    ///
    /// `read` is the reader of a present value, handed these fields:
    /// `try fields.optional("due") { try $0.signed64("due") }`.
    public func optional<T>(_ name: String, read: (PayloadFields) throws -> T) throws -> T? {
        if case .null = try find(name) {
            return nil
        }
        return try read(self)
    }

    private func find(_ name: String) throws -> JSONValue {
        guard let found = fields.first(where: { $0.key.utf8.elementsEqual(name.utf8) }) else {
            throw PayloadRefusal("the event's data has no '\(name)'")
        }
        return found.value
    }

    /// A whole-number field's written spelling.
    ///
    /// A truth value is not a number here. JSON spells both, and a schema that
    /// declared `uint32` and received `true` has been handed something its own type
    /// says cannot occur.
    private func number(_ name: String) throws -> String {
        guard case .number(let text) = try find(name) else {
            throw PayloadRefusal("'\(name)' is not a number")
        }
        return text
    }

    /// A signed whole number at the width its schema declares.
    ///
    /// A value the width cannot hold is refused and not wrapped: silently
    /// truncating is how a guard on a boundary answers for a value the document
    /// never received.
    public func signed<T: FixedWidthInteger & SignedInteger>(_ name: String) throws -> T {
        let text = try number(name)
        guard let whole = Int64(text) else {
            throw PayloadRefusal("'\(name)' is not a whole number (\(text))")
        }
        guard let narrowed = T(exactly: whole) else {
            throw PayloadRefusal("'\(name)' does not fit the width its schema declares (\(text))")
        }
        return narrowed
    }

    /// An unsigned whole number at the width its schema declares.
    public func unsigned<T: FixedWidthInteger & UnsignedInteger>(_ name: String) throws -> T {
        let text = try number(name)
        guard let whole = UInt64(text) else {
            throw PayloadRefusal("'\(name)' is not a whole number at or above zero (\(text))")
        }
        guard let narrowed = T(exactly: whole) else {
            throw PayloadRefusal("'\(name)' does not fit the width its schema declares (\(text))")
        }
        return narrowed
    }

    /// A 64-bit whole number's written spelling: its decimal digits as a text, or a
    /// JSON number when it is exact in a double (`|n| <= 2^53 - 1`).
    ///
    /// A 64-bit integer crosses as a text because a reader that takes a JSON number
    /// through a double carries one past 2^53 with its low bits wrong and no error;
    /// a number past that is refused here on every engine, so the same payload
    /// gives the same field wherever it is read.
    private func wide(_ name: String, signed: Bool) throws -> String {
        switch try find(name) {
        case .text(let text) where isDecimal(text, signed: signed):
            return text
        case .text(let text):
            throw PayloadRefusal("'\(name)' is not a whole number written as digits (\(text))")
        case .number(let text):
            guard let whole = Int128(text) else {
                throw PayloadRefusal("'\(name)' is not a whole number (\(text))")
            }
            if whole.magnitude <= exactInADouble {
                return text
            }
            throw PayloadRefusal(
                "'\(name)' is a number past 2^53 - 1, which is not exact everywhere (\(text)); "
                    + "a 64-bit integer that large is written as a text of its digits"
            )
        default:
            throw PayloadRefusal("'\(name)' is not a number")
        }
    }

    /// A 64-bit signed field: the canonical text of its digits, or a JSON number no
    /// larger than 2^53 - 1 in magnitude.
    public func signed64(_ name: String) throws -> Int64 {
        let text = try wide(name, signed: true)
        guard let value = Int64(text) else {
            throw PayloadRefusal("'\(name)' does not fit the width its schema declares (\(text))")
        }
        return value
    }

    /// A 64-bit unsigned field: the canonical text of its digits, or a JSON number
    /// no larger than 2^53 - 1.
    public func unsigned64(_ name: String) throws -> UInt64 {
        let text = try wide(name, signed: false)
        guard let value = UInt64(text) else {
            throw PayloadRefusal("'\(name)' does not fit the width its schema declares (\(text))")
        }
        return value
    }

    /// A 32-bit fractional field: the binary32 nearest the number the payload
    /// carries.
    ///
    /// A JSON number is a double wherever it is read, so the single is the one
    /// nearest that double (every engine rounds once, from the same double) and a
    /// number past the largest single does not fit the field, as a whole number past
    /// its width does not. Parsing the text to a `Float` directly would round from
    /// the text instead, and give infinity for `1e39`.
    public func float32(_ name: String) throws -> Float {
        let text = try number(name)
        guard let wide = Double(text) else {
            throw PayloadRefusal("'\(name)' is not a number (\(text))")
        }
        if abs(wide) > Double(Float.greatestFiniteMagnitude) {
            throw PayloadRefusal("'\(name)' does not fit the width its schema declares (\(text))")
        }
        return Float(wide)
    }

    /// A 64-bit fractional field.
    public func float64(_ name: String) throws -> Double {
        let text = try number(name)
        guard let value = Double(text) else {
            throw PayloadRefusal("'\(name)' is not a number (\(text))")
        }
        return value
    }

    /// A truth-value field.
    public func truth(_ name: String) throws -> Bool {
        guard case .bool(let value) = try find(name) else {
            throw PayloadRefusal("'\(name)' is not a truth value")
        }
        return value
    }

    /// A text field.
    public func text(_ name: String) throws -> String {
        guard case .text(let value) = try find(name) else {
            throw PayloadRefusal("'\(name)' is not a text")
        }
        return value
    }

    /// A byte-string field, at the capacity its schema declares when it declares
    /// one.
    ///
    /// JSON has no byte string, so the wire carries the byte-exact Latin-1 text the
    /// inject seam writes, and this reads it back the same way: every one of the 256
    /// values is one character and back. Printable ASCII, what a bytes guard
    /// compares, is the same bytes under either reading.
    public func bytes(_ name: String, capacity: Int? = nil) throws -> [UInt8] {
        guard case .text(let text) = try find(name) else {
            throw PayloadRefusal("'\(name)' is not a byte string")
        }
        var out: [UInt8] = []
        out.reserveCapacity(text.utf8.count)
        for scalar in text.unicodeScalars {
            if scalar.value > 0xFF {
                throw PayloadRefusal(
                    "'\(name)' carries a character above U+00FF, which no single byte spells"
                )
            }
            out.append(UInt8(scalar.value))
        }
        if let capacity, out.count > capacity {
            throw PayloadRefusal("'\(name)' is \(out.count) bytes, past the \(capacity) its schema declares")
        }
        return out
    }
}

extension JSONError {
    /// The sentence a malformed text carries, without the `not JSON (...)` frame.
    fileprivate var malformedWhat: String {
        if case .malformed(let what) = self {
            return what
        }
        return description
    }
}

/// The largest whole number every engine holds exactly as a number (`2^53 - 1`,
/// ECMAScript's `Number.MAX_SAFE_INTEGER`), and so the largest a JSON number may
/// carry for a 64-bit field.
private let exactInADouble: UInt128 = (1 << 53) - 1

/// `text` without the JSON whitespace around it.
private func trimmingJSONWhitespace(_ text: String) -> String {
    var bytes = ArraySlice(text.utf8)
    let isSpace = { (byte: UInt8) in byte == 0x20 || byte == 0x09 || byte == 0x0A || byte == 0x0D }
    while let first = bytes.first, isSpace(first) {
        bytes = bytes.dropFirst()
    }
    while let last = bytes.last, isSpace(last) {
        bytes = bytes.dropLast()
    }
    return String(decoding: bytes, as: UTF8.self)
}

/// Whether `text` is a whole number written as digits and nothing else: no sign but
/// `-` (and only where `signed`), no leading zero, no `-0`.
private func isDecimal(_ text: String, signed: Bool) -> Bool {
    var digits = Substring(text)
    if digits.first == "-" {
        if !signed {
            return false
        }
        digits = digits.dropFirst()
    }
    return !digits.isEmpty
        && digits.utf8.allSatisfy { $0 >= 0x30 && $0 <= 0x39 }
        && (digits == "0" || digits.first != "0")
        && !(digits == "0" && text.first == "-")
}

/// One field of an inject seam's payload, spelled as JSON, in schema order.
public struct PayloadPair {
    fileprivate var name: String
    fileprivate var json: String

    /// A text field.
    public static func text(_ name: String, _ value: String) -> PayloadPair {
        PayloadPair(name: name, json: quoteJSON(value))
    }

    /// A whole-number field up to 32 bits wide, written as a JSON number.
    public static func integer<T: FixedWidthInteger>(_ name: String, _ value: T) -> PayloadPair {
        PayloadPair(name: name, json: String(value))
    }

    /// A 64-bit signed field, written as the text of its digits.
    public static func signed64(_ name: String, _ value: Int64) -> PayloadPair {
        PayloadPair(name: name, json: quoteJSON(int64PayloadText(value)))
    }

    /// A 64-bit unsigned field, written as the text of its digits.
    public static func unsigned64(_ name: String, _ value: UInt64) -> PayloadPair {
        PayloadPair(name: name, json: quoteJSON(uint64PayloadText(value)))
    }

    /// A fractional field, spelled as ECMAScript spells a number. JSON has no
    /// spelling for a value that is not finite, so it is `null`.
    public static func real(_ name: String, _ value: Double) -> PayloadPair {
        PayloadPair(name: name, json: value.isFinite ? numberText(value) : "null")
    }

    /// A truth-value field.
    public static func truth(_ name: String, _ value: Bool) -> PayloadPair {
        PayloadPair(name: name, json: value ? "true" : "false")
    }

    /// A byte-string field, written as its Latin-1 text (see `PayloadFields.bytes`).
    public static func bytes(_ name: String, _ value: [UInt8]) -> PayloadPair {
        PayloadPair(name: name, json: quoteJSON(bytesAsPayloadText(value)))
    }

    /// A field whose value is already JSON text, such as an optional's `null`.
    public static func raw(_ name: String, json: String) -> PayloadPair {
        PayloadPair(name: name, json: json)
    }
}

/// The wire spelling of the fields an inject seam was given, so that the script
/// engine binds `_event.data` to the same values the typed carrier holds.
///
/// The fields are written in the order they are given, which is the order the
/// schema declares them, the order every other engine writes the same payload in.
/// Names and texts are written with `escapeJSONString`, the form every engine
/// writes a string in.
public func payloadJSON(_ fields: PayloadPair...) -> String {
    payloadJSON(fields)
}

/// `payloadJSON` over an array of fields.
public func payloadJSON(_ fields: [PayloadPair]) -> String {
    var out = "{"
    for (i, field) in fields.enumerated() {
        if i > 0 {
            out += ","
        }
        out += quoteJSON(field.name)
        out += ":"
        out += field.json
    }
    out += "}"
    return out
}

/// The text of the digits a 64-bit signed integer crosses as.
public func int64PayloadText(_ value: Int64) -> String {
    String(value)
}

/// The text of the digits a 64-bit unsigned integer crosses as.
public func uint64PayloadText(_ value: UInt64) -> String {
    String(value)
}

/// The Latin-1 spelling of a byte string, for the inject seam's `data` beside the
/// typed carrier: every byte is the character of that code point.
public func bytesAsPayloadText(_ bytes: [UInt8]) -> String {
    var text = String.UnicodeScalarView()
    for byte in bytes {
        text.append(Unicode.Scalar(byte))
    }
    return String(text)
}

/// The JSON spelling of one optional value, for the inject seam's `data`
/// (docs/adr/0055, section 4): an absent value is `null` and a present one is what
/// `write` spells it as. The key is always written, so a reader never takes a
/// sender that knows the field for one that does not (`PayloadFields.optional`).
public func optionalJSON<T>(_ value: T?, write: (T) -> String) -> String {
    value.map(write) ?? "null"
}
