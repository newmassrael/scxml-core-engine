// SCE-MAP: codec_simple_frame:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_simple_frame

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecSimpleFrame()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecSimpleFrame(
    var msgId: UByte = 0.toUByte(),
    var length: UByte = 0.toUByte(),
    var payload: UShort = 0.toUShort()
) {
    fun encode(): ByteArray {
        return byteArrayOf(
            msgId.toByte(),
            length.toByte(),
            (payload.toInt() ushr 8 and 0xFF).toByte(),
            (payload.toInt() and 0xFF).toByte()
        )
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecSimpleFrame? {
            val raw = cursor.peekSlice(4) ?: return null
            val value = CodecSimpleFrame(
                msgId = raw[0].toUByte(),
                length = raw[1].toUByte(),
                payload = (((raw[2].toInt() and 0xFF) shl 8) or (raw[3].toInt() and 0xFF)).toUShort()
            )
            if (!cursor.advance(4)) return null
            return value
        }
    }
}
