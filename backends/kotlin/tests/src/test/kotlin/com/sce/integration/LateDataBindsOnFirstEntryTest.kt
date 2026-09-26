// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3 / Appendix D enterStates, late binding: a state's <data> is
// bound on that state's FIRST entry and never again — Kotlin AOT path.
//
// `s` is entered, its `v` changed to 5, `s` left and entered again; its
// <onentry> records the `v` it sees each time. Measured 2026-09-26, this
// channel bound late data on every entry, so the second entry saw 1 again.
//
// Fixture: integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_late_data_binds_on_first_entry_kotlin.sh

package com.sce.integration

import com.sce.integration.late_data_binds_on_first_entry.LateDataBindsOnFirstEntryEvent
import com.sce.integration.late_data_binds_on_first_entry.LateDataBindsOnFirstEntryState
import com.sce.integration.late_data_binds_on_first_entry.LateDataBindsOnFirstEntryStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("LateDataBindsOnFirstEntry — W3C SCXML 5.3 / Appendix D enterStates")
class LateDataBindsOnFirstEntryTest {

    @Test
    fun aStateBindsItsDataOnlyOnItsFirstEntry() {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel
        // machine.
        val sm = LateDataBindsOnFirstEntryStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        assertEquals(LateDataBindsOnFirstEntryState.Idle, sm.currentState.value, "the run has to start in `idle`")

        // Enter `s`, change its `v`, leave it, enter it again — one step each.
        for (event in listOf(
            LateDataBindsOnFirstEntryEvent.Go,
            LateDataBindsOnFirstEntryEvent.Bump,
            LateDataBindsOnFirstEntryEvent.Back,
            LateDataBindsOnFirstEntryEvent.Go,
        )) {
            sm.send(event)
            sm.tick()
        }

        val seen = "entries=${sm.entries()} seen=${sm.seen()} contentSeen=${sm.contentSeen()} (wanted 2 / 15 / 7)"
        assertEquals(LateDataBindsOnFirstEntryState.S, sm.currentState.value, "the second `go` has to leave the machine in `s`. $seen")
        assertEquals(2L, sm.entries(), "both entries of `s` must have run. $seen")
        assertEquals(
            15L,
            sm.seen(),
            "`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: 11 is a " +
                "processor that binds late data on every entry, and no value at all one that never binds it. $seen",
        )
        assertEquals(7L, sm.contentSeen(), "`c` is bound from inline content, not an expr, and must be bound too. $seen")
    }
}
