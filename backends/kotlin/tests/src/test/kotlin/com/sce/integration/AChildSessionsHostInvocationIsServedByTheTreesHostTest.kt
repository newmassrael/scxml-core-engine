// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Running host invocations" (docs/adr/0045, section
// 3) — Kotlin half of the Rust suite's
// `a_child_sessions_host_invocation_is_served_by_the_trees_host.rs`: the invokers a
// host registers on the machine it built serve every session of that machine's
// tree. Each request names the session that made it, and the host ends an
// invocation of any session through the machine it holds, by that session's id and
// the request's token.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml,
// whose in-place child `worker` asks its host for the time (`sce-host-clock-now`,
// id `clock`) as it enters `second`, tells its parent `stamped` when the host
// answers, and keeps the id its grandchild is started under in `helper_id`
// (`idlocation`).

package com.sce.integration

import com.sce.integration.static_datamodel.static_invoke_tree.StaticInvokeTreeStateMachine
import com.sce.runtime.EventMetadata
import com.sce.runtime.InboxInput
import com.sce.runtime.Json
import com.sce.runtime.ManualClock
import com.sce.runtime.MeshCall
import com.sce.runtime.SavedInvokes
import com.sce.runtime.SavedState
import com.sce.runtime.StateMachineEngine
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class AChildSessionsHostInvocationIsServedByTheTreesHostTest {

    /** The session id the shared fixtures give their root. */
    private val root = "fixture-root"

    /** The session `worker` is: the root's id and the id its first start is started under (`<state>.<start>.<invoke>`). */
    private val worker = "fixture-root.working.1.worker"

    /** The standard host type the child asks. */
    private val clock = "sce-host-clock-now"

    /** A completion of the clock the child's result record reads. */
    private val now = """{"error":"","utc_ms":1700000000000,"monotonic_ms":5}"""

    /** The wall-clock moment the tree is saved at. */
    private val savedAtMs = 1_700_000_000_000L

    /** Every start the host was handed, in order. */
    private class Starts {
        val requests = mutableListOf<StateMachineEngine.HostInvokeRequest>()

        fun only(): StateMachineEngine.HostInvokeRequest {
            assertEquals(1, requests.size, "one start: $requests")
            return requests.single()
        }
    }

    /** Register the clock on [sm], the root, recording each start. */
    private fun serve(sm: StaticInvokeTreeStateMachine): Starts {
        val starts = Starts()
        sm.registerInvoker(clock) { event ->
            event.start?.let { starts.requests.add(it) }
            null
        }
        return starts
    }

    /** A root built under the fixtures' root id, served, and initialized. */
    private fun started(): Pair<StaticInvokeTreeStateMachine, Starts> {
        val sm = StaticInvokeTreeStateMachine(sessionId = root)
        sm.clock = ManualClock(0)
        val starts = serve(sm)
        sm.initialize()
        return sm to starts
    }

    /** `go`, which the root forwards to the child in one tick and the child takes, entering `second` and starting the clock, in the next. */
    private fun intoSecond(sm: StaticInvokeTreeStateMachine) {
        sm.sendEventByName("go")
        sm.tick()
        sm.tick()
    }

    @Test
    fun aChildsInvocationReachesTheRootsInvokerAndIsEndedThroughTheRoot() {
        val (sm, starts) = started()
        try {
            intoSecond(sm)
            val request = starts.only()
            assertEquals(clock, request.processorType)
            assertEquals("clock", request.invokeId)
            assertEquals(worker, request.sessionId, "the request names the child session that made it")
            assertFalse(request.restarted)

            assertTrue(
                sm.completeHostInvokeIn(worker, clock, "clock", request.token, now),
                "the child is running that start",
            )
            sm.tick()
            assertEquals(1u, sm.stamps, "the child took done.invoke.clock and told its parent")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anEndingForASessionTheTreeDoesNotHoldOrAStartItDoesNotRunIsRefused() {
        val (sm, starts) = started()
        try {
            intoSecond(sm)
            val request = starts.only()
            for (nowhere in listOf("fixture-root.nowhere", "fixture-root.working.1.worker.nowhere", "elsewhere")) {
                assertFalse(
                    sm.completeHostInvokeIn(nowhere, clock, "clock", request.token, now),
                    "no such session: $nowhere",
                )
            }
            // The root runs no clock of its own.
            assertFalse(sm.completeHostInvokeIn(root, clock, "clock", request.token, now), "the root runs no such start")
            // A start ended once is ended: the second ending is stale.
            assertTrue(sm.completeHostInvokeIn(worker, clock, "clock", request.token, now), "the first ending")
            assertFalse(sm.failHostInvokeIn(worker, clock, "clock", request.token, "late"), "the start has ended")
            sm.tick()
            assertEquals(1u, sm.stamps, "the refused endings raised nothing")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aResumedChildsInvocationStartsAgainAtItsFirstDrive() {
        val (sm, starts) = started()
        val text = try {
            intoSecond(sm)
            val first = starts.only()
            assertEquals(worker, first.sessionId)
            sm.save(savedAtMs).toJson()
        } finally {
            sm.cleanup()
        }
        val firstToken = starts.only().token

        val restored = StaticInvokeTreeStateMachine()
        try {
            restored.clock = ManualClock(0)
            restored.restore(SavedState.fromJson(text), savedAtMs)
            // The host registers its invokers on the machine the restore handed it,
            // after the restore built the child: nothing has started yet.
            val again = serve(restored)
            assertTrue(again.requests.isEmpty(), "a restore calls no host")

            // The parent's first drive of the child is the child's first macrostep.
            restored.tick()
            val request = again.only()
            assertTrue(request.restarted, "started again from the saved request")
            assertEquals(worker, request.sessionId)
            assertEquals("clock", request.invokeId)
            assertNotEquals(firstToken, request.token, "a new token")

            assertTrue(restored.completeHostInvokeIn(worker, clock, "clock", request.token, now), "the restarted start")
            restored.tick()
            assertEquals(1u, restored.stamps)
        } finally {
            restored.cleanup()
        }
    }

    /** The ending of [request] a host posts, and a Mesh delivery of `question` for `worker`. */
    private fun postsFor(request: StateMachineEngine.HostInvokeRequest): List<InboxInput<Nothing>> = listOf(
        InboxInput.HostInvokeEnding(
            sessionId = request.sessionId,
            processorType = clock,
            invokeId = "clock",
            token = request.token,
            done = true,
            data = now,
        ),
        InboxInput.MeshDelivery(MeshCall.Raise("question", EventMetadata.external()), sessionId = worker),
    )

    /**
     * docs/adr/0084, D2 as amended at step 10: what a host's own thread has for a
     * child session is posted through the inbox of the machine the host holds,
     * naming the session, and goes straight to that session's inbox. The post
     * wakes the tree's owner through the waker the host installed on the root, and
     * the child applies it at its own macrostep boundary with the call the host
     * would have made on its owner (`completeHostInvoke`, `applyMeshCall`), when
     * the host's next tick drives it.
     */
    @Test
    fun whatAnotherThreadPostsForAChildReachesItThroughItsOwnInbox() {
        val (sm, starts) = started()
        try {
            intoSecond(sm)
            val request = starts.only()
            val wakes = AtomicInteger(0)
            sm.inbox.wakeWith { wakes.incrementAndGet() }
            var taken = emptyList<Boolean>()
            val host = Thread { taken = postsFor(request).map { sm.inbox.post(it) } }
            host.start()
            host.join(10_000)
            assertEquals(listOf(true, true), taken, "the inbox of the session the posts name refused one")
            assertEquals(2, wakes.get(), "a post for a child session did not wake the tree's owner")

            sm.tick()
            assertEquals(1u, sm.stamps, "the host's next tick did not have the child take done.invoke.clock")
            assertEquals(1u, sm.answers, "the host's next tick did not have the child take the Mesh delivery")
            assertFalse(
                sm.completeHostInvokeIn(worker, clock, "clock", request.token, now),
                "the posted ending did not end the start",
            )
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The tree's directory holds a child from its start to its end: once the
     * parent has left `working`, which cancels `worker`, a post naming `worker` is
     * refused and says so, and a post naming the root is still taken.
     */
    @Test
    fun aPostForAChildSessionThatHasEndedIsRefused() {
        val (sm, starts) = started()
        try {
            intoSecond(sm)
            val request = starts.only()
            sm.sendEventByName("rest")
            sm.tick()
            var taken = emptyList<Boolean>()
            val host = Thread {
                taken = postsFor(request).map { sm.inbox.post(it) } +
                    sm.inbox.post(InboxInput.MeshDelivery(MeshCall.Raise("back", EventMetadata.external()), sessionId = root))
            }
            host.start()
            host.join(10_000)
            assertEquals(
                listOf(false, false, true),
                taken,
                "a post naming the ended `worker` was taken, or one naming the root was refused",
            )
            sm.tick()
            assertEquals(0u, sm.stamps, "an ending posted for the ended `worker` raised an event")
        } finally {
            sm.cleanup()
        }
    }

    /**
     * A child released by its parent's `cleanup` closes its inbox and leaves the
     * tree's directory as one that ended does: a post naming `worker` is refused
     * from then on, never taken by an inbox nothing will drain.
     */
    @Test
    fun aPostForAChildSessionReleasedByItsParentsCleanupIsRefused() {
        val (sm, starts) = started()
        try {
            intoSecond(sm)
            val request = starts.only()
            sm.cleanup()
            var taken = emptyList<Boolean>()
            val host = Thread { taken = postsFor(request).map { sm.inbox.post(it) } }
            host.start()
            host.join(10_000)
            assertEquals(
                listOf(false, false),
                taken,
                "a post naming `worker` was taken after its parent's cleanup released it",
            )
        } finally {
            sm.cleanup()
        }
    }

    /**
     * In the coroutine mode the child runs a loop of its own and is its own owner:
     * the root's loop does not reach into it. What was posted for it goes straight
     * to its inbox, and the child applies it on its loop.
     */
    @Test
    fun whatIsPostedForAChildRunningItsOwnLoopIsHandedToThatChild() = runBlocking {
        val sm = StaticInvokeTreeStateMachine(sessionId = root)
        sm.clock = ManualClock(0)
        val asked = CompletableDeferred<StateMachineEngine.HostInvokeRequest>()
        sm.registerInvoker(clock) { event ->
            event.start?.let { asked.complete(it) }
            null
        }
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            sm.start(scope)
            // Off the root's loop, a raise by name is a post to its inbox.
            sm.sendEventByName("go")
            val request = withTimeout(8_000) { asked.await() }
            assertEquals(worker, request.sessionId, "the request names the child session that made it")
            for (input in postsFor(request)) assertTrue(sm.inbox.post(input), "the root's inbox refused a post")

            val settled = withTimeout(8_000) {
                sm.snapshot.first { it.data.stamps == 1u && it.data.answers == 1u }
            }
            assertEquals(1u, settled.data.stamps)
        } finally {
            sm.stop()
            scope.cancel()
        }
    }

    @Test
    fun anInvokeIdlocationKeepsTheIdTheGrandchildIsStartedUnder() {
        // docs/adr/0045: the grandchild is started under `second.1.<its invoke id>`,
        // the `invokeid` a saved tree holds for it, and `helper_id` holds the same.
        val (sm, _) = started()
        try {
            intoSecond(sm)
            val saved = sm.save(savedAtMs)
            val child = (saved.invokes as SavedInvokes.Resumed).children[0].session
            val grandchildren = (child.invokes as SavedInvokes.Resumed).children
            assertEquals(1, grandchildren.size, "the hybrid candidate runs")
            val startedUnder = grandchildren[0].invokeId
            assertTrue(startedUnder.startsWith("second.1."), "the first start of the invoke of `second`: $startedUnder")
            assertEquals(startedUnder, child.variable("helper_id"))
            assertNotEquals(Json.Number("0"), child.variable("steps"))
        } finally {
            sm.cleanup()
        }
    }
}
