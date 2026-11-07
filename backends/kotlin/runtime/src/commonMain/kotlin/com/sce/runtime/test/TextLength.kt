// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The length a template's length restriction holds a text to
// (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201 873-1 B.1.4.1 restricts the
// length of a `universal charstring` in characters, and a Kotlin text is held
// as UTF-16 code units, so the two counts differ wherever a character takes two
// units.
//
// Cross-language siblings: `SCE::Test::textLength` (C++, over UTF-8 bytes);
// Rust needs no function, its count being `str::chars().count()`.

package com.sce.runtime.test

/**
 * The TTCN-3 length of [text]: how many characters it holds.
 *
 * ES 201 873-1 B.1.4.1 counts the length of a `universal charstring` in
 * characters, each one Unicode code point. A generated template's `length-min`
 * and `length-max` on a `string[N]` position compare this, and not the text's
 * size in UTF-8 bytes, which is what bounds it in SCE (docs/SCE_TEST_PROFILE.md
 * §6.3), nor its `length` in UTF-16 code units.
 *
 * A character past U+FFFF is a high surrogate followed by a low one, and is
 * counted once, so every code unit is counted but the low half of such a pair.
 * A surrogate that is not half of a pair is counted as one character, as the
 * code point it stands for. (Written out because the common standard library
 * has no `codePointCount`.)
 */
fun textLength(text: String): Int {
    var characters = 0
    var at = 0
    while (at < text.length) {
        val pair = text[at].isHighSurrogate() && at + 1 < text.length && text[at + 1].isLowSurrogate()
        at += if (pair) 2 else 1
        characters += 1
    }
    return characters
}
