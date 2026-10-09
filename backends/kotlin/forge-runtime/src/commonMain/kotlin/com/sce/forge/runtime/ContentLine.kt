// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The content lines (RFC 5545 §3.1) a `sce:encoding="content-line"` codec reads
 * and writes (SCE_FORGE.md §4.6.4, docs/adr/0010): `BEGIN:<component>`,
 * properties — a name, `;`-separated parameters, `:` and a value — and
 * `END:<component>`, folded at 75 octets.
 *
 * A generated codec calls these and spells no line grammar of its own. The
 * rules are written once in SCE_FORGE.md §4.6.4 and implemented once per
 * backend runtime; this is Kotlin's, the sibling of `sce_forge_runtime::
 * content_line` (Rust). **Reading** answers `null` for what the codec cannot
 * read — Kotlin's codec convention, where every decode failure is `null`;
 * [lastError] says which rule refused. **Writing** answers a [CodecError] or
 * `null` on success, as every Kotlin encode does.
 */
object ContentLine {
    /** The most octets of one physical line (SCE_FORGE.md §4.6.4, *Folding*). */
    const val FOLD_WIDTH = 75

    /**
     * Why the last read or write answered a failure. Diagnostic only — a
     * decode's contract is its `null` — and read by single-threaded tests that
     * pin which rule refused; it is not synchronised, so concurrent codecs
     * overwrite it.
     */
    var lastError: CodecError? = null
        internal set

    internal fun <T> refuse(error: CodecError): T? {
        lastError = error
        return null
    }

    internal fun fail(error: CodecError): CodecError {
        lastError = error
        return error
    }

    /** A control character a value never holds: below U+0020 but the tab, and U+007F. */
    internal fun isControl(b: Int): Boolean = (b < 0x20 && b != 0x09) || b == 0x7F

    /** A byte of a property or parameter name: letters, digits and hyphens. */
    internal fun isNameByte(b: Int): Boolean =
        (b in 0x30..0x39) || (b in 0x41..0x5A) || (b in 0x61..0x7A) || b == 0x2D

    internal fun lower(b: Int): Int = if (b in 0x41..0x5A) b + 0x20 else b

    /** Whether the unfolded text of `raw[from, to)` is [expected], without regard to case. */
    internal fun unfoldedEq(raw: ByteArray, from: Int, to: Int, expected: String): Boolean {
        val scan = Scan(raw, to, from)
        for (ch in expected) {
            val b = scan.bump()
            if (b < 0 || lower(b) != lower(ch.code)) return false
        }
        return scan.peek() < 0
    }
}

private const val CR: Byte = 13
private const val LF: Byte = 10
private const val SPACE: Byte = 32
private const val TAB: Byte = 9

/**
 * A walk over the bytes of one logical line, `raw[pos, limit)`, with its folds
 * removed as it goes: a line break and the one space or tab after it are not
 * part of the text, wherever the sender cut.
 */
internal class Scan(private val raw: ByteArray, private val limit: Int, var pos: Int) {
    private fun blank(i: Int): Boolean = i < limit && (raw[i] == SPACE || raw[i] == TAB)

    /** Step over every fold at the current position. */
    fun skipFolds() {
        while (true) {
            val skipped = when {
                pos + 1 < limit && raw[pos] == CR && raw[pos + 1] == LF && blank(pos + 2) -> 3
                pos < limit && raw[pos] == LF && blank(pos + 1) -> 2
                else -> 0
            }
            if (skipped == 0) return
            pos += skipped
        }
    }

    /** The next byte as 0..255 without taking it, or -1 at the end of the line. */
    fun peek(): Int {
        skipFolds()
        return if (pos < limit) raw[pos].toInt() and 0xFF else -1
    }

    fun bump(): Int {
        val b = peek()
        if (b >= 0) pos++
        return b
    }
}

/** One logical line of the input, with its folds still inside it. */
private class RawLine(val from: Int, val to: Int, val terminated: Boolean)

/** The name of a line and what follows it, as indexes into the input. */
private class Head(val nameEnd: Int, val separator: Int, val restStart: Int)

private fun headOf(raw: ByteArray, from: Int, to: Int): Head? {
    val scan = Scan(raw, to, from)
    var named = false
    while (true) {
        val b = scan.peek()
        if (b >= 0 && ContentLine.isNameByte(b)) {
            scan.bump()
            named = true
        } else {
            break
        }
    }
    if (!named) return null
    val nameEnd = scan.pos
    val separator = scan.peek()
    if (separator != ';'.code && separator != ':'.code) return null
    scan.bump()
    return Head(nameEnd, separator, scan.pos)
}

// ── Reading ─────────────────────────────────────────────────────────────

/** A reader over the properties of one component. */
class ContentLineReader private constructor(
    private val input: ByteArray,
    private val component: String,
    private var pos: Int = 0,
) {
    /** How many nested components the walk is inside (`VALARM` in `VEVENT`). */
    private var depth = 0

    /**
     * Whether [next] answered `null` because the reader refused, and not because
     * the component ended.
     */
    var failed = false
        private set

    /**
     * How many bytes of the input the walk has passed — after
     * `END:<component>` once [next] has answered `null` without [failed].
     */
    fun consumed(): Int = pos

    private fun nextRawLine(): RawLine? {
        if (pos >= input.size) return null
        val start = pos
        var i = start
        while (i < input.size) {
            val folded = i + 1 < input.size && (input[i + 1] == SPACE || input[i + 1] == TAB)
            if (input[i] == LF && !folded) {
                val end = if (i > start && input[i - 1] == CR) i - 1 else i
                pos = i + 1
                return RawLine(start, end, true)
            }
            i++
        }
        pos = input.size
        return RawLine(start, input.size, false)
    }

    private fun refuse(error: CodecError): ContentLineProperty? {
        failed = true
        return ContentLine.refuse(error)
    }

    /**
     * The next property line of the component, or `null` after its
     * `END:<component>` — or, with [failed] set, when the reader refused.
     *
     * A nested component is skipped through its `END:` without reading its lines.
     * An input that ends before `END:<component>` is [CodecError.NeedMoreBytes].
     */
    fun next(): ContentLineProperty? {
        while (true) {
            val line = nextRawLine() ?: return refuse(CodecError.NeedMoreBytes)
            val head = headOf(input, line.from, line.to)
            if (depth > 0) {
                if (head != null && head.separator == ':'.code) {
                    if (ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN")) {
                        depth++
                    } else if (ContentLine.unfoldedEq(input, line.from, head.nameEnd, "END")) {
                        depth--
                    }
                }
                continue
            }
            // A line with no name is cut short at the end of the input and
            // malformed anywhere else.
            val cut = if (line.terminated) CodecError.LineMalformed else CodecError.NeedMoreBytes
            if (head == null) return refuse(cut)
            if (head.separator == ':'.code) {
                if (ContentLine.unfoldedEq(input, line.from, head.nameEnd, "END")) {
                    if (ContentLine.unfoldedEq(input, head.restStart, line.to, component)) return null
                    return refuse(cut)
                }
                if (line.terminated && ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN")) {
                    depth = 1
                    continue
                }
            }
            if (!line.terminated) return refuse(CodecError.NeedMoreBytes)
            return ContentLineProperty(input, line.from, line.to, head.nameEnd)
        }
    }

    companion object {
        /**
         * Skip lines up to the first `BEGIN:<component>` and stand after it.
         * `null` when the input ends before it ([CodecError.NeedMoreBytes]).
         */
        fun begin(input: ByteArray, component: String): ContentLineReader? {
            val reader = ContentLineReader(input, component)
            while (true) {
                val line = reader.nextRawLine() ?: break
                if (!line.terminated) break
                val head = headOf(input, line.from, line.to) ?: continue
                if (head.separator == ':'.code &&
                    ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN") &&
                    ContentLine.unfoldedEq(input, head.restStart, line.to, component)
                ) {
                    return reader
                }
            }
            return ContentLine.refuse(CodecError.NeedMoreBytes)
        }
    }
}

/**
 * One property line the codec has been handed.
 *
 * Ask [isNamed] whether it is one the codec reads. Then, for a property that
 * declares parameters, loop on [nextParam] and read the ones the codec declares;
 * then read the value. A parameter no one reads is skipped by the next call, and
 * a property read by value alone skips them all. Every read answers `null` for
 * what the entry cannot hold.
 */
class ContentLineProperty internal constructor(
    private val raw: ByteArray,
    @Suppress("unused") private val from: Int,
    to: Int,
    private val nameEnd: Int,
) {
    private val scan = Scan(raw, to, nameEnd)
    private var paramFrom = -1
    private var paramTo = -1
    private var phase = Phase.AT_SEPARATOR

    /** Where the property stands in its line. */
    private enum class Phase {
        /** At the `;` that opens a parameter or the `:` that opens the value. */
        AT_SEPARATOR,

        /** After a parameter's `=`, before its value. */
        PARAM_VALUE,

        /** The value has been read. */
        DONE,
    }

    /** Whether this property is [name], without regard to case. */
    fun isNamed(name: String): Boolean = ContentLine.unfoldedEq(raw, from, nameEnd, name)

    /** Whether the parameter [nextParam] stands on is [name], without regard to case. */
    fun paramNamed(name: String): Boolean =
        paramFrom >= 0 && ContentLine.unfoldedEq(raw, paramFrom, paramTo, name)

    /**
     * Stand on the next parameter, skipping the value of one not read: `true`
     * when there is one, `false` when the value is next, `null` when the line
     * does not admit it.
     */
    fun nextParam(): Boolean? {
        when (phase) {
            Phase.PARAM_VALUE -> if (!skipParamValue()) return null
            Phase.DONE -> return ContentLine.refuse(CodecError.LineMalformed)
            Phase.AT_SEPARATOR -> {}
        }
        return when (scan.peek()) {
            ':'.code -> false
            ';'.code -> {
                scan.bump()
                scan.peek()
                val start = scan.pos
                while (true) {
                    val b = scan.peek()
                    if (b >= 0 && ContentLine.isNameByte(b)) scan.bump() else break
                }
                val end = scan.pos
                if (end == start || scan.bump() != '='.code) {
                    return ContentLine.refuse(CodecError.LineMalformed)
                }
                paramFrom = start
                paramTo = end
                phase = Phase.PARAM_VALUE
                true
            }
            else -> ContentLine.refuse(CodecError.LineMalformed)
        }
    }

    /**
     * Scan one parameter value — a quoted string, or text up to `;`, `:`, `,`
     * or `"` — handing each byte of it to [emit] (`null`: skipped). Answers 1
     * when another value follows a `,`, 0 when none does, -1 when refused.
     */
    private fun scanParamValue(emit: ((Int) -> Boolean)?): Int {
        if (scan.peek() == '"'.code) {
            scan.bump()
            while (true) {
                val b = scan.bump()
                if (b < 0) return refuseScan(CodecError.LineMalformed)
                if (b == '"'.code) break
                if (emit != null && !emit(b)) return -1
            }
        } else {
            while (true) {
                val b = scan.peek()
                if (b < 0 || b == ';'.code || b == ':'.code || b == ','.code) break
                if (b == '"'.code) return refuseScan(CodecError.LineMalformed)
                scan.bump()
                if (emit != null && !emit(b)) return -1
            }
        }
        return when (scan.peek()) {
            ','.code -> {
                scan.bump()
                1
            }
            ';'.code, ':'.code -> 0
            else -> refuseScan(CodecError.LineMalformed)
        }
    }

    private fun refuseScan(error: CodecError): Int {
        ContentLine.lastError = error
        return -1
    }

    private fun skipParamValue(): Boolean {
        while (true) {
            when (scanParamValue(null)) {
                1 -> continue
                0 -> break
                else -> return false
            }
        }
        paramFrom = -1
        phase = Phase.AT_SEPARATOR
        return true
    }

    /**
     * Read the value of the parameter [nextParam] stands on, into at most
     * [maxSize] bytes. A second value is [CodecError.LineBadValue].
     */
    fun readParamString(maxSize: Int): String? {
        if (phase != Phase.PARAM_VALUE) return ContentLine.refuse(CodecError.LineMalformed)
        val buf = ByteArray(maxSize)
        var n = 0
        val more = scanParamValue { b ->
            when {
                ContentLine.isControl(b) -> {
                    ContentLine.lastError = CodecError.LineBadValue
                    false
                }
                n == maxSize -> {
                    ContentLine.lastError = CodecError.LineTooLong
                    false
                }
                else -> {
                    buf[n++] = b.toByte()
                    true
                }
            }
        }
        if (more < 0) return null
        if (more > 0) return ContentLine.refuse(CodecError.LineBadValue)
        paramFrom = -1
        phase = Phase.AT_SEPARATOR
        return utf8(buf, n)
    }

    private fun utf8(buf: ByteArray, n: Int): String? =
        try {
            buf.decodeToString(0, n, throwOnInvalidSequence = true)
        } catch (_: CharacterCodingException) {
            ContentLine.refuse(CodecError.LineBadValue)
        }

    /** Stand at the value: skip the parameters still unread and step over `:`. */
    private fun beginValue(): Boolean {
        while (true) {
            when (nextParam()) {
                true -> continue
                false -> break
                null -> return false
            }
        }
        scan.bump()
        phase = Phase.DONE
        return true
    }

    /**
     * Read the value as a `string` of at most [maxSize] bytes. With [text],
     * `\\`, `\;`, `\,`, `\n` and `\N` are escapes; an unescaped `;` or `,` is itself.
     */
    fun readString(maxSize: Int, text: Boolean): String? {
        if (!beginValue()) return null
        val buf = ByteArray(maxSize)
        var n = 0
        while (true) {
            val b = scan.bump()
            if (b < 0) break
            val byte: Int
            if (text && b == '\\'.code) {
                byte = when (scan.bump()) {
                    '\\'.code -> '\\'.code
                    ';'.code -> ';'.code
                    ','.code -> ','.code
                    'n'.code, 'N'.code -> 0x0A
                    else -> return ContentLine.refuse(CodecError.LineBadEscape)
                }
            } else if (ContentLine.isControl(b)) {
                return ContentLine.refuse(CodecError.LineBadValue)
            } else {
                byte = b
            }
            if (n == maxSize) return ContentLine.refuse(CodecError.LineTooLong)
            buf[n++] = byte.toByte()
        }
        return utf8(buf, n)
    }

    /**
     * Read the value as a list of `string` cut at [separator], at most
     * [maxValues] of them and each at most [maxSize] bytes (docs/adr/0014). The
     * value is cut before it is unescaped: with [text] a separator that a
     * backslash precedes is part of the value, and with anything else every
     * separator cuts. The parts are judged left to right and the first failure is
     * the line's: a part past [maxValues] is [CodecError.LineTooMany], even one
     * that is empty or too long; an empty part is [CodecError.LineBadValue].
     */
    fun readStrings(separator: Char, maxValues: Int, maxSize: Int, text: Boolean): MutableList<String>? {
        if (!beginValue()) return null
        val values = mutableListOf<String>()
        val buf = ByteArray(maxSize)
        var n = 0
        while (true) {
            val b = scan.bump()
            if (b < 0 || b == separator.code) {
                if (n == 0) return ContentLine.refuse(CodecError.LineBadValue)
                values.add(utf8(buf, n) ?: return null)
                if (b < 0) return values
                n = 0
                // A separator opens another part, which may not pass the bound.
                if (values.size >= maxValues) return ContentLine.refuse(CodecError.LineTooMany)
                continue
            }
            val byte: Int
            if (text && b == '\\'.code) {
                byte = when (scan.bump()) {
                    '\\'.code -> '\\'.code
                    ';'.code -> ';'.code
                    ','.code -> ','.code
                    'n'.code, 'N'.code -> 0x0A
                    else -> return ContentLine.refuse(CodecError.LineBadEscape)
                }
            } else if (ContentLine.isControl(b)) {
                return ContentLine.refuse(CodecError.LineBadValue)
            } else {
                byte = b
            }
            if (n == maxSize) return ContentLine.refuse(CodecError.LineTooLong)
            buf[n++] = byte.toByte()
        }
    }

    /**
     * The decimal the rest of the value is — an optional sign, digits — as
     * (negative, magnitude). A `-` is read only when [allowMinus], so an unsigned
     * type refuses `-0` as well.
     */
    private fun readDecimal(allowMinus: Boolean): Pair<Boolean, ULong>? {
        if (!beginValue()) return null
        var next = scan.bump()
        val negative = next == '-'.code
        if (negative && !allowMinus) return ContentLine.refuse(CodecError.LineBadValue)
        if (negative || next == '+'.code) next = scan.bump()
        var magnitude = 0UL
        var digits = 0
        while (next >= 0) {
            if (next < '0'.code || next > '9'.code) return ContentLine.refuse(CodecError.LineBadValue)
            val d = (next - '0'.code).toULong()
            if (magnitude > (ULong.MAX_VALUE - d) / 10UL) {
                return ContentLine.refuse(CodecError.LineBadValue)
            }
            magnitude = magnitude * 10UL + d
            digits++
            next = scan.bump()
        }
        if (digits == 0) return ContentLine.refuse(CodecError.LineBadValue)
        return negative to magnitude
    }

    /** Read the value as an unsigned integer of at most [max]. */
    fun readUint(max: ULong): ULong? {
        val (_, magnitude) = readDecimal(false) ?: return null
        return if (magnitude <= max) magnitude else ContentLine.refuse(CodecError.LineBadValue)
    }

    /** Read the value as a signed integer within [min]..[max]. */
    fun readInt(min: Long, max: Long): Long? {
        val (negative, magnitude) = readDecimal(true) ?: return null
        val value: Long = if (negative) {
            if (magnitude > 0x8000_0000_0000_0000UL) return ContentLine.refuse(CodecError.LineBadValue)
            // 2^63 wraps to Long.MIN_VALUE, whose negation is itself.
            -(magnitude.toLong())
        } else {
            if (magnitude > Long.MAX_VALUE.toULong()) return ContentLine.refuse(CodecError.LineBadValue)
            magnitude.toLong()
        }
        return if (value in min..max) value else ContentLine.refuse(CodecError.LineBadValue)
    }

    /** Read the value as `TRUE` or `FALSE`, in either case. */
    fun readBool(): Boolean? {
        if (!beginValue()) return null
        val word = StringBuilder()
        while (true) {
            val b = scan.bump()
            if (b < 0) break
            if (word.length == 5) return ContentLine.refuse(CodecError.LineBadValue)
            word.append(ContentLine.lower(b).toChar())
        }
        return when (word.toString()) {
            "true" -> true
            "false" -> false
            else -> ContentLine.refuse(CodecError.LineBadValue)
        }
    }
}

// ── Writing ─────────────────────────────────────────────────────────────

/**
 * A writer of the lines of one component into a sink.
 *
 * Write [begin]; then a property by [property], each present parameter by
 * [param], and the value by one of the value methods, which ends the line; then
 * [finish]. Each answers the [CodecError] that refused it, or `null`.
 */
class ContentLineWriter(private val sink: SceSink, private val component: String) {
    /** Octets already on the current physical line. */
    private var column = 0

    private fun raw(text: String): CodecError? = sink.writeBytes(text.encodeToByteArray())

    /** Write `BEGIN:<component>`. */
    fun begin(): CodecError? = raw("BEGIN:") ?: raw(component) ?: raw("\r\n")

    /** Write `END:<component>`. */
    fun finish(): CodecError? = raw("END:") ?: raw(component) ?: raw("\r\n")

    /**
     * Write one unit — a character, or an escape — on the current line, after
     * cutting the line if it would pass [ContentLine.FOLD_WIDTH] octets.
     */
    private fun unit(bytes: ByteArray): CodecError? {
        if (column + bytes.size > ContentLine.FOLD_WIDTH) {
            sink.writeBytes(byteArrayOf(CR, LF, SPACE))?.let { return it }
            column = 1
        }
        sink.writeBytes(bytes)?.let { return it }
        column += bytes.size
        return null
    }

    private fun unit(b: Char): CodecError? = unit(byteArrayOf(b.code.toByte()))

    /** The units of ASCII [text]: one octet each. */
    private fun asciiUnits(text: String): CodecError? {
        for (c in text) unit(c)?.let { return it }
        return null
    }

    /** The units of UTF-8 [bytes]: one character each, escaped when [text]. */
    private fun valueUnits(bytes: ByteArray, text: Boolean): CodecError? {
        var i = 0
        while (i < bytes.size) {
            val lead = bytes[i].toInt() and 0xFF
            val length = when {
                lead < 0x80 -> 1
                lead < 0xE0 -> 2
                lead < 0xF0 -> 3
                else -> 4
            }.coerceAtMost(bytes.size - i)
            val escaped: ByteArray? = if (text && length == 1) {
                when (lead) {
                    '\\'.code -> byteArrayOf('\\'.code.toByte(), '\\'.code.toByte())
                    ';'.code -> byteArrayOf('\\'.code.toByte(), ';'.code.toByte())
                    ','.code -> byteArrayOf('\\'.code.toByte(), ','.code.toByte())
                    0x0A -> byteArrayOf('\\'.code.toByte(), 'n'.code.toByte())
                    else -> null
                }
            } else {
                null
            }
            unit(escaped ?: bytes.copyOfRange(i, i + length))?.let { return it }
            i += length
        }
        return null
    }

    /** Start a property's line with its name. */
    fun property(name: String): CodecError? {
        column = 0
        return asciiUnits(name)
    }

    /**
     * Write `;<name>=<value>`, quoting the value when it holds `:`, `;` or `,`. A
     * value past [maxSize] bytes is [CodecError.LineTooLong]; one with a control
     * character or a `"` is [CodecError.LineBadValue].
     */
    fun param(name: String, value: String, maxSize: Int): CodecError? {
        val bytes = value.encodeToByteArray()
        if (bytes.size > maxSize) return ContentLine.fail(CodecError.LineTooLong)
        if (bytes.any { it.toInt() == '"'.code || ContentLine.isControl(it.toInt() and 0xFF) }) {
            return ContentLine.fail(CodecError.LineBadValue)
        }
        val quoted = bytes.any { it.toInt() == ':'.code || it.toInt() == ';'.code || it.toInt() == ','.code }
        unit(';')?.let { return it }
        asciiUnits(name)?.let { return it }
        unit('=')?.let { return it }
        if (quoted) unit('"')?.let { return it }
        valueUnits(bytes, false)?.let { return it }
        if (quoted) unit('"')?.let { return it }
        return null
    }

    private fun endLine(): CodecError? {
        column = 0
        return raw("\r\n")
    }

    /**
     * Write `:<value>` and end the line. With [text], `\`, `;`, `,` and a line
     * feed are written as escapes. A value past [maxSize] bytes is
     * [CodecError.LineTooLong]; one with a control character (but a TEXT's line
     * feed) is [CodecError.LineBadValue].
     */
    fun string(value: String, text: Boolean, maxSize: Int): CodecError? {
        val bytes = value.encodeToByteArray()
        if (bytes.size > maxSize) return ContentLine.fail(CodecError.LineTooLong)
        if (bytes.any { ContentLine.isControl(it.toInt() and 0xFF) && !(text && it == LF) }) {
            return ContentLine.fail(CodecError.LineBadValue)
        }
        unit(':')?.let { return it }
        valueUnits(bytes, text)?.let { return it }
        return endLine()
    }

    /**
     * Write `:<value>{separator}<value>…` and end the line (docs/adr/0014). No
     * values is [CodecError.LineRequiredMissing] and more than [maxValues] is
     * [CodecError.LineTooMany]. A value past [maxSize] bytes is
     * [CodecError.LineTooLong]; one with a control character (but a TEXT's line
     * feed), an empty one, and one that is not a TEXT and holds the separator are
     * [CodecError.LineBadValue], because a reader would cut or refuse them. Every
     * value is held before any of the line is written.
     */
    fun strings(values: List<String>, separator: Char, text: Boolean, maxSize: Int, maxValues: Int): CodecError? {
        if (values.isEmpty()) return ContentLine.fail(CodecError.LineRequiredMissing)
        if (values.size > maxValues) return ContentLine.fail(CodecError.LineTooMany)
        val encoded = values.map { it.encodeToByteArray() }
        for (bytes in encoded) {
            if (bytes.size > maxSize) return ContentLine.fail(CodecError.LineTooLong)
            if (bytes.any { ContentLine.isControl(it.toInt() and 0xFF) && !(text && it == LF) }) {
                return ContentLine.fail(CodecError.LineBadValue)
            }
            if (bytes.isEmpty() || (!text && bytes.any { it.toInt() == separator.code })) {
                return ContentLine.fail(CodecError.LineBadValue)
            }
        }
        unit(':')?.let { return it }
        for ((index, bytes) in encoded.withIndex()) {
            if (index > 0) unit(separator)?.let { return it }
            valueUnits(bytes, text)?.let { return it }
        }
        return endLine()
    }

    private fun digits(text: String): CodecError? {
        unit(':')?.let { return it }
        asciiUnits(text)?.let { return it }
        return endLine()
    }

    /** Write `:<value>` as decimal digits and end the line. */
    fun uint(value: ULong): CodecError? = digits(value.toString())

    /** Write `:<value>` as decimal digits, `-` first when negative, and end the line. */
    fun int(value: Long): CodecError? = digits(value.toString())

    /** Write `:TRUE` or `:FALSE` and end the line. */
    fun boolean(value: Boolean): CodecError? = digits(if (value) "TRUE" else "FALSE")
}
