// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A verdict is its validity and its reason, and two verdicts of one validity and one reason
// are one verdict.

@Test func aVerdictCarriesItsValidityAndItsReason() {
    let ok = ValidationResult(valid: true, reason: "")
    #expect(ok.valid)
    #expect(ok.reason.isEmpty)
    let refused = ValidationResult(valid: false, reason: "rpm_out_of_range")
    #expect(!refused.valid)
    #expect(refused.reason == "rpm_out_of_range")
}

@Test func twoVerdictsOfOneValidityAndOneReasonAreEqual() {
    #expect(
        ValidationResult(valid: false, reason: "a") == ValidationResult(valid: false, reason: "a"))
    #expect(
        ValidationResult(valid: false, reason: "a") != ValidationResult(valid: false, reason: "b"))
    #expect(
        ValidationResult(valid: true, reason: "") != ValidationResult(valid: false, reason: ""))
}
