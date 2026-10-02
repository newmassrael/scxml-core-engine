// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" — Kotlin half of the Rust
// suite's `a_generated_machines_waiting_host_send_survives_a_restore.rs`.
//
// A delayed `<send>` the HOST serves is a wait a saved machine carries, with the
// request it will make when the wait ends — measured through the machine a
// generator wrote. `ASavedDelayedSendIsDeliveredAfterARestoreTest` holds the same
// fact at the engine's own save and restore, on a machine built by hand, because
// no shared static fixture used to declare a host processor. This one drives
// `statechart_static_delayed_host_send.scxml`, a `datamodel="sce-static"`
// document, generated with the declaration: the send is armed by the code the
// generator emitted, with its two typed `<param>`s read from the machine's
// fields, and saved and restored by the `save` and `restore` it emitted.
//
// The run `start`, `bump` changes `job` after the send was armed, so the request
// the wait carries (`job` 7) is told from one evaluated again when the wait ends
// (`job` 8): a restore hands the host the request the document made, and the
// machine itself carries on with the `job` it had.
//
// Driven entirely on `ManualClock`: no case sleeps, and none can be decided by
// how loaded the build machine is. The shared instance
// `saved/statechart_static_delayed_host_send_waiting.json` is the text the Rust
// suite writes and restores too.

package com.sce.integration

import com.sce.integration.statechart_static_delayed_host_send.StatechartStaticDelayedHostSendEvent
import com.sce.integration.statechart_static_delayed_host_send.StatechartStaticDelayedHostSendState
import com.sce.integration.statechart_static_delayed_host_send.StatechartStaticDelayedHostSendStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedAct
import com.sce.runtime.SavedState
import com.sce.runtime.StateMachineEngine.HostSendRequest
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AGeneratedMachinesWaitingHostSendSurvivesARestoreTest {

    /** The wall-clock moment the shared instance was saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    /** How long the document's send waits, and how much of that has gone by when the machine is saved. */
    private val waitMs = 500L
    private val waitedMs = 100L

    private val shared: String = File(
        repoRoot(),
        "sce-build/tests/fixtures/host_processor/saved/statechart_static_delayed_host_send_waiting.json",
    ).readText().trim()

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    /** What the host was asked: the engine's reading of "now" at the moment, and the request. */
    private class Host {
        val asked = mutableListOf<Pair<Long, HostSendRequest>>()

        fun registerOn(sm: StatechartStaticDelayedHostSendStateMachine) {
            sm.registerEventProcessor("x-sce-host") { request ->
                asked.add(sm.clock.elapsedMs() to request)
                emptyList()
            }
        }
    }

    private fun machine(): StatechartStaticDelayedHostSendStateMachine {
        val sm = StatechartStaticDelayedHostSendStateMachine()
        sm.clock = ManualClock(0)
        return sm
    }

    private fun drive(sm: StatechartStaticDelayedHostSendStateMachine, event: StatechartStaticDelayedHostSendEvent) {
        sm.send(event)
        sm.tick()
    }

    /** A machine that armed its send with `job` 7, saw `bump`, and has waited [waitedMs] of the [waitMs]. */
    private fun <T> withWaiting(body: (StatechartStaticDelayedHostSendStateMachine) -> T): T {
        val sm = machine()
        try {
            Host().registerOn(sm)
            sm.initialize()
            drive(sm, StatechartStaticDelayedHostSendEvent.Start)
            drive(sm, StatechartStaticDelayedHostSendEvent.Bump)
            sm.advanceTimeMs(waitedMs)
            assertTrue(StatechartStaticDelayedHostSendState.Waiting in sm.snapshot.value.configuration)
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    /** [text] restored [elapsedMs] after it was saved, its handler registered before the restore as a Kotlin host does it. */
    private fun <T> withRestored(
        text: String,
        elapsedMs: Long,
        body: (StatechartStaticDelayedHostSendStateMachine, Host) -> T,
    ): T {
        val sm = machine()
        val host = Host()
        try {
            host.registerOn(sm)
            sm.restore(SavedState.fromJson(text), savedAtMs + elapsedMs)
            return body(sm, host)
        } finally {
            sm.cleanup()
        }
    }

    /** The `job` of a request, from the text the wire carries it as. */
    private fun jobOf(request: HostSendRequest): String? = request.params["job"]?.firstOrNull()

    @Test
    fun aWaitingSendIsSavedWithTheRequestItWasArmedWith() {
        withWaiting { sm ->
            val saved = sm.save(savedAtMs)

            val entry = saved.pending.single()
            assertEquals(savedAtMs + waitMs - waitedMs, entry.due)
            val host = entry.act as SavedAct.Host
            assertEquals("x-sce-host", host.processorType)
            assertEquals("audit", host.event)
            assertEquals("job://report", host.target)
            assertEquals("a", host.sendId)
            // Armed with `job` 7: `bump` ran after, and the request does not follow it.
            assertEquals(mapOf("job" to listOf("7"), "label" to listOf("report")), host.params)

            // The text the Rust suite writes for the same machine, byte for byte. It
            // also says the machine itself carries the `job` it has now (8), apart
            // from the 7 the request was armed with.
            assertEquals(shared, saved.toJson())
        }
    }

    @Test
    fun aRestoredSendIsMadeWhenItsMomentComesWithTheJobItWasArmedWith() {
        // Back 50 ms after the save: the send has 350 ms left of the 400 it had.
        withRestored(shared, 50) { sm, host ->
            assertTrue(StatechartStaticDelayedHostSendState.Waiting in sm.snapshot.value.configuration)

            sm.advanceTimeMs(349)
            assertTrue(host.asked.isEmpty(), "nothing is due yet: ${host.asked}")
            sm.advanceTimeMs(1)

            val (at, request) = host.asked.single()
            assertEquals(350L, at, "at the moment it came due")
            assertEquals("x-sce-host", request.processorType)
            assertEquals("audit", request.eventName)
            assertEquals("job://report", request.target)
            assertEquals("a", request.sendId)
            assertEquals(
                "7",
                jobOf(request),
                "the request the document made, not one evaluated again with the `job` the machine holds now",
            )
            assertEquals("""{"job":7,"label":"report"}""", request.eventData)
        }
    }

    @Test
    fun aSendAlreadyDueWhenTheMachineComesBackIsMadeWhenItIsDriven() {
        // A minute away: the wait ran out while the process was dead.
        withRestored(shared, 60_000) { sm, host ->
            assertTrue(host.asked.isEmpty(), "restoring makes no send")

            sm.advanceTimeMs(0)
            assertEquals(1, host.asked.size, "${host.asked}")
            assertEquals(0L, host.asked.single().first)
        }
    }

    @Test
    fun aRestoredSendCanStillBeCancelledByItsId() {
        withRestored(shared, 0) { sm, host ->
            drive(sm, StatechartStaticDelayedHostSendEvent.Cancel)

            sm.advanceTimeMs(10_000)
            assertTrue(host.asked.isEmpty(), "the cancel found the send the saved state carried: ${host.asked}")
        }
    }

    @Test
    fun aRestoredMachineCarriesOnWithTheJobItHad() {
        // `job` is 8 in the saved state; a `bump` after the restore makes it 9, and a
        // new wait armed from `idle` reads the machine's own field. The restored wait
        // is over (at 400 ms) before the second is armed: a send under an id still
        // waiting is replaced by one backend and left beside it by another, and this
        // case is not about that.
        withRestored(shared, 0) { sm, host ->
            sm.advanceTimeMs(400)
            drive(sm, StatechartStaticDelayedHostSendEvent.Stop)
            drive(sm, StatechartStaticDelayedHostSendEvent.Bump)
            drive(sm, StatechartStaticDelayedHostSendEvent.Start)
            sm.advanceTimeMs(10_000)

            assertEquals(
                listOf("7", "9"),
                host.asked.map { jobOf(it.second) },
                "the restored wait first, then the one armed with the job the restored field held: ${host.asked}",
            )
        }
    }

    @Test
    fun aSavedStateRoundTripsThroughItsOwnText() {
        withRestored(shared, 0) { sm, _ ->
            assertEquals(shared, sm.save(savedAtMs).toJson())
        }
    }
}
