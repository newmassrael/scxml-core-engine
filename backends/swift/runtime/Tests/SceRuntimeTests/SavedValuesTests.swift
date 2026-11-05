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
