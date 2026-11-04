// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The fence of a text template's untrusted hole (docs/adr/0087, section 3.1,
 * item 4): the value is put between two markers, and every occurrence of either
 * marker is first removed from it, so that the value cannot close the fence and
 * write outside it.
 *
 * The removal repeats, taking the leftmost occurrence of either marker (the
 * longer first where both start at one place), until neither occurs: a value
 * cannot rebuild a marker out of two halves around one that is removed. It ends
 * because each pass shortens the value. The markers are printable ASCII, each
 * one UTF-16 unit as it is one UTF-8 byte, and neither encoding holds an ASCII
 * unit inside another character, so removing them from the units of a `String`
 * removes what the other runtimes remove from its UTF-8 bytes.
 *
 * Holds no state and does no I/O, as every forge runtime object.
 */
public object TextTemplate {
    /**
     * [value] with every occurrence of [open] and [close] removed, by the rule
     * of the object. An empty marker is no marker (a template's fence holds
     * none; the build refuses it).
     */
    public fun removeMarkers(value: String, open: String, close: String): String {
        val longest = maxOf(open.length, close.length)
        val text = StringBuilder(value)
        var from = 0
        while (true) {
            var at = -1
            var marker = 0
            var i = from
            while (i < text.length && at < 0) {
                val opens = open.isNotEmpty() && text.startsWith(open, i)
                val closes = close.isNotEmpty() && text.startsWith(close, i)
                if (opens || closes) {
                    at = i
                    marker = when {
                        opens && closes -> longest
                        opens -> open.length
                        else -> close.length
                    }
                }
                i++
            }
            if (at < 0) {
                return text.toString()
            }
            text.deleteRange(at, at + marker)
            // Before at - (longest - 1) nothing changed and nothing matched, so a
            // marker the removal made can only start at or after it.
            from = maxOf(0, at - (longest - 1))
        }
    }
}
