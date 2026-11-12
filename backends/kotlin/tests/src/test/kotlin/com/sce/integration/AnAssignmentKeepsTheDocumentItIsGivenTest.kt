// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: `<assign location="v" expr="_event.data"/>` leaves
// `v` holding the DOM a `<send>`'s XML `<content>` arrived as, and so does a
// member-path location — Kotlin AOT.
//
// The engine's `assign` runs `<location> = (<expr>)` itself, so the value
// never leaves it; the C++ engines once read a bare-name location's value out
// and wrote it back, which lost the document.
//
// Fixture: integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh kotlin-integration

package com.sce.integration

import com.sce.integration.an_assignment_keeps_the_document_it_is_given.AnAssignmentKeepsTheDocumentItIsGivenState
import com.sce.integration.an_assignment_keeps_the_document_it_is_given.AnAssignmentKeepsTheDocumentItIsGivenStateMachine
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.4 / B.2 — an `<assign>` keeps the document it is given (Kotlin AOT).
@DisplayName("AnAssignmentKeepsTheDocumentItIsGiven — W3C SCXML 5.4 / B.2")
class AnAssignmentKeepsTheDocumentItIsGivenTest {

    @Test
    fun bothLocationsHoldTheDocumentTheEventCarried() {
        val sm = AnAssignmentKeepsTheDocumentItIsGivenStateMachine(W3CTestBase.createEngine())
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(2000L)

        val reached = sm.terminalState
        val why = when (reached) {
            AnAssignmentKeepsTheDocumentItIsGivenState.NameLost ->
                "`<assign location=\"v\" expr=\"_event.data\"/>` did not leave the document in " +
                    "`v`: the value was read out of the data model and written back."
            AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost ->
                "`<assign location=\"holder.doc\" expr=\"_event.data\"/>` did not leave the " +
                    "document in `holder.doc`."
            AnAssignmentKeepsTheDocumentItIsGivenState.Kept -> ""
            else ->
                "settled in ${sm.currentState.value}, which is not a verdict state: its own " +
                    "`doc` event never arrived."
        }

        assertEquals(AnAssignmentKeepsTheDocumentItIsGivenState.Kept, reached, why)
    }
}
