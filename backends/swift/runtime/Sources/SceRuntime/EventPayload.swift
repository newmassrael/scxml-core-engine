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
// invoke boundary at all. No data, a missing field, or a value of another type is
// a delivery outside its type, which the engine refuses whole where it takes the
// event: no transition is selected for it, and an `error.execution` that carries
// the refusal is taken in its place (docs/adr/0086, section 4.2).
//
// Cross-language siblings: `sce_rust_runtime::event_payload` (Rust),
// `sce_runtime.event_payload` (Python), `sce.LiftPayload` (Go) and
// `SCE::Common::EventPayloadFields` (C++).

/// The class of a refusal: the enum `sce:std/lift/refusal_code.scxml` declares,
/// each case named and numbered as that document names and numbers it
/// (docs/adr/0087, section 2.3).
public enum RefusalCode: UInt8, CaseIterable, Equatable {
    /// The text is empty, is not JSON, carries more than one JSON value, or holds
    /// an object that repeats a name.
    case notJson = 1
    /// The text is one JSON value that is not an object.
    case notObject = 2
    /// A container is opened deeper than any a value of the schema holds.
    case tooDeep = 3
    /// A field of a record is absent and no revision default fills it.
    case missing = 4
    /// `null` stands where the type is not optional.
    case nullNotOptional = 5
    /// A value of another JSON kind.
    case wrongKind = 6
    /// A number not in its type's spelling.
    case badNumber = 7
    /// A whole number past its type's width, or a real past a `float32`'s range.
    case outOfRange = 8
    /// A text or byte string past its bound.
    case tooLong = 9
    /// A list past its bound.
    case tooMany = 10
    /// A name the enum does not declare.
    case unknownVariant = 11
    /// A byte string's text holding a character above U+00FF.
    case notByteText = 12
    /// A text holding U+0000.
    case nulInText = 13
    /// A list short of the lower count its type states (docs/adr/0086, section 15).
    case tooFew = 14

    /// The name the enum declares the code by: the case's own name.
    public var declaredName: String {
        String(describing: self)
    }

    /// The code the enum declares by `name`, if it declares one.
    public init?(declaredName name: String) {
        guard let code = RefusalCode.allCases.first(where: { $0.declaredName == name }) else {
            return nil
        }
        self = code
    }
}

/// Why an event's data cannot be read as its schema's fields.
///
/// The sentence rides in the `error.execution` event's data, where the document's
/// own handler can read it. Beside it the refusal carries what a statechart reads
/// as data (docs/adr/0087, section 2.4): its code, the place as a strict RFC 6901
/// pointer (`/items/2/tags/0`, `~` written `~0` and `/` written `~1`, empty for
/// the whole text), and the bound the value broke, for a text, a byte string or a
/// list past it.
public struct PayloadRefusal: Error, Equatable, CustomStringConvertible {
    /// Why the payload was refused, in the words the document's handler reads.
    public var reason: String
    /// The class of the refusal.
    public var code: RefusalCode
    /// The place refused, as a strict RFC 6901 pointer: empty for the whole text.
    public var pointer: String
    /// The bound the value broke, or nil for a refusal that names none.
    public var bound: UInt32?

    public init(_ reason: String, code: RefusalCode, pointer: String, bound: Int? = nil) {
        self.reason = reason
        self.code = code
        self.pointer = pointer
        // A bound no `uint32` holds is no bound a type declares, and is held at the
        // largest one.
        self.bound = bound.map { UInt32(clamping: $0) }
    }

    public var description: String {
        reason
    }
}

/// The refusal of a container `text` opens deeper than `depth`, the deepest a value
/// of a lift's schema holds (docs/adr/0087, section 2.1, item 4), or nil when it
/// opens none.
///
/// Decided over the brackets alone, before the text is read as JSON, so that every
/// runtime decides it alike and no reader descends past the type: a text is skipped
/// from its opening quote to the next quote no backslash escapes, each `{` and `[`
/// outside one opens a level and each `}` and `]` closes one, and the first opened
/// past `depth` is refused, whatever the text holds after it. A text that never
/// closes ends the walk with nothing refused, and the reading of the text as JSON
/// refuses it.
///
/// The pointer names the container refused (docs/adr/0087, section 2.4): for each
/// container it stands in, outermost first, the element's index in an array (the `,`
/// the scan met at that level since the array opened) or the member's name in an
/// object (the last text closed at that level before a `:`, since the object opened
/// or its last `,`), each an RFC 6901 token. It stops before the first place it
/// cannot write: a member with no name before its colon, a name that is no text (no
/// JSON string, or one holding U+0000), or a token that would take the pointer past
/// `pathBound` UTF-8 bytes, the room of the refusal record's `path`. The bound is
/// `depth`. The scan keeps one entry per open container, at most `depth` of them.
public func liftTooDeep(_ text: String, depth: Int, pathBound: Int) -> PayloadRefusal? {
    let bytes = Array(text.utf8)
    var open: [LiftScanLevel] = []
    var at = 0
    while at < bytes.count {
        let top = open.count - 1
        switch bytes[at] {
        case UInt8(ascii: "\""):
            let start = at
            at += 1
            while at < bytes.count && bytes[at] != UInt8(ascii: "\"") {
                at += bytes[at] == UInt8(ascii: "\\") ? 2 : 1
            }
            if at >= bytes.count {
                return nil
            }
            if top >= 0 {
                open[top].lastText = bytes[start...at]
            }
        case UInt8(ascii: ":") where top >= 0:
            open[top].member = open[top].lastText
            open[top].lastText = nil
        case UInt8(ascii: ",") where top >= 0:
            open[top].member = nil
            open[top].lastText = nil
            open[top].index += 1
        case UInt8(ascii: "{"), UInt8(ascii: "["):
            if open.count >= depth {
                return PayloadRefusal(
                    "the text opens a container deeper than the \(depth) levels a value of its schema holds",
                    code: .tooDeep,
                    pointer: liftScanPointer(open, pathBound: pathBound),
                    bound: depth
                )
            }
            open.append(LiftScanLevel(array: bytes[at] == UInt8(ascii: "[")))
        case UInt8(ascii: "}"), UInt8(ascii: "]"):
            if top >= 0 {
                open.removeLast()
            }
        default:
            break
        }
        at += 1
    }
    return nil
}

/// One container `liftTooDeep` stands in: whether it is an array, the elements it has
/// passed, the last text closed in it, and the member a `:` after that text named.
private struct LiftScanLevel {
    let array: Bool
    var index: UInt64 = 0
    var lastText: ArraySlice<UInt8>?
    var member: ArraySlice<UInt8>?

    init(array: Bool) {
        self.array = array
    }
}

/// The pointer to the place `open` stands at, as `liftTooDeep` writes it.
private func liftScanPointer(_ open: [LiftScanLevel], pathBound: Int) -> String {
    var pointer = ""
    for level in open {
        let token: String
        if level.array {
            token = String(level.index)
        } else {
            guard let member = level.member,
                case .text(let name)? = try? parseJSON(String(decoding: member, as: UTF8.self)),
                !name.unicodeScalars.contains("\0")
            else {
                break
            }
            token = name
        }
        let below = pointerBelow(pointer, token)
        if below.utf8.count > pathBound {
            break
        }
        pointer = below
    }
    return pointer
}

/// What a `<sce:lift>` reads (docs/adr/0087, section 2.1): `text` as a record of a
/// schema, whose fields `read` reads out of the fields it is handed, as a delivery
/// of the schema is read; or the refusal of the first place that is not one, its
/// code, pointer and bound the statechart keeps as a record. `PayloadFields.lift`
/// says what is refused before the fields are read. Each outcome is logged
/// (`sceLog`) with the schema's name and, for a refusal, its code, pointer and
/// bound; the text itself is not, since nothing bounds what a sender writes in it.
public func liftRecord<T>(
    _ text: String,
    schema: String,
    depth: Int,
    pathBound: Int,
    defaults: String?,
    read: (PayloadFields) throws -> T
) -> Result<T, PayloadRefusal> {
    let refusal: PayloadRefusal
    do {
        let lifted = try read(PayloadFields.lift(text, depth: depth, pathBound: pathBound, defaults: defaults))
        sceLog("<sce:lift>: the text is a record of \(schema)")
        return .success(lifted)
    } catch let refused as PayloadRefusal {
        refusal = refused
    } catch {
        // Every reader of this file refuses with a `PayloadRefusal`; an error of
        // another kind is the text not being JSON at all.
        refusal = PayloadRefusal(String(describing: error), code: .notJson, pointer: "")
    }
    sceLog(
        "<sce:lift>: the text is no record of \(schema): \(refusal.code.declaredName) at '\(refusal.pointer)', "
            + "bound \(refusal.bound.map { String($0) } ?? "none") (\(refusal.reason))"
    )
    return .failure(refusal)
}

/// `token` below the strict RFC 6901 pointer `prefix` (empty for the whole text):
/// `/` and the token with `~` written `~0` and `/` written `~1` (RFC 6901,
/// section 3).
public func pointerBelow(_ prefix: String, _ token: String) -> String {
    var escaped = ""
    for scalar in token.unicodeScalars {
        switch scalar {
        case "~": escaped += "~0"
        case "/": escaped += "~1"
        default: escaped.unicodeScalars.append(scalar)
        }
    }
    return prefix + "/" + escaped
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

    /// The strict RFC 6901 pointer to the record: empty for the payload.
    private let pointer: String

    fileprivate init(fields: [(key: String, value: JSONValue)], prefix: String = "", pointer: String = "") {
        self.fields = fields
        self.prefix = prefix
        self.pointer = pointer
    }

    /// Reads an event's data as the fields it names.
    ///
    /// The JSON read is §scxml-B-2-8-1's second rung, the same one the script engine
    /// takes for the same string, through the runtime's one JSON reader.
    public static func decode(_ data: String) throws -> PayloadFields {
        let trimmed = trimmingJSONWhitespace(data)
        if trimmed.isEmpty {
            throw PayloadRefusal("the event carries no data", code: .notJson, pointer: "")
        }
        if trimmed.first != "{" {
            // One sentence for a text that is no object, as it has always been; the
            // code tells one JSON value that is not an object from a text that is not
            // JSON at all (docs/adr/0087, section 2.3).
            let oneValue = (try? parseJSON(trimmed)) != nil
            throw PayloadRefusal(
                "the event's data is a bare value, and this event's schema declares named fields",
                code: oneValue ? .notObject : .notJson,
                pointer: ""
            )
        }
        let value: JSONValue
        do {
            value = try parseJSON(trimmed)
        } catch JSONError.trailing {
            throw PayloadRefusal("the event's data carries more than one JSON value", code: .notJson, pointer: "")
        } catch let error as JSONError {
            throw PayloadRefusal("the event's data is not JSON (\(error.malformedWhat))", code: .notJson, pointer: "")
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

    /// Reads the text a `<sce:lift>` reads as the fields of a record of a schema
    /// (docs/adr/0087, section 2.1): `depth` is the deepest container a value of the
    /// schema holds, `pathBound` the room of the refusal record's `path` a `tooDeep`
    /// pointer is held to (`liftTooDeep`), and `defaults`, for a schema a later
    /// revision of which added fields, the JSON object of their defaults, which a text
    /// that lacks one reads in its place.
    ///
    /// Three refusals come before the fields are read, in this order: a text that is
    /// empty or only whitespace (`notJson`); a container opened deeper than `depth`,
    /// decided over the text's brackets alone before it is read as JSON
    /// (`liftTooDeep`, `tooDeep`); and a text that is not one JSON object, as a
    /// delivery is refused (`decode`).
    public static func lift(_ text: String, depth: Int, pathBound: Int, defaults: String?) throws
        -> PayloadFields
    {
        if trimmingJSONWhitespace(text).isEmpty {
            throw PayloadRefusal("the text is empty", code: .notJson, pointer: "")
        }
        if let refusal = liftTooDeep(text, depth: depth, pathBound: pathBound) {
            throw refusal
        }
        let decoded = try decode(text)
        guard let defaults else {
            return decoded
        }
        return try decoded.withDefaults(defaults)
    }

    /// These fields, and the defaults of the fields a later revision of the schema
    /// added where the value lacks them.
    fileprivate func withDefaults(_ defaults: String) throws -> PayloadFields {
        guard case .object(let fallback)? = try? parseJSON(trimmingJSONWhitespace(defaults)) else {
            // A text that is none is the generator's, not a sender's, and is refused as
            // no JSON object of the whole.
            throw PayloadRefusal(
                "the schema's defaults are not a JSON object, which the generator never writes",
                code: .notJson,
                pointer: ""
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
        let at = pointerBelow(pointer, name)
        guard let found = fields.first(where: { $0.key.utf8.elementsEqual(name.utf8) }) else {
            throw PayloadRefusal("the event's data has no '\(path)'", code: .missing, pointer: at)
        }
        return PayloadValue(value: found.value, path: path, pointer: at)
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

    /// The text a field carries, held to no bound: what an enum field's variant is
    /// named by, which its reader matches against the names its enum declares. A text
    /// field is read by `text(_:bound:)`, which holds its bound.
    public func text(_ name: String) throws -> String {
        try field(name).name()
    }

    /// A text field of at most `bound` UTF-8 bytes (docs/adr/0086, section 4.4): the
    /// top level of a payload is held to its bound by `PayloadValue.text(bound:)`, the
    /// reader a text at any depth is held by.
    public func text(_ name: String, bound: Int) throws -> String {
        try field(name).text(bound: bound)
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

    /// The strict RFC 6901 pointer to this value (`/items/2`).
    public let pointer: String

    fileprivate init(value: JSONValue, path: String, pointer: String) {
        self.value = value
        self.path = path
        self.pointer = pointer
    }

    /// Whether the value is `null`, the one spelling of an absent value.
    public var isNull: Bool {
        if case .null = value {
            return true
        }
        return false
    }

    /// A refusal of this value, of class `code`, saying `what` of it after its path:
    /// `'items/1/level' (x) is not a variant of Level`.
    public func refuse(_ code: RefusalCode, _ what: String, bound: Int? = nil) -> PayloadRefusal {
        PayloadRefusal("'\(path)' \(what)", code: code, pointer: pointer, bound: bound)
    }

    /// The refusal of the text `name` this value carries, which the enum `alias`
    /// does not declare a variant by.
    public func unknownVariant(_ name: String, alias: String) -> PayloadRefusal {
        refuse(.unknownVariant, "(\(name)) is not a variant of \(alias)")
    }

    /// The refusal of `null` where the type is not optional, or `otherwise`.
    private func wrongKind(_ otherwise: String) -> PayloadRefusal {
        isNull ? refuse(.nullNotOptional, "is null, and its type is not optional") : refuse(.wrongKind, otherwise)
    }

    /// The refusal of a number past the width its type declares.
    private func pastWidth(_ text: String) -> PayloadRefusal {
        refuse(.outOfRange, "does not fit the width its schema declares (\(text))")
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
        // `-0` is a JSON number and no spelling of a whole number: every runtime
        // refuses it, as it refuses `"-0"` for a 64-bit integer.
        if text == "-0" {
            throw refuse(.badNumber, "is not a whole number (\(text))")
        }
        guard let whole = Int64(text) else {
            // A whole number no Int64 holds is past every signed width; any other
            // spelling is no whole number at all.
            throw refuse(spellsWhole(text) ? .outOfRange : .badNumber, "is not a whole number (\(text))")
        }
        guard let narrowed = T(exactly: whole) else {
            throw pastWidth(text)
        }
        return narrowed
    }

    /// An unsigned whole number at the width its type declares.
    public func unsigned<T: FixedWidthInteger & UnsignedInteger>() throws -> T {
        let text = try number()
        guard !text.hasPrefix("-"), let whole = UInt64(text) else {
            // Below zero is no spelling of an unsigned number; a whole number no
            // UInt64 holds is past every unsigned width.
            let past = spellsWhole(text) && !text.hasPrefix("-")
            throw refuse(past ? .outOfRange : .badNumber, "is not a whole number at or above zero (\(text))")
        }
        guard let narrowed = T(exactly: whole) else {
            throw pastWidth(text)
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
            throw refuse(.badNumber, "is not a whole number written as digits (\(text))")
        // `-0` is no whole number, and a number below zero no spelling of an
        // unsigned one, as for every narrower width.
        case .number(let text) where text == "-0" || (!signed && text.hasPrefix("-")):
            throw refuse(.badNumber, "is not a whole number of its type (\(text))")
        case .number(let text):
            guard let whole = Int128(text) else {
                throw refuse(.badNumber, "is not a whole number (\(text))")
            }
            if whole.magnitude <= exactInADouble {
                return text
            }
            throw refuse(
                .badNumber,
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
            throw pastWidth(text)
        }
        return whole
    }

    /// A 64-bit unsigned value: the canonical text of its digits, or a JSON number
    /// no larger than 2^53 - 1.
    public func unsigned64() throws -> UInt64 {
        let text = try wide(signed: false)
        guard let whole = UInt64(text) else {
            throw pastWidth(text)
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
            throw refuse(.badNumber, "is not a number (\(text))")
        }
        if abs(wide) > Double(Float.greatestFiniteMagnitude) {
            throw pastWidth(text)
        }
        return Float(wide)
    }

    /// A 64-bit fractional value. A number past the largest double (`1e999`) does
    /// not fit it, as one past the largest single does not fit a `float32`: no
    /// runtime holds the infinity a reader would round it to.
    public func float64() throws -> Double {
        let text = try number()
        guard let real = Double(text) else {
            throw refuse(.badNumber, "is not a number (\(text))")
        }
        if !real.isFinite {
            throw pastWidth(text)
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
    ///
    /// A text holding U+0000 is refused: a runtime that holds a text NUL-terminated
    /// would end it there, so no runtime holds one, and every reader refuses the same
    /// texts (docs/adr/0087, section 2.3). A byte string may hold the byte zero
    /// (`bytes(bound:)`).
    public func name() throws -> String {
        guard case .text(let text) = value else {
            throw wrongKind("is not a text")
        }
        if text.utf8.contains(0) {
            throw refuse(.nulInText, "carries U+0000, which no text holds")
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
                throw refuse(.notByteText, "carries a character above U+00FF, which no single byte spells")
            }
            out.append(UInt8(scalar.value))
        }
        if out.count > bound {
            throw pastBytes(out.count, bound)
        }
        return out
    }

    private func pastBytes(_ held: Int, _ bound: Int) -> PayloadRefusal {
        refuse(.tooLong, "is \(held) bytes, past the \(bound) its type declares", bound: bound)
    }

    /// A value of `optional<T>` (docs/adr/0055): absent where the value is `null`,
    /// and otherwise what `read` reads of it, held to `T`.
    public func optional<T>(_ read: (PayloadValue) throws -> T) throws -> T? {
        if isNull {
            return nil
        }
        return try read(self)
    }

    /// A value of `list<T>[least..bound]`: an array of at least `least` and at most
    /// `bound` elements, each read by `read` at the path of its index, in order.
    /// `least` is 0 for a list that states no lower count (docs/adr/0086, section 15).
    ///
    /// Both counts are held before any element is read: an array longer than `bound`
    /// is refused as `tooMany`, one shorter than `least` as `tooFew` naming `least`,
    /// and no element of either is decoded (ADR 0077, section 2).
    public func list<T>(least: Int = 0, bound: Int, _ read: (PayloadValue) throws -> T) throws -> [T] {
        guard case .array(let items) = value else {
            throw wrongKind("is not a list")
        }
        if items.count > bound {
            throw refuse(.tooMany, "holds \(items.count) elements, past the \(bound) its type declares", bound: bound)
        }
        if items.count < least {
            throw refuse(.tooFew, countShort(items.count, least), bound: least)
        }
        var out: [T] = []
        out.reserveCapacity(items.count)
        for (index, item) in items.enumerated() {
            let token = String(index)
            let element = PayloadValue(value: item, path: joinPath(path, token), pointer: pointerBelow(pointer, token))
            out.append(try read(element))
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
        var fields = PayloadFields(fields: members, prefix: path, pointer: pointer)
        if let defaults {
            fields = try fields.withDefaults(defaults)
        }
        return try read(fields)
    }

    /// A value of a union type (docs/adr/0092, section 4): one object holding its tag
    /// under the key `tag` and the fields of the case the tag names beside it.
    ///
    /// The tag is read first, by `readTag`, the reader of the tag's enum, wherever it
    /// stands in the object; `readCase` is then handed the case the tag named and this
    /// same value, and reads the case's record from it with `record(defaults:_:)` (an
    /// empty case reads nothing). The refusals come in one order on every runtime: a
    /// value that is not an object; the tag absent (`missing` at the tag's pointer),
    /// then whatever the enum's reader refuses of it (`null`, not a text, a name the
    /// enum does not declare); then the case's record. A key the case does not declare
    /// is never asked for, so it is stepped over, as a record steps over one.
    public func union<E, T>(
        tag: String,
        _ readTag: (PayloadValue) throws -> E,
        _ readCase: (E, PayloadValue) throws -> T
    ) throws -> T {
        guard case .object(let members) = value else {
            throw wrongKind("is not a union")
        }
        let object = PayloadFields(fields: members, prefix: path, pointer: pointer)
        let chosen = try readTag(try object.field(tag))
        return try readCase(chosen, self)
    }
}

/// `segment` below `prefix`, as a JSON Pointer's reference tokens are joined.
private func joinPath(_ prefix: String, _ segment: String) -> String {
    prefix.isEmpty ? segment : prefix + "/" + segment
}

/// Whether `text` spells a whole number: digits, after a `-` or not.
private func spellsWhole(_ text: String) -> Bool {
    let digits = text.hasPrefix("-") ? text.utf8.dropFirst() : text.utf8[...]
    return !digits.isEmpty && digits.allSatisfy { $0 >= UInt8(ascii: "0") && $0 <= UInt8(ascii: "9") }
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

/// Why a list of `held` elements is no value of a type whose lower count is `least`, in
/// the words the reader and the check both use (docs/adr/0086, section 15).
func countShort(_ held: Int, _ least: Int) -> String {
    "holds \(held) elements, fewer than the \(least) its type declares"
}

/// Holds a list a host built to its lower count and its bound, and each element to its
/// own type through `each`, at the path of its index (see `checkText`). A list is built
/// from empty an element at a time, so its lower count is held where it is written
/// (docs/adr/0086, section 15.6); `least` is 0 for a list that states none.
public func checkList<T>(
    _ path: String, _ items: [T], least: Int = 0, bound: Int, _ each: (String, T) throws -> Void
) throws {
    if items.count > bound {
        throw BoundExceeded("'\(path)' holds \(items.count) elements, past the \(bound) its type declares")
    }
    if items.count < least {
        throw BoundExceeded("'\(path)' \(countShort(items.count, least))")
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

    /// A composed value (docs/adr/0086): a list, a record, or an optional one of them,
    /// as the JSON its typed writers wrote it (`listJSON`, `recordJSON`, `optionalJSON`),
    /// whose keys they order by their bytes at every depth. A `<send>` that names the
    /// place holding one carries it as it is (section 5.3).
    case composed(String)

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
        case .composed(let written): return written
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
        // A list or a record is the JSON it was written as, as every other engine puts one
        // in a form (`ToWireString` in Go writes a list and an object as their JSON).
        case .composed(let written): return written
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

/// The JSON spelling of one union value (docs/adr/0092, section 2.1): the tag under the
/// key `tag`, its value `variant`, the case's name as its enum spells it, beside
/// `members`, the fields of the case's record (none for an empty case). The object
/// writes the tag among the fields, every key ascending by its UTF-8 bytes, as
/// `recordJSON` writes a record.
public func unionJSON(tag: String, variant: String, _ members: [PayloadPair]) -> String {
    recordJSON(members + [.text(tag, variant)])
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
