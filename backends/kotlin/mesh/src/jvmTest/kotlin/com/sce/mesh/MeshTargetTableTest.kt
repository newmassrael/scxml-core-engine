// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which <send target> values name a Mesh peer: tests/mesh/mesh_target_cases.json,
// the table the C++ core's SendHelper::isMeshTarget, the build and the Rust core
// read too. Read with this runtime's own JSON parser.

package com.sce.mesh

import com.sce.runtime.Json
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.fail

class MeshTargetTableTest {
    @Test
    fun theMeshTargetPredicateIsReadByTheSharedTable() {
        val root = System.getProperty("sce.repo.root") ?: fail("the build did not name the repository root")
        val table = Json.parse(File(root, "tests/mesh/mesh_target_cases.json").readText()) as Map<*, *>
        val cases = table["cases"] as List<*>
        assertTrue(cases.size >= 10, "the table lost cases: ${cases.size}")
        for (case in cases) {
            val fields = case as Map<*, *>
            val target = fields["target"] as String
            assertEquals(fields["peer"] as String?, meshPeer(target), target)
        }
    }
}
