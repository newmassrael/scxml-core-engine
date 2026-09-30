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
// An event name the machine does not declare is refused by the Rust twin,
// which reads the machine's own lookup; here that lookup is not public, and
// a step whose event were dropped would fail on the state it expects.

package com.sce.integration

import com.sce.integration.static_counter.StaticCounterStateMachine
import com.sce.integration.static_overflow.StaticOverflowStateMachine
import com.sce.integration.static_payload.StaticPayloadStateMachine
import com.sce.integration.sync_client.SyncClientStateMachine
import com.sce.runtime.EventMetadata
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
    ) {
        val steps = scenario.getValue("steps").jsonArray
        assertTrue(steps.isNotEmpty(), "a scenario with no steps judges nothing")
        steps.forEachIndexed { n, element ->
            val step = element.jsonObject
            val note = step["note"]?.jsonPrimitive?.content ?: ""
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
