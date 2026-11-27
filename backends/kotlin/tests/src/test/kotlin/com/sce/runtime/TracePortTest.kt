// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The trace port's writer (docs/adr/0115, D4 and D8) lays a run out in the one
// run trace format every lane writes (tests/forge/conformance/run_trace.schema.json),
// byte for byte as the Rust lane's `trace_port` and the C11 lane's
// `sce/trace_json.h` do: the cases are theirs, with this lane's name. That a run
// tells the port each step it takes is held by the Kotlin lane's recorded runs
// (`com.sce.formal.ARunIsABehaviourOfItsModelTest` in :sce-kotlin-w3c) and by
// `AComputedEventNameIsMatchedLikeAnyOtherTest`.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals

class TracePortTest {
    private fun step(kind: TraceStepKind): TraceStep = TraceStep(
        kind = kind,
        event = "go.on",
        fired = listOf(TraceFired("s0", 2)),
        exited = listOf("s01", "s0"),
        entered = listOf("s1"),
        configuration = listOf("s1"),
        outcome = "pass",
    )

    @Test
    fun `each kind writes the fields the format asks of it`() {
        assertEquals(
            """{"kind": "start", "entered": ["s1"], "configuration": ["s1"]}""",
            step(TraceStepKind.START).toJson(),
        )
        assertEquals(
            """{"kind": "eventless", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], """ +
                """"entered": ["s1"], "configuration": ["s1"]}""",
            step(TraceStepKind.EVENTLESS).toJson(),
        )
        assertEquals(
            """{"kind": "internal", "event": "go.on", "fired": [{"source": "s0", "index": 2}], """ +
                """"exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}""",
            step(TraceStepKind.INTERNAL).toJson(),
        )
        assertEquals(
            """{"kind": "external", "event": "go.on", "fired": [{"source": "s0", "index": 2}], """ +
                """"exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}""",
            step(TraceStepKind.EXTERNAL).toJson(),
        )
        assertEquals(
            """{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}""",
            step(TraceStepKind.STOP).toJson(),
        )
        assertEquals(
            """{"kind": "stop", "exited": ["s01", "s0"], "outcome": null}""",
            step(TraceStepKind.STOP).copy(outcome = null).toJson(),
        )
    }

    @Test
    fun `a trace is written with its version and lane one step to a line`() {
        val trace = RunTrace()
        assertEquals("{\n  \"version\": 1,\n  \"lane\": \"kotlin\",\n  \"steps\": []\n}\n", trace.toJson())
        trace.push(step(TraceStepKind.START))
        trace.push(step(TraceStepKind.STOP))
        assertEquals(
            "{\n  \"version\": 1,\n  \"lane\": \"kotlin\",\n  \"steps\": [\n    " +
                "{\"kind\": \"start\", \"entered\": [\"s1\"], \"configuration\": [\"s1\"]},\n    " +
                "{\"kind\": \"stop\", \"exited\": [\"s01\", \"s0\"], \"outcome\": \"pass\"}\n  ]\n}\n",
            trace.toJson(),
        )
        assertEquals(2, trace.steps.size)
    }

    @Test
    fun `a name is written as a JSON string`() {
        val quoted = step(TraceStepKind.START).copy(entered = listOf("a\"b\\c"), configuration = emptyList())
        assertEquals(
            """{"kind": "start", "entered": ["a\"b\\c"], "configuration": []}""",
            quoted.toJson(),
        )
    }
}
