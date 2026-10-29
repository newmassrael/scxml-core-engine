// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The filters `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a class whose `update` reads one sample and answers the filtered one, and whose `reset` forgets
// what it has seen. The units are of the conformance corpus, whose documents declare the
// window and the alpha.
final class FilterUnitTests: XCTestCase {
    // `filter_moving_average.scxml`: a mean over a window of five.
    func testAMovingAverageOfFiveIsTheMeanOfTheSamplesHeld() {
        let filter = FilterMovingAverage()
        XCTAssertEqual(filter.update(10), 10, accuracy: 1e-9)
        XCTAssertEqual(filter.update(20), 15, accuracy: 1e-9)
        XCTAssertEqual(filter.update(30), 20, accuracy: 1e-9)
        XCTAssertEqual(filter.update(40), 25, accuracy: 1e-9)
        XCTAssertEqual(filter.update(50), 30, accuracy: 1e-9)
        // The sixth sample takes the place of the first: 20 / 30 / 40 / 50 / 60.
        XCTAssertEqual(filter.update(60), 40, accuracy: 1e-9)
    }

    func testAMovingAverageForgetsItsSamplesOnReset() {
        let filter = FilterMovingAverage()
        _ = filter.update(100)
        _ = filter.update(200)
        filter.reset()
        XCTAssertEqual(filter.update(8), 8, accuracy: 1e-9)
    }

    // `filter_low_pass.scxml`: alpha 0.1. The first sample is the output.
    func testALowPassStartsAtItsFirstSampleAndMovesATenthTowardEachNext() {
        let filter = FilterLowPass()
        XCTAssertEqual(filter.update(10), 10, accuracy: 1e-9)
        XCTAssertEqual(filter.update(20), 11, accuracy: 1e-9)
        XCTAssertEqual(filter.update(20), 11.9, accuracy: 1e-9)
        filter.reset()
        XCTAssertEqual(filter.update(5), 5, accuracy: 1e-9)
    }

    // `filter_debounce.scxml`: a window of three. The output latches after three alike.
    func testADebounceLatchesAfterAWindowOfAlikeSamples() {
        let filter = FilterDebounce()
        XCTAssertTrue(filter.update(true))
        XCTAssertFalse(filter.update(false))
        XCTAssertFalse(filter.update(false))
        XCTAssertFalse(filter.update(false))
        // One sample that differs does not move a latched output.
        XCTAssertFalse(filter.update(true))
        XCTAssertFalse(filter.update(true))
        XCTAssertTrue(filter.update(true))
    }

    func testADebounceStartsAgainAfterAReset() {
        let filter = FilterDebounce()
        _ = filter.update(true)
        _ = filter.update(true)
        _ = filter.update(true)
        filter.reset()
        XCTAssertFalse(filter.update(false))
    }
}
