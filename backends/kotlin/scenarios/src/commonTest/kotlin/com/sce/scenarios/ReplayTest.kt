// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The replay held to what it judges, against a machine written here rather than
// generated: the generated machines replay their scenarios in `:sce-kotlin-tests`
// (`StaticScenarioTest`), and those pass when the replay judges nothing at all.
// What is held here is that it judges, and how it reports a step it judged false.

package com.sce.scenarios

import com.sce.runtime.MeshCall
import com.sce.runtime.SavedState
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import com.sce.runtime.Json as SavedJson

class ReplayTest {

    /** A machine the test moves by hand: what it holds is whatever the test set last. */
    private class Machine(val onEvent: Machine.(String, String) -> Unit = { _, _ -> }) {
        var current = "idle"
        var configuration = listOf("idle")
        var variables: Map<String, Any?> = emptyMap()
        var ended = false
        var donedata = ""
        val events = mutableListOf<Pair<String, String>>()
        var ticks = 0
        var advancedMs = 0L

        fun save(): SavedState = SavedState("fake", configuration, current, variables)

        fun send(name: String, data: String) {
            events += name to data
            onEvent(this, name, data)
        }
    }

    private fun replayOn(machine: Machine, scenario: String, advance: ((Long) -> Unit)? = null) =
        replay(
            parseScenario(scenario),
            send = machine::send,
            tick = { machine.ticks++ },
            save = machine::save,
            ended = { machine.ended },
            donedata = { machine.donedata },
            advance = advance,
        )

    private fun json(text: String): JsonElement = Json.parseToJsonElement(text)

    /**
     * Every member a step can state, held true: an event with its data, a wait, the
     * state, the active states in another order, an integer saved as its digits, a
     * real spelled two ways, a record holding both, and an end with its data. Red if
     * the replay stops reading the step's `data`, stops calling `advance`, or holds
     * the configuration in order rather than as a set.
     */
    @Test
    fun aMachineThatHoldsEveryStepPassesItsScenario() {
        val machine = Machine { name, _ ->
            when (name) {
                "go" -> {
                    current = "busy"
                    configuration = listOf("root", "busy")
                    variables = mapOf(
                        "count" to "2",
                        "ratio" to SavedJson.Number("1.6777216E7"),
                        "rec" to mapOf("a" to "1", "r" to SavedJson.Number("0.5")),
                    )
                }
                "stop" -> {
                    ended = true
                    donedata = """{"a":1,"b":2}"""
                }
            }
        }
        machine.variables = mapOf("count" to "0")
        replayOn(
            machine,
            """
            {"machine": "fake", "steps": [
              {"note": "start", "expect": {"state": "idle", "variables": {"count": 0}}},
              {"note": "go", "event": "go", "data": {"n": 2},
               "expect": {"state": "busy", "configuration": ["busy", "root"],
                          "variables": {"count": 2, "ratio": 16777216.0, "rec": {"a": 1, "r": 0.5}}}},
              {"note": "wait", "advance_ms": 1000, "expect": {"state": "busy"}},
              {"note": "end", "event": "stop", "expect": {"ended": true, "donedata": {"b": 2, "a": 1}}}
            ]}
            """,
            advance = { ms -> machine.advancedMs += ms },
        )
        assertEquals(listOf("go" to """{"n":2}""", "stop" to ""), machine.events)
        assertEquals(1000L, machine.advancedMs)
        // Once before the first step, and once after each event.
        assertEquals(3, machine.ticks)
    }

    /**
     * A step the machine does not hold names the step, its note and both sides. Red
     * if the replay stops comparing the current state, or counts steps from 1.
     */
    @Test
    fun aWrongStateIsReportedWithItsStepAndBothSides() {
        val machine = Machine { name, _ -> if (name == "go") current = "busy" }
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replayOn(
                machine,
                """
                {"machine": "fake", "steps": [
                  {"note": "first", "expect": {"state": "idle"}},
                  {"note": "second", "event": "go", "expect": {"state": "done"}}
                ]}
                """,
            )
        }
        assertEquals(1, mismatch.step)
        assertEquals("second", mismatch.note)
        assertEquals("the current state", mismatch.what)
        assertEquals(JsonPrimitive("done"), mismatch.expected)
        assertEquals(JsonPrimitive("busy"), mismatch.actual)
        assertEquals("step 1 (second): the current state is \"busy\", not \"done\"", mismatch.message)
    }

    /**
     * A scenario of a machine given no router (`"router": false`) is refused by a
     * driver that records sends, which registered a router, and is replayed by one
     * that records none. Red if [routed] stops reading the member, or the replay
     * stops holding the driver to it.
     */
    @Test
    fun aScenarioWithNoRouterIsReplayedOnlyByADriverThatGaveNone() {
        val text = """{"machine": "fake", "router": false, "steps": [{"note": "start", "expect": {"state": "idle"}}]}"""
        assertFalse(routed(parseScenario(text)))
        assertTrue(routed(parseScenario("""{"machine": "fake", "steps": []}""")))
        val machine = Machine()
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replay(
                parseScenario(text),
                send = machine::send,
                tick = { machine.ticks++ },
                save = machine::save,
                ended = { machine.ended },
                sent = { emptyList() },
            )
        }
        assertNull(mismatch.step)
        replayOn(machine, text)
    }

    /**
     * A step's `invoke` is a child's event: a driver that gives children no routers
     * fails the step rather than hand the event to the machine. Red if the replay
     * sends it to the machine instead.
     */
    @Test
    fun aChildsEventWithNoRoutersFailsTheStep() {
        val machine = Machine()
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replayOn(machine, """{"steps": [{"invoke": "list", "event": "ping", "expect": {"state": "idle"}}]}""")
        }
        assertEquals(0, mismatch.step)
        assertTrue(machine.events.isEmpty(), "the child's event reached the machine")
    }

    /**
     * A step's `invoke` reaches the running child of the path through the router the
     * child was given, and one that names no running child fails. Red if
     * [ChildRouters] loses what it was handed, or the replay passes a step whose child
     * is not there.
     */
    @Test
    fun aChildsEventReachesItsRouterAndNoOtherPath() {
        val children = ChildRouters(RecordedSends())
        val router = assertNotNull(children.factory("list", ""))
        val machine = Machine()
        replay(
            parseScenario("""{"steps": [{"invoke": "list", "event": "ping", "expect": {"state": "idle"}}]}"""),
            send = machine::send,
            tick = { machine.ticks++ },
            save = machine::save,
            ended = { machine.ended },
            children = children,
        )
        val calls = router.takeCalls()
        assertEquals(listOf("ping"), calls.map { (it as MeshCall.Raise).name })
        assertTrue(machine.events.isEmpty(), "the child's event reached the machine")
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replay(
                parseScenario("""{"steps": [{"invoke": "peer", "event": "ping", "expect": {"state": "idle"}}]}"""),
                send = machine::send,
                tick = { machine.ticks++ },
                save = machine::save,
                ended = { machine.ended },
                children = children,
            )
        }
        assertEquals("no child of `peer` is running to be sent `ping`", mismatch.what)
    }

    /** An active state the machine lacks fails the step. Red if the configuration is not compared. */
    @Test
    fun aMissingActiveStateFailsTheStep() {
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replayOn(Machine(), """{"steps": [{"expect": {"configuration": ["idle", "root"]}}]}""")
        }
        assertEquals(0, mismatch.step)
        assertEquals("the active states", mismatch.what)
        assertEquals(json("""["idle", "root"]"""), mismatch.expected)
        assertEquals(json("""["idle"]"""), mismatch.actual)
    }

    /**
     * A variable the saved state does not carry is absent, not null. Red if the
     * replay skips a variable it cannot find.
     */
    @Test
    fun aVariableTheMachineDoesNotHoldIsReportedAbsent() {
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replayOn(Machine(), """{"steps": [{"note": "n", "expect": {"variables": {"gone": 1}}}]}""")
        }
        assertEquals("variable `gone`", mismatch.what)
        assertNull(mismatch.actual)
        assertEquals("step 0 (n): variable `gone` is absent, not 1", mismatch.message)
    }

    /** A step that says the machine ended fails on one that did not. Red if `ended` is not asked. */
    @Test
    fun aMachineThatDidNotEndFailsAStepThatSaysItDid() {
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replayOn(Machine(), """{"steps": [{"expect": {"ended": true}}]}""")
        }
        assertEquals(JsonPrimitive(true), mismatch.expected)
        assertEquals(JsonPrimitive(false), mismatch.actual)
    }

    /**
     * A step that states what the driver does not record fails rather than pass
     * unjudged. Red if the replay skips `host_calls` when the driver gave no reader.
     */
    @Test
    fun aStepStatingHostCallsFailsADriverThatRecordsNone() {
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replayOn(Machine(), """{"steps": [{"expect": {"host_calls": []}}]}""")
        }
        assertEquals(0, mismatch.step)
        assertNull(mismatch.expected)
        assertNull(mismatch.actual)
        assertTrue("records no host calls" in mismatch.what, mismatch.what)
    }

    /** A wait with no clock to move fails the step. Red if the replay skips `advance_ms` without a driver. */
    @Test
    fun aWaitFailsADriverWithNoClockToAdvance() {
        val mismatch = assertFailsWith<ScenarioMismatch> {
            replayOn(Machine(), """{"steps": [{"note": "w", "advance_ms": 5, "expect": {}}]}""")
        }
        assertEquals(0, mismatch.step)
        assertEquals("w", mismatch.note)
    }

    /** A scenario with no steps judges nothing, so it fails. Red if an empty `steps` passes. */
    @Test
    fun aScenarioWithNoStepsFails() {
        val mismatch = assertFailsWith<ScenarioMismatch> { replayOn(Machine(), """{"steps": []}""") }
        assertNull(mismatch.step)
        assertEquals("a scenario with no steps judges nothing", mismatch.message)
    }

    /**
     * A step is judged on its own calls: one that states none still takes them, so
     * the next holds only what it made. Red if `hostCalls` is read only on a step
     * that states `host_calls`.
     */
    @Test
    fun hostCallsAreTakenOnEveryStep() {
        val calls = mutableListOf(hostCall("started"))
        val machine = Machine { name, _ -> if (name == "go") calls += hostCall("went", JsonPrimitive(1)) }
        replay(
            parseScenario(
                """
                {"steps": [
                  {"note": "start", "expect": {}},
                  {"note": "go", "event": "go", "expect": {"host_calls": [{"action": "went", "args": [1]}]}}
                ]}
                """,
            ),
            send = machine::send,
            tick = {},
            save = machine::save,
            ended = { machine.ended },
            hostCalls = {
                val fresh = calls.toList()
                calls.clear()
                fresh
            },
        )
    }

    /**
     * What a saved value is held to: an integer by its digits, a real by its value
     * however it is spelled, a record field by field. Red if any arm of `holds` is
     * taken out or widened.
     */
    @Test
    fun holdsReadsTheSavedSpellingOfAValue() {
        assertTrue(holds(JsonPrimitive("42"), JsonPrimitive(42)))
        assertFalse(holds(JsonPrimitive("42"), JsonPrimitive(43)))
        // A string is not a real: only an integer is saved as its text.
        assertFalse(holds(JsonPrimitive("1.5"), json("1.5")))
        assertTrue(holds(json("1.6777216E7"), json("16777216.0")))
        assertFalse(holds(json("0.5"), json("0.25")))
        assertTrue(holds(json("""{"a": "1", "r": 0.5}"""), json("""{"a": 1, "r": 0.50}""")))
        assertFalse(holds(json("""{"a": "1", "b": "2"}"""), json("""{"a": 1}""")))
        assertTrue(holds(json("""["1", "2"]"""), json("[1, 2]")))
        assertFalse(holds(json("""["1"]"""), json("[1, 2]")))
        assertFalse(holds(null, JsonPrimitive(1)))
    }

    /**
     * An absent value (docs/adr/0055) is `null` in the scenario and in the saved
     * state, and holds only a `null`: not a number, not the text `null`, and not
     * a value the saved state does not hold at all. A record's absent field is
     * held the same way.
     */
    @Test
    fun holdsReadsAnAbsentValueAsNullAndNothingElse() {
        assertTrue(holds(json("null"), json("null")))
        assertFalse(holds(json("7"), json("null")))
        assertFalse(holds(json("null"), json("7")))
        assertFalse(holds(json("\"7\""), json("null")))
        assertFalse(holds(json("null"), json("\"7\"")))
        assertFalse(holds(JsonPrimitive("null"), json("null")))
        assertFalse(holds(null, json("null")))
        assertTrue(holds(json("""{"id": 1, "due": null}"""), json("""{"id": 1, "due": null}""")))
        assertFalse(holds(json("""{"id": 1, "due": "12"}"""), json("""{"id": 1, "due": null}""")))
    }
}
