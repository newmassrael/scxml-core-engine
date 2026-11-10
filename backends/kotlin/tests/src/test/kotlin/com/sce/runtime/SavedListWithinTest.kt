// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A list of a type that states a lower count, read back from a saved state
// (docs/adr/0086, section 15): held to both its counts before an element is
// read, so a restore refuses a list a machine never holds as a StateRefusal,
// before the record that holds it is built (docs/adr/0087, "Landing order"
// item 8).

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class SavedListWithinTest {

    private val element = { saved: Any?, what: String -> SavedValues.uint8(saved, what) }

    @Test
    fun `a list within both its counts is read back element by element`() {
        assertEquals(
            listOf(1.toUByte(), 2.toUByte()),
            SavedValues.listWithin(listOf(Json.Number("1"), Json.Number("2")), "items", 1, 2, element),
        )
    }

    @Test
    fun `a list short of its lower count is refused before an element is read`() {
        val short = assertFailsWith<StateRefusal> {
            SavedValues.listWithin(emptyList<Any?>(), "items", 1, 2, element)
        }
        assertEquals("'items' holds 0 elements, fewer than the 1 its type declares", short.message)
    }

    @Test
    fun `a list past its bound and a value that is no list are refused`() {
        val past = assertFailsWith<StateRefusal> {
            SavedValues.listWithin(
                listOf(Json.Number("1"), Json.Number("2"), Json.Number("3")),
                "items",
                1,
                2,
                element,
            )
        }
        assertEquals("'items' holds 3 elements, past the 2 it is bounded by", past.message)
        assertFailsWith<StateRefusal> { SavedValues.listWithin("x", "items", 1, 2, element) }
    }
}
