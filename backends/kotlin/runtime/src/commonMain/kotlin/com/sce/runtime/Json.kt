// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * The runtime's one JSON reader and writer for typed values.
 *
 * Two things read JSON here: an event's `_event.data` lifted to its typed
 * payload ([EventPayload]) and a machine's saved state read back
 * ([SavedState]). One reader serves both, so a text one accepts and the other
 * refuses cannot exist.
 *
 * A value is a [Map] (an object, in the order its members were written), a
 * [List], a [String], a [Boolean], a [Json.Number] or `null`. A number keeps
 * the spelling it was written in, so a whole number stays exact until the one
 * who reads it chooses its width — a 64-bit value never passes through a
 * `Double`.
 *
 * ⚠ Hand-written rather than a JSON library's: this module has no JSON
 * dependency, and taking one for two small readers would put it on every
 * Android app that links the runtime.
 *
 * Cross-language sibling: `sce_rust_runtime::json` (Rust).
 */
object Json {

    /** Why a text is not one JSON value. */
    sealed class Error(message: String) : Exception(message) {
        /** The text breaks the grammar; the sentence says where. */
        class Malformed(val what: String) : Error("not JSON ($what)")

        /** One value is followed by more text. */
        class Trailing : Error("more than one JSON value")
    }

    /** A number kept in the spelling it was written in, so no width is lost. */
    class Number(val text: String) {
        override fun toString(): String = text
        override fun equals(other: Any?): Boolean = other is Number && other.text == text
        override fun hashCode(): Int = text.hashCode()
    }

    /**
     * Whether [c] is whitespace RFC 8259 has between tokens (section 2): space,
     * tab, line feed or carriage return.
     */
    fun isWhitespace(c: Char): Boolean = c == ' ' || c == '\t' || c == '\n' || c == '\r'

    /** [text] with the whitespace RFC 8259 has trimmed from both ends, and no other. */
    fun trimWhitespace(text: String): String = text.trim { isWhitespace(it) }

    /** Read [text] as exactly one JSON value, surrounding whitespace aside. */
    fun parse(text: String): Any? {
        val reader = Reader(text)
        val value = reader.readValue()
        reader.skipWhitespace()
        if (!reader.atEnd()) throw Error.Trailing()
        return value
    }

    /**
     * [value] as compact JSON text. A number is written in the spelling it
     * carries, so a value read by [parse] is written back unchanged.
     */
    fun write(value: Any?): String = StringBuilder().also { writeInto(value, it, canonical = false) }.toString()

    /**
     * [value] as compact JSON text with every object's members in the one
     * order every engine writes (ARCHITECTURE.md, "JSON Object Key Order"):
     * ascending by [compareKeys], at every depth. An array keeps its order.
     * Otherwise as [write]: a number is written in the spelling it carries.
     */
    fun writeCanonical(value: Any?): String =
        StringBuilder().also { writeInto(value, it, canonical = true) }.toString()

    /**
     * Orders two object keys by Unicode code point, which is the order of
     * their UTF-8 bytes — the order the other five engines sort in.
     *
     * `String.compareTo` compares UTF-16 units and agrees with that order
     * except in one place: a character above U+FFFF is a surrogate pair, whose
     * units (U+D800..U+DFFF) sort BELOW the basic-plane characters
     * U+E000..U+FFFF although the character itself is above them. So where two
     * keys first differ and exactly one of the two units is a surrogate, that
     * one is the greater.
     */
    fun compareKeys(left: String, right: String): Int {
        val shared = minOf(left.length, right.length)
        for (at in 0 until shared) {
            val l = left[at]
            val r = right[at]
            if (l == r) continue
            val leftIsSurrogate = l.code in SURROGATES
            val rightIsSurrogate = r.code in SURROGATES
            return when {
                leftIsSurrogate == rightIsSurrogate -> l.compareTo(r)
                leftIsSurrogate -> 1
                else -> -1
            }
        }
        return left.length - right.length
    }

    private val SURROGATES = 0xD800..0xDFFF

    /** Why a value has no form in the JSON Canonicalization Scheme ([writeJcs]). */
    class NotCanonical(val reason: String) : Exception(reason)

    /**
     * [value] in the JSON Canonicalization Scheme (RFC 8785): no whitespace,
     * the members of each object sorted by their names as sequences of UTF-16
     * code units (section 3.2.3), which is the order `String.compareTo` gives,
     * each text in the one form every engine writes ([quote], which is
     * ECMAScript's `JSON.stringify` of a text: section 3.2.2.2), and each
     * number as ECMAScript spells the double it reads as ([numberText],
     * section 3.2.2.3). Two values equal under the scheme are then written as
     * the same bytes, whatever order their members were built in.
     *
     * It is not [writeCanonical], whose order is the code points' and whose
     * numbers keep the spelling they carry: the two orders differ where a name
     * holds a character above U+FFFF.
     *
     * A [Map] names each member once, which the scheme requires (section
     * 3.1), so only what is no JSON value is refused.
     *
     * Cross-language sibling: `sce_rust_runtime::json::write_canonical` (Rust).
     *
     * @throws NotCanonical for a [Number] that is no JSON number or is past the
     *   range of a double, which the scheme has no spelling for; for an object
     *   member whose name is not a text; and for a value of a type that is no
     *   JSON value.
     */
    fun writeJcs(value: Any?): String = StringBuilder().also { writeJcsInto(value, it) }.toString()

    /** The grammar of a JSON number (RFC 8259 section 6). */
    private val JSON_NUMBER = Regex("-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?")

    private fun writeJcsInto(value: Any?, out: StringBuilder) {
        when (value) {
            null -> out.append("null")
            is Boolean -> out.append(value.toString())
            is Number -> {
                if (!JSON_NUMBER.matches(value.text)) {
                    throw NotCanonical("'${value.text}' is not a number")
                }
                val number = value.text.toDouble()
                if (!number.isFinite()) {
                    throw NotCanonical(
                        "the number ${value.text} is past the range of a double, which RFC 8785 " +
                            "writes every number as",
                    )
                }
                out.append(numberText(number))
            }
            is String -> out.append(quote(value))
            is List<*> -> {
                out.append('[')
                value.forEachIndexed { i, item ->
                    if (i > 0) out.append(',')
                    writeJcsInto(item, out)
                }
                out.append(']')
            }
            is Map<*, *> -> {
                val members = value.entries.map { (key, item) ->
                    val name = key as? String ?: throw NotCanonical(
                        "an object member is named by ${key?.let { it::class.simpleName }}, not a text",
                    )
                    name to item
                }.sortedBy { it.first }
                out.append('{')
                members.forEachIndexed { i, (name, item) ->
                    if (i > 0) out.append(',')
                    out.append(quote(name)).append(':')
                    writeJcsInto(item, out)
                }
                out.append('}')
            }
            else -> throw NotCanonical(
                "${value::class.simpleName} is not a JSON value; wrap a number in Json.Number",
            )
        }
    }

    private fun writeInto(value: Any?, out: StringBuilder, canonical: Boolean) {
        when (value) {
            null -> out.append("null")
            is Boolean -> out.append(value.toString())
            is Number -> out.append(value.text)
            is String -> out.append(quote(value))
            is List<*> -> {
                out.append('[')
                value.forEachIndexed { i, item ->
                    if (i > 0) out.append(',')
                    writeInto(item, out, canonical)
                }
                out.append(']')
            }
            is Map<*, *> -> {
                out.append('{')
                var first = true
                val members =
                    if (canonical) value.entries.sortedWith { l, r -> compareKeys(l.key as String, r.key as String) }
                    else value.entries
                for ((key, item) in members) {
                    if (!first) out.append(',')
                    first = false
                    out.append(quote(key as String)).append(':')
                    writeInto(item, out, canonical)
                }
                out.append('}')
            }
            else -> throw IllegalArgumentException(
                "${value::class.simpleName} is not a JSON value; wrap a number in Json.Number")
        }
    }

    /**
     * The JSON spelling of one text, in the one form every engine writes
     * (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"): `"` and `\`
     * escaped, the five short forms, every other U+0000-U+001F as `\u00xx`
     * in lowercase hex, everything else as it is.
     */
    fun quote(text: String): String {
        val sb = StringBuilder(text.length + 2)
        sb.append('"')
        for (c in text) {
            when (c) {
                '"' -> sb.append("\\\"")
                '\\' -> sb.append("\\\\")
                '\b' -> sb.append("\\b")
                '\u000C' -> sb.append("\\f")
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
     * A 64-bit float as ECMAScript's `Number::toString` spells it, radix 10.
     *
     * The one spelling SCE writes on every engine (ARCHITECTURE.md, "JSON
     * Number Text (Single Source of Truth)"): the fewest digits that read back
     * as the same double, in decimal notation when `1e-6 <= |x| < 1e21` and as
     * `d[.ddd]e[+-]n` otherwise, no fraction on a whole value and `0` for
     * either zero. A value that is not finite is spelled `NaN`, `Infinity` or
     * `-Infinity`, as `String(x)` does; a JSON writer, which has no spelling for
     * those, tests [Double.isFinite] before it calls this.
     * tests/json_text/real_text.json holds the cases.
     *
     * `Double.toString` is neither this layout (`5.0`, `1.0E21`) nor, before
     * JDK 19, always the shortest digits, so the digits come from
     * [shortestDigits] and the layout is the specification's.
     */
    fun numberText(value: Double): String {
        if (value.isNaN()) return "NaN"
        if (value.isInfinite()) return if (value > 0) "Infinity" else "-Infinity"
        if (value == 0.0) return "0"
        val shortest = shortestDigits(kotlin.math.abs(value))
        // The value is `0.<digits> * 10^point`; the specification's `k` and `n`.
        val digits = shortest.digits
        val point = shortest.point
        val k = digits.length
        val sign = if (value < 0) "-" else ""
        return when {
            point in k..21 -> sign + digits + "0".repeat(point - k)
            point in 1..21 -> sign + digits.substring(0, point) + "." + digits.substring(point)
            point in -5..0 -> sign + "0." + "0".repeat(-point) + digits
            else -> {
                val power = point - 1
                val head = digits.substring(0, 1) + if (k > 1) "." + digits.substring(1) else ""
                sign + head + "e" + (if (power < 0) "-" else "+") + kotlin.math.abs(power)
            }
        }
    }

    /**
     * It reads the whole grammar rather than only an object of scalars: a
     * payload may carry fields a document's schema does not name, and a nested
     * one still has to be walked past to reach the fields that follow.
     */
    private class Reader(private val text: String) {
        private var at = 0

        fun atEnd(): Boolean = at >= text.length

        /**
         * Past the whitespace RFC 8259 has between tokens (section 2): space,
         * tab, line feed and carriage return, and no other, so a no-break space
         * or a vertical tab is a text that is not JSON on every runtime
         * (tests/json_text/payload_refusal.json).
         */
        fun skipWhitespace() {
            while (at < text.length && Json.isWhitespace(text[at])) at++
        }

        private fun malformed(what: String): Nothing = throw Error.Malformed(what)

        fun readValue(): Any? {
            skipWhitespace()
            if (atEnd()) malformed("it ends where a value was expected")
            return when (val c = text[at]) {
                '{' -> readObject()
                '[' -> readArray()
                '"' -> readString()
                't' -> readKeyword("true", true)
                'f' -> readKeyword("false", false)
                'n' -> readKeyword("null", null)
                else ->
                    if (c == '-' || c in '0'..'9') readNumber() else malformed("unexpected '$c'")
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
                if (atEnd() || text[at] != '"') malformed("a field name was expected")
                val key = readString()
                // An object whose names repeat has no single meaning: a reader
                // that takes the first and one that takes the last would read
                // two values from one text, so it is refused (the Rust reader
                // refuses it the same way).
                if (out.containsKey(key)) malformed("the field \"$key\" appears twice")
                skipWhitespace()
                if (atEnd() || text[at] != ':') malformed("a ':' was expected")
                at++
                out[key] = readValue()
                skipWhitespace()
                if (atEnd()) malformed("it ends unclosed")
                when (text[at]) {
                    ',' -> at++
                    '}' -> { at++; return out }
                    else -> malformed("a ',' or '}' was expected")
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
                if (atEnd()) malformed("it ends unclosed")
                when (text[at]) {
                    ',' -> at++
                    ']' -> { at++; return out }
                    else -> malformed("a ',' or ']' was expected")
                }
            }
        }

        private fun readString(): String {
            at++ // '"'
            val sb = StringBuilder()
            while (true) {
                if (atEnd()) malformed("a text ends unclosed")
                when (val c = text[at++]) {
                    '"' -> return sb.toString()
                    '\\' -> {
                        if (atEnd()) malformed("an escape ends the text")
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
                                // A character past U+FFFF is written as the two
                                // halves of its UTF-16 pair, each its own escape;
                                // a half alone names no character, so the text is
                                // no JSON text any runtime reads alike (RFC 8259,
                                // sections 7 and 8.2).
                                val code = readHex4()
                                when (code) {
                                    in 0xD800..0xDBFF -> {
                                        if (!text.startsWith("\\u", at)) {
                                            malformed("a \\u escape is half of a character")
                                        }
                                        at += 2
                                        val low = readHex4()
                                        if (low !in 0xDC00..0xDFFF) {
                                            malformed("a \\u escape is half of a character")
                                        }
                                        sb.append(code.toChar()).append(low.toChar())
                                    }
                                    in 0xDC00..0xDFFF -> malformed("a \\u escape is half of a character")
                                    else -> sb.append(code.toChar())
                                }
                            }
                            else -> malformed("unknown escape '\\$e'")
                        }
                    }
                    // RFC 8259, section 7: a character below U+0020 stands in a
                    // text only as an escape.
                    else -> {
                        if (c < ' ') malformed("a control character stands in a text unescaped")
                        sb.append(c)
                    }
                }
            }
        }

        /** The four hex digits after a `\u`, as the number they spell: exactly four, no sign. */
        private fun readHex4(): Int {
            if (at + 4 > text.length) malformed("a \\u escape is short")
            var code = 0
            for (i in at until at + 4) {
                val digit = text[i].digitToIntOrNull(16) ?: malformed("a \\u escape is not hex")
                code = code * 16 + digit
            }
            at += 4
            return code
        }

        /**
         * One number in RFC 8259's grammar (section 6): `-`, then `0` or a digit
         * that is not `0` and more digits, then a fraction and an exponent, each
         * with at least one digit. A leading zero, a fraction or an exponent with
         * no digit, and a sign but `-` are no JSON number.
         */
        private fun readNumber(): Number {
            val start = at
            if (!atEnd() && text[at] == '-') at++
            when {
                !atEnd() && text[at] == '0' -> at++
                !atEnd() && text[at] in '1'..'9' -> skipDigits()
                else -> malformed("a number has no digits")
            }
            if (!atEnd() && text[at] == '.') {
                at++
                if (atEnd() || text[at] !in '0'..'9') malformed("a fraction has no digits")
                skipDigits()
            }
            if (!atEnd() && (text[at] == 'e' || text[at] == 'E')) {
                at++
                if (!atEnd() && (text[at] == '+' || text[at] == '-')) at++
                if (atEnd() || text[at] !in '0'..'9') malformed("an exponent has no digits")
                skipDigits()
            }
            // A digit right after the number is a number with a leading zero
            // (`01`), which no runtime reads alike.
            if (!atEnd() && text[at] in '0'..'9') malformed("a number has a leading zero")
            return Number(text.substring(start, at))
        }

        private fun skipDigits() {
            while (!atEnd() && text[at] in '0'..'9') at++
        }

        private fun readKeyword(word: String, value: Any?): Any? {
            if (!text.startsWith(word, at)) malformed("expected '$word'")
            at += word.length
            return value
        }
    }
}
