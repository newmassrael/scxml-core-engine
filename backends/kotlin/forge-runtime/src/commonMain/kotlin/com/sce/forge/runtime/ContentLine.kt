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
 * content_line` (Rust). **Reading** answers a [Decoded]: what was read, or the
 * [CodecError] of the rule that refused it, as the answer of the read that met
 * it (docs/adr/0099, amendment of 2026-11-16). **Writing** answers a
 * [CodecError] or `null` on success, as every Kotlin encode does.
 */
object ContentLine {
    /** The most octets of one physical line (SCE_FORGE.md §4.6.4, *Folding*). */
    const val FOLD_WIDTH = 75

    /**
     * The bound a reader or a writer is handed for a text or a list the protocol
     * states no bound of (docs/adr/0077 §7): the largest `Int`, past which no
     * frame this runtime reads has a length.
     */
    const val UNBOUNDED = Int.MAX_VALUE

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

/**
 * The bytes of one value as it is read. It grows as the line does: a value's
 * bound is the most it may hold and never a size to allocate, and it is
 * [ContentLine.UNBOUNDED] where the protocol states none (docs/adr/0077 §7).
 */
private class ValueBytes {
    var array = ByteArray(16)
        private set
    var size = 0
        private set

    fun add(byte: Int) {
        if (size == array.size) array = array.copyOf(array.size * 2)
        array[size++] = byte.toByte()
    }

    fun clear() {
        size = 0
    }
}

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
     * How many bytes of the input the walk has passed — after
     * `END:<component>` once [next] has answered no property.
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

    /**
     * The next property line of the component, or `null` after its
     * `END:<component>`; a line the grammar does not admit is refused.
     *
     * A nested component is skipped through its `END:` without reading its lines.
     * An input that ends before `END:<component>` is [CodecError.NeedMoreBytes].
     */
    fun next(): Decoded<ContentLineProperty?> {
        while (true) {
            val line = nextRawLine() ?: return Decoded.Refused(CodecError.NeedMoreBytes)
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
            if (head == null) return Decoded.Refused(cut)
            if (head.separator == ':'.code) {
                if (ContentLine.unfoldedEq(input, line.from, head.nameEnd, "END")) {
                    if (ContentLine.unfoldedEq(input, head.restStart, line.to, component)) return Decoded.Ok(null)
                    return Decoded.Refused(cut)
                }
                if (line.terminated && ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN")) {
                    depth = 1
                    continue
                }
            }
            if (!line.terminated) return Decoded.Refused(CodecError.NeedMoreBytes)
            return Decoded.Ok(ContentLineProperty(input, line.from, line.to, head.nameEnd))
        }
    }

    companion object {
        /**
         * Skip lines up to the first `BEGIN:<component>` and stand after it,
         * refused as [CodecError.NeedMoreBytes] when the input ends before it.
         */
        fun begin(input: ByteArray, component: String): Decoded<ContentLineReader> {
            val reader = ContentLineReader(input, component)
            while (true) {
                val line = reader.nextRawLine() ?: break
                if (!line.terminated) break
                val head = headOf(input, line.from, line.to) ?: continue
                if (head.separator == ':'.code &&
                    ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN") &&
                    ContentLine.unfoldedEq(input, head.restStart, line.to, component)
                ) {
                    return Decoded.Ok(reader)
                }
            }
            return Decoded.Refused(CodecError.NeedMoreBytes)
        }
    }
}

/**
 * One property line the codec has been handed.
 *
 * Ask [isNamed] whether it is one the codec reads. Then, for a property that
 * declares parameters, loop on [nextParam] and read the ones the codec declares;
 * then read the value. A parameter no one reads is skipped by the next call, and
 * a property read by value alone skips them all. Every read answers a
 * [Decoded], refused with the rule for what the entry cannot hold.
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
     * when there is one, `false` when the value is next, refused as
     * [CodecError.LineMalformed] when the line does not admit it.
     */
    fun nextParam(): Decoded<Boolean> {
        when (phase) {
            Phase.PARAM_VALUE -> skipParamValue()?.let { return Decoded.Refused(it) }
            Phase.DONE -> return Decoded.Refused(CodecError.LineMalformed)
            Phase.AT_SEPARATOR -> {}
        }
        return when (scan.peek()) {
            ':'.code -> Decoded.Ok(false)
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
                    return Decoded.Refused(CodecError.LineMalformed)
                }
                paramFrom = start
                paramTo = end
                phase = Phase.PARAM_VALUE
                Decoded.Ok(true)
            }
            else -> Decoded.Refused(CodecError.LineMalformed)
        }
    }

    /**
     * Scan one parameter value — a quoted string, or text up to `;`, `:`, `,`
     * or `"` — handing each byte of it to [emit] (`null`: skipped), which
     * answers the refusal of a byte its value cannot hold or `null`. Answers
     * whether another value follows a `,`, or the first refusal met.
     */
    private fun scanParamValue(emit: ((Int) -> CodecError?)?): Decoded<Boolean> {
        if (scan.peek() == '"'.code) {
            scan.bump()
            while (true) {
                val b = scan.bump()
                if (b < 0) return Decoded.Refused(CodecError.LineMalformed)
                if (b == '"'.code) break
                emit?.invoke(b)?.let { return Decoded.Refused(it) }
            }
        } else {
            while (true) {
                val b = scan.peek()
                if (b < 0 || b == ';'.code || b == ':'.code || b == ','.code) break
                if (b == '"'.code) return Decoded.Refused(CodecError.LineMalformed)
                scan.bump()
                emit?.invoke(b)?.let { return Decoded.Refused(it) }
            }
        }
        return when (scan.peek()) {
            ','.code -> {
                scan.bump()
                Decoded.Ok(true)
            }
            ';'.code, ':'.code -> Decoded.Ok(false)
            else -> Decoded.Refused(CodecError.LineMalformed)
        }
    }

    /** Skip the values of the parameter stood on: the refusal met, or `null`. */
    private fun skipParamValue(): CodecError? {
        while (true) {
            val more = scanParamValue(null).valueOr { refused -> return refused.error }
            if (!more) break
        }
        paramFrom = -1
        phase = Phase.AT_SEPARATOR
        return null
    }

    /**
     * Read the value of the parameter [nextParam] stands on, into at most
     * [maxSize] bytes. A second value is [CodecError.LineBadValue].
     */
    fun readParamString(maxSize: Int): Decoded<String> {
        if (phase != Phase.PARAM_VALUE) return Decoded.Refused(CodecError.LineMalformed)
        val buf = ValueBytes()
        val more = scanParamValue { b ->
            when {
                ContentLine.isControl(b) -> CodecError.LineBadValue
                buf.size == maxSize -> CodecError.LineTooLong
                else -> {
                    buf.add(b)
                    null
                }
            }
        }.valueOr { refused -> return refused }
        if (more) return Decoded.Refused(CodecError.LineBadValue)
        paramFrom = -1
        phase = Phase.AT_SEPARATOR
        return utf8(buf)
    }

    private fun utf8(buf: ValueBytes): Decoded<String> =
        try {
            Decoded.Ok(buf.array.decodeToString(0, buf.size, throwOnInvalidSequence = true))
        } catch (_: CharacterCodingException) {
            Decoded.Refused(CodecError.LineBadValue)
        }

    /**
     * Stand at the value: skip the parameters still unread and step over `:`.
     * The refusal met, or `null`.
     */
    private fun beginValue(): CodecError? {
        while (true) {
            val another = nextParam().valueOr { refused -> return refused.error }
            if (!another) break
        }
        scan.bump()
        phase = Phase.DONE
        return null
    }

    /**
     * Read the value as a `string` of at most [maxSize] bytes. With [text],
     * `\\`, `\;`, `\,`, `\n` and `\N` are escapes; an unescaped `;` or `,` is itself.
     */
    fun readString(maxSize: Int, text: Boolean): Decoded<String> {
        beginValue()?.let { return Decoded.Refused(it) }
        val buf = ValueBytes()
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
                    else -> return Decoded.Refused(CodecError.LineBadEscape)
                }
            } else if (ContentLine.isControl(b)) {
                return Decoded.Refused(CodecError.LineBadValue)
            } else {
                byte = b
            }
            if (buf.size == maxSize) return Decoded.Refused(CodecError.LineTooLong)
            buf.add(byte)
        }
        return utf8(buf)
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
    fun readStrings(separator: Char, maxValues: Int, maxSize: Int, text: Boolean): Decoded<MutableList<String>> {
        beginValue()?.let { return Decoded.Refused(it) }
        val values = mutableListOf<String>()
        val buf = ValueBytes()
        while (true) {
            val b = scan.bump()
            if (b < 0 || b == separator.code) {
                if (buf.size == 0) return Decoded.Refused(CodecError.LineBadValue)
                values.add(utf8(buf).valueOr { refused -> return refused })
                if (b < 0) return Decoded.Ok(values)
                buf.clear()
                // A separator opens another part, which may not pass the bound.
                if (values.size >= maxValues) return Decoded.Refused(CodecError.LineTooMany)
                continue
            }
            val byte: Int
            if (text && b == '\\'.code) {
                byte = when (scan.bump()) {
                    '\\'.code -> '\\'.code
                    ';'.code -> ';'.code
                    ','.code -> ','.code
                    'n'.code, 'N'.code -> 0x0A
                    else -> return Decoded.Refused(CodecError.LineBadEscape)
                }
            } else if (ContentLine.isControl(b)) {
                return Decoded.Refused(CodecError.LineBadValue)
            } else {
                byte = b
            }
            if (buf.size == maxSize) return Decoded.Refused(CodecError.LineTooLong)
            buf.add(byte)
        }
    }

    /**
     * The decimal the rest of the value is — an optional sign, digits — as
     * (negative, magnitude). A `-` is read only when [allowMinus], so an unsigned
     * type refuses `-0` as well.
     */
    private fun readDecimal(allowMinus: Boolean): Decoded<Pair<Boolean, ULong>> {
        beginValue()?.let { return Decoded.Refused(it) }
        var next = scan.bump()
        val negative = next == '-'.code
        if (negative && !allowMinus) return Decoded.Refused(CodecError.LineBadValue)
        if (negative || next == '+'.code) next = scan.bump()
        var magnitude = 0UL
        var digits = 0
        while (next >= 0) {
            if (next < '0'.code || next > '9'.code) return Decoded.Refused(CodecError.LineBadValue)
            val d = (next - '0'.code).toULong()
            if (magnitude > (ULong.MAX_VALUE - d) / 10UL) {
                return Decoded.Refused(CodecError.LineBadValue)
            }
            magnitude = magnitude * 10UL + d
            digits++
            next = scan.bump()
        }
        if (digits == 0) return Decoded.Refused(CodecError.LineBadValue)
        return Decoded.Ok(negative to magnitude)
    }

    /** Read the value as an unsigned integer of at most [max]. */
    fun readUint(max: ULong): Decoded<ULong> {
        val (_, magnitude) = readDecimal(false).valueOr { refused -> return refused }
        return if (magnitude <= max) Decoded.Ok(magnitude) else Decoded.Refused(CodecError.LineBadValue)
    }

    /** Read the value as a signed integer within [min]..[max]. */
    fun readInt(min: Long, max: Long): Decoded<Long> {
        val (negative, magnitude) = readDecimal(true).valueOr { refused -> return refused }
        val value: Long = if (negative) {
            if (magnitude > 0x8000_0000_0000_0000UL) return Decoded.Refused(CodecError.LineBadValue)
            // 2^63 wraps to Long.MIN_VALUE, whose negation is itself.
            -(magnitude.toLong())
        } else {
            if (magnitude > Long.MAX_VALUE.toULong()) return Decoded.Refused(CodecError.LineBadValue)
            magnitude.toLong()
        }
        return if (value in min..max) Decoded.Ok(value) else Decoded.Refused(CodecError.LineBadValue)
    }

    /** Read the value as `TRUE` or `FALSE`, in either case. */
    fun readBool(): Decoded<Boolean> {
        beginValue()?.let { return Decoded.Refused(it) }
        val word = StringBuilder()
        while (true) {
            val b = scan.bump()
            if (b < 0) break
            if (word.length == 5) return Decoded.Refused(CodecError.LineBadValue)
            word.append(ContentLine.lower(b).toChar())
        }
        return when (word.toString()) {
            "true" -> Decoded.Ok(true)
            "false" -> Decoded.Ok(false)
            else -> Decoded.Refused(CodecError.LineBadValue)
        }
    }

    /**
     * Read the value as the text of a variant of an enum (docs/adr/0015): the
     * carrier of the first row of [table] whose text it is, ASCII
     * case-insensitively. A value no row names is [CodecError.LineBadValue],
     * whatever its bytes are: it is no TEXT, so a backslash in it is not an
     * escape, and nothing of it is kept past the longest text.
     */
    fun readEnum(table: EnumTexts): Decoded<ULong> {
        beginValue()?.let { return Decoded.Refused(it) }
        val longest = table.longest()
        val word = ArrayList<Byte>()
        while (true) {
            val b = scan.bump()
            if (b < 0) break
            if (word.size <= longest) word.add(b.toByte())
        }
        return table.carrierOf(word.toByteArray())
    }

    /**
     * Read the value of the parameter [nextParam] stands on as the text of a
     * variant of an enum (docs/adr/0015). The whole value is scanned before it
     * is judged, so a line the grammar refuses is [CodecError.LineMalformed]
     * before it is [CodecError.LineBadValue]; a second value is
     * [CodecError.LineBadValue].
     */
    fun readParamEnum(table: EnumTexts): Decoded<ULong> {
        if (phase != Phase.PARAM_VALUE) return Decoded.Refused(CodecError.LineMalformed)
        val longest = table.longest()
        val word = ArrayList<Byte>()
        val more = scanParamValue { b ->
            if (word.size <= longest) word.add(b.toByte())
            null
        }.valueOr { refused -> return refused }
        paramFrom = -1
        phase = Phase.AT_SEPARATOR
        if (more) return Decoded.Refused(CodecError.LineBadValue)
        return table.carrierOf(word.toByteArray())
    }
}

/**
 * One variant of an enum a content-line entry is read and written by
 * (docs/adr/0015): the text the enum document gives it and the carrier value it
 * stands for.
 */
class EnumText(val text: String, val value: ULong)

/**
 * The variants of an enum in declaration order. A generated codec holds it as
 * data, because the enum's own type carries no text.
 */
class EnumTexts(private val rows: List<EnumText>) {
    internal fun longest(): Int = rows.maxOfOrNull { it.text.length } ?: 0

    /**
     * The carrier of the first variant whose text [value] is, compared ASCII
     * case-insensitively: only the 26 letters fold, so a character Unicode folds
     * to one of them (U+017F, U+0131) is not that letter. A value no variant names
     * is [CodecError.LineBadValue].
     */
    internal fun carrierOf(value: ByteArray): Decoded<ULong> {
        for (row in rows) {
            val text = row.text.encodeToByteArray()
            if (text.size != value.size) continue
            var same = true
            for (i in value.indices) {
                if (ContentLine.lower(text[i].toInt() and 0xFF) != ContentLine.lower(value[i].toInt() and 0xFF)) {
                    same = false
                    break
                }
            }
            if (same) return Decoded.Ok(row.value)
        }
        return Decoded.Refused(CodecError.LineBadValue)
    }

    /**
     * The text of the first variant with [carrier], as the enum declares it; a
     * carrier of an open enum that no variant declares has none, which the caller
     * answers [CodecError.LineBadValue].
     */
    internal fun textOf(carrier: ULong): String? = rows.firstOrNull { it.value == carrier }?.text
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
        if (bytes.size > maxSize) return CodecError.LineTooLong
        if (bytes.any { it.toInt() == '"'.code || ContentLine.isControl(it.toInt() and 0xFF) }) {
            return CodecError.LineBadValue
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
        if (bytes.size > maxSize) return CodecError.LineTooLong
        if (bytes.any { ContentLine.isControl(it.toInt() and 0xFF) && !(text && it == LF) }) {
            return CodecError.LineBadValue
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
        if (values.isEmpty()) return CodecError.LineRequiredMissing
        if (values.size > maxValues) return CodecError.LineTooMany
        val encoded = values.map { it.encodeToByteArray() }
        for (bytes in encoded) {
            if (bytes.size > maxSize) return CodecError.LineTooLong
            if (bytes.any { ContentLine.isControl(it.toInt() and 0xFF) && !(text && it == LF) }) {
                return CodecError.LineBadValue
            }
            if (bytes.isEmpty() || (!text && bytes.any { it.toInt() == separator.code })) {
                return CodecError.LineBadValue
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

    /**
     * Write `;<name>=<text>` for the variant with [carrier]. A text is letters,
     * digits and hyphens, so it is never quoted (docs/adr/0015). A carrier no
     * variant declares has no text and is [CodecError.LineBadValue].
     */
    fun enumParam(name: String, table: EnumTexts, carrier: ULong): CodecError? {
        val text = table.textOf(carrier) ?: return CodecError.LineBadValue
        unit(';')?.let { return it }
        asciiUnits(name)?.let { return it }
        unit('=')?.let { return it }
        return asciiUnits(text)
    }

    /**
     * Write `:<text>` for the variant with [carrier] and end the line
     * (docs/adr/0015). A carrier no variant declares has no text and is
     * [CodecError.LineBadValue].
     */
    fun enumValue(table: EnumTexts, carrier: ULong): CodecError? {
        val text = table.textOf(carrier) ?: return CodecError.LineBadValue
        return digits(text)
    }
}
