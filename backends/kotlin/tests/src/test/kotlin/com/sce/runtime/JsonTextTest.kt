// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json, read here with this runtime's own
// parser, so `Json.quote` is measured against the same table as every other
// engine's writer.

package com.sce.runtime

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class JsonTextTest {
    @Test
    fun `a string is written in the one form every engine writes`() {
        val table = Json.parse(File("tests/json_text/string_escape.json").readText()) as Map<*, *>
        val cases = table["cases"] as List<*>
        assertTrue(cases.size >= 8, "the table lost cases: ${cases.size}")
        for (case in cases) {
            val fields = case as Map<*, *>
            val name = fields["name"] as String
            val text = fields["text"] as String
            val escaped = fields["escaped"] as String
            assertEquals("\"$escaped\"", Json.quote(text), name)
        }
    }
}
