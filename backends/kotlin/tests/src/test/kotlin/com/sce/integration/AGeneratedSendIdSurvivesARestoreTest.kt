// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" — Kotlin half of the Rust
// suite's `a_generated_send_id_survives_a_restore.rs`.
//
// The id a machine generates for a `<send idlocation>` is held by the document in
// a variable, and by the send that waits under it, so a saved state carries both
// and the count the next id comes from. A machine restored from it cancels the
// send by the id the document holds, and numbers the ids it generates next after
// the count: an id the saved state already holds is never generated again, which
// would let one `<cancel>` remove two sends.
//
// `static_send_idlocation.scxml`: `arm` sends two delayed events, `first.due` and
// `second.due`, each under an id written to `first` and `second`, and
// `cancel.first` and `cancel.second` remove the send whose id the variable holds.
// Everything runs on a host-owned clock, so no case sleeps. The shared fixture
// `saved/static_send_idlocation_armed.json` is the text the Rust suite writes and
// restores too.

package com.sce.integration

import com.sce.integration.static_send_idlocation.StaticSendIdlocationEvent
import com.sce.integration.static_send_idlocation.StaticSendIdlocationStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AGeneratedSendIdSurvivesARestoreTest {

    /** The wall-clock moment the shared fixture was saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private val shared: String = File(
        repoRoot(),
        "sce-build/tests/fixtures/static_datamodel/saved/static_send_idlocation_armed.json",
    ).readText().trim()

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    private fun machine(): StaticSendIdlocationStateMachine {
        val sm = StaticSendIdlocationStateMachine()
        sm.clock = ManualClock(0)
        return sm
    }

    /** A machine that has run `arm` once, on a clock the test owns. */
    private fun <T> withArmed(body: (StaticSendIdlocationStateMachine) -> T): T {
        val sm = machine()
        try {
            sm.initialize()
            sm.send(StaticSendIdlocationEvent.Arm)
            sm.tick()
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    private fun <T> withRestored(body: (StaticSendIdlocationStateMachine) -> T): T {
        val sm = machine()
        try {
            sm.restore(SavedState.fromJson(shared), savedAtMs)
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aMachineThatArmedTwoSendsSavesTheIdsItGenerated() {
        withArmed { sm ->
            val saved = sm.save(savedAtMs)
            assertEquals(2L, saved.autoSendSeq, "two ids were generated")
            assertEquals(2, saved.pending.size, "two sends wait")
            val text = saved.toJson()
            for (id in listOf("_auto_send_1", "_auto_send_2")) {
                assertTrue(text.contains("\"sendid\":\"$id\""), "a waiting send carries $id: $text")
            }
            // The text the Rust suite writes for the same run, byte for byte: the
            // variables hold the ids the waiting sends are known by.
            assertEquals(shared, text)
        }
    }

    @Test
    fun aRestoredSendIsCancelledByTheIdTheDocumentHolds() {
        withRestored { sm ->
            sm.send(StaticSendIdlocationEvent.Cancel.First)
            sm.tick()
            sm.advanceTimeMs(250)
            assertEquals(0u, sm.firstFired, "its send was removed")
            assertEquals(1u, sm.secondFired, "the other came due")
        }
    }

    @Test
    fun aRestoredMachineNumbersItsNextIdsAfterTheSavedCount() {
        withRestored { sm ->
            // The ids the saved state holds are 1 and 2: `arm` again writes 3 and
            // 4 to the variables, and a `<cancel>` by `first` removes the send of
            // id 3 and not the one of id 1, which is still waiting.
            sm.send(StaticSendIdlocationEvent.Arm)
            sm.tick()
            val saved = sm.save(savedAtMs)
            assertEquals(4L, saved.autoSendSeq)
            assertTrue(
                saved.toJson().contains("\"first\":\"_auto_send_3\",\"second\":\"_auto_send_4\""),
                saved.toJson(),
            )

            sm.send(StaticSendIdlocationEvent.Cancel.First)
            sm.tick()
            sm.advanceTimeMs(250)
            assertEquals(1u, sm.firstFired, "the send of id 1 came due, the one of id 3 was removed")
            assertEquals(2u, sm.secondFired, "ids 2 and 4 came due")
        }
    }
}
