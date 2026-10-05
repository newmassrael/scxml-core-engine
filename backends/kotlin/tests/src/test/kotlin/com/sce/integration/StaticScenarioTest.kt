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
// An event name no event of the machine matches is refused by the Rust and Go
// twins, which ask the machine's own lookup unless the step says
// `"dropped": true`; here that lookup is not public, and a step whose event
// were dropped would fail on the state it expects — or, for a step that expects
// the drop, on the variables it says did not move.

package com.sce.integration

import com.sce.integration.static_block_ends.StaticBlockEndsStateMachine
import com.sce.integration.static_block_ends_list.StaticBlockEndsListStateMachine
import com.sce.integration.static_cancel_expr.StaticCancelExprStateMachine
import com.sce.integration.static_counter.StaticCounterStateMachine
import com.sce.integration.static_donedata.StaticDonedataStateMachine
import com.sce.integration.static_donedata_content.StaticDonedataContentStateMachine
import com.sce.integration.static_donedata_record.StaticDonedataRecordStateMachine
import com.sce.integration.static_enum.StaticEnumStateMachine
import com.sce.integration.static_event_arrival.StaticEventArrivalStateMachine
import com.sce.integration.static_event_wildcard.StaticEventWildcardStateMachine
import com.sce.integration.static_foreach.StaticForeachStateMachine
import com.sce.integration.static_list.StaticListStateMachine
import com.sce.integration.static_overflow.StaticOverflowStateMachine
import com.sce.integration.static_payload.StaticPayloadStateMachine
import com.sce.integration.static_payload_enum.StaticPayloadEnumStateMachine
import com.sce.integration.static_payload_relay.StaticPayloadRelayStateMachine
import com.sce.integration.static_real.StaticRealStateMachine
import com.sce.integration.static_record.StaticRecordStateMachine
import com.sce.integration.static_record_fields.StaticRecordFieldsStateMachine
import com.sce.integration.static_record_enum.StaticRecordEnumStateMachine
import com.sce.integration.static_record_list.StaticRecordListStateMachine
import com.sce.integration.static_record_real.StaticRecordRealStateMachine
import com.sce.integration.static_send_content.StaticSendContentStateMachine
import com.sce.integration.static_send_delay.StaticSendDelayStateMachine
import com.sce.integration.static_send_event.StaticSendEventStateMachine
import com.sce.integration.static_send_idlocation.StaticSendIdlocationStateMachine
import com.sce.integration.static_send_namelist.StaticSendNamelistStateMachine
import com.sce.integration.static_send_params.StaticSendParamsStateMachine
import com.sce.integration.static_string_capacity.StaticStringCapacityStateMachine
import com.sce.integration.static_whole_payload.StaticWholePayloadStateMachine
import com.sce.integration.static_wire_enum.StaticWireEnumStateMachine
import com.sce.integration.sync_client.SyncClientStateMachine
import com.sce.runtime.EventMetadata
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import java.io.File
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("StaticScenario — sce-static machines replay the scenarios every backend replays (Kotlin AOT)")
class StaticScenarioTest {

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .first { File(it, "sce-build").isDirectory }

    private fun scenario(machine: String): JsonObject =
        Json.parseToJsonElement(
            File(repoRoot(), "sce-build/tests/fixtures/static_datamodel/scenarios/$machine.json").readText(),
        ).jsonObject

    /**
     * Whether a saved value is the scenario's. The saved state writes a 64-bit
     * integer as its decimal text, so an expected number matches that text of
     * the same integer; everything else must be equal as written.
     */
    private fun holds(got: JsonElement?, want: JsonElement): Boolean {
        if (got is JsonPrimitive && want is JsonPrimitive && got.isString && !want.isString) {
            return want.content.toLongOrNull() != null && got.content == want.content
        }
        return got == want
    }

    /**
     * Replay [scenario] against a machine driven by [send] and [tick], read
     * back with [save] after every step, and asked whether it has [ended].
     */
    private fun replay(
        scenario: JsonObject,
        send: (String, String) -> Unit,
        tick: () -> Unit,
        save: () -> SavedState,
        ended: () -> Boolean,
        donedata: () -> String = { "" },
        advance: (Long) -> Unit = { error("this machine runs on no manual clock to advance") },
    ) {
        val steps = scenario.getValue("steps").jsonArray
        assertTrue(steps.isNotEmpty(), "a scenario with no steps judges nothing")
        steps.forEachIndexed { n, element ->
            val step = element.jsonObject
            val note = step["note"]?.jsonPrimitive?.content ?: ""
            // A step that moves the machine's time on, for a scenario of a delayed
            // send: the machine runs on a manual clock, so a wait is the one the
            // step names and not the one the test happened to take.
            step["advance_ms"]?.jsonPrimitive?.content?.let { ms -> advance(ms.toLong()) }
            step["event"]?.jsonPrimitive?.content?.let { event ->
                send(event, step["data"]?.toString() ?: "")
                tick()
            }
            val expect = step.getValue("expect").jsonObject
            // A machine that ended in a top-level <final> has no saved state to
            // read — the save refuses one — so what a scenario can say of it is
            // that it ended, and that is the whole of the step.
            if (expect["ended"]?.jsonPrimitive?.content == "true") {
                assertTrue(
                    expect["state"] == null && expect["variables"] == null,
                    "step $n ($note): an ended machine has no state or variables to read",
                )
                assertTrue(ended(), "step $n ($note): the machine ended in a top-level <final>")
                // The data its <donedata> left for the invoking parent, as the
                // JSON the done event carries; compared as a value, so the order
                // the pairs were written in is not part of the answer.
                expect["donedata"]?.let { want ->
                    assertEquals(
                        want,
                        Json.parseToJsonElement(donedata()),
                        "step $n ($note): the data the final's done event carries",
                    )
                }
                return@forEachIndexed
            }
            val saved = Json.parseToJsonElement(save().toJson()).jsonObject
            expect["state"]?.let { state ->
                assertEquals(state, saved["current"], "step $n ($note): the current state")
            }
            expect["variables"]?.jsonObject?.forEach { (name, want) ->
                val got = saved.getValue("variables").jsonObject[name]
                assertTrue(holds(got, want), "step $n ($note): variable `$name` is $got, not $want")
            }
        }
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

    // The payload of an event is a record of its schema taken whole: it replaces a
    // record variable in one assignment and is appended whole to a list.
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
