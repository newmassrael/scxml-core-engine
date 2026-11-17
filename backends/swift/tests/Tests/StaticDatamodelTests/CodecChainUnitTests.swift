// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A bounded list of entries of one codec, in two forms. One runs to the end of the frame
// (`tests/forge/resources/codec_tlv_chain_basic.scxml`: a header byte, then entries of a type, a length
// and that many bytes, at most 8). The other is closed by the entry itself
// (`tests/forge/resources/codec_zenoh_err.scxml`: a chain that is on the wire when the header says it
// is, whose every entry but the last has its `Z` bit set, at most 4). The fixtures hold no numerical
// oracle, so this is where the behaviour is held: the entries are read in order, an entry past the
// depth is refused and not left unread, an entry that promises another one the frame lacks leaves the
// frame short, and a frame that is refused consumes nothing.
final class CodecChainUnitTests: XCTestCase {
    /// An entry of `type` holding `body`, whose length the encode derives (docs/adr/0081 §3). Every
    /// body this file builds is within the entry's bound, so the bound's door answers it.
    private func entry(_ type: UInt8, _ body: [UInt8]) -> CodecTlvEntry {
        guard let held = BoundedBytes<CodecTlvEntry.EntryBodyBound>(body) else {
            preconditionFailure("a body of \(body.count) bytes is past the entry's bound")
        }
        return CodecTlvEntry(entry_type: type, entry_body: held)
    }

    func testEntriesAreReadInOrderToTheEndOfTheFrame() throws {
        var cursor = SceCursor([0x07, 0x01, 0x02, 0xAA, 0xBB, 0x02, 0x00])
        let frame = try CodecTlvChainBasic.decode(&cursor)
        XCTAssertEqual(frame.header_flags, 0x07)
        XCTAssertEqual(frame.extensions.entries, [entry(1, [0xAA, 0xBB]), entry(2, [])])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAFrameWithNoEntriesHasAnEmptyChain() throws {
        var cursor = SceCursor([0x07])
        XCTAssertEqual(try CodecTlvChainBasic.decode(&cursor).extensions.entries, [])
    }

    func testTheDeepestChainTheDocumentAdmitsIsTaken() throws {
        let eight: [UInt8] = [0x00] + (0..<8).flatMap { _ -> [UInt8] in [1, 0] }
        var cursor = SceCursor(eight)
        XCTAssertEqual(try CodecTlvChainBasic.decode(&cursor).extensions.count, 8)
    }

    func testAnEntryPastTheDepthIsRefusedAndNotLeftUnread() {
        // Nine entries of a type and a length of zero, where the bound is eight.
        let nine: [UInt8] = [0x00] + (0..<9).flatMap { _ -> [UInt8] in [1, 0] }
        var cursor = SceCursor(nine)
        XCTAssertThrowsError(try CodecTlvChainBasic.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .tooManyElements)
        }
        XCTAssertEqual(cursor.remaining, nine.count, "a refused frame leaves the cursor where it was")
    }

    func testAnEntryCutShortRefusesTheFrameAndConsumesNothing() {
        var cursor = SceCursor([0x07, 0x01])
        XCTAssertThrowsError(try CodecTlvChainBasic.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testAnEncodeWritesEachEntryInTurn() throws {
        let extensions = try XCTUnwrap(
            BoundedList<CodecTlvEntry, CodecTlvChainBasic.ExtensionsBound>([
                entry(1, [0xAA, 0xBB]), entry(2, []),
            ]))
        var sink = BytesSink()
        try CodecTlvChainBasic(header_flags: 0x07, extensions: extensions).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x07, 0x01, 0x02, 0xAA, 0xBB, 0x02, 0x00])
    }

    // docs/adr/0065 §6 R2: a chain past its `max-depth` is no value of its type, so no encode is
    // ever handed one.
    func testAChainPastItsDepthIsNoValueOfItsType() {
        let nine = (0..<9).map { _ in entry(1, []) }
        XCTAssertNil(BoundedList<CodecTlvEntry, CodecTlvChainBasic.ExtensionsBound>(nine))
        XCTAssertNotNil(BoundedList<CodecTlvEntry, CodecTlvChainBasic.ExtensionsBound>(Array(nine.prefix(8))))
    }

    // The chain of `CodecZenohErr` is gated by `Z` in its header and closed by the `Z` of an entry.
    func testAChainClosedByAnEntryFlagEndsAtTheEntryThatClearsIt() throws {
        var cursor = SceCursor([0x85, 0x81, 0x02, 0x00])
        let frame = try CodecZenohErr.decode(&cursor)
        let first = CodecZenohExtEntry(header: 0x81, body: .codecZenohExtUnit(CodecZenohExtUnit()))
        let last = CodecZenohExtEntry(header: 0x02, body: .codecZenohExtUnit(CodecZenohExtUnit()))
        XCTAssertEqual(
            frame.extensions?.entries, [first, last], "the first says another follows, the second does not")
        XCTAssertEqual(frame.payload.bytes, [])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAClosedChainWhoseGateIsClearIsAbsent() throws {
        var cursor = SceCursor([0x05, 0x02, 0xAA, 0xBB])
        let frame = try CodecZenohErr.decode(&cursor)
        XCTAssertNil(frame.extensions)
        XCTAssertEqual(frame.payload.bytes, [0xAA, 0xBB])
    }

    func testAnEntryThatPromisesAnotherTheFrameLacksLeavesTheFrameShort() {
        var cursor = SceCursor([0x85, 0x81])
        XCTAssertThrowsError(try CodecZenohErr.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testTheEntryAfterTheDepthIsRefusedWhenTheChainStillPromisesMore() {
        // A chain the protocol bounds, here at four (the Zenoh extensions state no bound, so they
        // are walked to their own end). The head byte, then four entries of a header with its
        // continuation flag set and a one-byte value, each promising another, and bytes left after.
        let promising: [UInt8] = [0x80, 0x01, 0x80, 0x01, 0x80, 0x01, 0x80, 0x01]
        var past = SceCursor([0x00] + promising + [0x02, 0x00])
        XCTAssertThrowsError(try CodecChainFlagCarrier.decode(&past)) { error in
            XCTAssertEqual(error as? CodecError, .tooManyElements)
        }
        XCTAssertEqual(past.remaining, 11)
        // The same four with nothing after them: the fifth the last promised is simply missing.
        var short = SceCursor([0x00] + promising)
        XCTAssertThrowsError(try CodecChainFlagCarrier.decode(&short)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
    }

    // The protocol states no depth for the Zenoh extensions, so the chain is walked to the entry
    // that says it is the last, however many came before: the same bytes that a bound of four
    // refuses (above) are five entries here.
    func testAChainTheProtocolGivesNoDepthIsWalkedToItsOwnEnd() throws {
        var cursor = SceCursor([0x85, 0x81, 0x81, 0x81, 0x81, 0x02, 0x00])
        let frame = try CodecZenohErr.decode(&cursor)
        XCTAssertEqual(frame.extensions?.count, 5)
        XCTAssertEqual(cursor.remaining, 0)
    }

    // docs/adr/0065 §6 R7: a chain closed by a flag holds at least one entry, so an empty one is no
    // value of its type and no encode is ever handed one.
    func testAChainClosedByAFlagHoldsAtLeastOneEntryWhenItIsPresent() {
        XCTAssertNil(BoundedList<CodecZenohExtEntry, CodecZenohErr.ExtensionsBound>([]))
    }

    // The encode sets the flag the chain ends on, on every entry but the last, whatever the entry
    // presents, and derives the header's `Z` from the chain being there (docs/adr/0081 §3).
    func testAnEncodeClearsTheFlagOfTheLastEntryWhateverItPresents() throws {
        let entry = CodecZenohExtEntry(header: 0x81, body: .codecZenohExtUnit(CodecZenohExtUnit()))
        let extensions = try XCTUnwrap(
            BoundedList<CodecZenohExtEntry, CodecZenohErr.ExtensionsBound>([entry]))
        var sink = BytesSink()
        try CodecZenohErr(header: 0x05, encoding: nil, extensions: extensions, payload: .fewest)
            .encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x85, 0x01, 0x00])
    }
}
