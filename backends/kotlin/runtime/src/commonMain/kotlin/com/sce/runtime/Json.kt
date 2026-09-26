// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    fun write(value: Any?): String = StringBuilder().also { writeInto(value, it) }.toString()

    private fun writeInto(value: Any?, out: StringBuilder) {
        when (value) {
            null -> out.append("null")
            is Boolean -> out.append(value.toString())
            is Number -> out.append(value.text)
            is String -> out.append(quote(value))
            is List<*> -> {
                out.append('[')
                value.forEachIndexed { i, item ->
                    if (i > 0) out.append(',')
                    writeInto(item, out)
                }
                out.append(']')
            }
            is Map<*, *> -> {
                out.append('{')
                var first = true
                for ((key, item) in value) {
                    if (!first) out.append(',')
                    first = false
                    out.append(quote(key as String)).append(':')
                    writeInto(item, out)
                }
                out.append('}')
            }
            else -> throw IllegalArgumentException(
                "${value::class.simpleName} is not a JSON value; wrap a number in Json.Number")
        }
    }

    /** The JSON spelling of one text. */
    fun quote(text: String): String {
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
     * It reads the whole grammar rather than only an object of scalars: a
     * payload may carry fields a document's schema does not name, and a nested
     * one still has to be walked past to reach the fields that follow.
     */
    private class Reader(private val text: String) {
        private var at = 0

        fun atEnd(): Boolean = at >= text.length

        fun skipWhitespace() {
            while (at < text.length && text[at].isWhitespace()) at++
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
                                if (at + 4 > text.length) malformed("a \\u escape is short")
                                val code = text.substring(at, at + 4).toIntOrNull(16)
                                    ?: malformed("a \\u escape is not hex")
                                sb.append(code.toChar())
                                at += 4
                            }
                            else -> malformed("unknown escape '\\$e'")
                        }
                    }
                    else -> sb.append(c)
                }
            }
        }

        private fun readNumber(): Number {
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
            if (slice.isEmpty() || slice == "-") malformed("a number has no digits")
            return Number(slice)
        }

        private fun readKeyword(word: String, value: Any?): Any? {
            if (!text.startsWith(word, at)) malformed("expected '$word'")
            at += word.length
            return value
        }
    }
}
