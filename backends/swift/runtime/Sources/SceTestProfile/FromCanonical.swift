// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A value read back from the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5;
// docs/SCE_TTCN3_LANE.md §10.3): the inverse of `Canonical`, which a recorded input trace's values are
// read with (`schemas/sce-test-trace.v1.schema.json`).
//
// Cross-language siblings: `sce_rust_runtime::test::from_canonical` (Rust), whose refusals these name
// in the same words, `com.sce.runtime.test.FromCanonical` (Kotlin), `sce/include/test/FromCanonical.h`
// (C++).

import SceRuntime

/// The readers of the canonical form of a value of the profile's types, each value read from the one
/// JSON value model of the runtime (`JSONValue`), whose reader keeps a number's spelling and refuses an
/// object that names a member twice.
///
/// Each reader takes one form and accepts its one spelling, so a value read here and written back by
/// `Canonical` is the same bytes:
///
/// - an integer is the text of its decimal digits, with no sign but a minus, no leading zero and no
///   `-0`, within the range of the type it is read as;
/// - a `bool` is a truth value;
/// - a value of an enum is the name its document declares for the variant, or, for an enum of an open
///   set, the text of a number no variant declares;
/// - a text is a text of at most its bound in UTF-8 bytes;
/// - a byte string is its octets in upper-case hex, two digits each;
/// - a record is an object of its fields by their declared ids, an absent field omitted, no field it
///   does not declare and none twice;
/// - a list is an array of its elements within its bounds, an absent element of a list of
///   `optional<T>` `null`;
/// - a value of a union is an object of one member, named by its case's variant, whose value is the
///   case's record, or `{}` for a case that holds none.
///
/// A value that does not belong to its type is refused with a `TraceRefusal`, naming where it stands
/// (`what`) and why, never converted. Which form a value takes is decided by the types its document
/// declares and not by its Swift type, since a byte string and a list of `uint8` are one Swift type,
/// `[UInt8]`: the generated reader of a record (`read_<record>` in the unit of a test configuration)
/// calls these by the walk its canonical writer is spelled by
/// (`forge::generator::test_template_walk::canonical_form`).
///
/// A name is compared by its UTF-8 bytes, never by Swift's `==` on a `String`, which holds two
/// canonically equivalent texts equal: two spellings of one name are two names here, as they are to
/// every other runtime's reader.
public enum FromCanonical {
    /// The refusal of the value at `what`, for `why`.
    private static func refused(_ what: String, _ why: String) -> TraceRefusal {
        TraceRefusal("\(what): \(why)")
    }

    /// The kind of a JSON value, as a refusal names it.
    private static func kind(_ value: JSONValue) -> String {
        switch value {
        case .null: return "null"
        case .bool: return "a truth value"
        case .number: return "a number"
        case .text: return "a text"
        case .array: return "an array"
        case .object: return "an object"
        }
    }

    /// The refusal of `value`, at `what`, which is not of the form `form`.
    private static func notA(_ value: JSONValue, _ what: String, _ form: String) -> TraceRefusal {
        refused(what, "\(kind(value)) is not \(form)")
    }

    /// Whether `a` and `b` are the same text, byte for byte.
    static func same(_ a: String, _ b: String) -> Bool {
        a.utf8.elementsEqual(b.utf8)
    }

    /// The digits of the greatest 128-bit integer.
    private static let greatest128 = Array("170141183460469231731687303715884105727".utf8)

    /// The digits of the magnitude of the least 128-bit integer, one more than the greatest's.
    private static let least128 = Array("170141183460469231731687303715884105728".utf8)

    /// `digits` where it spells an integer as the canonical form writes one: an optional minus and
    /// decimal digits with no leading zero, never `-0`, within the range of a 128-bit integer, which the
    /// Rust reader of the same text holds it to; nil for any other text.
    static func decimal(_ digits: String) -> String? {
        let bytes = Array(digits.utf8)
        let negative = bytes.first == UInt8(ascii: "-")
        let magnitude = negative ? Array(bytes.dropFirst()) : bytes
        let isDigit = { (byte: UInt8) in byte >= UInt8(ascii: "0") && byte <= UInt8(ascii: "9") }
        guard !magnitude.isEmpty, magnitude.allSatisfy(isDigit) else {
            return nil
        }
        let zero = magnitude == [UInt8(ascii: "0")]
        guard zero || magnitude[0] != UInt8(ascii: "0"), !(negative && zero) else {
            return nil
        }
        // Two texts of digits of one length, neither with a leading zero, are in the order of their
        // values.
        let bound = negative ? least128 : greatest128
        let within =
            magnitude.count < bound.count
            || (magnitude.count == bound.count && !bound.lexicographicallyPrecedes(magnitude))
        return within ? digits : nil
    }

    /// An integer of the type `type`: the text of its decimal digits.
    public static func integer<T: FixedWidthInteger>(_ value: JSONValue, _ type: T.Type, _ what: String)
        throws(TraceRefusal) -> T
    {
        guard case .text(let text) = value else {
            throw notA(value, what, "an integer, which is the text of its decimal digits")
        }
        guard let digits = decimal(text) else {
            throw refused(what, "'\(text)' is not the text of an integer's decimal digits")
        }
        guard let number = T(digits) else {
            throw refused(what, "\(digits) is past the range of its type")
        }
        return number
    }

    /// A `bool`.
    public static func boolean(_ value: JSONValue, _ what: String) throws(TraceRefusal) -> Bool {
        guard case .bool(let held) = value else {
            throw notA(value, what, "a truth value")
        }
        return held
    }

    /// A text of at most `max` bytes of UTF-8, the bound its type declares.
    public static func text(_ value: JSONValue, _ max: Int, _ what: String) throws(TraceRefusal) -> String {
        guard case .text(let text) = value else {
            throw notA(value, what, "a text")
        }
        let bytes = text.utf8.count
        if bytes > max {
            throw refused(what, "a text of \(bytes) bytes is past the \(max) its type holds")
        }
        return text
    }

    /// The value of one upper-case hex digit, or nil.
    private static func hexDigit(_ digit: UInt8) -> UInt8? {
        switch digit {
        case UInt8(ascii: "0")...UInt8(ascii: "9"): return digit - UInt8(ascii: "0")
        case UInt8(ascii: "A")...UInt8(ascii: "F"): return digit - UInt8(ascii: "A") + 10
        default: return nil
        }
    }

    /// The octets `text` spells in upper-case hex, two digits each, or nil.
    private static func upperHex(_ text: String) -> [UInt8]? {
        let digits = Array(text.utf8)
        if digits.count % 2 != 0 {
            return nil
        }
        var octets: [UInt8] = []
        octets.reserveCapacity(digits.count / 2)
        var index = 0
        while index < digits.count {
            guard let high = hexDigit(digits[index]), let low = hexDigit(digits[index + 1]) else {
                return nil
            }
            octets.append(high << 4 | low)
            index += 2
        }
        return octets
    }

    /// A byte string of at most `max` octets: the text of its octets in upper-case hex, two digits
    /// each.
    public static func bytes(_ value: JSONValue, _ max: Int, _ what: String) throws(TraceRefusal) -> [UInt8] {
        guard case .text(let text) = value else {
            throw notA(value, what, "a byte string, which is a text of hex")
        }
        guard let octets = upperHex(text) else {
            throw refused(what, "'\(text)' is not octets in upper-case hex, two digits each")
        }
        if octets.count > max {
            throw refused(what, "a byte string of \(octets.count) octets is past the \(max) its type holds")
        }
        return octets
    }

    /// A value of an enum of a closed set: the variant whose declared name the text is, from
    /// `declared`, each variant's name and its value.
    public static func enumerated<E>(_ value: JSONValue, _ declared: [(String, E)], _ what: String)
        throws(TraceRefusal) -> E
    {
        guard case .text(let name) = value else {
            throw notA(value, what, "a value of an enum, which is its name")
        }
        guard let variant = declared.first(where: { same($0.0, name) }) else {
            throw refused(what, "'\(name)' names no variant of its enum")
        }
        return variant.1
    }

    /// A value of an enum of an open set (`sce:strict-variants="false"`): the variant whose declared
    /// name the text is, or the value `fromUnderlying` makes of the number the text spells, of the
    /// enum's `underlying` type, where no variant declares that number. A declared number is written by
    /// its variant's name, so its digits are refused as the second spelling of one value.
    public static func openEnumerated<E: Equatable, U: FixedWidthInteger>(
        _ value: JSONValue, _ declared: [(String, E)], _ underlying: U.Type, _ what: String,
        _ fromUnderlying: (U) -> E
    ) throws(TraceRefusal) -> E {
        guard case .text(let name) = value else {
            throw notA(value, what, "a value of an enum, which is its name")
        }
        if let variant = declared.first(where: { same($0.0, name) }) {
            return variant.1
        }
        guard let digits = decimal(name) else {
            throw refused(what, "'\(name)' names no variant of its enum, and is no number")
        }
        guard let raw = U(digits) else {
            throw refused(what, "\(name) is past the range of its enum")
        }
        let held = fromUnderlying(raw)
        if declared.contains(where: { $0.1 == held }) {
            throw refused(what, "\(name) is the number of a declared variant, which is written by its name")
        }
        return held
    }

    /// A record whose fields are `fields`: an object, each member of which names one of them once. The
    /// object is handed to `build`, whose fields `field` and `optionalField` read, and the record it
    /// builds is answered.
    public static func record<T>(
        _ value: JSONValue, _ fields: [String], _ what: String,
        _ build: (JSONValue) throws(TraceRefusal) -> T
    ) throws(TraceRefusal) -> T {
        guard case .object(let members) = value else {
            throw notA(value, what, "a record, which is an object of its fields")
        }
        for (index, member) in members.enumerated() {
            if !fields.contains(where: { same($0, member.key) }) {
                throw refused(what, "'\(member.key)' is no field of its record")
            }
            if members[..<index].contains(where: { same($0.key, member.key) }) {
                throw refused(what, "the field '\(member.key)' is given twice")
            }
        }
        return try build(value)
    }

    /// The field `id` of the record `record`, whose type is not optional, so it is present: its value,
    /// which may be JSON `null`, which the reader of its type refuses.
    public static func field(_ record: JSONValue, _ id: String, _ what: String) throws(TraceRefusal) -> JSONValue {
        guard let held = record.member(id) else {
            throw refused(what, "the field is absent, and its type is not optional")
        }
        return held
    }

    /// The field `id` of the record `record`, whose type is optional: nil where it is absent, which the
    /// canonical form writes by omitting it, and the value `read` reads where it is present. A member
    /// that is present and `null` is handed to `read`, whose reader refuses it, since an absent field is
    /// omitted and never written as `null`.
    public static func optionalField<T>(
        _ record: JSONValue, _ id: String, _ read: (JSONValue) throws(TraceRefusal) -> T
    ) throws(TraceRefusal) -> T? {
        guard let held = record.member(id) else {
            return nil
        }
        return try read(held)
    }

    /// The elements of a list of at least `min` and at most `max` elements, each read by `read`.
    public static func elements<T>(
        _ value: JSONValue, _ min: Int, _ max: Int, _ what: String,
        _ read: (JSONValue) throws(TraceRefusal) -> T
    ) throws(TraceRefusal) -> [T] {
        guard case .array(let items) = value else {
            throw notA(value, what, "a list, which is an array")
        }
        if items.count < min || items.count > max {
            throw refused(what, "a list of \(items.count) elements is outside its bounds, \(min) to \(max)")
        }
        var elements: [T] = []
        elements.reserveCapacity(items.count)
        for item in items {
            elements.append(try read(item))
        }
        return elements
    }

    /// An element of a list of `optional<T>`: nil for `null`, which the canonical form writes in the
    /// place of an absent element, and the value `read` reads otherwise.
    public static func nullable<T>(_ value: JSONValue, _ read: (JSONValue) throws(TraceRefusal) -> T)
        throws(TraceRefusal) -> T?
    {
        if case .null = value {
            return nil
        }
        return try read(value)
    }

    /// The case a value of a union is in: its `variant` and the `value` of its record.
    public struct UnionCase: Equatable, Sendable {
        /// The variant of the case, one the union declares.
        public let variant: String
        /// The case's value: its record's object, or the empty object.
        public let value: JSONValue
    }

    /// A value of a union whose cases are `variants`: an object of one member, named by one of them,
    /// the variant of its case and the case's value. A variant no case declares is refused here, so a
    /// reader that chooses by the variant read has a branch for each declared one and needs none for
    /// another.
    public static func unionCase(_ value: JSONValue, _ variants: [String], _ what: String)
        throws(TraceRefusal) -> UnionCase
    {
        guard case .object(let members) = value else {
            throw notA(value, what, "a value of a union, which is an object of one member")
        }
        guard members.count == 1, let only = members.first else {
            throw refused(
                what, "a value of a union is an object of one member, and this one has \(members.count)")
        }
        guard variants.contains(where: { same($0, only.key) }) else {
            throw refused(what, "'\(only.key)' names no case of its union")
        }
        return UnionCase(variant: only.key, value: only.value)
    }

    /// The value `held` of a case of a union that holds no record, read from `value`, which is the
    /// empty object.
    public static func noRecord<C>(_ value: JSONValue, _ what: String, _ held: C) throws(TraceRefusal) -> C {
        guard case .object(let members) = value, members.isEmpty else {
            throw refused(what, "a case that holds no record is the empty object")
        }
        return held
    }
}
