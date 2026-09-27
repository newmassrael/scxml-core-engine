// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes
 * (SCE_FORGE.md §4.6.1): one definite-length map whose keys are small unsigned
 * integers and whose values are unsigned integers, booleans, text strings and
 * byte strings.
 *
 * A generated codec calls these and spells no CBOR of its own. **Writing** is
 * deterministic (RFC 8949 §4.2.1: every head in its shortest form), so this
 * runtime and every other write the same bytes for the same value. **Reading**
 * takes a head in any valid length and answers `null` for what the codec cannot
 * read as its entry — Kotlin's codec convention, where every decode failure is
 * `null`; [lastError] says which.
 *
 * Cross-language sibling: `sce_forge_runtime::cbor` (Rust).
 */
object Cbor {
    const val MAJOR_UNSIGNED = 0
    const val MAJOR_BYTES = 2
    const val MAJOR_TEXT = 3
    const val MAJOR_MAP = 5
    const val MAJOR_SIMPLE = 7

    /** The deepest an unknown entry's value may nest before [skip] refuses it. */
    const val MAX_SKIP_DEPTH = 16

    /**
     * Why the last read answered `null`. Diagnostic only — a decode's
     * contract is its `null` — and read by single-threaded tests that pin
     * which rule refused; it is not synchronised, so concurrent decodes
     * overwrite it.
     */
    var lastError: CodecError? = null
        private set

    private fun <T> refuse(error: CodecError): T? {
        lastError = error
        return null
    }

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

    fun writeMapHead(w: SceSink, entries: ULong): CodecError? = writeHead(w, MAJOR_MAP, entries)

    // ── Reading ─────────────────────────────────────────────────────────

    private fun readBe(c: SceCursor, n: Int): ULong? {
        var value = 0UL
        repeat(n) {
            val b = c.readByte() ?: return refuse(CodecError.NeedMoreBytes)
            value = (value shl 8) or b.toULong()
        }
        return value
    }

    /**
     * One head: its major type and the value its additional information
     * carries (for a string or a map, the length). An indefinite length and
     * the reserved values 28–30 are refused.
     */
    fun readHead(c: SceCursor): Pair<Int, ULong>? {
        val initial = c.readByte() ?: return refuse(CodecError.NeedMoreBytes)
        val major = initial ushr 5
        val value = when (val info = initial and 0x1F) {
            in 0..23 -> info.toULong()
            24 -> readBe(c, 1)
            25 -> readBe(c, 2)
            26 -> readBe(c, 4)
            27 -> readBe(c, 8)
            else -> return refuse(CodecError.CborMalformed)
        } ?: return null
        return major to value
    }

    private fun expect(c: SceCursor, major: Int): ULong? {
        val (m, value) = readHead(c) ?: return null
        return if (m == major) value else refuse(CodecError.CborMalformed)
    }

    fun readUint(c: SceCursor): ULong? = expect(c, MAJOR_UNSIGNED)

    /** An unsigned integer an entry of [max] holds. */
    fun readUintUpto(c: SceCursor, max: ULong): ULong? {
        val value = readUint(c) ?: return null
        return if (value > max) refuse(CodecError.CborOutOfRange) else value
    }

    fun readBool(c: SceCursor): Boolean? {
        val (m, value) = readHead(c) ?: return null
        return when {
            m == MAJOR_SIMPLE && value == 20UL -> false
            m == MAJOR_SIMPLE && value == 21UL -> true
            else -> refuse(CodecError.CborMalformed)
        }
    }

    private fun readPayload(c: SceCursor, len: ULong): ByteArray? {
        if (len > c.remaining().toULong()) return refuse(CodecError.NeedMoreBytes)
        val n = len.toInt()
        val bytes = c.peekSlice(n) ?: return refuse(CodecError.NeedMoreBytes)
        c.advance(n)
        return bytes
    }

    /** A text string of at most [maxSize] bytes (`null`: no bound). */
    fun readText(c: SceCursor, maxSize: Int?): String? {
        val len = expect(c, MAJOR_TEXT) ?: return null
        if (maxSize != null && len > maxSize.toULong()) return refuse(CodecError.CborOutOfRange)
        val bytes = readPayload(c, len) ?: return null
        return try {
            bytes.decodeToString(throwOnInvalidSequence = true)
        } catch (_: CharacterCodingException) {
            refuse(CodecError.CborMalformed)
        }
    }

    /** A byte string of at most [maxSize] bytes (`null`: no bound). */
    fun readBytes(c: SceCursor, maxSize: Int?): ByteArray? {
        val len = expect(c, MAJOR_BYTES) ?: return null
        if (maxSize != null && len > maxSize.toULong()) return refuse(CodecError.CborOutOfRange)
        return readPayload(c, len)
    }

    /** A byte string of exactly [length] bytes. */
    fun readBytesExact(c: SceCursor, length: Int): ByteArray? {
        val len = expect(c, MAJOR_BYTES) ?: return null
        if (len != length.toULong()) return refuse(CodecError.CborWrongLength)
        return readPayload(c, len)
    }

    /** The head of a definite-length map: how many entries follow. */
    fun readMapLen(c: SceCursor): ULong? = expect(c, MAJOR_MAP)

    /**
     * Skip one item — the value of a key the codec does not declare — and
     * everything nested in it, refusing one nested deeper than
     * [MAX_SKIP_DEPTH]. `false` when it cannot.
     */
    fun skip(c: SceCursor): Boolean = skipAt(c, 0)

    private fun skipAt(c: SceCursor, depth: Int): Boolean {
        if (depth >= MAX_SKIP_DEPTH) {
            lastError = CodecError.CborTooDeep
            return false
        }
        val (major, value) = readHead(c) ?: return false
        return when (major) {
            0, 1, 7 -> true
            2, 3 -> readPayload(c, value) != null
            4 -> {
                var i = 0UL
                while (i < value) {
                    if (!skipAt(c, depth + 1)) return false
                    i++
                }
                true
            }
            5 -> {
                var i = 0UL
                while (i < value) {
                    if (!skipAt(c, depth + 1) || !skipAt(c, depth + 1)) return false
                    i++
                }
                true
            }
            6 -> skipAt(c, depth + 1)
            else -> {
                lastError = CodecError.CborMalformed
                false
            }
        }
    }
}
