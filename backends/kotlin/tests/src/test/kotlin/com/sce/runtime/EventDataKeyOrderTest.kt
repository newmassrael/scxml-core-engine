// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The members of a JSON object written into `_event.data` come in one order on
// every engine (ARCHITECTURE.md, "JSON Object Key Order"). The cases live in
// tests/json_text/object_key_order.json, read here with this runtime's own
// parser, so the writer a generated `<send>` calls is measured against the
// same table as every other engine's.
//
// `buildJsonFromParams` and `putParam` are protected — generated machines are
// their callers — so the probe below is the smallest possible subclass rather
// than a mock: the writer under test is the real one, reached the way
// generated code reaches it.

package com.sce.runtime

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

internal object OrderState : State

internal object OrderEvent : Event

/** The smallest machine that can be asked what a `<send>` writes. */
internal class SendProbe : StateMachineEngine<OrderState, OrderEvent>() {
    override val initialState: OrderState = OrderState

    override fun firstEnabledTransition(state: OrderState, event: OrderEvent?): EnabledTransition<OrderState, HistoryId>? =
        null

    override fun onEntry(state: OrderState, isDefaultEntry: Boolean) {}

    override fun onExit(state: OrderState) {}

    override fun executeTransitionContent(source: OrderState, transitionIndex: Int) {}

    /** What a send writes for these params, collected as generated code collects them. */
    fun eventData(params: List<Pair<String, Any?>>): String {
        val payload = LinkedHashMap<String, Any?>()
        for ((name, value) in params) putParam(payload, name, value)
        return buildJsonFromParams(payload)
    }

    /** What a send writes for one value, as generated code asks for it. */
    fun valueJson(value: Any?): String = valueToJson(value)

    /** What an untyped `<param>` writes for one value on a form-encoded wire. */
    fun valueWire(value: Any?): String = valueToWireString(value)
}

/** A value a script engine keeps to itself; all it can say is its own JSON. */
private class HeldByTheEngine(private val json: String) : EngineHeldValue {
    override fun toJson(): String = json
}

class EventDataKeyOrderTest {

    /** A value as the data model holds it: [Json.Number] is a whole number here. */
    private fun held(value: Any?): Any? = when (value) {
        is Json.Number -> value.text.toLong()
        is Map<*, *> -> value.entries.associateTo(LinkedHashMap<String, Any?>()) { (k, v) -> k as String to held(v) }
        is List<*> -> value.map { held(it) }
        else -> value
    }

    @Test
    fun `the members of an object are written in the one order every engine writes`() {
        val table = Json.parse(File("tests/json_text/object_key_order.json").readText()) as Map<*, *>
        val cases = table["cases"] as List<*>
        assertTrue(cases.size >= 8, "the table lost cases: ${cases.size}")
        val probe = SendProbe()
        for (case in cases) {
            val fields = case as Map<*, *>
            val params = (fields["params"] as List<*>).map { pair ->
                val (name, value) = pair as List<*>
                (name as String) to held(value)
            }
            assertEquals(fields["data"] as String, probe.eventData(params), fields["name"] as String)
        }
    }

    /**
     * [value] spelled through the typed payload writers, one call per constructor, as a
     * generated raise spells a value of its type: an object as a record, an array as a
     * list, a number as written, anything else as a value of a scalar type.
     */
    private fun typedJson(value: Any?): EventPayload.Raw = when (value) {
        is Map<*, *> -> EventPayload.recordJson(value.entries.associate { (k, v) -> k as String to typedJson(v) })
        is List<*> -> EventPayload.listJson(value) { typedJson(it) }
        is Json.Number -> EventPayload.Raw(value.text)
        else -> EventPayload.valueJson(value)
    }

    // The typed writers a generated raise spells a record and a list with
    // (docs/adr/0086, section 5.3) are held to the same table, at every depth. A
    // case whose name repeats is a `<send>`'s list of params, which only the
    // parameter writer collects: a record holds each field once.
    @Test
    fun `a record and a list are written in the one order every engine writes`() {
        val table = Json.parse(File("tests/json_text/object_key_order.json").readText()) as Map<*, *>
        var written = 0
        for (case in table["cases"] as List<*>) {
            val fields = case as Map<*, *>
            val params = (fields["params"] as List<*>).map { pair ->
                val (name, value) = pair as List<*>
                (name as String) to value
            }
            if (params.map { it.first }.toSet().size < params.size) continue
            val text = EventPayload.recordJson(params.associate { (name, value) -> name to typedJson(value) }).json
            assertEquals(fields["data"] as String, text, fields["name"] as String)
            written++
        }
        assertTrue(written >= 10, "the table lost the cases a record is written for: $written")
    }

    // `JSON.stringify` writes an integer-like key before any other, whatever the
    // text says, so an object a script engine keeps comes back in an order of
    // the engine's choosing. What the runtime writes is the table's order, and
    // a number or a string in it is written as the engine spelled it.
    @Test
    fun `a value an engine keeps is written in the same order`() {
        val engineText = """{"2":3,"10":1,"b":{"z":1.5e+21,"a":"x\ny"},"a":[{"d":1,"c":2}]}"""
        assertEquals(
            """{"10":1,"2":3,"a":[{"c":2,"d":1}],"b":{"a":"x\ny","z":1.5e+21}}""",
            SendProbe().valueJson(HeldByTheEngine(engineText)),
        )
    }

    @Test
    fun `a key is ordered by code point, not by UTF-16 unit`() {
        val basicPlane = "～"
        val supplementary = "😀"
        assertTrue(supplementary < basicPlane, "String order puts the surrogate pair first")
        assertTrue(Json.compareKeys(basicPlane, supplementary) < 0)
        assertTrue(Json.compareKeys(supplementary, basicPlane) > 0)
        assertEquals(0, Json.compareKeys("same", "same"))
        assertTrue(Json.compareKeys("a", "ab") < 0)
        assertEquals(
            """{"a":1,"$basicPlane":2,"$supplementary":3}""",
            Json.writeCanonical(linkedMapOf("a" to Json.Number("1"), supplementary to Json.Number("3"), basicPlane to Json.Number("2"))),
        )
    }
}
