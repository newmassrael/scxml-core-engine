// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a delayed `<send>` still
// waiting is part of a machine's saved state — an event for this session's
// external queue, one for its internal queue (`#_internal`), and an act a
// host-served processor performs (W3C SCXML 6.2.5), which is saved with every
// field the document wrote so the handler a restored machine is given sees the
// request it would have seen had the process never died.
//
// The generated `save` / `restore` exist only for a `datamodel="sce-static"`
// document, and a host-served send needs a processor declaration the shared
// static fixtures do not carry. What this measures is the engine-level half,
// which does not depend on the data model, through the smallest machine that
// can be asked: the engine's own `savedState` / `beginRestore` / `enterSaved`,
// reached the way generated code reaches them. The Rust twin is
// `backends/rust/tests/tests/a_saved_delayed_host_send_is_performed_after_a_restore.rs`.
//
// Driven entirely on ManualClock: nothing here sleeps, and nothing can be
// decided by how loaded the build machine is.
//
// Also here, because a generated machine cannot reach it: a delayed send the
// saved state cannot carry — one routed to a parent — REFUSES the save instead
// of being left out. A document that makes one is generated without the save
// API, so the refusal is for a machine built by hand.

package com.sce.integration

import com.sce.runtime.EnabledTransition
import com.sce.runtime.Event
import com.sce.runtime.EventMetadata
import com.sce.runtime.HistoryId
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedAct
import com.sce.runtime.SavedSend
import com.sce.runtime.SavedState
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.StateRefusal
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

private object WaitingState : State

private sealed interface WaitEvent : Event {
    data object Probe : WaitEvent
    data object Beat : WaitEvent
}

/** The smallest machine whose delayed sends can be saved and restored. */
private class WaitingMachine : StateMachineEngine<WaitingState, WaitEvent>() {
    override val initialState: WaitingState = WaitingState

    /** The events the machine took, in order: what a delivery is observed by. */
    val taken = mutableListOf<WaitEvent>()

    override fun firstEnabledTransition(state: WaitingState, event: WaitEvent?): EnabledTransition<WaitingState, HistoryId>? {
        if (event != null) taken.add(event)
        return null
    }

    override fun onEntry(state: WaitingState, isDefaultEntry: Boolean) {}

    override fun onExit(state: WaitingState) {}

    override fun executeTransitionContent(source: WaitingState, transitionIndex: Int) {}

    override fun resolveState(stateId: String): WaitingState? = WaitingState.takeIf { stateId == "waiting" }

    override fun stateIdOf(state: WaitingState): String = "waiting"

    override fun resolveEventByName(name: String): WaitEvent? = when (name) {
        "probe" -> WaitEvent.Probe
        "beat" -> WaitEvent.Beat
        else -> null
    }

    override fun eventNameOf(event: WaitEvent): String? = when (event) {
        WaitEvent.Probe -> "probe"
        WaitEvent.Beat -> "beat"
    }

    // What a generated machine's send sites call, reached from the test.
    fun armExternal(sendId: String, delayMs: Long, data: String, origin: String) =
        scheduleSend(sendId, delayMs, WaitEvent.Probe, EventMetadata.external(sendId = sendId, origin = origin, data = data))

    fun armInternal(sendId: String, delayMs: Long, data: String) =
        scheduleInternalSend(sendId, delayMs, WaitEvent.Beat, EventMetadata.internal(data))

    fun armHost(delayMs: Long, request: StateMachineEngine.HostSendRequest) =
        scheduleHostSend(request.sendId, delayMs, request)

    fun armParent(sendId: String, delayMs: Long) =
        scheduleParentSend(sendId, delayMs, "report", "", WaitEvent.Probe)

    // What a generated machine's `save` and `restore` call.
    fun saveAt(wallNowMs: Long): SavedState = savedState(SHAPE, linkedMapOf(), wallNowMs)

    fun restoreAt(saved: SavedState, wallNowMs: Long) {
        beginRestore(saved, SHAPE)
        enterSaved(saved, wallNowMs)
    }

    companion object {
        /** Any text: the engine-level `save` records the shape it is told. */
        const val SHAPE = "shape"
    }
}

@DisplayName("A saved delayed send is delivered after a restore (§2.15)")
class ASavedDelayedSendIsDeliveredAfterARestoreTest {

    /** The wall-clock moment the machine is saved at. */
    private val savedAtMs = 1_700_000_000_000L

    private fun richRequest() = StateMachineEngine.HostSendRequest(
        processorType = "x-sce-host",
        eventName = "audit",
        target = "https://example.test/hook",
        content = "the body",
        // A repeated name keeps every value in document order.
        params = mapOf("kind" to listOf("first", "second"), "count" to listOf("3")),
        sendId = "rich",
        eventData = """{"kind":["first","second"],"count":["3"]}""",
        invokeId = "inv",
    )

    /**
     * A machine on host-owned time with a send of each kind armed: external
     * 100 ms, internal 150 ms, host-served 250 ms.
     *
     * [origin] is the session the external send names as its sender. A machine
     * with no script engine has no session, so what it delivers carries none —
     * and an event that names one is taken as a child's (§scxml-6.5), which
     * needs the script engine this machine does not have. A case that only
     * saves can name one; a case that delivers does not.
     */
    private fun armed(origin: String = ""): WaitingMachine {
        val sm = WaitingMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        sm.armExternal("p", 100, "d", origin)
        sm.armInternal("i", 150, "x")
        sm.armHost(250, richRequest())
        return sm
    }

    private fun savedThroughJson(sm: WaitingMachine): SavedState = SavedState.fromJson(sm.saveAt(savedAtMs).toJson())

    /** What a restored machine's handler was asked: the engine's reading of "now", and the request. */
    private class Calls {
        val asked = mutableListOf<Pair<Long, StateMachineEngine.HostSendRequest>>()
    }

    /** [saved] restored [elapsedMs] of wall-clock time after it was saved, on a clock starting at 0. */
    private fun <T> restored(saved: SavedState, elapsedMs: Long, body: (WaitingMachine, ManualClock, Calls) -> T): T {
        val sm = WaitingMachine()
        try {
            val clock = ManualClock(0)
            val calls = Calls()
            sm.clock = clock
            sm.registerEventProcessor("x-sce-host") { request ->
                calls.asked.add(clock.elapsedMs() to request)
                emptyList()
            }
            sm.restoreAt(saved, savedAtMs + elapsedMs)
            return body(sm, clock, calls)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aWaitingSendOfEachKindIsSavedAsTheMomentItComesDue() {
        val sm = armed(origin = "o")
        try {
            assertEquals(
                listOf(
                    SavedSend(savedAtMs + 100, SavedAct.Raise("probe", "d", "p", "o")),
                    SavedSend(savedAtMs + 150, SavedAct.Internal("beat", "x", "i", "")),
                    SavedSend(
                        savedAtMs + 250,
                        SavedAct.Host(
                            processorType = "x-sce-host",
                            event = "audit",
                            target = "https://example.test/hook",
                            content = "the body",
                            params = mapOf("count" to listOf("3"), "kind" to listOf("first", "second")),
                            sendId = "rich",
                            data = """{"kind":["first","second"],"count":["3"]}""",
                            invokeId = "inv",
                        ),
                    ),
                ),
                savedThroughJson(sm).pending,
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredSendIsDeliveredWhenItsMomentComesWithTheRequestItHad() {
        val sm = armed()
        val saved = try {
            savedThroughJson(sm)
        } finally {
            sm.cleanup()
        }
        // Back 50 ms after the save: the external send has 50 ms left, the
        // internal one 100 and the host-served one 200.
        restored(saved, 50) { machine, _, calls ->
            machine.advanceTimeMs(49)
            assertEquals(emptyList<WaitEvent>(), machine.taken, "nothing is due yet")
            machine.advanceTimeMs(1)
            assertEquals(listOf<WaitEvent>(WaitEvent.Probe), machine.taken, "the external send, at 50 ms")
            machine.advanceTimeMs(49)
            assertEquals(listOf<WaitEvent>(WaitEvent.Probe), machine.taken)
            machine.advanceTimeMs(1)
            assertEquals(listOf(WaitEvent.Probe, WaitEvent.Beat), machine.taken, "the internal send, at 100 ms")
            assertEquals(emptyList<Any>(), calls.asked, "no host-served send is due yet")

            machine.advanceTimeMs(99)
            assertEquals(0, calls.asked.size)
            machine.advanceTimeMs(1)
            assertEquals(1, calls.asked.size, "the host-served send, at 200 ms")
            val (at, request) = calls.asked.single()
            assertEquals(200L, at)
            assertEquals(richRequest(), request, "every field of the request crossed")
        }
    }

    @Test
    fun aSendAlreadyDueWhenTheMachineComesBackIsDeliveredInItsOrder() {
        val sm = armed()
        val saved = try {
            savedThroughJson(sm)
        } finally {
            sm.cleanup()
        }
        // A minute away: every wait ran out while the process was dead.
        restored(saved, 60_000) { machine, _, calls ->
            assertEquals(emptyList<WaitEvent>(), machine.taken, "nothing is delivered by restoring")
            machine.advanceTimeMs(0)
            assertEquals(
                listOf(WaitEvent.Probe, WaitEvent.Beat),
                machine.taken,
                "the order the saved machine would have delivered them in",
            )
            assertEquals(listOf(0L to richRequest()), calls.asked)
        }
    }

    @Test
    fun aMachineRestoredFromATextSavesThatTextAgain() {
        val sm = armed()
        val saved = try {
            savedThroughJson(sm)
        } finally {
            sm.cleanup()
        }
        restored(saved, 0) { machine, _, _ ->
            assertEquals(saved.toJson(), machine.saveAt(savedAtMs).toJson())
        }
    }

    /**
     * What a machine built by hand can hold that a generated one cannot: a
     * delayed send routed to a session the saved state does not carry. Leaving
     * it out would restore a machine that never delivers it and say nothing, so
     * the save is refused and says which send.
     */
    @Test
    fun aDelayedSendToTheParentRefusesTheSave() {
        val sm = WaitingMachine()
        try {
            sm.clock = ManualClock(0)
            sm.initialize()
            // A session its host started has no parent, but whether there is one
            // is judged when the entry comes due: the send is waiting.
            sm.armParent("to_parent", 300)
            val refusal = assertThrows(StateRefusal::class.java) { sm.saveAt(savedAtMs) }
            assertTrue(refusal.message!!.contains("to_parent"), refusal.message)
            assertTrue(refusal.message!!.contains("does not carry"), refusal.message)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aWaitingSendNamingAnEventTheDocumentLacksIsRefusedAndTheMachineIsLeftAsItWas() {
        val sm = armed()
        val saved = try {
            savedThroughJson(sm)
        } finally {
            sm.cleanup()
        }
        val bad = SavedState(
            shape = saved.shape,
            configuration = saved.configuration,
            current = saved.current,
            variables = saved.variables,
            history = saved.history,
            pending = listOf(SavedSend(savedAtMs, SavedAct.Raise("warp", "", "", ""))),
            external = saved.external,
        )
        val machine = WaitingMachine()
        try {
            machine.clock = ManualClock(0)
            val refusal = assertThrows(StateRefusal::class.java) { machine.restoreAt(bad, savedAtMs) }
            assertTrue(refusal.message!!.contains("pending[0]"), refusal.message)
            assertTrue(refusal.message!!.contains("warp"), refusal.message)
            // A refused restore leaves the machine as it was: never started.
            assertThrows(StateRefusal::class.java) { machine.saveAt(savedAtMs) }
        } finally {
            machine.cleanup()
        }
    }
}
