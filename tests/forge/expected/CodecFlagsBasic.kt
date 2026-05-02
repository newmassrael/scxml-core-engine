// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_flags_basic

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecFlagsBasic()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecFlagsBasic(
    var header: UByte = 0.toUByte()
) {
    // RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    // field. Kotlin's UByte / UShort / UInt / ULong don't expose direct
    // bitwise infix ops with literal masks, so the body widens through
    // `.toInt()` (UByte/UShort) or `.toLong()` (UInt/ULong), runs the
    // bit op against the Int/Long mask, then narrows back via the
    // carrier's `toU*` constructor.
    fun reliable(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setReliable(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
        }
    }

    fun more(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setMore(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x40).toUByte()
        } else {
            (this.header.toInt() and 0x40.inv()).toUByte()
        }
    }

    fun drop(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setDrop(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun first(): Boolean = (this.header.toInt() and 0x10) != 0

    fun setFirst(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x10).toUByte()
        } else {
            (this.header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun encode(): ByteArray {
        return byteArrayOf(
            header.toByte()
        )
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecFlagsBasic? {
            val raw = cursor.peekSlice(1) ?: return null
            val value = CodecFlagsBasic(
                header = raw[0].toUByte()
            )
            if (!cursor.advance(1)) return null
            return value
        }
    }
}
