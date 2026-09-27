// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A <send> delay is read as one CSS2 time on every engine (ARCHITECTURE.md,
// "Durations (Single Source of Truth)"). The cases live in
// tests/durations/css2_time.json, read here with this runtime's own parser, so
// `SendHelper.parseDelayMs` is measured against the same table as every other
// engine's reader.

package com.sce.runtime

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class DurationTest {
    @Test
    fun `a delay is read as the one CSS2 time every engine reads`() {
        val table = Json.parse(File("tests/durations/css2_time.json").readText()) as Map<*, *>
        val cases = table["cases"] as List<*>
        assertTrue(cases.size >= 20, "the table lost cases: ${cases.size}")
        for (case in cases) {
            val fields = case as Map<*, *>
            val name = fields["name"] as String
            val text = fields["text"] as String
            val want = (fields["ms"] as Json.Number?)?.text?.toLong()
            assertEquals(want, SendHelper.parseDelayMs(text), "$name: \"$text\"")
        }
    }
}
