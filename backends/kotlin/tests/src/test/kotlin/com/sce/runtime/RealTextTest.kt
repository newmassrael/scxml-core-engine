// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A float is spelled the way ECMAScript spells it on every engine
// (ARCHITECTURE.md, "JSON Number Text (Single Source of Truth)"). The cases
// live in tests/json_text/real_text.json, read here with this runtime's own
// parser, each as the IEEE 754 bit pattern of the value so that no number
// parser stands between the table and the double. Every writer in this runtime
// that spells one is held to it: `Json.numberText`, the JSON a generated
// `<send>` writes, the text of an untyped `<param>`, and a typed payload field.

package com.sce.runtime

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RealTextTest {
    private fun cases(): List<Triple<String, Double, String>> {
        val table = Json.parse(File("tests/json_text/real_text.json").readText()) as Map<*, *>
        val cases = table["cases"] as List<*>
        assertTrue(cases.size >= 40, "the table lost cases: ${cases.size}")
        return cases.map { case ->
            val fields = case as Map<*, *>
            val bits = java.lang.Long.parseUnsignedLong(fields["bits"] as String, 16)
            Triple(fields["name"] as String, Double.fromBits(bits), fields["text"] as String)
        }
    }

    @Test
    fun `a float is spelled as ECMAScript spells it`() {
        for ((name, value, text) in cases()) {
            assertEquals(text, Json.numberText(value), name)
        }
    }

    @Test
    fun `a float in a send's JSON is spelled the same way`() {
        val probe = SendProbe()
        for ((name, value, text) in cases()) {
            assertEquals(text, probe.valueJson(value), "$name: as JSON")
        }
    }

    @Test
    fun `a float as an untyped param is spelled the same way`() {
        val probe = SendProbe()
        for ((name, value, text) in cases()) {
            assertEquals(text, probe.valueWire(value), "$name: as an untyped param")
        }
    }

    @Test
    fun `a float in a typed payload is spelled the same way`() {
        for ((name, value, text) in cases()) {
            assertEquals("""{"x":$text}""", EventPayload.encode(mapOf("x" to value)), "$name: as a payload field")
        }
    }

    @Test
    fun `a float that is not finite has no JSON spelling and a wire one`() {
        val probe = SendProbe()
        for ((value, wire) in listOf(
            Double.NaN to "NaN",
            Double.POSITIVE_INFINITY to "Infinity",
            Double.NEGATIVE_INFINITY to "-Infinity",
        )) {
            assertEquals("null", probe.valueJson(value), "$value as JSON")
            assertEquals(wire, probe.valueWire(value), "$value as an untyped param")
            assertEquals("""{"x":null}""", EventPayload.encode(mapOf("x" to value)), "$value as a payload field")
        }
    }

    @Test
    fun `a whole-number type is written as its digits, not through a double`() {
        val probe = SendProbe()
        // 2^53 + 1 is the first whole number a double cannot hold.
        assertEquals("9007199254740993", probe.valueJson(9007199254740993L))
        assertEquals("9007199254740993", probe.valueWire(9007199254740993L))
        assertEquals("-5", probe.valueJson(-5))
    }
}
