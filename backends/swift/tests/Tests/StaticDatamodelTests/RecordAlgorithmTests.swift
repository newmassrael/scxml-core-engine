// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The algorithms `sce-codegen generate -l swift` writes over a record of an event
// schema (`scripts/regen_static_datamodel_swift.sh`): the record is the struct the
// schema's own unit declares, taken and returned by value.
final class RecordAlgorithmTests: XCTestCase {
    func testAScalarIsReadFromARecord() {
        XCTAssertEqual(LedgerAmount.ledgerAmount(SchemaEntryPayload(id: 1, amount: -7)), -7)
        XCTAssertEqual(LedgerAmount.ledgerAmount(SchemaEntryPayload()), 0)
    }

    func testTheLaterVersionWinsAndItsTextIsTaken() {
        let older = TextSlotPayload(slot: 4, version: 1, text: "old")
        let newer = TextSlotPayload(slot: 4, version: 2, text: "new")
        let merged = TextSlotLatest.textSlotLatest(older, newer)
        XCTAssertEqual(merged, TextSlotPayload(slot: 4, version: 2, text: "new"))
    }

    func testAnEqualOrOlderVersionKeepsTheFirstRecord() {
        let first = TextSlotPayload(slot: 4, version: 3, text: "kept")
        XCTAssertEqual(
            TextSlotLatest.textSlotLatest(first, TextSlotPayload(slot: 4, version: 3, text: "tie")),
            first)
        XCTAssertEqual(
            TextSlotLatest.textSlotLatest(first, TextSlotPayload(slot: 4, version: 2, text: "old")),
            first)
    }

    func testTheArgumentsAreValuesTheAlgorithmDoesNotWriteThrough() {
        let a = TextSlotPayload(slot: 1, version: 1, text: "a")
        let b = TextSlotPayload(slot: 1, version: 2, text: "b")
        _ = TextSlotLatest.textSlotLatest(a, b)
        XCTAssertEqual(a, TextSlotPayload(slot: 1, version: 1, text: "a"))
        XCTAssertEqual(b, TextSlotPayload(slot: 1, version: 2, text: "b"))
    }

    // A text is compared by the bytes it holds, as the other backends do: two
    // spellings of one character are two texts, though Swift's own `==` takes them for
    // one.
    func testTwoSpellingsOfOneCharacterAreTwoTexts() {
        let composed = "\u{00E9}"
        let decomposed = "e\u{0301}"
        XCTAssertEqual(composed, decomposed)
        let first = TextSlotPayload(slot: 1, version: 1, text: composed)
        let second = TextSlotPayload(slot: 1, version: 2, text: decomposed)
        let merged = TextSlotLatest.textSlotLatest(first, second)
        XCTAssertEqual(Array(merged.text.utf8), Array(decomposed.utf8))
    }
}
