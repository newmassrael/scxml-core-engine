// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
        XCTAssertEqual(
            frame.entries.map(\.after_marker),
            [.notAfterMarker(short_value: 0x11), .notAfterMarker(short_value: 0x22), .afterMarker(long_value: 0x3344)])
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testOnlyTheEntryRightAfterTheMarkerFollowsIt() throws {
        // The marker, an entry after it, and then an entry two places after the marker.
        var cursor = SceCursor([0x00, 0x84, 0x11, 0x81, 0x22, 0x33, 0x03, 0x44])
        let frame = try CodecChainPrevCarrier.decode(&cursor)
        XCTAssertEqual(
            frame.entries.map(\.after_marker),
            [.notAfterMarker(short_value: 0x11), .afterMarker(long_value: 0x2233), .notAfterMarker(short_value: 0x44)])
    }

    func testTheFirstEntryHasNoEntryBeforeIt() throws {
        // Its identifier is the marker's, but nothing precedes it.
        var cursor = SceCursor([0x00, 0x04, 0x77])
        let frame = try CodecChainPrevCarrier.decode(&cursor)
        XCTAssertEqual(frame.entries.map(\.after_marker), [.notAfterMarker(short_value: 0x77)])
    }

    func testAnInputOfTheCarrierReachesEveryEntryAlongsideTheChainsOwn() throws {
        // The head's bit 0 is handed to every entry as `tagged`: each reads a tag byte.
        var cursor = SceCursor([0x01, 0x03, 0x55, 0x66])
        let frame = try CodecChainPrevCarrier.decode(&cursor)
        XCTAssertEqual(frame.entries.map(\.after_marker), [.notAfterMarker(short_value: 0x55)])
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

    // The value holds the chain and the fields its kinds decide as one arm of the gates
    // `entries.has(7)`, `entries.has(9)` and `header.wide` (docs/adr/0065 §6 R6b), each arm holding
    // the chain in a type of the kinds it says.

    /// A chain of an entry of type 7 and none of type 9, as the arm `has0x7` holds it.
    private typealias Entry = CodecChainHasTaggedEntry
    private typealias SevenNotNine = SceChain<
        Entry, CodecChainHasTagged.EntriesBound,
        SceKindsOf<CodecChainHasTagged.EntriesKind7<Entry>, SceNoKinds<Entry>>,
        SceKindsOf<CodecChainHasTagged.EntriesKind9<Entry>, SceNoKinds<Entry>>
    >

    func testAFieldWhoseGateAsksForAnEntryIsOnTheWireOnlyWhenOneIsThere() throws {
        // No entry of type 7 and a narrow header: no priority; no entry of type 9: a checksum.
        var plain = SceCursor([0x00, 0x01, 0x00, 0xAA, 0x12, 0x34])
        guard case .neither(_, let checksum) = try CodecChainHasTagged.decode(&plain).entries_has_0x7_has_0x9_header_wide
        else {
            return XCTFail("no kind and a narrow header is the arm of no gate")
        }
        XCTAssertEqual(checksum, 0x1234)

        // An entry of type 7: a priority.
        var seven = SceCursor([0x00, 0x07, 0x00, 0xBB, 0x55, 0x12, 0x34])
        guard case .has0x7(_, let priority, let checksumWith) = try CodecChainHasTagged.decode(&seven)
            .entries_has_0x7_has_0x9_header_wide
        else {
            return XCTFail("an entry of type 7 is the arm of that kind")
        }
        XCTAssertEqual(priority, 0x55)
        XCTAssertEqual(checksumWith, 0x1234)
    }

    func testTheGateIsAskedOfEveryEntryNotOnlyTheFirst() throws {
        var cursor = SceCursor([0x00, 0x01, 0x01, 0xAA, 0x07, 0x00, 0xBB, 0x55, 0x12, 0x34])
        let frame = try CodecChainHasTagged.decode(&cursor)
        guard case .has0x7(let entries, let priority, _) = frame.entries_has_0x7_has_0x9_header_wide else {
            return XCTFail("the second entry is of type 7")
        }
        XCTAssertEqual(entries.count, 2)
        XCTAssertEqual(priority, 0x55)
    }

    func testAClauseOnTheEntriesJoinsAClauseOnAFlag() throws {
        // A wide header puts the priority on the wire with no entry of type 7.
        var cursor = SceCursor([0x01, 0x01, 0x00, 0xAA, 0x66, 0x12, 0x34])
        guard case .wide(_, let priority, _) = try CodecChainHasTagged.decode(&cursor)
            .entries_has_0x7_has_0x9_header_wide
        else {
            return XCTFail("a wide header and no kind is the arm of the flag")
        }
        XCTAssertEqual(priority, 0x66)
    }

    func testANegatedClauseTakesTheFieldOffTheWireWhenAnEntryIsThere() throws {
        var cursor = SceCursor([0x00, 0x09, 0x00, 0xCC])
        let frame = try CodecChainHasTagged.decode(&cursor)
        guard case .has0x9 = frame.entries_has_0x7_has_0x9_header_wide else {
            return XCTFail("an entry of type 9 is the arm of that kind, which holds no checksum")
        }
        XCTAssertEqual(cursor.remaining, 0)
    }

    // A value whose fields and entries disagree is no value of its type: the arm's chain is made only
    // of entries that hold the kinds the arm says, so no encode refuses one.
    func testAnArmsChainIsMadeOnlyOfEntriesOfTheKindsItSays() throws {
        let plain = try XCTUnwrap(
            BoundedList<Entry, CodecChainHasTagged.EntriesBound>([Entry(entry_type: 1, ctl: 0, body: 0)]))
        XCTAssertNil(SevenNotNine(plain), "no entry of type 7")
        let both = try XCTUnwrap(
            BoundedList<Entry, CodecChainHasTagged.EntriesBound>([
                Entry(entry_type: 7, ctl: 0, body: 0), Entry(entry_type: 9, ctl: 0, body: 0),
            ]))
        XCTAssertNil(SevenNotNine(both), "an entry of type 9")
    }

    // The encode derives `header.wide` from the arm and the flag the chain ends on from where each
    // entry stands (docs/adr/0081 §3).
    func testAnEncodeDerivesTheFlagAndTheEndFromTheArm() throws {
        let seven = try XCTUnwrap(
            BoundedList<Entry, CodecChainHasTagged.EntriesBound>([Entry(entry_type: 7, ctl: 1, body: 0)]))
        let entries = try XCTUnwrap(SevenNotNine(seven))
        let value = CodecChainHasTagged(
            header: 0, entries_has_0x7_has_0x9_header_wide: .has0x7(entries: entries, priority: 1, checksum: 0x0102))
        var sink = BytesSink()
        try value.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x00, 0x07, 0x00, 0x00, 0x01, 0x01, 0x02])
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
        // The marker holds nothing but its identifier (docs/adr/0065 §6 R6a), so the value holds the
        // other entries and the marker's place.
        var cursor = SceCursor([0x80, 0x92, 0x01, 0x00])
        let frame = try CodecChainHasMarker.decode(&cursor)
        guard case .has0x12(let extensions, let slices) = frame.extensions_has_0x12 else {
            return XCTFail("the chain holds the marker")
        }
        XCTAssertEqual(extensions.place, 0)
        XCTAssertEqual(extensions.count, 2)
        XCTAssertEqual(
            extensions.rest, [CodecZenohExtEntry(header: 0x01, body: .codecZenohExtUnit(CodecZenohExtUnit()))])
        XCTAssertEqual(slices.count, 0)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testWithoutTheMarkerThePayloadIsOnTheWireInsteadOfTheSlices() throws {
        var cursor = SceCursor([0x80, 0x01, 0x02, 0xAA, 0xBB])
        let frame = try CodecChainHasMarker.decode(&cursor)
        guard case .notHas0x12(let extensions, let payload) = frame.extensions_has_0x12 else {
            return XCTFail("the chain holds no marker")
        }
        XCTAssertEqual(extensions?.count, 1)
        XCTAssertEqual(payload.bytes, [0xAA, 0xBB])
    }

    func testAChainThatIsNotOnTheWireHasNoEntryToAsk() throws {
        // E clear: no chain, so no marker, so the payload.
        var cursor = SceCursor([0x00, 0x01, 0xCC])
        let frame = try CodecChainHasMarker.decode(&cursor)
        guard case .notHas0x12(let extensions, let payload) = frame.extensions_has_0x12 else {
            return XCTFail("no chain holds no marker")
        }
        XCTAssertNil(extensions)
        XCTAssertEqual(payload.bytes, [0xCC])
    }

    // The encode writes the marker at the place the value holds, made of its identifier, and sets the
    // flag the chain ends on, on every entry but the last.
    func testAnEncodeWritesTheMarkerAtItsPlace() throws {
        let other = CodecZenohExtEntry(header: 0x01, body: .codecZenohExtUnit(CodecZenohExtUnit()))
        let placed = try XCTUnwrap(
            ScePlaced<
                CodecZenohExtEntry, CodecChainHasMarker.ExtensionsBound,
                CodecChainHasMarker.ExtensionsKind12<CodecZenohExtEntry>
            >(rest: [other], place: 1))
        let slices = try XCTUnwrap(BoundedList<CodecChainHasMarkerSlice, CodecChainHasMarker.SlicesBound>([]))
        let value = CodecChainHasMarker(header: 0x80, extensions_has_0x12: .has0x12(extensions: placed, slices: slices))
        var sink = BytesSink()
        try value.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x80, 0x81, 0x12, 0x00])
    }

    func testAnEncodeOfAnAbsentChainAsksNoEntries() throws {
        var cursor = SceCursor([0x00, 0x01, 0xCC])
        let frame = try CodecChainHasMarker.decode(&cursor)
        var sink = BytesSink()
        try frame.encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x00, 0x01, 0xCC])
    }
}
