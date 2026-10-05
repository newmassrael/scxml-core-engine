// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<param expr>` of a `<send>` or an `<invoke>`
// the HOST serves, in a `datamodel="sce-static"` machine, carries the value of
// a typed expression read from the machine's own fields when the send or the
// invoke happens. Kotlin compile+run gate; the Rust twin is
// `a_static_machines_typed_params_reach_the_host`.
//
// The committed machine (com/sce/integration/statechart_static_host_params/) is
// generated from
// sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml
// (regen: scripts/regen_host_processor_kotlin.sh) and constructed with NO script
// engine: its variables are fields, and a `<param>` that needed an engine to be
// read would not have one to ask.
//
// What this holds is the value on the wire. The run `bump`, `go` changes every
// variable before the send and the invoke read it, so a copy taken at start-up
// (3, false, "idle") is told from what the fields hold now (4, true, "busy").
// Before the `<param>` was lowered, the Kotlin machine sent an empty payload,
// and a host invoke read a copy of the variable inside an engine that `<assign>`
// never wrote.

package com.sce.integration

import com.sce.integration.statechart_static_host_params.StatechartStaticHostParamsEvent
import com.sce.integration.statechart_static_host_params.StatechartStaticHostParamsState
import com.sce.integration.statechart_static_host_params.StatechartStaticHostParamsStateMachine
import com.sce.runtime.Json
import com.sce.runtime.StateMachineEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("StaticHostParams — a typed <param> reaches the host (sce-static, Kotlin AOT)")
class StaticHostParamsTest {

    // The type the fixture was compiled for; `scripts/regen_host_processor_kotlin.sh`
    // passes the same string to both declarations.
    private val declaredType = "x-sce-host"

    private class Host {
        val sends = mutableListOf<StateMachineEngine.HostSendRequest>()
        val starts = mutableListOf<StateMachineEngine.HostInvokeRequest>()
    }

    /** A machine with the host's side registered, standing at `idle`. */
    private fun <T> started(body: (StatechartStaticHostParamsStateMachine, Host) -> T): T {
        val host = Host()
        // No engine argument: the document is `sce-static`, so codegen emits a
        // machine with no script engine at all.
        val sm = StatechartStaticHostParamsStateMachine()
        try {
            sm.registerEventProcessor(declaredType) { request ->
                host.sends.add(request)
                emptyList()
            }
            sm.registerInvoker(declaredType) { event ->
                event.start?.let { host.starts.add(it) }
                null
            }
            sm.initialize()
            return body(sm, host)
        } finally {
            sm.cleanup()
        }
    }

    private fun drive(sm: StatechartStaticHostParamsStateMachine, vararg events: StatechartStaticHostParamsEvent) {
        for (event in events) {
            sm.send(event)
            sm.tick()
        }
    }

    /**
     * The text each param crosses as, given what `count`, `ready`, `label` and
     * `twice` hold. `delta` and `ratio` never change; `boom` is left out,
     * because the multiplication that makes it overflows a 32-bit field.
     */
    private fun wanted(count: String, ready: String, label: String, twice: String): Map<String, List<String>> =
        mapOf(
            "count" to listOf(count),
            "ready" to listOf(ready),
            "label" to listOf(label),
            "twice" to listOf(twice),
            "delta" to listOf("-5"),
            "ratio" to listOf("1.5"),
        )

    /**
     * `eventData` is the pairs as JSON, typed as the data model holds them: a
     * number stays a number, a bool a bool, a string a string. Compared as
     * values, not text, because the order of an object's members is the
     * backend's.
     */
    private fun assertTypedEventData(eventData: String, what: String) {
        val value = Json.parse(eventData) as? Map<*, *>
            ?: throw AssertionError("$what: eventData is not a JSON object: $eventData")
        fun number(name: String) = (value[name] as? Json.Number)?.text
        assertEquals("4", number("count"), "$what: `count` in $eventData")
        assertEquals(true, value["ready"], "$what: `ready` in $eventData")
        assertEquals("busy", value["label"], "$what: `label` in $eventData")
        assertEquals("8", number("twice"), "$what: `twice` in $eventData")
        assertEquals("-5", number("delta"), "$what: `delta` in $eventData")
        assertEquals("1.5", number("ratio"), "$what: `ratio` in $eventData")
        assertTrue(!value.containsKey("boom"), "$what: a pair whose value failed is left out: $eventData")
    }

    @Test
    fun aSendParamCarriesTheValueTheFieldsHoldWhenItIsSent() {
        started { sm, host ->
            drive(sm, StatechartStaticHostParamsEvent.Bump, StatechartStaticHostParamsEvent.Go)
            assertEquals(StatechartStaticHostParamsState.Working, sm.currentState.value)

            assertEquals(1, host.sends.size, "one <send>, one request: ${host.sends}")
            assertEquals(wanted("4", "true", "busy", "8"), host.sends[0].params, "the text each <param> crosses as")
            assertTypedEventData(host.sends[0].eventData, "send")
        }
    }

    @Test
    fun anInvokeParamCarriesTheValueTheFieldsHoldWhenItStarts() {
        started { sm, host ->
            drive(sm, StatechartStaticHostParamsEvent.Bump, StatechartStaticHostParamsEvent.Go)

            assertEquals(1, host.starts.size, "one <invoke>, one start: ${host.starts}")
            assertEquals(
                wanted("4", "true", "busy", "8"),
                host.starts[0].params,
                "the text each <param> crosses as: a copy taken at start-up would say count 3, ready false, label idle",
            )
            assertTypedEventData(host.starts[0].eventData, "invoke")
            assertEquals(
                "job://params",
                host.starts[0].src,
                "the src is the string the machine computed when the invocation started",
            )
        }
    }

    // W3C SCXML 6.4.1: an attribute that cannot be evaluated starts nothing. `big`
    // makes `count` too large for the multiplication the `srcexpr` is chosen by, so
    // the source cannot be computed and the host is never asked.
    @Test
    fun anInvokeWhoseSourceCannotBeComputedStartsNothing() {
        started { sm, host ->
            drive(sm, StatechartStaticHostParamsEvent.Big, StatechartStaticHostParamsEvent.Go)

            assertEquals(0, host.starts.size, "a source nobody could compute starts nothing: ${host.starts}")
        }
    }

    @Test
    fun aParamReadBeforeAnyBumpCarriesTheDeclaredValues() {
        // The same machine on the shorter run: nothing has written a variable,
        // so the fields still hold what `<data expr>` gave them. The control
        // that keeps the two cases above from passing on a value that is simply
        // always the new one.
        started { sm, host ->
            drive(sm, StatechartStaticHostParamsEvent.Go)
            assertTrue(host.sends.size == 1 && host.starts.size == 1, "${host.sends} ${host.starts}")
            assertEquals(wanted("3", "false", "idle", "6"), host.sends[0].params)
            assertEquals(wanted("3", "false", "idle", "6"), host.starts[0].params)
        }
    }

    // W3C SCXML 5.7.1: a `<param>` whose value cannot be computed — here a
    // multiplication a 32-bit field cannot hold — is reported with
    // `error.execution` and its pair left out, while the message still goes and
    // the invocation still starts. `errors` counts the reports the document
    // took, one for the send and one for the invoke, so a pair dropped in
    // silence is told from one reported.
    @Test
    fun aParamWhoseValueCannotBeComputedIsReportedAndLeftOut() {
        started { sm, host ->
            drive(sm, StatechartStaticHostParamsEvent.Bump, StatechartStaticHostParamsEvent.Go)

            assertEquals(1, host.sends.size, "the send still went")
            assertEquals(1, host.starts.size, "the invoke still started")
            assertEquals(2u, sm.errors, "one error.execution for the send's `boom` and one for the invoke's")
            assertTrue("boom" !in host.sends[0].params, "send: the failed pair is left out: ${host.sends[0].params}")
            assertTrue("boom" !in host.starts[0].params, "invoke: the failed pair is left out: ${host.starts[0].params}")
        }
    }
}
