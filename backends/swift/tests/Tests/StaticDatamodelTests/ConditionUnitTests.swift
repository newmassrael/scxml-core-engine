// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The conditions `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a named boolean guard is a static function of the caseless enum its own unit declares,
// called as `Pascal.camel(…)` in the same module.
final class ConditionUnitTests: XCTestCase {
    // `forge_call_high.scxml` holds `level > 20`: the boundary is above, not at or above.
    func testALevelIsHighWhereItIsAboveTwenty() {
        XCTAssertFalse(ForgeCallHigh.forgeCallHigh(0))
        XCTAssertFalse(ForgeCallHigh.forgeCallHigh(20))
        XCTAssertTrue(ForgeCallHigh.forgeCallHigh(21))
        XCTAssertTrue(ForgeCallHigh.forgeCallHigh(Int32.max))
        XCTAssertFalse(ForgeCallHigh.forgeCallHigh(Int32.min))
    }
}
