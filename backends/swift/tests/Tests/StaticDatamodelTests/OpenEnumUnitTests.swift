// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The unit `sce-codegen generate -l swift` writes for an open enum document
// (`sce:strict-variants="false"`): a struct that holds any number of the carrier, declared
// by a variant or not. `enum_uds_nrc.scxml` is a diagnostic vocabulary an ECU may answer
// outside of, so a number no variant declares is a value of it and not an error, which a
// Swift enum cannot hold. The numerical conformance harness holds the declared variants to
// the oracle; none of the backends' fragments probes a number outside them, so this does.
final class OpenEnumUnitTests: XCTestCase {
    func testEachConstantHoldsTheNumberTheDocumentDeclares() {
        XCTAssertEqual(EnumUdsNrc.generalReject.rawValue, 16)
        XCTAssertEqual(EnumUdsNrc.conditionsNotCorrect.rawValue, 34)
        XCTAssertEqual(EnumUdsNrc.responsePending.rawValue, 120)
    }

    func testANumberNoVariantDeclaresIsAValueAndHasNoName() {
        let undeclared = EnumUdsNrc.fromUnderlying(200)
        XCTAssertEqual(EnumUdsNrc.toUnderlying(undeclared), 200)
        XCTAssertNil(undeclared.declaredName)
        // Not one of the declared values, whatever its number.
        XCTAssertNotEqual(undeclared, EnumUdsNrc.generalReject)
        XCTAssertEqual(EnumUdsNrc(rawValue: 200), undeclared)
    }

    func testADeclaredNumberIsTheVariantThatDeclaresIt() {
        XCTAssertEqual(EnumUdsNrc.fromUnderlying(16), .generalReject)
        XCTAssertEqual(EnumUdsNrc.fromUnderlying(16).declaredName, "generalReject")
        XCTAssertEqual(EnumUdsNrc.fromUnderlying(120).declaredName, "responsePending")
    }

    func testANameThatTheDocumentDeclaresIsReadBackAndOtherIsNot() {
        XCTAssertEqual(EnumUdsNrc(declaredName: "requestOutOfRange"), .requestOutOfRange)
        XCTAssertNil(EnumUdsNrc(declaredName: "REQUEST_OUT_OF_RANGE"))
        XCTAssertNil(EnumUdsNrc(declaredName: ""))
    }
}
