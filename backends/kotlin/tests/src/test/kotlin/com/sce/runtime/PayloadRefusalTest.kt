// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// tests/json_text/payload_refusal.json, read here with this runtime's own
// parser: every case read through the payload reader, the code, pointer and
// bound of each refusal held to what the table says, and each case it accepts
// read (docs/adr/0087, section 9.1, item 3). The table is the one every
// runtime's reader is held to.

package com.sce.runtime

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail

class PayloadRefusalTest {

    /** [v] read as the type [type] the table describes, one call per constructor. */
    private fun read(type: Any?, v: EventPayload.Value) {
        when (type) {
            is String -> when (type) {
                "uint8" -> v.uint8()
                "uint16" -> v.uint16()
                "uint32" -> v.uint32()
                "uint64" -> v.uint64()
                "int8" -> v.int8()
                "int16" -> v.int16()
                "int32" -> v.int32()
                "int64" -> v.int64()
                "float32" -> v.float32()
                "float64" -> v.float64()
                "bool" -> v.boolean()
                "string" -> v.name()
                else -> fail("the table names no type $type")
            }
            is List<*> -> {
                val name = v.name()
                if (name !in type) {
                    throw v.unknownVariant(name, "Described")
                }
            }
            is Map<*, *> -> when {
                type.containsKey("optional") -> v.optional { read(type["optional"], it) }
                type.containsKey("string") -> v.string(bound(type["string"]))
                type.containsKey("bytes") -> v.bytes(bound(type["bytes"]))
                type.containsKey("list") -> v.list(bound(type["max"])) { read(type["list"], it) }
                type.containsKey("record") -> v.record { fields ->
                    for ((id, field) in type["record"] as Map<*, *>) {
                        read(field, fields.field(id as String))
                    }
                }
                else -> fail("a type of the table names no constructor: $type")
            }
            else -> fail("a type of the table is a name, a list or an object: $type")
        }
    }

    private fun bound(number: Any?): Int = (number as Json.Number).text.toInt()

    @Test
    fun `every case of the shared table is refused with its code pointer and bound`() {
        val table = Json.parse(File("tests/json_text/payload_refusal.json").readText()) as Map<*, *>
        val cases = table["cases"] as List<*>
        assertTrue(cases.size >= 40, "the table lost cases: ${cases.size}")
        val failures = mutableListOf<String>()
        for (case in cases) {
            val fields = case as Map<*, *>
            val name = fields["name"] as String
            val data = fields["data"] as String
            val want = fields["refusal"] as Map<*, *>?
            val got = try {
                read(fields["type"], EventPayload.decode(data).field("v"))
                null
            } catch (refusal: EventPayload.Refusal) {
                refusal
            }
            if (want == null) {
                if (got != null) failures += "$name: refused (${got.message}), the table reads it"
                continue
            }
            if (got == null) {
                failures += "$name: read, the table refuses it as ${want["code"]}"
                continue
            }
            val code = EventPayload.RefusalCode.named(want["code"] as String)
                ?: fail("$name: the enum declares no ${want["code"]}")
            val pointer = want["pointer"] as String
            val bound = (want["bound"] as Json.Number?)?.text?.toLong()
            if (got.code != code || got.pointer != pointer || got.bound != bound) {
                failures += "$name: refused ${got.code.declaredName} at '${got.pointer}' bound ${got.bound} " +
                    "(${got.message}), the table says ${code.declaredName} at '$pointer' bound $bound"
            }
        }
        assertTrue(failures.isEmpty(), failures.joinToString("\n"))
    }

    /** The codes are the variants `sce:std/lift/refusal_code.scxml` declares, by name and number. */
    @Test
    fun `every code is the variant the standard enum declares`() {
        val document = File("stdlib/lift/refusal_code.scxml").readText()
        assertEquals(EventPayload.RefusalCode.entries.size, document.split("<sce:variant ").size - 1)
        for (code in EventPayload.RefusalCode.entries) {
            val variant = "<sce:variant name=\"${code.declaredName}\" value=\"${code.number}\"/>"
            assertTrue(document.contains(variant), "$code: $variant")
            assertEquals(code, EventPayload.RefusalCode.named(code.declaredName))
        }
    }

    /** RFC 6901: `/` before each token, `~` written `~0` first and `/` written `~1`. */
    @Test
    fun `a pointer escapes tilde and solidus in each token`() {
        assertEquals("/items", EventPayload.pointerBelow("", "items"))
        assertEquals("/items/2", EventPayload.pointerBelow("/items", "2"))
        assertEquals("/a~0b~1c", EventPayload.pointerBelow("", "a~b/c"))
        assertEquals("/~01", EventPayload.pointerBelow("", "~1"))
        assertEquals("/", EventPayload.pointerBelow("", ""))
    }

    /** What a generated reader of an enum field answers for a name its enum does not declare. */
    @Test
    fun `a name no variant declares is refused as unknown variant`() {
        val refusal = EventPayload.decode("{\"status\":\"x\"}").field("status").unknownVariant("x", "Status")
        assertEquals("'status' (x) is not a variant of Status", refusal.message)
        assertEquals(EventPayload.RefusalCode.UNKNOWN_VARIANT, refusal.code)
        assertEquals("/status", refusal.pointer)
        assertNull(refusal.bound)
    }
}
