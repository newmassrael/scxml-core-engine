// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) — scenarios
// every backend that lowers the model replays, the twin of the Rust
// `tests/static_scenarios.rs`.
//
// A scenario (sce-build/tests/fixtures/static_datamodel/scenarios/*.json) is
// data: a list of steps, each an external event with its payload and what
// the machine must hold after it runs to quiescence — its current state and
// any of its variables. What the machine holds is read from its saved state,
// the text every backend saves byte for byte, so one scenario judges every
// backend by the same answer and needs no per-type glue.
//
// The replay is `:sce-kotlin-scenarios` (`com.sce.scenarios.replay`), the one a
// build outside this repository holds its own machines to the same scenarios
// with; this class is the driver of each machine here. An event name no event of
// the machine matches is refused by the Rust and Go twins, which ask the
// machine's own lookup unless the step says `"dropped": true`; here that lookup
// is not public, and a step whose event were dropped would fail on the state it
// expects — or, for a step that expects the drop, on the variables it says did
// not move.

package com.sce.integration

import com.sce.integration.static_block_ends.StaticBlockEndsStateMachine
import com.sce.integration.static_block_ends_list.StaticBlockEndsListStateMachine
import com.sce.integration.static_bytes.StaticBytesStateMachine
import com.sce.integration.static_bytes_wire.StaticBytesWireStateMachine
import com.sce.integration.static_cancel_expr.StaticCancelExprStateMachine
import com.sce.integration.static_child_host.RecordingStaticChildHostActions
import com.sce.integration.static_child_host.RecordingStaticChildHostSceSynthInvokeWorkerActions
import com.sce.integration.static_child_host.StaticChildHostStateMachine
import com.sce.integration.static_child_host_hybrid.RecordingStaticChildHostHybridActions
import com.sce.integration.static_child_host_hybrid.RecordingStaticHostedFirstActions
import com.sce.integration.static_child_host_hybrid.RecordingStaticHostedSecondActions
import com.sce.integration.static_child_host_hybrid.StaticChildHostHybridStateMachine
import com.sce.integration.static_child_mesh.StaticChildMeshStateMachine
import com.sce.integration.static_counter.StaticCounterStateMachine
import com.sce.integration.static_donedata.StaticDonedataStateMachine
import com.sce.integration.static_donedata_content.StaticDonedataContentStateMachine
import com.sce.integration.static_donedata_record.StaticDonedataRecordStateMachine
import com.sce.integration.static_enum.StaticEnumStateMachine
import com.sce.integration.static_enum_number.StaticEnumNumberStateMachine
import com.sce.integration.static_event_arrival.StaticEventArrivalStateMachine
import com.sce.integration.static_event_wildcard.StaticEventWildcardStateMachine
import com.sce.integration.static_foreach.StaticForeachStateMachine
import com.sce.integration.static_forge_calls.StaticForgeCallsStateMachine
import com.sce.integration.static_history.StaticHistoryStateMachine
import com.sce.integration.static_host_call.RecordingStaticHostCallActions
import com.sce.integration.static_host_call.StaticHostCallStateMachine
import com.sce.integration.static_host_duty.RecordingStaticHostDutyActions
import com.sce.integration.static_host_duty.StaticHostDutyStateMachine
import com.sce.integration.static_invoke.StaticInvokeStateMachine
import com.sce.integration.static_invoke_params.StaticInvokeParamsStateMachine
import com.sce.integration.static_invoke_string.StaticInvokeStringStateMachine
import com.sce.integration.static_list.StaticListStateMachine
import com.sce.integration.static_list_assign.StaticListAssignStateMachine
import com.sce.integration.static_list_index.StaticListIndexStateMachine
import com.sce.integration.static_optional.StaticOptionalDueRecord
import com.sce.integration.static_optional.StaticOptionalStateMachine
import com.sce.integration.static_overflow.StaticOverflowStateMachine
import com.sce.integration.static_payload.StaticPayloadStateMachine
import com.sce.integration.static_payload_bytes.StaticPayloadBytesStateMachine
import com.sce.integration.static_payload_enum.StaticPayloadEnumStateMachine
import com.sce.integration.static_payload_relay.StaticPayloadRelayStateMachine
import com.sce.integration.static_real.StaticRealStateMachine
import com.sce.integration.static_real32.StaticReal32StateMachine
import com.sce.integration.static_record.StaticRecordStateMachine
import com.sce.integration.static_record_fields.StaticRecordFieldsStateMachine
import com.sce.integration.static_record_enum.StaticRecordEnumStateMachine
import com.sce.integration.static_record_list.StaticRecordListStateMachine
import com.sce.integration.static_record_list_index.StaticRecordListIndexStateMachine
import com.sce.integration.static_record_real.StaticRecordRealStateMachine
import com.sce.integration.static_record_real32.StaticRecordReal32StateMachine
import com.sce.integration.static_send_content.StaticSendContentStateMachine
import com.sce.integration.static_send_delay.StaticSendDelayStateMachine
import com.sce.integration.static_send_event.StaticSendEventStateMachine
import com.sce.integration.static_send_idlocation.StaticSendIdlocationStateMachine
import com.sce.integration.static_send_namelist.StaticSendNamelistStateMachine
import com.sce.integration.static_send_params.StaticSendParamsStateMachine
import com.sce.integration.static_send_target.StaticSendTargetStateMachine
import com.sce.integration.static_send_type.StaticSendTypeStateMachine
import com.sce.integration.static_record_bytes.StaticRecordBytesStateMachine
import com.sce.integration.static_record_string.StaticRecordStringStateMachine
import com.sce.integration.static_schema_revision.StaticSchemaRevisionStateMachine
import com.sce.integration.static_state_algorithm.StaticStateAlgorithmStateMachine
import com.sce.integration.static_state_algorithm_enum.StaticStateAlgorithmEnumStateMachine
import com.sce.integration.static_string_capacity.StaticStringCapacityStateMachine
import com.sce.integration.static_text_record.StaticTextRecordStateMachine
import com.sce.integration.static_timers.StaticTimersStateMachine
import com.sce.integration.static_whole_payload.StaticWholePayloadStateMachine
import com.sce.integration.static_wire_wide.StaticWireWideStateMachine
import com.sce.integration.static_wire_enum.StaticWireEnumStateMachine
import com.sce.integration.presence_peer.PresencePeerStateMachine
import com.sce.integration.record_list.RecordListStateMachine
import com.sce.integration.record_list_parent.RecordListParentStateMachine
import com.sce.integration.sheet_peer.SheetPeerStateMachine
import com.sce.integration.sync_client.SyncClientStateMachine
import com.sce.integration.todo_peer.TodoPeerStateMachine
import com.sce.runtime.EventMetadata
import com.sce.runtime.ManualClock
import com.sce.scenarios.ChildHosts
import com.sce.scenarios.ChildRouters
import com.sce.scenarios.RecordedSends
import com.sce.scenarios.SETTLE_TICKS
import com.sce.scenarios.heldBy
import com.sce.scenarios.hostCall
import com.sce.scenarios.parseScenario
import com.sce.scenarios.replay
import com.sce.scenarios.routed
import java.io.File
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("StaticScenario — sce-static machines replay the scenarios every backend replays (Kotlin AOT)")
class StaticScenarioTest {

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .first { File(it, "sce-build").isDirectory }

    private fun scenario(machine: String): JsonObject =
        parseScenario(
            File(repoRoot(), "sce-build/tests/fixtures/static_datamodel/scenarios/$machine.json").readText(),
        )

    /**
     * What `static_host_call` asked of its host since this was last called, oldest
     * first, each as a scenario's `host_calls` writes it: the name the document
     * gives the action and its arguments in the order the document gives them.
     */
    private fun staticHostCallCalls(host: RecordingStaticHostCallActions): () -> List<JsonElement> = {
        val fresh =
            host.calls.map { call ->
                when (call) {
                    is RecordingStaticHostCallActions.Call.ShowAttempts ->
                        hostCall("showAttempts", JsonPrimitive(call.count.toLong()), JsonPrimitive(call.exhausted))
                }
            }
        host.clear()
        fresh
    }

    /**
     * What `static_host_duty` asked of its host since this was last called, oldest
     * first, each as a scenario's `host_calls` writes it. None of its actions takes
     * an argument.
     */
    private fun staticHostDutyCalls(host: RecordingStaticHostDutyActions): () -> List<JsonElement> = {
        val fresh =
            host.calls.map { call ->
                val action =
                    when (call) {
                        RecordingStaticHostDutyActions.Call.StartKeepalive -> "start_keepalive"
                        RecordingStaticHostDutyActions.Call.NoteEngaged -> "note_engaged"
                        RecordingStaticHostDutyActions.Call.StopKeepalive -> "stop_keepalive"
                    }
                hostCall(action)
            }
        host.clear()
        fresh
    }

    /** What a `worker` child of `static_child_host` recorded, read and forgotten. */
    private fun workerCalls(host: RecordingStaticChildHostSceSynthInvokeWorkerActions): List<JsonElement> {
        val fresh =
            host.calls.map { call ->
                when (call) {
                    is RecordingStaticChildHostSceSynthInvokeWorkerActions.Call.Started -> hostCall("started")
                    is RecordingStaticChildHostSceSynthInvokeWorkerActions.Call.Finished ->
                        hostCall("finished", JsonPrimitive(call.steps.toLong()))
                }
            }
        host.clear()
        return fresh
    }

    /** What a first candidate of `static_child_host_hybrid` recorded, read and forgotten. */
    private fun firstCalls(host: RecordingStaticHostedFirstActions): List<JsonElement> {
        val fresh =
            host.calls.map { call ->
                when (call) {
                    is RecordingStaticHostedFirstActions.Call.FirstRan -> hostCall("first_ran")
                }
            }
        host.clear()
        return fresh
    }

    /** What a second candidate of `static_child_host_hybrid` recorded, read and forgotten. */
    private fun secondCalls(host: RecordingStaticHostedSecondActions): List<JsonElement> {
        val fresh =
            host.calls.map { call ->
                when (call) {
                    is RecordingStaticHostedSecondActions.Call.SecondRan -> hostCall("second_ran")
                }
            }
        host.clear()
        return fresh
    }

    @Test
    fun syncClientRunsItsScenario() {
        val sm = SyncClientStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("sync_client"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticCounterCountsToTheFlagAndLetsGo() {
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_counter"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // An event arrives by name from outside the document, so the names it can
    // arrive under are open (§scxml-3.12.1): a name the document never writes
    // reaches the transition whose descriptor is a token prefix of it, and one no
    // descriptor matches is dropped.
    @Test
    fun staticEventArrivalDeliversANameTheDocumentDoesNotWrite() {
        val sm = StaticEventArrivalStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_event_arrival"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // ...and where the document listens with `event="*"`, a name no descriptor it
    // writes extends is delivered as the wildcard event instead of being dropped.
    @Test
    fun staticEventWildcardTakesANameTheDocumentDoesNotWrite() {
        val sm = StaticEventWildcardStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_event_wildcard"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A top-level final's <donedata> params are read from the machine's fields
    // when it is entered, and the pair whose value does not fit is left out.
    @Test
    fun staticDonedataHandsTheDoneEventItsParams() {
        val sm = StaticDonedataStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_donedata"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                donedata = { sm.donedataAtFinal() },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A byte string crosses a <param> and a <donedata> as its byte-exact Latin-1 text:
    // sent to itself and read back through the typed payload, and carried by the
    // done event's data.
    @Test
    fun staticBytesWireCarriesAByteStringAsItsLatin1Text() {
        val sm = StaticBytesWireStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_bytes_wire"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                donedata = { sm.donedataAtFinal() },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A top-level final whose <donedata> names a record in its `<content expr>`
    // hands its done event the pairs of the record's fields, read when the state
    // is entered.
    @Test
    fun staticDonedataRecordIsThePairsOfItsFields() {
        val sm = StaticDonedataRecordStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_donedata_record"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                donedata = { sm.donedataAtFinal() },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A top-level final whose <donedata> is inline <content> hands its done event
    // the text as the string it spells, with no script engine to read it as a number.
    @Test
    fun staticDonedataContentIsTheTextItSpells() {
        val sm = StaticDonedataContentStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_donedata_content"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                donedata = { sm.donedataAtFinal() },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A top-level final whose <donedata> carries a <content expr> that names one
    // value hands its done event that value as its whole data: a number as its
    // digits, a string quoted, and one that cannot be computed as the empty string.
    @Test
    fun staticDonedataContentThatNamesAValueIsTheWholeData() {
        replayDonedataContent(scenario("static_donedata_content_value"))
        replayDonedataContent(scenario("static_donedata_content_text"))
        replayDonedataContent(scenario("static_donedata_content_lost"))
    }

    /** One run of `static_donedata_content`, on a machine of its own. */
    private fun replayDonedataContent(scenario: JsonObject) {
        val sm = StaticDonedataContentStateMachine()
        sm.initialize()
        try {
            replay(
                scenario,
                send = { event, data -> sm.sendEventByName(event, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                donedata = { sm.donedataAtFinal() },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticCounterCountsToItsBound() {
        val sm = StaticCounterStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_counter_bound"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticOverflowKeepsItsValueAndSaysSo() {
        val sm = StaticOverflowStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_overflow"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticBlockEndsAtTheErrorThatEndsIt() {
        val sm = StaticBlockEndsStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_block_ends"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticBlockEndsListEndsAtAFullList() {
        val sm = StaticBlockEndsListStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_block_ends_list"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticListFillsToItsCapacityAndIsEmptied() {
        val sm = StaticListStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_list"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticListAssignTakesWhatAnAlgorithmReturnsWhole() {
        val sm = StaticListAssignStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_list_assign"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticListIndexReadsAnElementAndRefusesOneThatIsNotThere() {
        val sm = StaticListIndexStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_list_index"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRecordStepsNoFurtherThanTheMonthAllows() {
        val sm = StaticRecordStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRecordFieldsUpdatesARecordAFieldAtATime() {
        val sm = StaticRecordFieldsStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_fields"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRecordEnumHoldsAnEnumInAFieldOfARecord() {
        val sm = StaticRecordEnumStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_enum"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    /** An optional value as a scenario states it: `null` where it is absent (docs/adr/0055). */
    private fun <T : Any> optionalJson(value: T?, present: (T) -> JsonElement): JsonElement =
        value?.let(present) ?: JsonNull

    /** A `record:Due` of `static_optional` as a scenario states it, field by field. */
    private fun dueJson(record: StaticOptionalDueRecord): JsonObject =
        JsonObject(
            mapOf(
                "id" to JsonPrimitive(record.id.toLong()),
                "due" to optionalJson(record.due) { JsonPrimitive(it) },
                "title" to optionalJson(record.title) { JsonPrimitive(it) },
                "status" to optionalJson(record.status) { JsonPrimitive(it.declaredName) },
            ),
        )

    /**
     * What `static_optional` holds, as its saved state would write it. A machine
     * that holds an optional value has no saved state before docs/adr/0055 step 6,
     * so it is read through the variables it publishes, and its state through the
     * engine's names for its active states.
     */
    private fun staticOptionalHeld(sm: StaticOptionalStateMachine): JsonObject =
        heldBy(
            sm.nameOfState(sm.currentState.value),
            sm.activeConfiguration.map { sm.nameOfState(it) },
            mapOf(
                "due" to optionalJson(sm.due) { JsonPrimitive(it) },
                "other" to optionalJson(sm.other) { JsonPrimitive(it) },
                "title" to optionalJson(sm.title) { JsonPrimitive(it) },
                "status" to optionalJson(sm.status) { JsonPrimitive(it.declaredName) },
                "item" to dueJson(sm.item),
                "items" to JsonArray(sm.items.map { dueJson(it) }),
                "days" to JsonPrimitive(sm.days),
                "following" to JsonPrimitive(sm.following),
                "late" to JsonPrimitive(sm.late),
                "early" to JsonPrimitive(sm.early),
                "label" to JsonPrimitive(sm.label),
                "plain" to JsonPrimitive(sm.plain.declaredName),
                "kept" to JsonPrimitive(sm.kept),
                "listed" to JsonPrimitive(sm.listed),
                "missing" to JsonPrimitive(sm.missing.toLong()),
                "same" to JsonPrimitive(sm.same),
                "absent" to JsonPrimitive(sm.absent),
                "statusDone" to JsonPrimitive(sm.statusDone),
                "echoed" to JsonPrimitive(sm.echoed),
                "echoTitle" to optionalJson(sm.echoTitle) { JsonPrimitive(it) },
                "echoStatus" to optionalJson(sm.echoStatus) { JsonPrimitive(it.declaredName) },
                "fallbacks" to JsonPrimitive(sm.fallbacks.toLong()),
                "errors" to JsonPrimitive(sm.errors.toLong()),
            ),
        )

    // docs/adr/0055: a value that may be absent is held in a variable, a record's
    // field and a list of records, carried by a payload, a send and the done data
    // as `null` or a value, read only where a test proves it present, compared
    // totally, and read with a default evaluated only where it is absent.
    @Test
    fun staticOptionalHoldsTestsAndReadsAValueThatMayBeAbsent() {
        val sm = StaticOptionalStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_optional"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                ended = { sm.isInFinalState },
                donedata = { sm.donedataAtFinal() },
                held = { staticOptionalHeld(sm) },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRecordReal32HoldsASingleFieldAsTheSingleNearestThePayload() {
        val sm = StaticRecordReal32StateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_real32"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRecordRealHoldsARealFieldToTheBit() {
        val sm = StaticRecordRealStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_real"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRecordListAppendsARecordWholeAndWalksItByField() {
        val sm = StaticRecordListStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_list"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRecordListIndexReadsAFieldOfTheElementItIndexes() {
        val sm = StaticRecordListIndexStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_list_index"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The payload of an event is a record of its schema taken whole: it replaces a
    // record variable in one assignment and is appended whole to a list.
    // A 64-bit integer crosses a `<send>`'s `<param>` as the text of its digits and is
    // read back from it: the low digits of a value past 2^53 are the ones that were
    // sent, and a number past 2^53 - 1 is refused where it is read.
    @Test
    fun staticWireWideCarriesA64BitIntegerAsItsDigits() {
        val sm = StaticWireWideStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_wire_wide"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticWholePayloadIsTakenWholeAsARecord() {
        val sm = StaticWholePayloadStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_whole_payload"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A delivery written against an earlier revision of a schema that grew lacks
    // the fields a later revision added, and takes their defaults; a delivery that
    // lacks a field of the first revision is refused, and a field the schema does
    // not name is not read.
    @Test
    fun staticSchemaRevisionTakesTheDefaultsOfTheFieldsAddedSince() {
        val sm = StaticSchemaRevisionStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_schema_revision"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // An enum value as a <param> crosses as the name its enum declares for it: a
    // variable, a field of a record variable and a conditional, sent and read back
    // through the schema.
    @Test
    fun staticWireEnumCarriesTheNameItsEnumDeclares() {
        val sm = StaticWireEnumStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_wire_enum"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticForeachWalksAListAndAFailingBodyEndsTheBlock() {
        val sm = StaticForeachStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_foreach"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A native host action takes the machine's variables as typed arguments, each
    // read when the call is made, and the generated recording host is what the
    // scenario's `host_calls` are read from: one call per entry of `idle`, with the
    // datamodel as it stood.
    @Test
    fun staticHostCallTellsItsHostWhatTheDatamodelHeld() {
        val host = RecordingStaticHostCallActions()
        val sm = StaticHostCallStateMachine(host)
        sm.initialize()
        try {
            replay(
                scenario("static_host_call"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                hostCalls = staticHostCallCalls(host),
            )
        } finally {
            sm.cleanup()
        }
    }

    // A duty is taken on entering `engaged` and given back on leaving it, beside a
    // notice. The recording host simulates, so the machine is built for it on
    // purpose (docs/adr/0038): the scenario's `host_calls` are read from it.
    @Test
    fun staticHostDutyTakesAndReleasesItsDutyThroughItsHost() {
        val host = RecordingStaticHostDutyActions()
        val sm = StaticHostDutyStateMachine(host, acceptSimulatedHost = true)
        sm.initialize()
        try {
            replay(
                scenario("static_host_duty"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                hostCalls = staticHostDutyCalls(host),
            )
        } finally {
            sm.cleanup()
        }
    }

    // A child that declares `<sce:action>`s is built with the host its parent's host
    // answers for it, each time the invocation starts, and what the child asked of
    // that host is what the scenario's `child_hosts` holds. The parent declares no
    // act of its own: the generated recording host takes the function that answers
    // each child's host, and the book records what each of those hosts was asked.
    @Test
    fun staticChildHostGivesEachStartOfItsChildAHostOfItsOwn() {
        val children = ChildHosts()
        val sm = StaticChildHostStateMachine(
            RecordingStaticChildHostActions(
                actionsForWorkerSource = {
                    children.open("worker", null, RecordingStaticChildHostSceSynthInvokeWorkerActions(), ::workerCalls)
                },
            ),
        )
        sm.initialize()
        try {
            replay(
                scenario("static_child_host"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                childHosts = children::take,
            )
        } finally {
            sm.cleanup()
        }
    }

    // A hybrid `<invoke>` is given the host answered for the candidate it starts, and
    // for no other.
    @Test
    fun staticChildHostHybridGivesACandidateTheHostAnsweredForIt() {
        val children = ChildHosts()
        val sm = StaticChildHostHybridStateMachine(
            RecordingStaticChildHostHybridActions(
                actionsForWork0jStaticHostedFirstSource = {
                    children.open("work", "static_hosted_first", RecordingStaticHostedFirstActions(), ::firstCalls)
                },
                actionsForWork0jStaticHostedSecondSource = {
                    children.open("work", "static_hosted_second", RecordingStaticHostedSecondActions(), ::secondCalls)
                },
            ),
        )
        sm.initialize()
        try {
            replay(
                scenario("static_child_host_hybrid"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                childHosts = children::take,
            )
        } finally {
            sm.cleanup()
        }
    }

    // A child session an <invoke> started is driven through its parent by
    // autoforward, takes the events it waits for in order, and its end reaches the
    // parent as `done.invoke`, which the parent counts. The machine needs `tick`,
    // and the engine reports no point at which it has settled: a child takes an
    // event in one round and its end reaches the parent in the next.
    @Test
    fun staticInvokeCountsTheRunOfTheChildItDrives() {
        val sm = StaticInvokeStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_invoke"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // An <invoke> hands its child the values its <param>s and `namelist` name, as
    // they stand when the invoke executes, after the entry actions, and once.
    @Test
    fun staticInvokeParamsHandsTheChildItsValuesOnce() {
        val sm = StaticInvokeParamsStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_invoke_params"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A string an <invoke> hands its child is held to the bound the child declared,
    // in bytes: a value past it is left out and raises `error.execution`, and the
    // child still starts.
    @Test
    fun staticInvokeStringHoldsAHandedStringToTheChildsBound() {
        val sm = StaticInvokeStringStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_invoke_string"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // Leaving the state that holds an <invoke> cancels the child, which then ends
    // nothing and counts nothing.
    @Test
    fun staticInvokeAbortCancelsTheChildWithTheStateThatHoldsIt() {
        val sm = StaticInvokeStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_invoke_abort"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A <history> remembers what its parent held when it was left, and entering
    // it brings that back: the shallow one the child that was active, the deep
    // one the atomic states below, and a deep one of a <parallel> both regions at
    // once. The scenario states the whole active configuration of each step.
    @Test
    fun staticHistoryBringsBackWhatItsParentHeld() {
        val sm = StaticHistoryStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_history"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticRealIsANativeBinary64Field() {
        val sm = StaticRealStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_real"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticReal32IsANativeBinary32Field() {
        val sm = StaticReal32StateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_real32"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticEnumHoldsALayoutAndTheOneItCameFrom() {
        val sm = StaticEnumStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_enum"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // docs/adr/0047: a value of an enum read as the number its variant declares and
    // a number read back as the variant that declares it; a number no variant
    // declares is error.execution.
    @Test
    fun staticEnumNumberReadsAnEnumAsItsNumberAndBack() {
        val sm = StaticEnumNumberStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_enum_number"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // docs/adr/0048: a condition, a transform, two lookups, an interpolation and a
    // validator called as `Alias(args)`; a lookup whose miss is an error fails into
    // error.execution.
    @Test
    fun staticForgeCallsCallsTheKindsThatAreFunctions() {
        val sm = StaticForgeCallsStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_forge_calls"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `eventexpr` of a <send> is a string computed from the machine's fields
    // when the send runs, and names the event the send delivers.
    @Test
    fun staticSendEventIsNamedWhenTheSendRuns() {
        val sm = StaticSendEventStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_send_event"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `targetexpr` of a <send> is a string computed from the machine's fields
    // when the send runs, held to the routes the document declares as `sce:targets`
    // (docs/adr/0005, decision 3): the send goes by the entry that matches, and a
    // value in none of them is error.communication with nothing sent.
    @Test
    fun staticSendTargetIsChosenAmongTheDeclaredRoutes() {
        val sm = StaticSendTargetStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_send_target"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `typeexpr` of a <send> is a string computed from the machine's fields
    // when the send runs, held to the processors the document declares as
    // `sce:types` (docs/adr/0005, decision 3): the send is delivered by the processor
    // the matching entry names, and a value in none of them is error.execution with
    // nothing sent.
    @Test
    fun staticSendTypeIsChosenAmongTheDeclaredProcessors() {
        val sm = StaticSendTypeStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_send_type"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `delayexpr` of a <send> is a string computed from the machine's fields
    // when the send runs, and read as the CSS2 time it must be; the machine runs
    // on a manual clock, which the scenario's `advance_ms` steps move on.
    @Test
    fun staticSendDelayIsComputedWhenTheSendRuns() {
        val sm = StaticSendDelayStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            replay(
                scenario("static_send_delay"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                advance = { ms -> sm.advanceTimeMs(ms) },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `sendidexpr` of a <cancel> is a string computed from the machine's
    // fields when the cancel runs, the id of the delayed send it removes; the
    // machine runs on a manual clock, which the scenario's `advance_ms` steps
    // move on.
    @Test
    fun staticCancelRemovesTheSendItsIdNames() {
        val sm = StaticCancelExprStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            replay(
                scenario("static_cancel_expr"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                advance = { ms -> sm.advanceTimeMs(ms) },
            )
        } finally {
            sm.cleanup()
        }
    }

    // Four delayed sends armed on entering a state are delivered when each is
    // due, two due the same moment in the order they were sent, and the last takes
    // the machine to its final state; the machine runs on a manual clock, which
    // the scenario's `advance_ms` steps move on.
    @Test
    fun staticTimersDeliverEachSendWhenItIsDue() {
        val sm = StaticTimersStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            replay(
                scenario("static_timers"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                advance = { ms -> sm.advanceTimeMs(ms) },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The same machine stopped: a <cancel> by the id of the longest send removes
    // that one and no other, so the machine never ends.
    @Test
    fun staticTimersStopCancelsTheSendItsIdNamesAndNoOther() {
        val sm = StaticTimersStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            replay(
                scenario("static_timers_stop"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                advance = { ms -> sm.advanceTimeMs(ms) },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `idlocation` of a <send> names a string variable the machine writes the
    // id it generates for the send to, which a later <cancel sendidexpr> names;
    // the machine runs on a manual clock, which the scenario's `advance_ms` steps
    // move on.
    @Test
    fun staticSendHandsTheDocumentAnIdACancelCanName() {
        val sm = StaticSendIdlocationStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            replay(
                scenario("static_send_idlocation"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                advance = { ms -> sm.advanceTimeMs(ms) },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `<content expr>` of a <send> names a record, which crosses as the pairs
    // of its fields: a record variable and the payload of the event the
    // transition is on, taken whole.
    @Test
    fun staticSendContentCarriesTheRecordItNames() {
        val sm = StaticSendContentStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_send_content"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The `namelist` of a <send> names variables the machine holds, each carried
    // as the pair `<param name="x" expr="x"/>` it abbreviates, an enum value
    // among them as the name its enum declares.
    @Test
    fun staticSendNamelistCarriesTheVariablesItNames() {
        val sm = StaticSendNamelistStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_send_namelist"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A <send> hands its event the pairs of its <param>s, read from the machine's
    // fields when it runs; the pair whose value does not fit is left out, the
    // message still goes, and the receiver refuses it for the field it finds
    // missing.
    @Test
    fun staticSendParamsCrossAsTheTypedValuesOfTheMachine() {
        val sm = StaticSendParamsStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_send_params"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A statechart hands the algorithms the records and lists it holds, and the
    // event's own payload, and takes back what they return, written only when the
    // call succeeded and held to the bound of the variable it is written to.
    @Test
    fun staticStateAlgorithmHandsStateToAnAlgorithmAndTakesItBack() {
        val sm = StaticStateAlgorithmStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_state_algorithm"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A statechart hands the standard text algorithms the record and the list of
    // records it holds, whose text is bounded, and takes back what they return: a
    // text past the bound is refused where the payload is taken, the state as it
    // was.
    @Test
    fun staticTextRecordHandsBoundedTextToAnAlgorithmAndTakesItBack() {
        val sm = StaticTextRecordStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_text_record"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A statechart hands the algorithms records whose field is a value of an enum,
    // and a value of the enum itself, and takes back a list of those records and a
    // value of the enum: each crosses by the name its variant is declared with.
    @Test
    fun staticStateAlgorithmEnumHandsAnEnumToAnAlgorithmAndTakesItBack() {
        val sm = StaticStateAlgorithmEnumStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_state_algorithm_enum"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A string variable is held to the UTF-8 bytes it declares, not to the
    // UTF-16 units this platform's strings are made of: an assignment past the
    // bound writes nothing, raises error.execution and ends its block.
    @Test
    fun staticStringCapacityHoldsAStringToItsBytes() {
        val sm = StaticStringCapacityStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_string_capacity"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A bytes variable is held to the bytes it declares, as a string is to its
    // UTF-8 bytes: an assignment past the bound writes nothing, raises
    // error.execution and ends its block. It is saved as its byte-exact Latin-1
    // text, which is what the scenario reads it back as.
    @Test
    fun staticBytesHoldsAByteStringToItsBound() {
        val sm = StaticBytesStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_bytes"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A record's bytes field is held to the bytes its schema declares: an assignment
    // past the bound — from a literal or a bytes variable — writes nothing, raises
    // error.execution and ends its block, and a list of such records holds copies
    // with their bytes. It is saved as its byte-exact Latin-1 text.
    @Test
    fun staticRecordBytesHoldsABytesFieldWithinTheBoundItsSchemaDeclares() {
        val sm = StaticRecordBytesStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_bytes"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The bytes an event's payload carries are read into a bytes variable, into a
    // record's field and into a whole record, each held to its own bound: a value
    // past it writes nothing, raises error.execution and ends its block. The wire
    // spells a byte string as its byte-exact Latin-1 text.
    @Test
    fun staticPayloadBytesReadsTheBytesAnEventCarriesWithinTheirBounds() {
        val sm = StaticPayloadBytesStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_payload_bytes"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // A record's string field is held to the UTF-8 bytes its schema declares: an
    // assignment past the bound — from a literal, a string variable or a payload —
    // writes nothing, raises error.execution and ends its block, and a list of
    // such records holds copies with their text.
    @Test
    fun staticRecordStringHoldsAStringFieldWithinTheBoundItsSchemaDeclares() {
        val sm = StaticRecordStringStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_record_string"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // An event's payload carries an enum field, the variant's declared name: a
    // guard compares it to a variant and an assignment stores it in a variable of
    // the enum.
    @Test
    fun staticPayloadEnumReadsAVariantTheEventNames() {
        val sm = StaticPayloadEnumStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_payload_enum"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The payload of the event a transition is on is carried on as the <param>s
    // of a <send>: an enum field as the name its enum declares and an integer,
    // read where the send runs.
    @Test
    fun staticPayloadRelayCarriesOnWhatItsEventCarried() {
        val sm = StaticPayloadRelayStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_payload_relay"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // One copy of a shared to-do list, as each of the three that took part in one
    // run of it was driven: a person's copy stamps a change with its clock and
    // sends it to the hub, the hub passes a write on to every copy but the one it
    // came from, and each field of each to-do keeps the later write whatever order
    // the writes arrive in. Every step states what the copy sent its peers.
    @Test
    fun todoPeerIsAlicesCopyOfASharedList() {
        replayTodoPeer(scenario("todo_peer_alice"))
    }

    @Test
    fun todoPeerIsBobsCopyOfASharedList() {
        replayTodoPeer(scenario("todo_peer_bob"))
    }

    @Test
    fun todoPeerIsTheHubTheCopiesOfASharedListSendTo() {
        replayTodoPeer(scenario("todo_peer_server"))
    }

    // A child session that talks to a Mesh peer is given a router of its own by the
    // factory the driver registers as the machine's host (SCE_MESH.md, Child
    // Sessions in Mesh): what it sends is recorded under its invoke and start, and a
    // step's `invoke` reaches it through that router, a grandchild's through the
    // router its parent's router answered for it.
    @Test
    fun staticChildMeshGivesEachChildARouterOfItsOwn() {
        val sends = RecordedSends()
        val children = ChildRouters(sends)
        val sm = StaticChildMeshStateMachine()
        sm.registerMeshRouter(sends.router)
        sm.registerChildMeshRouters(children.factory)
        sm.initialize()
        try {
            replay(
                scenario("static_child_mesh"),
                send = { event, data -> sm.sendEventByName(event, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                sent = { sends.take() },
                children = children,
            )
        } finally {
            sm.cleanup()
        }
    }

    /** One run of `todo_peer`, on a machine of its own whose sends to its peers are recorded. */
    private fun replayTodoPeer(scenario: JsonObject) {
        val sends = RecordedSends()
        val sm = TodoPeerStateMachine()
        sm.registerMeshRouter(sends.router)
        sm.initialize()
        try {
            replay(
                scenario,
                send = { event, data -> sm.sendEventByName(event, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                sent = { sends.take() },
            )
        } finally {
            sm.cleanup()
        }
    }

    // The standard shared record list (sce:std/shared/record_list.scxml), its scenarios'
    // machine named by its library name: a person's copy, which sends its changes to the
    // hub by `#hub`, and the hub, which passes every change on to the people by `#people`.
    @Test
    fun recordListIsAPersonsCopyOfASharedList() {
        replayRecordList(scenario("record_list_person"))
    }

    @Test
    fun recordListIsTheHubItsPeopleSendTo() {
        replayRecordList(scenario("record_list_hub"))
    }

    // A standard record list a host started has no parent: it tells no one of the
    // registers its changes move, and nothing fails.
    @Test
    fun recordListStartedByAHostTellsNoParent() {
        replayRecordList(scenario("record_list_alone"))
    }

    // A copy given no Mesh router keeps every change it makes: what it does to itself is
    // queued before a send to a peer is refused where it is made (docs/adr/0046).
    @Test
    fun recordListHoldsItsChangesWithNoRouter() {
        replayRecordList(scenario("record_list_offline"))
    }

    // A change is one unit: staged, stamped once, taken in whole, and sent as parts a
    // copy holds until they are all there (docs/adr/0046).
    @Test
    fun recordListTakesAChangeAsOneUnit() {
        replayRecordList(scenario("record_list_change"))
    }

    // A field is emptied and a row retired by writing over what the copy has seen, and a
    // write it had not seen stays (docs/adr/0046).
    @Test
    fun recordListClearsAFieldAndRetiresARow() {
        replayRecordList(scenario("record_list_retire"))
    }

    /**
     * One run of `record_list`, on a machine of its own whose sends to its peers are
     * recorded, or, for a scenario that gives it no router, that is given none.
     */
    private fun replayRecordList(scenario: JsonObject) {
        val sends = RecordedSends()
        val sm = RecordListStateMachine()
        val routed = routed(scenario)
        if (routed) {
            sm.registerMeshRouter(sends.router)
        }
        val sent: (() -> List<JsonElement>)? = if (routed) {
            { sends.take() }
        } else {
            null
        }
        sm.initialize()
        try {
            replay(
                scenario,
                send = { event, data -> sm.sendEventByName(event, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                sent = sent,
            )
        } finally {
            sm.cleanup()
        }
    }

    // A statechart that invokes the standard record list by its library name is told
    // what each register a change moved holds: its count and its values in the order of
    // their dots, a count of 0 when every value was written over, and nothing for a
    // write the list already held. The machine needs `tick`: the list takes a write in
    // one round and what it tells reaches the parent in the next.
    @Test
    fun recordListParentIsToldWhatEachRegisterHolds() {
        val sm = RecordListParentStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("record_list_parent"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { repeat(SETTLE_TICKS) { sm.tick() } },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }

    // One copy of the list of who is online, as each of the three that took part in
    // one afternoon of it was driven: a client says it is here to the hub, the hub
    // keeps what it was told for a ttl and tells every copy on one topic, and an
    // entry is let go by the machine's own delayed send. The machine runs on a
    // manual clock, which the scenario's `advance_ms` steps move on, and every step
    // states what the copy sent its peers.
    @Test
    fun presencePeerIsAlicesCopyOfWhoIsOnline() {
        replayPresencePeer(scenario("presence_peer_alice"))
    }

    @Test
    fun presencePeerIsBobsCopyOfWhoIsOnline() {
        replayPresencePeer(scenario("presence_peer_bob"))
    }

    @Test
    fun presencePeerIsTheHubTheCopiesOfWhoIsOnlineSendTo() {
        replayPresencePeer(scenario("presence_peer_server"))
    }

    /** One run of `presence_peer`, on a manual clock, with its sends to its peers recorded. */
    private fun replayPresencePeer(scenario: JsonObject) {
        val sends = RecordedSends()
        val sm = PresencePeerStateMachine()
        sm.clock = ManualClock(0)
        sm.registerMeshRouter(sends.router)
        sm.initialize()
        try {
            replay(
                scenario,
                send = { event, data -> sm.sendEventByName(event, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                advance = { ms -> sm.advanceTimeMs(ms) },
                sent = { sends.take() },
            )
        } finally {
            sm.cleanup()
        }
    }

    // One copy of a shared sheet, as each of the three that took part in one run
    // of it was driven: its cells merge as the to-do list's fields do, and its
    // rows and columns are sequences every copy orders alike, with what was
    // removed kept apart from what was placed. Every step states what the copy
    // sent its peers.
    @Test
    fun sheetPeerIsAlicesCopyOfASharedSheet() {
        replaySheetPeer(scenario("sheet_peer_alice"))
    }

    @Test
    fun sheetPeerIsBobsCopyOfASharedSheet() {
        replaySheetPeer(scenario("sheet_peer_bob"))
    }

    @Test
    fun sheetPeerIsTheHubTheCopiesOfASharedSheetSendTo() {
        replaySheetPeer(scenario("sheet_peer_server"))
    }

    /** One run of `sheet_peer`, on a machine of its own whose sends to its peers are recorded. */
    private fun replaySheetPeer(scenario: JsonObject) {
        val sends = RecordedSends()
        val sm = SheetPeerStateMachine()
        sm.registerMeshRouter(sends.router)
        sm.initialize()
        try {
            replay(
                scenario,
                send = { event, data -> sm.sendEventByName(event, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
                sent = { sends.take() },
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun staticPayloadReadsTheFieldsOfItsEvent() {
        val sm = StaticPayloadStateMachine()
        sm.initialize()
        try {
            replay(
                scenario("static_payload"),
                send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
                tick = { sm.tick() },
                save = { sm.save() },
                ended = { sm.isInFinalState },
            )
        } finally {
            sm.cleanup()
        }
    }
}
