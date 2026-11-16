// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// A codec that asks its caller for a flag input
// (`tests/forge/resources/codec_zenoh_wireexpr.scxml`: a name and a suffix that is on the wire only
// when the input `N` is set; `codec_chain_flag_entry.scxml`: a short value when `wide` is clear and a
// long one when it is set; `codec_zenoh_decl_kexpr.scxml`, which hands its own input on;
// `codec_repeat_flag_carrier.scxml`, which hands each entry a bit of its own header). The fixtures hold
// no numerical oracle, so this is where the behaviour is held: the input is a parameter of
// `decode`, a gate reads it, an encode reads an input the value says off the value, and what
// embeds a codec hands it the input it was bound to.
final class CodecFlagInputUnitTests: XCTestCase {
    func testAFieldGatedByTheInputIsReadOnlyWhenTheInputIsSet() throws {
        var clear = SceCursor([0x05, 0x77])
        let bare = try CodecZenohWireexpr.decode(&clear, N: 0)
        XCTAssertEqual(bare.id, 5)
        XCTAssertNil(bare.suffix)
        XCTAssertEqual(clear.remaining, 1, "what follows the codec is the caller's")
        var set = SceCursor([0x05, 0x02, 0x68, 0x69])
        let named = try CodecZenohWireexpr.decode(&set, N: 1)
        XCTAssertEqual(named.suffix?.text, "hi")
        XCTAssertEqual(set.remaining, 0)
    }

    // The suffix is gated on `N` alone, so the value says `N` by holding the suffix or not
    // (docs/adr/0065 §6 R4a): the encode is handed no input and derives the length, and no value
    // contradicts the input.
    func testAnEncodeWritesTheGatedFieldsWhereTheValueHoldsThemAndDerivesTheLength() throws {
        let suffix = try XCTUnwrap(BoundedText<CodecZenohWireexpr.SuffixBound>("hi"))
        var sink = BytesSink()
        try CodecZenohWireexpr(id: 5, suffix: suffix).encode(into: &sink)
        XCTAssertEqual(sink.bytes, [0x05, 0x02, 0x68, 0x69])
        var bare = BytesSink()
        try CodecZenohWireexpr(id: 5, suffix: nil).encode(into: &bare)
        XCTAssertEqual(bare.bytes, [0x05])
    }

    func testTheValueSaysTheInputItsDecodeIsHanded() throws {
        let suffix = try XCTUnwrap(BoundedText<CodecZenohWireexpr.SuffixBound>("hi"))
        XCTAssertEqual(CodecZenohWireexpr.inputN(CodecZenohWireexpr(id: 5, suffix: suffix)), 1)
        XCTAssertEqual(CodecZenohWireexpr.inputN(CodecZenohWireexpr(id: 5, suffix: nil)), 0)
    }

    func testANegatedGateReadsTheInputTheOtherWay() throws {
        var narrow = SceCursor([0xAA, 0x07])
        let short = try CodecChainFlagEntry.decode(&narrow, wide: 0)
        XCTAssertEqual(short.short_value, 7)
        XCTAssertNil(short.long_value)
        var wide = SceCursor([0xAA, 0x01, 0x02])
        let long = try CodecChainFlagEntry.decode(&wide, wide: 1)
        XCTAssertNil(long.short_value)
        XCTAssertEqual(long.long_value, 0x0102)
    }

    func testAnEmbedderHandsOnTheInputItWasGiven() throws {
        var cursor = SceCursor([0x03, 0x09, 0x02, 0x61, 0x62])
        let frame = try CodecZenohDeclKexpr.decode(&cursor, N: 1)
        XCTAssertEqual(frame.id, 3)
        XCTAssertEqual(frame.wireexpr.id, 9)
        XCTAssertEqual(frame.wireexpr.suffix?.text, "ab")
        var plain = SceCursor([0x03, 0x09])
        XCTAssertNil(try CodecZenohDeclKexpr.decode(&plain, N: 0).wireexpr.suffix)
    }

    func testAListHandsEachEntryTheBitOfTheHeaderItWasBoundTo() throws {
        var wide = SceCursor([0x01, 0x02, 0xA1, 0x00, 0x05, 0xA2, 0x00, 0x06])
        let long = try CodecRepeatFlagCarrier.decode(&wide)
        XCTAssertEqual(long.entries.map(\.long_value), [5, 6])
        var narrow = SceCursor([0x00, 0x01, 0xA1, 0x09])
        XCTAssertEqual(try CodecRepeatFlagCarrier.decode(&narrow).entries.first?.short_value, 9)
    }
}
