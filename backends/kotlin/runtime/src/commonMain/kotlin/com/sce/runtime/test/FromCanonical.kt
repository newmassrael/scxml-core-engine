// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value read back from the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5; docs/SCE_TTCN3_LANE.md §10.3): the inverse
// of [Canonical], which a recorded input trace's values are read with
// (`schemas/sce-test-trace.v1.schema.json`).
//
// Cross-language sibling: `sce_rust_runtime::test::from_canonical` (Rust),
// whose refusals these name in the same words.

package com.sce.runtime.test

import com.sce.runtime.Json

/**
 * The readers of the canonical form of a value of the profile's types, each
 * value read from the one JSON value model of the runtime (`com.sce.runtime.Json`:
 * a [Map], a [List], a [String], a [Boolean], a [Json.Number] or null).
 *
 * Each reader takes one form and accepts its one spelling, so a value read
 * here and written back by [Canonical] is the same bytes:
 *
 * - an integer is the text of its decimal digits, with no sign but a minus,
 *   no leading zero and no `-0`, within the range of the type it is read as;
 * - a `bool` is a truth value;
 * - a value of an enum is the name its document declares for the variant, or,
 *   for an enum of an open set, the text of a number no variant declares;
 * - a text is a text of at most its bound in UTF-8 bytes;
 * - a byte string is its octets in upper-case hex, two digits each;
 * - a record is an object of its fields by their declared ids, an absent
 *   field omitted, no field it does not declare;
 * - a list is an array of its elements within its bounds, an absent element
 *   of a list of `optional<T>` null;
 * - a value of a union is an object of one member, named by its case's
 *   variant, whose value is the case's record, or `{}` for a case that holds
 *   none.
 *
 * A value that does not belong to its type is refused with a [TraceRefusal],
 * naming where it stands (`what`) and why, never converted. Which form a value
 * takes is decided by the types its document declares and not by its Kotlin
 * type: the generated reader of a record (`read_<record>` in the unit of a test
 * configuration) calls these by the walk its canonical writer is spelled by
 * (`forge::generator::test_template_walk::canonical_form`).
 *
 * A JSON object names each member once, which the runtime's JSON reader holds
 * a text to and a [Map] cannot break, so no reader here refuses a field given
 * twice, as the Rust reader of its list of members must.
 */
object FromCanonical {
    /**
     * An integer type of the profile, as a reader holds the text of its digits
     * to it: [of] is the value the digits spell, or null where the type does
     * not hold it.
     */
    class IntegerType<T : Any> internal constructor(private val narrow: (String) -> T?) {
        /** The value [digits], the canonical text of an integer, spells, or null past the type's range. */
        internal fun of(digits: String): T? = narrow(digits)
    }

    /** An `int8`. */
    val INT8: IntegerType<Byte> = IntegerType { it.toByteOrNull() }

    /** An `int16`. */
    val INT16: IntegerType<Short> = IntegerType { it.toShortOrNull() }

    /** An `int32`. */
    val INT32: IntegerType<Int> = IntegerType { it.toIntOrNull() }

    /** An `int64`. */
    val INT64: IntegerType<Long> = IntegerType { it.toLongOrNull() }

    /** A `uint8`. */
    val UINT8: IntegerType<UByte> = IntegerType { it.toUByteOrNull() }

    /** A `uint16`. */
    val UINT16: IntegerType<UShort> = IntegerType { it.toUShortOrNull() }

    /** A `uint32`. */
    val UINT32: IntegerType<UInt> = IntegerType { it.toUIntOrNull() }

    /** A `uint64`. */
    val UINT64: IntegerType<ULong> = IntegerType { it.toULongOrNull() }

    /** The refusal of the value at [what], for [why]. */
    private fun refused(what: String, why: String): TraceRefusal = TraceRefusal("$what: $why")

    /** The kind of a JSON value, as a refusal names it. */
    private fun kind(value: Any?): String = when (value) {
        null -> "null"
        is Boolean -> "a truth value"
        is Json.Number -> "a number"
        is String -> "a text"
        is List<*> -> "an array"
        is Map<*, *> -> "an object"
        else -> "a value of no JSON kind"
    }

    /** The refusal of [value], at [what], which is not of the form [form]. */
    private fun notA(value: Any?, what: String, form: String): TraceRefusal =
        refused(what, "${kind(value)} is not $form")

    /** The digits of the greatest 128-bit integer. */
    private const val GREATEST_128 = "170141183460469231731687303715884105727"

    /** The digits of the magnitude of the least 128-bit integer, one more than the greatest's. */
    private const val LEAST_128 = "170141183460469231731687303715884105728"

    /**
     * [digits] where it spells an integer as the canonical form writes one: an
     * optional minus and decimal digits with no leading zero, never `-0`,
     * within the range of a 128-bit integer, which the Rust reader of the same
     * text holds it to; null for any other text.
     */
    internal fun decimal(digits: String): String? {
        val negative = digits.startsWith("-")
        val magnitude = if (negative) digits.substring(1) else digits
        val spelled = magnitude.isNotEmpty() &&
            magnitude.all { it in '0'..'9' } &&
            (magnitude == "0" || magnitude[0] != '0') &&
            !(negative && magnitude == "0")
        if (!spelled) {
            return null
        }
        // Two texts of digits of one length, neither with a leading zero, are
        // in the order of their values.
        val bound = if (negative) LEAST_128 else GREATEST_128
        val within = magnitude.length < bound.length ||
            (magnitude.length == bound.length && magnitude <= bound)
        return if (within) digits else null
    }

    /** An integer of the type [type]: the text of its decimal digits. */
    fun <T : Any> integer(value: Any?, type: IntegerType<T>, what: String): T {
        if (value !is String) {
            throw notA(value, what, "an integer, which is the text of its decimal digits")
        }
        val digits = decimal(value)
            ?: throw refused(what, "'$value' is not the text of an integer's decimal digits")
        return type.of(digits) ?: throw refused(what, "$digits is past the range of its type")
    }

    /** A `bool`. */
    fun boolean(value: Any?, what: String): Boolean =
        value as? Boolean ?: throw notA(value, what, "a truth value")

    /** A text of at most [max] bytes of UTF-8, the bound its type declares. */
    fun text(value: Any?, max: Int, what: String): String {
        if (value !is String) {
            throw notA(value, what, "a text")
        }
        val bytes = value.encodeToByteArray().size
        if (bytes > max) {
            throw refused(what, "a text of $bytes bytes is past the $max its type holds")
        }
        return value
    }

    /** The value of one upper-case hex digit, or null. */
    private fun hexDigit(digit: Char): Int? = when (digit) {
        in '0'..'9' -> digit - '0'
        in 'A'..'F' -> digit - 'A' + 10
        else -> null
    }

    /** The octets [text] spells in upper-case hex, two digits each, or null. */
    private fun upperHex(text: String): ByteArray? {
        if (text.length % 2 != 0) {
            return null
        }
        val octets = ByteArray(text.length / 2)
        for (index in octets.indices) {
            val high = hexDigit(text[2 * index]) ?: return null
            val low = hexDigit(text[2 * index + 1]) ?: return null
            octets[index] = ((high shl 4) or low).toByte()
        }
        return octets
    }

    /**
     * A byte string of at most [max] octets: the text of its octets in
     * upper-case hex, two digits each.
     */
    fun bytes(value: Any?, max: Int, what: String): ByteArray {
        if (value !is String) {
            throw notA(value, what, "a byte string, which is a text of hex")
        }
        val octets = upperHex(value)
            ?: throw refused(what, "'$value' is not octets in upper-case hex, two digits each")
        if (octets.size > max) {
            throw refused(what, "a byte string of ${octets.size} octets is past the $max its type holds")
        }
        return octets
    }

    /**
     * A value of an enum of a closed set: the variant whose declared name the
     * text is, from [declared], each variant's name and its value.
     */
    fun <E : Any> enumerated(value: Any?, declared: List<Pair<String, E>>, what: String): E {
        if (value !is String) {
            throw notA(value, what, "a value of an enum, which is its name")
        }
        return declared.firstOrNull { it.first == value }?.second
            ?: throw refused(what, "'$value' names no variant of its enum")
    }

    /**
     * A value of an enum of an open set (`sce:strict-variants="false"`): the
     * variant whose declared name the text is, or the value [fromUnderlying]
     * makes of the number the text spells, of the enum's [underlying] type,
     * where no variant declares that number. A declared number is written by
     * its variant's name, so its digits are refused as the second spelling of
     * one value.
     */
    fun <E : Any, U : Any> openEnumerated(
        value: Any?,
        declared: List<Pair<String, E>>,
        underlying: IntegerType<U>,
        what: String,
        fromUnderlying: (U) -> E,
    ): E {
        if (value !is String) {
            throw notA(value, what, "a value of an enum, which is its name")
        }
        val named = declared.firstOrNull { it.first == value }
        if (named != null) {
            return named.second
        }
        val digits = decimal(value)
            ?: throw refused(what, "'$value' names no variant of its enum, and is no number")
        val raw = underlying.of(digits) ?: throw refused(what, "$value is past the range of its enum")
        val held = fromUnderlying(raw)
        if (declared.any { it.second == held }) {
            throw refused(what, "$value is the number of a declared variant, which is written by its name")
        }
        return held
    }

    /**
     * A record whose fields are [fields]: an object, each member of which names
     * one of them. Returns the object, whose fields [field] and [optionalField]
     * read.
     */
    fun record(value: Any?, fields: List<String>, what: String): Map<*, *> {
        if (value !is Map<*, *>) {
            throw notA(value, what, "a record, which is an object of its fields")
        }
        for (name in value.keys) {
            if (name !in fields) {
                throw refused(what, "'$name' is no field of its record")
            }
        }
        return value
    }

    /**
     * The field [id] of the record [record], whose type is not optional, so it
     * is present: its value, which may be JSON `null`, which the reader of its
     * type refuses.
     */
    fun field(record: Map<*, *>, id: String, what: String): Any? {
        if (!record.containsKey(id)) {
            throw refused(what, "the field is absent, and its type is not optional")
        }
        return record[id]
    }

    /**
     * The field [id] of the record [record], whose type is optional: null where
     * it is absent, which the canonical form writes by omitting it, and the
     * value [read] reads where it is present. A member that is present and
     * `null` is handed to [read], whose reader refuses it, since an absent
     * field is omitted and never written as `null`.
     */
    fun <T : Any> optionalField(record: Map<*, *>, id: String, read: (Any?) -> T): T? =
        if (record.containsKey(id)) read(record[id]) else null

    /** The elements of a list of at least [min] and at most [max] elements. */
    fun elements(value: Any?, min: Int, max: Int, what: String): List<*> {
        if (value !is List<*>) {
            throw notA(value, what, "a list, which is an array")
        }
        if (value.size < min || value.size > max) {
            throw refused(what, "a list of ${value.size} elements is outside its bounds, $min to $max")
        }
        return value
    }

    /**
     * An element of a list of `optional<T>`: null for `null`, which the
     * canonical form writes in the place of an absent element, and the value
     * [read] reads otherwise.
     */
    fun <T : Any> nullable(value: Any?, read: (Any?) -> T): T? =
        if (value == null) null else read(value)

    /** The case a value of a union is in: its [variant] and the [value] of its record. */
    class UnionCase(
        /** The variant of the case, one the union declares. */
        val variant: String,
        /** The case's value: its record's object, or the empty object. */
        val value: Any?,
    )

    /**
     * A value of a union whose cases are [variants]: an object of one member,
     * named by one of them, the variant of its case and the case's value. A
     * variant no case declares is refused here, so a reader that chooses by the
     * variant read has a branch for each declared one and needs none for
     * another.
     */
    fun unionCase(value: Any?, variants: List<String>, what: String): UnionCase {
        if (value !is Map<*, *>) {
            throw notA(value, what, "a value of a union, which is an object of one member")
        }
        if (value.size != 1) {
            throw refused(
                what,
                "a value of a union is an object of one member, and this one has ${value.size}",
            )
        }
        val (variant, case) = value.entries.first()
        if (variant !is String || variant !in variants) {
            throw refused(what, "'$variant' names no case of its union")
        }
        return UnionCase(variant, case)
    }

    /**
     * The value [case] of a case of a union that holds no record, read from
     * [value], which is the empty object.
     */
    fun <C> noRecord(value: Any?, what: String, case: C): C {
        if (value !is Map<*, *> || value.isNotEmpty()) {
            throw refused(what, "a case that holds no record is the empty object")
        }
        return case
    }
}
