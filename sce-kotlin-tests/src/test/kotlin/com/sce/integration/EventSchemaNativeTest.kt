// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A (EventSchema native lowering) — Kotlin compile+run gate,
// the twin of the Rust `tests/event_schema_native.rs`, the Go
// `event_schema_native` package, and the C11 `c11_integration_event_schema_native`
// tests.
//
// The committed SM (com/sce/integration/statechart_minimal/statechart_minimalSm.kt)
// is generated from sce-build/tests/fixtures/event_schema/statechart_minimal.scxml
// (regen: scripts/regen_event_schema_native_kotlin.sh). Because it compiles as
// part of this module, the generated payload data class, the type-erased
// `EventMetadata.typedPayload` carrier round-trip, and the per-event
// `raiseJobCompleted` inject seam are really type-checked.
//
// The transition guard `cond="_event.data.elapsed_ms === 0"` lowers to a native
// `pendingJobCompletedPayload != null && (…)` field comparison with NO script
// engine, so the machine is constructed WITHOUT a `ScxmlScriptEngine` (the
// MCU-relevant property: a typed-guard machine needs no JS/Lua engine — the
// no-arg constructor proves it). The per-event `raiseJobCompleted` seam binds
// the event name and the payload field value in one call.

package com.sce.integration

import com.sce.integration.statechart_bytes.StatechartBytesState
import com.sce.integration.statechart_bytes.StatechartBytesStateMachine
import com.sce.integration.statechart_minimal.StatechartMinimalState
import com.sce.integration.statechart_minimal.StatechartMinimalStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.10 + NL→IR Item C1 Path A — typed `_event.data` guard lowered to
/// a native Kotlin comparison (no script engine).
@DisplayName("EventSchemaNative — typed _event.data guard, no script engine (Kotlin AOT)")
class EventSchemaNativeTest {

    @Test
    fun typedPayloadGuardFiresNatively() {
        // No script-engine argument: needs_script_engine is false for a typed
        // guard, so the generated machine has a no-arg constructor.
        val sm = StatechartMinimalStateMachine()
        sm.initialize()
        assertEquals(
            StatechartMinimalState.Waiting,
            sm.currentState.value,
            "initial state must be Waiting"
        )

        // Per-event typed inject — elapsed_ms == 0 satisfies the native guard.
        sm.raiseJobCompleted(0u)
        sm.tick()

        try {
            assertEquals(
                StatechartMinimalState.Done,
                sm.currentState.value,
                "after raiseJobCompleted(0): elapsed_ms == 0 must fire the native " +
                    "typed `_event.data` guard and reach Done"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun typedPayloadGuardMissesOnNonzero() {
        val sm = StatechartMinimalStateMachine()
        sm.initialize()

        // Same event, a payload the guard rejects — the machine stays put.
        sm.raiseJobCompleted(5u)
        sm.tick()

        try {
            assertEquals(
                StatechartMinimalState.Waiting,
                sm.currentState.value,
                "after raiseJobCompleted(5): elapsed_ms == 5 must leave the machine " +
                    "in Waiting (native typed guard must not fire)"
            )
        } finally {
            sm.cleanup()
        }
    }

    // RFC rfc-eventschema-bytes-guard.md §6 — the bytes-field guard
    // `cond="_event.data.raw === 'ack'"` lowers to
    // `pending….raw.contentEquals("ack".toByteArray())`. Kotlin `==` on a
    // ByteArray is reference equality, so a regression to `==` would compare
    // identities (always false here) — only a runtime transition check
    // distinguishes that from the correct content comparison.
    @Test
    fun bytesPayloadGuardFiresOnMatch() {
        val sm = StatechartBytesStateMachine()
        sm.initialize()
        assertEquals(StatechartBytesState.Waiting, sm.currentState.value)

        sm.raiseSignalReceived("ack".toByteArray())
        sm.tick()

        try {
            assertEquals(
                StatechartBytesState.Done,
                sm.currentState.value,
                "raw == \"ack\" must fire the native bytes guard to Done"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun bytesPayloadGuardMissesOnNonmatch() {
        val sm = StatechartBytesStateMachine()
        sm.initialize()

        sm.raiseSignalReceived("no".toByteArray())
        sm.tick()

        try {
            assertEquals(
                StatechartBytesState.Waiting,
                sm.currentState.value,
                "raw == \"no\" must leave the machine in Waiting"
            )
        } finally {
            sm.cleanup()
        }
    }
}
