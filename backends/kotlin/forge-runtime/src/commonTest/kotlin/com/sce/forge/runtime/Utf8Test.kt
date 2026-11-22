// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

/**
 * A text is counted in the bytes UTF-8 encodes it to, and a text no UTF-8
 * holds has no count, so the value class of a codec's text makes no value of
 * it (docs/adr/0081 §2).
 */
class Utf8Test {

    @Test
    fun eachCodePointCountsTheBytesUtf8WritesItIn() {
        val cases = listOf(
            "" to 0,
            "a" to 1,
            "\u007F" to 1,
            "\u0080" to 2,
            "߿" to 2,
            "ࠀ" to 3,
            "￿" to 3,
            "😀" to 4,
            "a\u00E9\u4E2D\uD83D\uDE00" to 1 + 2 + 3 + 4,
        )
        for ((text, size) in cases) {
            assertEquals(size, Utf8.size(text), "the UTF-8 size of ${text.length} UTF-16 units")
            // The count is the count of the bytes Kotlin's own encoder writes.
            assertEquals(text.encodeToByteArray().size, Utf8.size(text))
        }
    }

    @Test
    fun aLoneSurrogateHasNoUtf8AndSoNoCount() {
        for (text in listOf("\uD83D", "\uDE00", "a\uD83Db", "\uDE00\uD83D", "\uD83D\uD83D")) {
            assertNull(Utf8.size(text), "a lone surrogate among ${text.length} units")
        }
    }
}
