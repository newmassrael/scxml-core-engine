// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes
 * (SCE_FORGE.md §4.6.1): one definite-length map whose keys are small unsigned
 * integers and whose values are unsigned integers, booleans, text strings and
 * byte strings, or a definite-length array of maps, each a record keyed by its
 * fields' ids as text (docs/adr/0086, section 9.2).
 *
 * A generated codec calls these and spells no CBOR of its own. **Writing** is
 * deterministic (RFC 8949 §4.2.1: every head in its shortest form), so this
 * runtime and every other write the same bytes for the same value. **Reading**
 * takes a head in any valid length and answers a [Decoded]: the item, or the
 * [CodecError] that refuses what the codec cannot read as its entry
 * (docs/adr/0099, amendment of 2026-11-16). The refusal is the answer of the
 * read that met it, so concurrent decodes never see each other's.
 *
 * Cross-language sibling: `sce_forge_runtime::cbor` (Rust).
 */
object Cbor {
    const val MAJOR_UNSIGNED = 0
    const val MAJOR_BYTES = 2
    const val MAJOR_TEXT = 3
    const val MAJOR_ARRAY = 4
    const val MAJOR_MAP = 5
    const val MAJOR_SIMPLE = 7

    /** The deepest an unknown entry's value may nest before [skip] refuses it. */
    const val MAX_SKIP_DEPTH = 16

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

    fun writeBool(w: SceSink, value: Boolean): CodecError? =
        w.writeU8(((MAJOR_SIMPLE shl 5) or if (value) 21 else 20).toByte())

    fun writeText(w: SceSink, value: String): CodecError? {
        val bytes = value.encodeToByteArray()
        return writeHead(w, MAJOR_TEXT, bytes.size.toULong()) ?: w.writeBytes(bytes)
    }

    fun writeBytes(w: SceSink, value: ByteArray): CodecError? =
        writeHead(w, MAJOR_BYTES, value.size.toULong()) ?: w.writeBytes(value)

    /** The head of a definite-length array of [items] items: a list of records an entry holds. */
    fun writeArrayHead(w: SceSink, items: ULong): CodecError? = writeHead(w, MAJOR_ARRAY, items)

    fun writeMapHead(w: SceSink, entries: ULong): CodecError? = writeHead(w, MAJOR_MAP, entries)

    // ── Reading ─────────────────────────────────────────────────────────

    /** One head: its major type and the value its additional information carries. */
    data class Head(val major: Int, val value: ULong)

    private fun readBe(c: SceCursor, n: Int): Decoded<ULong> {
        var value = 0UL
        repeat(n) {
            val b = c.readByte() ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            value = (value shl 8) or b.toULong()
        }
        return Decoded.Ok(value)
    }

    /**
     * One head: its major type and the value its additional information
     * carries (for a string or a map, the length). An indefinite length and
     * the reserved values 28–30 are refused.
     */
    fun readHead(c: SceCursor): Decoded<Head> {
        val initial = c.readByte() ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        val major = initial ushr 5
        val value = when (val info = initial and 0x1F) {
            in 0..23 -> info.toULong()
            24 -> readBe(c, 1).valueOr { refused -> return refused }
            25 -> readBe(c, 2).valueOr { refused -> return refused }
            26 -> readBe(c, 4).valueOr { refused -> return refused }
            27 -> readBe(c, 8).valueOr { refused -> return refused }
            else -> return Decoded.Refused(CodecError.CborMalformed)
        }
        return Decoded.Ok(Head(major, value))
    }

    private fun expect(c: SceCursor, major: Int): Decoded<ULong> {
        val head = readHead(c).valueOr { refused -> return refused }
        return if (head.major == major) Decoded.Ok(head.value) else Decoded.Refused(CodecError.CborMalformed)
    }

    fun readUint(c: SceCursor): Decoded<ULong> = expect(c, MAJOR_UNSIGNED)

    /** An unsigned integer an entry of [max] holds. */
    fun readUintUpto(c: SceCursor, max: ULong): Decoded<ULong> {
        val value = readUint(c).valueOr { refused -> return refused }
        return if (value > max) Decoded.Refused(CodecError.CborOutOfRange) else Decoded.Ok(value)
    }

    fun readBool(c: SceCursor): Decoded<Boolean> {
        val head = readHead(c).valueOr { refused -> return refused }
        return when {
            head.major == MAJOR_SIMPLE && head.value == 20UL -> Decoded.Ok(false)
            head.major == MAJOR_SIMPLE && head.value == 21UL -> Decoded.Ok(true)
            else -> Decoded.Refused(CodecError.CborMalformed)
        }
    }

    private fun readPayload(c: SceCursor, len: ULong): Decoded<ByteArray> {
        if (len > c.remaining().toULong()) return Decoded.Refused(CodecError.NeedMoreBytes)
        val n = len.toInt()
        val bytes = c.peekSlice(n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        c.advance(n)
        return Decoded.Ok(bytes)
    }

    /** A text string of at most [maxSize] bytes (`null`: no bound). */
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

    /** A byte string of at most [maxSize] bytes (`null`: no bound). */
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
     * Skip one item — the value of a key the codec does not declare — and
     * everything nested in it, refusing one nested deeper than
     * [MAX_SKIP_DEPTH] as [CodecError.CborTooDeep].
     */
    fun skip(c: SceCursor): Decoded<Unit> = skipAt(c, 0)

    private fun skipAt(c: SceCursor, depth: Int): Decoded<Unit> {
        if (depth >= MAX_SKIP_DEPTH) return Decoded.Refused(CodecError.CborTooDeep)
        val head = readHead(c).valueOr { refused -> return refused }
        when (head.major) {
            0, 1, 7 -> {}
            2, 3 -> readPayload(c, head.value).valueOr { refused -> return refused }
            4 -> {
                var i = 0UL
                while (i < head.value) {
                    skipAt(c, depth + 1).valueOr { refused -> return refused }
                    i++
                }
            }
            5 -> {
                var i = 0UL
                while (i < head.value) {
                    skipAt(c, depth + 1).valueOr { refused -> return refused }
                    skipAt(c, depth + 1).valueOr { refused -> return refused }
                    i++
                }
            }
            6 -> skipAt(c, depth + 1).valueOr { refused -> return refused }
            else -> return Decoded.Refused(CodecError.CborMalformed)
        }
        return Decoded.Ok(Unit)
    }
}
