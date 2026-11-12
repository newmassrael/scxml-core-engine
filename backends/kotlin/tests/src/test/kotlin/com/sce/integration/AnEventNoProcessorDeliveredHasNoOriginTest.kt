// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor
// delivered has no origin — a host-posted event and a host processor's reply
// that name none read both `_event.origin` and `_event.origintype` as the empty
// string — Kotlin AOT path.
//
// The Kotlin binding used to invent an origin for an external event that had
// none: the receiving session's own location, and the SCXML processor's type
// (`EventMetadata.external()` defaulted it too). docs/adr/0090, section 3,
// removes both, and this is the run that would read them.
//
// Fixture:
// integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/materialize.sh kotlin-integration
// (generated with `--host-processor x-sce-host`, the type registered below).

package com.sce.integration

import com.sce.integration.an_event_no_processor_delivered_has_no_origin.AnEventNoProcessorDeliveredHasNoOriginState
import com.sce.integration.an_event_no_processor_delivered_has_no_origin.AnEventNoProcessorDeliveredHasNoOriginStateMachine
import com.sce.runtime.StateMachineEngine
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AnEventNoProcessorDeliveredHasNoOrigin — W3C SCXML 5.10.1")
class AnEventNoProcessorDeliveredHasNoOriginTest {

    /** The type the regeneration script declares to the build. */
    private val hostType = "x-sce-host"

    @Test
    fun anEventNoProcessorDeliveredHasNoOrigin() {
        val sm = AnEventNoProcessorDeliveredHasNoOriginStateMachine(W3CTestBase.createEngine())
        // The host's processor answers the document's send with a reply that names
        // no party, so the reply carries no origin. Registered before the machine
        // starts, as every host registers.
        var served = 0
        sm.registerEventProcessor(hostType) {
            served++
            listOf(StateMachineEngine.HostSendResponse("host.reply"))
        }
        try {
            sm.initializeOnVirtualTime()
            // The host raises an event of its own, naming no origin: the door a host
            // names an event by, with the metadata it gives one by default.
            sm.sendEventByName("posted")
            sm.driveOnVirtualTime(1000L)

            assertEquals(1, served, "the host's processor was not handed the document's send once")
            assertEquals(
                AnEventNoProcessorDeliveredHasNoOriginState.Done,
                sm.terminalState,
                "the run must end in `done`; parked in ${sm.currentState.value}",
            )
            assertEquals(1L, sm.postedBlank(), "the host-posted event read an origin it was not given")
            assertEquals(1L, sm.replyBlank(), "the host's reply read an origin it was not given")
        } finally {
            sm.cleanup()
        }
    }
}
