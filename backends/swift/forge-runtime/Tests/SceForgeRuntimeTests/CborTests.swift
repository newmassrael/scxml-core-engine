// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// The CBOR items of a codec map (SCE_FORGE.md §4.6.1), held against the cases of the Go runtime's own
// tests (`backends/go/forge-runtime/codec/cbor_test.go`): a head is written in its shortest form and
// read in any valid one, a string is judged against its bound before its bytes are looked for, and what
// a codec cannot read as its entry (an indefinite length, a reserved head, another major type) is
// refused and not guessed at.

private func written(_ body: (inout [UInt8]) -> Void) -> [UInt8] {
    var buffer: [UInt8] = []
    body(&buffer)
    return buffer
}

@Test func aHeadIsWrittenInItsShortestForm() {
    #expect(written { Cbor.writeUint(0, to: &$0) } == [0x00])
    #expect(written { Cbor.writeUint(23, to: &$0) } == [0x17])
    #expect(written { Cbor.writeUint(24, to: &$0) } == [0x18, 0x18])
    #expect(written { Cbor.writeUint(255, to: &$0) } == [0x18, 0xFF])
    #expect(written { Cbor.writeUint(256, to: &$0) } == [0x19, 0x01, 0x00])
    #expect(written { Cbor.writeUint(65_535, to: &$0) } == [0x19, 0xFF, 0xFF])
    #expect(written { Cbor.writeUint(65_536, to: &$0) } == [0x1A, 0x00, 0x01, 0x00, 0x00])
    #expect(written { Cbor.writeUint(0xFFFF_FFFF, to: &$0) } == [0x1A, 0xFF, 0xFF, 0xFF, 0xFF])
    #expect(
        written { Cbor.writeUint(0x1_0000_0000, to: &$0) }
            == [0x1B, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00])
    #expect(
        written { Cbor.writeUint(UInt64.max, to: &$0) }
            == [0x1B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
}

@Test func aBooleanTextBytesAndMapHeadAreWrittenAsTheirMajorTypesSay() {
    #expect(written { Cbor.writeBool(false, to: &$0) } == [0xF4])
    #expect(written { Cbor.writeBool(true, to: &$0) } == [0xF5])
    #expect(written { Cbor.writeText("a", to: &$0) } == [0x61, 0x61])
    #expect(written { Cbor.writeText("é", to: &$0) } == [0x62, 0xC3, 0xA9], "a length is in bytes")
    #expect(written { Cbor.writeBytes([1, 2], to: &$0) } == [0x42, 0x01, 0x02])
    #expect(written { Cbor.writeMapHead(3, to: &$0) } == [0xA3])
}

@Test func aHeadIsReadInAnyValidLengthAndItsValueIsWhatIsRead() throws {
    // 5 as a one-byte head, as a two-byte argument and as a four-byte argument.
    for bytes: [UInt8] in [[0x05], [0x18, 0x05], [0x19, 0x00, 0x05], [0x1A, 0x00, 0x00, 0x00, 0x05]] {
        var cursor = SceCursor(bytes)
        #expect(try Cbor.readUint(&cursor) == 5)
        #expect(cursor.remaining == 0)
    }
}

@Test func anIndefiniteLengthAndAReservedHeadAreRefused() {
    for initial: UInt8 in [0x1C, 0x1D, 0x1E, 0x1F, 0x5F, 0x7F, 0xBF] {
        var cursor = SceCursor([initial, 0x00])
        #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readHead(&cursor) }
    }
}

@Test func aHeadCutShortNeedsMoreBytes() {
    var cursor = SceCursor([0x19, 0x01])
    #expect(throws: CodecError.needMoreBytes) { _ = try Cbor.readUint(&cursor) }
}

@Test func anItemOfAnotherMajorTypeIsNotTheEntryTheCodecReads() {
    var text = SceCursor([0x61, 0x61])
    #expect(throws: CodecError.cborWrongType) { _ = try Cbor.readUint(&text) }
    var number = SceCursor([0x05])
    #expect(throws: CodecError.cborWrongType) { _ = try Cbor.readBool(&number) }
    // A simple value that is neither false nor true.
    var null = SceCursor([0xF6])
    #expect(throws: CodecError.cborWrongType) { _ = try Cbor.readBool(&null) }
}

@Test func anItemThePlaceDoesNotTakeIsOfTheWrongType() {
    // A negative integer where an unsigned stands, null, a bignum (a tagged item).
    for bytes: [UInt8] in [[0x20], [0xF6], [0xC2, 0x41, 0x01]] {
        var cursor = SceCursor(bytes)
        #expect(throws: CodecError.cborWrongType, "\(bytes)") { _ = try Cbor.readUint(&cursor) }
    }
    // A float whose bits are a truth value's number is no truth value.
    var half = SceCursor([0xF9, 0x00, 0x14])
    #expect(throws: CodecError.cborWrongType) { _ = try Cbor.readBool(&half) }
    // An unsigned key where a record's text key stands.
    var key = SceCursor([0x01])
    #expect(throws: CodecError.cborWrongType) { _ = try Cbor.readText(&key, maxSize: nil) }
}

@Test func anItemThatIsNotWellFormedOrNotInTheProfileIsMalformed() {
    // An indefinite text and byte string, in chunks, and a break code where an item stands; a simple
    // value in the two-byte form below 32, which is not well-formed (RFC 8949 §3.3).
    var text = SceCursor([0x7F, 0x61, 0x61, 0x61, 0x62, 0xFF])
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readText(&text, maxSize: nil) }
    var bytes = SceCursor([0x5F, 0x41, 0x00, 0xFF])
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readBytes(&bytes, maxSize: nil) }
    var stop = SceCursor([0xFF])
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readUint(&stop) }
    var simple = SceCursor([0xF8, 0x14])
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readBool(&simple) }
}

/// A signed integer at each boundary of a head's length, on both sides of zero: the argument of a
/// negative number is `-1 - n`, so -24 is the last of one byte and -25 the first of two.
private let signed: [(Int64, [UInt8])] = [
    (0, [0x00]),
    (23, [0x17]),
    (24, [0x18, 0x18]),
    (-1, [0x20]),
    (-24, [0x37]),
    (-25, [0x38, 0x18]),
    (-256, [0x38, 0xFF]),
    (-257, [0x39, 0x01, 0x00]),
    (-500, [0x39, 0x01, 0xF3]),
    (-65_536, [0x39, 0xFF, 0xFF]),
    (-65_537, [0x3A, 0x00, 0x01, 0x00, 0x00]),
    (-0x1_0000_0000, [0x3A, 0xFF, 0xFF, 0xFF, 0xFF]),
    (-0x1_0000_0001, [0x3B, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]),
    (Int64.min, [0x3B, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
    (Int64.max, [0x1B, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
    (0xFFFF_FFFF, [0x1A, 0xFF, 0xFF, 0xFF, 0xFF]),
]

@Test func aSignedIntegerIsWrittenInItsShortestHeadAndReadBack() throws {
    for (value, bytes) in signed {
        #expect(written { Cbor.writeInt(value, to: &$0) } == bytes, "\(value)")
        var cursor = SceCursor(bytes)
        #expect(try Cbor.readIntWithin(&cursor, Int64.min, Int64.max) == value, "\(value)")
    }
}

@Test func aSignedIntegerIsReadInAnyHeadAndHeldToItsRange() throws {
    // -1 in its two-byte form; -128 and 127, an int8's ends.
    for (bytes, value): ([UInt8], Int64) in [([0x38, 0x00], -1), ([0x38, 0x7F], -128), ([0x18, 0x7F], 127)] {
        var cursor = SceCursor(bytes)
        #expect(try Cbor.readIntWithin(&cursor, -128, 127) == value)
    }
    // -129 and 128 are past an int8.
    for bytes: [UInt8] in [[0x38, 0x80], [0x18, 0x80]] {
        var cursor = SceCursor(bytes)
        #expect(throws: CodecError.cborOutOfRange) { _ = try Cbor.readIntWithin(&cursor, -128, 127) }
    }
    // Below Int64's least and past its greatest: no Int64 holds either.
    for bytes: [UInt8] in [[0x3B, 0x80, 0, 0, 0, 0, 0, 0, 0], [0x1B, 0x80, 0, 0, 0, 0, 0, 0, 0]] {
        var cursor = SceCursor(bytes)
        #expect(throws: CodecError.cborOutOfRange) {
            _ = try Cbor.readIntWithin(&cursor, Int64.min, Int64.max)
        }
    }
    // A float and a text are no integer.
    for bytes: [UInt8] in [[0xF9, 0x3C, 0x00], [0x61, 0x61]] {
        var cursor = SceCursor(bytes)
        #expect(throws: CodecError.cborWrongType) {
            _ = try Cbor.readIntWithin(&cursor, Int64.min, Int64.max)
        }
    }
}

/// RFC 8949 Appendix A, and the edges of each precision.
private let floats: [(Double, [UInt8])] = [
    (0.0, [0xF9, 0x00, 0x00]),
    (-0.0, [0xF9, 0x80, 0x00]),
    (1.0, [0xF9, 0x3C, 0x00]),
    (1.1, [0xFB, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]),
    (1.5, [0xF9, 0x3E, 0x00]),
    // The greatest half, and one past it in a half's spacing there, which a single holds.
    (65504.0, [0xF9, 0x7B, 0xFF]),
    (65505.0, [0xFA, 0x47, 0x7F, 0xE1, 0x00]),
    (100000.0, [0xFA, 0x47, 0xC3, 0x50, 0x00]),
    (Double(Float.greatestFiniteMagnitude), [0xFA, 0x7F, 0x7F, 0xFF, 0xFF]),
    (Double(Float(0.1)), [0xFA, 0x3D, 0xCC, 0xCC, 0xCD]),
    (1.0e300, [0xFB, 0x7E, 0x37, 0xE4, 0x3C, 0x88, 0x00, 0x75, 0x9C]),
    // The least half, a subnormal; the greatest half subnormal; the least half normal; half the
    // least half, which only a single holds.
    (0x1p-24, [0xF9, 0x00, 0x01]),
    (1023 * 0x1p-24, [0xF9, 0x03, 0xFF]),
    (0x1p-14, [0xF9, 0x04, 0x00]),
    (0x1p-25, [0xFA, 0x33, 0x00, 0x00, 0x00]),
    // One bit below a single's last: only a double holds it.
    (1.0 + 0x1p-24, [0xFB, 0x3F, 0xF0, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00]),
    (-4.0, [0xF9, 0xC4, 0x00]),
    (-4.1, [0xFB, 0xC0, 0x10, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66]),
    (Double.infinity, [0xF9, 0x7C, 0x00]),
    (-Double.infinity, [0xF9, 0xFC, 0x00]),
]

@Test func aFloatIsWrittenInTheShortestPrecisionThatHoldsItAndReadBack() throws {
    for (value, bytes) in floats {
        #expect(written { Cbor.writeFloat(value, to: &$0) } == bytes, "\(value)")
        var cursor = SceCursor(bytes)
        #expect(try Cbor.readFloat(&cursor).bitPattern == value.bitPattern, "\(value)")
    }
    // Every NaN is the one NaN, whatever its sign and payload.
    for nan in [Double.nan, Double(bitPattern: 0x7FF0_0000_0000_0001), Double(bitPattern: 0xFFF8_0000_0000_0000)] {
        #expect(written { Cbor.writeFloat(nan, to: &$0) } == [0xF9, 0x7E, 0x00])
    }
}

@Test func aFloatIsReadInAnyPrecisionThatHoldsIt() throws {
    // 1.5 as a half, a single and a double.
    for bytes: [UInt8] in [[0xF9, 0x3E, 0x00], [0xFA, 0x3F, 0xC0, 0x00, 0x00], [0xFB, 0x3F, 0xF8, 0, 0, 0, 0, 0, 0]] {
        var wide = SceCursor(bytes)
        #expect(try Cbor.readFloat(&wide) == 1.5)
        var narrow = SceCursor(bytes)
        #expect(try Cbor.readFloat32(&narrow) == 1.5)
    }
    // A NaN of each precision.
    for bytes: [UInt8] in [[0xF9, 0x7E, 0x00], [0xFA, 0x7F, 0xC0, 0x00, 0x00], [0xFB, 0x7F, 0xF8, 0, 0, 0, 0, 0, 0]] {
        var wide = SceCursor(bytes)
        #expect(try Cbor.readFloat(&wide).isNaN)
        var narrow = SceCursor(bytes)
        #expect(try Cbor.readFloat32(&narrow).isNaN)
    }
    // 0.1 as a double is no single: a float32 refuses it.
    let tenth: [UInt8] = [0xFB, 0x3F, 0xB9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]
    var wide = SceCursor(tenth)
    #expect(try Cbor.readFloat(&wide) == 0.1)
    var narrow = SceCursor(tenth)
    #expect(throws: CodecError.cborOutOfRange) { _ = try Cbor.readFloat32(&narrow) }
    // An integer, a truth value, null and a negative integer are no float.
    for bytes: [UInt8] in [[0x01], [0xF5], [0xF6], [0x20]] {
        var cursor = SceCursor(bytes)
        #expect(throws: CodecError.cborWrongType, "\(bytes)") { _ = try Cbor.readFloat(&cursor) }
    }
}

private let payment = ["card", "cash", "voucher"]

@Test func aUnionIsAMapOfOneEntryNamedByItsCase() throws {
    // docs/adr/0101 §2.8: `cash` of `union:Payment` is `a1 64 63 61 73 68 a0`.
    let cash: [UInt8] = [0xA1, 0x64, 0x63, 0x61, 0x73, 0x68, 0xA0]
    #expect(
        written {
            Cbor.writeVariant("cash", to: &$0)
            Cbor.writeMapHead(0, to: &$0)
        } == cash)
    var cursor = SceCursor(cash)
    #expect(try Cbor.readVariant(&cursor, names: payment) == 1)
    #expect(try Cbor.readMapLen(&cursor) == 0)
}

@Test func aUnionOfNoEntryOrTwoOrOfAnUnknownCaseIsRefused() {
    let refusals: [([UInt8], CodecError)] = [
        // No entry, and two: the count is judged before any key is read.
        ([0xA0], .cborUnionEntries),
        ([0xA2], .cborUnionEntries),
        // A case no variant names.
        ([0xA1, 0x64, 0x63, 0x6F, 0x69, 0x6E, 0xA0], .cborUnknownVariant),
        // A key that is no text, and an array where the union's map stands.
        ([0xA1, 0x01, 0xA0], .cborWrongType),
        ([0x81, 0xA0], .cborWrongType),
    ]
    for (bytes, error) in refusals {
        var cursor = SceCursor(bytes)
        #expect(throws: error, "\(bytes)") { _ = try Cbor.readVariant(&cursor, names: payment) }
    }
}

@Test func aBooleanIsReadAsFalseOrTrue() throws {
    var cursor = SceCursor([0xF4, 0xF5])
    #expect(try Cbor.readBool(&cursor) == false)
    #expect(try Cbor.readBool(&cursor) == true)
}

@Test func anUnsignedIntegerIsHeldToTheMaximumOfItsEntry() throws {
    var fits = SceCursor([0x18, 0xFF])
    #expect(try Cbor.readUintUpto(&fits, 255) == 255)
    var over = SceCursor([0x19, 0x01, 0x00])
    #expect(throws: CodecError.cborOutOfRange) { _ = try Cbor.readUintUpto(&over, 255) }
}

@Test func aTextIsHeldToItsBoundBeforeItsBytesAreLookedFor() throws {
    var fits = SceCursor([0x62, 0xC3, 0xA9])
    #expect(try Cbor.readText(&fits, maxSize: 2) == "é")
    // The map says three bytes under a bound of two and holds none: the bound answers, not the wait.
    var over = SceCursor([0x63])
    #expect(throws: CodecError.cborOutOfRange) { _ = try Cbor.readText(&over, maxSize: 2) }
    var open = SceCursor([0x63, 0x61, 0x62, 0x63])
    #expect(try Cbor.readText(&open, maxSize: nil) == "abc")
}

@Test func aTextOfBytesThatAreNotUtf8IsRefusedAndNotRepaired() {
    var cursor = SceCursor([0x61, 0xFF])
    #expect(throws: CodecError.invalidUtf8) { _ = try Cbor.readText(&cursor, maxSize: nil) }
}

@Test func aStringThatSaysMoreThanTheInputHoldsNeedsMoreBytes() {
    var text = SceCursor([0x63, 0x61])
    #expect(throws: CodecError.needMoreBytes) { _ = try Cbor.readText(&text, maxSize: nil) }
    var bytes = SceCursor([0x43, 0x01])
    #expect(throws: CodecError.needMoreBytes) { _ = try Cbor.readBytes(&bytes, maxSize: nil) }
}

@Test func aByteStringIsHeldToItsBoundOrItsExactLength() throws {
    var fits = SceCursor([0x42, 0x01, 0x02])
    #expect(try Cbor.readBytes(&fits, maxSize: 2) == [1, 2])
    var over = SceCursor([0x43])
    #expect(throws: CodecError.cborOutOfRange) { _ = try Cbor.readBytes(&over, maxSize: 2) }

    var exact = SceCursor([0x42, 0x01, 0x02])
    #expect(try Cbor.readBytesExact(&exact, length: 2) == [1, 2])
    // The length is judged on what the map says, before the bytes are looked for.
    var wrong = SceCursor([0x43])
    #expect(throws: CodecError.cborWrongLength) { _ = try Cbor.readBytesExact(&wrong, length: 2) }
}

@Test func aMapIsReadAsTheNumberOfEntriesItSays() throws {
    var cursor = SceCursor([0xA2])
    #expect(try Cbor.readMapLen(&cursor) == 2)
    var array = SceCursor([0x82, 0x01, 0x02])
    #expect(throws: CodecError.cborWrongType) { _ = try Cbor.readMapLen(&array) }
}

@Test func anArrayHeadIsWrittenShortestAndReadAsItsCountAndNoOtherItemIs() throws {
    #expect(written { Cbor.writeArrayHead(2, to: &$0) } == [0x82])
    #expect(written { Cbor.writeArrayHead(24, to: &$0) } == [0x98, 0x18])
    var three = SceCursor([0x83])
    #expect(try Cbor.readArrayLen(&three) == 3)
    var wide = SceCursor([0x98, 0x18])
    #expect(try Cbor.readArrayLen(&wide) == 24)
    // An indefinite-length array, and a map where an array stands.
    var indefinite = SceCursor([0x9F])
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readArrayLen(&indefinite) }
    var map = SceCursor([0xA1])
    #expect(throws: CodecError.cborWrongType) { _ = try Cbor.readArrayLen(&map) }
}

@Test func aSkipTakesAnItemAndEverythingNestedInIt() throws {
    // A byte string, a text, an array of two, a map of one pair, a tag over an integer, a float's head
    // and a simple value, then a marker the skips must stop before.
    let items: [[UInt8]] = [
        [0x42, 0x01, 0x02],
        [0x61, 0x61],
        [0x82, 0x01, 0x02],
        [0xA1, 0x01, 0x02],
        [0xC1, 0x05],
        [0xF9, 0x00, 0x00],
        [0xF5],
        [0x20],
    ]
    for item in items {
        var cursor = SceCursor(item + [0xAA])
        try Cbor.skip(&cursor)
        #expect(cursor.remaining == 1, "\(item) was skipped whole")
    }
}

@Test func aValueNestedPastTheDepthIsRefusedAndOneAtItIsSkipped() throws {
    // `n` arrays each holding the next, around an integer.
    func nested(_ n: Int) -> [UInt8] {
        [UInt8](repeating: 0x81, count: n) + [0x00]
    }
    var allowed = SceCursor(nested(Cbor.maxSkipDepth - 1))
    try Cbor.skip(&allowed)
    #expect(allowed.remaining == 0)
    var deep = SceCursor(nested(Cbor.maxSkipDepth))
    #expect(throws: CodecError.cborTooDeep) { try Cbor.skip(&deep) }
}

@Test func aSkipOfAStringThatSaysMoreThanTheInputHoldsNeedsMoreBytes() {
    var cursor = SceCursor([0x43, 0x01])
    #expect(throws: CodecError.needMoreBytes) { try Cbor.skip(&cursor) }
}
