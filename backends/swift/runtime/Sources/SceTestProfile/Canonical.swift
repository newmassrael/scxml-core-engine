// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The canonical form of a message's value in the test log (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091,
// C3 and answer 8): one JSON value per value of the profile's types, built from texts, truth values,
// arrays and objects alone, so that the JSON Canonicalization Scheme (`writeCanonicalJSON`) writes two
// equal values as the same bytes on every runtime.
//
// Cross-language siblings: `sce_rust_runtime::test::canonical` (Rust), `SCE::Test::Canonical` (C++),
// `com.sce.runtime.test.Canonical` (Kotlin).

import SceRuntime

/// The writers of the canonical form of a value of the profile's types, each a `JSONValue` that holds
/// no number and no `null`:
///
/// - an integer of any width is the text of its decimal digits, since an `int64` or a `uint64` is past
///   what a JSON reader that holds numbers as doubles reads exactly;
/// - a `bool` is a truth value;
/// - a value of an enum is the name its document declares for its variant;
/// - a text is a text;
/// - a byte string is the text of its octets in upper-case hex, two digits each;
/// - a record is an object of its fields by their declared ids, a field that is absent omitted, as
///   TTCN-3's `omit` leaves it out;
/// - a list is an array of its elements in order;
/// - a value of a union is an object of one member, named by its case's variant, whose value is the
///   case's record, or the empty object for a case that holds none.
///
/// The generated unit of a test template over a record spells its `canonical` with these, by the types
/// its document declares (`forge/swift/test_template.swift.jinja2`): the form is the profile type's, not
/// the language's, since a byte string and a list of `uint8` are one Swift type, `[UInt8]`, and take two
/// forms.
public enum Canonical {
    /// An integer of any width: the text of its decimal digits.
    public static func integer(_ value: some BinaryInteger) -> JSONValue {
        .text(String(value))
    }

    /// A `bool`.
    public static func boolean(_ value: Bool) -> JSONValue {
        .bool(value)
    }

    /// A text.
    public static func text(_ value: String) -> JSONValue {
        .text(value)
    }

    /// A byte string: its octets in upper-case hex, two digits each, as TTCN-3 writes an `octetstring`
    /// (`'00FF'O`) without its quotes and its `O`.
    public static func bytes(_ value: [UInt8]) -> JSONValue {
        let hex = Array("0123456789ABCDEF".utf8)
        var out: [UInt8] = []
        out.reserveCapacity(value.count * 2)
        for octet in value {
            out.append(hex[Int(octet >> 4)])
            out.append(hex[Int(octet & 0x0F)])
        }
        return .text(String(decoding: out, as: UTF8.self))
    }

    /// A value of a closed enum: the name its document declares for the variant.
    public static func enumerated(_ declared: String) -> JSONValue {
        .text(declared)
    }

    /// A value of an enum of an open set (`sce:strict-variants="false"`): the name its document declares
    /// for it, or, for a number no variant declares, the text of that number's decimal digits. No
    /// variant's name is such a text, since a name is an identifier. The TTCN-3 export refuses an enum of
    /// an open set (docs/SCE_TEST_PROFILE.md §9.5), so only a native log holds this form.
    public static func openEnumerated(_ declared: String?, _ number: some BinaryInteger) -> JSONValue {
        if let declared {
            return .text(declared)
        }
        return integer(number)
    }

    /// A record: each field by its declared id, in any order, since the scheme sorts an object's
    /// members; a field given as nil is absent and omitted.
    public static func record(_ fields: (String, JSONValue?)...) -> JSONValue {
        var members: [(key: String, value: JSONValue)] = []
        for field in fields {
            if let value = field.1 {
                members.append((key: field.0, value: value))
            }
        }
        return .object(members)
    }

    /// A list: its elements in order.
    public static func list(_ elements: [JSONValue]) -> JSONValue {
        .array(elements)
    }

    /// A value of a union: one member, named by the variant of its case, holding the case's record, or
    /// the empty object for a case that holds none.
    public static func unionCase(_ variant: String, _ record: JSONValue?) -> JSONValue {
        .object([(key: variant, value: record ?? .object([]))])
    }
}
