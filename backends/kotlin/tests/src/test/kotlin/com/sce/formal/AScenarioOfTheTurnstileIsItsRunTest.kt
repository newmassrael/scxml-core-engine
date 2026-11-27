// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Each scenario of the properties judge's turnstile, replayed by the Kotlin lane
// (docs/adr/0115, D15 and the converse of D8). A scenario is the run TLC gave of
// a violation or a witness, with the events its environment put on the external
// queue; this test gives the machine those events, on a manual clock, with its
// trace port on, and holds the run the port is told to the recorded run
// `<name>.kotlin.json` beside the scenario, byte for byte.
// `sce_build::formal::replay` judges each recorded run to be the scenario's,
// step for step (sce-build/tests/a_scenario_is_the_run_of_every_lane.rs), as it
// judges the Rust and the C11 lanes'. The twin of the Rust lane's
// `a_scenario_of_the_turnstile_is_its_run` and of the C11 lane's
// `c11_a_scenario_of_the_turnstile_is_its_run`.
//
// The harness gives an input once the machine has taken every step it can,
// which is after the step the input names unless the model's environment put it
// on the queue in the middle of a macrostep. A macrostep that also sends to its
// own external queue after that point would take the two in another order here;
// the judge then finds the runs part, which is a defect of this harness to
// settle, and not a verdict about the document. A delayed send comes due by the
// clock moving to the next deadline, which is the earliest: a scenario in which
// the model's time makes a later one due first is one no clock replays, and the
// judge finds that too.
//
// The machine is the formal fixture's (the kotlin-integration tree's package
// `com.sce.integration.turnstile`); the scenarios are written by
// `sce-codegen formal properties --scenarios
// sce-build/tests/fixtures/formal_properties/scenarios`. Recording the lane's
// runs again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 ./gradlew :sce-kotlin-tests:test --tests 'com.sce.formal.*'
//
// Fixture: sce-build/tests/fixtures/formal_properties/turnstile.scxml

package com.sce.formal

import com.sce.integration.turnstile.TurnstileStateMachine
import com.sce.runtime.RunTrace
import com.sce.w3c.W3CTestBase
import com.sce.w3c.initializeOnVirtualTime
import java.io.File
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.intOrNull
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("Each scenario of the turnstile is its run on the Kotlin lane (docs/adr/0115, D15)")
class AScenarioOfTheTurnstileIsItsRunTest {

    /** Where an event the environment put on the external queue came from. */
    private enum class InputKind {
        /** It came from outside. */
        ARRIVE,

        /** A delayed send the machine made came due. */
        DUE,
    }

    /**
     * One event the environment put on the external queue during a scenario,
     * [after] the count of steps of the run that were over when it came, from 1.
     */
    private data class Input(val kind: InputKind, val after: Int, val event: String)

    /** The directory of the turnstile's scenarios and of each lane's run of them. */
    private val scenarios = File("sce-build/tests/fixtures/formal_properties/scenarios")

    /** Each scenario in [scenarios], by its name, in the order of the names. */
    private fun scenarioNames(): List<String> =
        scenarios.listFiles().orEmpty()
            .map { file -> file.name }
            .filter { name -> name.length > SCENARIO_SUFFIX.length && name.endsWith(SCENARIO_SUFFIX) }
            .map { name -> name.removeSuffix(SCENARIO_SUFFIX) }
            .sorted()

    /**
     * The inputs of the scenario [text], which `sce_build::run_trace` reads of
     * reference: every other member of the document is read there and stepped
     * over here. An input with a member the form does not have, or without one
     * it has, is refused rather than replayed in part.
     */
    private fun inputsOf(name: String, text: String): List<Input> {
        val document = Json.parseToJsonElement(text) as? JsonObject
            ?: throw AssertionError("$name: the scenario is no JSON object")
        val listed = document["inputs"] ?: return emptyList()
        val inputs = listed as? JsonArray ?: throw AssertionError("$name: `inputs` is no array")
        return inputs.mapIndexed { at, element ->
            val input = element as? JsonObject
            val kind = (input?.get("kind") as? JsonPrimitive)?.takeIf { it.isString }?.content
            val after = (input?.get("after") as? JsonPrimitive)?.takeIf { !it.isString }?.intOrNull
            val event = (input?.get("event") as? JsonPrimitive)?.takeIf { it.isString }?.content
            if (input == null || input.size != 3 || kind == null || after == null || event == null) {
                throw AssertionError("$name: inputs[$at] is not an object of a `kind`, an `after` and an `event`")
            }
            val read = when (kind) {
                "arrive" -> InputKind.ARRIVE
                "due" -> InputKind.DUE
                else -> throw AssertionError("$name: inputs[$at] is of the kind '$kind', which the form does not have")
            }
            Input(read, after, event)
        }
    }

    /** The run the trace port is told when the turnstile is given [inputs]. */
    private fun replay(name: String, inputs: List<Input>): RunTrace {
        val trace = RunTrace()
        val machine = TurnstileStateMachine(W3CTestBase.createEngine())
        machine.tracePort = { step -> trace.push(step) }
        try {
            // The manual clock goes in before the start: the entry configuration's
            // `<onentry>` may arm a delayed send.
            machine.initializeOnVirtualTime()
            for (input in inputs) {
                val told = trace.steps.size
                assertTrue(
                    told >= input.after,
                    "$name: the machine took $told steps, and the model ${input.after} before `${input.event}` came"
                )
                when (input.kind) {
                    InputKind.ARRIVE -> {
                        machine.sendEventByName(input.event)
                        machine.tick()
                    }
                    InputKind.DUE -> {
                        val wait = machine.timeUntilNextScheduledMs()
                            ?: throw AssertionError(
                                "$name: `${input.event}` comes due, and the machine holds no delayed send"
                            )
                        machine.advanceTimeMs(wait)
                    }
                }
            }
            return trace
        } finally {
            machine.tracePort = null
            machine.cleanup()
        }
    }

    /**
     * Hold [trace] to the recorded run in [file], or record it again when
     * `UPDATE_EXPECT` is set.
     */
    private fun holdsToTheRecordedRun(file: File, trace: RunTrace) {
        val text = trace.toJson()
        if (System.getenv("UPDATE_EXPECT") != null) {
            file.writeBytes(text.encodeToByteArray())
            return
        }
        assertTrue(
            file.isFile,
            "${file.path} cannot be read: record it with `UPDATE_EXPECT=1 ./gradlew " +
                ":sce-kotlin-tests:test --tests 'com.sce.formal.*'`"
        )
        assertEquals(
            file.readBytes().decodeToString(),
            text,
            "the run of ${file.path} is not the recorded one the replay judge reads"
        )
    }

    @Test
    fun eachScenarioOfTheTurnstileIsItsRunOnTheKotlinLane() {
        val names = scenarioNames()
        assertTrue(
            names.isNotEmpty(),
            "${scenarios.path} holds no scenario: write them with `sce-codegen formal properties --scenarios`"
        )
        for (name in names) {
            val text = File(scenarios, name + SCENARIO_SUFFIX).readText()
            val trace = replay(name, inputsOf(name, text))
            holdsToTheRecordedRun(File(scenarios, "$name.kotlin.json"), trace)
        }
    }

    private companion object {
        /** What a scenario's file is named by after its name (`sce_build::formal::replay::SCENARIO_SUFFIX`). */
        const val SCENARIO_SUFFIX = ".scenario.json"
    }
}
