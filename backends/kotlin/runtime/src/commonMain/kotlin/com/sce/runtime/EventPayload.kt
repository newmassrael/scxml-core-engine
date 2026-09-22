// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Lifting an event's typed `_event.data` view out of the data it carries.
 *
 * NL→IR Item C1 Path A gives a schema'd event a typed payload that the natively
 * lowered guards read. One producer fills it: the generated `raise<Event>`
 * inject seam. Every other producer — `<send>` with `<param>`, namelist or
 * `<content>`, an invoke forwarding an event either way, autoforward,
 * BasicHTTP, mesh — fills [EventMetadata.data], the wire §scxml-5.10
 * describes and §scxml-B-2-8-1 reads.
 *
 * Until this file the two never met, so the same guard answered differently
 * depending on where its event came from, and a typed payload could not cross
 * an invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE
 * already refuses for the same guard, measured on the same document: no data, a
 * missing field, or a value of another type each give `error.execution` and a
 * guard that does not fire. A native lowering that answered differently would
 * make the optimisation observable, which is the one thing it may not be.
 *
 * ⚠ The reader here is hand-written rather than a JSON library's, because this
 * module has no JSON dependency and its writing half ([StateMachineEngine
 * .valueToJson]) is hand-written for the same reason. The two are a pair: what
 * one writes, the other must read back.
 *
 * Cross-language siblings: `sce_runtime.event_payload` (Python),
 * `sce.LiftPayload` (Go), `SCE::EventPayload` (C++).
 */
object EventPayload {

    /** Why an event's data cannot be read as its schema's fields. */
    class Refusal(message: String) : Exception(message)

    /**
     * The fields an event carries, each value still in the spelling it was
     * written in, so a whole number stays exact until it is read at the width
     * its schema declares.
     */
    class Fields internal constructor(private val values: Map<String, Any?>) {

        private fun raw(name: String): Any? {
            if (!values.containsKey(name)) {
                throw Refusal("the event's data has no '$name'")
            }
            return values[name]
        }

        /**
         * A field's value as the number it was written as.
         *
         * ⚠ A truth value is not a number here. JSON spells both, and a schema
         * that declared `uint32` and received `true` has been handed something
         * its own type says cannot occur.
         */
        private fun number(name: String): String {
            val value = raw(name)
            if (value !is JsonNumber) {
                throw Refusal("'$name' is not a number ($value)")
            }
            return value.text
        }

        private fun whole(name: String): Long {
            val text = number(name)
            return text.toLongOrNull()
                ?: throw Refusal("'$name' is not a whole number ($text)")
        }

        private fun <T> narrowed(name: String, value: Long, min: Long, max: Long, of: (Long) -> T): T {
            if (value < min || value > max) {
                throw Refusal("'$name' does not fit the width its schema declares ($value)")
            }
            return of(value)
        }

        fun int8(name: String): Byte =
            narrowed(name, whole(name), Byte.MIN_VALUE.toLong(), Byte.MAX_VALUE.toLong()) { it.toByte() }

        fun int16(name: String): Short =
            narrowed(name, whole(name), Short.MIN_VALUE.toLong(), Short.MAX_VALUE.toLong()) { it.toShort() }

        fun int32(name: String): Int =
            narrowed(name, whole(name), Int.MIN_VALUE.toLong(), Int.MAX_VALUE.toLong()) { it.toInt() }

        fun int64(name: String): Long = whole(name)

        fun uint8(name: String): UByte =
            narrowed(name, whole(name), 0L, UByte.MAX_VALUE.toLong()) { it.toUByte() }

        fun uint16(name: String): UShort =
            narrowed(name, whole(name), 0L, UShort.MAX_VALUE.toLong()) { it.toUShort() }

        fun uint32(name: String): UInt =
            narrowed(name, whole(name), 0L, UInt.MAX_VALUE.toLong()) { it.toUInt() }

        /**
         * An unsigned 64-bit field. Its top half has no signed spelling, so the
         * text is read as unsigned rather than through [whole].
         */
        fun uint64(name: String): ULong {
            val text = number(name)
            return text.toULongOrNull()
                ?: throw Refusal("'$name' is not a whole number at or above zero ($text)")
        }

        fun float32(name: String): Float {
            val text = number(name)
            return text.toFloatOrNull() ?: throw Refusal("'$name' is not a number ($text)")
        }

        fun float64(name: String): Double {
            val text = number(name)
            return text.toDoubleOrNull() ?: throw Refusal("'$name' is not a number ($text)")
        }

        fun boolean(name: String): Boolean {
            val value = raw(name)
            if (value !is Boolean) {
                throw Refusal("'$name' is not a truth value ($value)")
            }
            return value
        }

        fun string(name: String): String {
            val value = raw(name)
            if (value !is String) {
                throw Refusal("'$name' is not a text ($value)")
            }
            return value
        }

        /**
         * A byte-string field.
         *
         * JSON has no byte string, so the wire carries the byte-exact Latin-1
         * text the inject seam writes, and this reads it back the same way:
         * every one of the 256 values is one character and back. Printable
         * ASCII — what a bytes guard compares — is the same bytes either way.
         */
        fun bytes(name: String): ByteArray {
            val text = string(name)
            val out = ByteArray(text.length)
            for (i in text.indices) {
                val c = text[i].code
                if (c > 0xFF) {
                    throw Refusal(
                        "'$name' carries a character above U+00FF, which no single byte spells")
                }
                out[i] = c.toByte()
            }
            return out
        }
    }

    /** A number kept in the spelling it was written in, so no width is lost. */
    internal class JsonNumber(val text: String) {
        override fun toString(): String = text
    }

    /**
     * The fields the event's data names.
     *
     * The JSON read is §scxml-B-2-8-1's second rung, the same one the script
     * engine takes for the same string.
     */
    fun decode(data: String): Fields {
        val trimmed = data.trim()
        if (trimmed.isEmpty()) {
            throw Refusal("the event carries no data")
        }
        val reader = JsonReader(trimmed)
        val value = reader.readValue()
        reader.skipWhitespace()
        if (!reader.atEnd()) {
            throw Refusal("the event's data carries more than one JSON value")
        }
        @Suppress("UNCHECKED_CAST")
        val fields = value as? Map<String, Any?>
            ?: throw Refusal(
                "the event's data is a bare value, and this event's schema declares named fields")
        return Fields(fields)
    }

    /**
     * The wire spelling of the fields an inject seam was given, so the script
     * engine binds `_event.data` to the same values the typed carrier holds.
     */
    fun encode(fields: Map<String, Any?>): String {
        val parts = fields.entries.joinToString(",") { (key, value) ->
            "${quote(key)}:${literal(value)}"
        }
        return "{$parts}"
    }

    /** A byte string as the Latin-1 text the wire carries (see [Fields.bytes]). */
    fun bytesAsText(bytes: ByteArray): String {
        val chars = CharArray(bytes.size) { (bytes[it].toInt() and 0xFF).toChar() }
        return chars.concatToString()
    }

    private fun literal(value: Any?): String = when (value) {
        null -> "null"
        is Boolean -> value.toString()
        is UByte, is UShort, is UInt, is ULong -> value.toString()
        is Byte, is Short, is Int, is Long -> value.toString()
        is Float, is Double -> value.toString()
        is ByteArray -> quote(bytesAsText(value))
        else -> quote(value.toString())
    }

    private fun quote(text: String): String {
        val sb = StringBuilder(text.length + 2)
        sb.append('"')
        for (c in text) {
            when (c) {
                '"' -> sb.append("\\\"")
                '\\' -> sb.append("\\\\")
                '\n' -> sb.append("\\n")
                '\r' -> sb.append("\\r")
                '\t' -> sb.append("\\t")
                else ->
                    if (c.code < 0x20) {
                        sb.append("\\u").append(c.code.toString(16).padStart(4, '0'))
                    } else {
                        sb.append(c)
                    }
            }
        }
        sb.append('"')
        return sb.toString()
    }

    /**
     * A JSON reader over one payload.
     *
     * It reads the whole grammar rather than only an object of scalars: a
     * payload may carry fields this document's schema does not name, and a
     * nested one still has to be walked past to reach the fields that follow.
     */
    internal class JsonReader(private val text: String) {
        private var at = 0

        fun atEnd(): Boolean = at >= text.length

        fun skipWhitespace() {
            while (at < text.length && text[at].isWhitespace()) at++
        }

        fun readValue(): Any? {
            skipWhitespace()
            if (atEnd()) throw Refusal("the event's data ends where a value was expected")
            return when (val c = text[at]) {
                '{' -> readObject()
                '[' -> readArray()
                '"' -> readString()
                't' -> readKeyword("true", true)
                'f' -> readKeyword("false", false)
                'n' -> readKeyword("null", null)
                else ->
                    if (c == '-' || c in '0'..'9') {
                        readNumber()
                    } else {
                        throw Refusal("the event's data is not JSON (unexpected '$c')")
                    }
            }
        }

        private fun readObject(): Map<String, Any?> {
            at++ // '{'
            val out = LinkedHashMap<String, Any?>()
            skipWhitespace()
            if (!atEnd() && text[at] == '}') {
                at++
                return out
            }
            while (true) {
                skipWhitespace()
                if (atEnd() || text[at] != '"') {
                    throw Refusal("the event's data is not JSON (a field name was expected)")
                }
                val key = readString()
                skipWhitespace()
                if (atEnd() || text[at] != ':') {
                    throw Refusal("the event's data is not JSON (a ':' was expected)")
                }
                at++
                out[key] = readValue()
                skipWhitespace()
                if (atEnd()) throw Refusal("the event's data is not JSON (it ends unclosed)")
                when (text[at]) {
                    ',' -> at++
                    '}' -> { at++; return out }
                    else -> throw Refusal("the event's data is not JSON (a ',' or '}' was expected)")
                }
            }
        }

        private fun readArray(): List<Any?> {
            at++ // '['
            val out = ArrayList<Any?>()
            skipWhitespace()
            if (!atEnd() && text[at] == ']') {
                at++
                return out
            }
            while (true) {
                out.add(readValue())
                skipWhitespace()
                if (atEnd()) throw Refusal("the event's data is not JSON (it ends unclosed)")
                when (text[at]) {
                    ',' -> at++
                    ']' -> { at++; return out }
                    else -> throw Refusal("the event's data is not JSON (a ',' or ']' was expected)")
                }
            }
        }

        private fun readString(): String {
            at++ // '"'
            val sb = StringBuilder()
            while (true) {
                if (atEnd()) throw Refusal("the event's data is not JSON (a text ends unclosed)")
                when (val c = text[at++]) {
                    '"' -> return sb.toString()
                    '\\' -> {
                        if (atEnd()) throw Refusal("the event's data is not JSON (an escape ends the text)")
                        when (val e = text[at++]) {
                            '"' -> sb.append('"')
                            '\\' -> sb.append('\\')
                            '/' -> sb.append('/')
                            'b' -> sb.append('\b')
                            'f' -> sb.append('\u000C')
                            'n' -> sb.append('\n')
                            'r' -> sb.append('\r')
                            't' -> sb.append('\t')
                            'u' -> {
                                if (at + 4 > text.length) {
                                    throw Refusal("the event's data is not JSON (a \\u escape is short)")
                                }
                                val code = text.substring(at, at + 4).toIntOrNull(16)
                                    ?: throw Refusal("the event's data is not JSON (a \\u escape is not hex)")
                                sb.append(code.toChar())
                                at += 4
                            }
                            else -> throw Refusal("the event's data is not JSON (unknown escape '\\$e')")
                        }
                    }
                    else -> sb.append(c)
                }
            }
        }

        private fun readNumber(): JsonNumber {
            val start = at
            if (!atEnd() && text[at] == '-') at++
            while (!atEnd() && text[at] in '0'..'9') at++
            if (!atEnd() && text[at] == '.') {
                at++
                while (!atEnd() && text[at] in '0'..'9') at++
            }
            if (!atEnd() && (text[at] == 'e' || text[at] == 'E')) {
                at++
                if (!atEnd() && (text[at] == '+' || text[at] == '-')) at++
                while (!atEnd() && text[at] in '0'..'9') at++
            }
            val slice = text.substring(start, at)
            if (slice.isEmpty() || slice == "-") {
                throw Refusal("the event's data is not JSON (a number has no digits)")
            }
            return JsonNumber(slice)
        }

        private fun readKeyword(word: String, value: Any?): Any? {
            if (!text.startsWith(word, at)) {
                throw Refusal("the event's data is not JSON (expected '$word')")
            }
            at += word.length
            return value
        }
    }
}
