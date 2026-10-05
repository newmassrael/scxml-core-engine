// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The value a hybrid <invoke srcexpr> computes is reduced to the stem of the
// document it names (docs/SCE_ACCEPTED_SUBSET.md §2.13), and every engine
// reduces it the same way. The cases live in tests/document_stem/document_stem.json,
// read here with this runtime's own parser, so `DocumentStem.of` is measured
// against the same table as every other engine's reader and the build's.

package com.sce.runtime

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class DocumentStemTest {
    @Test
    fun `a value is reduced to the one stem every engine reduces it to`() {
        val table = Json.parse(File("tests/document_stem/document_stem.json").readText()) as Map<*, *>
        val cases = table["cases"] as List<*>
        assertTrue(cases.size >= 15, "the table lost cases: ${cases.size}")
        for (case in cases) {
            val fields = case as Map<*, *>
            val name = fields["name"] as String
            val value = fields["value"] as String
            val stem = fields["stem"] as String
            assertEquals(stem, DocumentStem.of(value), "$name: \"$value\"")
        }
    }
}
