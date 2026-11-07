// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The TTCN-3 length of a text (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201
// 873-1 B.1.4.1 counts a universal charstring in characters, so a character of
// two, three or four UTF-8 bytes is one, as a character of one byte is.
//
// The first six cases are the twins of the Rust module `text_length` in
// backends/rust/runtime/tests/a_text_length_is_its_count_of_characters.rs,
// named `TestTextLength_<Case>` in Pascal case, and of
// tests/test_runtime/TextLengthTest.cpp, and pin TextLength apart from the
// byte length `len`, which bounds a text in SCE. The last case holds what only
// a Go string has to: a byte that is no UTF-8.

package testprofile

import "testing"

func holdsCharacters(t *testing.T, text string, bytes, characters int) {
	t.Helper()
	if got := len(text); got != bytes {
		t.Errorf("len(%q) = %d, want %d", text, got, bytes)
	}
	if got := TextLength(text); got != characters {
		t.Errorf("TextLength(%q) = %d, want %d", text, got, characters)
	}
}

func TestTextLength_AnEmptyTextHoldsNoCharacter(t *testing.T) {
	holdsCharacters(t, "", 0, 0)
}

func TestTextLength_AnAsciiTextHoldsOneCharacterPerByte(t *testing.T) {
	holdsCharacters(t, "started", 7, 7)
}

// U+00E9, two bytes: C3 A9.
func TestTextLength_ACharacterOfTwoBytesIsOne(t *testing.T) {
	holdsCharacters(t, "café", 5, 4)
}

// U+20AC, three bytes: E2 82 AC.
func TestTextLength_ACharacterOfThreeBytesIsOne(t *testing.T) {
	holdsCharacters(t, "€5", 4, 2)
}

// U+1F600, four bytes: F0 9F 98 80.
func TestTextLength_ACharacterOfFourBytesIsOne(t *testing.T) {
	holdsCharacters(t, "a\U0001f600b", 6, 3)
}

// One character of each width in a row: ten bytes, four characters.
func TestTextLength_CharactersOfEveryWidthAreCountedTogether(t *testing.T) {
	holdsCharacters(t, "aé€\U0001f600", 10, 4)
}

// A byte that begins no UTF-8 sequence, a continuation byte with no lead byte
// before it, and a lead byte cut short are each one character, the U+FFFD a
// `range` over the string reads.
func TestTextLength_AByteThatIsNoUtf8IsOneCharacter(t *testing.T) {
	holdsCharacters(t, "a\xffb", 3, 3)
	holdsCharacters(t, "\x80", 1, 1)
	holdsCharacters(t, "\xe2\x82", 2, 2)
}
