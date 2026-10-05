// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring", for a hybrid `<invoke>`
// (§2.13) — Kotlin half of the Rust suite's
// `a_static_hybrid_invoke_is_started_again_by_a_restore.rs`.
//
// A saved state names the invocation whose child is running, and a restore
// starts it again from its beginning (W3C SCXML 6.4), as it does a static child
// — and what the start reads is read again, `srcexpr` among it, so the restored
// fields name the candidate as they would had the state been entered.
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

    /** The machine saved and restored, as a host carries it through its own storage. */
    private fun restored(sm: StaticInvokeHybridSavedStateMachine): StaticInvokeHybridSavedStateMachine {
        val saved = SavedState.fromJson(sm.save(savedAtMs).toJson())
        val restored = StaticInvokeHybridSavedStateMachine()
        restored.clock = ManualClock(0)
        restored.restore(saved, savedAtMs)
        return restored
    }

    private fun working(sm: StaticInvokeHybridSavedStateMachine): Boolean =
        StaticInvokeHybridSavedState.Working in sm.snapshot.value.configuration

    @Test
    fun aRestoredChildIsHandedTheValuesTheRestoredMachineHolds() {
        val sm = started()
        try {
            settle(sm)
            // The watcher was handed 7 and waits for 8; `bump` makes `base` 8 and
            // it is not told.
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            assertEquals(0u, sm.completed)

            // A restored child is started again from its beginning, and a start
            // evaluates its arguments: the new watcher is handed 8, and ends.
            val restored = restored(sm)
            try {
                settle(restored)
                assertEquals(1u, restored.completed)
            } finally {
                restored.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoreReadsTheSrcexprAgain() {
        val sm = started()
        try {
            settle(sm)
            // The watcher is the child that is running when the machine is saved.
            sm.send(StaticInvokeHybridSavedEvent.Swap)
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            assertEquals(0u, sm.completed)

            // The restored fields name the holder, which is handed 8 and waits for
            // 7. The watcher, which would have ended on that 8, is not what a
            // restore starts.
            val restored = restored(sm)
            try {
                settle(restored)
                assertEquals(0u, restored.completed, "the watcher was started")
                assertTrue(working(restored), "the holder waits for a 7 it is not handed")
            } finally {
                restored.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredChildOfTheOtherCandidateIsSavedAgain() {
        val sm = started()
        try {
            settle(sm)
            sm.send(StaticInvokeHybridSavedEvent.Swap)
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            // The watcher is running, started when the state was entered.
            assertEquals(listOf("watch"), sm.save(savedAtMs).invokes)

            // The restore started the holder, which waits for a 7 it is not handed:
            // the invocation is still running, and a second save names it.
            val restored = restored(sm)
            try {
                settle(restored)
                assertTrue(working(restored))
                assertEquals(listOf("watch"), restored.save(savedAtMs).invokes)
            } finally {
                restored.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aChildNothingHasChangedForIsHandedWhatItWasHanded() {
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
            sm.send(StaticInvokeHybridSavedEvent.Bump)
            settle(sm)
            val once = restored(sm)
            try {
                settle(once)
                assertEquals(1u, once.completed)

                // Saved once its `done.invoke` was taken, the machine is past
                // `working`, and a restore names no invocation to start.
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
}
