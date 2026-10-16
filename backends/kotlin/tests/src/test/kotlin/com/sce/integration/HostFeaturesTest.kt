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

import com.sce.hostfeatures.HttpHeader
import com.sce.hostfeatures.HttpRequest
import com.sce.hostfeatures.HttpResponse
import com.sce.hostfeatures.Reading
import com.sce.hostfeatures.ReferenceHost
import com.sce.hostfeatures.ScriptedClock
import com.sce.hostfeatures.Transport
import com.sce.hostfeatures.TransportFailure
import com.sce.hostfeatures.Zone
import com.sce.hostfeatures.serve
import com.sce.integration.static_host_clock.StaticHostClockStateMachine
import com.sce.integration.static_host_http.StaticHostHttpStateMachine
import com.sce.integration.static_host_kv.StaticHostKvStateMachine
import com.sce.integration.static_host_notify.StaticHostNotifyStateMachine
import com.sce.runtime.Event
import com.sce.runtime.EventMetadata
import com.sce.runtime.ManualClock
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import java.io.File
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
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
                    else -> error("`$name` is not a host directive")
                }
            }
        }

        /**
         * What the host was started with since the last step, as a scenario writes it:
         * the request as its record, `at_ms` being the one integer field.
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
                                    if (name == "at_ms") put(name, text.toLong()) else put(name, text)
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

    private fun <S : State, E : Event> replay(machine: String, sm: StateMachineEngine<S, E>, save: () -> String) {
        val scenario = scenario(machine)
        assertEquals(machine, scenario.getValue("machine").jsonPrimitive.content)
        val rig = Rig()
        serve(sm, rig.host)
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            val steps = scenario.getValue("steps").jsonArray
            assertTrue(steps.isNotEmpty(), "a scenario with no steps judges nothing")
            sm.tick()
            steps.forEachIndexed { n, element ->
                val step = element.jsonObject
                val note = step["note"]?.jsonPrimitive?.content ?: ""
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
    fun everyScenarioOfTheDirectoryIsReplayedHere() {
        val names = File(repoRoot(), "tests/host_features/scenarios").listFiles()!!
            .filter { it.extension == "json" }.map { it.nameWithoutExtension }.sorted()
        assertEquals(listOf("static_host_clock", "static_host_http", "static_host_kv", "static_host_notify"), names)
    }
}
