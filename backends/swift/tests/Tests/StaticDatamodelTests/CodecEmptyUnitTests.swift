// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A codec that declares no field (`tests/forge/resources/codec_zenoh_keep_alive.scxml`) is a frame
// of no bytes, the body of a message whose tag is written outside it. Encoding it writes nothing,
// and decoding it moves the cursor by nothing and leaves what follows for the caller.
final class CodecEmptyUnitTests: XCTestCase {
    func testTheFrameIsNoBytes() {
        XCTAssertEqual(CodecZenohKeepAlive.frameLength, 0)
    }

    func testEncodingWritesNothing() throws {
        var sink = BytesSink()
        try CodecZenohKeepAlive().encode(into: &sink)
        XCTAssertEqual(sink.bytes, [])
    }

    func testDecodingMovesTheCursorByNothing() throws {
        var cursor = SceCursor([0x01, 0x02])
        let frame = try CodecZenohKeepAlive.decode(&cursor)
        XCTAssertEqual(frame, CodecZenohKeepAlive())
        XCTAssertEqual(cursor.remaining, 2)
    }

    func testDecodingAnEmptyInputIsNotAnError() throws {
        var cursor = SceCursor([])
        _ = try CodecZenohKeepAlive.decode(&cursor)
        XCTAssertEqual(cursor.remaining, 0)
    }
}
