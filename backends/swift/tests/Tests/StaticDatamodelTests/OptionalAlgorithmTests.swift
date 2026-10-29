// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The algorithms `sce-codegen generate -l swift` writes over a value that may be absent
// (`scripts/regen_static_datamodel_swift.sh`): the value is a `T?`, absent is `nil`, a
// test of presence is `== nil` and `!= nil`, a read with a default is `??`, and a read
// of a value proven present is `SceOptional.proven`. A value of zero is a value:
// presence is what is tested, never the value.
final class OptionalAlgorithmTests: XCTestCase {
    private func goal(_ id: UInt32, _ day: Int64?, _ weight: Int32 = 0) -> SchemaGoalPayload {
        SchemaGoalPayload(id: id, day: day, weight: weight)
    }

    func testARecordNotWrittenHasNoDay() {
        XCTAssertNil(SchemaGoalPayload().day)
        XCTAssertEqual(SchemaGoalPayload().id, 0)
    }

    func testTheGoalsWithNoDayAreCounted() throws {
        let goals = [goal(1, 10), goal(2, nil), goal(3, nil), goal(4, 0)]
        XCTAssertEqual(try GoalUndated.goalUndated(goals), 2)
        XCTAssertEqual(try GoalUndated.goalUndated([]), 0)
    }

    // A day of 0 is a day, so it is not counted as absent.
    func testADayOfZeroIsADayAndNotAnAbsentOne() throws {
        XCTAssertEqual(try GoalUndated.goalUndated([goal(1, 0)]), 0)
    }

    func testTheEarliestDayIsTheLeastOfThosePresentAndNilWhereNoneIs() {
        XCTAssertEqual(GoalEarliest.goalEarliest([goal(1, 30), goal(2, nil), goal(3, 12)]), 12)
        XCTAssertNil(GoalEarliest.goalEarliest([goal(1, nil), goal(2, nil)]))
        XCTAssertNil(GoalEarliest.goalEarliest([]))
    }

    func testTheEarliestDayCanBeZeroOrNegative() {
        XCTAssertEqual(GoalEarliest.goalEarliest([goal(1, 5), goal(2, 0)]), 0)
        XCTAssertEqual(GoalEarliest.goalEarliest([goal(1, 5), goal(2, -3)]), -3)
    }

    func testADayIsReadWithADefaultWhereItIsAbsent() throws {
        // `g.day ?? instead`: 10 + 7 + 0 + 7.
        let goals = [goal(1, 10), goal(2, nil), goal(3, 0), goal(4, nil)]
        XCTAssertEqual(try GoalDaySum.goalDaySum(goals, 7), 24)
        XCTAssertEqual(try GoalDaySum.goalDaySum([], 7), 0)
    }

    func testTheDaysUntilADayAreItsDistanceFromTodayAndNilWhereThereIsNone() throws {
        XCTAssertEqual(try GoalUntil.goalUntil(100, 130), 30)
        XCTAssertEqual(try GoalUntil.goalUntil(100, 100), 0)
        XCTAssertNil(try GoalUntil.goalUntil(100, nil))
    }

    // The difference is a checked subtraction, whose overflow is the failure the
    // function throws and not a trap.
    func testADistanceThatDoesNotFitIsAnOverflowThatIsThrown() {
        XCTAssertThrowsError(try GoalUntil.goalUntil(Int64.max, Int64.min)) { error in
            XCTAssertEqual(error as? AlgorithmError, .overflow)
        }
    }

    func testTheFirstItemAtOrAboveTheLeastIsPickedAsAnOptionalRecord() {
        let items = [
            SchemaItemPickPayload(id: 1, title: "a", count: 2),
            SchemaItemPickPayload(id: 2, title: "b", count: 5),
            SchemaItemPickPayload(id: 3, title: "c", count: 9),
        ]
        XCTAssertEqual(ItemPickFirst.itemPickFirst(items, 4), items[1])
        XCTAssertEqual(ItemPickFirst.itemPickFirst(items, 2), items[0])
        XCTAssertNil(ItemPickFirst.itemPickFirst(items, 10))
        XCTAssertNil(ItemPickFirst.itemPickFirst([], 0))
    }
}
