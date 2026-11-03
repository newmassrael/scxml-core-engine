// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The entries of a CBOR map that are enumerations (`stdlib/mesh/envelope.scxml`, SCE_FORGE.md §4.6.1):
// `pattern` (key 3) and `datacontenttype` (key 4) are required, `rpc_status` (key 10) is optional. An
// enum entry is the number the document declares for a variant, read at the width of the enum's own
// carrier and converted by the enum's `fromUnderlying`: a number no variant declares, which the closed
// set refuses, is no value of the entry, and a refused map consumes nothing.
final class CodecCborEnumUnitTests: XCTestCase {
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

    private func map(_ pairs: [(UInt64, [UInt8])]) -> [UInt8] {
        var buffer: [UInt8] = []
        Cbor.writeMapHead(UInt64(pairs.count), to: &buffer)
        for (key, value) in pairs {
            Cbor.writeUint(key, to: &buffer)
            buffer += value
        }
        return buffer
    }

    /// The six required entries, `pattern` and `datacontenttype` as the numbers given.
    private func required(pattern: UInt64 = 2, codec: UInt64 = 1) -> [(UInt64, [UInt8])] {
        [
            (0, bytes(id16)), (1, text("a")), (2, text("b")),
            (3, uint(pattern)), (4, uint(codec)), (5, bytes([9])),
        ]
    }

    private func sample() -> Envelope {
        var value = Envelope()
        value.id = id16
        value.source = "a"
        value.event_type = "b"
        value.pattern = .rpcRequest
        value.datacontenttype = .json
        value.data = [9]
        return value
    }

    func testAnEnumEntryIsTheVariantTheDocumentDeclaresForTheNumber() throws {
        var cursor = SceCursor(map(required()))
        let frame = try Envelope.decode(&cursor)
        XCTAssertEqual(frame.pattern, .rpcRequest)
        XCTAssertEqual(frame.datacontenttype, .json)
        XCTAssertNil(frame.rpc_status)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testAnOptionalEnumEntryIsAbsentUnlessTheMapHoldsIt() throws {
        var cursor = SceCursor(map(required() + [(10, uint(3))]))
        XCTAssertEqual(try Envelope.decode(&cursor).rpc_status, .invalidArgument)
    }

    func testANumberInAHoleOfTheClosedSetIsRefusedAndConsumesNothing() {
        // 10 to 13 are reserved for the Stream patterns and no variant declares them.
        let bytes = map(required(pattern: 10))
        var cursor = SceCursor(bytes)
        XCTAssertThrowsError(try Envelope.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .cborOutOfRange)
        }
        XCTAssertEqual(cursor.remaining, bytes.count)
    }

    func testANumberPastTheWidthOfTheCarrierIsRefused() {
        // `pattern` is carried in 16 bits, `datacontenttype` in 8.
        var wide = SceCursor(map(required(pattern: 65_536)))
        XCTAssertThrowsError(try Envelope.decode(&wide)) { error in
            XCTAssertEqual(error as? CodecError, .cborOutOfRange)
        }
        var narrow = SceCursor(map(required(codec: 256)))
        XCTAssertThrowsError(try Envelope.decode(&narrow)) { error in
            XCTAssertEqual(error as? CodecError, .cborOutOfRange)
        }
    }

    func testAnEncodeWritesTheNumberTheDocumentDeclaresForTheVariant() throws {
        var value = sample()
        // `invokeStart` declares 14, not its position among the variants.
        value.pattern = .invokeStart
        value.datacontenttype = .cbor
        var sink = BytesSink()
        try value.encode(into: &sink)
        XCTAssertEqual(sink.bytes, map(required(pattern: 14, codec: 2)))
    }

    func testADecodeTakesWhatAnEncodeWrote() throws {
        var value = sample()
        value.rpc_status = .unavailable
        var sink = BytesSink()
        try value.encode(into: &sink)
        var cursor = SceCursor(sink.bytes)
        XCTAssertEqual(try Envelope.decode(&cursor), value)
        XCTAssertEqual(cursor.remaining, 0)
    }
}
