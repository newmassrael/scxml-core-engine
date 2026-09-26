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
import com.sce.integration.static_overflow.StaticOverflowEvent
import com.sce.integration.static_overflow.StaticOverflowState
import com.sce.integration.static_overflow.StaticOverflowStateMachine
import com.sce.integration.static_record.StaticRecordDayRecord
import com.sce.integration.static_record.StaticRecordEvent
import com.sce.integration.static_record.StaticRecordStateMachine
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import java.io.File
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
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
            assertEquals(StaticCounterState.Done, sm.terminalState, "ready is true after five ticks")
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
        // `shown.dayOfMonth < DaysInMonth(shown.year, shown.month)` guards
        // `shown.dayOfMonth + 1`, the bound computed by the imported
        // algorithm: September has 30 days, so six steps from the 24th reach
        // 30 and the seventh finds the guard false. The other fields are
        // carried over untouched by each update.
        val sm = StaticRecordStateMachine()
        sm.initialize()
        try {
            repeat(7) {
                sm.send(StaticRecordEvent.Next)
                sm.tick()
            }
            assertEquals(day(2026, 9, 30), sm.shown)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theImportedAlgorithmBoundsTheStepByTheMonthItIsIn() {
        // The same guard in February: 28 days in 2027, 29 in the leap year
        // 2028 — the algorithm is called with the record's fields each time,
        // not folded to a constant.
        val sm = StaticRecordStateMachine()
        sm.initialize()
        try {
            sm.raiseDayPicked(2027.toUShort(), 2.toUByte(), 27.toUByte())
            sm.tick()
            repeat(3) {
                sm.send(StaticRecordEvent.Next)
                sm.tick()
            }
            assertEquals(day(2027, 2, 28), sm.shown)

            sm.raiseDayPicked(2028.toUShort(), 2.toUByte(), 27.toUByte())
            sm.tick()
            repeat(3) {
                sm.send(StaticRecordEvent.Next)
                sm.tick()
            }
            assertEquals(day(2028, 2, 29), sm.shown)
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
    fun aListIsMeasuredByLenInAnAssignmentAndInAGuard() {
        // `count` is assigned len(picked) after each append, and `full` is
        // taken only while len(picked) === 3.
        val sm = StaticListStateMachine()
        sm.initialize()
        try {
            pick(sm, 4)
            pick(sm, 5)
            assertEquals(2u, sm.count)
            sm.send(StaticListEvent.Full)
            sm.tick()
            assertEquals(
                setOf(com.sce.integration.static_list.StaticListState.Collecting),
                sm.snapshot.value.configuration,
                "two of three picked: the guard holds `full` back"
            )
            pick(sm, 6)
            assertEquals(3u, sm.count)
            sm.send(StaticListEvent.Full)
            sm.tick()
            assertEquals(
                setOf(com.sce.integration.static_list.StaticListState.Done),
                sm.snapshot.value.configuration
            )
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

    // ── static_overflow: an integer operation that overflows is received ──

    private fun overflowSend(sm: StaticOverflowStateMachine, event: StaticOverflowEvent) {
        sm.send(event)
        sm.tick()
    }

    @Test
    fun anAssignmentThatOverflowsIsSkippedAndIsAnExecutionError() {
        // `level + 3` from 253 would be 256, which a uint8 cannot hold: the
        // assignment does not happen — no wrap to 0 — and error.execution
        // says so (SCE_FORGE.md §3.4.1, E12 D5).
        val sm = StaticOverflowStateMachine()
        sm.initialize()
        try {
            overflowSend(sm, StaticOverflowEvent.Up)
            assertEquals(253.toUByte(), sm.level)
            assertEquals(0u, sm.refusals)
            overflowSend(sm, StaticOverflowEvent.Up)
            assertEquals(253.toUByte(), sm.level, "the overflowing assignment wrote nothing")
            assertEquals(1u, sm.refusals, "error.execution reached the document once")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aConditionThatOverflowsIsFalseAndIsAnExecutionError() {
        // W3C SCXML 5.9: a condition that cannot be evaluated is false, and
        // error.execution says why — `level + 10` overflows at 253.
        val sm = StaticOverflowStateMachine()
        sm.initialize()
        try {
            overflowSend(sm, StaticOverflowEvent.Up)
            overflowSend(sm, StaticOverflowEvent.Probe)
            assertEquals(setOf(StaticOverflowState.Waiting), sm.snapshot.value.configuration)
            assertEquals(1u, sm.refusals)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aSumIsCheckedAtItsOperandsWidthNotAtTheComparisons() {
        // `level + 10` is a uint8 operation, so from 250 the sum is 260 —
        // past what it can hold — and the guard fails even though 260 > 0
        // would hold in a wider type: one rule for every place an integer is
        // computed (SCE_FORGE.md §3.4.1).
        val sm = StaticOverflowStateMachine()
        sm.initialize()
        try {
            overflowSend(sm, StaticOverflowEvent.Probe)
            assertEquals(setOf(StaticOverflowState.Waiting), sm.snapshot.value.configuration)
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

    // ── saving a machine and restoring it into a new process (E17) ──────────
    //
    // Each saved state goes through its JSON text and back before it is
    // restored, as one that crossed a process boundary would.

    private fun throughJson(saved: SavedState): SavedState = SavedState.fromJson(saved.toJson())

    /**
     * The saved state the shared fixture holds — the text every backend must
     * save after the same run and must restore from, which is what makes a
     * state saved by one backend a state another can read.
     */
    private fun sharedFixture(machine: String): String =
        File(repoRoot(), "sce-build/tests/fixtures/static_datamodel/saved/$machine.json").readText().trim()

    // Found by walking up rather than by a fixed depth, because Gradle's
    // working directory is the project's and that is a build detail.
    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no ancestor of ${System.getProperty("user.dir")} holds sce-build/")

    @Test
    fun theSavedStateSchemaFileDeclaresTheStatusThisRuntimeDoes() {
        val schema = com.sce.runtime.Json.parse(
            File(repoRoot(), "schemas/sce-saved-state.v1.schema.json").readText()
        ) as Map<*, *>
        assertEquals(
            SavedState.SCHEMA_STATUS,
            schema["x-sce-schema-status"],
            "SCHEMA_STATUS and the schema header move together (SCE_WIRE_CONTRACTS.md)"
        )
    }

    @Test
    fun aRestoredMachineCarriesOnWhereTheSavedOneStood() {
        // Seven ticks: count 7, and `ready` set at 5 and not yet cleared.
        // `step` is the machine's own — saved though no snapshot publishes it.
        val sm = StaticCounterStateMachine()
        sm.initialize()
        val restored = StaticCounterStateMachine()
        try {
            ticks(sm, 7)
            restored.restore(throughJson(sm.save()))
            assertEquals(sm.snapshot.value, restored.snapshot.value)

            ticks(sm, 2)
            ticks(restored, 2)
            assertEquals(
                sm.snapshot.value,
                restored.snapshot.value,
                "both run on alike: count 9, and the <elseif> cleared ready"
            )
        } finally {
            sm.cleanup()
            restored.cleanup()
        }
    }

    @Test
    fun aRestoreRunsNoOnentry() {
        // `idle`'s <onentry> calls the host. The saved run already made that
        // call; the restored machine must not make it again.
        val sm = StaticHostCallStateMachine(RecordingStaticHostCallActions())
        sm.initialize()
        val host = RecordingStaticHostCallActions()
        val restored = StaticHostCallStateMachine(host)
        try {
            sm.send(StaticHostCallEvent.Retry)
            sm.tick()
            restored.restore(throughJson(sm.save()))
            assertEquals(emptyList<Any>(), host.calls, "restoring entered nothing, so it called nothing")
            restored.send(StaticHostCallEvent.Retry)
            restored.tick()
            assertEquals(
                listOf(RecordingStaticHostCallActions.Call.ShowAttempts(2u, false)),
                host.calls,
                "the next entry sees the attempts the saved run had made"
            )
        } finally {
            sm.cleanup()
            restored.cleanup()
        }
    }

    @Test
    fun aRecordAndAListSaveTheTextEveryBackendSaves() {
        // The same runs the Rust suite makes, saving the shared fixture's text
        // byte for byte: keys are the document's ids, a record is an object of
        // its schema's fields, a list an array.
        val record = StaticRecordStateMachine()
        record.initialize()
        val list = StaticListStateMachine()
        list.initialize()
        try {
            record.raiseDayPicked(2027.toUShort(), 2.toUByte(), 27.toUByte())
            record.tick()
            assertEquals(sharedFixture("static_record"), record.save().toJson())

            pick(list, 4)
            pick(list, 2)
            assertEquals(sharedFixture("static_list"), list.save().toJson())
        } finally {
            record.cleanup()
            list.cleanup()
        }
    }

    @Test
    fun aStateAnotherBackendSavedIsRestored() {
        val record = StaticRecordStateMachine()
        val list = StaticListStateMachine()
        try {
            record.restore(SavedState.fromJson(sharedFixture("static_record")))
            assertEquals(day(2027, 2, 27), record.shown)

            list.restore(SavedState.fromJson(sharedFixture("static_list")))
            assertEquals(listOf(4.toUByte(), 2.toUByte()), list.picked)
            assertEquals(2u, list.count)
            pick(list, 6)
            assertEquals(3u, list.count, "the restored machine runs on from the saved values")
        } finally {
            record.cleanup()
            list.cleanup()
        }
    }

    @Test
    fun aStateSavedFromAnotherDocumentIsRefused() {
        val sm = StaticCounterStateMachine()
        try {
            val refusal = assertThrows(StateRefusal::class.java) {
                sm.restore(SavedState.fromJson(sharedFixture("static_list")))
            }
            assertTrue(refusal.message!!.contains("shape"), refusal.message)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aListLongerThanItsBoundIsRefusedAndTheMachineIsLeftAsItWas() {
        // A machine never holds more than sce:capacity="3"; a saved state that
        // claims it did is not one this machine wrote. `refusals` reads before
        // `picked` fails, and still nothing is written.
        val text = sharedFixture("static_list").replace("\"picked\":[4,2]", "\"picked\":[1,2,3,4]")
        val sm = StaticListStateMachine()
        try {
            val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text)) }
            assertTrue(refusal.message!!.contains("bounded by"), refusal.message)
            assertEquals(emptyList<UByte>(), sm.picked)
            assertEquals(0u, sm.count)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aConfigurationThatIsNotOneOfTheDocumentIsRefused() {
        val text = sharedFixture("static_list").replace("[\"collecting\"]", "[\"nowhere\"]")
        val sm = StaticListStateMachine()
        try {
            val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text)) }
            assertTrue(refusal.message!!.contains("nowhere"), refusal.message)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aMachineThatIsNotRunningIsNotSavedAndAStartedOneIsNotRestored() {
        val fresh = StaticCounterStateMachine()
        val list = StaticListStateMachine()
        list.initialize()
        try {
            assertThrows(StateRefusal::class.java) { fresh.save() }
            assertThrows(StateRefusal::class.java) {
                list.restore(SavedState.fromJson(sharedFixture("static_list")))
            }
            listOf(1, 2, 3).forEach { pick(list, it) }
            list.send(StaticListEvent.Full)
            list.tick()
            assertThrows(StateRefusal::class.java, { list.save() }, "ended at a top-level <final>")
        } finally {
            fresh.cleanup()
            list.cleanup()
        }
    }
}
