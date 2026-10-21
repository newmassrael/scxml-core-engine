// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

/** The proven read of an optional value (docs/adr/0055), [SceOptional.proven]. */
class OptionalTest {

    @Test
    fun aPresentValueIsReadAsItIs() {
        val due: Long? = 12L
        assertEquals(12L, SceOptional.proven(due))
        val count: UInt? = 7u
        assertEquals(7u, SceOptional.proven(count))
        val title: String? = "soon"
        assertEquals("soon", SceOptional.proven(title))
    }

    /** No generated read reaches it, and it is no failure a statement receives. */
    @Test
    fun anAbsentValueIsNoAlgorithmFailure() {
        val due: Long? = null
        assertFailsWith<IllegalStateException> { SceOptional.proven<Long>(due) }
    }
}
