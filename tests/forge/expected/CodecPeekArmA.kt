// SCE-MAP: codec_peek_arm_a:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_peek_arm_a

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecPeekArmA()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecPeekArmA(
    var header: UByte = 0.toUByte(),
    var payload: UByte = 0.toUByte()
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
            payload.toByte()
        )
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecPeekArmA? {
            val raw = cursor.peekSlice(2) ?: return null
            val value = CodecPeekArmA(
                header = raw[0].toUByte(),
                payload = raw[1].toUByte()
            )
            if (!cursor.advance(2)) return null
            return value
        }
    }
}
