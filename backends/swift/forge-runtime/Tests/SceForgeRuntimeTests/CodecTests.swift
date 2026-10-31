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
