// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The algorithms `sce-codegen generate -l swift` writes that call another algorithm
// (`scripts/regen_static_datamodel_swift.sh`): the callee is a static member of the
// enum its own unit declares, called as `Pascal.camel(…)` in the same module, and a
// callee that may fail has its failure passed on by the caller.
final class CallAlgorithmTests: XCTestCase {
    func testACalleeThatMayFailGivesItsValue() throws {
        XCTAssertEqual(try DaysInMonth.daysInMonth(2023, 4), 30)
        XCTAssertEqual(try DaysInMonth.daysInMonth(2023, 2), 28)
        XCTAssertEqual(try DaysInMonth.daysInMonth(2024, 2), 29)
    }

    func testADayCountIsTakenFromTheDaysOfTheMonthsBeforeIt() throws {
        XCTAssertEqual(try DaysFromCivil.daysFromCivil(1970, 1, 1), 0)
        XCTAssertEqual(try DaysFromCivil.daysFromCivil(1970, 1, 2), 1)
        XCTAssertEqual(try DaysFromCivil.daysFromCivil(2000, 3, 1), 11017)
    }

    func testAMonthIsAddedAndTheDayIsHeldToTheLengthOfTheNewMonth() throws {
        XCTAssertEqual(
            try AddMonths.addMonths(2024, 1, 31, 1), CivilDatePayload(year: 2024, month: 2, day: 29))
        XCTAssertEqual(
            try AddMonths.addMonths(2023, 1, 31, 1), CivilDatePayload(year: 2023, month: 2, day: 28))
        XCTAssertEqual(
            try AddMonths.addMonths(2023, 12, 15, 1), CivilDatePayload(year: 2024, month: 1, day: 15))
        XCTAssertEqual(
            try AddMonths.addMonths(2024, 3, 31, -1), CivilDatePayload(year: 2024, month: 2, day: 29))
    }

    // A month outside 1 through 12 is the precondition the body states, thrown before
    // any callee is asked about it.
    func testAMonthOutsideTheYearIsAPrecondition() {
        XCTAssertThrowsError(try AddMonths.addMonths(2024, 13, 1, 1)) { error in
            XCTAssertEqual(error as? AlgorithmError, .precondition)
        }
    }

    private func cell(
        _ row: UInt64, _ col: UInt64, wall: Int64, counter: UInt32 = 0, node: UInt64 = 1,
        value: Int64 = 0
    ) -> LwwCellPayload {
        var c = LwwCellPayload()
        c.row = row
        c.col = col
        c.value = value
        c.wallTime = wall
        c.counter = counter
        c.nodeId = node
        return c
    }

    // A write to a cell is kept where its timestamp is the later, by the callee that
    // orders two timestamps, and a cell the list does not hold is added.
    func testALaterWriteReplacesTheCellInPlace() throws {
        let cells = [cell(1, 1, wall: 10, value: 1), cell(2, 2, wall: 10, value: 2)]
        let write = cell(1, 1, wall: 20, value: 9)
        XCTAssertEqual(try LwwWrite.lwwWrite(cells, write), [write, cells[1]])
    }

    func testAnEarlierWriteIsNotKept() throws {
        let cells = [cell(1, 1, wall: 20, value: 1)]
        let write = cell(1, 1, wall: 10, value: 9)
        XCTAssertEqual(try LwwWrite.lwwWrite(cells, write), cells)
    }

    func testACellTheListDoesNotHoldIsAdded() throws {
        let cells = [cell(1, 1, wall: 10, value: 1)]
        let write = cell(3, 4, wall: 5, value: 7)
        XCTAssertEqual(try LwwWrite.lwwWrite(cells, write), cells + [write])
        XCTAssertEqual(try LwwWrite.lwwWrite([], write), [write])
    }
}
