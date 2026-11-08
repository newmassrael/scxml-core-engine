// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readUint(&text) }
    var number = SceCursor([0x05])
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readBool(&number) }
    // A simple value that is neither false nor true.
    var null = SceCursor([0xF6])
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readBool(&null) }
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
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readMapLen(&array) }
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
    #expect(throws: CodecError.cborMalformed) { _ = try Cbor.readArrayLen(&map) }
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
