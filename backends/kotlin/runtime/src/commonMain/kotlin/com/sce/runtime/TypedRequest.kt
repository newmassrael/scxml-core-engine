// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * A typed host-run request (`sce:request`, SCE Accepted Subset §2.12).
 *
 * A request crosses to the host as text, like every `<param>`. What makes it
 * typed is that each value is held to its field's type where the invocation
 * starts — [wire] — so the text a host reads back is always one its field's
 * type parses. The Kotlin port of `sce_rust_runtime::host_processor`'s
 * typed-request half; the rule is the same in every runtime.
 */
object TypedRequest {

    /** The type one field of a typed request declares, as the generated start site names it. */
    class FieldType private constructor(val kind: String, val cap: Int) {
        companion object {
            val UINT8 = FieldType("uint8", 0)
            val UINT16 = FieldType("uint16", 0)
            val UINT32 = FieldType("uint32", 0)
            val UINT64 = FieldType("uint64", 0)
            val INT8 = FieldType("int8", 0)
            val INT16 = FieldType("int16", 0)
            val INT32 = FieldType("int32", 0)
            val INT64 = FieldType("int64", 0)
            val FLOAT32 = FieldType("float32", 0)
            val FLOAT64 = FieldType("float64", 0)
            val BOOL = FieldType("bool", 0)
            val STRING = FieldType("string", 0)

            /** A `bytes` field of at most [cap] bytes (`sce:max-size`). */
            fun bytes(cap: Int) = FieldType("bytes", cap)
        }
    }

    /** 2^63, the first whole double no signed 64-bit count holds. */
    private const val TWO_63 = 9.223372036854775807E18

    /**
     * The decimal digits of a whole number the data model evaluated, or
     * `null` when it is not one. A double at or past 2^63 is spelled through
     * its half, which is exact: at that magnitude every double is even.
     */
    private fun wholeDigits(value: Any?): String? = when (value) {
        is Byte, is Short, is Int, is Long -> (value as Number).toLong().toString()
        is Float, is Double -> {
            val d = (value as Number).toDouble()
            when {
                d.isNaN() || d.isInfinite() || d != kotlin.math.truncate(d) -> null
                d >= -TWO_63 && d < TWO_63 -> d.toLong().toString()
                d >= TWO_63 && d < 2 * TWO_63 -> ((d / 2).toLong().toULong() * 2u).toString()
                // Beyond every 64-bit count: no width holds it, and this
                // spelling is only what the refusal names.
                else -> d.toString()
            }
        }
        else -> null
    }

    private val SIGNED_BOUNDS = mapOf(
        "int8" to (Byte.MIN_VALUE.toLong() to Byte.MAX_VALUE.toLong()),
        "int16" to (Short.MIN_VALUE.toLong() to Short.MAX_VALUE.toLong()),
        "int32" to (Int.MIN_VALUE.toLong() to Int.MAX_VALUE.toLong()),
        "int64" to (Long.MIN_VALUE to Long.MAX_VALUE),
    )

    private val UNSIGNED_MAX = mapOf(
        "uint8" to UByte.MAX_VALUE.toULong(),
        "uint16" to UShort.MAX_VALUE.toULong(),
        "uint32" to UInt.MAX_VALUE.toULong(),
        "uint64" to ULong.MAX_VALUE,
    )

    /**
     * Hold one evaluated `<param>` to the field it supplies, and spell it for
     * the request.
     *
     * §scxml-6.4.1: an argument that cannot be evaluated starts nothing, and a
     * value the record's field cannot hold is such an argument — the host was
     * promised that record. The refusal is an [EventPayload.Refusal], whose
     * sentence the `error.execution` event carries, because it is the
     * judgement the payload lift makes on a completion that does not fit its
     * record, made on the other half of the same invocation.
     *
     * The text returned is the value at the field's type — a whole number's
     * digits, a fraction as every untyped `<param>` spells one, which is what
     * [fractionalWire] is (the engine's wire spelling, handed in so it is not
     * spelled twice) — and it is what the readers below parse back, so the
     * adapter reading a checked request cannot fail. A byte string rides as
     * its byte-exact Latin-1 text, the spelling a completion's byte field
     * uses.
     */
    fun wire(value: Any?, name: String, type: FieldType, fractionalWire: (Any?) -> String): String {
        val signed = SIGNED_BOUNDS[type.kind]
        val unsignedMax = UNSIGNED_MAX[type.kind]
        if (signed != null || unsignedMax != null) {
            val digits = wholeDigits(value) ?: throw EventPayload.Refusal(
                if (value is Number) "'$name' is not a whole number" else "'$name' is not a number"
            )
            val fits = if (signed != null) {
                digits.toLongOrNull()?.let { it in signed.first..signed.second } ?: false
            } else {
                digits.toULongOrNull()?.let { it <= unsignedMax!! } ?: false
            }
            if (!fits) {
                throw EventPayload.Refusal("'$name' does not fit the width its schema declares ($digits)")
            }
            return digits
        }
        return when (type.kind) {
            "float32", "float64" -> {
                if (value !is Number) {
                    throw EventPayload.Refusal("'$name' is not a number")
                }
                val d = value.toDouble()
                // JSON, which a completion's record crosses as, has no
                // spelling for these, so a request may not carry one either.
                if (d.isNaN() || d.isInfinite()) {
                    throw EventPayload.Refusal("'$name' is not a finite number")
                }
                if (type.kind == "float32") {
                    val narrowed = d.toFloat()
                    if (narrowed.isInfinite()) {
                        throw EventPayload.Refusal("'$name' does not fit the width its schema declares")
                    }
                    fractionalWire(narrowed.toDouble())
                } else {
                    fractionalWire(d)
                }
            }
            "bool" -> (value as? Boolean)?.toString()
                ?: throw EventPayload.Refusal("'$name' is not a truth value")
            "string" -> value as? String ?: throw EventPayload.Refusal("'$name' is not a text")
            "bytes" -> {
                val text = value as? String ?: throw EventPayload.Refusal("'$name' is not a byte string")
                if (text.any { it.code > 0xFF }) {
                    throw EventPayload.Refusal(
                        "'$name' carries a character above U+00FF, which no single byte spells"
                    )
                }
                if (text.length > type.cap) {
                    throw EventPayload.Refusal(
                        "'$name' is ${text.length} bytes, past the ${type.cap} its schema declares"
                    )
                }
                text
            }
            else -> error("TypedRequest.wire: unknown field type '${type.kind}'")
        }
    }

    /**
     * The one text a checked typed request carries for [name].
     *
     * Throws [IllegalStateException] when there is not exactly one, or when
     * the text does not parse at its type: a request that reached a typed
     * adapter was checked field by field where it started, so this is a
     * broken promise between two halves of generated code, not a value a host
     * or document can supply — and one that would otherwise hand the host a
     * record the document never sent.
     */
    private fun text(params: Map<String, List<String>>, invokeId: String, name: String): String {
        val values = params[name].orEmpty()
        check(values.size == 1) {
            "typed request '$invokeId' does not carry '$name' exactly once, though its record declares it"
        }
        return values[0]
    }

    private fun <T : Any> parsed(
        params: Map<String, List<String>>,
        invokeId: String,
        name: String,
        parse: (String) -> T?,
    ): T {
        val text = text(params, invokeId, name)
        return checkNotNull(parse(text)) {
            "typed request '$invokeId' carries '$name' as '$text', which its type does not parse, " +
                "though the start site checked it"
        }
    }

    fun uint8(params: Map<String, List<String>>, invokeId: String, name: String): UByte =
        parsed(params, invokeId, name, String::toUByteOrNull)

    fun uint16(params: Map<String, List<String>>, invokeId: String, name: String): UShort =
        parsed(params, invokeId, name, String::toUShortOrNull)

    fun uint32(params: Map<String, List<String>>, invokeId: String, name: String): UInt =
        parsed(params, invokeId, name, String::toUIntOrNull)

    fun uint64(params: Map<String, List<String>>, invokeId: String, name: String): ULong =
        parsed(params, invokeId, name, String::toULongOrNull)

    fun int8(params: Map<String, List<String>>, invokeId: String, name: String): Byte =
        parsed(params, invokeId, name, String::toByteOrNull)

    fun int16(params: Map<String, List<String>>, invokeId: String, name: String): Short =
        parsed(params, invokeId, name, String::toShortOrNull)

    fun int32(params: Map<String, List<String>>, invokeId: String, name: String): Int =
        parsed(params, invokeId, name, String::toIntOrNull)

    fun int64(params: Map<String, List<String>>, invokeId: String, name: String): Long =
        parsed(params, invokeId, name, String::toLongOrNull)

    fun float32(params: Map<String, List<String>>, invokeId: String, name: String): Float =
        parsed(params, invokeId, name, String::toFloatOrNull)

    fun float64(params: Map<String, List<String>>, invokeId: String, name: String): Double =
        parsed(params, invokeId, name, String::toDoubleOrNull)

    fun boolean(params: Map<String, List<String>>, invokeId: String, name: String): Boolean =
        parsed(params, invokeId, name, String::toBooleanStrictOrNull)

    fun string(params: Map<String, List<String>>, invokeId: String, name: String): String =
        text(params, invokeId, name)

    fun bytes(params: Map<String, List<String>>, invokeId: String, name: String): ByteArray =
        parsed(params, invokeId, name) { text ->
            if (text.any { it.code > 0xFF }) null
            else ByteArray(text.length) { text[it].code.toByte() }
        }
}
