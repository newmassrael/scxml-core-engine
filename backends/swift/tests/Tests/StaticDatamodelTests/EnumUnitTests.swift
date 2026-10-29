// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The unit `sce-codegen generate -l swift` writes for an enum document
// (`scripts/regen_static_datamodel_swift.sh`): a closed enum over the document's own
// carrier. `enum_item_status.scxml` declares its values apart from the variants'
// positions, and spells `in_progress` with an underscore, so a value that crossed by
// its position or by Swift's own spelling of the case would not be the one the
// document names.
final class EnumUnitTests: XCTestCase {
    func testEachCaseHoldsTheNumberTheDocumentDeclares() {
        XCTAssertEqual(EnumItemStatus.open.rawValue, 2)
        XCTAssertEqual(EnumItemStatus.inProgress.rawValue, 5)
        XCTAssertEqual(EnumItemStatus.done.rawValue, 7)
        XCTAssertEqual(EnumItemStatus.allCases.count, 3)
    }

    func testAValueIsNamedByTheDocumentsSpellingNotTheCases() {
        XCTAssertEqual(EnumItemStatus.open.declaredName, "open")
        XCTAssertEqual(EnumItemStatus.inProgress.declaredName, "in_progress")
        XCTAssertEqual(EnumItemStatus.done.declaredName, "done")
    }

    func testANameThatTheDocumentDeclaresIsReadBackAndOtherIsNot() {
        XCTAssertEqual(EnumItemStatus(declaredName: "in_progress"), .inProgress)
        XCTAssertNil(EnumItemStatus(declaredName: "inProgress"))
        XCTAssertNil(EnumItemStatus(declaredName: "IN_PROGRESS"))
        XCTAssertNil(EnumItemStatus(declaredName: ""))
    }

    func testANumberIsReadAsTheVariantThatDeclaresItAndANumberNoVariantDeclaresIsNil() {
        XCTAssertEqual(EnumItemStatus.fromUnderlying(5), .inProgress)
        XCTAssertEqual(EnumItemStatus.toUnderlying(.done), 7)
        // 1 is the position of `in_progress`, not a value of the enum.
        XCTAssertNil(EnumItemStatus.fromUnderlying(1))
        XCTAssertNil(EnumItemStatus.fromUnderlying(0))
        XCTAssertNil(EnumItemStatus.fromUnderlying(255))
    }
}
