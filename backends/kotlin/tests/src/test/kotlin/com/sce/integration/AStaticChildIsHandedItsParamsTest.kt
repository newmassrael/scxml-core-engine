// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions" — Kotlin half of the Rust suite's
// `a_static_child_is_handed_its_params.rs`.
//
// An `<invoke type="scxml">` hands its child the values its `<param>`s and
// `namelist` name (W3C SCXML 6.4.1), each to the child's variable of the same
// name, of that variable's own type.
//
// `static_invoke_params.scxml` invokes `worker`, which ends the moment it holds
// `start = 7` (from a `<param>` reading `base`) and `enabled = true` (from the
// `namelist`); handed less, it would wait and the parent would stay in
// `working`. `base` is 4 when `working` is entered and the entry action adds 3,
// so 7 arrives only if the value is read when the invoke executes, at the end of
// the macrostep. `control`, the same child handed nothing, keeps its declared
// defaults and never ends.

package com.sce.integration

import com.sce.integration.static_invoke_params.StaticInvokeParamsEvent
import com.sce.integration.static_invoke_params.StaticInvokeParamsState
import com.sce.integration.static_invoke_params.StaticInvokeParamsStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AStaticChildIsHandedItsParamsTest {

    /** The wall-clock moment the machine is saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private fun started(): StaticInvokeParamsStateMachine {
        val sm = StaticInvokeParamsStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        return sm
    }

    /** Let the child run and report to its parent. */
    private fun settle(sm: StaticInvokeParamsStateMachine) {
        repeat(5) { sm.tick() }
    }

    private fun inState(sm: StaticInvokeParamsStateMachine, state: StaticInvokeParamsState): Boolean =
        state in sm.snapshot.value.configuration

    @Test
    fun aChildIsHandedTheValuesItsInvokeNames() {
        val sm = started()
        try {
            settle(sm)
            // `worker` ended, so it held both values: the parent left `working`
            // and counted it.
            assertTrue(
                inState(sm, StaticInvokeParamsState.Plain),
                "the child was handed `start` and `enabled`, so it ended",
            )
            assertEquals(1u, sm.completed)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredChildIsHandedTheValuesTheRestoredMachineHolds() {
        val sm = started()
        try {
            settle(sm)
            // `bump` makes `base` 8 while `watcher` runs with the 7 it was handed
            // once, when it started, so it does not end.
            sm.send(StaticInvokeParamsEvent.Bump)
            settle(sm)
            assertEquals(1u, sm.completed)

            // A restored child is started again from its beginning, and a start
            // evaluates its arguments: the new `watcher` is handed 8, and ends.
            val saved = SavedState.fromJson(sm.save(savedAtMs).toJson())
            val restored = StaticInvokeParamsStateMachine()
            try {
                restored.clock = ManualClock(0)
                restored.restore(saved, savedAtMs)
                settle(restored)
                assertEquals(11u, restored.completed)
            } finally {
                restored.cleanup()
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aChildHandedNothingKeepsTheValuesItsDataGaveIt() {
        val sm = started()
        try {
            settle(sm)
            settle(sm)
            // `control` is the same child handed nothing: it still waits for 7
            // and true, so `done.invoke.control` never counted.
            assertTrue(inState(sm, StaticInvokeParamsState.Plain))
            assertEquals(1u, sm.completed)
        } finally {
            sm.cleanup()
        }
    }
}
