// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals

/** The fence of an untrusted hole (docs/adr/0087, section 3.1), [TextTemplate.removeMarkers]. */
class TextTemplateTest {

    /**
     * The cases every runtime's removal is held to (the Rust runtime's
     * text_template tests): a value with no marker is kept whole; each marker
     * is removed wherever it stands; a marker the removal rebuilds is removed
     * too; and multi-byte text beside a marker is kept as it was.
     */
    @Test
    fun everyMarkerIsRemovedUntilNoneIsLeft() {
        val cases = listOf(
            "plain words" to "plain words",
            "a<<<TEXTb" to "ab",
            "aTEXT>>>b<<<TEXTc" to "abc",
            "<<<TE<<<TEXTXT" to "",
            "TEXT>>TEXT>>>>" to "",
            "é<<<TEXTü" to "éü",
            "" to "",
        )
        for ((value, kept) in cases) {
            assertEquals(kept, TextTemplate.removeMarkers(value, "<<<TEXT", "TEXT>>>"), value)
        }
    }

    /** Where both markers start at one place, the longer is removed. */
    @Test
    fun theLongerMarkerIsRemovedWhereBothStart() {
        assertEquals("d", TextTemplate.removeMarkers("abcd", "ab", "abc"))
    }

    /** An empty marker removes nothing, and the other is still removed. */
    @Test
    fun anEmptyMarkerIsNoMarker() {
        assertEquals("xy", TextTemplate.removeMarkers("x>>y", "", ">>"))
        assertEquals("abc", TextTemplate.removeMarkers("abc", "", ""))
    }
}
