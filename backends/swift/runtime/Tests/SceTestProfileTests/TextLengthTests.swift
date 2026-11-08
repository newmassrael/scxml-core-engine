// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceTestProfile
import Testing

// The TTCN-3 length of a text (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201 873-1 B.1.4.1 counts a
// universal charstring in characters, so a character of two, three or four UTF-8 bytes is one, as a
// character of one byte is.
//
// The first six cases are the twins of the Rust module `text_length` in
// backends/rust/runtime/tests/a_text_length_is_its_count_of_characters.rs, of
// tests/test_runtime/TextLengthTest.cpp and of the Kotlin `TextLengthTest`, and pin `textLength` apart
// from the byte length, which bounds a text in SCE. The last holds what only a Swift text has to: its
// `count` is of grapheme clusters, and a letter followed by a combining mark is one cluster of two
// characters. A Swift text holds no surrogate that is half of no pair, so the Kotlin case of one has no
// twin here. Every character outside ASCII is written as its escape.

private func utf8Bytes(_ text: String) -> Int {
    text.utf8.count
}

@Test func anEmptyTextHoldsNoCharacter() {
    #expect(textLength("") == 0)
}

@Test func anAsciiTextHoldsOneCharacterPerByte() {
    #expect(textLength("started") == 7)
}

// U+00E9, two bytes: C3 A9.
@Test func aCharacterOfTwoBytesIsOne() {
    let text = "caf\u{E9}"
    #expect(utf8Bytes(text) == 5)
    #expect(textLength(text) == 4)
}

// U+20AC, three bytes: E2 82 AC.
@Test func aCharacterOfThreeBytesIsOne() {
    let text = "\u{20AC}5"
    #expect(utf8Bytes(text) == 4)
    #expect(textLength(text) == 2)
}

// U+1F600, four bytes: F0 9F 98 80.
@Test func aCharacterOfFourBytesIsOne() {
    let text = "a\u{1F600}b"
    #expect(utf8Bytes(text) == 6)
    #expect(textLength(text) == 3)
}

// One character of each width in a row: ten bytes, four characters.
@Test func charactersOfEveryWidthAreCountedTogether() {
    let text = "a\u{E9}\u{20AC}\u{1F600}"
    #expect(utf8Bytes(text) == 10)
    #expect(textLength(text) == 4)
}

// `e` and U+0301, the combining acute accent: one grapheme cluster, two code points, three bytes.
@Test func aCombiningMarkIsACharacterOfItsOwn() {
    let text = "e\u{301}"
    #expect(text.count == 1)
    #expect(utf8Bytes(text) == 3)
    #expect(textLength(text) == 2)
}
