// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The observers `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a class that holds a hysteresis state for each monitor, whose `update` answers the events a sample
// raised in the order the monitors are written, and that raises nothing on a sample that changes no
// monitor. The unit is the document of `tests/forge/resources/observer_coolant.scxml`.
final class ObserverUnitTests: XCTestCase {
    private typealias Tag = ObserverCoolant.Tag

    // `warning` is entered above 110 and left below 100, and raises both ways; `critical` is entered
    // above 120 and left below 105, and raises only on entering.
    func testASampleInsideNoZoneRaisesNothing() {
        let observer = ObserverCoolant()
        XCTAssertTrue(observer.update(95.0).isEmpty)
        XCTAssertTrue(observer.update(110.0).isEmpty, "the threshold itself is not above it")
    }

    func testAMonitorRaisesOnEnteringAndOnLeaving() {
        let observer = ObserverCoolant()
        XCTAssertEqual(observer.update(111.0).asArray(), [Tag.emitWarning])
        XCTAssertEqual(observer.update(105.0).asArray(), [], "between the two thresholds the zone is kept")
        XCTAssertEqual(observer.update(99.0).asArray(), [Tag.clearWarning])
    }

    func testAMonitorRaisesOnceWhileItsZoneIsKept() {
        let observer = ObserverCoolant()
        XCTAssertEqual(observer.update(115.0).asArray(), [Tag.emitWarning])
        XCTAssertTrue(observer.update(116.0).isEmpty)
        XCTAssertTrue(observer.update(117.0).isEmpty)
    }

    func testTheEventsOfOneSampleAreInTheOrderTheMonitorsAreWritten() {
        let observer = ObserverCoolant()
        XCTAssertEqual(observer.update(125.0).asArray(), [Tag.emitWarning, Tag.emergencyShutdown])
    }

    // `critical` names no leave event, so it leaves its zone without saying so, and enters it again
    // on the next sample above its threshold.
    func testAMonitorWithNoLeaveEventLeavesQuietlyAndEntersAgain() {
        let observer = ObserverCoolant()
        XCTAssertEqual(observer.update(125.0).asArray(), [Tag.emitWarning, Tag.emergencyShutdown])
        XCTAssertEqual(observer.update(99.0).asArray(), [Tag.clearWarning])
        XCTAssertEqual(observer.update(125.0).asArray(), [Tag.emitWarning, Tag.emergencyShutdown])
    }

    // The hysteresis is the observer's, so two instances do not share a zone.
    func testTwoObserversHoldTheirOwnZones() {
        let first = ObserverCoolant()
        let second = ObserverCoolant()
        XCTAssertEqual(first.update(111.0).asArray(), [Tag.emitWarning])
        XCTAssertEqual(second.update(111.0).asArray(), [Tag.emitWarning])
    }
}
