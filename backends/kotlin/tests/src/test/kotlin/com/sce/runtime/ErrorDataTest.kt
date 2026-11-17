// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0106 #111: the `_event.data` of an `error.*` event the engine
// raises is the record SCE_MESH.md section 10.7.1 pins, `errorName`, `reason`
// and `detail`, in the bytes every engine writes; the Rust twin is
// `event::tests::a_communication_code_names_its_event_and_its_detail_is_escaped`.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals

class ErrorDataTest {
    @Test
    fun `an execution code is raised as error execution with its detail`() {
        assertEquals(
            """{"errorName":"execution","reason":"assignmentFailed","detail":"<assign> to 'x' failed"}""",
            errorData(ErrorReason.ASSIGNMENT_FAILED, "<assign> to 'x' failed"),
        )
    }

    @Test
    fun `a communication code names its event and its detail is escaped`() {
        assertEquals("execution", errorName(ErrorReason.EXPRESSION_FAILED))
        assertEquals("execution", errorName(ErrorReason.PAYLOAD_REFUSED))
        assertEquals("communication", errorName(ErrorReason.TARGET_UNREACHABLE))
        assertEquals(
            """{"errorName":"communication","reason":"targetUnreachable","detail":"say \"hi\"\n"}""",
            errorData(ErrorReason.TARGET_UNREACHABLE, "say \"hi\"\n"),
        )
    }
}
