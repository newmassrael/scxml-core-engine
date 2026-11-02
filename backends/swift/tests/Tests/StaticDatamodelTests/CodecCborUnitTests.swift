// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A codec that writes one CBOR map (`tests/forge/resources/codec_cbor_map.scxml`, SCE_FORGE.md
// §4.6.1): an exact 16-byte `id` at key 0, a `source` text of at most 64 bytes at key 1, a `pattern`
// byte at key 3, a `payload` of at most 256 bytes at key 5, an optional `urgent` truth value at key 7
// and an optional `deadline` at key 12. The oracle holds the bytes of a map in key order; this is
// where the rest of the contract is held: a decode takes the keys in any order and skips one it does
// not declare, a key given twice or an entry of the wrong kind is no map it reads, a required entry the
// map lacks is refused, a value past its bound is refused on both sides, and a refused map consumes
// nothing and writes nothing.
final class CodecCborUnitTests: XCTestCase {
    private let id16 = [UInt8](repeating: 0xAB, count: 16)

    private func uint(_ value: UInt64) -> [UInt8] {
        var buffer: [UInt8] = []
        Cbor.writeUint(value, to: &buffer)
        return buffer
    }

    private func text(_ value: String) -> [UInt8] {
        var buffer: [UInt8] = []
        Cbor.writeText(value, to: &buffer)
        return buffer
    }

    private func bytes(_ value: [UInt8]) -> [UInt8] {
        var buffer: [UInt8] = []
        Cbor.writeBytes(value, to: &buffer)
        return buffer
    }

    /// A map of `pairs`, written in the order given.
    private func map(_ pairs: [(UInt64, [UInt8])]) -> [UInt8] {
        var buffer: [UInt8] = []
        Cbor.writeMapHead(UInt64(pairs.count), to: &buffer)
        for (key, value) in pairs {
            Cbor.writeUint(key, to: &buffer)
            buffer += value
        }
        return buffer
    }

    private func required() -> [(UInt64, [UInt8])] {
        [(0, bytes(id16)), (1, text("hi")), (3, uint(7)), (5, bytes([1, 2, 3]))]
    }

    private func value() -> CodecCborMap {
        CodecCborMap(source: "hi", id: id16, pattern: 7, payload: [1, 2, 3], deadline: nil, urgent: nil)
    }

    func testTheKeysAreTakenInAnyOrderAndAnUnknownOneIsSkipped() throws {
        var pairs = required()
        pairs.reverse()
        // A key this codec does not declare, with a value nested in an array.
        pairs.insert((99, [0x82, 0x01, 0x02]), at: 2)
        var cursor = SceCursor(map(pairs))
        let frame = try CodecCborMap.decode(&cursor)
        XCTAssertEqual(frame, value())
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAnOptionalEntryIsAbsentUnlessTheMapHoldsIt() throws {
        var cursor = SceCursor(map(required() + [(7, [0xF5]), (12, uint(70_000))]))
        let frame = try CodecCborMap.decode(&cursor)
        XCTAssertEqual(frame.urgent, true)
        XCTAssertEqual(frame.deadline, 70_000)
        var bare = SceCursor(map(required()))
        let absent = try CodecCborMap.decode(&bare)
        XCTAssertNil(absent.urgent)
        XCTAssertNil(absent.deadline)
    }

    func testAKeyGivenTwiceIsNoMapTheCodecReadsAndConsumesNothing() {
        let twice = map(required() + [(3, uint(8))])
        var cursor = SceCursor(twice)
        XCTAssertThrowsError(try CodecCborMap.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .cborMalformed)
        }
        XCTAssertEqual(cursor.remaining, twice.count)
    }

    func testARequiredEntryTheMapLacksIsRefused() {
        let lacking = map(Array(required().dropLast()))
        var cursor = SceCursor(lacking)
        XCTAssertThrowsError(try CodecCborMap.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .cborRequiredKeyMissing)
        }
        XCTAssertEqual(cursor.remaining, lacking.count)
    }

    func testAnEntryOfAnotherKindIsNotTheEntryTheCodecReads() {
        var pairs = required()
        pairs[2] = (3, text("7"))
        var cursor = SceCursor(map(pairs))
        XCTAssertThrowsError(try CodecCborMap.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .cborMalformed)
        }
    }

    func testAValuePastItsBoundIsRefusedOnADecode() {
        // The exact length of an id, the bytes of a source, the largest a pattern holds.
        for (index, replacement) in [
            (0, (UInt64(0), bytes([UInt8](repeating: 1, count: 15)))),
            (1, (UInt64(1), text(String(repeating: "a", count: 65)))),
            (2, (UInt64(3), uint(256))),
            (3, (UInt64(5), bytes([UInt8](repeating: 1, count: 257)))),
        ] {
            var pairs = required()
            pairs[index] = replacement
            var cursor = SceCursor(map(pairs))
            XCTAssertThrowsError(try CodecCborMap.decode(&cursor), "entry \(index)") { error in
                XCTAssertEqual(
                    error as? CodecError, index == 0 ? .cborWrongLength : .cborOutOfRange)
            }
        }
    }

    func testAMapCutShortNeedsMoreBytesAndAnIndefiniteOneIsRefused() {
        let whole = map(required())
        var short = SceCursor(Array(whole.dropLast()))
        XCTAssertThrowsError(try CodecCborMap.decode(&short)) { error in
            XCTAssertEqual(error as? CodecError, .needMoreBytes)
        }
        XCTAssertEqual(short.remaining, whole.count - 1)
        var indefinite = SceCursor([0xBF, 0xFF])
        XCTAssertThrowsError(try CodecCborMap.decode(&indefinite)) { error in
            XCTAssertEqual(error as? CodecError, .cborMalformed)
        }
    }

    func testAnUnknownValueNestedPastTheDepthIsRefused() {
        let deep = [UInt8](repeating: 0x81, count: Cbor.maxSkipDepth) + [0x00]
        var cursor = SceCursor(map(required() + [(99, deep)]))
        XCTAssertThrowsError(try CodecCborMap.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .cborTooDeep)
        }
    }

    func testTheEntriesAreWrittenInAscendingKeyOrderWhateverTheOrderTheyWereDeclaredIn() throws {
        var sink = BytesSink()
        try value().encode(into: &sink)
        XCTAssertEqual(sink.bytes, map(required()))
    }

    func testAnAbsentOptionalEntryIsNotWrittenNorCounted() throws {
        var full = value()
        full.deadline = 5
        full.urgent = false
        var sink = BytesSink()
        try full.encode(into: &sink)
        XCTAssertEqual(sink.bytes, map(required() + [(7, [0xF4]), (12, uint(5))]))
    }

    func testAValuePastItsBoundIsRefusedOnAnEncodeBeforeAByteIsWritten() {
        var sink = BytesSink()
        var wrongId = value()
        wrongId.id = [1, 2, 3]
        XCTAssertThrowsError(try wrongId.encode(into: &sink)) { error in
            XCTAssertEqual(error as? CodecError, .cborWrongLength)
        }
        var longText = value()
        longText.source = String(repeating: "é", count: 33)
        XCTAssertThrowsError(try longText.encode(into: &sink)) { error in
            XCTAssertEqual(error as? CodecError, .cborOutOfRange)
        }
        var longPayload = value()
        longPayload.payload = [UInt8](repeating: 0, count: 257)
        XCTAssertThrowsError(try longPayload.encode(into: &sink)) { error in
            XCTAssertEqual(error as? CodecError, .cborOutOfRange)
        }
        XCTAssertEqual(sink.bytes, [], "a refused map has written nothing")
    }

    func testADecodeTakesWhatAnEncodeWrote() throws {
        var full = value()
        full.deadline = UInt64.max
        full.urgent = true
        var sink = BytesSink()
        try full.encode(into: &sink)
        var cursor = SceCursor(sink.bytes)
        XCTAssertEqual(try CodecCborMap.decode(&cursor), full)
        XCTAssertEqual(cursor.remaining, 0)
    }
}
