// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A — the OTHER carrier, the Kotlin twin of the Rust
// `event_schema_native.rs` lifted cases, the Go `event_schema_lifted` package,
// the Python `test_event_schema_lifted.py`, the C11
// `c11_integration_event_schema_lifted` and the C++ `EventSchemaLiftedAotTest`.
//
// The committed SM (com/sce/integration/statechart_lifted/statechart_liftedSm.kt)
// is generated from sce-build/tests/fixtures/event_schema/statechart_lifted.scxml
// (regen: scripts/regen_event_schema_native_kotlin.sh).
//
// A schema'd event's typed payload is filled by ONE producer: the generated
// `raiseJobCompleted` seam. Every other producer — `<send>` with `<param>`, an
// invoke forwarding an event either way, autoforward, BasicHTTP, mesh — fills
// `EventMetadata.data`, and until 2026-09-22 a natively lowered guard could
// read nothing but the typed carrier. The same guard therefore answered
// differently depending on where its event came from.
//
// ⚠ What a refusal does is not a policy chosen here: it is what the SCRIPT
// ENGINE answers for the same guard on the same data (W3C SCXML 3.13, measured
// on this document). A native lowering that answered differently would make
// the optimisation observable, which is the one thing it may not be.

package com.sce.integration

import com.sce.integration.statechart_lifted.StatechartLiftedEvent
import com.sce.integration.statechart_lifted.StatechartLiftedState
import com.sce.integration.statechart_lifted.StatechartLiftedStateMachine
import com.sce.runtime.EventMetadata
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.10 + NL→IR Item C1 Path A — the typed `_event.data` view read
/// out of the data wire every producer but the inject seam fills.
@DisplayName("EventSchemaLifted — a typed _event.data guard reads the data wire too (Kotlin AOT)")
class EventSchemaLiftedTest {

    /// The fixture lowers its guard natively and carries no executable
    /// content, so the machine is built with no script engine at all.
    private fun machine(): StatechartLiftedStateMachine {
        val sm = StatechartLiftedStateMachine()
        sm.initialize()
        assertEquals(
            StatechartLiftedState.Waiting,
            sm.currentState.value,
            "initial state must be Waiting"
        )
        return sm
    }

    /// The event as every producer but the inject seam delivers it: its fields
    /// on the `data` wire, with no typed payload riding along.
    private fun StatechartLiftedStateMachine.deliverData(data: String) {
        send(
            StatechartLiftedEvent.Job.Completed,
            EventMetadata(data = data, type = "external")
        )
        tick()
    }

    @Test
    fun aPayloadOnTheDataWireFiresTheSameGuard() {
        val sm = machine()
        try {
            sm.deliverData("""{"elapsed_ms": 0}""")
            assertEquals(
                StatechartLiftedState.Done,
                sm.currentState.value,
                "a payload that arrived on the `data` wire must satisfy the same " +
                    "native guard the inject seam's typed payload does"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theInjectSeamStillFiresItsOwnGuard() {
        val sm = machine()
        try {
            sm.raiseJobCompleted(0u)
            sm.tick()
            assertEquals(
                StatechartLiftedState.Done,
                sm.currentState.value,
                "the typed inject seam must still fire the guard it was built for"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aValueOfAnotherTypeIsRefusedAsTheScriptEngineRefusesIt() {
        val sm = machine()
        try {
            sm.deliverData("""{"elapsed_ms": "nought"}""")
            assertEquals(
                StatechartLiftedState.Refused,
                sm.currentState.value,
                "a text where the schema declares a number must raise " +
                    "error.execution and leave the guard unfired"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anEventWithNoDataIsRefusedTheSameWay() {
        val sm = machine()
        try {
            sm.deliverData("")
            assertEquals(
                StatechartLiftedState.Refused,
                sm.currentState.value,
                "an event carrying no data cannot answer a guard that reads a " +
                    "field of it"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aFieldTheDataDoesNotNameIsRefused() {
        val sm = machine()
        try {
            sm.deliverData("""{"other": 0}""")
            assertEquals(
                StatechartLiftedState.Refused,
                sm.currentState.value,
                "data that names none of the schema's fields cannot answer the guard"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aPayloadTheGuardRejectsIsNotAnError() {
        val sm = machine()
        try {
            // The payload reads perfectly; the comparison is simply false.
            // Nothing failed, so nothing is raised — the machine waits.
            sm.deliverData("""{"elapsed_ms": 5}""")
            assertEquals(
                StatechartLiftedState.Waiting,
                sm.currentState.value,
                "a well-typed payload the guard rejects must not route the " +
                    "machine to the error handler"
            )
        } finally {
            sm.cleanup()
        }
    }
}
