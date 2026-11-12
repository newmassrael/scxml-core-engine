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
//   * a post is taken before the run starts and refused once the engine stops,
//     and one naming a session the tree does not hold is refused;
//   * what the inbox holds when the engine stops is applied as a direct call
//     then is: an event joins the queue nothing dequeues and is counted as
//     unseen, and an ending finds its invocation cancelled by the exit;
//   * the external queue takes events in the order they arrived: a post made
//     during a macrostep is applied before an event that macrostep sends to its
//     own session after it (W3C test 201's shape), in both driving modes;
//   * an ending posted for a child session during its parent's macrostep is on
//     that child's queue before an event the parent sends it later in the same
//     macrostep (docs/adr/0084, D2 as amended at step 10), in both driving modes.
//
// The machine is written by hand, as the smallest one with a host-run
// invocation: `working` starts `job` on entry and cancels it on exit, and every
// event it takes is recorded with its `_event.data`. The arrival-order cases use
// a second one, smaller still, that sends itself an event after a callback. The
// child-session cases run the generated `static_invoke_ending_order`
// (sce-build/tests/fixtures/static_datamodel/static_invoke_ending_order.scxml).

package com.sce.integration

import com.sce.integration.static_datamodel.static_invoke_ending_order.StaticInvokeEndingOrderActions
import com.sce.integration.static_datamodel.static_invoke_ending_order.StaticInvokeEndingOrderEvent
import com.sce.integration.static_datamodel.static_invoke_ending_order.StaticInvokeEndingOrderStateMachine
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
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
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

private sealed interface ArrivalProbeState : State {
    data object Waiting : ArrivalProbeState
}

private sealed interface ArrivalProbeEvent : Event {
    data object Go : ArrivalProbeEvent
    data object Posted : ArrivalProbeEvent
    data object Own : ArrivalProbeEvent
}

/**
 * W3C test 201's shape without the HTTP listener. `waiting` takes `go` by
 * calling [whileGoIsTaken], the host's callback inside the macrostep (where a
 * BasicHTTP send waits for its listener), and then sends `own` to its own
 * session, as the document's `<send event="timeout"/>` does. `posted` and `own`
 * are recorded in the order they are taken.
 */
private class ArrivalProbeMachine(
    private val whileGoIsTaken: (ArrivalProbeMachine) -> Unit,
    private val takenLatch: CountDownLatch? = null,
) : StateMachineEngine<ArrivalProbeState, ArrivalProbeEvent>() {
    override val initialState: ArrivalProbeState = ArrivalProbeState.Waiting

    /** What the machine took, in order. Written by the owner, read by the test's thread. */
    val taken = CopyOnWriteArrayList<String>()

    override fun firstEnabledTransition(
        state: ArrivalProbeState,
        event: ArrivalProbeEvent?,
    ): EnabledTransition<ArrivalProbeState, HistoryId>? {
        val index = when (event) {
            null -> return null
            ArrivalProbeEvent.Go -> GO
            ArrivalProbeEvent.Posted -> POSTED
            ArrivalProbeEvent.Own -> OWN
        }
        return EnabledTransition<ArrivalProbeState, HistoryId>(
            state,
            emptyList(),
            index,
            hasActions = true,
            isInternal = false,
        )
    }

    override fun onEntry(state: ArrivalProbeState, isDefaultEntry: Boolean) {}

    override fun onExit(state: ArrivalProbeState) {}

    override fun executeTransitionContent(source: ArrivalProbeState, transitionIndex: Int) {
        val record = when (transitionIndex) {
            GO -> {
                whileGoIsTaken(this)
                send(ArrivalProbeEvent.Own, EventMetadata.external(data = "own"))
                return
            }
            POSTED -> "posted"
            OWN -> "own"
            else -> return
        }
        taken.add(record)
        takenLatch?.countDown()
    }

    override fun resolveState(stateId: String): ArrivalProbeState? =
        if (stateId == "waiting") ArrivalProbeState.Waiting else null

    override fun stateIdOf(state: ArrivalProbeState): String = "waiting"

    override fun resolveEventByName(name: String): ArrivalProbeEvent? = when (name) {
        "go" -> ArrivalProbeEvent.Go
        "posted" -> ArrivalProbeEvent.Posted
        "own" -> ArrivalProbeEvent.Own
        else -> null
    }

    override fun eventNameOf(event: ArrivalProbeEvent): String = when (event) {
        ArrivalProbeEvent.Go -> "go"
        ArrivalProbeEvent.Posted -> "posted"
        ArrivalProbeEvent.Own -> "own"
    }

    companion object {
        const val GO = 0
        const val POSTED = 1
        const val OWN = 2
    }
}

/**
 * The host of `static_invoke_ending_order`: its one host action, `beforeQuestion`,
 * which the parent performs inside the macrostep that takes `ask`, before it
 * sends `question` to `worker`, runs [whileAsked]. A test sets that once it has
 * the machine and the token of the clock's start.
 */
private class AskingHost : StaticInvokeEndingOrderActions {
    @Volatile
    var whileAsked: () -> Unit = {}

    override fun beforeQuestion() {
        whileAsked()
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
            // A post naming a session the tree does not hold is refused, and says so.
            val astray = MeshCall.Raise("ping", EventMetadata.external(data = "astray"))
            assertFalse(
                postFromAnotherThread(sm, InboxInput.MeshDelivery(astray, sessionId = "nowhere")),
                "the inbox took a Mesh delivery naming a session the tree does not hold",
            )
            assertFalse(
                postFromAnotherThread(sm, ending(token, done = true, data = "astray", sessionId = "nowhere")),
                "the inbox took an ending naming a session the tree does not hold",
            )
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

    @Test
    fun whatTheInboxHoldsWhenTheRunIsStoppedIsAppliedAsADirectCallThenIs() {
        val sm = InboxProbeMachine()
        val log = CopyOnWriteArrayList<String>()
        serve(sm, log)
        sm.initialize()
        try {
            val token = latestToken(log)
            // Posted while the owner does not drain: the inbox still holds all four
            // when the run is stopped.
            assertTrue(postFromAnotherThread(sm, InboxInput.ExternalEvent(InboxProbeEvent.Ping, EventMetadata.external(data = "held"))))
            assertTrue(postFromAnotherThread(sm, InboxInput.ExternalEventByName("ping", EventMetadata.external(data = "by-name"))))
            // An ending of the start that is running now, which the stop's exit cancels.
            assertTrue(postFromAnotherThread(sm, ending(token, done = true, data = "held-ending")))
            val raise = MeshCall.Raise("ping", EventMetadata.external(data = "held-mesh"))
            assertTrue(postFromAnotherThread(sm, InboxInput.MeshDelivery(raise)))
            assertEquals(0, sm.unseenExternalEvents(), "an event was counted before the run was stopped")

            sm.stop()

            assertEquals(emptyList<String>(), sm.taken.toList(), "a stopped run took an event")
            // The exit cancelled `job` before the ending was applied: an ending
            // applied first would have ended it, and the exit would have had
            // nothing to cancel.
            assertEquals(listOf("start:$token", "cancel:$token"), log.toList(), "the stop's exit did not cancel `job`")
            // The three events joined a queue nothing dequeues, and the ending,
            // refused by the token check, raised no `done.invoke.job` beside them.
            assertEquals(3, sm.unseenExternalEvents(), "what the inbox held at the stop was not applied as a direct call is")
            assertEquals(InboxProbeEvent.Ping, sm.lastUnseenEvent())

            assertFalse(
                sm.inbox.post(InboxInput.ExternalEvent(InboxProbeEvent.Ping, EventMetadata.external(data = "late"))),
                "a stopped engine's inbox took a post",
            )
            assertEquals(3, sm.unseenExternalEvents(), "a refused post was counted")

            // The next run's inbox starts empty: nothing the stopped run held, or
            // was refused, reaches it.
            sm.initialize()
            assertEquals(emptyList<String>(), sm.taken.toList(), "the next run took what the stopped one held")
            assertTrue(sm.inbox.post(InboxInput.ExternalEvent(InboxProbeEvent.Ping, EventMetadata.external(data = "next"))))
            sm.tick()
            assertEquals(listOf("ping:next"), sm.taken.toList())
            assertEquals(3, sm.unseenExternalEvents(), "the next run counted the stopped run's events again")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun whatAnEngineThatNeverRanHoldsWhenItIsStoppedIsCountedAndNotPostedAgain() {
        val sm = InboxProbeMachine()
        serve(sm, CopyOnWriteArrayList())
        try {
            assertTrue(sm.inbox.post(InboxInput.ExternalEvent(InboxProbeEvent.Ping, EventMetadata.external(data = "early"))))
            val raise = MeshCall.Raise("ping", EventMetadata.external(data = "early-mesh"))
            assertTrue(sm.inbox.post(InboxInput.MeshDelivery(raise)))

            // An engine that never ran has no owner, and a direct `send` on it
            // posts to the inbox. The stop's drain is the owner's all the same:
            // both join the queue the stop counts, rather than being offered to
            // the inbox it has just closed, which would refuse them unsaid.
            sm.stop()
            assertEquals(2, sm.unseenExternalEvents(), "what an engine that never ran held at its stop was dropped")

            sm.initialize()
            assertEquals(emptyList<String>(), sm.taken.toList(), "the first run took what was held at an earlier stop")
            assertEquals(2, sm.unseenExternalEvents())
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The host's callback inside the macrostep: another thread posts `posted`
     * and this one joins it, as a BasicHTTP send waits for the listener that
     * posts the event it caused. The join is the only ordering; nothing sleeps.
     * Answers, through [taken], whether the inbox took the post.
     */
    private fun postAndJoin(sm: ArrivalProbeMachine, taken: AtomicInteger) {
        val posted = InboxInput.ExternalEvent<ArrivalProbeEvent>(
            ArrivalProbeEvent.Posted,
            EventMetadata.external(data = "posted"),
        )
        thrownOnAnotherThread { if (sm.inbox.post(posted)) taken.incrementAndGet() }
    }

    @Test
    fun anEventPostedBeforeTheMachinesOwnSendIsTakenFirst() {
        val postsTaken = AtomicInteger(0)
        val sm = ArrivalProbeMachine({ machine -> postAndJoin(machine, postsTaken) })
        sm.initialize()
        try {
            sm.send(ArrivalProbeEvent.Go)
            sm.tick()
            assertEquals(1, postsTaken.get(), "the callback's post was not taken")
            assertEquals(
                listOf("posted", "own"),
                sm.taken.toList(),
                "the machine's own send overtook an event that arrived before it",
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anEventPostedBeforeACoroutineModeLoopsOwnSendIsTakenFirst() {
        val recorded = CountDownLatch(2)
        val postsTaken = AtomicInteger(0)
        val sm = ArrivalProbeMachine({ machine -> postAndJoin(machine, postsTaken) }, recorded)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            sm.start(scope)
            assertTrue(sm.inbox.post(InboxInput.ExternalEvent(ArrivalProbeEvent.Go, EventMetadata.external())))
            assertTrue(recorded.await(boundSeconds, TimeUnit.SECONDS), "the loop took ${sm.taken}")
            assertEquals(1, postsTaken.get(), "the callback's post was not taken")
            assertEquals(
                listOf("posted", "own"),
                sm.taken.toList(),
                "the loop's own send overtook an event that arrived before it",
            )
        } finally {
            sm.stop()
            scope.cancel()
            sm.cleanup()
        }
    }

    /** The session id the shared fixtures give their root. */
    private val treeRoot = "fixture-root"

    /** The session `worker` is: the root's id and the id its first start is started under. */
    private val treeWorker = "fixture-root.working.1.worker"

    /** The standard host type `worker` asks, and a completion its result record reads. */
    private val clockType = "sce-host-clock-now"
    private val clockNow = """{"error":"","utc_ms":1700000000000,"monotonic_ms":5}"""

    /**
     * `beforeQuestion`'s work: another thread posts the ending of `clock`'s start
     * [token] to [sm]'s inbox, naming `worker`'s session, and the parent's thread
     * joins it before the action returns, so the post is made before the parent's
     * send of `question`. [postsTaken] counts the posts the inbox took, and the
     * test asserts the count once the parent's macrostep is over.
     */
    private fun postClockEnding(sm: StaticInvokeEndingOrderStateMachine, token: Long, postsTaken: AtomicInteger) {
        val ending = InboxInput.HostInvokeEnding(
            sessionId = treeWorker,
            processorType = clockType,
            invokeId = "clock",
            token = token,
            done = true,
            data = clockNow,
        )
        thrownOnAnotherThread { if (sm.inbox.post(ending)) postsTaken.incrementAndGet() }
    }

    /** What the parent heard, in the order it heard it: `stamped` first, then `answer`. */
    private fun assertTheEndingWasTakenFirst(postsTaken: Int, stampedAt: UInt, answerAt: UInt) {
        assertEquals(1, postsTaken, "the inbox refused the ending posted for `worker` while the parent took `ask`")
        assertEquals(
            1u,
            stampedAt,
            "`worker` took the `question` its parent sent after the ending of `clock` was posted for it " +
                "before it took that ending, so the parent heard `stamped` second",
        )
        assertEquals(
            2u,
            answerAt,
            "the parent did not hear `answer` second, after `stamped`: `worker` answered `question` " +
                "ahead of the ending posted for it first",
        )
    }

    @Test
    fun anEndingPostedForAChildBeforeItsParentSendsToItIsTakenByTheChildFirst() {
        val host = AskingHost()
        val sm = StaticInvokeEndingOrderStateMachine(host, sessionId = treeRoot)
        val tokens = CopyOnWriteArrayList<Long>()
        // Records each start's token and does not answer, so `clock` keeps running.
        sm.registerInvoker(clockType) { event ->
            event.start?.let { tokens.add(it.token) }
            null
        }
        val postsTaken = AtomicInteger(0)
        sm.initialize()
        try {
            // `worker` starts with `working`, and starts `clock` as it enters `waiting`.
            repeat(3) { if (tokens.isEmpty()) sm.tick() }
            assertEquals(1, tokens.size, "`worker` did not start `clock` once: $tokens")
            val token = tokens.single()
            host.whileAsked = { postClockEnding(sm, token, postsTaken) }

            sm.send(StaticInvokeEndingOrderEvent.Ask)
            repeat(3) { sm.tick() }

            assertTheEndingWasTakenFirst(postsTaken.get(), sm.stampedAt, sm.answerAt)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anEndingPostedForAChildRunningItsOwnLoopBeforeItsParentSendsToItIsTakenByTheChildFirst() {
        val host = AskingHost()
        val sm = StaticInvokeEndingOrderStateMachine(host, sessionId = treeRoot)
        val tokens = CopyOnWriteArrayList<Long>()
        val started = CountDownLatch(1)
        sm.registerInvoker(clockType) { event ->
            event.start?.let {
                tokens.add(it.token)
                started.countDown()
            }
            null
        }
        val postsTaken = AtomicInteger(0)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            sm.start(scope)
            assertTrue(started.await(boundSeconds, TimeUnit.SECONDS), "`worker` never started `clock` in coroutine mode")
            val token = tokens.single()
            host.whileAsked = { postClockEnding(sm, token, postsTaken) }

            assertTrue(sm.inbox.post(InboxInput.ExternalEvent(StaticInvokeEndingOrderEvent.Ask)), "the root's inbox refused `ask`")
            // Quiet once the parent has heard both: each is numbered as it is heard.
            val settled = runBlocking {
                withTimeout(boundSeconds * 1000) {
                    sm.snapshot.first { it.data.stampedAt != 0u && it.data.answerAt != 0u }
                }
            }

            assertTheEndingWasTakenFirst(postsTaken.get(), settled.data.stampedAt, settled.data.answerAt)
        } finally {
            sm.stop()
            scope.cancel()
            sm.cleanup()
        }
    }
}
