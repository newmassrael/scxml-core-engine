// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// A float is spelled the way ECMAScript spells it by every writer that spells one
// (ARCHITECTURE.md, "JSON Number Text"). The cases live in
// tests/json_text/real_text.json, read here where they are.

@Test func aFloatIsSpelledAsEcmascriptSpellsItByEveryWriter() throws {
    let cases = try sharedCases("json_text/real_text.json")
    #expect(cases.count >= 40, "the table lost cases: \(cases.count)")
    for c in cases {
        let name = try #require(c["name"] as? String)
        let bits = try #require(c["bits"] as? String)
        let text = try #require(c["text"] as? String)
        let pattern = try #require(UInt64(bits, radix: 16), "\(name): bits \(bits)")
        let got = numberText(Double(bitPattern: pattern))
        #expect(got == text, "\(name): wrote \(got), the table says \(text)")
    }
}

@Test func aFloatThatIsNotFiniteIsSpelledAsEcmascriptSpellsIt() {
    #expect(numberText(.nan) == "NaN")
    #expect(numberText(.infinity) == "Infinity")
    #expect(numberText(-.infinity) == "-Infinity")
}

@Test func eitherZeroIsZero() {
    #expect(numberText(0.0) == "0")
    #expect(numberText(-0.0) == "0")
}
