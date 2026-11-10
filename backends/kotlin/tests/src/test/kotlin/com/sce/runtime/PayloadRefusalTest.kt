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
                // The lower count, where the case states one (docs/adr/0086, section 15).
                type.containsKey("list") -> v.listWithin(
                    if (type.containsKey("min")) bound(type["min"]) else 0,
                    bound(type["max"]),
                ) { read(type["list"], it) }
                type.containsKey("record") -> v.record { fields ->
                    for ((id, field) in type["record"] as Map<*, *>) {
                        read(field, fields.field(id as String))
                    }
                }
                // The tag's enum is the cases' names; an empty case is null, any
                // other the record it holds (docs/adr/0092, section 4).
                type.containsKey("union") -> {
                    val described = type["union"] as Map<*, *>
                    val cases = described["cases"] as Map<*, *>
                    v.union(described["tag"] as String, { tag ->
                        val name = tag.name()
                        if (!cases.containsKey(name)) {
                            throw tag.unknownVariant(name, "Described")
                        }
                        name
                    }) { name, objectValue -> cases[name]?.let { read(it, objectValue) } }
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
            // A case that gives a depth is read as a `<sce:lift>` of the record of
            // the one field `v` reads it (docs/adr/0087, section 2.1).
            val depth = (fields["depth"] as Json.Number?)?.text?.toInt()
            // Its pointer is held to `pathBound` bytes, the standard refusal's 256
            // where the case gives none.
            val pathBound = (fields["pathBound"] as Json.Number?)?.text?.toInt() ?: 256
            val got = try {
                val decoded = if (depth == null) {
                    EventPayload.decode(data)
                } else {
                    EventPayload.lift(data, depth, pathBound, null)
                }
                read(fields["type"], decoded.field("v"))
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

    /**
     * A lift refuses a container past its depth over the brackets alone, before
     * the text is read as JSON, and names the container it refused, held to the
     * room of the refusal's path by whole tokens (docs/adr/0087, sections 2.1
     * and 2.4).
     */
    @Test
    fun `a lift refuses a container past its depth before it reads the text`() {
        fun refused(text: String, depth: Int = 1, pathBound: Int = 256) =
            EventPayload.liftTooDeep(text, depth, pathBound)?.let { Triple(it.code, it.pointer, it.bound) }
        val tooDeep = { pointer: String, depth: Long -> Triple(EventPayload.RefusalCode.TOO_DEEP, pointer, depth) }
        assertNull(refused("{\"title\":\"a\",\"count\":1}"))
        assertEquals(tooDeep("/title", 1), refused("{\"title\":[\"a\"]}"))
        assertEquals(tooDeep("/extra", 1), refused("{\"extra\":{\"a\":1}}"))
        assertEquals(tooDeep("/title", 1), refused("{\"\\u0074itle\":[1]}"))
        assertEquals(tooDeep("/0", 1), refused("[[1]]"))
        assertEquals(tooDeep("/2", 1), refused("[1,2,[3]]"))
        assertEquals(tooDeep("/extra", 1), refused("{\"extra\":[[[[[[["))
        assertNull(refused("{\"title\":\"[[{\\\"[\",\"count\":1}"))
        assertNull(refused("{\"title\":{\"a\":1}}", 2))
        assertNull(refused("{\"title\":\"[["))
        assertEquals(tooDeep("/extra", 1), refused("{\"title\":\"x\",\"extra\":[1]}"))
        assertEquals(tooDeep("/a/b", 2), refused("{\"a\":{\"b\":[1]}}", 2))
        assertEquals(tooDeep("/a/1/b", 3), refused("{\"a\":[0,{\"b\":[1]}]}", 3))
        assertEquals(tooDeep("/a~1b~0c", 1), refused("{\"a/b~c\":[1]}"))
        assertEquals(tooDeep("", 1), refused("{[1]}"))
        assertEquals(tooDeep("/a", 2), refused("{\"a\":{\"abcdef\":[1]}}", 2, 4))
        assertEquals(tooDeep("/a/bc", 2), refused("{\"a\":{\"bc\":[1]}}", 2, 5))
    }

    /** A lift reads its record as a delivery's is read, or answers its refusal as data. */
    @Test
    fun `a lift reads a record or answers its refusal`() {
        val read = { fields: EventPayload.Fields -> Pair(fields.uint8("v"), fields.uint8("w")) }
        val lifted = EventPayload.liftRecord("{\"v\":1}", 1, 256, "{\"w\":7}", read)
        assertTrue(lifted is EventPayload.LiftOutcome.Lifted && lifted.value == Pair(1.toUByte(), 7.toUByte()))
        for ((text, code) in listOf(
            " \t" to EventPayload.RefusalCode.NOT_JSON,
            "[[" to EventPayload.RefusalCode.TOO_DEEP,
            "[1]" to EventPayload.RefusalCode.NOT_OBJECT,
            "{\"v\":1,\"v\":2}" to EventPayload.RefusalCode.NOT_JSON,
            "{\"v\":256,\"w\":1}" to EventPayload.RefusalCode.OUT_OF_RANGE,
            "{\"w\":1}" to EventPayload.RefusalCode.MISSING,
        )) {
            val outcome = EventPayload.liftRecord(text, 1, 256, null, read)
            assertTrue(outcome is EventPayload.LiftOutcome.Refused && outcome.refusal.code == code, text)
        }
    }

    /**
     * A lift of a value read whole hands the whole object to the reader of its
     * type, which reads the record's defaults itself and holds a list to both
     * its counts before an element is read; what is refused before the value is
     * read is what a lift of a record refuses (docs/adr/0087, "Landing order"
     * item 8).
     */
    @Test
    fun `a lift reads a value whole by the reader of its type or answers its refusal`() {
        val read = { v: EventPayload.Value ->
            v.record("{\"w\":7}") { fields ->
                Pair(fields.field("items").listWithin(1, 2) { item -> item.uint8() }, fields.uint8("w"))
            }
        }
        val lifted = EventPayload.liftValue("{\"items\":[1,2]}", 2, 256, read)
        assertTrue(
            lifted is EventPayload.LiftOutcome.Lifted &&
                lifted.value == Pair(listOf(1.toUByte(), 2.toUByte()), 7.toUByte()),
        )
        for ((text, expected) in listOf(
            " \t" to Triple(EventPayload.RefusalCode.NOT_JSON, "", null),
            "{\"items\":[[1]]}" to Triple(EventPayload.RefusalCode.TOO_DEEP, "/items/0", 2L),
            "[1]" to Triple(EventPayload.RefusalCode.NOT_OBJECT, "", null),
            "{\"items\":[1,2,3]}" to Triple(EventPayload.RefusalCode.TOO_MANY, "/items", 2L),
            "{\"items\":[]}" to Triple(EventPayload.RefusalCode.TOO_FEW, "/items", 1L),
            "{\"items\":[1,256]}" to Triple(EventPayload.RefusalCode.OUT_OF_RANGE, "/items/1", null),
            "{\"w\":1}" to Triple(EventPayload.RefusalCode.MISSING, "/items", null),
        )) {
            val outcome = EventPayload.liftValue(text, 2, 256, read)
            val refusal = (outcome as? EventPayload.LiftOutcome.Refused)?.refusal ?: fail("$text is lifted")
            assertEquals(expected, Triple(refusal.code, refusal.pointer, refusal.bound), text)
        }
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
