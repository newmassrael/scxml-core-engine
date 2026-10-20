// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The recorders a driver hands the replay, held to the entries a scenario's
// `host_calls`, `child_hosts` and `sent` write.

package com.sce.scenarios

import com.sce.runtime.StateMachineEngine
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame

class RecordersTest {

    private fun json(text: String): JsonElement = Json.parseToJsonElement(text)

    /** What a list of recorded calls holds, read and forgotten, as a recording host's reader does. */
    private fun takeAll(recorded: MutableList<JsonElement>): List<JsonElement> {
        val fresh = recorded.toList()
        recorded.clear()
        return fresh
    }

    /** A host call is its action and its arguments in order. Red if `args` loses its order or its name. */
    @Test
    fun aHostCallIsItsActionAndItsArguments() {
        assertEquals(
            json("""{"action": "showAttempts", "args": [0, false]}"""),
            hostCall("showAttempts", JsonPrimitive(0), JsonPrimitive(false)),
        )
        assertEquals(json("""{"action": "started", "args": []}"""), hostCall("started"))
    }

    /**
     * A child is announced once, with the calls of the step it was asked for in;
     * after that it has an entry only in a step it called its host in, and each
     * start of one invoke is counted from 1. Red if `announced` is never set, if
     * `start` counts every child rather than those of its invoke, or if a child
     * named inline is given a `candidate`.
     */
    @Test
    fun aChildIsAnnouncedOnceAndThenOnlyWhenItCalls() {
        val book = ChildHosts()
        val first = mutableListOf<JsonElement>()
        assertSame(first, book.open("worker", null, first, ::takeAll))
        assertEquals(
            listOf(json("""{"invoke": "worker", "start": 1, "host_calls": []}""")),
            book.take(),
        )
        assertEquals(emptyList<JsonElement>(), book.take())

        first += hostCall("started")
        val second = mutableListOf<JsonElement>()
        book.open("worker", "static_hosted_first", second, ::takeAll)
        val other = mutableListOf<JsonElement>()
        book.open("other", null, other, ::takeAll)
        assertEquals(
            listOf(
                json("""{"invoke": "worker", "start": 1, "host_calls": [{"action": "started", "args": []}]}"""),
                json("""{"invoke": "worker", "start": 2, "candidate": "static_hosted_first", "host_calls": []}"""),
                json("""{"invoke": "other", "start": 1, "host_calls": []}"""),
            ),
            book.take(),
        )
        assertEquals(emptyList<JsonElement>(), book.take())
    }

    /**
     * A send is its target, its event and its data as JSON, the data left out when
     * there is none, and each is taken once. Red if `take` does not clear, or writes
     * an empty `data`.
     */
    @Test
    fun aSendIsRecordedOnceAsTheScenarioWritesIt() {
        val sends = RecordedSends()
        val answered = sends.router(
            StateMachineEngine.HostSendRequest(
                processorType = "sce:mesh",
                eventName = "hello",
                target = "#server",
                eventData = """{"n":1}""",
            ),
        )
        assertEquals(emptyList<StateMachineEngine.HostSendResponse>(), answered)
        sends.router(StateMachineEngine.HostSendRequest(processorType = "sce:mesh", eventName = "bye", target = "#server"))
        assertEquals(
            listOf(
                json("""{"target": "#server", "event": "hello", "data": {"n": 1}}"""),
                json("""{"target": "#server", "event": "bye"}"""),
            ),
            sends.take(),
        )
        assertEquals(emptyList<JsonElement>(), sends.take())
    }
}
