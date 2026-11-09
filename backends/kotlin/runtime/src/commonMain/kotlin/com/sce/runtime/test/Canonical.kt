// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The canonical form of a message's value in the test log
// (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3 and answer 8): one JSON
// value per value of the profile's types, built from texts, truth values, lists
// and maps alone, so that the JSON Canonicalization Scheme (`Json.writeJcs`)
// writes two equal values as the same bytes on every runtime.
//
// Cross-language sibling: `sce_rust_runtime::test::canonical` (Rust).

package com.sce.runtime.test

/**
 * The writers of the canonical form of a value of the profile's types, each
 * value a [String], a [Boolean], a [List] or a [Map] of them, as the runtime's
 * JSON writer reads one (`com.sce.runtime.Json`):
 *
 * - an integer of any width is the text of its decimal digits, since an
 *   `int64` or a `uint64` is past what a JSON reader that holds numbers as
 *   doubles reads exactly;
 * - a `bool` is a truth value;
 * - a value of an enum is the name its document declares for its variant;
 * - a text is a text;
 * - a byte string is the text of its octets in upper-case hex, two digits each;
 * - a record is a map of its fields by their declared ids, a field that is
 *   absent omitted, as TTCN-3's `omit` leaves it out;
 * - a list is a list of its elements in order, an element of `optional<T>`
 *   that is absent null, the one value no other form takes;
 * - a value of a union is a map of one member, named by its case's variant,
 *   whose value is the case's record, or the empty map for a case that holds
 *   none.
 *
 * The generated unit of a test template over a record spells its `canonical`
 * with these, by the types its document declares
 * (`forge/kotlin/test_template.kt.jinja2`): the form is the profile type's,
 * not the language's, so it is written from the document's types on every
 * backend alike, where a byte string and a list of `uint8` are one type in
 * some languages and two in others.
 */
object Canonical {
    /** An `int8`: the text of its decimal digits. */
    fun integer(value: Byte): String = value.toString()

    /** An `int16`: the text of its decimal digits. */
    fun integer(value: Short): String = value.toString()

    /** An `int32`: the text of its decimal digits. */
    fun integer(value: Int): String = value.toString()

    /** An `int64`: the text of its decimal digits. */
    fun integer(value: Long): String = value.toString()

    /** A `uint8`: the text of its decimal digits. */
    fun integer(value: UByte): String = value.toString()

    /** A `uint16`: the text of its decimal digits. */
    fun integer(value: UShort): String = value.toString()

    /** A `uint32`: the text of its decimal digits. */
    fun integer(value: UInt): String = value.toString()

    /** A `uint64`: the text of its decimal digits. */
    fun integer(value: ULong): String = value.toString()

    /** A `bool`. */
    fun boolean(value: Boolean): Boolean = value

    /** A text. */
    fun text(value: String): String = value

    /**
     * A byte string: its octets in upper-case hex, two digits each, as TTCN-3
     * writes an `octetstring` (`'00FF'O`) without its quotes and its `O`.
     */
    fun bytes(value: ByteArray): String {
        val out = StringBuilder(value.size * 2)
        for (octet in value) {
            val unsigned = octet.toInt() and 0xFF
            out.append(HEX[unsigned shr 4]).append(HEX[unsigned and 0x0F])
        }
        return out.toString()
    }

    private const val HEX = "0123456789ABCDEF"

    /** A value of a closed enum: the name its document declares for the variant. */
    fun enumerated(declared: String): String = declared

    /**
     * A value of an enum of an open set (`sce:strict-variants="false"`): the
     * name its document declares for it, or, for a number no variant declares,
     * [number], the text of that number's decimal digits ([integer]). No
     * variant's name is such a text, since a name is an identifier. The TTCN-3
     * export refuses an enum of an open set (docs/SCE_TEST_PROFILE.md §9.5),
     * so only a native log holds this form.
     */
    fun openEnumerated(declared: String?, number: String): String = declared ?: number

    /**
     * A record: each field by its declared id, in any order, since the scheme
     * sorts a map's members; a field given as null is absent and omitted.
     */
    fun record(vararg fields: Pair<String, Any?>): Map<String, Any> {
        val members = LinkedHashMap<String, Any>()
        for ((id, value) in fields) {
            if (value != null) {
                members[id] = value
            }
        }
        return members
    }

    /**
     * A list: its elements in order, an absent element of a list of
     * `optional<T>` null in its place ([optionalElement]).
     */
    fun list(elements: List<Any?>): List<Any?> = elements.toList()

    /**
     * An element of a list of `optional<T>`: the value [held] holds, in the
     * form [write] gives it, or null, which `Json.writeJcs` writes as JSON
     * `null`, where it holds none. An element is not omitted as an absent field
     * is, since the array names it by its position, which leaving it out would
     * move for every element after it. The TTCN-3 export refuses a list of
     * `optional<T>` (rule E1, docs/SCE_TEST_PROFILE.md §6.3), so only a native
     * log holds this form.
     */
    fun <T : Any> optionalElement(held: T?, write: (T) -> Any): Any? =
        if (held == null) null else write(held)

    /**
     * A value of a union: one member, named by the variant of its case,
     * holding the case's record, or the empty map for a case that holds none.
     */
    fun unionCase(variant: String, record: Any?): Map<String, Any> =
        mapOf(variant to (record ?: emptyMap<String, Any>()))
}
