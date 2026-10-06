// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

import com.sce.integration.static_bytes.StaticBytesStateMachine
import com.sce.integration.static_counter.StaticCounterEvent
import com.sce.integration.static_counter.StaticCounterState
import com.sce.integration.static_counter.StaticCounterStateMachine
import com.sce.integration.static_enum.StaticEnumEvent
import com.sce.integration.static_enum.StaticEnumStateMachine
import com.sce.integration.static_enum.StaticEnumViewModeEnum
import com.sce.integration.static_host_call.RecordingStaticHostCallActions
import com.sce.integration.static_host_call.StaticHostCallEvent
import com.sce.integration.static_host_call.StaticHostCallState
import com.sce.integration.static_host_call.StaticHostCallStateMachine
import com.sce.integration.static_history.StaticHistoryEvent
import com.sce.integration.static_history.StaticHistoryState
import com.sce.integration.static_history.StaticHistoryStateMachine
import com.sce.integration.static_list.StaticListEvent
import com.sce.integration.static_list.StaticListStateMachine
import com.sce.integration.static_overflow.StaticOverflowEvent
import com.sce.integration.static_overflow.StaticOverflowState
import com.sce.integration.static_overflow.StaticOverflowStateMachine
import com.sce.integration.static_record.StaticRecordDayRecord
import com.sce.integration.static_record.StaticRecordEvent
import com.sce.integration.static_record.StaticRecordStateMachine
import com.sce.integration.static_record_bytes.StaticRecordBytesEvent
import com.sce.integration.static_record_bytes.StaticRecordBytesStateMachine
import com.sce.integration.static_record_string.StaticRecordStringStateMachine
import com.sce.integration.static_string_capacity.StaticStringCapacityStateMachine
import com.sce.integration.static_timers.StaticTimersEvent
import com.sce.integration.static_timers.StaticTimersStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import java.io.File
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.jupiter.api.Assertions.assertArrayEquals
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
    fun anEventSentAndNotYetDrivenThroughIsSavedWithTheMachine() {
        // The host sent `tick` and saved before ticking: the event is part of
        // the state — only the internal queue is empty at a macrostep boundary.
        val sm = StaticCounterStateMachine()
        sm.initialize()
        val restored = StaticCounterStateMachine()
        try {
            sm.send(StaticCounterEvent.Tick)
            val saved = throughJson(sm.save())
            assertEquals(1, saved.external.size, saved.toJson())
            restored.restore(saved)
            assertEquals(0u, restored.count, "not yet delivered")
            restored.tick()
            sm.tick()
            assertEquals(1u, restored.count, "delivered after the restore")
            assertEquals(sm.snapshot.value, restored.snapshot.value)
        } finally {
            sm.cleanup()
            restored.cleanup()
        }
    }

    @Test
    fun aMachineItsOwnCoroutineDrivesIsNotSaved() = runBlocking {
        // Its macrosteps run on another thread while a save would read it, and
        // its queued events sit in a channel: refused for the mode, which the
        // host chose, never for the moment.
        val sm = StaticCounterStateMachine()
        sm.start(this)
        try {
            withTimeout(10_000) { sm.snapshot.first { it.configuration.isNotEmpty() } }
            val refusal = assertThrows(StateRefusal::class.java) { sm.save() }
            assertTrue(refusal.message!!.contains("coroutine"), refusal.message)
        } finally {
            sm.stop()
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

    // ── static_enum: a variable that holds a variant of an imported enum ──

    private fun layouts(): StaticEnumStateMachine {
        val sm = StaticEnumStateMachine()
        sm.initialize()
        return sm
    }

    private fun send(sm: StaticEnumStateMachine, event: StaticEnumEvent) {
        sm.send(event)
        sm.tick()
    }

    @Test
    fun anEnumVariableStartsAtItsVariantAndTheSnapshotCarriesIt() {
        val sm = layouts()
        try {
            assertEquals(StaticEnumViewModeEnum.MONTH, sm.layout)
            assertEquals(
                StaticEnumViewModeEnum.MONTH,
                sm.snapshot.value.data.layout,
                "a host reads the variable as the machine's own enum type",
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anEnumVariableIsSavedAsTheNameTheDocumentDeclares() {
        // The same run the Rust suite makes: `agenda_list`, with its
        // underscore, is the document's name for the variant, not the constant
        // this backend spells (`AGENDA_LIST`) nor Rust's (`AgendaList`).
        val sm = layouts()
        try {
            send(sm, StaticEnumEvent.Swap)
            send(sm, StaticEnumEvent.Agenda)
            assertEquals(StaticEnumViewModeEnum.AGENDA_LIST, sm.layout)
            assertEquals(sharedFixture("static_enum"), sm.save().toJson())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aStateAnotherBackendSavedHoldsAnEnumThatIsRestored() {
        val sm = StaticEnumStateMachine()
        try {
            sm.restore(SavedState.fromJson(sharedFixture("static_enum")))
            assertEquals(StaticEnumViewModeEnum.AGENDA_LIST, sm.layout)
            // `previous` is the machine's own and was restored too: back goes to it.
            send(sm, StaticEnumEvent.Back)
            assertEquals(StaticEnumViewModeEnum.WEEK, sm.layout)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aSavedEnumValueThatIsNotADeclaredVariantIsRefusedAndTheMachineIsLeftAsItWas() {
        for ((written, wanted) in listOf(
            // Not a variant of the enum at all.
            "\"layout\":\"yearly\"" to "is not a variant of ViewMode",
            // The constant a backend spells for it is not the name the
            // document declares, so it is not what a saved state holds.
            "\"layout\":\"AgendaList\"" to "is not a variant of ViewMode",
            "\"layout\":\"AGENDA_LIST\"" to "is not a variant of ViewMode",
            // A number is not a name.
            "\"layout\":3" to "is not a text",
        )) {
            val text = sharedFixture("static_enum").replace("\"layout\":\"agenda_list\"", written)
            val sm = StaticEnumStateMachine()
            try {
                val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text)) }
                assertTrue(refusal.message!!.contains(wanted), "$written: ${refusal.message}")
                assertTrue(refusal.message!!.contains("layout"), "names the variable: ${refusal.message}")
                assertEquals(StaticEnumViewModeEnum.MONTH, sm.layout, "nothing was written")
            } finally {
                sm.cleanup()
            }
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
    fun aStringLongerThanItsBoundIsRefusedInBytesAndTheMachineIsLeftAsItWas() {
        // A machine never holds more than sce:capacity="4" UTF-8 bytes in `title`,
        // so a saved state that claims it did is not one this machine wrote. The
        // bound counts bytes, not the UTF-16 units a Kotlin string is made of: two
        // characters of two bytes fit it, and an `é` and a `€` (five bytes) do not.
        val source = StaticStringCapacityStateMachine()
        source.initialize()
        val json = try {
            source.save().toJson()
        } finally {
            source.cleanup()
        }
        for ((claimed, fits) in listOf("abcd" to true, "éé" to true, "abcde" to false, "é€" to false)) {
            val text = json.replace("\"title\":\"ab\"", "\"title\":\"$claimed\"")
            val sm = StaticStringCapacityStateMachine()
            try {
                if (fits) {
                    sm.restore(SavedState.fromJson(text))
                    assertEquals(claimed, sm.title)
                } else {
                    val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text)) }
                    assertTrue(refusal.message!!.contains("bounded by"), refusal.message)
                    assertEquals("ab", sm.title)
                }
            } finally {
                sm.cleanup()
            }
        }
    }

    // A byte string is its Latin-1 text (docs/adr/0005, decision 2): each character
    // is one byte, so four of them fit sce:capacity="4" whatever their code points
    // are, five do not, and a character past U+00FF is no byte at all.
    @Test
    fun aByteStringIsSavedAsLatin1TextAndRefusedPastItsBoundOrAByte() {
        val source = StaticBytesStateMachine()
        source.initialize()
        val json = try {
            source.save().toJson()
        } finally {
            source.cleanup()
        }
        for ((claimed, fits) in listOf("abcd" to true, "éÿ\u0080ÿ" to true, "abcde" to false, "é€" to false)) {
            val text = json.replace("\"tail\":\"xy\"", "\"tail\":\"$claimed\"")
            val sm = StaticBytesStateMachine()
            try {
                if (fits) {
                    sm.restore(SavedState.fromJson(text))
                    val bytes = claimed.map { it.code.toByte() }.toByteArray()
                    assertArrayEquals(bytes, sm.tail)
                    // Written back, and read again, as the same bytes: none became another.
                    val again = StaticBytesStateMachine()
                    try {
                        again.restore(SavedState.fromJson(sm.save().toJson()))
                        assertArrayEquals(bytes, again.tail)
                    } finally {
                        again.cleanup()
                    }
                } else {
                    val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text)) }
                    assertTrue(refusal.message!!.contains("tail"), refusal.message)
                    assertArrayEquals("xy".toByteArray(), sm.tail)
                }
            } finally {
                sm.cleanup()
            }
        }
    }

    // A host that is handed a byte string is handed a copy of it: the array is the
    // machine's own, and a write into it would change the variable behind its bound.
    @Test
    fun aHostThatWritesIntoAByteStringItWasHandedChangesNothingOfTheMachine() {
        val sm = StaticBytesStateMachine()
        sm.initialize()
        try {
            sm.frame[0] = 'z'.code.toByte()
            assertArrayEquals("ab".toByteArray(), sm.frame)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRecordStringFieldLongerThanItsBoundIsRefusedInBytesAndTheMachineIsLeftAsItWas() {
        // A machine never holds more than the sce:max-size="8" UTF-8 bytes its schema
        // declares in `last.label`, so a saved state that claims it did is not one
        // this machine wrote. The bound counts bytes: seven bytes in three characters
        // fit it, and nine bytes in four do not.
        val source = StaticRecordStringStateMachine()
        source.initialize()
        val json = try {
            source.save().toJson()
        } finally {
            source.cleanup()
        }
        for ((claimed, fits) in listOf("abcdefgh" to true, "é€é" to true, "abcdefghi" to false, "é€éé" to false)) {
            val text = json.replace("\"label\":\"a\"", "\"label\":\"$claimed\"")
            val sm = StaticRecordStringStateMachine()
            try {
                if (fits) {
                    sm.restore(SavedState.fromJson(text))
                    assertEquals(claimed, sm.last.label)
                } else {
                    val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text)) }
                    assertTrue(refusal.message!!.contains("last.label"), refusal.message)
                    assertTrue(refusal.message!!.contains("bounded by"), refusal.message)
                    assertEquals("a", sm.last.label)
                }
            } finally {
                sm.cleanup()
            }
        }
    }

    @Test
    fun aRecordBytesFieldLongerThanItsBoundOrHoldingNoByteIsRefusedAndTheMachineIsLeftAsItWas() {
        // A machine never holds more than the sce:max-size="8" bytes its schema
        // declares in `last.frame`, so a saved state that claims it did is not one this
        // machine wrote, and a character past U+00FF is no byte at all.
        val source = StaticRecordBytesStateMachine()
        source.initialize()
        val json = try {
            source.save().toJson()
        } finally {
            source.cleanup()
        }
        for ((claimed, fits) in listOf("abcdefgh" to true, "éÿ\u0080ÿ" to true, "abcdefghi" to false, "é€" to false)) {
            val text = json.replace("\"frame\":\"ab\"", "\"frame\":\"$claimed\"")
            val sm = StaticRecordBytesStateMachine()
            try {
                if (fits) {
                    sm.restore(SavedState.fromJson(text))
                    assertArrayEquals(claimed.map { it.code.toByte() }.toByteArray(), sm.last.frame)
                } else {
                    val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text)) }
                    assertTrue(refusal.message!!.contains("last.frame"), refusal.message)
                    assertArrayEquals("ab".toByteArray(), sm.last.frame)
                }
            } finally {
                sm.cleanup()
            }
        }
    }

    @Test
    fun aHostThatWritesIntoARecordsBytesItWasHandedChangesNothingOfTheMachine() {
        // The array is the machine's own, and a write into it would change the record
        // behind the bound the machine keeps: a record, and each record of a list, is
        // handed out with a copy of its bytes. Two records of the same bytes are equal.
        val sm = StaticRecordBytesStateMachine()
        sm.initialize()
        try {
            sm.send(StaticRecordBytesEvent.Keep)
            sm.tick()
            sm.last.frame[0] = 'z'.code.toByte()
            sm.frames[0].frame[0] = 'z'.code.toByte()
            assertArrayEquals("ab".toByteArray(), sm.last.frame)
            assertArrayEquals("ab".toByteArray(), sm.frames[0].frame)
            assertEquals(sm.last, sm.frames[0])
            assertEquals(sm.last.hashCode(), sm.frames[0].hashCode())
            sm.send(StaticRecordBytesEvent.Fill)
            sm.tick()
            assertTrue(sm.last != sm.frames[0])
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

    // ── what a <history> recorded is part of the saved state ────────────────

    /**
     * The run `static_history.scxml` describes: `last`, `deepest` and
     * `crew_all` all recorded, standing in `paused`.
     */
    private val historyRun = listOf(
        StaticHistoryEvent.Faster,
        StaticHistoryEvent.Boost,
        StaticHistoryEvent.Pause,
        StaticHistoryEvent.Work,
        StaticHistoryEvent.LeftNext,
        StaticHistoryEvent.RightNext,
        StaticHistoryEvent.Break,
    )

    private fun historyDrive(sm: StaticHistoryStateMachine, events: List<StaticHistoryEvent>) {
        for (event in events) {
            sm.send(event)
            sm.tick()
        }
    }

    /** A machine restored from the shared fixture's text, and cleaned up after [body]. */
    private fun <T> withRestoredHistory(saved: SavedState = SavedState.fromJson(sharedFixture("static_history")), body: (StaticHistoryStateMachine) -> T): T {
        val sm = StaticHistoryStateMachine()
        try {
            sm.restore(saved)
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aHistoryIsSavedWithTheMachineAsTheTextEveryBackendSaves() {
        val sm = StaticHistoryStateMachine()
        sm.initialize()
        try {
            historyDrive(sm, historyRun)
            assertEquals(sharedFixture("static_history"), sm.save().toJson())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aMachineThatHasExitedNothingRecordsNoHistory() {
        val sm = StaticHistoryStateMachine()
        sm.initialize()
        try {
            assertEquals(emptyMap<String, List<String>>(), sm.save().history)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredMachineResumesThroughEachHistoryAsTheSavedOneWould() {
        // Shallow: the child `fast` was active, and entering it takes its
        // initial child.
        withRestoredHistory { sm ->
            historyDrive(sm, listOf(StaticHistoryEvent.ResumeLast))
            assertTrue(StaticHistoryState.Cruise in sm.snapshot.value.configuration, "${sm.snapshot.value}")
            assertEquals(1u.toUByte(), sm.resumed)
        }
        // Deep: the atomic state itself.
        withRestoredHistory { sm ->
            historyDrive(sm, listOf(StaticHistoryEvent.ResumeDeep))
            assertTrue(StaticHistoryState.Burst in sm.snapshot.value.configuration, "${sm.snapshot.value}")
        }
        // Deep, below a <parallel>: one state in EACH region.
        withRestoredHistory { sm ->
            historyDrive(sm, listOf(StaticHistoryEvent.ResumeCrew))
            val configuration = sm.snapshot.value.configuration
            assertTrue(StaticHistoryState.L2 in configuration && StaticHistoryState.R2 in configuration, "$configuration")
            assertTrue(StaticHistoryState.L1 !in configuration && StaticHistoryState.R1 !in configuration, "$configuration")
        }
    }

    @Test
    fun aHistoryThatWasNeverRecordedTakesItsDefaultAfterARestore() {
        // Saved before any exit of `running`, `last` has nothing recorded, so
        // resuming through it takes its default transition, as an unsaved
        // machine would.
        val shared = SavedState.fromJson(sharedFixture("static_history"))
        val saved = SavedState(
            shape = shared.shape,
            configuration = shared.configuration,
            current = shared.current,
            variables = shared.variables,
            history = shared.history.filterKeys { it != "last" },
            external = shared.external,
        )
        withRestoredHistory(saved) { sm ->
            historyDrive(sm, listOf(StaticHistoryEvent.ResumeLast))
            assertTrue(StaticHistoryState.Slow in sm.snapshot.value.configuration, "${sm.snapshot.value}")
        }
    }

    @Test
    fun aSavedHistorySurvivesItsMachineBeingSavedAgain() {
        withRestoredHistory { sm ->
            assertEquals(sharedFixture("static_history"), sm.save().toJson(), "restoring and saving again changes nothing")
            historyDrive(sm, listOf(StaticHistoryEvent.ResumeLast, StaticHistoryEvent.Pause))
            assertEquals(
                mapOf(
                    "crew_all" to listOf("l2", "r2"),
                    "deepest" to listOf("cruise"),
                    "last" to listOf("fast"),
                ),
                sm.save().history,
                "resumed through `last`, then left `running` again: `deepest` now records `cruise`",
            )
        }
    }

    /** The refusal a restore of `static_history` answers with once `history` is [entries]. */
    private fun refusedHistory(vararg entries: Pair<String, List<String>>): String {
        val shared = SavedState.fromJson(sharedFixture("static_history"))
        val saved = SavedState(
            shape = shared.shape,
            configuration = shared.configuration,
            current = shared.current,
            variables = shared.variables,
            history = linkedMapOf(*entries),
            external = shared.external,
        )
        val sm = StaticHistoryStateMachine()
        try {
            val refusal = assertThrows(StateRefusal::class.java) { sm.restore(saved) }
            // A refused restore leaves the machine as it was: never started.
            assertThrows(StateRefusal::class.java) { sm.save() }
            return refusal.message!!
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aHistoryTheDocumentDoesNotDeclareIsRefused() {
        assertTrue(refusedHistory("nowhere" to listOf("fast")).contains("does not declare"))

        // A document with no <history> refuses a state that records one.
        val counter = StaticCounterStateMachine()
        counter.initialize()
        val restored = StaticCounterStateMachine()
        try {
            ticks(counter, 1)
            val saved = counter.save()
            val withHistory = SavedState(
                shape = saved.shape,
                configuration = saved.configuration,
                current = saved.current,
                variables = saved.variables,
                history = mapOf("last" to listOf("counting")),
                external = saved.external,
            )
            val refusal = assertThrows(StateRefusal::class.java) { restored.restore(withHistory) }
            assertTrue(refusal.message!!.contains("does not declare"), refusal.message)
        } finally {
            counter.cleanup()
            restored.cleanup()
        }
    }

    @Test
    fun aHistoryNamingAStateTheDocumentLacksIsRefused() {
        assertTrue(refusedHistory("last" to listOf("warp")).contains("warp"))
    }

    @Test
    fun aHistoryThatRecordsNothingOrAStateTwiceIsRefused() {
        assertTrue(refusedHistory("last" to emptyList()).contains("at least one"))
        assertTrue(refusedHistory("last" to listOf("fast", "fast")).contains("twice"))
    }

    @Test
    fun aHistoryNamingAStateOutsideItsParentIsRefused() {
        assertTrue(refusedHistory("last" to listOf("l1")).contains("not below"))
        // The parent itself is below nothing.
        assertTrue(refusedHistory("last" to listOf("running")).contains("not below"))
    }

    @Test
    fun aShallowHistoryRecordsChildrenAndADeepOneAtomicStates() {
        assertTrue(refusedHistory("last" to listOf("cruise")).contains("a shallow history records the children"))
        assertTrue(refusedHistory("deepest" to listOf("fast")).contains("a deep history records atomic states"))
    }

    @Test
    fun aHistoryThatIsPartOfNoConfigurationIsRefused() {
        // `running` holds one active child, and these are in two.
        assertTrue(refusedHistory("deepest" to listOf("slow", "burst")).contains("no configuration"))
        // A <parallel> holds every region, and this is one of two.
        assertTrue(refusedHistory("crew_all" to listOf("l2")).contains("no configuration"))
        // Two states of one region.
        assertTrue(refusedHistory("crew_all" to listOf("l1", "l2", "r1")).contains("no configuration"))
    }

    @Test
    fun aSavedStateWhoseObjectRepeatsANameIsNotThisFormat() {
        // `{"last":[..],"last":[..]}` has no single meaning: a reader that took
        // the first and one that took the last would restore two machines from
        // one text, so neither backend reads it.
        val text = sharedFixture("static_history").replace("\"last\":[\"fast\"]", "\"last\":[\"fast\"],\"last\":[\"slow\"]")
        val refusal = assertThrows(StateRefusal::class.java) { SavedState.fromJson(text) }
        assertTrue(refusal.message!!.contains("appears twice"), refusal.message)
    }

    // ── a delayed <send> still waiting is part of the saved state ───────────
    //
    // `static_timers.scxml` arms four on entering `waiting`; each appends a
    // digit to `trace` when delivered, so the number says which arrived and in
    // what order. Everything runs on a host-owned clock, so no case sleeps and
    // none depends on how loaded the machine is.

    /** The wall-clock moment the shared fixtures were saved at, as the Rust suite has it: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private fun timers(): StaticTimersStateMachine {
        val sm = StaticTimersStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        return sm
    }

    /** [text] restored [elapsedMs] of wall-clock time after it was saved, into a process whose own clock starts at 0. */
    private fun <T> withRestoredTimers(text: String, elapsedMs: Long, body: (StaticTimersStateMachine) -> T): T {
        val sm = StaticTimersStateMachine()
        try {
            sm.clock = ManualClock(0)
            sm.restore(SavedState.fromJson(text), savedAtMs + elapsedMs)
            return body(sm)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aWaitingSendIsSavedAsTheWallClockMomentItComesDue() {
        // The same runs the Rust suite makes, saving the shared fixtures' text
        // byte for byte: four sends in the order they would be delivered,
        // `beat` ahead of `echo` since they are due together and were sent in
        // that order.
        val sm = timers()
        try {
            assertEquals(sharedFixture("static_timers"), sm.save(savedAtMs).toJson())

            // 1.5 s later `inner` has been delivered. What is left is due at
            // the same wall-clock moments as before, because the moment is what
            // is saved — a restore is told the time, not the wait.
            sm.advanceTimeMs(1500)
            assertEquals(1u, sm.trace)
            assertEquals(sharedFixture("static_timers_midway"), sm.save(savedAtMs + 1500).toJson())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aSaveWithoutAClockReadsTheWallClock() {
        val sm = timers()
        try {
            val before = SavedState.wallClockMs()
            val saved = sm.save()
            val after = SavedState.wallClockMs()
            // `inner` waits 1 s on an engine whose clock is at 0.
            val due = saved.pending[0].due
            assertTrue(due in (before + 1000)..(after + 1000), "due $due is not 1 s past the wall clock at the save ($before..$after)")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredMachineDeliversEachWaitingSendWhenItsMomentComes() {
        // Back 500 ms after the save: `inner` has 500 ms left, `beat` and
        // `echo` 1500, `timeout` 4500.
        withRestoredTimers(sharedFixture("static_timers"), 500) { sm ->
            assertEquals(0u, sm.trace)
            sm.advanceTimeMs(499)
            assertEquals(0u, sm.trace, "`inner` is not due yet")
            sm.advanceTimeMs(1)
            assertEquals(1u, sm.trace, "`inner`, at 500 ms")
            sm.advanceTimeMs(999)
            assertEquals(1u, sm.trace, "`beat` and `echo` are not due")
            sm.advanceTimeMs(1)
            assertEquals(124u, sm.trace, "`beat` then `echo`, at 1500 ms")
            assertTrue(!sm.isInFinalState)
            sm.advanceTimeMs(3000)
            assertEquals(1243u, sm.trace, "`timeout`, at 4500 ms")
            assertTrue(sm.isInFinalState)
        }
    }

    @Test
    fun aSendAlreadyDueWhenTheMachineComesBackIsDeliveredInItsOrder() {
        // A minute away: every wait ran out while the process was dead. Each is
        // delivered, one macrostep apart, in the order the saved machine would
        // have — `beat` ahead of `echo`, which were due together.
        withRestoredTimers(sharedFixture("static_timers"), 60_000) { sm ->
            assertEquals(0u, sm.trace, "nothing is delivered by restoring")
            sm.advanceTimeMs(0)
            assertEquals(1243u, sm.trace)
            assertTrue(sm.isInFinalState)
        }
    }

    @Test
    fun aWaitThatWasHalfOverWhenSavedHasTheRestToRun() {
        // Saved 1.5 s in with `inner` delivered and `trace` 1; restored at once.
        withRestoredTimers(sharedFixture("static_timers_midway"), 1500) { sm ->
            assertEquals(1u, sm.trace)
            sm.advanceTimeMs(499)
            assertEquals(1u, sm.trace)
            sm.advanceTimeMs(1)
            assertEquals(124u, sm.trace, "`beat` then `echo`")
            sm.advanceTimeMs(3000)
            assertEquals(1243u, sm.trace)
        }
    }

    @Test
    fun aRestoredSendCanStillBeCancelledByItsId() {
        // `stop` cancels `timer`, so what was saved has to carry the id it names.
        withRestoredTimers(sharedFixture("static_timers"), 0) { sm ->
            sm.send(StaticTimersEvent.Stop)
            sm.tick()
            sm.advanceTimeMs(60_000)
            assertEquals(124u, sm.trace, "`timeout` was cancelled")
            assertTrue(!sm.isInFinalState)
        }
    }

    @Test
    fun aMachineRestoredFromATextSavesThatTextAgain() {
        // Nothing is lost on the way through: the order, the moments and the
        // ids a second save writes are the first's.
        for ((machine, elapsed) in listOf("static_timers" to 0L, "static_timers_midway" to 1500L)) {
            withRestoredTimers(sharedFixture(machine), elapsed) { sm ->
                assertEquals(sharedFixture(machine), sm.save(savedAtMs + elapsed).toJson())
            }
        }
    }

    @Test
    fun aWaitingSendNamingAnEventTheDocumentLacksIsRefused() {
        val text = sharedFixture("static_timers").replace("\"event\":\"beat\"", "\"event\":\"warp\"")
        val sm = StaticTimersStateMachine()
        try {
            val refusal = assertThrows(StateRefusal::class.java) { sm.restore(SavedState.fromJson(text), savedAtMs) }
            assertTrue(refusal.message!!.contains("pending[1]"), refusal.message)
            assertTrue(refusal.message!!.contains("warp"), refusal.message)
            // A refused restore leaves the machine as it was: never started.
            assertThrows(StateRefusal::class.java) { sm.save() }
        } finally {
            sm.cleanup()
        }
    }
}
