// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045,
// section 8) — Kotlin half of the Rust suite's
// `a_saved_tree_holds_every_session_where_it_stood.rs`: the tree fixture saved
// byte for byte, restored from that text, and refused for a grandchild of another
// shape and a child of a later revision.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml.
// The shared text, sce-build/tests/fixtures/static_datamodel/saved/
// static_invoke_tree_working.json, is the tree `working` leaves:
//
// - `worker` was started twice (`rest`, `back`), and has taken `go`: it counted a
//   step, holds a delayed `report` to its parent, asks its host for the time (a
//   host-run invocation that has not answered), and runs a grandchild under the
//   hybrid candidate `static_hybrid_watcher`, whose `pick` was changed to the other
//   candidate after it started (`swap`); it keeps the id the grandchild was started
//   under in `helper_id` (`idlocation`).
// - The parent holds a delayed `nudge` to the child (`arm`) and the child's `answer`
//   in its queue; the child holds the parent's `swap` in its own.
//
// Rust is the first backend to write the text, so it is the run's own save, and
// this suite holds Kotlin's save to it.

package com.sce.integration

import com.sce.integration.static_invoke_tree.StaticInvokeTreeStateMachine
import com.sce.runtime.Json
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedAct
import com.sce.runtime.SavedChild
import com.sce.runtime.SavedInvokes
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class ASavedTreeHoldsEverySessionWhereItStoodTest {

    /** The session id the shared format-2 fixtures give their root. */
    private val root = "fixture-root"

    /** The wall-clock moment the tree is saved at. */
    private val savedAtMs = 1_700_000_000_000L

    /** The standard host type the child asks. */
    private val clock = "sce-host-clock-now"

    private val shared: String = File(
        repoRoot(),
        "sce-build/tests/fixtures/static_datamodel/saved/static_invoke_tree_working.json",
    ).readText().trim()

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    /** How many times the host was asked for the time. */
    private var asked = 0

    /**
     * The run the shared text is the save of. The host serves the clock and never
     * answers, so the invocation is running when the tree is saved.
     */
    private fun working(): StaticInvokeTreeStateMachine {
        asked = 0
        val sm = StaticInvokeTreeStateMachine(sessionId = root)
        sm.clock = ManualClock(0)
        sm.registerInvoker(clock) { event ->
            if (event.start != null) asked++
            null
        }
        sm.initialize()
        // Started twice: left and entered again.
        for (event in listOf("rest", "back")) {
            sm.sendEventByName(event)
            sm.tick()
        }
        // `go`, forwarded in one tick and taken by the child in the next.
        sm.sendEventByName("go")
        sm.tick()
        sm.tick()
        assertEquals(1, asked, "the child asked its host once")
        // `swap`, forwarded and taken: `pick` names the other candidate now.
        sm.sendEventByName("swap")
        sm.tick()
        sm.tick()
        // A delayed send to the child.
        sm.sendEventByName("arm")
        sm.tick()
        // `ask`, then `swap`, taken two to a call: the child answers `question` at
        // once, behind the host's `swap`, and the call ends after the parent takes
        // `swap`, which it forwards to the child, so the child's answer waits in the
        // parent's queue and the parent's `swap` in the child's.
        sm.setMaxExternalEventsPerCall(2)
        sm.sendEventByName("ask")
        sm.sendEventByName("swap")
        sm.tick()
        return sm
    }

    private fun saved(sm: StaticInvokeTreeStateMachine): SavedState = sm.save(savedAtMs)

    private fun children(session: SavedState): List<SavedChild> =
        (session.invokes as? SavedInvokes.Resumed ?: error("a format-2 session")).children

    private fun variable(session: SavedState, id: String): Any? = session.variable(id)

    private fun <T> restored(text: String, body: (StaticInvokeTreeStateMachine) -> T): T {
        val sm = StaticInvokeTreeStateMachine()
        try {
            sm.clock = ManualClock(0)
            sm.restore(SavedState.fromJson(text), savedAtMs)
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theTreeHoldsEverySessionWhereItStood() {
        val sm = working()
        try {
            val rootState = saved(sm)
            assertEquals(root, rootState.sessionId)
            assertEquals("working", rootState.current)
            assertEquals(mapOf("worker" to 2L), rootState.invokeStarts, "the worker was started twice")
            assertEquals(
                listOf("answer"),
                rootState.external.map { it.name },
                "the child's answer waits in the parent's queue",
            )
            assertEquals(
                listOf("nudge"),
                rootState.pending.mapNotNull { (it.act as? SavedAct.Child)?.event },
                "the parent holds a delayed send to the child",
            )

            val worker = children(rootState).single()
            assertEquals("worker", worker.id)
            assertEquals(2L, worker.start)
            assertEquals("working.2.worker", worker.invokeId)
            val child = worker.session
            assertEquals("fixture-root.working.2.worker", child.sessionId)
            assertEquals("second", child.current, "the child took `go`")
            assertEquals(Json.Number("1"), variable(child, "steps"))
            assertEquals(
                "static_hybrid_holder.scxml",
                variable(child, "pick"),
                "`pick` was changed after the grandchild started",
            )
            assertEquals(
                listOf("swap"),
                child.external.map { it.name },
                "the parent's `swap` waits in the child's queue",
            )
            assertEquals(
                listOf("report"),
                child.pending.mapNotNull { (it.act as? SavedAct.Parent)?.event },
                "the child holds a delayed send to its parent",
            )
            assertEquals(
                listOf(clock to "clock"),
                child.hostInvokes.map { it.processorType to it.invokeId },
                "the child's host-run invocation runs",
            )

            val helper = children(child).single()
            assertEquals(
                "static_hybrid_watcher",
                helper.document,
                "the candidate that was started, whatever `pick` names now",
            )
            assertEquals(1L, helper.start)
            assertTrue(helper.invokeId.startsWith("second.1."), helper.invokeId)
            assertEquals(
                helper.invokeId,
                variable(child, "helper_id"),
                "the idlocation holds the id the grandchild was started under",
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theTreeSavesTheSharedTextByteForByte() {
        val sm = working()
        try {
            assertEquals(shared, saved(sm).toJson(), "the tree's text is the shared one")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theSharedTextRestoresAndSavesAgainAsItWas() {
        restored(shared) { sm ->
            assertEquals(shared, saved(sm).toJson(), "a restored tree stands where the saved one stood")
        }
    }

    @Test
    fun aGrandchildOfAnotherShapeRefusesTheTreeByItsPath() {
        val shape = "a85ea43d8de628b004074e59c0ca90c198a4f7736073bd953750bca35c174c7a"
        val text = shared.replace(shape, "0".repeat(64))
        assertNotEquals(shared, text, "the change applied")
        val sm = StaticInvokeTreeStateMachine()
        try {
            val refusal = assertFailsWith<StateRefusal> { sm.restore(SavedState.fromJson(text), savedAtMs) }
            assertTrue(
                refusal.message!!.contains("invokes[0].session.invokes[0].session"),
                "the refusal names the grandchild's path: ${refusal.message}",
            )
            assertTrue(refusal.message!!.contains("shape"), refusal.message)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aChildOfALaterRevisionRefusesTheTreeByItsPath() {
        // The child holds a record of `schema_revised.scxml`, saved at its third
        // revision; a fourth this build does not know is refused, naming the child.
        assertTrue(shared.contains("\"revision\":3"), "the child names the revision of its record")
        val later = shared.replace("\"revision\":3", "\"revision\":4")
        val sm = StaticInvokeTreeStateMachine()
        try {
            val refusal = assertFailsWith<StateRefusal> { sm.restore(SavedState.fromJson(later), savedAtMs) }
            assertTrue(refusal.message!!.contains("invokes[0].session"), "the refusal names the child's path: ${refusal.message}")
            assertTrue(refusal.message!!.contains("later than"), refusal.message)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aSaveAfterAnyHostCallFindsNothingInTransit() {
        // Half of the Rust suite's `nothing_is_in_transit_when_a_tree_is_saved.rs`.
        // The refusal itself has no Kotlin test, because the Kotlin parent has no
        // buffer an event of a child waits in between two sessions (what a child
        // sends `#_parent` is handed to its parent where it is sent), so there is
        // nothing for a save to meet. What the refusal guards is held here: after
        // each call of a run that drives the child every way there is (a send made
        // at once, a delayed one armed, a forwarded event, a tick, a start again),
        // the tree saves, the text it saves restores, and the restored tree saves
        // that text again.
        val sm = StaticInvokeTreeStateMachine(sessionId = root)
        try {
            sm.clock = ManualClock(0)
            sm.registerInvoker(clock) { _ -> null }
            sm.initialize()
            val run = listOf("ask", "go", "ask", "swap", "ask", "arm", "ask", "rest", "back", "ask", "go", "ask")
            for (event in run) {
                // The two calls a host makes: the event it raises, then the tick.
                sm.sendEventByName(event)
                assertSavesAndRestores(sm, "raising `$event`")
                sm.tick()
                assertSavesAndRestores(sm, "ticking after `$event`")
            }
        } finally {
            sm.cleanup()
        }
    }

    /** The tree saves where it stands, and what it saves restores and saves again as it was. */
    private fun assertSavesAndRestores(sm: StaticInvokeTreeStateMachine, after: String) {
        val text = try {
            saved(sm).toJson()
        } catch (refusal: StateRefusal) {
            error("the tree saves at every boundary, but after $after: ${refusal.message}")
        }
        restored(text) { copy ->
            assertEquals(text, saved(copy).toJson(), "the tree restored after $after stands where it was saved")
        }
    }
}
