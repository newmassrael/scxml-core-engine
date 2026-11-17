// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

/**
 * Two lists of one count are made into one value only where they hold as many
 * entries as each other, as a frame writes one count for both (docs/adr/0065
 * §6 R5, docs/adr/0081 §2).
 */
class AlignedTest {

    /** A list of `size` entries, as the class of a codec's list counts it. */
    private data class Entries(override val size: Int) : Counted

    @Test
    fun twoListsOfOneCountAreOneValue() {
        val aligned = assertNotNull(Aligned.of(Entries(2), Entries(2)))
        assertEquals(2, aligned.first.size)
        assertEquals(2, aligned.second.size)
    }

    @Test
    fun twoListsOfDifferentCountsAreNoValue() {
        assertNull(Aligned.of(Entries(2), Entries(3)))
        assertNull(Aligned.of(Entries(0), Entries(1)))
    }

    @Test
    fun twoValuesOfTheSameListsAreEqual() {
        assertEquals(Aligned.of(Entries(1), Entries(1)), Aligned.of(Entries(1), Entries(1)))
        assertNotEquals(Aligned.of(Entries(1), Entries(1)), Aligned.of(Entries(0), Entries(0)))
    }
}
