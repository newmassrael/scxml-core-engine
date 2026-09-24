// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) — Kotlin
// compile+run gate.
//
// The committed SM (com/sce/integration/static_counter/static_counterSm.kt) is
// generated from sce-build/tests/fixtures/static_datamodel/static_counter.scxml
// (regen: scripts/regen_static_datamodel_kotlin.sh). Its variables are Kotlin
// fields and every expression — the guard reading `count` and `In()`, the
// `<assign>`s, the `<if>` / `<elseif>` pair — was lowered to native Kotlin, so
// the machine is constructed with NO script engine and its datamodel is read
// straight off the fields.

package com.sce.integration

import com.sce.integration.static_counter.StaticCounterEvent
import com.sce.integration.static_counter.StaticCounterState
import com.sce.integration.static_counter.StaticCounterStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.2 / 5.4 under SCE's statically typed data model.
@DisplayName("StaticDatamodel — sce-static variables as native fields, no script engine (Kotlin AOT)")
class StaticDatamodelTest {

    private fun ticks(sm: StaticCounterStateMachine, n: Int) {
        repeat(n) {
            sm.send(StaticCounterEvent.Tick)
            sm.tick()
        }
    }

    @Test
    fun theVariablesStartAtTheirDeclaredValues() {
        // No script-engine argument: the model is engine-free by definition.
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            assertEquals(StaticCounterState.Counting, sm.currentState.value)
            assertEquals(0u, sm.count, "count is declared expr=\"0\"")
            assertEquals(false, sm.ready, "ready is declared expr=\"false\"")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anAssignmentAndAConditionalReadTheFields() {
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            ticks(sm, 5)
            assertEquals(5u, sm.count, "each tick assigns count + 1")
            assertEquals(true, sm.ready, "the <if cond=\"count === 5\"> branch ran")

            ticks(sm, 3)
            assertEquals(8u, sm.count)
            assertEquals(false, sm.ready, "the <elseif cond=\"count > 7\"> branch ran")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theGuardStopsTheCounterAtItsBound() {
        // `count < 10 && In('counting')`: the eleventh tick finds the guard
        // false, so no transition is taken and count stays where it was.
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            ticks(sm, 11)
            assertEquals(10u, sm.count, "the guard holds count at 10")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aGuardReadingABoolFieldTakesItsTransition() {
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            sm.send(StaticCounterEvent.Go)
            sm.tick()
            assertEquals(
                StaticCounterState.Counting,
                sm.currentState.value,
                "ready is false, so `go` is not taken"
            )

            ticks(sm, 5)
            sm.send(StaticCounterEvent.Go)
            sm.tick()
            assertEquals(StaticCounterState.Done, sm.currentState.value, "ready is true after five ticks")
        } finally {
            sm.cleanup()
        }
    }
}
