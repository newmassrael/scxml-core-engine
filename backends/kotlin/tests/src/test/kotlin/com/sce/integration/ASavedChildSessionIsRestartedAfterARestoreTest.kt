// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" — Kotlin half of the Rust
// suite's `a_saved_child_session_is_restarted_after_a_restore.rs`.
//
// A child session an `<invoke type="scxml">` started is part of what a machine
// is doing, and the process that ran it is gone. A saved state lists the
// invocations whose child is running (`invokes`) and a restore starts each
// again from the beginning of its child, under the same id (W3C SCXML 6.4).
//
// `static_invoke.scxml` invokes `worker`, a child that takes `a` and then `b`
// and ends; the parent forwards what it is sent (`autoforward`), so the host
// drives the child through the parent and every case is read from the parent:
// `done.invoke.worker` counts the run in `completed` and leaves the parent in
// `working`. A restored machine's child starts over, so `b` alone does not end
// it where `a` then `b` does, and a child that had ended is not started again.
//
// The shared instance `saved/static_invoke_working.json` is the text the Rust
// suite writes and restores too.

package com.sce.integration

import com.sce.integration.static_invoke.StaticInvokeEvent
import com.sce.integration.static_invoke.StaticInvokeState
import com.sce.integration.static_invoke.StaticInvokeStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class ASavedChildSessionIsRestartedAfterARestoreTest {

    /** The wall-clock moment the shared instance was saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private val shared: String = File(repoRoot(), "sce-build/tests/fixtures/static_datamodel/saved/static_invoke_working.json")
        .readText().trim()

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    private fun started(): StaticInvokeStateMachine {
        val sm = StaticInvokeStateMachine()
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

    @Test
    fun aMachineIsSavedWithTheChildItIsRunning() {
        val sm = started()
        try {
            val saved = sm.save(savedAtMs)
            assertEquals(listOf("worker"), saved.invokes)
            // The text the Rust suite writes for the same machine, byte for byte.
            assertEquals(shared, saved.toJson())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredMachineStartsItsChildAgainFromTheBeginning() {
        withRestored(shared) { sm ->
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
    fun aChildThatHadTakenAnEventIsStartedOverNotResumed() {
        // Its progress is not saved: the child had left `first` for `second`,
        // and a restored one is at `first` again.
        val before = started()
        val saved = try {
            send(before, StaticInvokeEvent.A)
            before.save(savedAtMs)
        } finally {
            before.cleanup()
        }
        assertEquals(listOf("worker"), saved.invokes, "it was running")

        withRestored(saved.toJson()) { after ->
            send(after, StaticInvokeEvent.B)
            assertTrue(working(after), "a child resumed at `second` would have ended on `b`")
            send(after, StaticInvokeEvent.A)
            send(after, StaticInvokeEvent.B)
            assertEquals(1u, after.completed)
        }
    }

    @Test
    fun aChildThatHasEndedIsNotStartedAgain() {
        // `done.invoke.worker` leaves the parent in `working`, so a machine
        // saved after its child ended stands in the state that invokes it with
        // nothing running. The invocation is complete: it is not listed, and a
        // restore does not run it a second time.
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
        assertTrue(saved.invokes.isEmpty(), "the child ended: ${saved.invokes}")

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
        assertTrue(saved.invokes.isEmpty(), "cancelled with its state: ${saved.invokes}")

        // And nothing is started when it comes back: no child is there to take
        // `a` and `b`, and `idle` invokes nothing.
        withRestored(saved.toJson()) { again ->
            send(again, StaticInvokeEvent.A)
            send(again, StaticInvokeEvent.B)
            assertTrue(!again.isInFinalState)
        }
    }

    @Test
    fun anInvocationTheSavedStateDoesNotListIsNotStarted() {
        // `working` with no running child: a state that says so is a state
        // whose child ended, and starting it again would run the invocation
        // twice.
        val text = shared.replace("\"invokes\":[\"worker\"]", "\"invokes\":[]")
        withRestored(text) { sm ->
            assertTrue(working(sm))
            send(sm, StaticInvokeEvent.A)
            send(sm, StaticInvokeEvent.B)
            assertTrue(working(sm), "nobody is there to take them")
            assertEquals(0u, sm.completed)
        }
    }

    @Test
    fun anEventSavedInTheQueueReachesTheChildTheRestoreStarted() {
        // The saved external queue and the restarted child are restored
        // together: `a` was raised to the machine before it was saved, and it
        // is the restarted child that takes it when the parent forwards it.
        val text = shared.replace(
            "\"external\":[]",
            "\"external\":[{\"name\":\"a\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\"," +
                "\"origin\":\"\",\"origintype\":\"\",\"invokeid\":\"\"}]",
        )
        withRestored(text) { sm ->
            repeat(3) { sm.tick() }
            send(sm, StaticInvokeEvent.B)
            assertEquals(1u, sm.completed, "the queued `a` reached the restarted child")
        }
    }

    @Test
    fun aSavedStateNamingAnInvocationTheDocumentCannotHaveRunIsRefused() {
        val cases = listOf(
            Triple(
                "an id the document does not invoke",
                shared.replace("\"invokes\":[\"worker\"]", "\"invokes\":[\"stranger\"]"),
                "which the document does not invoke",
            ),
            Triple(
                "an id named twice",
                shared.replace("\"invokes\":[\"worker\"]", "\"invokes\":[\"worker\",\"worker\"]"),
                "which an earlier entry already names",
            ),
            Triple(
                "an invocation whose state the configuration is not in",
                shared.replace("\"configuration\":[\"working\"]", "\"configuration\":[\"idle\"]")
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
    fun theInvokesFieldIsRequiredAndMustBeAListOfIds() {
        val cases = listOf(
            Triple("absent", shared.replace("\"invokes\":[\"worker\"],", ""), "has no 'invokes'"),
            Triple("not an array", shared.replace("\"invokes\":[\"worker\"]", "\"invokes\":\"worker\""), "'invokes' is not an array"),
            Triple("an element that is not a text", shared.replace("\"invokes\":[\"worker\"]", "\"invokes\":[5]"), "'invokes' is not a text"),
        )
        for ((what, text, expected) in cases) {
            val refusal = assertFailsWith<StateRefusal>(what) { SavedState.fromJson(text) }
            assertTrue(refusal.message!!.contains(expected), "$what: ${refusal.message}")
        }
    }
}
