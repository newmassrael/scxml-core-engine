// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, ARCHITECTURE.md "Engine Threading": a Kotlin engine is confined
// to its owner, and what another thread has for it enters through its inbox.
//
// The defect this repays: a host-run invocation's ending made from the host's own
// thread mutated the engine's running-invocation table and its scheduled sends
// beside the thread running a macrostep, without the lock the macrosteps hold.
// The cases hold the replacement to its contract:
//
//   * many threads post endings and external events while macrosteps run, and
//     what the machine takes, and what is left running, are those of a
//     single-threaded replay of the same posts, in both driving modes;
//   * an owner-thread operation called from another thread is refused with an
//     error that names the inbox, and changes nothing;
//   * an ending posted for a start that was cancelled, or started again, is
//     refused by the token check, as a direct one is (§scxml-6.4);
//   * a post is taken before the run starts and refused once the engine stops.
//
// The machine is written by hand, as the smallest one with a host-run
// invocation: `working` starts `job` on entry and cancels it on exit, and every
// event it takes is recorded with its `_event.data`.

package com.sce.integration

import com.sce.runtime.EnabledTransition
import com.sce.runtime.Event
import com.sce.runtime.EventMetadata
import com.sce.runtime.HistoryId
import com.sce.runtime.InboxInput
import com.sce.runtime.MeshCall
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.StateTarget
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.CyclicBarrier
import java.util.concurrent.Semaphore
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Assertions.fail
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

private sealed interface InboxProbeState : State {
    data object Idle : InboxProbeState
    data object Working : InboxProbeState
}

private sealed interface InboxProbeEvent : Event {
    data object Go : InboxProbeEvent
    data object Leave : InboxProbeEvent
    data object Ping : InboxProbeEvent
    data object JobDone : InboxProbeEvent
    data object JobFailed : InboxProbeEvent
}

/**
 * `working` (initial) starts the host-run invocation `job` on entry and cancels
 * it on exit; `leave` goes to `idle` and `go` back. `ping`, `done.invoke.job`
 * and `error.invoke.job` are taken in either state and recorded as
 * `ping:<data>`, `done:<data>` and `failed:<data>`. [takenLatch], when given,
 * is counted down once per recorded event.
 */
private class InboxProbeMachine(
    private val takenLatch: CountDownLatch? = null,
) : StateMachineEngine<InboxProbeState, InboxProbeEvent>() {
    override val initialState: InboxProbeState = InboxProbeState.Working

    /** What the machine took, in order. Written by the owner, read by the test's thread. */
    val taken = CopyOnWriteArrayList<String>()

    override val hostInvokeIds: Set<String> = setOf(JOB)

    private fun transition(source: InboxProbeState, target: InboxProbeState?, index: Int) =
        EnabledTransition<InboxProbeState, HistoryId>(
            source,
            if (target == null) emptyList() else listOf(StateTarget(target)),
            index,
            hasActions = target == null,
            isInternal = false,
        )

    override fun firstEnabledTransition(
        state: InboxProbeState,
        event: InboxProbeEvent?,
    ): EnabledTransition<InboxProbeState, HistoryId>? =
        when (event) {
            null -> null
            InboxProbeEvent.Go ->
                if (state == InboxProbeState.Idle) transition(state, InboxProbeState.Working, GO) else null
            InboxProbeEvent.Leave ->
                if (state == InboxProbeState.Working) transition(state, InboxProbeState.Idle, LEAVE) else null
            InboxProbeEvent.Ping -> transition(state, null, PING)
            InboxProbeEvent.JobDone -> transition(state, null, DONE)
            InboxProbeEvent.JobFailed -> transition(state, null, FAILED)
        }

    override fun onEntry(state: InboxProbeState, isDefaultEntry: Boolean) {
        if (state != InboxProbeState.Working) return
        deferInvoke(state, JOB) {
            performHostInvoke(HostInvokeRequest(processorType = TYPE, invokeId = JOB))
        }
    }

    override fun onExit(state: InboxProbeState) {
        if (state != InboxProbeState.Working) return
        cancelPendingInvokesForState(state)
        cancelHostInvoke(TYPE, JOB)
    }

    override fun executeTransitionContent(source: InboxProbeState, transitionIndex: Int) {
        val data = currentEventMetadata.data
        val record = when (transitionIndex) {
            PING -> "ping:$data"
            DONE -> "done:$data"
            FAILED -> "failed:$data"
            else -> return
        }
        taken.add(record)
        takenLatch?.countDown()
    }

    override fun resolveState(stateId: String): InboxProbeState? = when (stateId) {
        "idle" -> InboxProbeState.Idle
        "working" -> InboxProbeState.Working
        else -> null
    }

    override fun stateIdOf(state: InboxProbeState): String = when (state) {
        InboxProbeState.Idle -> "idle"
        InboxProbeState.Working -> "working"
    }

    override fun documentOrderOf(state: InboxProbeState): Int = when (state) {
        InboxProbeState.Working -> 0
        InboxProbeState.Idle -> 1
    }

    override fun resolveEventByName(name: String): InboxProbeEvent? = when (name) {
        "go" -> InboxProbeEvent.Go
        "leave" -> InboxProbeEvent.Leave
        "ping" -> InboxProbeEvent.Ping
        "done.invoke.$JOB" -> InboxProbeEvent.JobDone
        "error.invoke.$JOB" -> InboxProbeEvent.JobFailed
        else -> null
    }

    override fun eventNameOf(event: InboxProbeEvent): String = when (event) {
        InboxProbeEvent.Go -> "go"
        InboxProbeEvent.Leave -> "leave"
        InboxProbeEvent.Ping -> "ping"
        InboxProbeEvent.JobDone -> "done.invoke.$JOB"
        InboxProbeEvent.JobFailed -> "error.invoke.$JOB"
    }

    companion object {
        const val TYPE = "x-sce-probe"
        const val JOB = "job"
        const val GO = 0
        const val LEAVE = 1
        const val PING = 2
        const val DONE = 3
        const val FAILED = 4
    }
}

@DisplayName("What another thread posts enters through the inbox (docs/adr/0084)")
class WhatAnotherThreadPostsEntersThroughTheInboxTest {

    private val type = InboxProbeMachine.TYPE
    private val job = InboxProbeMachine.JOB

    /** Bounded far above anything this waits for, so a stall names its stage. */
    private val boundSeconds = 10L

    /** The host's half: records `start:<token>` and `cancel:<token>`, and runs the work past the call. */
    private fun serve(sm: InboxProbeMachine, log: MutableList<String>, started: CountDownLatch? = null) {
        sm.registerInvoker(type) { event ->
            event.start?.let {
                log.add("start:${it.token}")
                started?.countDown()
            }
            event.cancel?.let { log.add("cancel:${it.token}") }
            null
        }
    }

    /** The token of the latest start [log] recorded. */
    private fun latestToken(log: List<String>): Long =
        log.lastOrNull { it.startsWith("start:") }?.removePrefix("start:")?.toLong() ?: fail("`job` never started: $log")

    private fun ending(token: Long, done: Boolean, data: String, sessionId: String? = null) =
        InboxInput.HostInvokeEnding(
            sessionId = sessionId,
            processorType = type,
            invokeId = job,
            token = token,
            done = done,
            data = data,
        )

    /**
     * What [producers] threads post, [perProducer] each: external events, typed
     * and by name, and now and then an ending of [token], completed or failed.
     * Each data names its thread and its place, so the record says which post
     * it came from.
     */
    private fun postOf(producer: Int, k: Int, token: Long): InboxInput<InboxProbeEvent> {
        val data = "p$producer-$k"
        return when (k % 7) {
            3 -> ending(token, done = true, data = data)
            5 -> ending(token, done = false, data = data)
            1 -> InboxInput.ExternalEventByName("ping", EventMetadata.external(data = data))
            else -> InboxInput.ExternalEvent(InboxProbeEvent.Ping, EventMetadata.external(data = data))
        }
    }

    /**
     * What the machine must take from [posted], in the order the posts were
     * taken: every event, and of the endings only the first, since it ends `job`
     * and the token check refuses the rest.
     */
    private fun expectedFrom(posted: List<InboxInput<InboxProbeEvent>>): List<String> {
        var ended = false
        val expected = mutableListOf<String>()
        for (input in posted) {
            when (input) {
                is InboxInput.ExternalEvent -> expected += "ping:${input.metadata.data}"
                is InboxInput.ExternalEventByName -> expected += "ping:${input.metadata.data}"
                is InboxInput.HostInvokeEnding -> if (!ended) {
                    ended = true
                    expected += (if (input.done) "done:" else "failed:") + input.data
                }
                is InboxInput.MeshDelivery -> fail<Unit>("this run posts no Mesh delivery")
            }
        }
        return expected
    }

    /**
     * Start [producers] threads, release them together, and have each post
     * [perProducer] inputs to [sm]. The order the inbox took them in is recorded
     * under one lock with the post, so it is the order of the inbox itself.
     * [whileRunning] is the owner's work for as long as any producer runs; each
     * producer releases [wake] when it has posted everything, after it stopped
     * counting itself as running, so an owner asleep on [wake] is not left
     * waiting by the last one.
     */
    private fun postFromManyThreads(
        sm: InboxProbeMachine,
        token: Long,
        producers: Int,
        perProducer: Int,
        wake: Semaphore,
        whileRunning: () -> Unit,
    ): List<InboxInput<InboxProbeEvent>> {
        val posted = mutableListOf<InboxInput<InboxProbeEvent>>()
        val order = Any()
        val failures = ConcurrentLinkedQueue<String>()
        val running = AtomicInteger(producers)
        val together = CyclicBarrier(producers + 1)
        val threads = (0 until producers).map { producer ->
            Thread({
                try {
                    together.await(boundSeconds, TimeUnit.SECONDS)
                    for (k in 0 until perProducer) {
                        val input = postOf(producer, k, token)
                        synchronized(order) {
                            if (sm.inbox.post(input)) posted += input else failures += "post $producer/$k was refused"
                        }
                    }
                } catch (failure: Throwable) {
                    failures += "producer $producer: $failure"
                } finally {
                    running.decrementAndGet()
                    wake.release()
                }
            }, "inbox-producer-$producer").also { it.start() }
        }
        together.await(boundSeconds, TimeUnit.SECONDS)
        while (running.get() > 0) whileRunning()
        for (thread in threads) {
            thread.join(boundSeconds * 1000)
            assertFalse(thread.isAlive, "${thread.name} never finished posting")
        }
        assertTrue(failures.isEmpty(), "an open inbox refused posts: $failures")
        return synchronized(order) { posted.toList() }
    }

    /** [posted] replayed on a machine of its own by one thread: every post, then one tick. */
    private fun replay(posted: List<InboxInput<InboxProbeEvent>>): Pair<List<String>, List<String>> {
        val sm = InboxProbeMachine()
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log)
        sm.initialize()
        try {
            for (input in posted) assertTrue(sm.inbox.post(input), "the replay's inbox refused a post")
            sm.tick()
            sm.stop()
            return sm.taken.toList() to log.toList()
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun postsFromManyThreadsDuringMacrostepsAreTakenAsASingleThreadedReplayTakesThem() {
        val sm = InboxProbeMachine()
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log)
        // The waker is what tells this owner there is something to apply; the
        // owner sleeps on it between ticks rather than on a clock.
        val wake = Semaphore(0)
        sm.inbox.wakeWith { wake.release() }
        sm.initialize()
        try {
            val token = latestToken(log)
            val posted = postFromManyThreads(sm, token, producers = 8, perProducer = 60, wake = wake) {
                wake.tryAcquire(boundSeconds, TimeUnit.SECONDS)
                sm.tick()
            }
            // What the last producer posted after the owner's last tick.
            sm.tick()
            // The invocation table: the first ending ended `job`, so the stop that
            // exits `working` has nothing to cancel.
            sm.stop()

            val expected = expectedFrom(posted)
            assertEquals(expected, sm.taken.toList(), "the machine did not take the posts in the order the inbox took them")
            assertEquals(listOf("start:$token"), log.toList(), "an ending was not applied, or a refused one was")
            val (replayTaken, replayLog) = replay(posted)
            assertEquals(replayTaken, sm.taken.toList(), "the run differs from a single-threaded replay of its posts")
            assertEquals(replayLog, log.toList(), "the invocations differ from a single-threaded replay of the posts")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun postsFromManyThreadsReachACoroutineModeLoopAsTheyReachTheReplay() {
        val producers = 6
        val perProducer = 40
        // Every post but the refused endings is recorded: count them first.
        val expectedCount = expectedFrom(
            (0 until producers).flatMap { p -> (0 until perProducer).map { k -> postOf(p, k, 0L) } },
        ).size
        val recorded = CountDownLatch(expectedCount)
        val started = CountDownLatch(1)
        val sm = InboxProbeMachine(recorded)
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log, started)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            sm.start(scope)
            assertTrue(started.await(boundSeconds, TimeUnit.SECONDS), "`job` was never started in coroutine mode")
            val token = latestToken(log)
            // The loop owns the engine and wakes on each post; the test's thread
            // only waits for the producers.
            val done = Semaphore(0)
            val posted = postFromManyThreads(sm, token, producers, perProducer, done) {
                done.tryAcquire(boundSeconds, TimeUnit.SECONDS)
            }
            assertTrue(recorded.await(boundSeconds, TimeUnit.SECONDS), "the loop took ${sm.taken.size} of $expectedCount")
            sm.stop()

            assertEquals(expectedFrom(posted), sm.taken.toList(), "the loop did not take the posts in the order the inbox took them")
            assertEquals(listOf("start:$token"), log.toList(), "an ending was not applied, or a refused one was")
            val (replayTaken, replayLog) = replay(posted)
            assertEquals(replayTaken, sm.taken.toList(), "the run differs from a single-threaded replay of its posts")
            assertEquals(replayLog, log.toList())
        } finally {
            sm.stop()
            scope.cancel()
            sm.cleanup()
        }
    }

    /** Run [call] on a thread of its own and answer what it threw. */
    private fun thrownOnAnotherThread(call: () -> Unit): Throwable? {
        var thrown: Throwable? = null
        val thread = Thread {
            try {
                call()
            } catch (failure: Throwable) {
                thrown = failure
            }
        }
        thread.start()
        thread.join(boundSeconds * 1000)
        assertFalse(thread.isAlive, "the call on another thread never returned")
        return thrown
    }

    /** [call], the owner-thread operation [name], made from another thread: refused, naming itself and the inbox. */
    private fun refusedOnAnotherThread(name: String, call: () -> Unit) {
        assertRefusal(name, thrownOnAnotherThread(call))
    }

    private fun assertRefusal(name: String, thrown: Throwable?) {
        assertTrue(thrown is IllegalStateException, "$name off its owner was not refused: $thrown")
        val message = thrown?.message ?: ""
        assertTrue(name in message, "the refusal does not name `$name`: $message")
        assertTrue("inbox" in message, "the refusal does not name the inbox: $message")
    }

    @Test
    fun anOwnerThreadOperationFromAnotherThreadIsRefusedAndNamesTheInbox() {
        val sm = InboxProbeMachine()
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log)
        sm.initialize()
        try {
            val token = latestToken(log)
            refusedOnAnotherThread("send") { sm.send(InboxProbeEvent.Ping, EventMetadata.external(data = "off")) }
            refusedOnAnotherThread("sendEventByName") { sm.sendEventByName("ping", EventMetadata.external(data = "off")) }
            refusedOnAnotherThread("completeHostInvoke") { sm.completeHostInvoke(type, job, token, "off") }
            refusedOnAnotherThread("failHostInvoke") { sm.failHostInvoke(type, job, token, "off") }
            refusedOnAnotherThread("completeHostInvokeIn") { sm.completeHostInvokeIn(sm.sessionId, type, job, token, "off") }
            refusedOnAnotherThread("failHostInvokeIn") { sm.failHostInvokeIn(sm.sessionId, type, job, token, "off") }
            refusedOnAnotherThread("cancelHostInvoke") { sm.cancelHostInvoke(type, job) }
            refusedOnAnotherThread("applyMeshCalls") { sm.applyMeshCalls() }
            refusedOnAnotherThread("applyMeshCall") {
                sm.applyMeshCall(MeshCall.Raise("ping", EventMetadata.external(data = "off")))
            }
            refusedOnAnotherThread("tick") { sm.tick() }
            refusedOnAnotherThread("advanceTimeMs") { sm.advanceTimeMs(0) }

            // Nothing the refused calls asked for happened: `job` is still running,
            // and no event was queued.
            assertEquals(listOf("start:$token"), log.toList(), "a refused cancel reached the host")
            assertTrue(sm.completeHostInvoke(type, job, token, "owner"), "a refused call ended `job`")
            sm.tick()
            assertEquals(listOf("done:owner"), sm.taken.toList(), "a refused call queued an event")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aCoroutineModeEngineRefusesADirectEndingOffItsLoopAndTakesItsPost() {
        val recorded = CountDownLatch(2)
        val started = CountDownLatch(1)
        val sm = InboxProbeMachine(recorded)
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log, started)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            sm.start(scope)
            assertTrue(started.await(boundSeconds, TimeUnit.SECONDS), "`job` was never started in coroutine mode")
            val token = latestToken(log)

            // The test's own thread is not the loop.
            assertRefusal("completeHostInvoke", runCatching { sm.completeHostInvoke(type, job, token, "direct") }.exceptionOrNull())
            assertRefusal("tick", runCatching { sm.tick() }.exceptionOrNull())
            // `send` off the loop is this mode's post of an external event, as it
            // always was.
            sm.send(InboxProbeEvent.Ping, EventMetadata.external(data = "sent"))
            assertTrue(sm.inbox.post(ending(token, done = true, data = "posted")))
            assertTrue(recorded.await(boundSeconds, TimeUnit.SECONDS), "the loop took ${sm.taken}")
            assertEquals(listOf("ping:sent", "done:posted"), sm.taken.toList())
        } finally {
            sm.stop()
            scope.cancel()
            sm.cleanup()
        }
    }

    /** Post [input] from a thread of its own, as a host's worker does, and answer whether it was taken. */
    private fun postFromAnotherThread(sm: InboxProbeMachine, input: InboxInput<InboxProbeEvent>): Boolean {
        var taken = false
        assertEquals(null, thrownOnAnotherThread { taken = sm.inbox.post(input) })
        return taken
    }

    @Test
    fun aPostedEndingOfACancelledOrRestartedStartIsRefusedAsADirectOneIs() {
        val sm = InboxProbeMachine()
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log)
        sm.initialize()
        try {
            val first = latestToken(log)
            // `leave` exits `working`, which cancels the first start.
            sm.send(InboxProbeEvent.Leave)
            sm.tick()
            assertEquals(listOf("start:$first", "cancel:$first"), log.toList())

            assertTrue(postFromAnotherThread(sm, ending(first, done = true, data = "after-cancel")))
            sm.tick()
            assertEquals(emptyList<String>(), sm.taken.toList(), "a cancelled start's posted ending was taken")
            assertFalse(sm.completeHostInvoke(type, job, first, "direct"), "a cancelled start's direct ending was taken")

            // `go` enters `working` again: the same invocation, a new start.
            sm.send(InboxProbeEvent.Go)
            sm.tick()
            val second = latestToken(log)
            assertTrue(first != second, "a restart reused the first start's token")

            assertTrue(postFromAnotherThread(sm, ending(first, done = false, data = "stale")))
            // Addressed by the session id, as a host serving a tree ends one.
            assertTrue(postFromAnotherThread(sm, ending(second, done = true, data = "fresh", sessionId = sm.sessionId)))
            sm.tick()
            assertEquals(listOf("done:fresh"), sm.taken.toList(), "the first start's ending was taken for the second's")
            assertFalse(sm.failHostInvoke(type, job, first, "direct"), "the first start's direct ending was taken")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aMeshDeliveryIsPostedAndAppliedAsApplyMeshCallAppliesIt() {
        val sm = InboxProbeMachine()
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log)
        sm.initialize()
        try {
            val token = latestToken(log)
            val raise = MeshCall.Raise("ping", EventMetadata.external(data = "peer"))
            assertTrue(postFromAnotherThread(sm, InboxInput.MeshDelivery(raise)))
            // Addressed by the session id: this machine.
            val addressed = MeshCall.Raise("ping", EventMetadata.external(data = "addressed"))
            assertTrue(postFromAnotherThread(sm, InboxInput.MeshDelivery(addressed, sessionId = sm.sessionId)))
            // A session the tree does not hold takes nothing.
            val astray = MeshCall.Raise("ping", EventMetadata.external(data = "astray"))
            assertTrue(postFromAnotherThread(sm, InboxInput.MeshDelivery(astray, sessionId = "nowhere")))
            sm.tick()
            assertEquals(listOf("ping:peer", "ping:addressed"), sm.taken.toList())
            assertEquals(listOf("start:$token"), log.toList())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aPostBeforeTheRunIsTakenByItAndOneAfterTheStopIsRefused() {
        val sm = InboxProbeMachine()
        serve(sm, CopyOnWriteArrayList())
        var woken = 0
        sm.inbox.wakeWith { woken++ }
        try {
            assertTrue(sm.inbox.post(InboxInput.ExternalEvent(InboxProbeEvent.Ping, EventMetadata.external(data = "early"))))
            assertEquals(1, woken, "a taken post did not wake the owner")
            assertEquals(emptyList<String>(), sm.taken.toList(), "a post was applied before the run started")
            // Applied at the first macrostep boundary of the run.
            sm.initialize()
            assertEquals(listOf("ping:early"), sm.taken.toList())

            sm.stop()
            assertFalse(
                sm.inbox.post(InboxInput.ExternalEvent(InboxProbeEvent.Ping, EventMetadata.external(data = "late"))),
                "a stopped engine's inbox took a post",
            )
            assertEquals(1, woken, "a refused post woke the owner")

            // A run started again has an open inbox.
            sm.initialize()
            assertTrue(sm.inbox.post(InboxInput.ExternalEventByName("ping", EventMetadata.external(data = "again"))))
            sm.tick()
            assertEquals(listOf("ping:early", "ping:again"), sm.taken.toList())
        } finally {
            sm.cleanup()
        }
    }
}
