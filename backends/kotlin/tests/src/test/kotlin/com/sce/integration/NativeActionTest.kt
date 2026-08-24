// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML G.7 `<sce:action>` — Kotlin compile+run gate for native host
// dispatch.
//
// The committed machine is generated from
// `sce-build/tests/fixtures/event_schema/statechart_native_action.scxml`
// (regen: `scripts/regen_native_action.sh`), the same document the Rust, Go,
// Python, C++ and C11 channels drive. Because the tree is part of this module
// it is REALLY compiled: the generated class takes a
// `StatechartNativeActionActions` implementation as its first constructor
// parameter and carries no script engine at all, so this gate proves the
// engine-free dispatch surface compiles AND that the effects actually fire.
//
// What the scenarios measure:
//
//   * `appendFragmentPayload` reads two typed `_event.data` fields (a `bytes`
//     payload lowered to `ByteArray`, a `uint32` offset lowered to `UInt`);
//   * `resetSlot` takes no arguments;
//   * `onIdleEntry` / `onAssemblingExit` appear in NO transition, so they
//     prove the engine-free entry/exit path and that an eventless-only action
//     still gets a generated interface method;
//   * an event sent BY NAME carries no typed payload, and the arg-bearing
//     action must not fire against a zero value it would take for data. That
//     one is the half a configuration assertion cannot see — the machine
//     reaches `assembling` either way.

package com.sce.integration

import com.sce.integration.statechart_native_action.StatechartNativeActionActions
import com.sce.integration.statechart_native_action.StatechartNativeActionEvent
import com.sce.integration.statechart_native_action.StatechartNativeActionState
import com.sce.integration.statechart_native_action.StatechartNativeActionStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML G.7 — host operations a document dispatches without a script engine.
@DisplayName("NativeAction — W3C SCXML G.7")
class NativeActionTest {

    /// Host implementation of the generated operations. Records each dispatch
    /// so a test can assert the engine-free call path fired with the arguments
    /// the event carried.
    private class Recorder : StatechartNativeActionActions {
        val appended = mutableListOf<ByteArray>()
        val offsets = mutableListOf<UInt>()
        var resets = 0
        var idleEntries = 0
        var assemblingExits = 0

        override fun appendFragmentPayload(payload: ByteArray, offset: UInt) {
            // Copied: the array handed over is the machine's own storage, and a
            // test that kept the reference would assert on whatever the next
            // event wrote there.
            appended.add(payload.copyOf())
            offsets.add(offset)
        }

        override fun resetSlot() { resets++ }
        override fun onIdleEntry() { idleEntries++ }
        override fun onAssemblingExit() { assemblingExits++ }
    }

    // Not started here: `idle`'s `<onentry>` performs an act, so the host has
    // to be in place before `initialize()` — which is why it is a constructor
    // parameter and not a setter.
    private fun machine(host: Recorder) = StatechartNativeActionStateMachine(host)

    @Test
    fun nativeActionDispatchesTypedPayloadToHostInterface() {
        val host = Recorder()
        val sm = machine(host)
        sm.initialize()
        try {
            assertEquals(StatechartNativeActionState.Idle, sm.currentState.value)
            // `<onentry>` of the initial state fires on entry — the engine-free
            // entry-effect path, with no transition carrying the action.
            assertEquals(1, host.idleEntries, "onIdleEntry must fire on the initial entry to idle")

            // Per-event typed inject: `fragment.received` with a bytes payload
            // and an offset. The transition fires appendFragmentPayload.
            // `send` only queues on this engine; `tick` is what drains it.
            sm.raiseFragmentReceived("abc".toByteArray(), 7u)
            sm.tick()

            assertEquals(
                StatechartNativeActionState.Assembling,
                sm.currentState.value,
                "fragment.received must move idle -> assembling"
            )
            assertEquals(1, host.appended.size, "appendFragmentPayload fired ${host.appended.size} times")
            assertArrayEquals(
                "abc".toByteArray(),
                host.appended[0],
                "appendFragmentPayload must receive the typed _event.data payload natively"
            )
            assertEquals(listOf(7u), host.offsets.toList(), "the typed uint32 offset did not arrive")

            // `reset` fires the no-argument resetSlot and returns to idle.
            // Exiting `assembling` fires its `<onexit>` effect; re-entering
            // `idle` fires `<onentry>` a second time.
            sm.send(StatechartNativeActionEvent.Reset)
            sm.tick()

            assertEquals(StatechartNativeActionState.Idle, sm.currentState.value)
            assertEquals(1, host.resets, "resetSlot must have fired once")
            assertEquals(1, host.assemblingExits, "onAssemblingExit must fire when leaving assembling")
            assertEquals(2, host.idleEntries, "re-entering idle must fire its <onentry> again")
        } finally {
            sm.cleanup()
        }
    }

    /// An event sent by NAME carries no typed payload. The transition still
    /// fires — the guard is the event name — but the arg-bearing action has
    /// nothing to read, and handing the host a zeroed buffer it would take for
    /// data is the one outcome this seam must never produce.
    @Test
    fun nativeActionDoesNotFireWithoutItsTypedPayload() {
        val host = Recorder()
        val sm = machine(host)
        sm.initialize()
        try {
            sm.send(StatechartNativeActionEvent.Fragment.Received)
            sm.tick()

            assertEquals(
                StatechartNativeActionState.Assembling,
                sm.currentState.value,
                "the transition is guarded by the event name and must still be taken"
            )
            assertEquals(
                0,
                host.appended.size,
                "appendFragmentPayload fired without a typed payload to read"
            )
            // The eventless effects still ran: they read no payload, so nothing
            // about this delivery should have stopped them.
            assertEquals(1, host.idleEntries)
        } finally {
            sm.cleanup()
        }
    }
}
