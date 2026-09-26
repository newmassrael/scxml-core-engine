// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7: a <donedata> pair that cannot be evaluated raises
// error.execution and is ignored; the done events are raised all the same —
// Kotlin AOT path.
//
// Measured 2026-09-27, this channel treated an empty `location` as a
// structural error that withheld done.state.<parent> while still raising the
// <parallel>'s done event.
//
// Fixture: integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_bad_donedata_pair_is_ignored_kotlin.sh

package com.sce.integration

import com.sce.integration.a_bad_donedata_pair_is_ignored.ABadDonedataPairIsIgnoredState
import com.sce.integration.a_bad_donedata_pair_is_ignored.ABadDonedataPairIsIgnoredStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ABadDonedataPairIsIgnored — W3C SCXML 5.7")
class ABadDonedataPairIsIgnoredTest {

    @Test
    fun theBadPairsAreDroppedAndTheDoneEventsStillArrive() {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel
        // machine. The run needs nothing from the host.
        val sm = ABadDonedataPairIsIgnoredStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val seen = "errors=${sm.errors()} shape=${sm.shape()} (wanted 2 / 1)"
        assertEquals(ABadDonedataPairIsIgnoredState.Done, sm.terminalState, "done.state.p must still arrive and carry the run to `done`. $seen")
        assertEquals(2L, sm.errors(), "each ignored pair raises its own error.execution. $seen")
        assertEquals(1L, sm.shape(), "done.state.r1 must carry the surviving pair and neither ignored one. $seen")
    }
}
