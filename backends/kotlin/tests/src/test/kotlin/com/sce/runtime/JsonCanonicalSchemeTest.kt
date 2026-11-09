// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The JSON Canonicalization Scheme (RFC 8785) of `Json.writeJcs`, which writes
// the test log of the test profile (docs/SCE_TEST_PROFILE.md §12.5). Each case
// is the twin of the Rust case of the same name in
// backends/rust/runtime/src/json.rs (`canonical_members_are_sorted_by_their_utf16_code_units`
// is `canonical members are sorted by their utf16 code units`), with the same
// value and the same bytes, so the two writers are held to one text.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class JsonCanonicalSchemeTest {

    /**
     * RFC 8785 section 3.2.3, the example of the RFC: members are sorted by
     * their names as UTF-16 code units, so U+1F600, whose first unit is
     * 0xD83D, comes before U+FB33, which an order of code points
     * ([Json.writeCanonical]) would put after it; U+0080 is written as it is,
     * since only U+0000 to U+001F are escaped.
     */
    @Test
    fun `canonical members are sorted by their utf16 code units`() {
        val value = linkedMapOf(
            "\u20ac" to "Euro Sign",
            "\r" to "Carriage Return",
            "\ufb33" to "Hebrew Letter Dalet With Dagesh",
            "1" to "One",
            "\ud83d\ude00" to "Emoji: Grinning Face",
            "\u0080" to "Control",
            "\u00f6" to "Latin Small Letter O With Diaeresis",
        )
        assertEquals(
            "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u0080\":\"Control\"," +
                "\"\u00f6\":\"Latin Small Letter O With Diaeresis\",\"\u20ac\":\"Euro Sign\"," +
                "\"\ud83d\ude00\":\"Emoji: Grinning Face\"," +
                "\"\ufb33\":\"Hebrew Letter Dalet With Dagesh\"}",
            Json.writeJcs(value),
        )
        assertTrue(
            Json.writeCanonical(value).endsWith("\"\ud83d\ude00\":\"Emoji: Grinning Face\"}"),
            "the order of code points puts U+1F600 last, which is why the scheme has a writer of its own",
        )
    }

    /**
     * RFC 8785 section 3.2.2.2: a text is escaped as `JSON.stringify` escapes
     * it, the five short forms, `\u00xx` in lower-case hex for the other
     * controls, and nothing else: not `/`, not U+007F, not a character past
     * U+FFFF.
     */
    @Test
    fun `a canonical text escapes only what json stringify escapes`() {
        assertEquals(
            "\"\\b\\t\\n\\f\\r\\\"\\\\\\u0000\\u001f\u007f/\ud83d\ude00\"",
            Json.writeJcs("\b\t\n\u000C\r\"\\\u0000\u001f\u007f/\ud83d\ude00"),
        )
    }

    /**
     * Nested lists and maps are written with no whitespace, each map's members
     * sorted at its own level, a list's items in their order.
     */
    @Test
    fun `a canonical value has no whitespace and sorts every level`() {
        val value = linkedMapOf(
            "b" to listOf(linkedMapOf("z" to null, "a" to false), "y", emptyList<Any>()),
            "a" to emptyMap<String, Any>(),
        )
        assertEquals("{\"a\":{},\"b\":[{\"a\":false,\"z\":null},\"y\",[]]}", Json.writeJcs(value))
    }

    /**
     * RFC 8785 section 3.2.2.3: a number is written as ECMAScript writes the
     * double it reads as; one past the range of a double is refused.
     */
    @Test
    fun `a canonical number is the double it reads as`() {
        for ((spelling, written) in listOf(
            "1E3" to "1000",
            "-0" to "0",
            "0.10" to "0.1",
            "1e21" to "1e+21",
            "1e-7" to "1e-7",
        )) {
            assertEquals(written, Json.writeJcs(Json.Number(spelling)), spelling)
        }
        val refused = assertFailsWith<Json.NotCanonical> { Json.writeJcs(Json.Number("1e400")) }
        assertTrue(refused.reason.contains("1e400"), refused.reason)
    }

    /**
     * What is no JSON value has no form in the scheme: a number not held as a
     * [Json.Number], a number in no JSON spelling, and a member named by
     * something other than a text, at whatever depth.
     */
    @Test
    fun `a value that is no json value is refused`() {
        val bare = assertFailsWith<Json.NotCanonical> { Json.writeJcs(listOf(7)) }
        assertTrue(bare.reason.contains("Int"), bare.reason)
        val spelled = assertFailsWith<Json.NotCanonical> { Json.writeJcs(Json.Number("0x10")) }
        assertTrue(spelled.reason.contains("0x10"), spelled.reason)
        val named = assertFailsWith<Json.NotCanonical> { Json.writeJcs(listOf(mapOf(1 to "one"))) }
        assertTrue(named.reason.contains("not a text"), named.reason)
    }
}
