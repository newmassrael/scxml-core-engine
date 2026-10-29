// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A read of a value proven present answers the value itself, of whatever type the
// optional is over. An absent one is a fault of the proof and stops the program
// (`preconditionFailure`), which a test cannot observe without ending itself, so it is
// held by the doc of the function rather than here.

private struct Row: Equatable {
    var id: UInt32
}

@Test func provenAnswersTheNumberTheOptionalHolds() {
    let value: Int64? = 42
    #expect(SceOptional.proven(value) == 42)
    let zero: UInt8? = 0
    #expect(SceOptional.proven(zero) == 0)
}

@Test func provenAnswersTheOtherTypesAnOptionalIsOver() {
    let flag: Bool? = false
    #expect(SceOptional.proven(flag) == false)
    let text: String? = "kept"
    #expect(SceOptional.proven(text) == "kept")
    let row: Row? = Row(id: 9)
    #expect(SceOptional.proven(row) == Row(id: 9))
}

@Test func provenOfAPresentValueIsTheValueEvenWhenItIsTheFalsyOne() {
    // `0`, `""` and `false` are values, not absence: the read is by presence.
    let number: Int32? = 0
    let empty: String? = ""
    #expect(SceOptional.proven(number) == 0)
    #expect(SceOptional.proven(empty) == "")
}
