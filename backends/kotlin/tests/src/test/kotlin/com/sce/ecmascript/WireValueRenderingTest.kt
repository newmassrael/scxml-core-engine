// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-C-2 — a form-encoded `<param>` carries the value, not the platform's
// spelling of it.
//
// The BasicHTTP Event I/O Processor sends each `<param>` as one `name=value`
// pair, so the value crosses as text and the receiving end hands that text to
// `_event.data`; no script engine reads it at either end. Generated HTTP sends
// used to render it with `v?.toString() ?: ""`, which is whatever the *engine's
// host object* prints — and this backend ships three engines. Rhino hands back
// a Double for every number, so a document that sent `5` put `5.0` on the wire
// here while the C++ channel sent `5` for the same document.
//
// Measured 2026-08-21, that one field had six spellings across six backends.
// All six now answer the column C++ `ScriptResultUtils::resultToString` gives,
// which is ECMAScript's `String(value)` with absence empty (§scxml-C-1) and a
// structured value as JSON. The rows below are the same rows as
// `backends/rust/runtime/tests/wire_value_is_not_engine_source.rs`,
// `backends/go/runtime/wire_value_test.go` and
// `backends/python/tests/ecmascript/test_wire_value_is_not_engine_source.py`.
//
// `valueToWireString` is protected — generated machines are its callers — so
// the probe below is the smallest possible subclass rather than a mock: the
// method under test is the real one, reached the way generated code reaches it.

package com.sce.ecmascript

import com.sce.runtime.Event
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.TransitionResult
import kotlin.test.Test
import kotlin.test.assertEquals

private object ProbeState : State

private object ProbeEvent : Event

/** The smallest machine that can be asked how it renders a value. */
private class WireProbe : StateMachineEngine<ProbeState, ProbeEvent>() {
    override val initialState: ProbeState = ProbeState

    override fun processEvent(state: ProbeState, event: ProbeEvent): TransitionResult<ProbeState> =
        TransitionResult.Ignored

    override fun onEntry(state: ProbeState, pathChild: ProbeState?) {}

    override fun onExit(state: ProbeState) {}

    override fun executeTransitionActions(source: ProbeState, event: ProbeEvent?, transitionIndex: Int) {}

    fun wire(value: Any?): String = valueToWireString(value)

    fun json(value: Any?): String = valueToJson(value)
}

class WireValueRenderingTest {

    @Test
    fun `a wire param reads the same whoever sent it`() {
        val probe = WireProbe()
        // Each row's comment is what this channel used to put on the wire.
        val rows: List<Pair<Any?, String>> = listOf(
            null to "",                       // was ""  (agreed already)
            true to "true",
            false to "false",
            42 to "42",
            5.0 to "5",                       // was "5.0" — Rhino's Double
            2.5 to "2.5",
            "plain" to "plain",
            // The quotes belong to the value.
            "\"quoted\"" to "\"quoted\"",
            listOf(1, 2) to "[1,2]",          // was "[1, 2]" — Kotlin's List.toString
            mapOf("k" to "v") to "{\"k\":\"v\"}", // was "{k=v}"
        )
        for ((value, expected) in rows) {
            assertEquals(expected, probe.wire(value), "wire text for $value")
        }
    }

    @Test
    fun `a structured value on the wire is the payload encoder's bytes`() {
        val probe = WireProbe()
        val value = mapOf("b" to 2, "a" to listOf(1, "x"))
        assertEquals(probe.json(value), probe.wire(value))
    }

    @Test
    fun `a string is quoted as JSON and bare on the wire`() {
        val probe = WireProbe()
        assertEquals("\"v\"", probe.json("v"))
        assertEquals("v", probe.wire("v"))
    }
}
