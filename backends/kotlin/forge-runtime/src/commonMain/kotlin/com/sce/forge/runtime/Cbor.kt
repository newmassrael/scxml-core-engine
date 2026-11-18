// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes
 * (SCE_FORGE.md §4.6.1, docs/adr/0101): one definite-length map whose keys are
 * small unsigned integers, and below it the form of each type of the language:
 * an integer of either sign, a float, a truth value, a text or byte string, an
 * array, a record's map keyed by its fields' ids as text, and a union's map of
 * one entry keyed by its case's variant name.
 *
 * A generated codec calls these and spells no CBOR of its own. **Writing** is
 * deterministic (RFC 8949 section 4.2.1: every head in its shortest form, a
 * float in the shortest precision that holds it exactly), so this runtime and
 * every other write the same bytes for the same value. **Reading** takes the
 * definite-length profile of CBOR and, within it, every well-formed encoding
 * of a value (a head longer than it needs, a float wider than it needs), and
 * answers a [Decoded]: the item, or the [CodecError] that names the class of
 * what refused it (docs/adr/0101 section 3.3; docs/adr/0099, amendment of
 * 2026-11-16). The refusal is the answer of the read that met it, so
 * concurrent decodes never see each other's.
 *
 * Cross-language sibling: `sce_forge_runtime::cbor` (Rust).
 */
object Cbor {
    const val MAJOR_UNSIGNED = 0
    const val MAJOR_NEGATIVE = 1
    const val MAJOR_BYTES = 2
    const val MAJOR_TEXT = 3
    const val MAJOR_ARRAY = 4
    const val MAJOR_MAP = 5
    const val MAJOR_TAG = 6
    const val MAJOR_SIMPLE = 7

    /** The deepest an unknown entry's value may nest before [skip] refuses it. */
    const val MAX_SKIP_DEPTH = 16

    /** The additional information of `false` and `true` (RFC 8949 section 3.3). */
    private const val SIMPLE_FALSE = 20
    private const val SIMPLE_TRUE = 21

    /** The additional information of a half, a single and a double precision float. */
    private const val INFO_HALF = 25
    private const val INFO_SINGLE = 26
    private const val INFO_DOUBLE = 27

    /** The least simple value the two-byte form may carry: one below it is not well-formed. */
    private const val LEAST_TWO_BYTE_SIMPLE = 32UL

    /** The one NaN written, the quiet NaN of half precision (docs/adr/0086, section 9.2). */
    private const val NAN_HALF: UShort = 0x7e00u

    /** A half precision subnormal's unit, 2^-24. */
    private const val HALF_SUBNORMAL_UNIT = 1.0 / 16777216.0

    // ── Writing ─────────────────────────────────────────────────────────

    /** A head of [major] carrying [value], in its shortest form. */
    fun writeHead(w: SceSink, major: Int, value: ULong): CodecError? {
        val m = (major shl 5)
        return when {
            value < 24UL -> w.writeU8((m or value.toInt()).toByte())
            value <= 0xFFUL -> w.writeBytes(byteArrayOf((m or 24).toByte(), value.toByte()))
            value <= 0xFFFFUL -> w.writeU8((m or 25).toByte()) ?: w.writeU16Be(value.toUShort())
            value <= 0xFFFF_FFFFUL -> w.writeU8((m or 26).toByte()) ?: w.writeU32Be(value.toUInt())
            else -> w.writeU8((m or 27).toByte()) ?: w.writeU64Be(value)
        }
    }

    fun writeUint(w: SceSink, value: ULong): CodecError? = writeHead(w, MAJOR_UNSIGNED, value)

    /**
     * A signed integer: major type 0 for zero or more, and below zero major
     * type 1, whose head carries `-1 - value` (RFC 8949 section 3.1).
     */
    fun writeInt(w: SceSink, value: Long): CodecError? =
        if (value >= 0L) {
            writeHead(w, MAJOR_UNSIGNED, value.toULong())
        } else {
            writeHead(w, MAJOR_NEGATIVE, (-1L - value).toULong())
        }

    /**
     * A float in the shortest of half, single and double precision that holds
     * it exactly, and a NaN as `f9 7e 00` (RFC 8949 sections 4.1 and 4.2.1).
     * A `float32` and a `float64` share it: a single is a double exactly.
     */
    fun writeFloat(w: SceSink, value: Double): CodecError? {
        if (value.isNaN()) return writeFloatHead(w, INFO_HALF) ?: w.writeU16Be(NAN_HALF)
        val half = halfOf(value)
        if (half != null) return writeFloatHead(w, INFO_HALF) ?: w.writeU16Be(half)
        val single = value.toFloat()
        if (single.toDouble() == value) {
            return writeFloatHead(w, INFO_SINGLE) ?: w.writeU32Be(single.toRawBits().toUInt())
        }
        return writeFloatHead(w, INFO_DOUBLE) ?: w.writeU64Be(value.toRawBits().toULong())
    }

    /** A `float32`, written as [writeFloat] writes the double it is exactly. */
    fun writeFloat(w: SceSink, value: Float): CodecError? = writeFloat(w, value.toDouble())

    private fun writeFloatHead(w: SceSink, info: Int): CodecError? =
        w.writeU8(((MAJOR_SIMPLE shl 5) or info).toByte())

    fun writeBool(w: SceSink, value: Boolean): CodecError? =
        w.writeU8(((MAJOR_SIMPLE shl 5) or if (value) SIMPLE_TRUE else SIMPLE_FALSE).toByte())

    fun writeText(w: SceSink, value: String): CodecError? {
        val bytes = value.encodeToByteArray()
        return writeHead(w, MAJOR_TEXT, bytes.size.toULong()) ?: w.writeBytes(bytes)
    }

    fun writeBytes(w: SceSink, value: ByteArray): CodecError? =
        writeHead(w, MAJOR_BYTES, value.size.toULong()) ?: w.writeBytes(value)

    /** The head of a definite-length array of [items] items: a list an entry holds. */
    fun writeArrayHead(w: SceSink, items: ULong): CodecError? = writeHead(w, MAJOR_ARRAY, items)

    fun writeMapHead(w: SceSink, entries: ULong): CodecError? = writeHead(w, MAJOR_MAP, entries)

    /**
     * The head of a union's value: a map of one entry keyed by the case's
     * [variant] name as a text string. The case's record follows, as a map, or
     * the empty map (`a0`) for a case that holds none (docs/adr/0101
     * section 2.1).
     */
    fun writeVariant(w: SceSink, variant: String): CodecError? =
        writeMapHead(w, 1UL) ?: writeText(w, variant)

    /**
     * The half precision float that is [x] exactly, as its bits, or `null`
     * when no half is (IEEE 754 binary16: 5 bits of exponent, 10 of fraction,
     * subnormal below 2^-14). [x] is not a NaN.
     */
    private fun halfOf(x: Double): UShort? {
        val bits = x.toRawBits()
        val sign = ((bits ushr 48) and 0x8000L).toInt()
        val exponent = ((bits ushr 52) and 0x7ffL).toInt()
        val fraction = bits and ((1L shl 52) - 1L)
        if (exponent == 0x7ff) {
            // An infinity; a NaN is written before this is asked.
            return if (fraction == 0L) (sign or 0x7c00).toUShort() else null
        }
        if (exponent == 0) {
            // Zero, or a double's subnormal, which is far below a half's least.
            return if (fraction == 0L) sign.toUShort() else null
        }
        val power = exponent - 1023
        if (power in -14..15) {
            // A half's normal number keeps the top 10 bits of the fraction and nothing below them.
            if ((fraction and ((1L shl 42) - 1L)) != 0L) return null
            return (sign or ((power + 15) shl 10) or (fraction ushr 42).toInt()).toUShort()
        }
        if (power in -24..-15) {
            // A half's subnormal number is k * 2^-24 for k from 1 to 1023.
            val significand = fraction or (1L shl 52)
            val shift = 28 - power
            if ((significand and ((1L shl shift) - 1L)) != 0L) return null
            return (sign or (significand ushr shift).toInt()).toUShort()
        }
        return null
    }

    /** The value of the half precision float whose bits are [half] (RFC 8949 Appendix D). */
    private fun halfValue(half: Int): Double {
        val exponent = (half ushr 10) and 0x1f
        val fraction = half and 0x3ff
        val magnitude = when (exponent) {
            0 -> fraction * HALF_SUBNORMAL_UNIT
            0x1f -> if (fraction == 0) Double.POSITIVE_INFINITY else Double.NaN
            else -> Double.fromBits(((exponent - 15 + 1023).toLong() shl 52) or (fraction.toLong() shl 42))
        }
        return if ((half and 0x8000) != 0) -magnitude else magnitude
    }

    // ── Reading ─────────────────────────────────────────────────────────

    /**
     * One head: its major type, its additional information, and the value
     * that information carries (for a string, an array or a map, the length;
     * for a float, its bits).
     */
    data class Head(val major: Int, val info: Int, val value: ULong)

    private fun readBe(c: SceCursor, n: Int): Decoded<ULong> {
        var value = 0UL
        repeat(n) {
            val b = c.readByte() ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            value = (value shl 8) or b.toULong()
        }
        return Decoded.Ok(value)
    }

    /**
     * One head. What is not well-formed (RFC 8949 Appendix F: the reserved
     * values 28-30, a break code, a simple value in the two-byte form below
     * 32) and an indefinite length, which is outside the profile the codec
     * reads (docs/adr/0101 section 3.1), are refused as
     * [CodecError.CborMalformed].
     */
    fun readHead(c: SceCursor): Decoded<Head> {
        val initial = c.readByte() ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        val major = initial ushr 5
        val info = initial and 0x1F
        val value = when (info) {
            in 0..23 -> info.toULong()
            24 -> readBe(c, 1).valueOr { refused -> return refused }
            25 -> readBe(c, 2).valueOr { refused -> return refused }
            26 -> readBe(c, 4).valueOr { refused -> return refused }
            27 -> readBe(c, 8).valueOr { refused -> return refused }
            else -> return Decoded.Refused(CodecError.CborMalformed)
        }
        if (major == MAJOR_SIMPLE && info == 24 && value < LEAST_TWO_BYTE_SIMPLE) {
            return Decoded.Refused(CodecError.CborMalformed)
        }
        return Decoded.Ok(Head(major, info, value))
    }

    /** The value of a head of [major]; a head of another major type is [CodecError.CborWrongType]. */
    private fun expect(c: SceCursor, major: Int): Decoded<ULong> {
        val head = readHead(c).valueOr { refused -> return refused }
        return if (head.major == major) Decoded.Ok(head.value) else Decoded.Refused(CodecError.CborWrongType)
    }

    /** An unsigned integer; a negative one, as any other item, is [CodecError.CborWrongType]. */
    fun readUint(c: SceCursor): Decoded<ULong> = expect(c, MAJOR_UNSIGNED)

    /** An unsigned integer an entry of [max] holds. */
    fun readUintUpto(c: SceCursor, max: ULong): Decoded<ULong> {
        val value = readUint(c).valueOr { refused -> return refused }
        return if (value > max) Decoded.Refused(CodecError.CborOutOfRange) else Decoded.Ok(value)
    }

    /**
     * A signed integer from [min] to [max], of major type 0 or 1: one outside
     * them is [CodecError.CborOutOfRange], an item of another major type
     * [CodecError.CborWrongType].
     */
    fun readIntWithin(c: SceCursor, min: Long, max: Long): Decoded<Long> {
        val head = readHead(c).valueOr { refused -> return refused }
        return when (head.major) {
            MAJOR_UNSIGNED ->
                if (max < 0L || head.value > max.toULong()) {
                    Decoded.Refused(CodecError.CborOutOfRange)
                } else {
                    Decoded.Ok(head.value.toLong())
                }
            // The item is -1 - n, which is at least `min` when n is at most -1 - min.
            MAJOR_NEGATIVE ->
                if (min >= 0L || head.value > (-1L - min).toULong()) {
                    Decoded.Refused(CodecError.CborOutOfRange)
                } else {
                    Decoded.Ok(-1L - head.value.toLong())
                }
            else -> Decoded.Refused(CodecError.CborWrongType)
        }
    }

    /**
     * A `float64`: a half, a single or a double, each read to the value it is
     * exactly. Any other item, an integer among them, is
     * [CodecError.CborWrongType].
     */
    fun readFloat(c: SceCursor): Decoded<Double> {
        val head = readHead(c).valueOr { refused -> return refused }
        if (head.major != MAJOR_SIMPLE) return Decoded.Refused(CodecError.CborWrongType)
        return when (head.info) {
            INFO_HALF -> Decoded.Ok(halfValue(head.value.toInt()))
            INFO_SINGLE -> Decoded.Ok(Float.fromBits(head.value.toInt()).toDouble())
            INFO_DOUBLE -> Decoded.Ok(Double.fromBits(head.value.toLong()))
            else -> Decoded.Refused(CodecError.CborWrongType)
        }
    }

    /**
     * A `float32`: a float of any precision whose value a single holds
     * exactly, or a NaN. A double that no single is, is
     * [CodecError.CborOutOfRange].
     */
    fun readFloat32(c: SceCursor): Decoded<Float> {
        val value = readFloat(c).valueOr { refused -> return refused }
        val single = value.toFloat()
        return if (value.isNaN() || single.toDouble() == value) {
            Decoded.Ok(single)
        } else {
            Decoded.Refused(CodecError.CborOutOfRange)
        }
    }

    /** `false` or `true`; any other item, `null` among them, is [CodecError.CborWrongType]. */
    fun readBool(c: SceCursor): Decoded<Boolean> {
        val head = readHead(c).valueOr { refused -> return refused }
        return when {
            head.major == MAJOR_SIMPLE && head.info == SIMPLE_FALSE -> Decoded.Ok(false)
            head.major == MAJOR_SIMPLE && head.info == SIMPLE_TRUE -> Decoded.Ok(true)
            else -> Decoded.Refused(CodecError.CborWrongType)
        }
    }

    private fun readPayload(c: SceCursor, len: ULong): Decoded<ByteArray> {
        if (len > c.remaining().toULong()) return Decoded.Refused(CodecError.NeedMoreBytes)
        val n = len.toInt()
        val bytes = c.peekSlice(n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        c.advance(n)
        return Decoded.Ok(bytes)
    }

    /**
     * A text string of at most [maxSize] bytes (`null`: no bound), its length
     * judged before its bytes are read. A text in chunks has an indefinite
     * length and is [CodecError.CborMalformed].
     */
    fun readText(c: SceCursor, maxSize: Int?): Decoded<String> {
        val len = expect(c, MAJOR_TEXT).valueOr { refused -> return refused }
        if (maxSize != null && len > maxSize.toULong()) return Decoded.Refused(CodecError.CborOutOfRange)
        val bytes = readPayload(c, len).valueOr { refused -> return refused }
        return try {
            Decoded.Ok(bytes.decodeToString(throwOnInvalidSequence = true))
        } catch (_: CharacterCodingException) {
            // The kind every runtime gives a text that is not UTF-8.
            Decoded.Refused(CodecError.InvalidUtf8)
        }
    }

    /**
     * A byte string of at most [maxSize] bytes (`null`: no bound). One in
     * chunks has an indefinite length and is [CodecError.CborMalformed].
     */
    fun readBytes(c: SceCursor, maxSize: Int?): Decoded<ByteArray> {
        val len = expect(c, MAJOR_BYTES).valueOr { refused -> return refused }
        if (maxSize != null && len > maxSize.toULong()) return Decoded.Refused(CodecError.CborOutOfRange)
        return readPayload(c, len)
    }

    /** A byte string of exactly [length] bytes. */
    fun readBytesExact(c: SceCursor, length: Int): Decoded<ByteArray> {
        val len = expect(c, MAJOR_BYTES).valueOr { refused -> return refused }
        if (len != length.toULong()) return Decoded.Refused(CodecError.CborWrongLength)
        return readPayload(c, len)
    }

    /** The head of a definite-length array: how many items follow. An indefinite length is refused, as a map's is. */
    fun readArrayLen(c: SceCursor): Decoded<ULong> = expect(c, MAJOR_ARRAY)

    /** The head of a definite-length map: how many entries follow. */
    fun readMapLen(c: SceCursor): Decoded<ULong> = expect(c, MAJOR_MAP)

    /**
     * The case of a union's value, as the index of its name in [variants]:
     * the map is judged for one entry before its key is read (no entry or
     * more than one is [CodecError.CborUnionEntries]), then its key (one that
     * is not a text is [CodecError.CborWrongType], a name no case declares
     * [CodecError.CborUnknownVariant]). The case's value follows, for its
     * own read (docs/adr/0101 section 3.3).
     */
    fun readVariant(c: SceCursor, variants: List<String>): Decoded<Int> {
        val entries = readMapLen(c).valueOr { refused -> return refused }
        if (entries != 1UL) return Decoded.Refused(CodecError.CborUnionEntries)
        val name = readText(c, null).valueOr { refused -> return refused }
        val index = variants.indexOf(name)
        return if (index < 0) Decoded.Refused(CodecError.CborUnknownVariant) else Decoded.Ok(index)
    }

    /**
     * Skip one item — the value of a key the codec does not declare — and
     * everything nested in it, refusing one nested deeper than
     * [MAX_SKIP_DEPTH] as [CodecError.CborTooDeep] and an indefinite length
     * anywhere in it as [CodecError.CborMalformed].
     */
    fun skip(c: SceCursor): Decoded<Unit> = skipAt(c, 0)

    private fun skipAt(c: SceCursor, depth: Int): Decoded<Unit> {
        if (depth >= MAX_SKIP_DEPTH) return Decoded.Refused(CodecError.CborTooDeep)
        val head = readHead(c).valueOr { refused -> return refused }
        when (head.major) {
            MAJOR_UNSIGNED, MAJOR_NEGATIVE, MAJOR_SIMPLE -> {}
            MAJOR_BYTES, MAJOR_TEXT -> readPayload(c, head.value).valueOr { refused -> return refused }
            MAJOR_ARRAY -> {
                var i = 0UL
                while (i < head.value) {
                    skipAt(c, depth + 1).valueOr { refused -> return refused }
                    i++
                }
            }
            MAJOR_MAP -> {
                var i = 0UL
                while (i < head.value) {
                    skipAt(c, depth + 1).valueOr { refused -> return refused }
                    skipAt(c, depth + 1).valueOr { refused -> return refused }
                    i++
                }
            }
            MAJOR_TAG -> skipAt(c, depth + 1).valueOr { refused -> return refused }
            else -> return Decoded.Refused(CodecError.CborMalformed)
        }
        return Decoded.Ok(Unit)
    }
}
