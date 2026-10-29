// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The interpolations `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a table interpolated between its breakpoints, a static function of the caseless enum its own unit
// declares, over arrays it holds. An input outside an axis answers the value at the nearer end.
// The two-axis read is held by the runtime's own tests (`InterpolationTests`) and by the text the
// generator writes for a table of two axes (`a_swift_interpolation_unit.rs`): the fixtures this
// target compiles hold one table of one axis.
final class InterpolationUnitTests: XCTestCase {
    // `forge_call_limit.scxml`: a limit per engine speed, over 0 / 1000 / 2000 rpm and 100 / 150 / 120.
    func testALimitIsInterpolatedBetweenTheBreakpointsAndClampedOutsideThem() {
        XCTAssertEqual(ForgeCallLimit.lookup(0), 100.0, accuracy: 1e-9)
        XCTAssertEqual(ForgeCallLimit.lookup(500), 125.0, accuracy: 1e-9)
        XCTAssertEqual(ForgeCallLimit.lookup(1000), 150.0, accuracy: 1e-9)
        XCTAssertEqual(ForgeCallLimit.lookup(1500), 135.0, accuracy: 1e-9)
        XCTAssertEqual(ForgeCallLimit.lookup(2000), 120.0, accuracy: 1e-9)
        XCTAssertEqual(ForgeCallLimit.lookup(UInt16.max), 120.0, accuracy: 1e-9)
    }
}
