// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) — Kotlin
// compile+run gate.
//
// The committed SMs (com/sce/integration/<machine>/<machine>Sm.kt) are
// generated from sce-build/tests/fixtures/static_datamodel/<machine>.scxml
// (regen: scripts/regen_static_datamodel_kotlin.sh). Their variables are
// Kotlin fields and every expression — the guard reading `count` and `In()`,
// the `<assign>`s, the `<if>` / `<elseif>` pair, a record's field updates —
// was lowered to native Kotlin, so each machine is constructed with NO script
// engine and its datamodel is read straight off the fields.

package com.sce.integration

import com.sce.integration.static_counter.StaticCounterEvent
import com.sce.integration.static_counter.StaticCounterState
import com.sce.integration.static_counter.StaticCounterStateMachine
import com.sce.integration.static_host_call.RecordingStaticHostCallActions
import com.sce.integration.static_host_call.StaticHostCallEvent
import com.sce.integration.static_host_call.StaticHostCallState
import com.sce.integration.static_host_call.StaticHostCallStateMachine
import com.sce.integration.static_list.StaticListEvent
import com.sce.integration.static_list.StaticListStateMachine
import com.sce.integration.static_record.StaticRecordDayRecord
import com.sce.integration.static_record.StaticRecordEvent
import com.sce.integration.static_record.StaticRecordStateMachine
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

    @Test
    fun theSnapshotIsPublishedAtEachMacrostepBoundary() {
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            // The first macrostep settles inside initialize().
            val first = sm.snapshot.value
            assertEquals(setOf(StaticCounterState.Counting), first.configuration)
            assertEquals(StaticCounterStateMachine.Data(count = 0u, ready = false), first.data)
            assertEquals(false, first.truncated)

            ticks(sm, 5)
            assertEquals(
                StaticCounterStateMachine.Data(count = 5u, ready = true),
                sm.snapshot.value.data,
                "the snapshot carries the datamodel as the macrostep left it"
            )

            sm.send(StaticCounterEvent.Go)
            sm.tick()
            assertEquals(
                setOf(StaticCounterState.Done),
                sm.snapshot.value.configuration,
                "reaching a top-level final state is a macrostep boundary too"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aPublishedSnapshotDoesNotChangeAfterwards() {
        // Immutable by construction: a host holding an earlier snapshot keeps
        // what it saw, whatever the machine does next.
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            val before = sm.snapshot.value
            ticks(sm, 3)
            assertEquals(StaticCounterStateMachine.Data(count = 0u, ready = false), before.data)
            assertEquals(setOf(StaticCounterState.Counting), before.configuration)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aHostActionTakesTypedDatamodelArgumentsWithNoEventInScope() {
        // `<sce:action name="showAttempts">` in `<onentry>`, its arguments a
        // variable and a comparison over it. Under any other data model an
        // eventless action takes no arguments; here each is a typed
        // expression, and the host method's parameter types are theirs.
        // The generated recording host: no hand-written stand-in, and its
        // calls cannot drift from the interface they record.
        val host = RecordingStaticHostCallActions()
        val sm = StaticHostCallStateMachine(host)
        sm.initialize()
        try {
            repeat(4) {
                sm.send(StaticHostCallEvent.Retry)
                sm.tick()
            }
            assertEquals(
                listOf(
                    RecordingStaticHostCallActions.Call.ShowAttempts(0u, false),
                    RecordingStaticHostCallActions.Call.ShowAttempts(1u, false),
                    RecordingStaticHostCallActions.Call.ShowAttempts(2u, false),
                    RecordingStaticHostCallActions.Call.ShowAttempts(3u, true),
                ),
                host.calls,
                "one call per entry of `idle`, each with the datamodel as it stood; " +
                    "the fourth retry finds `attempts < 3` false and re-enters nothing"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aMachineThatPublishesNoVariableSnapshotsItsConfigurationAlone() {
        // `attempts` is not declared sce:direction="out", so it is the
        // machine's own: the snapshot carries no `Data`, only where the
        // machine is.
        val sm = StaticHostCallStateMachine(RecordingStaticHostCallActions())
        sm.initialize()
        try {
            assertEquals(setOf(StaticHostCallState.Idle), sm.snapshot.value.configuration)
            assertEquals(false, sm.snapshot.value.truncated)
        } finally {
            sm.cleanup()
        }
    }

    // ── static_record: a record variable, built whole, updated field by field ──

    private fun day(year: Int, month: Int, dayOfMonth: Int) =
        StaticRecordDayRecord(year.toUShort(), month.toUByte(), dayOfMonth.toUByte())

    @Test
    fun aRecordVariableStartsAsItsSetsBuiltIt() {
        val sm = StaticRecordStateMachine()
        sm.initialize()
        try {
            assertEquals(day(2026, 9, 24), sm.shown, "one <sce:set> per field of Day")
            assertEquals(
                StaticRecordStateMachine.Data(shown = day(2026, 9, 24), refusals = 0u),
                sm.snapshot.value.data,
                "the snapshot carries the record as one value"
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aFieldIsUpdatedAloneAndAGuardReadsIt() {
        // `shown.dayOfMonth < 28` guards `shown.dayOfMonth + 1`: four steps
        // reach 28, and the fifth finds the guard false. The other fields
        // are carried over untouched by each update.
        val sm = StaticRecordStateMachine()
        sm.initialize()
        try {
            repeat(5) {
                sm.send(StaticRecordEvent.Next)
                sm.tick()
            }
            assertEquals(day(2026, 9, 28), sm.shown)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aTypedPayloadReplacesTheRecordFieldByField() {
        val sm = StaticRecordStateMachine()
        sm.initialize()
        try {
            sm.raiseDayPicked(2027.toUShort(), 1.toUByte(), 3.toUByte())
            sm.tick()
            assertEquals(day(2027, 1, 3), sm.shown)
            assertEquals(day(2027, 1, 3), sm.snapshot.value.data.shown)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aDeliveryWithoutThePayloadRunsNoneOfTheContent() {
        // W3C SCXML 3.12.2 / 4.9: the content reads a payload this delivery
        // did not carry — an execution error, which stops the block before
        // any of it runs and goes on the internal queue for the document to
        // answer.
        val sm = StaticRecordStateMachine()
        sm.initialize()
        try {
            sm.send(StaticRecordEvent.Day.Picked)
            sm.tick()
            assertEquals(day(2026, 9, 24), sm.shown, "no field was assigned")
            assertEquals(1u, sm.refusals, "error.execution reached the document once")
        } finally {
            sm.cleanup()
        }
    }

    // ── static_list: a list variable, appended to, cleared, held to its bound ──

    private fun pick(sm: StaticListStateMachine, dayOfMonth: Int) {
        sm.raiseDayPicked(2026.toUShort(), 9.toUByte(), dayOfMonth.toUByte())
        sm.tick()
    }

    @Test
    fun aListStartsEmptyAndTakesEachAppendInOrder() {
        val sm = StaticListStateMachine()
        sm.initialize()
        try {
            assertEquals(emptyList<UByte>(), sm.picked)
            pick(sm, 3)
            pick(sm, 1)
            assertEquals(listOf(3.toUByte(), 1.toUByte()), sm.picked)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anAppendPastTheCapacityAppendsNothingAndIsAnExecutionError() {
        // sce:capacity="3" is kept on every backend: the fourth pick finds
        // the list full, leaves it as it was, and says so with
        // error.execution (W3C SCXML 3.12.2) rather than growing.
        val sm = StaticListStateMachine()
        sm.initialize()
        try {
            listOf(5, 6, 7, 8).forEach { pick(sm, it) }
            assertEquals(listOf(5.toUByte(), 6.toUByte(), 7.toUByte()), sm.picked)
            assertEquals(1u, sm.refusals)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aClearEmptiesTheListAndAPublishedSnapshotKeepsWhatItSaw() {
        val sm = StaticListStateMachine()
        sm.initialize()
        try {
            pick(sm, 9)
            val before = sm.snapshot.value
            sm.send(StaticListEvent.Reset)
            sm.tick()
            assertEquals(emptyList<UByte>(), sm.picked)
            assertEquals(emptyList<UByte>(), sm.snapshot.value.data.picked)
            assertEquals(
                listOf(9.toUByte()),
                before.data.picked,
                "the list is immutable, so the earlier snapshot is unchanged"
            )
        } finally {
            sm.cleanup()
        }
    }
}
