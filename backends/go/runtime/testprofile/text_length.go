// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import "unicode/utf8"

// The length a template's length restriction holds a text to
// (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201 873-1 B.1.4.1 restricts the
// length of a `universal charstring` in characters, and a Go string is held as
// bytes, so `len` counts a character of two, three or four UTF-8 bytes two,
// three or four times.
//
// Cross-language siblings: `SCE::Test::textLength` (C++, over UTF-8 bytes),
// `com.sce.runtime.test.textLength` (Kotlin, over UTF-16 units); Rust needs no
// function, its count being `str::chars().count()`.

// TextLength is the TTCN-3 length of text: how many characters it holds.
//
// ES 201 873-1 B.1.4.1 counts the length of a `universal charstring` in
// characters, each one Unicode code point. A generated template's
// `length-min` and `length-max` on a `string[N]` position compare this, and not
// the text's size in UTF-8 bytes, which is what bounds it in SCE
// (docs/SCE_TEST_PROFILE.md §6.3).
//
// A Go string may hold bytes that are no UTF-8. Each such byte is counted as
// one character, the U+FFFD a `range` over the string reads it as, so a text a
// decoder took apart is counted as that decoder reads it rather than dropped.
// A text of a generated record is valid UTF-8, and the two counts agree on it.
func TextLength(text string) int {
	return utf8.RuneCountInString(text)
}
