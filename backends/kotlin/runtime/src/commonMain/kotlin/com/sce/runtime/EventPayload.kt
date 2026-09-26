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
 * The JSON is read through the runtime's one reader ([Json]); what the inject
 * seam writes goes through its [Json.quote].
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
            if (value !is Json.Number) {
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

    /**
     * The fields the event's data names.
     *
     * The JSON read is §scxml-B-2-8-1's second rung, the same one the script
     * engine takes for the same string — through the runtime's one reader.
     */
    fun decode(data: String): Fields {
        val trimmed = data.trim()
        if (trimmed.isEmpty()) {
            throw Refusal("the event carries no data")
        }
        if (!trimmed.startsWith('{')) {
            throw Refusal(
                "the event's data is a bare value, and this event's schema declares named fields")
        }
        val value = try {
            Json.parse(trimmed)
        } catch (e: Json.Error.Trailing) {
            throw Refusal("the event's data carries more than one JSON value")
        } catch (e: Json.Error.Malformed) {
            throw Refusal("the event's data is not JSON (${e.what})")
        }
        @Suppress("UNCHECKED_CAST")
        return Fields(value as Map<String, Any?>)
    }

    /**
     * The wire spelling of the fields an inject seam was given, so the script
     * engine binds `_event.data` to the same values the typed carrier holds.
     */
    fun encode(fields: Map<String, Any?>): String {
        val parts = fields.entries.joinToString(",") { (key, value) ->
            "${Json.quote(key)}:${literal(value)}"
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
        is ByteArray -> Json.quote(bytesAsText(value))
        else -> Json.quote(value.toString())
    }
}
