// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" — Kotlin half of the Rust
// suite's `a_saved_mesh_request_is_not_sent_again_after_a_restore.rs`.
//
// A Mesh request (`<invoke type="sce:mesh-rpc">`) has one request in flight, and the
// peer it reached may already have acted on it. A saved state holds the call as it
// was sent; a restore does NOT send it again, because the peer would act on it twice,
// and the router that carried it is gone with the process that saved it. The document
// is told on the first macrostep that the call was interrupted — `error.invoke.<id>`
// carrying `"interrupted"` — and sends a new request if it means to ask again.
//
// `statechart_static_mesh_request.scxml` asks from `asking`; `done.invoke.ask` and
// `error.invoke.ask` each leave it, counting in `answered` and `failed`.
//
// Driven entirely on `ManualClock`: no case sleeps. The shared instance
// `saved/statechart_static_mesh_request_asking.json` is the text the Rust suite
// writes and restores too.

package com.sce.integration

import com.sce.integration.statechart_static_mesh_request.StatechartStaticMeshRequestEvent
import com.sce.integration.statechart_static_mesh_request.StatechartStaticMeshRequestState
import com.sce.integration.statechart_static_mesh_request.StatechartStaticMeshRequestStateMachine
import com.sce.runtime.MESH_RPC_INVOKE_TYPE
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import com.sce.runtime.StateMachineEngine.HostInvokeRequest
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ASavedMeshRequestIsNotSentAgainAfterARestoreTest {

    /** The wall-clock moment the shared instance was saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private val shared: String = File(
        repoRoot(),
        "sce-build/tests/fixtures/host_processor/saved/statechart_static_mesh_request_asking.json",
    ).readText().trim()

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    /** The requests the router was handed, in order. */
    private class Router {
        val starts = mutableListOf<HostInvokeRequest>()

        fun registerOn(sm: StatechartStaticMeshRequestStateMachine) {
            sm.registerMeshRpcInvoker { event ->
                event.start?.let { starts.add(it) }
                null
            }
        }
    }

    private fun machine(): StatechartStaticMeshRequestStateMachine {
        val sm = StatechartStaticMeshRequestStateMachine()
        sm.clock = ManualClock(0)
        return sm
    }

    /** A machine whose request to `#motor` is in flight, the router having been handed it. */
    private fun <T> withAsking(body: (StatechartStaticMeshRequestStateMachine, Router) -> T): T {
        val sm = machine()
        val router = Router()
        try {
            router.registerOn(sm)
            sm.initialize()
            sm.send(StatechartStaticMeshRequestEvent.Go)
            sm.tick()
            return body(sm, router)
        } finally {
            sm.cleanup()
        }
    }

    /** [text] restored [elapsedMs] after it was saved, the router registered before the restore as a Kotlin host does it. */
    private fun <T> withRestored(
        text: String,
        elapsedMs: Long,
        body: (StatechartStaticMeshRequestStateMachine, Router) -> T,
    ): T {
        val sm = machine()
        val router = Router()
        try {
            router.registerOn(sm)
            sm.restore(SavedState.fromJson(text), savedAtMs + elapsedMs)
            return body(sm, router)
        } finally {
            sm.cleanup()
        }
    }

    private fun standingIn(sm: StatechartStaticMeshRequestStateMachine) = sm.snapshot.value.configuration

    @Test
    fun aRequestInFlightIsSavedAsTheCallItWas() {
        withAsking { sm, router ->
            assertEquals(1, router.starts.size, "the router was handed the request")

            val saved = sm.save(savedAtMs)
            val entry = saved.hostInvokes.single()
            assertEquals(MESH_RPC_INVOKE_TYPE, entry.processorType)
            assertEquals("ask", entry.invokeId)
            assertEquals("#motor", entry.src)
            // The router owns the deadline of a Mesh request, so the engine holds none.
            assertNull(entry.due)
            assertTrue(saved.pending.isEmpty(), "${saved.pending}")

            // The text the Rust suite writes for the same machine, byte for byte.
            assertEquals(shared, saved.toJson())
        }
    }

    @Test
    fun aRestoreAsksNothingOfTheRouterUntilTheMachineIsDriven() {
        withRestored(shared, 1000) { sm, router ->
            assertTrue(router.starts.isEmpty(), "the restore itself calls nobody")
            assertEquals(setOf<Any>(StatechartStaticMeshRequestState.Asking), standingIn(sm))
        }
    }

    @Test
    fun aRestoredRequestIsNotSentAgainAndTheDocumentIsToldItWasInterrupted() {
        withRestored(shared, 1000) { sm, router ->
            sm.tick()

            assertTrue(
                router.starts.isEmpty(),
                "the peer may have acted on the first request already, so no second one leaves: ${router.starts}",
            )
            assertEquals(1u, sm.failed, "error.invoke.ask arrived")
            assertEquals(0u, sm.answered)
            assertEquals(0u, sm.refused)
            assertEquals(setOf<Any>(StatechartStaticMeshRequestState.Done), standingIn(sm))
        }
    }

    @Test
    fun aCallIsInterruptedWheneverTheMachineComesBack() {
        // There is no moment after which the call is still worth sending again.
        for (elapsedMs in listOf(0L, 250L, 60_000L)) {
            withRestored(shared, elapsedMs) { sm, router ->
                sm.tick()
                assertTrue(router.starts.isEmpty(), "$elapsedMs ms: ${router.starts}")
                assertEquals(1u, sm.failed, "$elapsedMs ms after the save")
            }
        }
    }

    @Test
    fun aMachineOwingTheInterruptionSaysItNeedsATickNow() {
        // The interruption is told in the first macrostep, so a host driving the machine by
        // its wake-up query must not be told there is nothing to wait for.
        withRestored(shared, 1000) { sm, _ ->
            assertEquals(0L, sm.timeUntilNextScheduledMs())
        }
    }

    @Test
    fun aRestoredMachineSavedAgainBeforeItWasDrivenKeepsTheCall() {
        withRestored(shared, 1000) { sm, _ ->
            val entry = sm.save(savedAtMs + 1000).hostInvokes.single()
            assertEquals(MESH_RPC_INVOKE_TYPE, entry.processorType)
            assertEquals("ask", entry.invokeId)
        }
    }
}
