// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
     * The largest whole number every engine holds exactly as a number (`2^53 - 1`,
     * ECMAScript's `Number.MAX_SAFE_INTEGER`): the largest a JSON number may carry
     * for a 64-bit field.
     */
    private const val EXACT_IN_A_DOUBLE: Long = (1L shl 53) - 1

    /**
     * Whether [text] is a whole number written as digits and nothing else: no sign
     * but `-` (and only where [signed]), no leading zero, no `-0`.
     */
    private fun isDecimal(text: String, signed: Boolean): Boolean {
        val digits = if (text.startsWith("-")) {
            if (!signed) return false
            text.substring(1)
        } else {
            text
        }
        if (digits.isEmpty() || !digits.all { it in '0'..'9' }) return false
        if (digits != "0" && digits.startsWith("0")) return false
        return !(digits == "0" && text.startsWith("-"))
    }

    /**
     * The fields an event carries, each value still in the spelling it was
     * written in, so a whole number stays exact until it is read at the width
     * its schema declares.
     */
    class Fields internal constructor(internal val values: Map<String, Any?>) {

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

        /**
         * A 64-bit whole number's digits: written as a text of them, or as a JSON
         * number when it is exact in a double (`|n| <= 2^53 - 1`). A 64-bit integer
         * crosses as a text because a reader that takes a JSON number through a
         * double carries one past 2^53 with its low bits wrong and no error; a
         * number past that is refused here on every engine, so the same payload
         * gives the same field wherever it is read.
         */
        private fun wide(name: String, signed: Boolean): String =
            when (val value = raw(name)) {
                is String ->
                    if (isDecimal(value, signed)) value
                    else throw Refusal("'$name' is not a whole number written as digits ($value)")
                is Json.Number -> {
                    val n = value.text.toLongOrNull()
                        ?: throw Refusal("'$name' is not a whole number (${value.text})")
                    if (n > EXACT_IN_A_DOUBLE || n < -EXACT_IN_A_DOUBLE) {
                        throw Refusal(
                            "'$name' is a number past 2^53 - 1, which is not exact everywhere " +
                                "(${value.text}); a 64-bit integer that large is written as a " +
                                "text of its digits",
                        )
                    }
                    value.text
                }
                else -> throw Refusal("'$name' is not a number ($value)")
            }

        fun int64(name: String): Long {
            val text = wide(name, true)
            return text.toLongOrNull()
                ?: throw Refusal("'$name' does not fit the width its schema declares ($text)")
        }

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
            val text = wide(name, false)
            return text.toULongOrNull()
                ?: throw Refusal("'$name' does not fit the width its schema declares ($text)")
        }

        /**
         * A 32-bit fractional field: the binary32 nearest the number the payload
         * carries. A JSON number is a double wherever it is read, so the single is
         * the one nearest that double, rounded once, and a number past the largest
         * single does not fit the field, as a whole number past its width does not.
         * Parsing the text to a `Float` directly would round from the text instead,
         * and give infinity for `1e39`.
         */
        fun float32(name: String): Float {
            val text = number(name)
            val wide = text.toDoubleOrNull() ?: throw Refusal("'$name' is not a number ($text)")
            if (kotlin.math.abs(wide) > Float.MAX_VALUE.toDouble()) {
                throw Refusal("'$name' does not fit the width its schema declares ($text)")
            }
            return wide.toFloat()
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

        /**
         * A field of `optional<T>` (docs/adr/0055, section 4): absent where the
         * delivery carries `null`, and otherwise the value [read] takes out of the
         * field, held to its type as any field's is. A delivery that lacks the key
         * is refused, as for every field: a missing key says the sender's revision
         * has no such field, and `null` is the one spelling of absence. Where a
         * later revision added the field, [decode] with its defaults already put
         * the default (`null` when it declares none) in the key's place.
         */
        fun <T : Any> optional(name: String, read: (String) -> T): T? =
            if (raw(name) == null) null else read(name)
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
     * [decode] for a schema a later revision of which added fields: [defaults]
     * is the JSON object of those fields' defaults, which the generator spells
     * from the schema, and a field the delivery lacks is read out of it instead
     * (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").
     *
     * A delivery with no data at all lacks every field, so it is read as one
     * that names none: a field of the first revision is then refused by name,
     * and a schema all of whose fields came later reads its defaults. A field
     * the delivery does carry is read as it came, and held to its type as any
     * other value is.
     */
    fun decode(data: String, defaults: String): Fields {
        val delivered = if (data.isBlank()) emptyMap() else decode(data).values
        val fallback = decode(defaults).values
        return Fields(fallback + delivered)
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
        is UByte, is UShort, is UInt -> value.toString()
        is Byte, is Short, is Int -> value.toString()
        // A 64-bit integer crosses as the text of its digits.
        is ULong, is Long -> Json.quote(value.toString())
        // JSON has no spelling for a double that is not finite. A float32 is
        // still written as the platform spells it: the contract pins the
        // 64-bit spelling (ARCHITECTURE.md, "JSON Number Text").
        is Double -> if (value.isFinite()) Json.numberText(value) else "null"
        is Float -> value.toString()
        is ByteArray -> Json.quote(bytesAsText(value))
        else -> Json.quote(value.toString())
    }
}
