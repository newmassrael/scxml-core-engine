// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// An entry of a CBOR map that is an open enumeration
// (`tests/forge/conformance/codec_cbor_open_enum/codec_cbor_open_enum.scxml`: `status` at key 1,
// required, and `detail` at key 2, optional, both over a one-byte carrier whose set is open). An open
// enum holds every number its carrier holds, declared by a variant or not, so the entry takes every
// number the map gives it that fits the carrier and writes it back as it was; only a number past the
// width of the carrier is refused, as for a closed enum.
final class CodecCborOpenEnumUnitTests: XCTestCase {
    private func uint(_ value: UInt64) -> [UInt8] {
        var buffer: [UInt8] = []
        Cbor.writeUint(value, to: &buffer)
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

    func testANumberNoVariantDeclaresIsTheValueOfTheEntry() throws {
        var cursor = SceCursor(map([(1, uint(0xFF))]))
        let frame = try CodecCborOpenEnum.decode(&cursor)
        XCTAssertEqual(frame.status.rawValue, 0xFF)
        XCTAssertNil(frame.status.declaredName)
        XCTAssertNil(frame.detail)
        XCTAssertEqual(cursor.remaining, 0)
    }

    func testANumberAVariantDeclaresIsThatVariant() throws {
        var cursor = SceCursor(map([(1, uint(16)), (2, uint(0x7F))]))
        let frame = try CodecCborOpenEnum.decode(&cursor)
        XCTAssertEqual(frame.status, .generalReject)
        XCTAssertEqual(frame.detail?.rawValue, 0x7F)
        XCTAssertNil(frame.detail?.declaredName)
    }

    func testAnEncodeWritesTheNumberTheValueHolds() throws {
        var value = CodecCborOpenEnum()
        value.status = EnumUdsNrc(rawValue: 0xFE)
        value.detail = .responsePending
        var sink = BytesSink()
        try value.encode(into: &sink)
        XCTAssertEqual(sink.bytes, map([(1, uint(0xFE)), (2, uint(120))]))
        var cursor = SceCursor(sink.bytes)
        XCTAssertEqual(try CodecCborOpenEnum.decode(&cursor), value)
    }

    func testANumberPastTheWidthOfTheCarrierIsRefusedAndConsumesNothing() {
        let bytes = map([(1, uint(256))])
        var cursor = SceCursor(bytes)
        XCTAssertThrowsError(try CodecCborOpenEnum.decode(&cursor)) { error in
            XCTAssertEqual(error as? CodecError, .cborOutOfRange)
        }
        XCTAssertEqual(cursor.remaining, bytes.count)
    }
}
