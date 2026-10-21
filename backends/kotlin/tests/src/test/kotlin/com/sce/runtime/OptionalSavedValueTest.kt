// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value that may be absent in a saved state (docs/adr/0055, section 4): an
// absent value is written as `null` and a present one as its type writes it, and
// read back the same way; a `null` where a value of `T` is held is refused by the
// reader of `T`.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class OptionalSavedValueTest {

    @Test
    fun `an absent value is written as null and a present one as its type writes it`() {
        val absent: Long? = null
        val due: Long? = 12L
        val count: UInt? = 7u
        assertNull(SavedValues.optional<Long>(absent) { SavedValues.of(it) })
        assertEquals("12", SavedValues.optional<Long>(due) { SavedValues.of(it) })
        assertEquals(Json.Number("7"), SavedValues.optional<UInt>(count) { SavedValues.of(it) })
    }

    @Test
    fun `a saved value that may be absent is read back as it was written`() {
        assertNull(SavedValues.optional(null, "due") { v, w -> SavedValues.int64(v, w) })
        assertEquals(12L, SavedValues.optional("12", "due") { v, w -> SavedValues.int64(v, w) })
        assertEquals(
            "soon",
            SavedValues.optional("soon", "title") { v, w -> SavedValues.string(v, w, 16) },
        )
    }

    @Test
    fun `a present value is held to its type, and null to an optional place`() {
        assertFailsWith<StateRefusal> {
            SavedValues.optional("soon", "due") { v, w -> SavedValues.int64(v, w) }
        }
        assertFailsWith<StateRefusal> { SavedValues.int64(null, "due") }
    }
}
