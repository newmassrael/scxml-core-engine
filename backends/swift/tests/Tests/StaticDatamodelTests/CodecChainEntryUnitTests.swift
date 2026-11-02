// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A chain whose entries are not read alike: an entry laid out by the entry before it
// (`tests/forge/resources/codec_chain_prev_carrier.scxml`), and fields of the frame that are on the wire
// by the identifiers the entries hold (`codec_chain_has_tagged.scxml`, `codec_chain_has_marker.scxml`).
// The fixtures hold no numerical oracle, so this is where the behaviour is held: the value the chain
// carries from one entry to the next is 1 only right after an entry of the identifier the document
// names, the first entry has none before it, the encode walks the same values the decode did, and a gate
// on the entries is asked of the entries the frame holds, by the same answer on both sides.
final class CodecChainEntryUnitTests: XCTestCase {
    // Entry codec `codec_chain_prev_entry`: a header (ext_id in bits 0-3, Z in bit 7), then a byte when
    // it follows no marker, two bytes when it follows one, and a tag byte when the chain is tagged.
    // The marker is the entry whose identifier is 4.

    func testAnEntryAfterTheMarkerIsReadWithItsLongValue() throws {
        // Entry 0 (id 1), entry 1 the marker (id 4), entry 2 (id 2) right after the marker.
        var cursor = SceCursor([0x00, 0x81, 0x11, 0x84, 0x22, 0x02, 0x33, 0x44])
        let frame = try CodecChainPrevCarrier.decode(&cursor)
        XCTAssertEqual(frame.entries.map(\.short_value), [0x11, 0x22, nil])
        XCTAssertEqual(frame.entries.map(\.long_value), [nil, nil, 0x3344])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testOnlyTheEntryRightAfterTheMarkerFollowsIt() throws {
        // The marker, an entry after it, and then an entry two places after the marker.
        var cursor = SceCursor([0x00, 0x84, 0x11, 0x81, 0x22, 0x33, 0x03, 0x44])
        let frame = try CodecChainPrevCarrier.decode(&cursor)
        XCTAssertEqual(frame.entries.map(\.short_value), [0x11, nil, 0x44])
        XCTAssertEqual(frame.entries.map(\.long_value), [nil, 0x2233, nil])
    }

    func testTheFirstEntryHasNoEntryBeforeIt() throws {
        // Its identifier is the marker's, but nothing precedes it.
        var cursor = SceCursor([0x00, 0x04, 0x77])
        let frame = try CodecChainPrevCarrier.decode(&cursor)
        XCTAssertEqual(frame.entries.map(\.short_value), [0x77])
        XCTAssertEqual(frame.entries.map(\.long_value), [nil])
    }

    func testAnInputOfTheCarrierReachesEveryEntryAlongsideTheChainsOwn() throws {
        // The head's bit 0 is handed to every entry as `tagged`: each reads a tag byte.
        var cursor = SceCursor([0x01, 0x03, 0x55, 0x66])
        let frame = try CodecChainPrevCarrier.decode(&cursor)
        XCTAssertEqual(frame.entries.map(\.short_value), [0x55])
        XCTAssertEqual(frame.entries.map(\.tag), [0x66])
    }

    func testAnEntryThatTheMarkerLeavesShortRefusesTheFrameAndConsumesNothing() {
        var cursor = SceCursor([0x00, 0x84])
        XCTAssertThrowsError(try CodecChainPrevCarrier.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testAnEncodeWalksTheValuesTheDecodeDid() throws {
        for bytes: [UInt8] in [
            [0x00, 0x81, 0x11, 0x84, 0x22, 0x02, 0x33, 0x44],
            [0x00, 0x84, 0x11, 0x81, 0x22, 0x33, 0x03, 0x44],
            [0x01, 0x03, 0x55, 0x66],
        ] {
            var cursor = SceCursor(bytes)
            let frame = try CodecChainPrevCarrier.decode(&cursor)
            var sink = BytesSink()
            try frame.encode(into: &sink)
            XCTAssertEqual(sink.bytes, bytes)
        }
    }

    // Codec `codec_chain_has_tagged`: a header (wide in bit 0), entries of an entry type, a control byte
    // (more in bit 0) and a body byte, then `priority` (a byte) on the wire when an entry of type 7 is
    // there or the header is wide, then `checksum` (two bytes) on the wire when no entry is of type 9.

    func testAFieldWhoseGateAsksForAnEntryIsOnTheWireOnlyWhenOneIsThere() throws {
        // No entry of type 7 and a narrow header: no priority; no entry of type 9: a checksum.
        var plain = SceCursor([0x00, 0x01, 0x00, 0xAA, 0x12, 0x34])
        let none = try CodecChainHasTagged.decode(&plain)
        XCTAssertNil(none.priority)
        XCTAssertEqual(none.checksum, 0x1234)

        // An entry of type 7: a priority.
        var seven = SceCursor([0x00, 0x07, 0x00, 0xBB, 0x55, 0x12, 0x34])
        let with = try CodecChainHasTagged.decode(&seven)
        XCTAssertEqual(with.priority, 0x55)
        XCTAssertEqual(with.checksum, 0x1234)
    }

    func testTheGateIsAskedOfEveryEntryNotOnlyTheFirst() throws {
        var cursor = SceCursor([0x00, 0x01, 0x01, 0xAA, 0x07, 0x00, 0xBB, 0x55, 0x12, 0x34])
        let frame = try CodecChainHasTagged.decode(&cursor)
        XCTAssertEqual(frame.entries.count, 2)
        XCTAssertEqual(frame.priority, 0x55)
    }

    func testAClauseOnTheEntriesJoinsAClauseOnAFlag() throws {
        // A wide header puts the priority on the wire with no entry of type 7.
        var cursor = SceCursor([0x01, 0x01, 0x00, 0xAA, 0x66, 0x12, 0x34])
        XCTAssertEqual(try CodecChainHasTagged.decode(&cursor).priority, 0x66)
    }

    func testANegatedClauseTakesTheFieldOffTheWireWhenAnEntryIsThere() throws {
        var cursor = SceCursor([0x00, 0x09, 0x00, 0xCC])
        let frame = try CodecChainHasTagged.decode(&cursor)
        XCTAssertNil(frame.checksum)
        XCTAssertNil(frame.priority)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAnEncodeRefusesAValueWhoseFieldAndEntriesDisagreeBeforeAByteIsWritten() {
        let entry = CodecChainHasTaggedEntry(entry_type: 7, ctl: 0, body: 0)
        var sink = BytesSink()
        // An entry of type 7 and no priority.
        XCTAssertThrowsError(
            try CodecChainHasTagged(header: 0, entries: [entry], priority: nil, checksum: 1)
                .encode(into: &sink)
        ) { error in
            XCTAssertEqual(error as? CodecError, .presentIfMismatch)
        }
        // No entry of type 9 and no checksum.
        XCTAssertThrowsError(
            try CodecChainHasTagged(header: 0, entries: [entry], priority: 1, checksum: nil)
                .encode(into: &sink)
        ) { error in
            XCTAssertEqual(error as? CodecError, .presentIfMismatch)
        }
        XCTAssertEqual(sink.bytes, [])
    }

    func testAnEncodeAsksTheEntriesWhatTheDecodeAskedTheFrame() throws {
        for bytes: [UInt8] in [
            [0x00, 0x01, 0x00, 0xAA, 0x12, 0x34],
            [0x00, 0x07, 0x00, 0xBB, 0x55, 0x12, 0x34],
            [0x01, 0x01, 0x00, 0xAA, 0x66, 0x12, 0x34],
            [0x00, 0x09, 0x00, 0xCC],
        ] {
            var cursor = SceCursor(bytes)
            let frame = try CodecChainHasTagged.decode(&cursor)
            var sink = BytesSink()
            try frame.encode(into: &sink)
            XCTAssertEqual(sink.bytes, bytes)
        }
    }

    // Codec `codec_chain_has_marker`: the entries are Zenoh extensions, whose identifier is the header
    // byte without its continuation flag Z (bit 7). A header with E set puts the chain on the wire.

    func testAnIdentifierThatLeavesAFlagOutIsTheSameWhetherOrNotTheFlagIsSet() throws {
        // The marker is 0x12. Its header with Z set (0x92) is still the marker, then an entry that closes
        // the chain; the marker puts a slice count on the wire (here none), and no payload.
        var cursor = SceCursor([0x80, 0x92, 0x01, 0x00])
        let frame = try CodecChainHasMarker.decode(&cursor)
        XCTAssertEqual(frame.extensions?.count, 2)
        XCTAssertEqual(frame.slice_count, 0)
        XCTAssertEqual(frame.slices?.count, 0)
        XCTAssertNil(frame.payload_len)
        XCTAssertNil(frame.payload)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testWithoutTheMarkerThePayloadIsOnTheWireInsteadOfTheSlices() throws {
        var cursor = SceCursor([0x80, 0x01, 0x02, 0xAA, 0xBB])
        let frame = try CodecChainHasMarker.decode(&cursor)
        XCTAssertEqual(frame.extensions?.count, 1)
        XCTAssertEqual(frame.payload_len, 2)
        XCTAssertEqual(frame.payload, [0xAA, 0xBB])
        XCTAssertNil(frame.slice_count)
        XCTAssertNil(frame.slices)
    }

    func testAChainThatIsNotOnTheWireHasNoEntryToAsk() throws {
        // E clear: no chain, so no marker, so the payload.
        var cursor = SceCursor([0x00, 0x01, 0xCC])
        let frame = try CodecChainHasMarker.decode(&cursor)
        XCTAssertNil(frame.extensions)
        XCTAssertEqual(frame.payload, [0xCC])
    }

    func testAnEncodeOfAnAbsentChainAsksNoEntries() throws {
        var cursor = SceCursor([0x00, 0x01, 0xCC])
        let frame = try CodecChainHasMarker.decode(&cursor)
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x00, 0x01, 0xCC])
    }
}
