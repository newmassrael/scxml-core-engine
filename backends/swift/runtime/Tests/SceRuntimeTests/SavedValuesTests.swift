// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// How a variable is written into a saved state and read back: as its `sce:type` says, and only if it is one.

private func refusal<T>(_ read: () throws -> T) -> String? {
    do {
        _ = try read()
        return nil
    } catch let refusal as StateRefusal {
        return refusal.message
    } catch {
        return "\(error)"
    }
}

@Test func aNumberUpToThirtyTwoBitsIsAJSONNumberAndASixtyFourBitOneIsAText() {
    #expect(writeJSON(SavedValues.of(UInt8(255))) == "255")
    #expect(writeJSON(SavedValues.of(UInt16(65535))) == "65535")
    #expect(writeJSON(SavedValues.of(UInt32.max)) == "4294967295")
    #expect(writeJSON(SavedValues.of(Int8.min)) == "-128")
    #expect(writeJSON(SavedValues.of(Int16.min)) == "-32768")
    #expect(writeJSON(SavedValues.of(Int32.min)) == "-2147483648")
    // A reader that holds JSON numbers as doubles would lose these low bits.
    #expect(writeJSON(SavedValues.of(UInt64.max)) == "\"18446744073709551615\"")
    #expect(writeJSON(SavedValues.of(Int64.min)) == "\"-9223372036854775808\"")
}

@Test func aValueIsReadBackAsTheTypeItWasWrittenAs() throws {
    #expect(try SavedValues.uint8(SavedValues.of(UInt8(7)), "v") == 7)
    #expect(try SavedValues.uint16(SavedValues.of(UInt16.max), "v") == UInt16.max)
    #expect(try SavedValues.uint32(SavedValues.of(UInt32.max), "v") == UInt32.max)
    #expect(try SavedValues.int8(SavedValues.of(Int8.min), "v") == Int8.min)
    #expect(try SavedValues.int16(SavedValues.of(Int16.min), "v") == Int16.min)
    #expect(try SavedValues.int32(SavedValues.of(Int32.min), "v") == Int32.min)
    #expect(try SavedValues.uint64(SavedValues.of(UInt64.max), "v") == UInt64.max)
    #expect(try SavedValues.int64(SavedValues.of(Int64.min), "v") == Int64.min)
    #expect(try SavedValues.bool(SavedValues.of(true), "v") == true)
    #expect(try SavedValues.string(SavedValues.of("héllo"), "v") == "héllo")
}

@Test func aRealIsWrittenAsANumberAndANonFiniteOneAsATextEveryBackendReads() throws {
    #expect(writeJSON(SavedValues.of(1.5)) == "1.5")
    #expect(writeJSON(SavedValues.of(Double.nan)) == "\"NaN\"")
    #expect(writeJSON(SavedValues.of(Double.infinity)) == "\"Infinity\"")
    #expect(writeJSON(SavedValues.of(-Double.infinity)) == "\"-Infinity\"")
    #expect(try SavedValues.float64(SavedValues.of(0.1), "v") == 0.1)
    #expect(try SavedValues.float64(SavedValues.of(Double.nan), "v").isNaN)
    #expect(try SavedValues.float64(SavedValues.of(Double.infinity), "v") == .infinity)
    #expect(try SavedValues.float64(SavedValues.of(-Double.infinity), "v") == -.infinity)
    // A 32-bit real is written as the 64-bit real it widens to, which is exact.
    let single: Float = 0.3
    #expect(writeJSON(SavedValues.of(single)) == "\(Double(single))")
    #expect(try SavedValues.float32(SavedValues.of(single), "v") == single)
    // A number a real does not hold is refused for what it is, and a whole number is a real too.
    #expect(try SavedValues.float64(.number("3"), "v") == 3)
}

@Test func aValueOfAnotherKindOrOneTheTypeCannotHoldIsRefused() {
    #expect(refusal { try SavedValues.uint8(.number("256"), "count") } == "'count' (256) is not a whole number its type can hold")
    #expect(refusal { try SavedValues.uint8(.number("-1"), "count") } == "'count' (-1) is not a whole number its type can hold")
    #expect(refusal { try SavedValues.int32(.number("1.5"), "count") } == "'count' (1.5) is not a whole number its type can hold")
    #expect(refusal { try SavedValues.uint32(.text("7"), "count") } == "'count' is not a number")
    #expect(refusal { try SavedValues.uint64(.number("7"), "wide") } == "'wide' is not a 64-bit whole number written as a text")
    #expect(refusal { try SavedValues.uint64(.text("18446744073709551616"), "wide") } == "'wide' (18446744073709551616) is not a whole number its type can hold")
    #expect(refusal { try SavedValues.int64(.text("x"), "wide") } == "'wide' (x) is not a whole number its type can hold")
    #expect(refusal { try SavedValues.bool(.number("1"), "ready") } == "'ready' is not a truth value")
    #expect(refusal { try SavedValues.string(.number("1"), "name") } == "'name' is not a text")
    #expect(refusal { try SavedValues.float64(.text("soon"), "level") } == "'level' is not a number")
}

@Test func aNullIsNamedAsAnAbsentValueWhereTheTypeIsNotOptional() {
    #expect(refusal { try SavedValues.uint32(.null, "count") } == "'count' is absent (null), and its type is not optional")
    #expect(refusal { try SavedValues.string(.null, "name") } == "'name' is absent (null), and its type is not optional")
    #expect(refusal { try SavedValues.bool(.null, "ready") } == "'ready' is absent (null), and its type is not optional")
}

@Test func aTextIsReadBackOnlyWithinTheUTF8BytesItIsBoundedBy() throws {
    #expect(try SavedValues.string(.text("abc"), "name", capacity: 3) == "abc")
    #expect(refusal { try SavedValues.string(.text("abcd"), "name", capacity: 3) } == "'name' holds 4 UTF-8 bytes, past the 3 it is bounded by")
    // The bound counts bytes and not characters: two characters, four bytes.
    #expect(refusal { try SavedValues.string(.text("éé"), "name", capacity: 3) } == "'name' holds 4 UTF-8 bytes, past the 3 it is bounded by")
    #expect(try SavedValues.string(.text("éé"), "name", capacity: 4) == "éé")
}

private enum Shade: UInt8, SavedEnumeration {
    case light = 1
    case dark = 2

    static var savedAlias: String { "Shade" }

    var declaredName: String { self == .light ? "light" : "dark" }

    init?(declaredName: String) {
        switch declaredName {
        case "light": self = .light
        case "dark": self = .dark
        default: return nil
        }
    }
}

@Test func anEnumIsTheNameItsDocumentDeclaresAndNothingElse() throws {
    #expect(writeJSON(SavedValues.of(Shade.dark)) == "\"dark\"")
    #expect(try SavedValues.enumeration(.text("light"), "tone", as: Shade.self) == .light)
    #expect(
        refusal { try SavedValues.enumeration(.text("grey"), "tone", as: Shade.self) }
            == "'tone' (grey) is not a variant of Shade")
    // A number is the underlying value, which a state does not hold: a build that renumbered would read it wrong.
    #expect(
        refusal { try SavedValues.enumeration(.number("2"), "tone", as: Shade.self) } == "'tone' is not a text")
}

@Test func aByteStringIsItsLatin1TextAndAnyCharacterPastItIsNoByte() throws {
    #expect(writeJSON(SavedValues.of([UInt8]([0x41, 0xE9, 0xFF]))) == "\"A\u{E9}\u{FF}\"")
    #expect(try SavedValues.bytes(.text("A\u{E9}\u{FF}"), "blob") == [0x41, 0xE9, 0xFF])
    #expect(try SavedValues.bytes(SavedValues.of([UInt8]([])), "blob") == [])
    #expect(
        refusal { try SavedValues.bytes(.text("A\u{100}"), "blob") }
            == "'blob' holds U+0100, which is past U+00FF and so no byte")
    #expect(
        refusal { try SavedValues.bytes(.text("abcd"), "blob", capacity: 3) }
            == "'blob' holds 4 bytes, past the 3 it is bounded by")
    #expect(try SavedValues.bytes(.text("abc"), "blob", capacity: 3) == [0x61, 0x62, 0x63])
}

@Test func aValueThatMayBeAbsentIsNullOrItsPresentValue() throws {
    #expect(writeJSON(SavedValues.optional(UInt32?.none) { SavedValues.of($0) }) == "null")
    #expect(writeJSON(SavedValues.optional(UInt32?.some(7)) { SavedValues.of($0) }) == "7")
    #expect(try SavedValues.optional(.null, "due") { try SavedValues.uint32($0, $1) } == nil)
    #expect(try SavedValues.optional(.number("7"), "due") { try SavedValues.uint32($0, $1) } == 7)
    // The present value is held to its type as anywhere: what is wrong with it names what holds it.
    #expect(
        refusal { try SavedValues.optional(.text("x"), "due") { try SavedValues.uint32($0, $1) } }
            == "'due' is not a number")
}

@Test func aListIsReadBackElementByElementWithinItsBound() throws {
    #expect(writeJSON(SavedValues.list([4 as UInt8, 2]) { SavedValues.of($0) }) == "[4,2]")
    #expect(
        try SavedValues.list(.array([.number("4"), .number("2")]), "picked", capacity: 2) {
            try SavedValues.uint8($0, $1)
        } == [4, 2])
    #expect(
        refusal {
            try SavedValues.list(.array([.number("1"), .number("2"), .number("3")]), "picked", capacity: 2) {
                try SavedValues.uint8($0, $1)
            }
        } == "'picked' holds 3 elements, past the 2 it is bounded by")
    #expect(
        refusal {
            try SavedValues.list(.array([.number("1"), .text("x")]), "picked", capacity: 2) {
                try SavedValues.uint8($0, $1)
            }
        } == "'picked[1]' is not a number")
    #expect(
        refusal { try SavedValues.list(.number("1"), "picked", capacity: 2) { try SavedValues.uint8($0, $1) } }
            == "'picked' is not an array")
}

@Test func aRecordFieldIsFoundByItsIdAndAMissingOneIsRefusedByName() throws {
    let record = SavedValues.record([(key: "id", value: .number("2")), (key: "title", value: .text("beta"))])
    #expect(writeJSON(record) == "{\"id\":2,\"title\":\"beta\"}")
    #expect(try SavedValues.field(record, "item", "title") == .text("beta"))
    #expect(refusal { try SavedValues.field(record, "item", "count") } == "'item' has no field 'count'")
    #expect(refusal { try SavedValues.field(.number("1"), "item", "id") } == "'item' is not an object")
}

// A union is one object, its tag under the tag's key as the declared name of its case, before the fields of the
// case's record (docs/adr/0092, section 6.1 item 7), read back by its tag first.
@Test func aUnionIsItsTagBesideTheFieldsOfItsCaseAndIsReadBackByItsTag() throws {
    let circle = SavedValues.union(
        tag: "kind", variant: "circle", SavedValues.record([(key: "radius", value: .number("7"))]))
    #expect(writeJSON(circle) == "{\"kind\":\"circle\",\"radius\":7}")
    #expect(writeJSON(SavedValues.union(tag: "kind", variant: "dot", nil)) == "{\"kind\":\"dot\"}")
    #expect(try SavedValues.unionTag(circle, "shape", tag: "kind") == "circle")
    // The tag is read wherever it stands, and the fields beside it are the case's record's to read.
    let tagLast = JSONValue.object([(key: "radius", value: .number("7")), (key: "kind", value: .text("disc"))])
    #expect(try SavedValues.unionTag(tagLast, "shape", tag: "kind") == "disc")
    #expect(try SavedValues.uint16(SavedValues.field(tagLast, "shape", "radius"), "shape.radius") == 7)
    #expect(refusal { try SavedValues.unionTag(.number("1"), "shape", tag: "kind") } == "'shape' is not an object")
    #expect(
        refusal { try SavedValues.unionTag(.object([(key: "radius", value: .number("7"))]), "shape", tag: "kind") }
            == "'shape' has no field 'kind'")
    #expect(
        refusal { try SavedValues.unionTag(.object([(key: "kind", value: .number("3"))]), "shape", tag: "kind") }
            == "'shape.kind' is not a text")
}
