// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A list of entries of another codec (`tests/forge/resources/codec_repeat_*.scxml`,
// `codec_until_eof_basic.scxml`, `codec_declared_bounds.scxml`; an entry is `CodecRepeatElem`, one
// big-endian `uint16`). The fixtures hold no numerical oracle, so this is where the behaviour is
// held: a list is read as many entries as its count says, or to the end of the frame; a gated list is
// `nil` when its gate is closed and an empty list is a value; a count past the bound is refused
// before an entry is read; and a frame that is refused consumes nothing.
final class CodecListUnitTests: XCTestCase {
    private func entry(_ seq: UInt16) -> CodecRepeatElem {
        CodecRepeatElem(seq: seq)
    }

    func testAListIsAsManyEntriesAsItsCountSays() throws {
        var cursor = SceCursor([0x00, 0x02, 0xCA, 0xFE, 0xBE, 0xEF, 0x77])
        let frame = try CodecRepeatUnconditionalCount.decode(&cursor)
        XCTAssertEqual(frame.links, [entry(0xCAFE), entry(0xBEEF)])
        XCTAssertNil(frame.weights, "a closed gate leaves the gated list absent")
        XCTAssertEqual(cursor.remaining, 1, "what follows the list is the caller's")
    }

    func testAGatedListBorrowsTheCountOfAnUngatedField() throws {
        var cursor = SceCursor([0x01, 0x02, 0xCA, 0xFE, 0xBE, 0xEF, 0x11, 0x11, 0x22, 0x22])
        let frame = try CodecRepeatUnconditionalCount.decode(&cursor)
        XCTAssertEqual(frame.weights, [entry(0x1111), entry(0x2222)])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAnEncodeWritesEachEntryInTurn() throws {
        let frame = CodecRepeatUnconditionalCount(
            options: 0x01, links_len: 2, links: [entry(0xCAFE), entry(0xBEEF)],
            weights: [entry(0x1111), entry(0x2222)])
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(
            sink.bytes, [0x01, 0x02, 0xCA, 0xFE, 0xBE, 0xEF, 0x11, 0x11, 0x22, 0x22])
    }

    func testAnEntryCutShortRefusesTheFrameAndConsumesNothing() {
        var cursor = SceCursor([0x00, 0x02, 0xCA, 0xFE, 0xBE])
        XCTAssertThrowsError(try CodecRepeatUnconditionalCount.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 5)
    }

    func testAListToTheEndOfTheFrameTakesEveryEntryThatIsLeft() throws {
        var cursor = SceCursor([0xCA, 0xFE, 0xBE, 0xEF])
        let frame = try CodecUntilEofBasic.decode(&cursor)
        XCTAssertEqual(frame.msgs.entries, [entry(0xCAFE), entry(0xBEEF)])
        XCTAssertEqual(cursor.remaining, 0)
        var empty = SceCursor([])
        XCTAssertEqual(try CodecUntilEofBasic.decode(&empty).msgs.entries, [])
    }

    // docs/adr/0065 §6 R2: the list is held in a type bounded by its `max-count`, made through its
    // one door, and the encode writes the entries it holds in turn.
    func testAListToTheEndOfTheFrameIsWrittenEntryByEntry() throws {
        let msgs = try XCTUnwrap(
            BoundedList<CodecRepeatElem, CodecUntilEofBasic.MsgsBound>([entry(0xCAFE), entry(0xBEEF)]))
        var sink = BytesSink()
        try CodecUntilEofBasic(msgs: msgs).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0xCA, 0xFE, 0xBE, 0xEF])
    }

    func testATrailingEntryCutShortRefusesTheWholeFrame() {
        var cursor = SceCursor([0xCA, 0xFE, 0xBE])
        XCTAssertThrowsError(try CodecUntilEofBasic.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 3)
    }

    // The bound is `max-count="64"`: the 65th entry is refused before it is read, so a frame that
    // is also cut short is refused for its count and not for its missing bytes.
    func testAnEntryPastTheBoundIsRefusedBeforeItIsRead() throws {
        var atTheBound = SceCursor([UInt8](repeating: 0x01, count: 128))
        XCTAssertEqual(try CodecUntilEofBasic.decode(&atTheBound).msgs.count, 64)
        var past = SceCursor([UInt8](repeating: 0x01, count: 129))
        XCTAssertThrowsError(try CodecUntilEofBasic.decode(&past)) { error in
            XCTAssertEqual(error as? CodecError, .tooManyElements)
        }
        XCTAssertEqual(past.remaining, 129)
    }

    func testAClosedGateLeavesTheListAbsentAndAnEmptyListIsAValue() throws {
        var closed = SceCursor([0x00])
        let absent = try CodecRepeatPresentIfBasic.decode(&closed)
        XCTAssertNil(absent.elems)
        var empty = SceCursor([0x01, 0x00])
        let none = try CodecRepeatPresentIfBasic.decode(&empty)
        XCTAssertEqual(none.elems?.entries, [])
    }

    // The count and the gate flag are derived from the list (docs/adr/0081 §3): the value holds
    // neither, and the encode writes the count of the entries the value holds and sets the flag
    // where it holds a list.
    func testAGatedListIsReadWhenItsGateIsOpen() throws {
        var cursor = SceCursor([0x01, 0x02, 0xBE, 0xEF, 0xFA, 0xCE])
        let frame = try CodecRepeatPresentIfBasic.decode(&cursor)
        XCTAssertEqual(frame.elems?.entries, [entry(0xBEEF), entry(0xFACE)])
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x01, 0x02, 0xBE, 0xEF, 0xFA, 0xCE])
    }

    func testAnAbsentGatedListWritesNothing() throws {
        var sink = BytesSink()
        try CodecRepeatPresentIfBasic(carrier: 0x00, elems: nil).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x00])
    }

    func testAnEmptyGatedListIsWrittenWithItsGateSetAndACountOfNone() throws {
        let elems = try XCTUnwrap(BoundedList<CodecRepeatElem, CodecRepeatPresentIfBasic.ElemsBound>([]))
        var sink = BytesSink()
        try CodecRepeatPresentIfBasic(carrier: 0x00, elems: elems).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x01, 0x00])
    }

    // docs/adr/0065 §6 R2: a list past its `max-count` is no value of its type, so no encode is
    // ever handed one.
    func testAListPastItsBoundIsNoValueOfItsType() {
        let past = [CodecRepeatElem](repeating: entry(1), count: 33)
        XCTAssertNil(BoundedList<CodecRepeatElem, CodecRepeatPresentIfBasic.ElemsBound>(past))
        let most = [CodecRepeatElem](repeating: entry(1), count: 32)
        XCTAssertNotNil(BoundedList<CodecRepeatElem, CodecRepeatPresentIfBasic.ElemsBound>(most))
    }

    // `count` says 3 entries and the bound is 2: refused for that, though the frame holds none.
    func testACountPastTheBoundIsRefusedBeforeAnEntryIsReadAndNotWaitedOn() {
        var cursor = SceCursor([0x00, 0x01, 0x03])
        XCTAssertThrowsError(try CodecDeclaredBounds.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .tooManyElements)
        }
        XCTAssertEqual(cursor.remaining, 3)
    }

    // A length of 2^32 + 4 is judged in the 64 bits it was read in: narrowed to 32 first it would
    // read 4 and the frame would be taken.
    func testAWideLengthIsJudgedInTheTypeItWasReadIn() {
        var cursor = SceCursor([0x84, 0x80, 0x80, 0x80, 0x10])
        XCTAssertThrowsError(try CodecDeclaredBounds.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .lengthOutOfRange)
        }
        XCTAssertEqual(cursor.remaining, 5)
    }

    // The tag's length is its carried value less one, so a carried 0 says -1: refused, not wrapped.
    func testALengthThatSaysLessThanNothingIsRefusedAndNotWrapped() {
        var cursor = SceCursor([0x00, 0x00])
        XCTAssertThrowsError(try CodecDeclaredBounds.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .lengthOutOfRange)
        }
    }
}
