// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The unit `sce-codegen generate -l swift` writes for an event schema
// (`scripts/regen_static_datamodel_swift.sh`): the struct the schema's fields are,
// and the constants a host names the event by.
final class EventSchemaUnitTests: XCTestCase {
    func testAPayloadStartsAtTheZeroOfEveryField() {
        let payload = SchemaEntryPayload()
        XCTAssertEqual(payload.id, 0)
        XCTAssertEqual(payload.amount, 0)
        XCTAssertEqual(SchemaFramedPayload().frame, [])
    }

    func testAPayloadIsBuiltWholeFromItsFieldsInTheSchemasOrder() {
        let payload = SchemaEntryPayload(id: 7, amount: -3)
        XCTAssertEqual(payload.id, 7)
        XCTAssertEqual(payload.amount, -3)
    }

    func testAPayloadIsAValue() {
        var copy = SchemaEntryPayload(id: 1, amount: 2)
        let original = copy
        copy.amount = 5
        XCTAssertEqual(original.amount, 2)
        XCTAssertNotEqual(copy, original)
        XCTAssertEqual(original, SchemaEntryPayload(id: 1, amount: 2))
    }

    func testTheUnitNamesTheEventItsRevisionAndEachBound() {
        XCTAssertEqual(SchemaEntry.eventName, "ledger.entry")
        XCTAssertEqual(SchemaEntry.revision, 1)
        XCTAssertEqual(SchemaFramed.frameMaxSize, 8)
    }
}
