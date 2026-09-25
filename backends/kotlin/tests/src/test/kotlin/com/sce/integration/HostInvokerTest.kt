// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 — Kotlin compile+run gate for an `<invoke type>` the HOST
// runs.
//
// The Kotlin engine carried the registry (`registerInvoker`) and the template
// lowered the start and the cancel, but no Kotlin test ran either, so the one
// backend the calendar app ships on held the lifecycle on trust. This is the
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
import com.sce.integration.statechart_host_invoker.StatechartHostInvokerStateMachine
import com.sce.runtime.EventMetadata
import com.sce.runtime.StateMachineEngine
import com.sce.w3c.W3CTestBase
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
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
}
