// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// The cases every runtime's removal of a fence's markers is held to (the Rust runtime's
// text_template tests): a value with no marker is kept whole; each marker is removed wherever
// it stands; a marker the removal rebuilds is removed too; and multi-byte text beside a marker
// is kept as it was.

@Test func everyMarkerIsRemovedUntilNoneIsLeft() {
    let cases: [(String, String)] = [
        ("plain words", "plain words"),
        ("a<<<TEXTb", "ab"),
        ("aTEXT>>>b<<<TEXTc", "abc"),
        ("<<<TE<<<TEXTXT", ""),
        ("TEXT>>TEXT>>>>", ""),
        ("\u{e9}<<<TEXT\u{fc}", "\u{e9}\u{fc}"),
        ("", ""),
    ]
    for (value, kept) in cases {
        #expect(TextTemplate.removeMarkers(value, open: "<<<TEXT", close: "TEXT>>>") == kept)
    }
}

@Test func aMarkerBeforeACombiningMarkIsRemovedAsItsBytes() {
    // "<" then U+0301 is one grapheme cluster to a `String`, and still the byte "<".
    #expect(TextTemplate.removeMarkers("a<<\u{301}", open: "<<", close: ">>") == "a\u{301}")
}

@Test func theLongerMarkerIsRemovedWhereBothStart() {
    #expect(TextTemplate.removeMarkers("abcd", open: "ab", close: "abc") == "d")
}

@Test func anEmptyMarkerIsNoMarker() {
    #expect(TextTemplate.removeMarkers("x>>y", open: "", close: ">>") == "xy")
    #expect(TextTemplate.removeMarkers("abc", open: "", close: "") == "abc")
}
