// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The TTCN-3 length of a text (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201
// 873-1 B.1.4.1 counts a universal charstring in characters, so a character of
// two, three or four UTF-8 bytes is one, as a character of one byte is.
//
// The six cases are the twins of the Rust module `text_length` in
// backends/rust/runtime/tests/a_text_length_is_its_count_of_characters.rs and of
// tests/test_runtime/TextLengthTest.cpp, and pin `textLength` apart from the
// byte length, which bounds a text in SCE, and from a Kotlin text's `length`,
// its count of UTF-16 code units, which a character of four UTF-8 bytes counts
// twice. The last case holds what only a UTF-16 text has to: a surrogate that is
// half of no pair.

package com.sce.runtime.test

import kotlin.test.Test
import kotlin.test.assertEquals

class TextLengthTest {

    private fun utf8Bytes(text: String): Int = text.encodeToByteArray().size

    @Test
    fun `an empty text holds no character`() {
        assertEquals(0, textLength(""))
    }

    @Test
    fun `an ascii text holds one character per byte`() {
        assertEquals(7, textLength("started"))
    }

    // U+00E9, two bytes: C3 A9.
    @Test
    fun `a character of two bytes is one`() {
        val text = "café"
        assertEquals(5, utf8Bytes(text))
        assertEquals(4, textLength(text))
    }

    // U+20AC, three bytes: E2 82 AC.
    @Test
    fun `a character of three bytes is one`() {
        val text = "€5"
        assertEquals(4, utf8Bytes(text))
        assertEquals(2, textLength(text))
    }

    // U+1F600, four bytes: F0 9F 98 80; two UTF-16 units: D83D DE00.
    @Test
    fun `a character of four bytes is one`() {
        val text = "a😀b"
        assertEquals(6, utf8Bytes(text))
        assertEquals(4, text.length)
        assertEquals(3, textLength(text))
    }

    // One character of each width in a row: ten bytes, four characters.
    @Test
    fun `characters of every width are counted together`() {
        val text = "aé€😀"
        assertEquals(10, utf8Bytes(text))
        assertEquals(4, textLength(text))
    }

    // A high surrogate with no low one after it, and a low one with no high one
    // before it, each stand for one code point.
    @Test
    fun `a surrogate that is half of no pair is one character`() {
        assertEquals(3, textLength("a\uD83Db"))
        assertEquals(1, textLength("\uD83D"))
        assertEquals(2, textLength("\uDE00\uD83D"))
    }
}
