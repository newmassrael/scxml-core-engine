// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A payload value of any bounded type is read at its path, or refused there
// before anything of it is kept (docs/adr/0086, sections 4 and 14.3): every
// element kind, at list depths 1 to 3, each bound 2, with an empty value, a
// maximal one, one past the bound at the innermost and the outermost list, a
// bad element, `null` where the element is not optional, an unknown key and an
// older revision. The oracle is the construction: each case is built with the
// value or the path it must come to. A maximal value is written at exactly the
// size the calculus gives it, and reads back as itself; a value a host builds is
// refused where it is made.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class ComposedPayloadTest {

    /** The element kinds. `REC` is `{a: uint8, b: string[3]}`, whose schema's later revision added `c: uint8` with the default 9. */
    private enum class Kind(val label: String) {
        U8("uint8"), U16("uint16"), U32("uint32"), U64("uint64"),
        I8("int8"), I16("int16"), I32("int32"), I64("int64"),
        F32("float32"), F64("float64"), BOOL("bool"), ENUM("enum"),
        TEXT("string[3]"), BYTES("bytes[3]"), OPT("optional<uint8>"),
        REC("record"), REC_LIST("record of a list"),
    }

    /** A kind, or a list of a type when [elem] is set. */
    private data class Ty(val kind: Kind?, val elem: Ty? = null)

    /** An absent optional of the model. */
    private object Absent {
        override fun toString(): String = "absent"
    }

    /** What a case must come to: a value, or a refusal at a path. */
    private sealed interface Want

    private data class Reads(val value: Any?) : Want

    private data class RefusedAt(val path: String, val sentence: String) : Want

    private fun nested(kind: Kind, depth: Int): Ty = (0 until depth).fold(Ty(kind)) { inner, _ -> Ty(null, inner) }

    /** The reader of [ty], one call per constructor, as the generator writes it. */
    private fun read(ty: Ty, v: EventPayload.Value): Any? {
        val elem = ty.elem
        if (elem != null) {
            return v.list(BOUND) { read(elem, it) }
        }
        return when (ty.kind!!) {
            Kind.U8 -> v.uint8().toULong()
            Kind.U16 -> v.uint16().toULong()
            Kind.U32 -> v.uint32().toULong()
            Kind.U64 -> v.uint64()
            Kind.I8 -> v.int8().toLong()
            Kind.I16 -> v.int16().toLong()
            Kind.I32 -> v.int32().toLong()
            Kind.I64 -> v.int64()
            Kind.F32 -> v.float32().toDouble()
            Kind.F64 -> v.float64()
            Kind.BOOL -> v.boolean()
            Kind.ENUM -> v.name().also { if (it !in ENUM_NAMES) throw v.refuse("($it) is not a variant of Level") }
            Kind.TEXT -> v.string(TEXT_BOUND)
            Kind.BYTES -> v.bytes(TEXT_BOUND).toList()
            Kind.OPT -> v.optional { it.uint8().toULong() } ?: Absent
            Kind.REC -> v.record("""{"c":9}""") { f ->
                listOf(
                    "a" to f.field("a").uint8().toULong(),
                    "b" to f.field("b").string(TEXT_BOUND),
                    "c" to f.field("c").uint8().toULong(),
                )
            }
            Kind.REC_LIST -> v.record { f -> listOf("tags" to f.field("tags").list(BOUND) { it.string(TEXT_BOUND) }) }
        }
    }

    /** The maximal value of [ty]: every list full, every text at its bound of control characters, every number at its longest spelling. */
    private fun maximal(ty: Ty): Any? {
        val elem = ty.elem
        if (elem != null) {
            return List(BOUND) { maximal(elem) }
        }
        return when (ty.kind!!) {
            Kind.U8 -> UByte.MAX_VALUE.toULong()
            Kind.U16 -> UShort.MAX_VALUE.toULong()
            Kind.U32 -> UInt.MAX_VALUE.toULong()
            Kind.U64 -> ULong.MAX_VALUE
            Kind.I8 -> Byte.MIN_VALUE.toLong()
            Kind.I16 -> Short.MIN_VALUE.toLong()
            Kind.I32 -> Int.MIN_VALUE.toLong()
            Kind.I64 -> Long.MIN_VALUE
            // Seventeen digits after `0.00000`: the longest spelling of a number, which only the
            // decimal band from 1e-6 to 1e-5 reaches.
            Kind.F32 -> Float.fromBits(0xb5a5b36dL.toInt()).toDouble()
            Kind.F64 -> -Double.fromBits(0x3eb4b66dc01ec6fbL)
            Kind.BOOL -> false
            Kind.ENUM -> "medium"
            Kind.TEXT -> CONTROLS
            Kind.BYTES -> listOf<Byte>(1, 1, 1)
            // `null` (4 bytes) is longer than any uint8's digits.
            Kind.OPT -> Absent
            Kind.REC -> listOf("a" to 255uL, "b" to CONTROLS, "c" to 255uL)
            Kind.REC_LIST -> listOf("tags" to listOf(CONTROLS, CONTROLS))
        }
    }

    private fun fieldType(id: String): Ty = when (id) {
        "b" -> Ty(Kind.TEXT)
        "tags" -> Ty(null, Ty(Kind.TEXT))
        else -> Ty(Kind.U8)
    }

    /**
     * [held] as JSON, written by the runtime's writers, one call per constructor; a
     * record's members are handed in reverse to show that the writer orders them. A
     * single is written as the double it widens to, as the C11 and Rust writers
     * write it.
     */
    private fun write(ty: Ty, held: Any?): EventPayload.Raw {
        val elem = ty.elem
        if (elem != null) {
            return EventPayload.listJson((held as List<*>).toList()) { write(elem, it) }
        }
        return when (ty.kind!!) {
            Kind.U8 -> EventPayload.valueJson((held as ULong).toUByte())
            Kind.U16 -> EventPayload.valueJson((held as ULong).toUShort())
            Kind.U32 -> EventPayload.valueJson((held as ULong).toUInt())
            Kind.U64 -> EventPayload.valueJson(held as ULong)
            Kind.I8 -> EventPayload.valueJson((held as Long).toByte())
            Kind.I16 -> EventPayload.valueJson((held as Long).toShort())
            Kind.I32 -> EventPayload.valueJson((held as Long).toInt())
            Kind.I64 -> EventPayload.valueJson(held as Long)
            Kind.F32, Kind.F64 -> EventPayload.valueJson(held as Double)
            Kind.BOOL -> EventPayload.valueJson(held as Boolean)
            Kind.ENUM, Kind.TEXT -> EventPayload.valueJson(held as String)
            Kind.BYTES -> EventPayload.valueJson((held as List<*>).map { it as Byte }.toByteArray())
            Kind.OPT -> if (held == Absent) EventPayload.valueJson(null) else EventPayload.valueJson((held as ULong).toUByte())
            Kind.REC, Kind.REC_LIST -> EventPayload.recordJson(
                (held as List<*>).reversed().associate { member ->
                    val (id, value) = member as Pair<*, *>
                    (id as String) to write(fieldType(id), value)
                },
            )
        }
    }

    /** The largest JSON text of [ty] (docs/adr/0086, section 2.1). */
    private fun largest(ty: Ty): Int {
        val list = { size: Int -> 2 + BOUND * size + (BOUND - 1) }
        val record = { fields: List<Pair<String, Int>> -> 2 + fields.sumOf { (id, size) -> id.length + 2 + 1 + size } + fields.size - 1 }
        val elem = ty.elem
        if (elem != null) {
            return list(largest(elem))
        }
        val text = 2 + 6 * TEXT_BOUND
        return when (ty.kind!!) {
            Kind.U8 -> 3
            Kind.U16 -> 5
            Kind.U32 -> 10
            Kind.U64, Kind.I64 -> 22
            Kind.I8 -> 4
            Kind.I16 -> 6
            Kind.I32 -> 11
            Kind.F32, Kind.F64 -> 25
            Kind.BOOL -> 5
            Kind.ENUM -> 2 + "medium".length
            Kind.TEXT, Kind.BYTES -> text
            Kind.OPT -> 4
            Kind.REC -> record(listOf("a" to 3, "b" to text, "c" to 3))
            Kind.REC_LIST -> record(listOf("tags" to list(text)))
        }
    }

    private fun data(list: Any): String = """{"v":$list,"zz":{"unknown":[1,null]}}"""

    /** The path of the innermost list along the last index at each depth. */
    private fun innermost(depth: Int): String = "v" + "/${BOUND - 1}".repeat(depth - 1)

    /** [held] with the element at the end of the last index at every depth replaced by [leaf]. */
    private fun withLastLeaf(ty: Ty, held: Any?, leaf: String): String {
        val elem = ty.elem ?: return leaf
        val items = held as List<*>
        val parts = items.map { write(elem, it).json }.toMutableList()
        parts[parts.size - 1] = withLastLeaf(elem, items.last(), leaf)
        return parts.joinToString(",", "[", "]")
    }

    /** The maximal value with one more element in the list at [level] (1 the outermost) along the last index. */
    private fun onePast(ty: Ty, held: Any?, level: Int): String {
        val elem = ty.elem!!
        val items = held as List<*>
        val parts = items.map { write(elem, it).json }.toMutableList()
        if (level == 1) {
            parts.add(write(elem, items.first()).json)
        } else {
            parts[parts.size - 1] = onePast(elem, items.last(), level - 1)
        }
        return parts.joinToString(",", "[", "]")
    }

    /** [held] with its last leaf replaced by [leaf]; a record, the model's list of `(id, value)` pairs, is a leaf. */
    private fun setLastLeaf(held: Any?, leaf: Any?): Any? {
        if (held !is List<*> || held.firstOrNull() is Pair<*, *>) {
            return leaf
        }
        return held.dropLast(1) + listOf(setLastLeaf(held.last(), leaf))
    }

    /** A bad element of [kind], the path below the element it is refused at, and the sentence. */
    private fun bad(kind: Kind): Triple<String, String, String> = when (kind) {
        Kind.U8, Kind.U16, Kind.U32, Kind.OPT -> Triple("256000000000", "", "does not fit the width")
        Kind.U64 -> Triple("\"-1\"", "", "is not a whole number written as digits")
        Kind.I8, Kind.I16, Kind.I32 -> Triple("1.5", "", "is not a whole number")
        Kind.I64 -> Triple("9007199254740992", "", "past 2^53 - 1")
        Kind.F32 -> Triple("1e39", "", "does not fit the width")
        Kind.F64 -> Triple("true", "", "is not a number")
        Kind.BOOL -> Triple("0", "", "is not a truth value")
        Kind.ENUM -> Triple("\"lowest\"", "", "is not a variant of Level")
        Kind.TEXT -> Triple("\"abcd\"", "", "is 4 bytes, past the 3 its type declares")
        Kind.BYTES -> Triple("\"\\u0100\"", "", "carries a character above U+00FF")
        Kind.REC -> Triple("""{"a":"x","b":""}""", "/a", "is not a number")
        Kind.REC_LIST -> Triple("""{"tags":["","","x"]}""", "/tags", "holds 3 elements, past the 2")
    }

    private fun lift(ty: Ty, text: String): Any? = read(ty, EventPayload.decode(text).field("v"))

    @Test
    fun `every element kind at every depth is read or refused at its path`() {
        val failures = mutableListOf<String>()
        var cases = 0
        fun expect(label: String, want: Want, read: () -> Any?) {
            cases++
            val got = runCatching(read)
            val ok = when (want) {
                is Reads -> got.isSuccess && got.getOrNull() == want.value
                is RefusedAt -> {
                    val refusal = got.exceptionOrNull()
                    refusal is EventPayload.Refusal &&
                        refusal.message.orEmpty().contains("'${want.path}'") &&
                        refusal.message.orEmpty().contains(want.sentence)
                }
            }
            if (!ok) {
                failures.add("$label: read ${got.getOrNull()} (${got.exceptionOrNull()?.message}), the oracle says $want")
            }
        }
        val pastBound = "holds 3 elements, past the 2 its type declares"
        for (kind in Kind.entries) {
            for (depth in 1..3) {
                val ty = nested(kind, depth)
                val full = maximal(ty)
                val leaf = "${innermost(depth)}/${BOUND - 1}"
                val label = { case: String -> "${kind.label} at depth $depth, $case" }
                expect(label("empty"), Reads(emptyList<Any?>())) { lift(ty, data("[]")) }
                expect(label("maximal"), Reads(full)) { lift(ty, data(write(ty, full))) }
                expect(label("one past the innermost bound"), RefusedAt(innermost(depth), pastBound)) {
                    lift(ty, data(onePast(ty, full, depth)))
                }
                expect(label("one past the outermost bound"), RefusedAt("v", pastBound)) {
                    lift(ty, data(onePast(ty, full, 1)))
                }
                if (kind == Kind.OPT) {
                    // The maximal list of optionals is every element absent; a present one is read as present.
                    expect(label("a present element among absent ones"), Reads(setLastLeaf(full, 7uL))) {
                        lift(ty, data(withLastLeaf(ty, full, "7")))
                    }
                } else {
                    expect(label("null where the element is not optional"), RefusedAt(leaf, "is null, and its type is not optional")) {
                        lift(ty, data(withLastLeaf(ty, full, "null")))
                    }
                }
                val (text, below, sentence) = bad(kind)
                expect(label("a bad element"), RefusedAt(leaf + below, sentence)) {
                    lift(ty, data(withLastLeaf(ty, full, text)))
                }
                // A key no schema declares, inside a record element and beside the field, is stepped over.
                val unknown = if (kind == Kind.REC) {
                    withLastLeaf(ty, full, """{"zz":[{"q":1}],"c":255,"b":"\u0001\u0001\u0001","a":255}""")
                } else {
                    write(ty, full).json
                }
                expect(label("an unknown key"), Reads(full)) { lift(ty, data(unknown)) }
                // An older revision: a record element without the field its schema's later revision added
                // reads that field's default, and a payload without a list a later revision added reads it empty.
                if (kind == Kind.REC) {
                    val older = setLastLeaf(full, listOf("a" to 1uL, "b" to "", "c" to 9uL))
                    expect(label("an element of an older revision"), Reads(older)) {
                        lift(ty, data(withLastLeaf(ty, full, """{"a":1,"b":""}""")))
                    }
                } else {
                    expect(label("a delivery of the revision before the list"), Reads(emptyList<Any?>())) {
                        read(ty, EventPayload.decode("""{"zz":1}""", """{"v":[]}""").field("v"))
                    }
                }
            }
        }
        assertEquals(17 * 3 * 8, cases, "the cross product's size")
        assertTrue(failures.isEmpty(), failures.joinToString("\n"))
    }

    @Test
    fun `a maximal value is written at exactly its largest size`() {
        val failures = mutableListOf<String>()
        for (kind in Kind.entries) {
            for (depth in 0..3) {
                val ty = nested(kind, depth)
                val full = maximal(ty)
                val text = write(ty, full).json
                val size = largest(ty)
                if (text.length != size) {
                    failures.add("${kind.label} at depth $depth: ${text.length} bytes, the calculus says $size: $text")
                }
                val back = runCatching { lift(ty, data(text)) }
                if (back.getOrNull() != full) {
                    failures.add("${kind.label} at depth $depth: written and read back as $back")
                }
            }
        }
        assertTrue(failures.isEmpty(), failures.joinToString("\n"))
    }

    @Test
    fun `a top-level text and byte string are held to their bound when read`() {
        val fields = EventPayload.decode("""{"note": "abcde", "blob": "abcde", "t": "éé", "n": null}""")
        assertEquals(
            "'note' is 5 bytes, past the 4 its type declares",
            assertFailsWith<EventPayload.Refusal> { fields.field("note").string(4) }.message,
        )
        assertEquals(
            "'blob' is 5 bytes, past the 4 its type declares",
            assertFailsWith<EventPayload.Refusal> { fields.field("blob").bytes(4) }.message,
        )
        // A text's bound counts UTF-8 bytes: two characters, four bytes.
        assertFailsWith<EventPayload.Refusal> { fields.field("t").string(3) }
        assertEquals("éé", fields.field("t").string(4))
        assertEquals(
            "'n' is null, and its type is not optional",
            assertFailsWith<EventPayload.Refusal> { fields.field("n").int64() }.message,
        )
        assertEquals(
            "the event's data has no 'gone'",
            assertFailsWith<EventPayload.Refusal> { fields.field("gone") }.message,
        )
    }

    @Test
    fun `a list past its bound is refused before an element is read`() {
        val fields = EventPayload.decode("""{"v": [1, 2, "x"], "s": "x"}""")
        var asked = 0
        val refusal = assertFailsWith<EventPayload.Refusal> {
            fields.field("v").list(2) {
                asked++
                it.uint8()
            }
        }
        assertEquals("'v' holds 3 elements, past the 2 its type declares", refusal.message)
        assertEquals(0, asked, "no element was read before the bound was held")
        assertEquals(
            "'s' is not a list",
            assertFailsWith<EventPayload.Refusal> { fields.field("s").list(2) { it.uint8() } }.message,
        )
    }

    @Test
    fun `a record value names its missing field and reads its later fields from defaults`() {
        val fields = EventPayload.decode("""{"r": {"a": 1, "zz": [null]}, "s": 5}""")
        assertEquals(
            "the event's data has no 'r/b'",
            assertFailsWith<EventPayload.Refusal> { fields.field("r").record { it.field("b") } }.message,
        )
        assertEquals(9u.toUByte(), fields.field("r").record("""{"b": 9}""") { it.field("b").uint8() })
        assertEquals(
            "'s' is not a record",
            assertFailsWith<EventPayload.Refusal> { fields.field("s").record { it.field("a") } }.message,
        )
        assertFailsWith<EventPayload.Refusal> { fields.field("r").record("[]") { it.field("a") } }
    }

    @Test
    fun `a list and a record are written in their order and by their key bytes`() {
        assertEquals("[]", EventPayload.listJson(emptyList<Int>()) { EventPayload.valueJson(it) }.json)
        assertEquals("[2,1]", EventPayload.listJson(listOf(2, 1)) { EventPayload.valueJson(it) }.json)
        assertEquals(
            """{"B":3,"aa":1,"b":2}""",
            EventPayload.recordJson(linkedMapOf("b" to 2, "aa" to 1, "B" to 3)).json,
        )
        assertEquals(
            """{"x":[1,{"a":null}]}""",
            EventPayload.encode(mapOf("x" to EventPayload.Raw("""[1,{"a":null}]"""))),
        )
    }

    /** A record a host builds, checked in its `init` at every depth, as the generator writes it. */
    private data class Item(val label: String, val tags: List<String>) {
        init {
            ValueBounds.text("label", label, 8)
            ValueBounds.list("tags", tags, 2) { path, tag -> ValueBounds.text(path, tag, 3) }
        }
    }

    @Test
    fun `a value a host builds is refused where it is made`() {
        assertEquals(listOf("abc", "ab"), Item("eight888", listOf("abc", "ab")).tags)
        assertEquals(
            "'label' is 9 bytes, past the 8 its type declares",
            assertFailsWith<BoundExceeded> { Item("nine99999", emptyList()) }.message,
        )
        assertEquals(
            "'tags' holds 3 elements, past the 2 its type declares",
            assertFailsWith<BoundExceeded> { Item("a", listOf("a", "b", "c")) }.message,
        )
        assertEquals(
            "'tags/1' is 4 bytes, past the 3 its type declares",
            assertFailsWith<BoundExceeded> { Item("a", listOf("a", "abcd")) }.message,
        )
        assertEquals(
            "'blob' is 5 bytes, past the 4 its type declares",
            assertFailsWith<BoundExceeded> { ValueBounds.bytes("blob", ByteArray(5), 4) }.message,
        )
        val checked = mutableListOf<String>()
        ValueBounds.optional("due", null as String?) { path, _ -> checked.add(path) }
        ValueBounds.optional("due", "x") { path, _ -> checked.add(path) }
        assertEquals(listOf("due"), checked, "an absent value holds nothing to check")
    }

    private companion object {
        const val BOUND = 2
        const val TEXT_BOUND = 3
        val ENUM_NAMES = listOf("low", "medium", "high")
        val CONTROLS = "\u0001".repeat(TEXT_BOUND)
    }
}
