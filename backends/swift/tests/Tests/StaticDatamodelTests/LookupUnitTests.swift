// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The lookups `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a table read by exact match, a static function of the caseless enum its own unit declares.
// What a key the table does not list answers is the table's declaration: no value, which is an
// error, or a default.
final class LookupUnitTests: XCTestCase {
    // `forge_call_code.scxml` holds `on-miss="error"`: a gear the table lists answers its code,
    // and any other answers nothing, which the caller reads as the failure it is.
    func testATableThatErrorsOnAMissAnswersNothingForAKeyItDoesNotList() {
        XCTAssertEqual(ForgeCallCode.lookupCode(1), 100)
        XCTAssertEqual(ForgeCallCode.lookupCode(2), 200)
        XCTAssertNil(ForgeCallCode.lookupCode(0))
        XCTAssertNil(ForgeCallCode.lookupCode(3))
        XCTAssertNil(ForgeCallCode.lookupCode(255))
    }

    // `forge_call_ratio.scxml` holds `default="0"`: a key it does not list is the default, a value
    // like any other, and a listed key is its own.
    func testATableWithADefaultAnswersItForAKeyItDoesNotList() {
        XCTAssertEqual(ForgeCallRatio.lookupRatio(1), 35)
        XCTAssertEqual(ForgeCallRatio.lookupRatio(2), 20)
        XCTAssertEqual(ForgeCallRatio.lookupRatio(3), 14)
        XCTAssertEqual(ForgeCallRatio.lookupRatio(0), 0)
        XCTAssertEqual(ForgeCallRatio.lookupRatio(200), 0)
    }

    // `forge_call_weight.scxml` is keyed and answers by the variants of two enums, written by
    // the name each variant declares and never by its number: a variant the table does not
    // list (`done`) is a miss.
    func testATableOfEnumVariantsIsKeyedAndAnswersByTheVariants() {
        XCTAssertEqual(ForgeCallWeight.lookupWeight(.open), .light)
        XCTAssertEqual(ForgeCallWeight.lookupWeight(.held), .heavy)
        XCTAssertNil(ForgeCallWeight.lookupWeight(.done))
    }
}
