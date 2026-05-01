// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_subbyte

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecSubbyte()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecSubbyte(
    var priority: UByte = 0.toUByte(),
    var channel: UByte = 0.toUByte(),
    var direction: UByte = 0.toUByte()
) {
    fun encode(): ByteArray = byteArrayOf(
        ((priority.toInt() and 0x07 shl 5) or (channel.toInt() and 0x07 shl 2) or (direction.toInt() and 0x03 shl 0)).toByte()
    )

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecSubbyte? {
            val raw = cursor.peekSlice(1) ?: return null
            val value = CodecSubbyte(
                priority = ((raw[0].toInt() ushr 5) and 0x07).toUByte(),
                channel = ((raw[0].toInt() ushr 2) and 0x07).toUByte(),
                direction = ((raw[0].toInt() ushr 0) and 0x03).toUByte()
            )
            if (!cursor.advance(1)) return null
            return value
        }
    }
}
