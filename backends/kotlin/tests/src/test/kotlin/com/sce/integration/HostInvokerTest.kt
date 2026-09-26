// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 — Kotlin compile+run gate for an `<invoke type>` the HOST
// runs.
//
// The Kotlin engine carried the registry (`registerInvoker`) and the template
// lowered the start and the cancel, but no Kotlin test ran either, so the one
// backend an Android host builds on held the lifecycle on trust. This is the
// channel the C++, Rust, Go and Python gates already are.
//
// An invoke is not a send: it has a LIFETIME. The cases hold the outcomes
// apart, because the configuration alone cannot:
//
//   * a registered invoker is STARTED with what the document wrote, each
//     invocation as itself, and its completion names the invocation;
//   * leaving the state CANCELS each one, once;
//   * a cancel is delivered only for an invocation that started;
//   * a declared type with nothing registered raises `error.execution`;
//   * an invoker registered for another type does not run this one.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml
// (shared with the C++, Rust, Go and Python channels). Regeneration:
//   scripts/regen_host_processor_kotlin.sh

package com.sce.integration

import com.sce.integration.statechart_host_invoker.StatechartHostInvokerEvent
import com.sce.integration.statechart_host_invoker.StatechartHostInvokerPermRequest
import com.sce.integration.statechart_host_invoker.StatechartHostInvokerPermResult
import com.sce.integration.statechart_host_invoker.StatechartHostInvokerState
import com.sce.integration.statechart_host_invoker.StatechartHostInvokerStateMachine
import com.sce.integration.statechart_host_invoker.StatechartHostInvokerXSceHostInvoker
import com.sce.runtime.EventMetadata
import com.sce.runtime.EventPayload
import com.sce.runtime.TypedRequest
import com.sce.runtime.HOST_INVOKE_DEADLINE_PARAM
import com.sce.runtime.ManualClock
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.parseHostInvokeDeadlineMs
import com.sce.w3c.W3CTestBase
import java.io.File
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4.1 — an `<invoke type>` the host runs.
@DisplayName("HostInvoker — W3C SCXML 6.4.1")
class HostInvokerTest {

    // The type the fixture was compiled for.
    // `scripts/regen_host_processor_kotlin.sh` passes this same string to
    // `--host-invoker`; a test registering a different one would measure
    // nothing and pass, so the `refused` counter is asserted rather than the
    // registration trusted.
    private val declaredType = "x-sce-host"

    // The fixture counts every outcome with `<assign>`, so this is an
    // ECMAScript-datamodel machine. Not started here: the invoker has to be
    // registered BEFORE `initialize()`, because the invocations run at the end
    // of the entry macrostep.
    private fun machine(): StatechartHostInvokerStateMachine =
        StatechartHostInvokerStateMachine(W3CTestBase.createEngine())

    private fun counter(sm: StatechartHostInvokerStateMachine, name: String): Long {
        val value = when (name) {
            "started" -> sm.started()
            "started2" -> sm.started2()
            "refused" -> sm.refused()
            "ended" -> sm.ended()
            "entered" -> sm.entered()
            "dropped" -> sm.dropped()
            "matched" -> sm.matched()
            "slotted" -> sm.slotted()
            "pinged" -> sm.pinged()
            "leaked" -> sm.leaked()
            "lost" -> sm.lost()
            "expired" -> sm.expired()
            "finished" -> sm.finished()
            "misdated" -> sm.misdated()
            "granted" -> sm.granted()
            "denied" -> sm.denied()
            "unreadable" -> sm.unreadable()
            else -> error("the fixture declares no counter named `$name`")
        }
        assertNotNull(value, "the fixture declares `$name` and the machine could not read it")
        return value!!
    }

    /// A recording invoker. Answers a completion on start so the `done.invoke`
    /// path is exercised too, and records both arms in call order.
    private fun recordingInvoker(log: MutableList<String>): (StateMachineEngine.HostInvokeEvent) -> StateMachineEngine.HostInvokeResponse? =
        { ev ->
            val start = ev.start
            val cancel = ev.cancel
            if (start != null) {
                val within = start.params["within"]?.firstOrNull() ?: "absent"
                log.add("START id=${start.invokeId} type=${start.processorType} src=${start.src} within=$within")
                StateMachineEngine.HostInvokeResponse(doneData = "ok")
            } else {
                if (cancel != null) {
                    log.add("CANCEL id=${cancel.invokeId}")
                }
                null
            }
        }

    @Test
    fun aRegisteredInvokerIsStartedWithWhatTheDocumentWrote() {
        val sm = machine()
        val log = mutableListOf<String>()
        sm.registerInvoker(declaredType, recordingInvoker(log))
        sm.initialize()
        try {
            // The fixture counts a completion only when its `_event.invokeid`
            // names the invocation (W3C SCXML 5.10.1), so each counter is that
            // assertion too.
            assertEquals(1L, counter(sm, "started"), "done.invoke.probe never arrived, or arrived without its invokeid")
            assertEquals(1L, counter(sm, "started2"), "done.invoke.probe2 never arrived, or arrived without its invokeid")
            assertEquals(0L, counter(sm, "refused"), "a started invocation also raised error.execution")
            // The false-positive guard: ordinary entry content must still run.
            assertEquals(1L, counter(sm, "entered"), "the entry chain stopped running")

            // `src` and `<param>` are how W3C SCXML 6.4.1 lets the document say
            // WHAT to invoke and with what. Each invocation is started as
            // itself: `probe2` begins with `probe`.
            assertEquals(
                listOf(
                    "START id=probe type=$declaredType src=pane://turn within=2500",
                    "START id=probe2 type=$declaredType src=pane://other within=absent",
                ),
                log,
                "the start requests lost part of what the document wrote",
            )
        } finally {
            sm.cleanup()
        }
    }

    /// The invocation ends with the state that started it. Without this the
    /// host is told to begin work and never told to stop — which no
    /// configuration assertion can detect.
    /**
     * W3C SCXML 6.4.1: `srcexpr`, `namelist`, `<param expr>` and
     * `<content expr>` are read from the data model when the invocation
     * starts. The request used to carry the literal params alone, so a
     * document that computed what to invoke handed the host an empty
     * description.
     */
    @Test
    fun whatTheRequestSaysIsEvaluatedWhenTheInvocationStarts() {
        val sm = machine()
        val starts = mutableListOf<StateMachineEngine.HostInvokeRequest>()
        sm.registerInvoker(declaredType) { ev ->
            ev.start?.let { starts.add(it) }
            null
        }
        sm.initialize()
        try {
            starts.clear() // `probe` / `probe2`, which the case above already reads
            sm.send(StatechartHostInvokerEvent.Evaluate)
            sm.tick()

            // `req3`'s srcexpr cannot be evaluated, so it is never started.
            assertEquals(listOf("req", "req2"), starts.map { it.invokeId }, "started")
            assertEquals("pane://dyn", starts[0].src, "srcexpr was not evaluated")
            // A repeated name keeps both values in document order; the
            // `<param>` that failed is absent (W3C SCXML 5.7.1) while the
            // invocation still started.
            assertEquals(mapOf("n" to listOf("7"), "twice" to listOf("a", "8")), starts[0].params, "params")
            assertEquals("body:7", starts[1].content, "content")
            // One error.execution for the dropped `<param>`, one for `req3`.
            assertEquals(2L, counter(sm, "dropped"), "a failed evaluation was not reported")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * An invoker whose work outlives the call: it answers nothing on start,
     * so each invocation stays running until the test completes or cancels
     * it. Records the lines [recordingInvoker] does and every start's token
     * for a later `completeHostInvoke`.
     */
    private fun runningInvoker(
        log: MutableList<String>,
        starts: MutableList<Pair<String, Long>>,
    ): (StateMachineEngine.HostInvokeEvent) -> StateMachineEngine.HostInvokeResponse? =
        { ev ->
            ev.start?.let {
                log.add("START id=${it.invokeId}")
                starts.add(it.invokeId to it.token)
            }
            ev.cancel?.let { log.add("CANCEL id=${it.invokeId}") }
            null
        }

    /** The token of the latest start of [invokeId]. */
    private fun tokenOf(starts: List<Pair<String, Long>>, invokeId: String): Long =
        starts.lastOrNull { it.first == invokeId }?.second ?: error("`$invokeId` never started")

    /** `send` queues; `tick` is this backend's macrostep driver. */
    private fun deliver(sm: StatechartHostInvokerStateMachine, event: StatechartHostInvokerEvent) {
        sm.send(event)
        sm.tick()
    }

    @Test
    fun leavingTheStateCancelsTheInvocation() {
        val sm = machine()
        val log = mutableListOf<String>()
        // Still running when the state exits — a completed invocation has
        // nothing left to cancel (aCompletedInvocationIsNotCancelled).
        sm.registerInvoker(declaredType, runningInvoker(log, mutableListOf()))
        sm.initialize()
        try {
            // `send` queues; `tick` is this backend's macrostep driver.
            sm.send(StatechartHostInvokerEvent.Leave)
            sm.tick()
            assertEquals(1L, counter(sm, "ended"), "the machine never left the invoking state")
            for (id in listOf("probe", "probe2")) {
                assertEquals(1, log.count { it == "CANCEL id=$id" }, "cancel for $id did not reach the invoker exactly once: $log")
            }
        } finally {
            sm.cleanup()
        }
    }

    /// A cancel is delivered once, and only for an invocation that started. The
    /// engine owns that judgement: the exit chain cancels unconditionally.
    /// Asserted at the engine surface, because driving the machine cannot
    /// produce the "never started" case — every call that advances it runs a
    /// macrostep, and the pending invoke executes at the end of it.
    @Test
    fun cancelIsNotDeliveredForAnInvocationThatNeverStarted() {
        val sm = machine()
        val log = mutableListOf<String>()
        sm.registerInvoker(declaredType, runningInvoker(log, mutableListOf()))
        try {
            assertFalse(sm.cancelHostInvoke(declaredType, "probe"), "a cancel was reported for an invocation that never started")
            assertTrue(log.isEmpty(), "the invoker was called for an invocation that never started: $log")

            sm.initialize()
            assertTrue(sm.cancelHostInvoke(declaredType, "probe"), "a started invocation reported nothing to cancel")
            assertFalse(sm.cancelHostInvoke(declaredType, "probe"), "the same invocation was cancelled twice")
            assertEquals(1, log.count { it.startsWith("CANCEL") }, "cancel reached the invoker more than once: $log")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * §scxml-6.4: `done.invoke` says the invoked process is over, so leaving
     * the state afterwards has nothing to stop. Both invocations here complete
     * synchronously; neither is cancelled.
     */
    @Test
    fun aCompletedInvocationIsNotCancelled() {
        val sm = machine()
        val log = mutableListOf<String>()
        sm.registerInvoker(declaredType, recordingInvoker(log))
        sm.initialize()
        try {
            deliver(sm, StatechartHostInvokerEvent.Leave)
            assertEquals(1L, counter(sm, "started"))
            assertTrue(log.none { it.startsWith("CANCEL") }, "a completed invocation was cancelled: $log")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * A host that finishes later reports it with the start's token, and the
     * completion is taken once: a second report of the same run finds nothing,
     * and the state's exit then cancels only the invocation still running.
     */
    @Test
    fun aLateCompletionIsAcceptedExactlyOnce() {
        val sm = machine()
        val log = mutableListOf<String>()
        val starts = mutableListOf<Pair<String, Long>>()
        sm.registerInvoker(declaredType, runningInvoker(log, starts))
        sm.initialize()
        try {
            assertEquals(0L, counter(sm, "started"))
            val token = tokenOf(starts, "probe")
            assertTrue(sm.completeHostInvoke(declaredType, "probe", token, "ok"), "a running invocation's completion was refused")
            sm.tick()
            // The fixture counts it only when `_event.invokeid` names the
            // invocation (§scxml-5.10.1), so this is that assertion too.
            assertEquals(1L, counter(sm, "started"))
            assertFalse(sm.completeHostInvoke(declaredType, "probe", token, "again"), "the same run completed twice")
            sm.tick()
            assertEquals(1L, counter(sm, "started"))

            deliver(sm, StatechartHostInvokerEvent.Leave)
            assertEquals(listOf("CANCEL id=probe2"), log.filter { it.startsWith("CANCEL") }, "only the invocation still running is cancelled")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * §scxml-6.4: once the state has exited, what the cancelled process sends
     * is ignored. The host's reply arrives after the cancel and is refused.
     */
    @Test
    fun aCompletionAfterTheCancelIsRefused() {
        val sm = machine()
        val starts = mutableListOf<Pair<String, Long>>()
        sm.registerInvoker(declaredType, runningInvoker(mutableListOf(), starts))
        sm.initialize()
        try {
            val token = tokenOf(starts, "probe")
            deliver(sm, StatechartHostInvokerEvent.Leave)
            assertFalse(sm.completeHostInvoke(declaredType, "probe", token, "late"), "a cancelled run's completion was accepted")
            sm.tick()
            assertEquals(0L, counter(sm, "started"))
        } finally {
            sm.cleanup()
        }
    }

    /**
     * Re-entering the state starts the same `<invoke>` again under the same
     * id. The first run's late reply carries the first start's token and is
     * refused; the second run's is accepted.
     */
    @Test
    fun aRestartedInvokeRefusesTheFirstRunsReply() {
        val sm = machine()
        val starts = mutableListOf<Pair<String, Long>>()
        sm.registerInvoker(declaredType, runningInvoker(mutableListOf(), starts))
        sm.initialize()
        try {
            val first = tokenOf(starts, "probe")
            deliver(sm, StatechartHostInvokerEvent.Leave)
            deliver(sm, StatechartHostInvokerEvent.Again)
            val second = tokenOf(starts, "probe")
            assertTrue(first != second, "a restart reused the first start's token")

            assertFalse(sm.completeHostInvoke(declaredType, "probe", first, "stale"), "the first run's reply was taken for the second run's")
            sm.tick()
            assertEquals(0L, counter(sm, "started"))
            assertTrue(sm.completeHostInvoke(declaredType, "probe", second, "ok"))
            sm.tick()
            assertEquals(1L, counter(sm, "started"))
        } finally {
            sm.cleanup()
        }
    }

    /**
     * A host-run invocation's `done.invoke` raised through the ordinary
     * external-event API skipped the running check, so the engine refuses it
     * and counts the refusal. The metadata names the invocation, so without
     * the refusal the fixture's guarded transition would take it.
     */
    @Test
    fun aDoneInvokeRaisedTheOldWayIsRefusedAndCounted() {
        val sm = machine()
        sm.registerInvoker(declaredType, runningInvoker(mutableListOf(), mutableListOf()))
        sm.initialize()
        try {
            sm.sendEventByName("done.invoke.probe", EventMetadata(invokeId = "probe"))
            sm.tick()
            assertEquals(0L, counter(sm, "started"), "a completion that skipped the running check reached the document")
            assertEquals(1L, sm.refusedHostInvokeCompletions)
        } finally {
            sm.cleanup()
        }
    }

    /**
     * W3C SCXML 6.4.1: an invoke with no id and an `idlocation` gets a
     * generated `stateid.platformid` id, written to the location, handed to
     * the host, and carried as `_event.invokeid` on the completion. The
     * document names no specific `done.invoke.<id>`, so the completion arrives
     * as the generic `done.invoke`, and `matched` counts it only when its
     * invokeid is what the document stored.
     */
    @Test
    fun anIdlocationHoldsTheIdTheHostIsHanded() {
        val sm = machine()
        val log = mutableListOf<String>()
        sm.registerInvoker(declaredType, recordingInvoker(log))
        sm.initialize()
        try {
            deliver(sm, StatechartHostInvokerEvent.Leave)
            assertTrue(log.any { it.startsWith("START id=done._invoke_0 ") }, "the host was not handed the generated id: $log")
            assertEquals(1L, counter(sm, "matched"), "the completion did not arrive, or its invokeid is not what idlocation holds")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * W3C SCXML 6.2.4 / 6.4.1: an `idlocation` is a location expression, so
     * the id is written the way `<assign>` writes (5.4): `slot.id` and
     * `slot.sid` are member paths, and land. `n.nope.deeper` cannot take a
     * value, so each element raises error.execution and is abandoned (5.9.2) —
     * the host is never asked to start that invoke, and that message is never
     * sent.
     */
    @Test
    fun anIdlocationIsAssignedLikeALocation() {
        val sm = machine()
        val log = mutableListOf<String>()
        sm.registerInvoker(declaredType, recordingInvoker(log))
        sm.initialize()
        try {
            deliver(sm, StatechartHostInvokerEvent.Locate)
            assertTrue(log.any { it.startsWith("START id=locating._invoke_1 ") }, "the member-path invoke was not started: $log")
            assertFalse(log.any { "locating._invoke_2" in it }, "an invoke whose idlocation could not take the id was started: $log")

            val raw = sm.slot()
            assertNotNull(raw, "the fixture declares `slot` as an object")
            val slot = Json.parseToJsonElement(raw!!).jsonObject
            assertEquals("locating._invoke_1", slot["id"]?.jsonPrimitive?.content, "slot.id: $slot")
            assertTrue(!slot["sid"]?.jsonPrimitive?.content.isNullOrEmpty(), "slot.sid did not receive the send id: $slot")

            assertEquals(1L, counter(sm, "slotted"))
            assertEquals(1L, counter(sm, "pinged"))
            assertEquals(0L, counter(sm, "leaked"), "a send whose idlocation could not take the id was still sent")
            assertEquals(2L, counter(sm, "lost"))
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The generic `done.invoke` is a host completion too when its invokeid
     * names a host-run invoke, so raised around `completeHostInvoke` it is
     * refused like the specific name is.
     */
    @Test
    fun aGenericDoneInvokeRaisedTheOldWayIsRefused() {
        val sm = machine()
        sm.registerInvoker(declaredType, runningInvoker(mutableListOf(), mutableListOf()))
        sm.initialize()
        try {
            deliver(sm, StatechartHostInvokerEvent.Leave)
            sm.sendEventByName("done.invoke", EventMetadata(invokeId = "done._invoke_0"))
            sm.tick()
            assertEquals(0L, counter(sm, "matched"), "a completion that skipped the running check reached the document")
            assertEquals(1L, sm.refusedHostInvokeCompletions)
        } finally {
            sm.cleanup()
        }
    }

    /// The build declared the type, so codegen emitted a start — but nothing
    /// was registered, so no process ran. The same event as an unsupported
    /// type, because from the document's side it is the same fact.
    @Test
    fun aDeclaredTypeWithNoInvokerStillRaisesErrorExecution() {
        val sm = machine()
        sm.initialize()
        try {
            // One error.execution per invocation nobody ran.
            assertEquals(2L, counter(sm, "refused"), "an unregistered invoker was silently treated as started")
            assertEquals(0L, counter(sm, "started"), "done.invoke arrived for an invocation nobody ran")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * A machine on [ManualClock] whose invoker answers nothing and records,
     * per start, whether the request still carried the deadline param, driven
     * into `timed`. The clock is installed before `initialize()`, which arms
     * against it.
     */
    private fun timed(
        log: MutableList<String>,
        starts: MutableList<Pair<String, Long>>,
    ): StatechartHostInvokerStateMachine {
        val sm = machine()
        sm.clock = ManualClock(0L)
        sm.registerInvoker(declaredType) { ev ->
            ev.start?.let {
                log.add("START id=${it.invokeId} deadline-param=${HOST_INVOKE_DEADLINE_PARAM in it.params}")
                starts.add(it.invokeId to it.token)
            }
            ev.cancel?.let { log.add("CANCEL id=${it.invokeId}") }
            null
        }
        sm.initialize()
        deliver(sm, StatechartHostInvokerEvent.Time)
        return sm
    }

    /**
     * A deadline that passes while the invocation is still running ends it:
     * the host is told to stop, the document receives `error.invoke.slow` with
     * `_event.data` "deadline", and a reply afterwards is refused. The param is
     * the engine's — the host never sees it.
     */
    @Test
    fun aDeadlineThatPassesEndsTheInvocation() {
        val log = mutableListOf<String>()
        val starts = mutableListOf<Pair<String, Long>>()
        val sm = timed(log, starts)
        try {
            assertTrue("START id=slow deadline-param=false" in log, "the host was handed the deadline param, or `slow` never started: $log")
            // A `<cancel>` of the empty send id must not reach the deadline.
            deliver(sm, StatechartHostInvokerEvent.Forget)
            sm.advanceTimeMs(49)
            assertEquals(0L, counter(sm, "expired"), "expired early")
            sm.advanceTimeMs(1)
            assertEquals(1L, counter(sm, "expired"))
            assertTrue("CANCEL id=slow" in log, "the host was not told to stop: $log")

            assertFalse(sm.completeHostInvoke(declaredType, "slow", tokenOf(starts, "slow"), "late"), "a reply after the deadline was accepted")
            sm.tick()
            assertEquals(0L, counter(sm, "finished"))
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The discriminator: a completion before the deadline is the outcome, and
     * the deadline that comes due afterwards does nothing — no cancel, no
     * `error.invoke`, and nothing left for the host to tick toward.
     */
    @Test
    fun aCompletionBeforeTheDeadlineDisarmsIt() {
        val log = mutableListOf<String>()
        val starts = mutableListOf<Pair<String, Long>>()
        val sm = timed(log, starts)
        try {
            assertTrue(sm.completeHostInvoke(declaredType, "slow", tokenOf(starts, "slow"), "ok"))
            sm.tick()
            assertEquals(1L, counter(sm, "finished"))
            assertNull(sm.timeUntilNextScheduledMs(), "the disarmed deadline is still keeping the host ticking")

            sm.advanceTimeMs(100)
            assertEquals(0L, counter(sm, "expired"))
            assertFalse("CANCEL id=slow" in log, "a completed invocation was cancelled by its deadline: $log")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * §scxml-6.4.1: a deadline that is not a whole number of milliseconds is an
     * argument that cannot be evaluated — `error.execution`, and the host is
     * never asked to start the invocation.
     */
    @Test
    fun aDeadlineThatIsNotMillisecondsStartsNothing() {
        val log = mutableListOf<String>()
        val sm = timed(log, mutableListOf())
        try {
            assertEquals(1L, counter(sm, "misdated"))
            assertFalse(log.any { it.startsWith("START id=undated") }, "an invocation with an unreadable deadline was started: $log")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The coroutine mode keeps its deadlines too. Its loop used to wait on the
     * event channel alone, so a deadline armed in `scheduledSends` never came
     * due there and the host was never told to stop. Real time, because the
     * engine's coroutine runs on `Dispatchers.Default`, outside any test
     * scheduler; the wait is bounded far above the 50 ms deadline. The signal
     * is the host's own cancel, which arrives on the engine's thread — nothing
     * here reads the datamodel from another one.
     */
    @Test
    fun aDeadlineExpiresInCoroutineMode() = runBlocking {
        val started = CompletableDeferred<Unit>()
        val cancelled = CompletableDeferred<Unit>()
        val sm = machine()
        sm.registerInvoker(declaredType) { ev ->
            if (ev.start?.invokeId == "slow") started.complete(Unit)
            if (ev.cancel?.invokeId == "slow") cancelled.complete(Unit)
            null
        }
        sm.start(this)
        try {
            // Each wait is bounded well inside the runner's own per-test limit,
            // so a stall names its stage instead of surfacing as that limit.
            sm.send(StatechartHostInvokerEvent.Time)
            assertNotNull(withTimeoutOrNull(3_000) { started.await() }, "`slow` was never started in coroutine mode")
            assertNotNull(withTimeoutOrNull(3_000) { cancelled.await() }, "`slow` started, and its 50 ms deadline never expired")
            // The loop goes on serving events after performing an act, and
            // `done` is reached only once the expiry's own macrostep has run —
            // so the engine is idle when it is stopped below.
            val after = withTimeoutOrNull(3_000) { sm.sendAndAwait(StatechartHostInvokerEvent.Leave) }
            assertEquals(StatechartHostInvokerState.Done, after, "the loop stopped serving events after the deadline")
        } finally {
            sm.stop()
        }
    }

    /** `perm` completed with [doneData] in a machine driven into `typed`: the
     * counters `granted`, `denied` and `unreadable`. */
    private fun typedCompletion(doneData: String): List<Long> {
        val sm = machine()
        val starts = mutableListOf<Pair<String, Long>>()
        sm.registerInvoker(declaredType, runningInvoker(mutableListOf(), starts))
        sm.initialize()
        try {
            deliver(sm, StatechartHostInvokerEvent.Type)
            assertTrue(sm.completeHostInvoke(declaredType, "perm", tokenOf(starts, "perm"), doneData), "a running invocation's completion was refused")
            sm.tick()
            return listOf(counter(sm, "granted"), counter(sm, "denied"), counter(sm, "unreadable"))
        } finally {
            sm.cleanup()
        }
    }

    /**
     * SCE Accepted Subset §2.12: `sce:result` makes `perm`'s completion a
     * `PermResult` record, so its guards read `granted` as a typed field —
     * true and false each select their own transition — and a completion
     * whose data is not that record is refused as any typed payload the data
     * does not fit is: error.execution, and neither guard fires.
     */
    @Test
    fun aTypedCompletionIsReadAsItsRecord() {
        assertEquals(listOf(1L, 0L, 0L), typedCompletion("""{"granted":true}"""), "granted")
        assertEquals(listOf(0L, 1L, 0L), typedCompletion("""{"granted":false}"""), "denied")
        assertEquals(listOf(0L, 0L, 1L), typedCompletion("yes"), "not the record")
    }

    /**
     * Every runtime reads a deadline's text by one grammar, held to one table.
     * `toLongOrNull` would not do on its own: it reads a sign, and a deadline
     * this backend honours while another refuses it makes a document depend on
     * where it was compiled.
     */
    @Test
    fun aDeadlineIsReadByTheSharedTable() {
        // Found by walking up rather than by a fixed depth, because Gradle's
        // working directory is the project's and that is a build detail.
        val root = generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no ancestor of ${System.getProperty("user.dir")} holds sce-build/")
        val table = Json.parseToJsonElement(
            File(root, "sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json").readText()
        ).jsonObject
        val accepted = table.getValue("accepted").jsonArray
        val refused = table.getValue("refused").jsonArray
        // A floor: an empty table would pass every assertion below.
        assertTrue(accepted.isNotEmpty() && refused.isNotEmpty(), "the table is empty")
        for (pair in accepted) {
            val (written, ms) = pair.jsonArray.map { it.jsonPrimitive.content }
            assertEquals(ms.toLong(), parseHostInvokeDeadlineMs(written), "\"$written\"")
        }
        for (value in refused) {
            val written = value.jsonPrimitive.content
            assertNull(parseHostInvokeDeadlineMs(written), "\"$written\" was accepted")
        }
    }

    /// The registry is keyed. A lookup falling back to "any invoker" would hand
    /// a document's process to one it never named.
    @Test
    fun anInvokerRegisteredForAnotherTypeDoesNotRunThisOne() {
        val sm = machine()
        val log = mutableListOf<String>()
        sm.registerInvoker("x-some-other-host", recordingInvoker(log))
        sm.initialize()
        try {
            assertEquals(0L, counter(sm, "started"), "an invoker for a different type ran this one")
            assertEquals(2L, counter(sm, "refused"), "the unregistered type was not reported")
            assertTrue(log.isEmpty(), "the other type's invoker was called: $log")
        } finally {
            sm.cleanup()
        }
    }

    /** A host implementing the generated interface; its work outlives the call. */
    private class PermHost : StatechartHostInvokerXSceHostInvoker {
        val starts = mutableListOf<Pair<StatechartHostInvokerPermRequest, Long>>()
        val cancels = mutableListOf<Long>()

        override fun startPerm(request: StatechartHostInvokerPermRequest, token: Long): StatechartHostInvokerPermResult? {
            starts.add(request to token)
            return null
        }

        override fun cancelPerm(token: Long) {
            cancels.add(token)
        }
    }

    /**
     * A machine driven into `typed` with a [PermHost] registered through the
     * generated adapter, and the untyped invokes of the same type served by
     * [runningInvoker].
     */
    private fun <T> withTypedHost(body: (StatechartHostInvokerStateMachine, PermHost, List<String>) -> T): T {
        val sm = machine()
        val host = PermHost()
        val log = mutableListOf<String>()
        sm.registerXSceHostInvoker(host, runningInvoker(log, mutableListOf()))
        sm.initialize()
        try {
            deliver(sm, StatechartHostInvokerEvent.Type)
            return body(sm, host, log)
        } finally {
            sm.cleanup()
        }
    }

    /**
     * SCE Accepted Subset §2.12: through the generated interface a host is
     * handed `perm`'s request as its `PermRequest` record — the datamodel's
     * values at their declared types — and completes it with a `PermResult`,
     * which the document reads as that record. An invoke of the same type the
     * document does not type still reaches the host, through the fallback.
     */
    @Test
    fun aTypedRequestReachesItsInvokerAsItsRecord() = withTypedHost { sm, host, log ->
        assertEquals(1, host.starts.size, "perm started once")
        val (request, token) = host.starts[0]
        assertEquals(StatechartHostInvokerPermRequest(scope = "storage", level = 2u), request)
        assertTrue("START id=probe" in log, "the untyped `probe` never reached the fallback: $log")
        assertTrue(sm.completePerm(token, StatechartHostInvokerPermResult(granted = true)))
        sm.tick()
        assertEquals(1L, counter(sm, "granted"))
        assertEquals(0L, counter(sm, "unreadable"))
        // A completion is accepted once: the token now names nothing running.
        assertFalse(sm.completePerm(token, StatechartHostInvokerPermResult(granted = true)))
    }

    /**
     * SCE Accepted Subset §2.12, W3C SCXML 6.4.1: a request value its record's
     * field cannot hold is an argument that cannot be evaluated. `retype` sets
     * `level` to a text and re-enters `typed`: the running start is cancelled,
     * and the new one raises error.execution and is never handed to the host.
     */
    @Test
    fun aRequestThatDoesNotFitItsRecordStartsNothing() = withTypedHost { sm, host, _ ->
        val first = host.starts[0].second
        deliver(sm, StatechartHostInvokerEvent.Retype)
        assertEquals(listOf(first), host.cancels, "the running start was cancelled")
        assertEquals(1, host.starts.size, "the misfit request was started: ${host.starts}")
        assertEquals(1L, counter(sm, "unreadable"))
    }

    /**
     * The start site's check and the adapter's reading are one rule: every
     * value the check accepts is spelled as text its field's type parses back
     * to the same value, and a value the field cannot hold is refused rather
     * than narrowed.
     */
    @Test
    fun aRequestFieldIsCheckedAndReadBackByOneRule() {
        val fractional: (Any?) -> String = { (it as Double).toString() }
        fun text(value: Any?, type: TypedRequest.FieldType) =
            mapOf("f" to listOf(TypedRequest.wire(value, "f", type, fractional)))

        assertEquals(255.toUByte(), TypedRequest.uint8(text(255, TypedRequest.FieldType.UINT8), "perm", "f"))
        assertEquals((-3).toShort(), TypedRequest.int16(text(-3.0, TypedRequest.FieldType.INT16), "perm", "f"))
        assertEquals(0.1, TypedRequest.float64(text(0.1, TypedRequest.FieldType.FLOAT64), "perm", "f"))
        assertEquals(0.1f, TypedRequest.float32(text(0.1, TypedRequest.FieldType.FLOAT32), "perm", "f"))
        assertEquals(false, TypedRequest.boolean(text(false, TypedRequest.FieldType.BOOL), "perm", "f"))
        assertEquals("a b", TypedRequest.string(text("a b", TypedRequest.FieldType.STRING), "perm", "f"))
        assertEquals(
            18446744073709549568uL,
            TypedRequest.uint64(text(1.8446744073709550e19, TypedRequest.FieldType.UINT64), "perm", "f"),
        )

        for ((value, type, why) in listOf(
            Triple(256, TypedRequest.FieldType.UINT8, "past the width"),
            Triple(-1, TypedRequest.FieldType.UINT32, "below zero"),
            Triple(1.5, TypedRequest.FieldType.INT32, "not whole"),
            Triple(Double.NaN, TypedRequest.FieldType.FLOAT64, "not finite"),
            Triple(1e39, TypedRequest.FieldType.FLOAT32, "past float32"),
            Triple("2", TypedRequest.FieldType.UINT8, "a text"),
            Triple(1, TypedRequest.FieldType.BOOL, "a number"),
            Triple(1, TypedRequest.FieldType.STRING, "a number"),
            Triple("abc", TypedRequest.FieldType.bytes(2), "past cap"),
            Triple("Ā", TypedRequest.FieldType.bytes(8), "no byte"),
        )) {
            val refused = runCatching { TypedRequest.wire(value, "f", type, fractional) }
                .exceptionOrNull() is EventPayload.Refusal
            assertTrue(refused, "$why: $value was accepted")
        }
    }
}
