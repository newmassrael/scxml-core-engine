// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The UTF-8 a codec's text is counted and written in (docs/adr/0077 §3).
 *
 * A codec's text element is held to its bound in the bytes UTF-8 encodes it to,
 * as every backend counts a text's length on the wire, by the value class of
 * its bound that the codec's printed encode reads (docs/adr/0081 §2). A Kotlin
 * `String` is UTF-16 and may hold a lone surrogate, which no UTF-8 holds and
 * `encodeToByteArray` would write as a replacement character, so the class
 * makes no value of such a text: [size] answers `null` for it.
 */
object Utf8 {
    /**
     * How many bytes UTF-8 encodes [text] to, or `null` where [text] holds a
     * surrogate that is not one half of a pair, which no UTF-8 holds.
     */
    fun size(text: String): Int? {
        var size = 0
        var at = 0
        while (at < text.length) {
            val unit = text[at]
            when {
                unit.code < 0x80 -> size += 1
                unit.code < 0x800 -> size += 2
                unit.isHighSurrogate() -> {
                    if (at + 1 >= text.length || !text[at + 1].isLowSurrogate()) return null
                    // A pair is one code point past the basic plane: four bytes.
                    size += 4
                    at += 1
                }
                unit.isLowSurrogate() -> return null
                else -> size += 3
            }
            at += 1
        }
        return size
    }
}
