// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The length a template's length restriction holds a text to (docs/SCE_TEST_PROFILE.md §6.1, §11.4):
// ES 201 873-1 B.1.4.1 restricts the length of a `universal charstring` in characters, each one
// Unicode code point.
//
// Cross-language siblings: `SCE::Test::textLength` (C++, over UTF-8 bytes),
// `com.sce.runtime.test.textLength` (Kotlin, over UTF-16 units), `testprofile.TextLength` (Go, over
// UTF-8 bytes), `sce_runtime.testprofile.text_length` (Python, a text's `len`),
// `sce_test_text_length` (C11, over UTF-8 bytes); Rust needs no function, its count being
// `str::chars().count()`.

/// The TTCN-3 length of `text`: how many characters it holds, each one Unicode code point.
///
/// A generated template's `length-min` and `length-max` on a `string[N]` position compare this, and not
/// the text's size in UTF-8 bytes, which is what bounds it in SCE (docs/SCE_TEST_PROFILE.md §6.3), nor
/// its `count`: a Swift `String` counts its grapheme clusters, and a letter followed by a combining
/// mark is one cluster of two code points. A Swift text is valid Unicode, so it holds no surrogate
/// that is half of no pair, which a Kotlin text can.
public func textLength(_ text: String) -> Int {
    text.unicodeScalars.count
}
