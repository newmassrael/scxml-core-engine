// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// The JSON Canonicalization Scheme (RFC 8785) of `writeCanonicalJSON`, which writes the test log of the
// test profile (docs/SCE_TEST_PROFILE.md §12.5). Each case is the twin of the Rust case of the same name
// in backends/rust/runtime/src/json.rs (`canonical_members_are_sorted_by_their_utf16_code_units` is
// `canonicalMembersAreSortedByTheirUtf16CodeUnits`) and of the Kotlin one in JsonCanonicalSchemeTest.kt,
// with the same value and the same bytes, so the writers are held to one text. Every text is compared
// by its UTF-8 bytes: Swift's `==` on a `String` holds two canonically equivalent texts equal, which
// two logs that agree under the scheme are not.

/// An object of `members`, in the order given.
private func object(_ members: (String, JSONValue)...) -> JSONValue {
    .object(members.map { (key: $0.0, value: $0.1) })
}

/// The bytes `writeCanonicalJSON` writes of `value`.
private func written(_ value: JSONValue) throws -> [UInt8] {
    Array(try writeCanonicalJSON(value).utf8)
}

/// The bytes of `text`.
private func bytes(_ text: String) -> [UInt8] {
    Array(text.utf8)
}

/// RFC 8785 section 3.2.3, the example of the RFC: members are sorted by their names as UTF-16 code
/// units, so U+1F600, whose first unit is 0xD83D, comes before U+FB33, which an order of code points
/// would put after it; U+0080 is written as it is, since only U+0000 to U+001F are escaped.
@Test func canonicalMembersAreSortedByTheirUtf16CodeUnits() throws {
    let value = object(
        ("\u{20ac}", .text("Euro Sign")),
        ("\r", .text("Carriage Return")),
        ("\u{fb33}", .text("Hebrew Letter Dalet With Dagesh")),
        ("1", .text("One")),
        ("\u{1f600}", .text("Emoji: Grinning Face")),
        ("\u{80}", .text("Control")),
        ("\u{f6}", .text("Latin Small Letter O With Diaeresis"))
    )
    let expected =
        "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u{80}\":\"Control\","
        + "\"\u{f6}\":\"Latin Small Letter O With Diaeresis\",\"\u{20ac}\":\"Euro Sign\","
        + "\"\u{1f600}\":\"Emoji: Grinning Face\","
        + "\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}"
    #expect(try written(value) == bytes(expected))
}

/// Two names Swift's `String` holds equal, U+00E9 and `e` followed by U+0301, are two names under the
/// scheme, sorted by their code units, the decomposed one first since `e` is 0x65; neither is a member
/// named twice.
@Test func canonicallyEquivalentNamesAreTwoMembers() throws {
    let value = object(("\u{e9}", .text("precomposed")), ("e\u{301}", .text("decomposed")))
    #expect(try written(value) == bytes("{\"e\u{301}\":\"decomposed\",\"\u{e9}\":\"precomposed\"}"))
}

/// RFC 8785 section 3.2.2.2: a text is escaped as `JSON.stringify` escapes it, the five short forms,
/// `\u00xx` in lower-case hex for the other controls, and nothing else: not `/`, not U+007F, not a
/// character past U+FFFF.
@Test func aCanonicalTextEscapesOnlyWhatJsonStringifyEscapes() throws {
    let value = JSONValue.text("\u{8}\t\n\u{c}\r\"\\\u{0}\u{1f}\u{7f}/\u{1f600}")
    #expect(try written(value) == bytes("\"\\b\\t\\n\\f\\r\\\"\\\\\\u0000\\u001f\u{7f}/\u{1f600}\""))
}

/// Nested arrays and objects are written with no whitespace, each object's members sorted at its own
/// level, an array's items in their order.
@Test func aCanonicalValueHasNoWhitespaceAndSortsEveryLevel() throws {
    let value = object(
        ("b", .array([object(("z", .null), ("a", .bool(false))), .text("y"), .array([])])),
        ("a", object())
    )
    #expect(try written(value) == bytes("{\"a\":{},\"b\":[{\"a\":false,\"z\":null},\"y\",[]]}"))
}

/// RFC 8785 section 3.2.2.3: a number is written as ECMAScript writes the double it reads as; one past
/// the range of a double is refused.
@Test func aCanonicalNumberIsTheDoubleItReadsAs() throws {
    let cases: [(String, String)] = [
        ("1E3", "1000"),
        ("-0", "0"),
        ("0.10", "0.1"),
        ("1e21", "1e+21"),
        ("1e-7", "1e-7"),
    ]
    for (spelling, expected) in cases {
        #expect(try written(.number(spelling)) == bytes(expected), "\(spelling)")
    }
    let refused = #expect(throws: NotCanonical.self, "past the range of a double") {
        _ = try writeCanonicalJSON(.number("1e400"))
    }
    #expect(refused?.reason.contains("1e400") == true, "\(String(describing: refused))")
}

/// What is no JSON number has no form in the scheme, although `Double`'s own reader takes it: a hex
/// float, a word, and a number with whitespace around it.
@Test func aNumberInNoJsonSpellingIsRefused() {
    for spelling in ["0x10", "nan", " 1", "1 ", "+1", "01", ""] {
        let refused = #expect(throws: NotCanonical.self, "'\(spelling)' is no JSON number") {
            _ = try writeCanonicalJSON(.array([.number(spelling)]))
        }
        #expect(refused?.reason == "'\(spelling)' is not a number", "\(String(describing: refused))")
    }
}

/// RFC 8785 section 3.1: an object that names one member twice has no single meaning, at whatever
/// depth.
@Test func aCanonicalObjectNamesEachMemberOnce() {
    let twice = JSONValue.array([object(("a", .text("1")), ("b", .text("2")), ("a", .text("3")))])
    let refused = #expect(throws: NotCanonical.self, "a member twice") {
        _ = try writeCanonicalJSON(twice)
    }
    #expect(refused?.reason.contains("\"a\"") == true, "\(String(describing: refused))")
}
