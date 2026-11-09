// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// The contract of the cursor and the sinks (RFC §synth-5-B L494-519), held against the cases of the
// Go runtime's own tests: a decode that runs out of bytes answers "need more bytes" and moves
// nothing, an encode into a bounded sink that is too small answers "buffer overflow" and writes
// nothing, and an integer is the same bytes in both directions in either byte order.

@Test func aCursorReportsWhatIsLeftAndMovesOnlyWhenAdvanced() throws {
    var cursor = SceCursor([1, 2, 3, 4])
    #expect(cursor.remaining == 4)
    #expect(Array(try cursor.peekSlice(2)) == [1, 2])
    #expect(cursor.remaining == 4, "a peek moves nothing")
    try cursor.advance(3)
    #expect(cursor.remaining == 1)
    #expect(Array(try cursor.peekSlice(1)) == [4])
}

@Test func aCursorThatRunsOutAnswersNeedMoreBytesAndStaysWhereItWas() throws {
    var cursor = SceCursor([1, 2, 3])
    try cursor.advance(1)
    #expect(throws: CodecError.needMoreBytes) { try cursor.peekSlice(3) }
    #expect(throws: CodecError.needMoreBytes) { try cursor.advance(3) }
    #expect(cursor.remaining == 2, "a refused advance has consumed nothing")
    // Exactly the rest is not too much.
    try cursor.advance(2)
    #expect(cursor.remaining == 0)
    #expect(throws: CodecError.needMoreBytes) { try cursor.peekSlice(1) }
}

@Test func aNegativeCountIsNotMoreThanTheCursorHasButIsRefusedAsWell() throws {
    var cursor = SceCursor([1, 2])
    #expect(throws: CodecError.needMoreBytes) { try cursor.peekSlice(-1) }
    #expect(throws: CodecError.needMoreBytes) { try cursor.advance(-1) }
    #expect(cursor.remaining == 2)
}

@Test func anIntegerIsReadFromItsOffsetInEitherByteOrder() throws {
    var cursor = SceCursor([0xAA, 0x12, 0x34, 0x56, 0x78])
    try cursor.advance(1)
    #expect(try cursor.peekInteger(UInt8.self, at: 0, .big) == 0x12)
    #expect(try cursor.peekInteger(UInt16.self, at: 0, .big) == 0x1234)
    #expect(try cursor.peekInteger(UInt16.self, at: 0, .little) == 0x3412)
    #expect(try cursor.peekInteger(UInt16.self, at: 2, .big) == 0x5678)
    #expect(try cursor.peekInteger(UInt32.self, at: 0, .big) == 0x1234_5678)
    #expect(try cursor.peekInteger(UInt32.self, at: 0, .little) == 0x7856_3412)
    #expect(cursor.remaining == 4, "reading an integer moves nothing")
}

@Test func aSignedIntegerIsTheTwosComplementReadingOfItsBytes() throws {
    let cursor = SceCursor([0xFF, 0xFE, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
    #expect(try cursor.peekInteger(Int8.self, at: 0, .big) == -1)
    #expect(try cursor.peekInteger(Int16.self, at: 0, .big) == -2)
    #expect(try cursor.peekInteger(Int16.self, at: 0, .little) == Int16(bitPattern: 0xFEFF))
    #expect(try cursor.peekInteger(Int8.self, at: 2, .big) == Int8.min)
    #expect(try cursor.peekInteger(Int64.self, at: 2, .big) == Int64(bitPattern: 0x8000_0000_0000_0000))
}

@Test func anIntegerPastTheEndIsNeedMoreBytes() throws {
    let cursor = SceCursor([1, 2, 3])
    #expect(throws: CodecError.needMoreBytes) { try cursor.peekInteger(UInt32.self, at: 0, .big) }
    #expect(throws: CodecError.needMoreBytes) { try cursor.peekInteger(UInt16.self, at: 2, .big) }
    #expect(throws: CodecError.needMoreBytes) { try cursor.peekInteger(UInt8.self, at: -1, .big) }
    #expect(try cursor.peekInteger(UInt16.self, at: 1, .big) == 0x0203)
}

@Test func aWordOfFewerBytesThanItsTypeIsReadFromThoseBytesAlone() throws {
    var cursor = SceCursor([0xAA, 0x12, 0x34, 0x56, 0x78])
    try cursor.advance(1)
    #expect(try cursor.peekInteger(UInt32.self, bytes: 3, at: 0, .big) == 0x12_3456)
    #expect(try cursor.peekInteger(UInt32.self, bytes: 3, at: 0, .little) == 0x56_3412)
    #expect(try cursor.peekInteger(UInt64.self, bytes: 3, at: 1, .big) == 0x34_5678)
    #expect(throws: CodecError.needMoreBytes) {
        try cursor.peekInteger(UInt32.self, bytes: 3, at: 2, .big)
    }
    #expect(cursor.remaining == 4, "reading a word moves nothing")
}

@Test func aWrittenIntegerReadsBackAsItself() throws {
    var sink = BytesSink()
    try sink.writeInteger(UInt16(0x1234), .big)
    try sink.writeInteger(UInt16(0x1234), .little)
    try sink.writeInteger(Int8(-2), .big)
    try sink.writeInteger(Int32(-2), .big)
    try sink.writeInteger(UInt64.max, .little)
    #expect(Array(sink.bytes.prefix(4)) == [0x12, 0x34, 0x34, 0x12])
    #expect(sink.bytes[4] == 0xFE)
    #expect(Array(sink.bytes[5..<9]) == [0xFF, 0xFF, 0xFF, 0xFE])

    let cursor = SceCursor(sink.bytes)
    #expect(try cursor.peekInteger(UInt16.self, at: 0, .big) == 0x1234)
    #expect(try cursor.peekInteger(UInt16.self, at: 2, .little) == 0x1234)
    #expect(try cursor.peekInteger(Int8.self, at: 4, .big) == -2)
    #expect(try cursor.peekInteger(Int32.self, at: 5, .big) == -2)
    #expect(try cursor.peekInteger(UInt64.self, at: 9, .little) == UInt64.max)
}

@Test func anIntegerStoredInAFrameLandsAtItsOffsetAndLeavesTheRestAlone() throws {
    var frame = [UInt8](repeating: 0xEE, count: 8)
    storeInteger(UInt16(0x1234), into: &frame, at: 1, .big)
    storeInteger(UInt16(0x1234), into: &frame, at: 3, .little)
    storeInteger(Int16(-2), into: &frame, at: 5, .big)
    #expect(frame == [0xEE, 0x12, 0x34, 0x34, 0x12, 0xFF, 0xFE, 0xEE])

    // What is stored reads back as itself, which is what a decode of an encode relies on.
    let cursor = SceCursor(frame)
    #expect(try cursor.peekInteger(UInt16.self, at: 1, .big) == 0x1234)
    #expect(try cursor.peekInteger(UInt16.self, at: 3, .little) == 0x1234)
    #expect(try cursor.peekInteger(Int16.self, at: 5, .big) == -2)
}

@Test func aFrameStoredWholeIsWrittenToABoundedSinkOrNotAtAll() throws {
    var frame = [UInt8](repeating: 0, count: 4)
    storeInteger(UInt32(0xDEAD_BEEF), into: &frame, at: 0, .big)
    var tooSmall = BoundedSink(capacity: 3)
    #expect(throws: CodecError.bufferOverflow) { try tooSmall.writeBytes(frame) }
    #expect(tooSmall.written.isEmpty, "a frame that does not fit leaves no part of itself in the sink")
    var fits = BoundedSink(capacity: 4)
    try fits.writeBytes(frame)
    #expect(fits.written == [0xDE, 0xAD, 0xBE, 0xEF])
}

@Test func aGrowableSinkNeverFailsAndCountsWhatItWrote() throws {
    var sink = BytesSink()
    #expect(sink.position == 0)
    try sink.writeBytes([])
    try sink.writeBytes([1, 2, 3])
    try sink.writeBytes(Array(repeating: 9, count: 1000))
    #expect(sink.position == 1003)
    #expect(Array(sink.bytes.prefix(3)) == [1, 2, 3])
}

@Test func aSinkContinuedBehindAPrefixCountsOnlyItsOwnBytes() throws {
    var sink = BytesSink(prefix: [7, 7])
    try sink.writeBytes([1, 2])
    #expect(sink.position == 2, "the frame packed behind another counts its own bytes")
    #expect(sink.bytes == [7, 7, 1, 2])
}

@Test func aBoundedSinkRefusesAWriteThatDoesNotFitAndWritesNothingOfIt() throws {
    var sink = BoundedSink(capacity: 4)
    try sink.writeBytes([1, 2, 3])
    #expect(throws: CodecError.bufferOverflow) { try sink.writeBytes([4, 5]) }
    #expect(sink.written == [1, 2, 3], "a refused write left no part of itself behind")
    #expect(sink.remaining == 1)
    // Exactly what is left fits, and an empty write always does.
    try sink.writeBytes([])
    try sink.writeBytes([4])
    #expect(sink.written == [1, 2, 3, 4])
    #expect(sink.remaining == 0)
    #expect(sink.position == 4)
    #expect(throws: CodecError.bufferOverflow) { try sink.writeInteger(UInt8(5), .big) }
}

@Test func aBoundedSinkOfNoCapacityTakesNothingButTheEmptyWrite() throws {
    var sink = BoundedSink(capacity: 0)
    try sink.writeBytes([])
    #expect(throws: CodecError.bufferOverflow) { try sink.writeBytes([1]) }
    #expect(sink.written.isEmpty)
    var negative = BoundedSink(capacity: -3)
    #expect(negative.remaining == 0, "a capacity below zero is no capacity")
    try negative.writeBytes([])
}

// A frame whose layout depends on what it holds is read field by field and written field by field:
// the reads move the cursor, a variable-length integer is the canonical Zenoh ZInt in both
// directions, and a refused read moves nothing.

@Test func aReadMovesTheCursorPastWhatItReturned() throws {
    var cursor = SceCursor([0x01, 0x02, 0x03, 0x04, 0x05, 0x06])
    #expect(try cursor.readInteger(UInt16.self, .big) == 0x0102)
    #expect(try cursor.readInteger(UInt16.self, .little) == 0x0403)
    #expect(Array(try cursor.readSlice(1)) == [0x05])
    #expect(cursor.remaining == 1)
    #expect(throws: CodecError.needMoreBytes) { try cursor.readInteger(UInt16.self, .big) }
    #expect(cursor.remaining == 1, "a refused read has consumed nothing")
}

@Test func aVariableLengthIntegerTakesTheFewestBytesThatHoldIt() throws {
    func bytes<T: FixedWidthInteger & UnsignedInteger>(_ value: T) -> [UInt8] {
        var out: [UInt8] = []
        appendVle(value, to: &out)
        return out
    }
    #expect(bytes(UInt64(0)) == [0x00])
    #expect(bytes(UInt64(127)) == [0x7F])
    #expect(bytes(UInt64(128)) == [0x80, 0x01])
    #expect(bytes(UInt16.max) == [0xFF, 0xFF, 0x03])
    #expect(bytes(UInt32.max).count == 5)
    #expect(bytes(UInt64.max) == [UInt8](repeating: 0xFF, count: 9), "a 64-bit value takes 9 bytes, not 10")
}

@Test func aVariableLengthIntegerReadsBackAsItselfAtEveryWidth() throws {
    func roundTrip<T: FixedWidthInteger & UnsignedInteger>(_ values: [T]) throws {
        for value in values {
            var out: [UInt8] = []
            appendVle(value, to: &out)
            var cursor = SceCursor(out + [0xEE])
            #expect(try cursor.readVle(T.self) == value, "\(T.self) \(value)")
            #expect(cursor.remaining == 1, "the read stops at the end of the chain")
        }
    }
    try roundTrip([UInt8(0), 1, 127, 128, 255])
    try roundTrip([UInt16(0), 1, 127, 128, 16383, 16384, .max])
    try roundTrip([UInt32(0), 127, 128, 1 << 21, 1 << 28, .max])
    try roundTrip([UInt64(0), 127, 128, 1 << 35, 1 << 56, 1 << 63, .max])
}

@Test func aChainWiderThanItsTypeIsRefusedAndOneCutShortNeedsMoreBytes() throws {
    // The last byte of a 16-bit value holds 2 bits: 4 is one too many.
    var wide = SceCursor([0xFF, 0xFF, 0x04])
    #expect(throws: CodecError.vleWidthOverflow) { try wide.readVle(UInt16.self) }
    #expect(wide.remaining == 3, "a refused read has consumed nothing")
    var cut = SceCursor([0x80, 0x80])
    #expect(throws: CodecError.needMoreBytes) { try cut.readVle(UInt32.self) }
    #expect(cut.remaining == 2)
    var empty = SceCursor([])
    #expect(throws: CodecError.needMoreBytes) { try empty.readVle(UInt8.self) }
}

@Test func anIntegerAppendedIsTheBytesOfItsByteOrder() throws {
    var out: [UInt8] = []
    appendInteger(UInt32(0x0A0B0C0D), to: &out, .big)
    appendInteger(UInt16(0x0102), to: &out, .little)
    appendInteger(Int8(-2), to: &out, .big)
    #expect(out == [0x0A, 0x0B, 0x0C, 0x0D, 0x02, 0x01, 0xFE])
}

@Test func aTextIsTheUtf8BytesTheCursorReadsAndMovesPast() throws {
    // "a", "é" (two bytes) and a four-byte scalar.
    let bytes: [UInt8] = [0x61, 0xC3, 0xA9, 0xF0, 0x9F, 0x98, 0x80, 0x7F]
    var cursor = SceCursor(bytes)
    #expect(try cursor.readString(7) == "aé😀")
    #expect(cursor.remaining == 1, "the read stopped at the count")
    var empty = SceCursor([])
    #expect(try empty.readString(0) == "")
}

@Test func bytesThatAreNotUtf8AreRefusedAndNotRepairedAndConsumeNothing() {
    var cursor = SceCursor([0x61, 0xFF, 0x62])
    #expect(throws: CodecError.invalidUtf8) { try cursor.readString(3) }
    #expect(cursor.remaining == 3, "a refused read has consumed nothing")
    // A scalar cut off at the end of the bytes is not text either.
    var cut = SceCursor([0xC3])
    #expect(throws: CodecError.invalidUtf8) { try cut.readString(1) }
}

@Test func aTextOfMoreBytesThanTheCursorHoldsNeedsMoreBytes() {
    var cursor = SceCursor([0x61, 0x62])
    #expect(throws: CodecError.needMoreBytes) { try cursor.readString(3) }
    #expect(throws: CodecError.needMoreBytes) { try cursor.readString(-1) }
    #expect(cursor.remaining == 2)
}

@Test func theBytesAWriteLeavesInASinkOfItsOwnAreAnsweredAndARefusalIsNil() {
    #expect(encodeToBytes { try $0.writeBytes([1, 2, 3]) } == [1, 2, 3])
    #expect(encodeToBytes { _ in throw CodecError.lengthOutOfRange } == nil)
    // A write that refuses after it wrote has no bytes to give: the answer is nil and not a prefix.
    #expect(
        encodeToBytes { sink in
            try sink.writeBytes([1])
            throw CodecError.tooManyElements
        } == nil)
}

@Test func anEntrysIdentifierIsTheSameNumberWhateverTypeItIsReadFrom() {
    #expect(chainIdentifier(true) == 1)
    #expect(chainIdentifier(false) == 0)
    #expect(chainIdentifier(UInt8(0x4)) == 4)
    #expect(chainIdentifier(UInt16(0xFFFF)) == 0xFFFF)
    #expect(chainIdentifier(UInt64.max) == UInt64.max)
}
