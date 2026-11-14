// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// A record whose field is a value of an enum, and the algorithms that take and give
// one (`scripts/regen_static_datamodel_swift.sh`): the field and the parameter are the
// unit the enum document is written as (`EnumItemStatus`), and a record is built whole,
// its status the one it is given (docs/adr/0099: no case stands for a value nobody
// wrote).
final class EnumFieldTests: XCTestCase {
    private func item(_ id: UInt32, _ status: EnumItemStatus) -> SchemaItemPayload {
        SchemaItemPayload(id: id, status: status)
    }

    func testARecordHoldsTheCaseItIsBuiltWith() {
        let built = item(4, .inProgress)
        XCTAssertEqual(built.id, 4)
        XCTAssertEqual(built.status, .inProgress)
        // `open` is the first variant the document declares, not the lowest number or
        // the one named first by Swift's own ordering of cases.
        XCTAssertEqual(EnumItemStatus.allCases[0], .open)
    }

    func testAStatusIsReplacedOnTheItemOfTheGivenId() throws {
        let items = [item(1, .open), item(2, .done)]
        XCTAssertEqual(
            try ItemStatusSet.itemStatusSet(items, 2, .inProgress),
            [item(1, .open), item(2, .inProgress)])
    }

    func testTheItemsOfAStatusAreCounted() throws {
        let items = [item(1, .open), item(2, .done), item(3, .open)]
        XCTAssertEqual(try ItemStatusCount.itemStatusCount(items, .open), 2)
        XCTAssertEqual(try ItemStatusCount.itemStatusCount(items, .done), 1)
        XCTAssertEqual(try ItemStatusCount.itemStatusCount(items, .inProgress), 0)
    }

    func testTheStatusOfAnItemIsReadAndAnAbsentOneIsTheFirstCase() {
        let items = [item(1, .done), item(2, .inProgress)]
        XCTAssertEqual(ItemStatusOf.itemStatusOf(items, 2), .inProgress)
        XCTAssertEqual(ItemStatusOf.itemStatusOf(items, 1), .done)
        XCTAssertEqual(ItemStatusOf.itemStatusOf(items, 9), .open)
    }

    func testAnEnumIsComparedByItsCaseAndNotByItsNumber() {
        // 5 is `in_progress`, and 1 is its position: the value is what the document
        // declares.
        XCTAssertEqual(EnumItemStatus(rawValue: 5), .inProgress)
        XCTAssertNil(EnumItemStatus(rawValue: 1))
    }
}
