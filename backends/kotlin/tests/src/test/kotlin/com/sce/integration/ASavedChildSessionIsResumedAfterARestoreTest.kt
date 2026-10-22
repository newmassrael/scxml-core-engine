// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045) — Kotlin half
// of the Rust suite's `a_saved_child_session_is_resumed_after_a_restore.rs`.
//
// A child session an `<invoke type="scxml">` started is part of what a machine is
// doing, so a saved state holds each running child inside its parent, with its
// whole state, and a restore resumes it where it stood. Nothing a child did at
// its start happens again, and it raises its `done.invoke` once, when it ends.
//
// `static_invoke.scxml` invokes `worker`, a child that takes `a` and then `b` and
// ends; the parent forwards what it is sent (`autoforward`), so the host drives
// the child through the parent and every case is read from the parent:
// `done.invoke.worker` counts the run in `completed` and leaves the parent in
// `working`. A child that had taken `a` is resumed at `second`, so `b` alone ends
// it.
//
// The shared instance `saved/static_invoke_working.json` is the format-2 text of
// the machine just started, its root given the id the fixtures name it by. A
// format-1 text holds the child's id alone, and the build that wrote it promised
// to start the child again from its beginning: `saved/format1/static_invoke_working.json`
// is the one case in which a restore does (docs/adr/0045, section 3).

package com.sce.integration

import com.sce.integration.static_invoke.StaticInvokeEvent
import com.sce.integration.static_invoke.StaticInvokeState
import com.sce.integration.static_invoke.StaticInvokeStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedInvokes
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class ASavedChildSessionIsResumedAfterARestoreTest {

    /** The session id the shared format-2 fixtures give their root. */
    private val root = "fixture-root"

    /** The wall-clock moment the shared instance was saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private fun fixture(path: String): String =
        File(repoRoot(), "sce-build/tests/fixtures/static_datamodel/saved/$path").readText().trim()

    private val shared: String = fixture("static_invoke_working.json")
    private val format1: String = fixture("format1/static_invoke_working.json")

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    private fun started(): StaticInvokeStateMachine {
        val sm = StaticInvokeStateMachine(sessionId = root)
        sm.clock = ManualClock(0)
        sm.initialize()
        return sm
    }

    /** [text] restored into a machine on a host-owned clock, handed to [body], and cleaned up. */
    private fun <T> withRestored(text: String, body: (StaticInvokeStateMachine) -> T): T {
        val sm = StaticInvokeStateMachine()
        try {
            sm.clock = ManualClock(0)
            sm.restore(SavedState.fromJson(text), savedAtMs)
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    /** Send [event] to the parent, which forwards it to its child, and let the child run. */
    private fun send(sm: StaticInvokeStateMachine, event: StaticInvokeEvent) {
        sm.send(event)
        repeat(3) { sm.tick() }
    }

    private fun working(sm: StaticInvokeStateMachine): Boolean =
        StaticInvokeState.Working in sm.snapshot.value.configuration

    private fun children(saved: SavedState) =
        (saved.invokes as? SavedInvokes.Resumed ?: error("a format-2 state: ${saved.invokes}")).children

    @Test
    fun aMachineIsSavedWithTheWholeStateOfTheChildItIsRunning() {
        val sm = started()
        try {
            val saved = sm.save(savedAtMs)
            val child = children(saved).single()
            assertEquals("worker", child.id)
            assertEquals(1L, child.start, "its first start")
            assertEquals(mapOf("worker" to 1L), saved.invokeStarts)
            // Its id is this session's and the id it was started under: the tree's alone.
            assertEquals("working.1.worker", child.invokeId)
            assertEquals("fixture-root.working.1.worker", child.session.sessionId)
            assertEquals("first", child.session.current)
            // The text every backend writes for the same machine, byte for byte.
            assertEquals(shared, saved.toJson())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aChildThatHadTakenAnEventIsResumedWithItTaken() {
        val before = started()
        val saved = try {
            send(before, StaticInvokeEvent.A)
            before.save(savedAtMs)
        } finally {
            before.cleanup()
        }

        // Resumed at `second`: `b` alone ends it, and its `done.invoke` arrives once.
        withRestored(saved.toJson()) { after ->
            assertTrue(working(after))
            send(after, StaticInvokeEvent.B)
            assertEquals(1u, after.completed, "the child resumed where it stood, and ended on `b`")
            send(after, StaticInvokeEvent.B)
            assertEquals(1u, after.completed, "it ended once")
        }
    }

    @Test
    fun aRestoredTreeSavesTheTextItWasRestoredFrom() {
        withRestored(shared) { sm ->
            assertEquals(
                shared,
                sm.save(savedAtMs).toJson(),
                "the child kept its id, its start and where it stood",
            )
        }
    }

    @Test
    fun theCountOfStartsIsCarriedThroughARestore() {
        // The resumed child was not started again, and when it ends the count the
        // restore carried on is still its one start.
        val before = started()
        val saved = try {
            send(before, StaticInvokeEvent.A)
            before.save(savedAtMs)
        } finally {
            before.cleanup()
        }
        withRestored(saved.toJson()) { after ->
            send(after, StaticInvokeEvent.B)
            assertEquals(1u, after.completed)
            val again = after.save(savedAtMs)
            assertEquals(mapOf("worker" to 1L), again.invokeStarts)
            assertTrue(again.invokeIds.isEmpty(), "the child ended: ${again.invokes}")
        }
    }

    @Test
    fun aChildThatHasEndedIsNotResumedOrStartedAgain() {
        // `done.invoke.worker` leaves the parent in `working`, so a machine saved
        // after its child ended stands in the state that invokes it with nothing
        // running. The invocation is complete: it is not listed, and a restore does
        // not run it a second time.
        val before = started()
        val saved = try {
            send(before, StaticInvokeEvent.A)
            send(before, StaticInvokeEvent.B)
            assertEquals(1u, before.completed)
            assertTrue(working(before))
            before.save(savedAtMs)
        } finally {
            before.cleanup()
        }
        assertTrue(saved.invokeIds.isEmpty(), "the child ended: ${saved.invokes}")

        withRestored(saved.toJson()) { after ->
            assertEquals(1u, after.completed, "the run is kept")
            send(after, StaticInvokeEvent.A)
            send(after, StaticInvokeEvent.B)
            assertEquals(1u, after.completed, "a child started again would have ended again")
        }
    }

    @Test
    fun aMachineThatLeftTheInvokingStateSavesNoInvocation() {
        val sm = started()
        val saved = try {
            send(sm, StaticInvokeEvent.Abort)
            assertTrue(!working(sm))
            sm.save(savedAtMs)
        } finally {
            sm.cleanup()
        }
        assertTrue(saved.invokeIds.isEmpty(), "cancelled with its state: ${saved.invokes}")

        // And nothing is started when it comes back: no child is there to take `a`
        // and `b`, and `idle` invokes nothing.
        withRestored(saved.toJson()) { again ->
            send(again, StaticInvokeEvent.A)
            send(again, StaticInvokeEvent.B)
            assertTrue(!again.isInFinalState)
        }
    }

    @Test
    fun anEventSavedInTheChildsQueueIsTakenByTheResumedChild() {
        // `a` reached the child's own queue and the child had not taken it when the
        // tree was saved: it is in the child's `external`, and the resumed child
        // takes it, then ends on `b`.
        val inChild = "\"invokestarts\":{},\"hostinvokes\":[],\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[]}}]"
        val text = shared.replace(
            inChild,
            inChild.replace(
                "\"external\":[]}}]",
                "\"external\":[{\"name\":\"a\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\"," +
                    "\"origin\":\"\",\"origintype\":\"\",\"invokeid\":\"\",\"hostinvoketoken\":null}]}}]",
            ),
        )
        assertNotEquals(shared, text, "the child's queue was changed")
        withRestored(text) { sm ->
            repeat(3) { sm.tick() }
            send(sm, StaticInvokeEvent.B)
            assertEquals(1u, sm.completed, "the queued `a` reached the resumed child")
        }
    }

    @Test
    fun aTreeWhoseChildIsNotOneTheDocumentCouldRunIsRefusedWhole() {
        val cases = listOf(
            listOf(
                "a child the document does not invoke",
                "\"id\":\"worker\"", "\"id\":\"stranger\"",
                "which the document does not invoke",
            ),
            listOf(
                "a start that is not the count its invoke holds",
                "\"start\":\"1\"", "\"start\":\"2\"",
                "the session counts 1 starts of it",
            ),
            listOf(
                "a child of a document its invoke does not start",
                "\"document\":\"static_invoke__sce_synth_invoke__worker\"", "\"document\":\"static_hybrid_holder\"",
                "does not start",
            ),
            listOf(
                "a child whose id is not its parent's and the id it was started under",
                "\"sessionid\":\"fixture-root.working.1.worker\"", "\"sessionid\":\"elsewhere\"",
                "a child of 'fixture-root' started under 'working.1.worker'",
            ),
            listOf(
                "a child of another shape",
                "688ad00d954ad9f35bdaa5eae06768a788173e0f8a8236e670efa68038781a6d",
                "0000000000000000000000000000000000000000000000000000000000000000",
                "the session at invokes[0].session: the saved state is of a document of shape",
            ),
            listOf(
                "a child that stands in no state of its document",
                "\"configuration\":[\"first\"],\"current\":\"first\"",
                "\"configuration\":[\"nowhere\"],\"current\":\"nowhere\"",
                "the session at invokes[0].session: the document has no state 'nowhere'",
            ),
        )
        for ((what, from, to, expected) in cases) {
            val text = shared.replaceFirst(from, to)
            assertNotEquals(shared, text, "$what: the change did not apply")
            val sm = StaticInvokeStateMachine()
            try {
                val refusal = assertFailsWith<StateRefusal>(what) { sm.restore(SavedState.fromJson(text), savedAtMs) }
                assertTrue(refusal.message!!.contains(expected), "$what: ${refusal.message}")
                assertTrue(sm.snapshot.value.configuration.isEmpty(), "$what: a refused restore builds nothing")
            } finally {
                sm.cleanup()
            }
        }
    }

    // ── format 1: a child named by its id is started again ─────────────────

    @Test
    fun aFormat1StateStartsItsChildAgainFromTheBeginning() {
        // The only case in which a child restarts: the text holds nothing of it.
        withRestored(format1) { sm ->
            assertTrue(working(sm))

            // The child is at `first`: `b` is not what it waits for.
            send(sm, StaticInvokeEvent.B)
            assertTrue(working(sm), "`b` alone does not end a child that starts over")
            assertEquals(0u, sm.completed)

            send(sm, StaticInvokeEvent.A)
            send(sm, StaticInvokeEvent.B)
            assertEquals(1u, sm.completed, "`a` then `b` ends it, and `done.invoke.worker` arrived once")
        }
    }

    @Test
    fun aFormat1StateNamingAnInvocationTheDocumentCannotHaveRunIsRefused() {
        val cases = listOf(
            Triple(
                "an id the document does not invoke",
                format1.replace("\"invokes\":[\"worker\"]", "\"invokes\":[\"stranger\"]"),
                "which the document does not invoke",
            ),
            Triple(
                "an id named twice",
                format1.replace("\"invokes\":[\"worker\"]", "\"invokes\":[\"worker\",\"worker\"]"),
                "which an earlier entry already names",
            ),
            Triple(
                "an invocation whose state the configuration is not in",
                format1.replace("\"configuration\":[\"working\"]", "\"configuration\":[\"idle\"]")
                    .replace("\"current\":\"working\"", "\"current\":\"idle\""),
                "whose state the saved configuration does not stand in",
            ),
        )
        for ((what, text, expected) in cases) {
            val sm = StaticInvokeStateMachine()
            try {
                val refusal = assertFailsWith<StateRefusal>(what) { sm.restore(SavedState.fromJson(text), savedAtMs) }
                assertTrue(refusal.message!!.contains(expected), "$what: ${refusal.message}")
            } finally {
                sm.cleanup()
            }
        }
    }

    @Test
    fun theInvokesFieldOfAFormat1StateIsRequiredAndAListOfIds() {
        val cases = listOf(
            Triple("absent", format1.replace("\"invokes\":[\"worker\"],", ""), "has no 'invokes'"),
            Triple("not an array", format1.replace("\"invokes\":[\"worker\"]", "\"invokes\":\"worker\""), "'invokes' is not an array"),
            Triple("an element that is not a text", format1.replace("\"invokes\":[\"worker\"]", "\"invokes\":[5]"), "'invokes' is not a text"),
        )
        for ((what, text, expected) in cases) {
            val refusal = assertFailsWith<StateRefusal>(what) { SavedState.fromJson(text) }
            assertTrue(refusal.message!!.contains(expected), "$what: ${refusal.message}")
        }
    }
}
