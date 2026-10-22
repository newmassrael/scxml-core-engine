// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" — Kotlin half of the Rust
// suite's `a_saved_machine_carries_on_the_send_ids_it_generated.rs`.
//
// A machine hands the document an id for a `<send idlocation>`, the document
// keeps it, and may name it in a `<cancel>` long after. A restore must not start
// counting again: the document that was saved may still hold `_auto_send_1`, and
// a send waiting in the saved state carries it. A saved state says how many ids
// the machine has generated (`sendseq`), and a restore carries on from it.
//
// The shared instance `saved/statechart_static_host_invoke_running.json` is the
// text the Rust suite restores too; it has generated no id, so each case that
// needs a count writes one into it. The id a restored machine generates next is
// read where the machine generates one, in the suites of the documents that ask
// for it; what is held here is that the count survives the round trip.

package com.sce.integration

import com.sce.integration.statechart_static_host_invoke.StatechartStaticHostInvokeStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class ASavedMachineCarriesOnTheSendIdsItGeneratedTest {

    /** The wall-clock moment the shared instance was saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private val shared: String = File(
        repoRoot(),
        "sce-build/tests/fixtures/host_processor/saved/statechart_static_host_invoke_running.json",
    ).readText().trim()

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    /** The shared instance with `sendseq` written as [count]. */
    private fun sharedWithCount(count: String) = shared.replace("\"sendseq\":\"0\"", "\"sendseq\":\"$count\"")

    private fun <T> withRestored(text: String, body: (StatechartStaticHostInvokeStateMachine) -> T): T {
        val sm = StatechartStaticHostInvokeStateMachine()
        sm.clock = ManualClock(0)
        try {
            sm.restore(SavedState.fromJson(text), savedAtMs)
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aMachineThatGeneratedNoIdIsSavedWithACountOfZero() {
        val sm = StatechartStaticHostInvokeStateMachine()
        sm.clock = ManualClock(0)
        try {
            sm.initialize()
            val saved = sm.save(savedAtMs)
            assertEquals(0L, saved.autoSendSeq)
            assertTrue(saved.toJson().contains("\"sendseq\":\"0\""), saved.toJson())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredMachineIsSavedAgainWithTheCountItWasRestoredWith() {
        val text = sharedWithCount("5")
        assertEquals(5L, SavedState.fromJson(text).autoSendSeq)
        withRestored(text) { sm ->
            assertEquals(text, sm.save(savedAtMs).toJson(), "repeating the round trip never loses the count")
        }
    }

    @Test
    fun theSendCountIsRequiredAndMustBeACount() {
        fun swap(from: String, to: String) = shared.replace(from, to)
        val cases = listOf(
            Triple("no sendseq", swap("\"sendseq\":\"0\",", ""), "has no 'sendseq'"),
            Triple("a count that is a number", swap("\"sendseq\":\"0\"", "\"sendseq\":0"), "'sendseq' is not a text"),
            Triple(
                "a count that is not digits",
                swap("\"sendseq\":\"0\"", "\"sendseq\":\"-1\""),
                "'sendseq' (-1) is not a whole number",
            ),
            Triple(
                "a count past a signed 64-bit count",
                swap("\"sendseq\":\"0\"", "\"sendseq\":\"9223372036854775808\""),
                "'sendseq' (9223372036854775808) is not a whole number",
            ),
        )
        for ((what, text, expected) in cases) {
            val refusal = assertFailsWith<StateRefusal>(what) { SavedState.fromJson(text) }
            assertTrue(refusal.message!!.contains(expected), "$what: ${refusal.message}")
        }
    }
}
