// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring", for a hybrid `<invoke>`
// (§2.13) — Kotlin half of the Rust suite's
// `a_static_hybrid_invoke_is_started_again_by_a_restore.rs`.
//
// A format-2 saved state records the candidate that was running, with its whole
// state, and a restore resumes it whatever `srcexpr` reads now: the arguments
// were evaluated when the invocation started (§scxml-6.4), and a resumed child did
// not start (docs/adr/0045). A format-1 state named the invocation alone, and its
// restore starts it again from its beginning, reading `srcexpr` and the arguments
// from the restored variables as entering the state would: the one case in which
// a child restarts.
//
// `static_invoke_hybrid_saved.scxml` invokes `watch`, whose `srcexpr` is `pick`
// among `static_hybrid_watcher` (ends on the 8 it is handed) and
// `static_hybrid_holder` (ends on the 7). `base` is 7 when the child starts, and
// `bump` makes it 8 without telling the child; `swap` makes `pick` name the other
// candidate.

package com.sce.integration

import com.sce.integration.static_invoke_hybrid_saved.StaticInvokeHybridSavedEvent
import com.sce.integration.static_invoke_hybrid_saved.StaticInvokeHybridSavedState
import com.sce.integration.static_invoke_hybrid_saved.StaticInvokeHybridSavedStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedInvokes
import com.sce.runtime.SavedState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AStaticHybridInvokeIsStartedAgainByARestoreTest {

    /** The wall-clock moment the machine is saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private fun started(): StaticInvokeHybridSavedStateMachine {
        val sm = StaticInvokeHybridSavedStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        return sm
    }

    /** Let the child run and report to its parent. */
    private fun settle(sm: StaticInvokeHybridSavedStateMachine) {
        repeat(5) { sm.tick() }
    }

    /** [saved] restored into a machine on a host-owned clock, as a host carries it through its own storage. */
    private fun restoredFrom(saved: SavedState): StaticInvokeHybridSavedStateMachine {
        val restored = StaticInvokeHybridSavedStateMachine()
        restored.clock = ManualClock(0)
        restored.restore(SavedState.fromJson(saved.toJson()), savedAtMs)
        return restored
    }

    /** The machine saved and restored. */
    private fun restored(sm: StaticInvokeHybridSavedStateMachine): StaticInvokeHybridSavedStateMachine =
        restoredFrom(sm.save(savedAtMs))

    /**
     * The same moment as a format-1 state, which names the running invocation by
     * its id alone. The document holds no enum, so its format-1 shape is the
     * format-2 one the save wrote.
     */
    private fun asFormat1(saved: SavedState): SavedState = SavedState(
        shape = saved.shape,
        configuration = saved.configuration,
        current = saved.current,
        variables = saved.variables,
        history = saved.history,
        pending = saved.pending,
        invokes = SavedInvokes.Restarted(saved.invokeIds),
        hostInvokes = saved.hostInvokes,
        hostInvokeToken = saved.hostInvokeToken,
        autoSendSeq = saved.autoSendSeq,
        external = saved.external,
        revisions = saved.revisions,
    )

    /** The document each running child of [saved] runs. */
    private fun documents(saved: SavedState): List<String> =
        (saved.invokes as? SavedInvokes.Resumed ?: error("a format-2 state names each child's document"))
            .children.map { it.document }

    private fun working(sm: StaticInvokeHybridSavedStateMachine): Boolean =
        StaticInvokeHybridSavedState.Working in sm.snapshot.value.configuration

    @Test
    fun aRestoredChildKeepsTheValuesItWasHanded() {
        val sm = started()
        try {
            settle(sm)
            // The watcher was handed 7 and waits for 8; `bump` makes `base` 8 and
            // it is not told.
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            assertEquals(0u, sm.completed)

            // Resumed, the watcher holds the 7 it was handed: nothing is evaluated
            // again, so it still waits.
            val restored = restored(sm)
            try {
                settle(restored)
                assertEquals(0u, restored.completed)
                assertTrue(working(restored))
            } finally {
                restored.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoreResumesTheCandidateThatWasRunningWhateverSrcexprReadsNow() {
        val sm = started()
        try {
            settle(sm)
            // The watcher is the child that is running when the machine is saved,
            // though `pick` now names the holder.
            sm.send(StaticInvokeHybridSavedEvent.Swap)
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            assertEquals(0u, sm.completed)
            val saved = sm.save(savedAtMs)
            assertEquals(listOf("static_hybrid_watcher"), documents(saved))

            val restored = restoredFrom(saved)
            try {
                settle(restored)
                assertEquals(
                    listOf("static_hybrid_watcher"),
                    documents(restored.save(savedAtMs)),
                    "the watcher was resumed, not the holder `pick` names now",
                )
                assertTrue(working(restored), "the watcher waits for an 8 it was not handed")
            } finally {
                restored.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aChildNothingHasChangedForWaitsAsItDid() {
        val sm = started()
        try {
            settle(sm)
            val restored = restored(sm)
            try {
                settle(restored)
                // `base` is still 7: the watcher waits for an 8, as it did.
                assertEquals(0u, restored.completed)
                assertTrue(working(restored))
            } finally {
                restored.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aChildThatEndedIsNotStartedAgain() {
        val sm = started()
        try {
            settle(sm)
            // In format 1 the watcher started again is handed the 8 `base` holds,
            // and ends.
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            val once = restoredFrom(asFormat1(sm.save(savedAtMs)))
            try {
                settle(once)
                assertEquals(1u, once.completed)

                // Saved once its `done.invoke` was taken, the machine is past
                // `working`, and a restore names no invocation to resume or start.
                assertTrue(!working(once))
                val again = restored(once)
                try {
                    settle(again)
                    assertEquals(1u, again.completed, "the child ran twice")
                    assertTrue(!working(again))
                } finally {
                    again.cleanup()
                }
            } finally {
                once.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    // ── format 1: the invocation named by its id is started again ───────────

    @Test
    fun aFormat1RestoreHandsTheChildTheValuesTheRestoredMachineHolds() {
        val sm = started()
        try {
            settle(sm)
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            assertEquals(0u, sm.completed)

            // Started again from its beginning, the watcher is handed 8, and ends.
            val restarted = restoredFrom(asFormat1(sm.save(savedAtMs)))
            try {
                settle(restarted)
                assertEquals(1u, restarted.completed)
            } finally {
                restarted.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aFormat1RestoreReadsTheSrcexprAgain() {
        val sm = started()
        try {
            settle(sm)
            sm.send(StaticInvokeHybridSavedEvent.Swap)
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            assertEquals(0u, sm.completed)

            // The restored fields name the holder, which is handed 8 and waits for
            // 7. The watcher, which would have ended on that 8, is not what the
            // restore starts.
            val restarted = restoredFrom(asFormat1(sm.save(savedAtMs)))
            try {
                settle(restarted)
                assertEquals(0u, restarted.completed, "the watcher was started")
                assertTrue(working(restarted), "the holder waits for a 7 it is not handed")
                // The invocation is still running, and a save names the holder.
                assertEquals(listOf("static_hybrid_holder"), documents(restarted.save(savedAtMs)))
            } finally {
                restarted.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }
}
