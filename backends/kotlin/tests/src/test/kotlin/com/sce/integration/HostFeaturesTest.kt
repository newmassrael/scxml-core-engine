// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The standard host features (docs/SCE_HOST_FEATURES.md) as a generated Kotlin
// machine sees them through the reference host: every scenario of
// `tests/host_features/scenarios` replayed against the machine it names, with
// `com.sce.hostfeatures.ReferenceHost` registered as the invoker of every
// standard type.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` and `expect.host_state` are what the host was asked and
// what it holds afterwards. The expected values are derived from the
// specification, not observed from the host.

package com.sce.integration

import com.sce.hostfeatures.ExhaustiveProvider
import com.sce.hostfeatures.HttpHeader
import com.sce.hostfeatures.HttpRequest
import com.sce.hostfeatures.HttpResponse
import com.sce.hostfeatures.OutcomeClass
import com.sce.hostfeatures.Reading
import com.sce.hostfeatures.ReferenceHost
import com.sce.hostfeatures.ScriptedAnswer
import com.sce.hostfeatures.ScriptedClock
import com.sce.hostfeatures.ScriptedProvider
import com.sce.hostfeatures.Transport
import com.sce.hostfeatures.TransportFailure
import com.sce.hostfeatures.Zone
import com.sce.hostfeatures.serve
import com.sce.integration.static_datamodel.static_host_clock.StaticHostClockStateMachine
import com.sce.integration.static_datamodel.static_host_http.StaticHostHttpStateMachine
import com.sce.integration.static_datamodel.static_host_kv.StaticHostKvStateMachine
import com.sce.integration.static_datamodel.static_host_model.StaticHostModelAnswerSchemas
import com.sce.integration.static_datamodel.static_host_model.StaticHostModelStateMachine
import com.sce.integration.static_datamodel.static_host_notify.StaticHostNotifyStateMachine
import com.sce.integration.static_datamodel.static_model_harness.StaticModelHarnessAnswerSchemas
import com.sce.integration.static_datamodel.static_model_harness.StaticModelHarnessState
import com.sce.integration.static_datamodel.static_model_harness.StaticModelHarnessStateMachine
import com.sce.integration.static_datamodel.static_model_harness.StaticModelHarnessTransitions
import com.sce.runtime.AnswerSchema
import com.sce.runtime.Event
import com.sce.runtime.EventMetadata
import com.sce.runtime.ManualClock
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.TransitionTaken
import java.io.File
import java.util.concurrent.TimeUnit
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import kotlinx.serialization.json.put
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Timeout

class HostFeaturesTest {

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .first { File(it, "sce-build").isDirectory }

    private fun scenario(machine: String): JsonObject =
        Json.parseToJsonElement(
            File(repoRoot(), "tests/host_features/scenarios/$machine.json").readText(),
        ).jsonObject

    /** Answers what a scenario says and keeps what it was given. */
    private class ScriptTransport : Transport {
        var set = false
        var hold = false
        var fail = false
        var response: HttpResponse? = null
        val log = mutableListOf<HttpRequest>()

        override fun exchange(request: HttpRequest): HttpResponse? {
            log.add(request)
            if (hold) return null
            if (fail || !set) throw TransportFailure("the transport has no response")
            return response
        }

        fun directive(d: JsonObject) {
            set = true
            hold = d["hold"]?.jsonPrimitive?.boolean == true
            fail = d.containsKey("fail")
            response = null
            if (hold || fail) return
            val body: ByteArray = when {
                d.containsKey("body") -> d.getValue("body").jsonPrimitive.content.encodeToByteArray()
                d.containsKey("body_hex") -> {
                    val hex = d.getValue("body_hex").jsonPrimitive.content
                    ByteArray(hex.length / 2) { hex.substring(2 * it, 2 * it + 2).toInt(16).toByte() }
                }
                d.containsKey("body_repeat") -> {
                    val repeat = d.getValue("body_repeat").jsonArray
                    repeat[0].jsonPrimitive.content.repeat(repeat[1].jsonPrimitive.int).encodeToByteArray()
                }
                else -> ByteArray(0)
            }
            val headers = d.getValue("headers").jsonArray.map {
                HttpHeader(it.jsonArray[0].jsonPrimitive.content, it.jsonArray[1].jsonPrimitive.content)
            }
            response = HttpResponse(d.getValue("status").jsonPrimitive.int, headers, body)
        }
    }

    private class Rig {
        val clock = ScriptedClock()
        val transport = ScriptTransport()
        val host = ReferenceHost(clock, transport)
        val model = ScriptedProvider().also { host.setModelProvider(it) }

        /** Apply a `model` directive to the scripted provider: each member it names. */
        fun scriptModel(d: JsonObject) {
            d["answers"]?.jsonArray?.let { answers ->
                model.answers.clear()
                for (raw in answers) {
                    val answer = raw.jsonObject
                    val text = answer["text"]?.jsonPrimitive?.content ?: answer.getValue("text_repeat").jsonArray.let {
                        it[0].jsonPrimitive.content.repeat(it[1].jsonPrimitive.int)
                    }
                    model.answers.addLast(ScriptedAnswer(text, answer["finish"]?.jsonPrimitive?.content ?: "stop"))
                }
            }
            d["constrained"]?.let { model.constrained = it.jsonPrimitive.boolean }
            d["name"]?.let { model.name = it.jsonPrimitive.content }
            d["context"]?.let { model.context = it.jsonPrimitive.long }
            d["hold"]?.let { model.hold = it.jsonPrimitive.boolean }
        }

        fun directives(directives: JsonObject) {
            for ((name, value) in directives) {
                when (name) {
                    "clock" -> {
                        val c = value.jsonObject
                        val reading = clock.reading!!
                        val zone = clock.zoneNow!!
                        clock.reading = Reading(
                            c["utc_ms"]?.jsonPrimitive?.long ?: reading.utcMs,
                            c["monotonic_ms"]?.jsonPrimitive?.long ?: reading.monotonicMs,
                        )
                        clock.zoneNow = Zone(
                            c["zone_id"]?.jsonPrimitive?.content ?: zone.id,
                            c["utc_offset_s"]?.jsonPrimitive?.int ?: zone.utcOffsetS,
                        )
                    }
                    "permission" -> host.setPermission(value.jsonPrimitive.content)
                    "user_decision" -> host.setUserDecision(value.jsonPrimitive.content)
                    "quota" -> host.setQuota(value.jsonPrimitive.int)
                    "fail" -> host.setFaults(value.jsonObject.mapValues { it.value.jsonPrimitive.content })
                    "http" -> transport.directive(value.jsonObject)
                    "model" -> scriptModel(value.jsonObject)
                    else -> error("`$name` is not a host directive")
                }
            }
        }

        /**
         * What the host was started with since the last step, as a scenario writes it:
         * the request as its record, the integer fields as numbers.
         */
        fun starts(): JsonArray =
            JsonArray(
                host.takeStarts().map { start ->
                    buildJsonObject {
                        put("type", start.operation)
                        put("id", start.id)
                        put(
                            "request",
                            buildJsonObject {
                                for ((name, text) in start.params) {
                                    if (name in INTEGER_FIELDS) put(name, text.toLong()) else put(name, text)
                                }
                            },
                        )
                    }
                },
            )

        fun hostState(wanted: JsonObject): JsonObject =
            buildJsonObject {
                for (name in wanted.keys) {
                    when (name) {
                        "store" -> put(
                            name,
                            buildJsonObject { host.storeEntries().forEach { (key, value) -> put(key, value) } },
                        )
                        "waiting" -> put(name, JsonArray(host.waiting().map { JsonPrimitive(it) }))
                        "shown" -> put(name, JsonArray(host.shown().map { JsonPrimitive(it) }))
                        "cancelled" -> put(name, JsonArray(host.cancelled().map { JsonPrimitive(it) }))
                        "http" -> put(
                            name,
                            JsonArray(
                                transport.log.map { r ->
                                    buildJsonObject {
                                        put("method", r.method)
                                        put("url", r.url)
                                        put(
                                            "headers",
                                            JsonArray(
                                                r.headers.map { h ->
                                                    buildJsonObject {
                                                        put("name", h.name)
                                                        put("value", h.value)
                                                    }
                                                },
                                            ),
                                        )
                                        put("body", r.body)
                                    }
                                },
                            ),
                        )
                        else -> error("`$name` is not host state")
                    }
                }
            }
    }

    /** Whether a saved value is the scenario's: a 64-bit integer is saved as its digits. */
    private fun holds(got: JsonElement?, want: JsonElement): Boolean {
        if (got is JsonPrimitive && want is JsonPrimitive && got.isString && !want.isString) {
            return want.content.toLongOrNull() != null && got.content == want.content
        }
        return got == want
    }

    private fun <S : State, E : Event> replay(
        machine: String,
        sm: StateMachineEngine<S, E>,
        schemas: List<AnswerSchema> = emptyList(),
        save: () -> String,
    ) = replayScenario(machine, machine, sm, schemas, save)

    /**
     * Replay the scenario [stem] against [machine], the machine the scenario names: a
     * machine whose run ends has one scenario a run (the reference harness's cases).
     */
    private fun <S : State, E : Event> replayScenario(
        stem: String,
        machine: String,
        sm: StateMachineEngine<S, E>,
        schemas: List<AnswerSchema>,
        save: () -> String,
    ) {
        val scenario = scenario(stem)
        assertEquals(machine, scenario.getValue("machine").jsonPrimitive.content, "the scenario $stem names its machine")
        val rig = Rig()
        rig.host.setAnswerSchemas(schemas)
        serve(sm, rig.host)
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            val steps = scenario.getValue("steps").jsonArray
            assertTrue(steps.isNotEmpty(), "a scenario with no steps judges nothing")
            sm.tick()
            steps.forEachIndexed { n, element ->
                val step = element.jsonObject
                val note = stem + ": " + (step["note"]?.jsonPrimitive?.content ?: "")
                step["host"]?.let { rig.directives(it.jsonObject) }
                step["advance_ms"]?.let {
                    sm.advanceTimeMs(it.jsonPrimitive.long)
                    sm.tick()
                }
                step["event"]?.jsonPrimitive?.content?.let { event ->
                    sm.sendEventByName(event, EventMetadata(data = step["data"]?.toString() ?: ""))
                    sm.tick()
                }
                val expect = step.getValue("expect").jsonObject
                val started = rig.starts()
                expect["host_invokes"]?.let { want ->
                    assertEquals(want, started, "step $n ($note): what the host was asked")
                }
                expect["host_state"]?.let { want ->
                    assertEquals(want, rig.hostState(want.jsonObject), "step $n ($note): what the host holds")
                }
                val saved = Json.parseToJsonElement(save()).jsonObject
                expect["state"]?.let { state ->
                    assertEquals(state, saved["current"], "step $n ($note): the current state")
                }
                expect["variables"]?.jsonObject?.forEach { (name, want) ->
                    val got = saved.getValue("variables").jsonObject[name]
                    assertTrue(holds(got, want), "step $n ($note): variable `$name` is $got, not $want")
                }
            }
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aMachineAsksTheKeyValueStoreOfItsHost() {
        val sm = StaticHostKvStateMachine()
        replay("static_host_kv", sm) { sm.save().toJson() }
    }

    @Test
    fun aMachineReadsTheClockOfItsHost() {
        val sm = StaticHostClockStateMachine()
        replay("static_host_clock", sm) { sm.save().toJson() }
    }

    @Test
    fun aMachinePostsNotificationsThroughItsHost() {
        val sm = StaticHostNotifyStateMachine()
        replay("static_host_notify", sm) { sm.save().toJson() }
    }

    @Test
    fun aMachineMakesAnHttpExchangeThroughItsHost() {
        val sm = StaticHostHttpStateMachine()
        replay("static_host_http", sm) { sm.save().toJson() }
    }

    @Test
    fun aMachineAsksItsHostForATextUnderAnAnswerSchema() {
        val sm = StaticHostModelStateMachine()
        replay("static_host_model", sm, StaticHostModelAnswerSchemas.all) { sm.save().toJson() }
    }

    @Test
    fun theReferenceHarnessRunsEachCaseOfItsScenario() {
        for (stem in HARNESS_SCENARIOS) {
            val sm = StaticModelHarnessStateMachine()
            replayScenario(stem, "static_model_harness", sm, StaticModelHarnessAnswerSchemas.all) { sm.save().toJson() }
        }
    }

    /** What the exhaustive provider answers for a class of the table. */
    private fun outcomeClass(cls: JsonObject): OutcomeClass =
        when (val end = cls.getValue("end").jsonPrimitive.content) {
            "answer" -> OutcomeClass.Answer(
                cls.getValue("text").jsonPrimitive.content,
                cls.getValue("finish").jsonPrimitive.content,
            )
            "failure" -> OutcomeClass.Failure(cls.getValue("error").jsonPrimitive.content)
            "refusal" -> OutcomeClass.Refusal
            "hold" -> OutcomeClass.Hold
            else -> error("`$end` is not an end")
        }

    /**
     * Run static_host_model against the exhaustive provider answering the classes
     * [sequence] names, one call a class, and hold what the machine publishes after each
     * call to what its class says.
     */
    private fun runSequence(table: JsonObject, sequence: List<Int>) {
        val classes = table.getValue("classes").jsonArray.map { it.jsonObject }
        val named = sequence.map { classes[it].getValue("class").jsonPrimitive.content }
        val sm = StaticHostModelStateMachine()
        val host = ReferenceHost(ScriptedClock(), ScriptTransport())
        host.setAnswerSchemas(StaticHostModelAnswerSchemas.all)
        host.setModelProvider(ExhaustiveProvider(sequence.map { outcomeClass(classes[it]) }))
        serve(sm, host)
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            sm.tick()
            val ask = table.getValue("ask").jsonObject
            val event = ask.getValue("event").jsonPrimitive.content
            var answer: JsonElement = JsonNull
            sequence.forEachIndexed { n, i ->
                val cls = classes[i]
                sm.sendEventByName(event, EventMetadata(data = ask.getValue("data").toString()))
                sm.tick()
                when (cls["then"]?.jsonPrimitive?.content) {
                    "advance" -> {
                        sm.advanceTimeMs(table.getValue("deadline_ms").jsonPrimitive.long)
                        sm.tick()
                    }
                    "leave" -> {
                        sm.sendEventByName(table.getValue("leave").jsonPrimitive.content, EventMetadata(data = ""))
                        sm.tick()
                    }
                }
                val expect = cls.getValue("expect").jsonObject
                expect["answer"]?.let { answer = it }
                val at = "$named, call ${n + 1}"
                val saved = Json.parseToJsonElement(sm.save().toJson()).jsonObject
                assertEquals(JsonPrimitive("idle"), saved["current"], "$at: the machine is idle")
                val variables = saved.getValue("variables").jsonObject
                for (name in listOf("outcome", "why", "finish")) {
                    assertTrue(holds(variables[name], expect.getValue(name)), "$at: `$name` is ${variables[name]}")
                }
                assertTrue(holds(variables["answer"], answer), "$at: `answer` is ${variables["answer"]}, not $answer")
                assertTrue(holds(variables["calls"], JsonPrimitive(n + 1)), "$at: `calls` is ${variables["calls"]}")
            }
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The machine around a model is tested exhaustively (docs/adr/0087, section 9.4):
     * every sequence of outcome classes up to the table's bound, each call read as its
     * class says, and every refusal code of the library either reached by a class or
     * named as deferred with its reason.
     */
    @Test
    fun everySequenceOfOutcomeClassesIsReadAsItsClassesSay() {
        val table = Json.parseToJsonElement(
            File(repoRoot(), "tests/host_features/model_outcome_classes.json").readText(),
        ).jsonObject
        assertEquals("static_host_model", table.getValue("machine").jsonPrimitive.content)
        assertEquals(
            table.getValue("answer_schema").jsonPrimitive.content,
            StaticHostModelAnswerSchemas.all.first().id,
        )
        val classes = table.getValue("classes").jsonArray
        val bound = table.getValue("bound").jsonPrimitive.int
        assertTrue(classes.isNotEmpty() && bound > 0, "a table that names nothing judges nothing")
        var runs = 0
        var expected = 0
        var sequences = listOf(emptyList<Int>())
        for (length in 1..bound) {
            sequences = sequences.flatMap { prefix -> classes.indices.map { prefix + it } }
            expected += sequences.size
            for (sequence in sequences) {
                runSequence(table, sequence)
                runs++
            }
        }
        var power = 1
        assertEquals((1..bound).sumOf { power *= classes.size; power }, expected)
        assertEquals(expected, runs, "every sequence up to the bound was run")
        val codes = classes.mapNotNull { cls ->
            (cls.jsonObject.getValue("expect").jsonObject["why"] as? JsonObject)?.getValue("code")?.jsonPrimitive?.content
        }.toMutableSet()
        table.getValue("deferred").jsonArray.forEach { codes.add(it.jsonObject.getValue("code").jsonPrimitive.content) }
        // The fourteen codes, tooFew the last (docs/adr/0086, section 15.11).
        assertEquals(14, codes.size, "every refusal code is reached or deferred: $codes")
    }

    /**
     * The end of the host operation a class gives a call (docs/adr/0087, section 9.4): an
     * answer by its `finish`, a failure by its code, the refusal, and a held call,
     * whichever way it is left.
     */
    private fun hostOutcome(cls: JsonObject): String =
        when (val end = cls.getValue("end").jsonPrimitive.content) {
            "answer" -> "answer:" + cls.getValue("finish").jsonPrimitive.content
            "failure" -> cls.getValue("error").jsonPrimitive.content
            "refusal" -> "invalid_argument"
            "hold" -> "hold"
            else -> error("`$end` is not an end")
        }

    /** What one run of the harness against the exhaustive provider showed. */
    private class HarnessRun(
        /** Every row the transition observer was told, in order. */
        val rows: List<TransitionTaken<StaticModelHarnessState>>,
        /** How many calls the host was asked to start. */
        val calls: Int,
        /** The state the run ended in. */
        val ended: String,
        /** The code of each refusal the harness kept, in order. */
        val codes: List<String>,
    )

    private fun settleHarness(sm: StaticModelHarnessStateMachine) {
        repeat(HARNESS_SETTLE_TICKS) { sm.tick() }
    }

    /**
     * Run the reference harness against the exhaustive provider answering the classes
     * [sequence] names, one call a class: offer it the table's ids, start it, and leave
     * each call it holds as the table's harness section says, until it ends.
     */
    private fun runHarness(table: JsonObject, sequence: List<Int>): HarnessRun {
        val classes = table.getValue("classes").jsonArray.map { it.jsonObject }
        val harness = table.getValue("harness").jsonObject
        assertEquals(
            "advance",
            harness.getValue("then").jsonPrimitive.content,
            "a held call of the harness is left by its own deadline",
        )
        val sm = StaticModelHarnessStateMachine()
        val rows = mutableListOf<TransitionTaken<StaticModelHarnessState>>()
        sm.transitionObserver = { taken -> rows.add(taken) }
        val host = ReferenceHost(ScriptedClock(), ScriptTransport())
        host.setAnswerSchemas(StaticModelHarnessAnswerSchemas.all)
        host.setModelProvider(ExhaustiveProvider(sequence.map { outcomeClass(classes[it]) }))
        serve(sm, host)
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            settleHarness(sm)
            val offer = harness.getValue("offer").jsonPrimitive.content
            for (id in harness.getValue("offered").jsonArray) {
                sm.sendEventByName(offer, EventMetadata(data = buildJsonObject { put("id", id) }.toString()))
                settleHarness(sm)
            }
            sm.sendEventByName(
                harness.getValue("start").jsonPrimitive.content,
                EventMetadata(data = harness.getValue("data").toString()),
            )
            settleHarness(sm)
            val bound = harness.getValue("bound").jsonPrimitive.int
            val deadline = harness.getValue("deadline_ms").jsonPrimitive.long
            fun saved(): JsonObject = Json.parseToJsonElement(sm.save().toJson()).jsonObject
            fun current(): String = saved().getValue("current").jsonPrimitive.content
            var waits = 0
            while (current() != "accepted" && current() != "gave_up") {
                assertTrue(waits <= bound, "$sequence: the run neither ended nor held a call it could be moved past")
                sm.advanceTimeMs(deadline)
                settleHarness(sm)
                waits++
            }
            val codes = saved().getValue("variables").jsonObject.getValue("refusals").jsonArray.map {
                it.jsonObject.getValue("code").jsonPrimitive.content
            }
            return HarnessRun(rows.toList(), host.takeStarts().size, current(), codes)
        } finally {
            sm.cleanup()
        }
    }

    private fun rowName(row: TransitionTaken<StaticModelHarnessState>): String = "${row.source}[${row.index}]"

    /**
     * The reference harness is tested exhaustively, and its coverage is measured
     * (docs/adr/0087, section 9.4 item 1, Landing order item 7): every sequence of `bound`
     * outcome classes is a run, the transition observer of docs/adr/0089 tells each row of
     * the emitted transition table the run takes, and the runs together take every row of
     * the table, reach every refusal code, and reach every host outcome at every call site
     * (a row that enters the state whose invoke is the call).
     *
     * The harness makes at most `bound` calls and the provider answers a call by its
     * position alone, so every sequence of fewer classes the harness can consume is a
     * prefix of a sequence run here, and none is asked past.
     *
     * The runs are the classes of the table to the power of `bound`, so the test's cost grows
     * with every class the table gains (24 classes and a bound of 3 are 13,824 runs, a little
     * under a millisecond each) and not with the suite's other tests. The suite's ten-second
     * default is the guard of a test that should finish in milliseconds, and this one is
     * measured against the work it is told to do: a hang still ends it, after two minutes.
     */
    @Test
    @Timeout(value = 2, unit = TimeUnit.MINUTES)
    fun everySequenceOfOutcomeClassesTakesEveryRowOfTheReferenceHarness() {
        val table = Json.parseToJsonElement(
            File(repoRoot(), "tests/host_features/model_outcome_classes.json").readText(),
        ).jsonObject
        val harness = table.getValue("harness").jsonObject
        assertEquals("static_model_harness", harness.getValue("machine").jsonPrimitive.content)
        assertEquals(
            table.getValue("answer_schema").jsonPrimitive.content,
            StaticModelHarnessAnswerSchemas.all.first().id,
        )
        val classes = table.getValue("classes").jsonArray.map { it.jsonObject }
        val bound = harness.getValue("bound").jsonPrimitive.int
        val call = harness.getValue("call").jsonPrimitive.content
        assertTrue(classes.isNotEmpty() && bound > 0, "a table that names nothing judges nothing")
        val rows = StaticModelHarnessTransitions.all.map { TransitionTaken(it.from, it.index) }.toSet()
        val sites = StaticModelHarnessTransitions.all.filter { it.targets == call }
            .map { TransitionTaken(it.from, it.index) }.toSet()
        assertTrue(sites.isNotEmpty(), "no row of the table enters `$call`")
        val outcomes = classes.map { hostOutcome(it) }.toSortedSet()

        val taken = mutableSetOf<TransitionTaken<StaticModelHarnessState>>()
        val codes = mutableSetOf<String>()
        val reached = mutableSetOf<Pair<TransitionTaken<StaticModelHarnessState>, String>>()
        var sequences = listOf(emptyList<Int>())
        repeat(bound) { sequences = sequences.flatMap { prefix -> classes.indices.map { prefix + it } } }
        var runs = 0
        for (sequence in sequences) {
            val named = sequence.map { classes[it].getValue("class").jsonPrimitive.content }
            val run = runHarness(table, sequence)
            assertTrue(
                run.ended == "accepted" || run.ended == "gave_up",
                "$named: the run ended in `${run.ended}`, not a final",
            )
            val called = run.rows.filter { it in sites }
            assertEquals(run.calls, called.size, "$named: each call is made by one row that enters `$call`")
            assertTrue(run.calls <= bound, "$named: ${run.calls} calls, past the bound")
            called.zip(sequence).forEach { (site, cls) -> reached.add(site to hostOutcome(classes[cls])) }
            taken.addAll(run.rows)
            codes.addAll(run.codes)
            runs++
        }
        var expected = 1
        repeat(bound) { expected *= classes.size }
        assertEquals(expected, runs, "every sequence was run")

        val toldNoRow = (taken - rows).map { rowName(it) }.sorted()
        assertTrue(toldNoRow.isEmpty(), "the observer was told pairs that are no row of the table: $toldNoRow")
        val neverTaken = (rows - taken).map { rowName(it) }.sorted()
        assertTrue(
            neverTaken.isEmpty(),
            "every row of the harness's transition table is taken; never taken: $neverTaken",
        )

        val deferred = table.getValue("deferred").jsonArray
            .map { it.jsonObject.getValue("code").jsonPrimitive.content }.toSet()
        val liftCodes = classes.mapNotNull {
            (it.getValue("expect").jsonObject["why"] as? JsonObject)?.getValue("code")?.jsonPrimitive?.content
        }
        val expectedCodes = (liftCodes + APPLICATION_CODES + deferred).toSortedSet()
        assertEquals(15, expectedCodes.size, "the fourteen lift codes and the application's own: $expectedCodes")
        val neverReached = expectedCodes.filter { it !in codes && it !in deferred }
        assertTrue(neverReached.isEmpty(), "every refusal code is reached; never reached: $neverReached")
        val unknown = codes.filter { it !in expectedCodes }.sorted()
        assertTrue(unknown.isEmpty(), "the harness kept codes no class reaches: $unknown")

        val notAtEverySite = sites.flatMap { site ->
            outcomes.filter { (site to it) !in reached }.map { "${rowName(site)} $it" }
        }.sorted()
        assertTrue(
            notAtEverySite.isEmpty(),
            "every host outcome is reached at every call site; never reached: $notAtEverySite",
        )
    }

    @Test
    fun everyScenarioOfTheDirectoryIsReplayedHere() {
        val names = File(repoRoot(), "tests/host_features/scenarios").listFiles()!!
            .filter { it.extension == "json" }.map { it.nameWithoutExtension }.sorted()
        val replayed =
            listOf("static_host_clock", "static_host_http", "static_host_kv", "static_host_model", "static_host_notify") +
                HARNESS_SCENARIOS
        assertEquals(replayed.sorted(), names)
    }

    private companion object {
        /** The request fields a scenario writes as numbers; every other is text. */
        val INTEGER_FIELDS = setOf("at_ms", "max_tokens", "temperature_milli")

        /**
         * The cases of the reference harness's scenario (docs/adr/0087, section 9.1 item 8),
         * one scenario each: a run of the harness ends at one of its finals, so each case is
         * a run of its own.
         */
        val HARNESS_SCENARIOS = listOf(
            "static_model_harness_accepted",
            "static_model_harness_exhausted",
            "static_model_harness_not_offered",
            "static_model_harness_repaired",
            "static_model_harness_timeout",
        )

        /**
         * The codes of the application's own checks, after the fourteen of the lift its enum
         * repeats (enum_harness_code.scxml), each named for the class that reaches it.
         */
        val APPLICATION_CODES = listOf("notOffered")

        /**
         * The ticks a harness run is given after each event: a run may make its calls and
         * take each completion within one event, a call a round.
         */
        const val HARNESS_SETTLE_TICKS = 8
    }
}
