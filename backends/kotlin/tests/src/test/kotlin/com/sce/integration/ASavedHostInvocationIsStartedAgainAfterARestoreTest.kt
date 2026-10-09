// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" — Kotlin half of the Rust
// suite's `a_saved_host_invocation_is_started_again_after_a_restore.rs`.
//
// An `<invoke>` the HOST runs is part of what a machine is doing (§scxml-6.4.1),
// and the process that ran it is gone with the process that saved it. A saved
// state holds each running host-run invocation with the request it was started
// with and the deadline it had left, and a restore starts it again from that
// request — never evaluating the element a second time — with `restarted` set, a
// new token and what was left of the deadline.
//
// `statechart_static_host_invoke.scxml` hands a job to the host from `working`.
// `done.invoke.h` and `error.invoke.h` each leave it and record a digit in
// `seen`, so the number says which arrived.
//
// Driven entirely on `ManualClock`: no case sleeps, and none can be decided by
// how loaded the build machine is. The shared instance
// `saved/statechart_static_host_invoke_running.json` is the text the Rust suite
// writes and restores too.

package com.sce.integration

import com.sce.integration.statechart_static_host_invoke.StatechartStaticHostInvokeEvent
import com.sce.integration.statechart_static_host_invoke.StatechartStaticHostInvokeState
import com.sce.integration.statechart_static_host_invoke.StatechartStaticHostInvokeStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import com.sce.runtime.StateMachineEngine.HostInvokeCancel
import com.sce.runtime.StateMachineEngine.HostInvokeRequest
import com.sce.runtime.StateRefusal
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ASavedHostInvocationIsStartedAgainAfterARestoreTest {

    /** The wall-clock moment the shared instance was saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private val shared: String = File(
        repoRoot(),
        "sce-build/tests/fixtures/host_processor/saved/statechart_static_host_invoke_running.json",
    ).readText().trim()

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    /** What the host was told, in order. */
    private sealed interface Told {
        data class Start(val request: HostInvokeRequest) : Told
        data class Cancel(val cancel: HostInvokeCancel) : Told
    }

    private class Host {
        val told = mutableListOf<Told>()
        val starts: List<HostInvokeRequest> get() = told.filterIsInstance<Told.Start>().map { it.request }

        fun registerOn(sm: StatechartStaticHostInvokeStateMachine) {
            sm.registerInvoker("x-sce-host") { event ->
                event.start?.let { told.add(Told.Start(it)) }
                event.cancel?.let { told.add(Told.Cancel(it)) }
                null
            }
        }
    }

    /** Whether two requests say the same thing: everything but the token the engine assigns and the mark a restart carries. */
    private fun sameRequest(a: HostInvokeRequest, b: HostInvokeRequest): Boolean =
        a.processorType == b.processorType && a.invokeId == b.invokeId && a.src == b.src &&
            a.params == b.params && a.eventData == b.eventData && a.content == b.content

    private fun machine(): StatechartStaticHostInvokeStateMachine {
        val sm = StatechartStaticHostInvokeStateMachine()
        sm.clock = ManualClock(0)
        return sm
    }

    /** A machine standing in `working`, its invocation started with token 0. */
    private fun <T> withWorking(body: (StatechartStaticHostInvokeStateMachine, Host) -> T): T {
        val sm = machine()
        val host = Host()
        try {
            host.registerOn(sm)
            sm.initialize()
            sm.send(StatechartStaticHostInvokeEvent.Start)
            sm.tick()
            return body(sm, host)
        } finally {
            sm.cleanup()
        }
    }

    /** [text] restored [elapsedMs] after it was saved, its invoker registered before the restore as a Kotlin host does it. */
    private fun <T> withRestored(
        text: String,
        elapsedMs: Long,
        register: Boolean = true,
        body: (StatechartStaticHostInvokeStateMachine, Host) -> T,
    ): T {
        val sm = machine()
        val host = Host()
        try {
            if (register) host.registerOn(sm)
            sm.restore(SavedState.fromJson(text), savedAtMs + elapsedMs)
            return body(sm, host)
        } finally {
            sm.cleanup()
        }
    }

    private fun standingIn(sm: StatechartStaticHostInvokeStateMachine) = sm.snapshot.value.configuration

    @Test
    fun aMachineIsSavedWithTheRequestItsInvocationWasStartedWith() {
        withWorking { sm, host ->
            val first = host.starts.single()
            assertFalse(first.restarted, "the document's own start is not a restart")
            assertEquals(0L, first.token)

            val saved = sm.save(savedAtMs)
            val entry = saved.hostInvokes.single()
            assertEquals("x-sce-host", entry.processorType)
            assertEquals("h", entry.invokeId)
            assertEquals(first.src, entry.src)
            assertEquals(first.content, entry.content)
            assertEquals(first.eventData, entry.data)
            // The deadline is the invocation's own, not a send waiting to be delivered.
            assertEquals(savedAtMs + 5000, entry.due)
            assertTrue(saved.pending.isEmpty(), "${saved.pending}")
            // One start has been made, so the next token is 1.
            assertEquals(1L, saved.hostInvokeToken)

            // The text the Rust suite writes for the same machine, byte for byte.
            assertEquals(shared, saved.toJson())
        }
    }

    @Test
    fun aRestoreAsksNothingOfTheHostUntilTheMachineIsDriven() {
        withRestored(shared, 1000) { sm, host ->
            assertTrue(host.told.isEmpty(), "the restore itself calls nobody")
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.Working), standingIn(sm))

            // The first macrostep starts it, where an invocation the document
            // entered would have started.
            sm.tick()
            assertEquals(1, host.starts.size)
        }
    }

    @Test
    fun aRestoredInvocationIsStartedAgainFromTheRequestItWasSavedWith() {
        val first = withWorking { _, host -> host.starts.single() }

        withRestored(shared, 1000) { sm, host ->
            sm.tick()
            val again = host.starts.single()
            assertTrue(again.restarted, "the host is told this is a resumption")
            assertEquals(1L, again.token, "the next token the saved state carried on")
            assertTrue(sameRequest(again, first), "the same request, not one evaluated again:\n$again\n$first")
        }
    }

    @Test
    fun aRestoredInvocationKeepsTheDeadlineItHadLeft() {
        // Saved with 5000 ms to run, restored 1000 ms later: 4000 ms are left.
        withRestored(shared, 1000) { sm, host ->
            sm.tick()
            sm.advanceTimeMs(3999)
            assertEquals(0u, sm.seen, "not due yet")
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.Working), standingIn(sm))

            sm.advanceTimeMs(1)
            assertEquals(2u, sm.seen, "error.invoke.h, at 4000 ms")
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.Failed), standingIn(sm))
            // The host is told to stop what it was given, with the token it was given.
            val last = host.told.last()
            assertTrue(last is Told.Cancel && last.cancel.token == 1L && last.cancel.invokeId == "h", "${host.told}")
        }
    }

    @Test
    fun aDeadlineThatPassedWhileTheMachineWasAwayEndsTheInvocationUnstarted() {
        // 6 s after the save, and the deadline was at 5 s.
        withRestored(shared, 6000) { sm, host ->
            sm.tick()
            assertTrue(
                host.told.isEmpty(),
                "the host is not asked to begin what it would be told to stop: ${host.told}",
            )
            assertEquals(2u, sm.seen, "the document hears error.invoke.h")
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.Failed), standingIn(sm))
        }
    }

    /**
     * A host drives a machine by asking when it next needs a tick and sleeping that
     * long. A restored invocation owes the machine a tick before anything else: it is
     * started in the first macrostep, and the deadline it keeps is armed there. A
     * machine that answered "nothing" would be left asleep for good, holding an
     * invocation no one had started.
     */
    @Test
    fun aRestoredMachineOwingAStartSaysItNeedsATickNow() {
        for (elapsedMs in listOf(1000L, 6000L)) {
            withRestored(shared, elapsedMs) { sm, _ ->
                assertEquals(
                    0L,
                    sm.timeUntilNextScheduledMs(),
                    "$elapsedMs ms after the save, before any tick",
                )
                sm.tick()
            }
        }
    }

    @Test
    fun whatTheFirstRunHandedAHostCannotAnswerForTheSecond() {
        // Token 0 belonged to the process that saved the machine. A late reply
        // carrying it is a different process's, and is ignored (§scxml-6.4).
        withRestored(shared, 1000) { sm, host ->
            sm.tick()
            val token = host.starts.single().token
            assertFalse(sm.completeHostInvoke("x-sce-host", "h", 0, "late"))
            sm.tick()
            assertEquals(0u, sm.seen)
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.Working), standingIn(sm))

            assertTrue(sm.completeHostInvoke("x-sce-host", "h", token, "done"))
            sm.tick()
            assertEquals(1u, sm.seen)
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.After), standingIn(sm))
        }
    }

    @Test
    fun aCompletionStillQueuedWhenTheMachineWasSavedIsAcceptedAfterTheRestore() {
        val saved = withWorking { sm, host ->
            // The host answers; the completion is on the external queue, stamped
            // with the start's token, and the machine has not been driven through it.
            assertTrue(sm.completeHostInvoke("x-sce-host", "h", 0, "done"))
            val saved = sm.save(savedAtMs)
            assertEquals(1, host.told.size, "started once, never cancelled")
            saved
        }
        assertTrue(saved.hostInvokes.isEmpty(), "the invocation ended: ${saved.hostInvokes}")
        val queued = saved.external.single()
        assertEquals("done.invoke.h", queued.name)
        assertEquals(0L, queued.hostInvokeToken)

        withRestored(saved.toJson(), 0) { again, host ->
            again.tick()
            assertEquals(1u, again.seen, "taken once, not refused")
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.After), standingIn(again))
            assertTrue(host.told.isEmpty(), "an ended invocation is not started again")
            assertEquals(0L, again.refusedHostInvokeCompletions)
        }
    }

    @Test
    fun aCompletionTheEngineWasAboutToRefuseIsStillRefusedAfterTheRestore() {
        val saved = withWorking { sm, _ ->
            // Raised through the ordinary door, so it carries no token: the engine
            // refuses it when it is dequeued, because it may be a cancelled run's reply.
            sm.send(StatechartStaticHostInvokeEvent.Done.Invoke.H)
            sm.save(savedAtMs)
        }
        assertNull(saved.external[0].hostInvokeToken)

        withRestored(saved.toJson(), 0) { again, _ ->
            again.tick()
            assertEquals(1L, again.refusedHostInvokeCompletions)
            assertEquals(0u, again.seen)
            assertEquals(setOf<Any>(StatechartStaticHostInvokeState.Working), standingIn(again))
        }
    }

    @Test
    fun anInvocationCancelledBeforeItWasStartedAgainIsNeverStarted() {
        withRestored(shared, 1000) { sm, host ->
            // Nothing was started, so there is nothing to tell the host to stop.
            assertFalse(sm.cancelHostInvoke("x-sce-host", "h"))
            sm.tick()
            assertTrue(host.told.isEmpty(), "${host.told}")
        }
    }

    @Test
    fun aMachineRestoredAndSavedAgainBeforeItWasDrivenKeepsItsInvocation() {
        withRestored(shared, 1000) { sm, _ ->
            val again = sm.save(savedAtMs + 1000)
            // The same request, due at the same wall-clock moment, and the same counter.
            assertEquals(shared, again.toJson())
        }
    }

    @Test
    fun aRestartNobodyCanRunIsTheErrorAStartNobodyCanRunIs() {
        withRestored(shared, 1000, register = false) { sm, _ ->
            // No invoker registered by the first macrostep.
            sm.tick()
            assertEquals(1, sm.unhandledErrorEvents(), "error.execution")
        }
    }

    @Test
    fun aSavedStateNamingAHostInvocationTheDocumentCannotHaveRunIsRefused() {
        fun swap(from: String, to: String) = shared.replace(from, to)
        val start = shared.indexOf("\"hostinvokes\":[") + "\"hostinvokes\":[".length
        val end = shared.indexOf("],\"hostinvoketoken\"")
        val one = shared.substring(start, end)
        val cases = listOf(
            Triple(
                "an id the document does not invoke",
                swap("\"id\":\"h\"", "\"id\":\"stranger\""),
                "which the document does not have a host invoker run",
            ),
            Triple(
                "a type the document does not hand that id to",
                swap("\"type\":\"x-sce-host\"", "\"type\":\"x-other\""),
                "which the document does not have a host invoker run",
            ),
            Triple(
                "an invocation whose state the configuration is not in",
                swap("\"configuration\":[\"working\"]", "\"configuration\":[\"idle\"]")
                    .replace("\"current\":\"working\"", "\"current\":\"idle\""),
                "whose state the saved configuration does not stand in",
            ),
            Triple(
                "an invocation named twice",
                shared.replace(one, "$one,$one"),
                "which an earlier entry already names",
            ),
        )
        for ((what, text, expected) in cases) {
            val sm = machine()
            try {
                val refusal = assertFailsWith<StateRefusal>(what) {
                    sm.restore(SavedState.fromJson(text), savedAtMs)
                }
                assertTrue(refusal.message!!.contains(expected), "$what: ${refusal.message}")
            } finally {
                sm.cleanup()
            }
        }
    }

    @Test
    fun theHostInvokeFieldsAreRequiredAndMustBeWhatTheySay() {
        fun swap(from: String, to: String) = shared.replace(from, to)
        val start = shared.indexOf("\"hostinvokes\":[")
        val end = shared.indexOf(",\"hostinvoketoken\"")
        val cases = listOf(
            Triple("no hostinvokes", swap("\"hostinvokes\":[", "\"hostinvokes_\":["), "has no 'hostinvokes'"),
            Triple(
                "hostinvokes that is not an array",
                shared.substring(0, start) + "\"hostinvokes\":\"h\"" + shared.substring(end),
                "'hostinvokes' is not an array",
            ),
            Triple("no hostinvoketoken", swap("\"hostinvoketoken\":\"1\",", ""), "has no 'hostinvoketoken'"),
            Triple(
                "a token that is a number",
                swap("\"hostinvoketoken\":\"1\"", "\"hostinvoketoken\":1"),
                "'hostinvoketoken' is not a text",
            ),
            Triple(
                "a token past a signed 64-bit count",
                swap("\"hostinvoketoken\":\"1\"", "\"hostinvoketoken\":\"9223372036854775808\""),
                "is not a whole number a token can be",
            ),
            Triple(
                "a deadline that is not a moment",
                swap("\"due\":\"1700000005000\"", "\"due\":\"soon\""),
                "is not a whole number of milliseconds",
            ),
            Triple(
                "a deadline that is a number",
                swap("\"due\":\"1700000005000\"", "\"due\":1700000005000"),
                "'hostinvokes[0].due' is neither a text nor null",
            ),
        )
        for ((what, text, expected) in cases) {
            val refusal = assertFailsWith<StateRefusal>(what) { SavedState.fromJson(text) }
            assertTrue(refusal.message!!.contains(expected), "$what: ${refusal.message}")
        }
    }
}
