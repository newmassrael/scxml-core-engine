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
///
/// The fields of a record value are read the same way, at the record's path
/// (`PayloadValue.record`).
public struct PayloadFields {
    private var fields: [(key: String, value: JSONValue)]

    /// The path of the record these are the fields of: empty for the payload.
    private let prefix: String

    fileprivate init(fields: [(key: String, value: JSONValue)], prefix: String = "") {
        self.fields = fields
        self.prefix = prefix
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
        let decoded = trimmingJSONWhitespace(data).isEmpty ? PayloadFields(fields: []) : try decode(data)
        return try decoded.withDefaults(defaults)
    }

    /// These fields, and the defaults of the fields a later revision of the schema
    /// added where the value lacks them.
    fileprivate func withDefaults(_ defaults: String) throws -> PayloadFields {
        guard case .object(let fallback)? = try? parseJSON(trimmingJSONWhitespace(defaults)) else {
            throw PayloadRefusal(
                "the schema's defaults are not a JSON object, which the generator never writes"
            )
        }
        var merged = self
        for (name, value) in fallback where !merged.fields.contains(where: { $0.key.utf8.elementsEqual(name.utf8) }) {
            merged.fields.append((key: name, value: value))
        }
        return merged
    }

    /// The value of the field `name`, at its path: the delivered one, else the
    /// default a later revision of the schema gave it, else a refusal naming the
    /// path. A member the schema does not declare is never asked for, so it is
    /// stepped over.
    public func field(_ name: String) throws -> PayloadValue {
        let path = joinPath(prefix, name)
        guard let found = fields.first(where: { $0.key.utf8.elementsEqual(name.utf8) }) else {
            throw PayloadRefusal("the event's data has no '\(path)'")
        }
        return PayloadValue(value: found.value, path: path)
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
        if try field(name).isNull {
            return nil
        }
        return try read(self)
    }

    /// A signed whole number at the width its schema declares (`PayloadValue.signed`).
    public func signed<T: FixedWidthInteger & SignedInteger>(_ name: String) throws -> T {
        try field(name).signed()
    }

    /// An unsigned whole number at the width its schema declares.
    public func unsigned<T: FixedWidthInteger & UnsignedInteger>(_ name: String) throws -> T {
        try field(name).unsigned()
    }

    /// A 64-bit signed field (`PayloadValue.signed64`).
    public func signed64(_ name: String) throws -> Int64 {
        try field(name).signed64()
    }

    /// A 64-bit unsigned field (`PayloadValue.unsigned64`).
    public func unsigned64(_ name: String) throws -> UInt64 {
        try field(name).unsigned64()
    }

    /// A 32-bit fractional field (`PayloadValue.float32`).
    public func float32(_ name: String) throws -> Float {
        try field(name).float32()
    }

    /// A 64-bit fractional field.
    public func float64(_ name: String) throws -> Double {
        try field(name).float64()
    }

    /// A truth-value field.
    public func truth(_ name: String) throws -> Bool {
        try field(name).truth()
    }

    /// A text field, as the text it carries; `PayloadValue.text(bound:)` holds it to
    /// its bound.
    public func text(_ name: String) throws -> String {
        try field(name).name()
    }

    /// A byte-string field, at the capacity its schema declares when it declares
    /// one (`PayloadValue.bytes(bound:)`).
    public func bytes(_ name: String, capacity: Int? = nil) throws -> [UInt8] {
        try field(name).bytes(bound: capacity ?? Int.max)
    }
}

/// One value of an event's data, at the path through every depth that names it: a
/// field's name, then each index and field name below it, joined by `/`
/// (docs/adr/0086, section 4). A field may be of any type of the language, and
/// three combinators walk the constructors that hold other values, each calling the
/// reader of its part: `list(bound:_:)`, `optional(_:)` and `record(defaults:_:)`.
/// A value outside its type is refused before anything of it is kept, and the
/// refusal names the path: `'items/2/tags/0' is 9 bytes, past the 8 its type
/// declares`.
public struct PayloadValue {
    fileprivate let value: JSONValue

    /// The path that names this value.
    public let path: String

    fileprivate init(value: JSONValue, path: String) {
        self.value = value
        self.path = path
    }

    /// Whether the value is `null`, the one spelling of an absent value.
    public var isNull: Bool {
        if case .null = value {
            return true
        }
        return false
    }

    /// A refusal of this value, saying `what` of it after its path:
    /// `'items/1/level' (x) is not a variant of Level`.
    public func refuse(_ what: String) -> PayloadRefusal {
        PayloadRefusal("'\(path)' \(what)")
    }

    /// The refusal of `null` where the type is not optional, or `otherwise`.
    private func wrongKind(_ otherwise: String) -> PayloadRefusal {
        isNull ? refuse("is null, and its type is not optional") : refuse(otherwise)
    }

    /// A whole-number value's written spelling.
    ///
    /// A truth value is not a number here. JSON spells both, and a schema that
    /// declared `uint32` and received `true` has been handed something its own type
    /// says cannot occur.
    private func number() throws -> String {
        guard case .number(let text) = value else {
            throw wrongKind("is not a number")
        }
        return text
    }

    /// A signed whole number at the width its type declares.
    ///
    /// A value the width cannot hold is refused and not wrapped: silently
    /// truncating is how a guard on a boundary answers for a value the document
    /// never received.
    public func signed<T: FixedWidthInteger & SignedInteger>() throws -> T {
        let text = try number()
        guard let whole = Int64(text) else {
            throw refuse("is not a whole number (\(text))")
        }
        guard let narrowed = T(exactly: whole) else {
            throw refuse("does not fit the width its schema declares (\(text))")
        }
        return narrowed
    }

    /// An unsigned whole number at the width its type declares.
    public func unsigned<T: FixedWidthInteger & UnsignedInteger>() throws -> T {
        let text = try number()
        guard let whole = UInt64(text) else {
            throw refuse("is not a whole number at or above zero (\(text))")
        }
        guard let narrowed = T(exactly: whole) else {
            throw refuse("does not fit the width its schema declares (\(text))")
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
    private func wide(signed: Bool) throws -> String {
        switch value {
        case .text(let text) where isDecimal(text, signed: signed):
            return text
        case .text(let text):
            throw refuse("is not a whole number written as digits (\(text))")
        case .number(let text):
            guard let whole = Int128(text) else {
                throw refuse("is not a whole number (\(text))")
            }
            if whole.magnitude <= exactInADouble {
                return text
            }
            throw refuse(
                "is a number past 2^53 - 1, which is not exact everywhere (\(text)); "
                    + "a 64-bit integer that large is written as a text of its digits"
            )
        default:
            throw wrongKind("is not a number")
        }
    }

    /// A 64-bit signed value: the canonical text of its digits, or a JSON number no
    /// larger than 2^53 - 1 in magnitude.
    public func signed64() throws -> Int64 {
        let text = try wide(signed: true)
        guard let whole = Int64(text) else {
            throw refuse("does not fit the width its schema declares (\(text))")
        }
        return whole
    }

    /// A 64-bit unsigned value: the canonical text of its digits, or a JSON number
    /// no larger than 2^53 - 1.
    public func unsigned64() throws -> UInt64 {
        let text = try wide(signed: false)
        guard let whole = UInt64(text) else {
            throw refuse("does not fit the width its schema declares (\(text))")
        }
        return whole
    }

    /// A 32-bit fractional value: the binary32 nearest the number the payload
    /// carries.
    ///
    /// A JSON number is a double wherever it is read, so the single is the one
    /// nearest that double (every engine rounds once, from the same double) and a
    /// number past the largest single does not fit, as a whole number past its width
    /// does not. Parsing the text to a `Float` directly would round from the text
    /// instead, and give infinity for `1e39`.
    public func float32() throws -> Float {
        let text = try number()
        guard let wide = Double(text) else {
            throw refuse("is not a number (\(text))")
        }
        if abs(wide) > Double(Float.greatestFiniteMagnitude) {
            throw refuse("does not fit the width its schema declares (\(text))")
        }
        return Float(wide)
    }

    /// A 64-bit fractional value.
    public func float64() throws -> Double {
        let text = try number()
        guard let real = Double(text) else {
            throw refuse("is not a number (\(text))")
        }
        return real
    }

    /// A truth value.
    public func truth() throws -> Bool {
        guard case .bool(let truth) = value else {
            throw wrongKind("is not a truth value")
        }
        return truth
    }

    /// The text the value carries, held to no bound: what an enum's variant is named
    /// by, which its reader matches against the names its enum declares.
    public func name() throws -> String {
        guard case .text(let text) = value else {
            throw wrongKind("is not a text")
        }
        return text
    }

    /// A text of at most `bound` UTF-8 bytes (docs/adr/0086, section 4.4): one past
    /// the bound is refused, at whatever depth it stands, and never cut.
    public func text(bound: Int) throws -> String {
        let text = try name()
        let size = text.utf8.count
        if size > bound {
            throw pastBytes(size, bound)
        }
        return text
    }

    /// A byte string of at most `bound` bytes.
    ///
    /// JSON has no byte string, so the wire carries the byte-exact Latin-1 text the
    /// inject seam writes, and this reads it back the same way: every one of the 256
    /// values is one character and back. Printable ASCII, what a bytes guard
    /// compares, is the same bytes under either reading.
    public func bytes(bound: Int) throws -> [UInt8] {
        guard case .text(let text) = value else {
            throw wrongKind("is not a byte string")
        }
        var out: [UInt8] = []
        out.reserveCapacity(text.utf8.count)
        for scalar in text.unicodeScalars {
            if scalar.value > 0xFF {
                throw refuse("carries a character above U+00FF, which no single byte spells")
            }
            out.append(UInt8(scalar.value))
        }
        if out.count > bound {
            throw pastBytes(out.count, bound)
        }
        return out
    }

    private func pastBytes(_ held: Int, _ bound: Int) -> PayloadRefusal {
        refuse("is \(held) bytes, past the \(bound) its type declares")
    }

    /// A value of `optional<T>` (docs/adr/0055): absent where the value is `null`,
    /// and otherwise what `read` reads of it, held to `T`.
    public func optional<T>(_ read: (PayloadValue) throws -> T) throws -> T? {
        if isNull {
            return nil
        }
        return try read(self)
    }

    /// A value of `list<T>[bound]`: an array of at most `bound` elements, each read
    /// by `read` at the path of its index, in order.
    ///
    /// The bound is held before any element is read: an array longer than `bound`
    /// is refused whole, and no element of it is decoded (ADR 0077, section 2).
    public func list<T>(bound: Int, _ read: (PayloadValue) throws -> T) throws -> [T] {
        guard case .array(let items) = value else {
            throw wrongKind("is not a list")
        }
        if items.count > bound {
            throw refuse("holds \(items.count) elements, past the \(bound) its type declares")
        }
        var out: [T] = []
        out.reserveCapacity(items.count)
        for (index, item) in items.enumerated() {
            out.append(try read(PayloadValue(value: item, path: joinPath(path, String(index)))))
        }
        return out
    }

    /// A value of a record type: an object whose fields `read` reads through the
    /// `PayloadFields` it is handed. `defaults`, for a record whose schema a later
    /// revision added fields to, is the JSON object of their defaults, which the
    /// generator spells from that schema, and a field the value lacks is read out of
    /// it (docs/adr/0086, section 10.3).
    public func record<T>(defaults: String? = nil, _ read: (PayloadFields) throws -> T) throws -> T {
        guard case .object(let members) = value else {
            throw wrongKind("is not a record")
        }
        var fields = PayloadFields(fields: members, prefix: path)
        if let defaults {
            fields = try fields.withDefaults(defaults)
        }
        return try read(fields)
    }
}

/// `segment` below `prefix`, as a JSON Pointer's reference tokens are joined.
private func joinPath(_ prefix: String, _ segment: String) -> String {
    prefix.isEmpty ? segment : prefix + "/" + segment
}

/// A value a host built is past a bound its type declares: thrown by the check a
/// generated raise runs before it encodes the event (docs/adr/0086, section 5.4).
public struct BoundExceeded: Error, Equatable, CustomStringConvertible {
    /// What is past which bound, at the path that names it.
    public var reason: String

    public init(_ reason: String) {
        self.reason = reason
    }

    public var description: String {
        reason
    }
}

/// Holds a text a host built to the bound its type declares, in UTF-8 bytes. Swift has
/// no checked constructor on a plain struct, so a generated raise runs the check its
/// composition generates, one call per constructor, and refuses the event whole.
public func checkText(_ path: String, _ value: String, bound: Int) throws {
    let size = value.utf8.count
    if size > bound {
        throw BoundExceeded("'\(path)' is \(size) bytes, past the \(bound) its type declares")
    }
}

/// Holds a byte string a host built to its bound (see `checkText`).
public func checkBytes(_ path: String, _ value: [UInt8], bound: Int) throws {
    if value.count > bound {
        throw BoundExceeded("'\(path)' is \(value.count) bytes, past the \(bound) its type declares")
    }
}

/// Holds a list a host built to its bound, and each element to its own type through
/// `each`, at the path of its index (see `checkText`).
public func checkList<T>(_ path: String, _ items: [T], bound: Int, _ each: (String, T) throws -> Void) throws {
    if items.count > bound {
        throw BoundExceeded("'\(path)' holds \(items.count) elements, past the \(bound) its type declares")
    }
    for (index, item) in items.enumerated() {
        try each(joinPath(path, String(index)), item)
    }
}

/// Holds an optional value a host built to its type through `present`, where it is
/// present (see `checkText`).
public func checkOptional<T>(_ path: String, _ value: T?, _ present: (String, T) throws -> Void) throws {
    if let value {
        try present(path, value)
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

/// One value a `<send>`'s `<param>` or `<content expr>` crosses as, of the type the
/// document's expression has: the generated machine computes it from its own fields
/// and hands it to the send, which writes it as JSON the way every engine writes the
/// same value (`sce.ScriptValueToJSON` in Go, `SceWire` in Kotlin).
public enum WireValue {
    /// A text.
    case text(String)

    /// A whole number up to 32 bits wide, written as a JSON number.
    case integer(Int64)

    /// A 64-bit signed number, written as the text of its digits.
    case signed64(Int64)

    /// A 64-bit unsigned number, written as the text of its digits.
    case unsigned64(UInt64)

    /// A real number; a value that is not finite is `null`.
    case real(Double)

    /// A truth value.
    case truth(Bool)

    /// A byte string, written as its Latin-1 text.
    case bytes([UInt8])

    /// A value that is absent, written as JSON `null`.
    case null

    /// The value as JSON text.
    public var json: String {
        switch self {
        case .text(let value): return quoteJSON(value)
        case .integer(let value): return String(value)
        case .signed64(let value): return quoteJSON(int64PayloadText(value))
        case .unsigned64(let value): return quoteJSON(uint64PayloadText(value))
        case .real(let value): return value.isFinite ? numberText(value) : "null"
        case .truth(let value): return value ? "true" : "false"
        case .bytes(let value): return quoteJSON(bytesAsPayloadText(value))
        case .null: return "null"
        }
    }

    /// The value as the text a form carries (§scxml-C-2): ECMAScript's `String(value)`,
    /// with absence as the empty text (§scxml-C-1), as every other engine puts it on the
    /// wire (`ToWireString` in Go). Not `json`, which would wrap a text in quotes that are
    /// no part of it, and write absence as a word.
    public var formText: String {
        switch self {
        case .text(let value): return value
        case .integer(let value): return String(value)
        case .signed64(let value): return int64PayloadText(value)
        case .unsigned64(let value): return uint64PayloadText(value)
        case .real(let value): return numberText(value)
        case .truth(let value): return value ? "true" : "false"
        case .bytes(let value): return bytesAsPayloadText(value)
        case .null: return ""
        }
    }
}

/// Adds a `<param>` of a BasicHTTP `<send>` to the form it posts (§scxml-C-2): the value
/// as the text a form carries it as, after the values `name` already has. A form
/// parameter may repeat where a JSON key may not, and each value is kept in document
/// order.
public func appendFormParam(_ params: inout [String: [String]], _ name: String, _ value: WireValue) {
    params[name, default: []].append(value.formText)
}

extension PayloadPair {
    /// A pair of `name` and a value of the type the expression computed it as.
    public init(_ name: String, _ value: WireValue) {
        self.init(name: name, json: value.json)
    }

    /// A pair of `name` and a value a script engine already wrote as JSON, which the pair carries as
    /// it is: an object or an array the document's expression computed keeps its shape, and the
    /// receiver's `_event.data.name` reads it as one. The text must be a JSON value; the engine that
    /// wrote it is the one place that knows it is.
    public static func json(_ name: String, rawJSON: String) -> PayloadPair {
        PayloadPair(name: name, json: rawJSON)
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
///
/// W3C SCXML 6.2: a name may repeat and every value must be delivered, which a JSON object cannot
/// say with one key twice (a reader keeps the last), so a repeated name is one member whose value is
/// the array of its values in document order, and a name that does not repeat is its value as it is.
/// Members stand in the order their names first appear.
public func payloadJSON(_ fields: [PayloadPair]) -> String {
    var names: [String] = []
    var values: [String: [String]] = [:]
    for field in fields {
        if values[field.name] == nil {
            names.append(field.name)
        }
        values[field.name, default: []].append(field.json)
    }
    var out = "{"
    for (i, name) in names.enumerated() {
        if i > 0 {
            out += ","
        }
        out += quoteJSON(name)
        out += ":"
        let all = values[name] ?? []
        out += all.count == 1 ? all[0] : "[" + all.joined(separator: ",") + "]"
    }
    out += "}"
    return out
}

/// The JSON spelling of one list (docs/adr/0086, section 3.1): an array of what
/// `write` spells each element as, in the list's order. The empty list is `[]`.
public func listJSON<T>(_ items: [T], _ write: (T) -> String) -> String {
    "[" + items.map(write).joined(separator: ",") + "]"
}

/// The JSON spelling of one record: each pair is a field's id and its value, and the
/// object writes them ascending by the id's UTF-8 bytes, whatever order they are
/// given in (ARCHITECTURE.md, "JSON Object Key Order", at every depth). A value
/// that is itself a list or a record is a `PayloadPair.json` of its writer's text.
public func recordJSON(_ members: [PayloadPair]) -> String {
    let sorted = members.sorted { $0.name.utf8.lexicographicallyPrecedes($1.name.utf8) }
    return "{" + sorted.map { quoteJSON($0.name) + ":" + $0.json }.joined(separator: ",") + "}"
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
