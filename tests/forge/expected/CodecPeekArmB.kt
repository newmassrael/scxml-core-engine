// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_peek_arm_b

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecPeekArmB()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecPeekArmB(
    var header: UByte = 0.toUByte(),
    var payload: UShort = 0.toUShort()
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun kind(): Boolean = (this.header.toInt() and 0x01) != 0

    fun setKind(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x01).toUByte()
        } else {
            (this.header.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun encode(): ByteArray {
        return byteArrayOf(
            header.toByte(),
            (payload.toInt() ushr 8 and 0xFF).toByte(),
            (payload.toInt() and 0xFF).toByte()
        )
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecPeekArmB? {
            val raw = cursor.peekSlice(3) ?: return null
            val value = CodecPeekArmB(
                header = raw[0].toUByte(),
                payload = (((raw[1].toInt() and 0xFF) shl 8) or (raw[2].toInt() and 0xFF)).toUShort()
            )
            if (!cursor.advance(3)) return null
            return value
        }
    }
}
