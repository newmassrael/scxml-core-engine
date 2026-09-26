// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1: `_event.type` names the queue an event was taken from —
// Kotlin AOT path.
//
// The document queues an external event first and two internal ones after
// it. Measured 2026-09-26, this channel already typed events from their
// metadata; the fixture pins it.
//
// Fixture: integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_event_type_names_its_queue_kotlin.sh

package com.sce.integration

import com.sce.integration.event_type_names_its_queue.EventTypeNamesItsQueueState
import com.sce.integration.event_type_names_its_queue.EventTypeNamesItsQueueStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("EventTypeNamesItsQueue — W3C SCXML 5.10.1")
class EventTypeNamesItsQueueTest {

    @Test
    fun eachEventIsTypedByTheQueueItWasTakenFrom() {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel
        // machine. The document queues its own events; the run needs nothing
        // from the host.
        val sm = EventTypeNamesItsQueueStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val seen = "intCode=${sm.intCode()} sendCode=${sm.sendCode()} extCode=${sm.extCode()} " +
            "(1 internal, 2 external, 3 other; wanted 1 / 1 / 2)"
        assertEquals(EventTypeNamesItsQueueState.Done, sm.terminalState, "`ext` must carry the run to `done`. $seen")
        assertEquals(1L, sm.intCode(), "`int` came off the internal queue while `ext` waited on the external one. $seen")
        assertEquals(1L, sm.sendCode(), "a `<send target=\"#_internal\">` with a payload rides the internal queue too. $seen")
        assertEquals(2L, sm.extCode(), "`ext` came off the external queue. $seen")
    }
}
