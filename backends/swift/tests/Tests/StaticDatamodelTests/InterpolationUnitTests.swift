// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The interpolations `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a table interpolated between its breakpoints, a static function of the caseless enum its own unit
// declares, over arrays it holds. An input outside an axis answers the value at the nearer end.
// The tables of the conformance corpus are units of this target as the fixtures' are.
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

    // `interpolation_2d_bilinear.scxml`: rows by engine speed, columns by load, both read under the
    // names the signature declares.
    func testATableOfTwoAxesIsInterpolatedAlongBothAndClampedOutsideEither() {
        XCTAssertEqual(Interpolation2dBilinear.lookup(800, 10), 2.1, accuracy: 1e-9)
        XCTAssertEqual(Interpolation2dBilinear.lookup(3000, 100), 11.0, accuracy: 1e-9)
        // Half way along the first axis, at the first load: between 2.1 and 2.5.
        XCTAssertEqual(Interpolation2dBilinear.lookup(1000, 10), 2.3, accuracy: 1e-9)
        // Half way along the second axis, at the first speed: between 2.1 and 4.5.
        XCTAssertEqual(Interpolation2dBilinear.lookup(800, 30), 3.3, accuracy: 1e-9)
        // Outside either axis is the value at the nearer end.
        XCTAssertEqual(Interpolation2dBilinear.lookup(0, 0), 2.1, accuracy: 1e-9)
        XCTAssertEqual(Interpolation2dBilinear.lookup(UInt16.max, UInt8.max), 11.0, accuracy: 1e-9)
    }
}
