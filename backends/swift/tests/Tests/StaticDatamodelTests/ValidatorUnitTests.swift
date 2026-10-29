// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The validators `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a class whose `validate` checks one set of inputs against the document's range, rate-of-change
// and plausibility rules, in that order, and answers the first rule an input fails. It keeps the
// last value of each input a rate-of-change rule reads, from the last check that passed.
final class ValidatorUnitTests: XCTestCase {
    private func ok() -> ValidationResult {
        ValidationResult(valid: true, reason: "")
    }

    private func refused(_ reason: String) -> ValidationResult {
        ValidationResult(valid: false, reason: reason)
    }

    // `validator_rpm_check.scxml`: rpm at most 8000, at most 500 from the last check, and 0 or an
    // engine that is not stopped.
    func testARateOfChangeIsHeldAgainstTheLastCheckThatPassed() {
        let validator = ValidatorRpmCheck()
        XCTAssertEqual(validator.validate(400, "RUN"), ok())
        XCTAssertEqual(validator.validate(800, "RUN"), ok())
        // 600 above the last value: refused, and the last value stays 800.
        XCTAssertEqual(validator.validate(1400, "RUN"), refused("rpm_rate_of_change_exceeded"))
        // 500 above 800 is not more than 500.
        XCTAssertEqual(validator.validate(1300, "RUN"), ok())
    }

    func testARangeIsCheckedBeforeARateOfChangeAndBothBeforePlausibility() {
        let validator = ValidatorRpmCheck()
        XCTAssertEqual(validator.validate(9000, "RUN"), refused("rpm_out_of_range"))
        XCTAssertEqual(validator.validate(400, "RUN"), ok())
        XCTAssertEqual(validator.validate(400, "STOP"), refused("plausibility_failed"))
        // A stopped engine at zero is plausible, but zero is 400 from the last value: it passes.
        XCTAssertEqual(validator.validate(0, "STOP"), ok())
    }

    func testAFreshValidatorKeepsNothingFromAnother() {
        XCTAssertEqual(ValidatorRpmCheck().validate(400, "RUN"), ok())
        XCTAssertEqual(ValidatorRpmCheck().validate(400, "RUN"), ok())
    }

    // `validator_signed_roc.scxml`: speed -100..500 within 50 of the last, altitude at most 50000
    // within 100 of the last.
    func testASignedAndARealRateOfChangeAreHeldToTheirBounds() {
        let validator = ValidatorSignedRoc()
        XCTAssertEqual(validator.validate(40, 0.0), ok())
        // 50 below is not more than 50.
        XCTAssertEqual(validator.validate(-10, 100.0), ok())
        XCTAssertEqual(validator.validate(-100, 100.0), refused("speed_rate_of_change_exceeded"))
        XCTAssertEqual(validator.validate(-10, 201.0), refused("altitude_rate_of_change_exceeded"))
    }

    func testASignedRangeHoldsBothEnds() {
        let validator = ValidatorSignedRoc()
        XCTAssertEqual(validator.validate(-101, 0.0), refused("speed_out_of_range"))
        XCTAssertEqual(validator.validate(501, 0.0), refused("speed_out_of_range"))
        XCTAssertEqual(validator.validate(0, 60_000.0), refused("altitude_out_of_range"))
    }

    // `forge_call_range.scxml`: level at most 30 and offset at least 0, each a bound of its own.
    func testEachBoundOfARangeIsItsOwnCheck() {
        let validator = ForgeCallRange()
        XCTAssertEqual(validator.validate(30, 0), ok())
        XCTAssertEqual(validator.validate(31, 0), refused("level_out_of_range"))
        XCTAssertEqual(validator.validate(0, -1), refused("offset_out_of_range"))
        XCTAssertEqual(validator.validate(Int32.min, Int32.max), ok())
    }
}
