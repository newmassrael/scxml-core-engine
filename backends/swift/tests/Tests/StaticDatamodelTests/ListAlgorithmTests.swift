// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The algorithms `sce-codegen generate -l swift` writes that build a list and hand it
// back (`scripts/regen_static_datamodel_swift.sh`): the buffer is a local `Array` the
// body appends to, and what is returned is that array by value.
final class ListAlgorithmTests: XCTestCase {
    func testAListOfNumbersIsBuiltAndReturned() throws {
        XCTAssertEqual(try DayRun.dayRun(3, 4), [3, 4, 5, 6])
        XCTAssertEqual(try DayRun.dayRun(9, 0), [])
    }

    // A sum that does not fit its width is the checked addition's failure, thrown from
    // the loop that was building the list: the caller gets no half-built list.
    func testAFailureWhileBuildingThrowsAndReturnsNoList() {
        XCTAssertThrowsError(try DayRun.dayRun(250, 10)) { error in
            XCTAssertEqual(error as? AlgorithmError, .overflow)
        }
    }

    func testTheSameValueIsAppendedAsOftenAsAsked() throws {
        XCTAssertEqual(try DayRepeat.dayRepeat(7, 3), [7, 7, 7])
        XCTAssertEqual(try DayRepeat.dayRepeat(7, 0), [])
    }

    // The bound an algorithm declares for its list is a capacity, not a limit, on a
    // backend whose lists grow: the machine that takes the list holds its own
    // variable's bound (`Checked.within`).
    func testAListPastTheDeclaredCapacityGrows() throws {
        XCTAssertEqual(try DayRepeat.dayRepeat(7, 6), Array(repeating: 7, count: 6))
    }

    func testARecordIsAppendedToAListOfRecords() throws {
        let entries = [SchemaEntryPayload(id: 1, amount: 10), SchemaEntryPayload(id: 2, amount: 20)]
        let added = try LedgerAdd.ledgerAdd(entries, 3, 30)
        XCTAssertEqual(added, entries + [SchemaEntryPayload(id: 3, amount: 30)])
    }

    func testAnEntryOfTheSameIdIsReplacedInPlace() throws {
        let entries = [
            SchemaEntryPayload(id: 1, amount: 10), SchemaEntryPayload(id: 2, amount: 20),
            SchemaEntryPayload(id: 3, amount: 30),
        ]
        let replaced = try LedgerAdd.ledgerAdd(entries, 2, -5)
        XCTAssertEqual(
            replaced,
            [
                SchemaEntryPayload(id: 1, amount: 10), SchemaEntryPayload(id: 2, amount: -5),
                SchemaEntryPayload(id: 3, amount: 30),
            ])
    }

    func testTheListHandedInIsNotWrittenThrough() throws {
        let entries = [SchemaEntryPayload(id: 1, amount: 10)]
        _ = try LedgerAdd.ledgerAdd(entries, 1, 99)
        XCTAssertEqual(entries, [SchemaEntryPayload(id: 1, amount: 10)])
    }

    func testAnEmptyListTakesTheFirstEntry() throws {
        XCTAssertEqual(
            try LedgerAdd.ledgerAdd([], 5, 50), [SchemaEntryPayload(id: 5, amount: 50)])
    }
}
