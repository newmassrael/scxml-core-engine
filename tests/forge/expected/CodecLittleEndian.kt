// SCE-MAP: codec_little_endian:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_little_endian

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLittleEndian()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLittleEndian(
    var sensorId: UByte = 0.toUByte(),
    var value: UShort = 0.toUShort(),
    var status: UByte = 0.toUByte()
) {
    fun encode(): ByteArray {
        return byteArrayOf(
            sensorId.toByte(),
            (value.toInt() and 0xFF).toByte(),
            (value.toInt() ushr 8 and 0xFF).toByte(),
            status.toByte()
        )
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecLittleEndian? {
            val raw = cursor.peekSlice(4) ?: return null
            val value = CodecLittleEndian(
                sensorId = raw[0].toUByte(),
                value = ((raw[1].toInt() and 0xFF) or ((raw[2].toInt() and 0xFF) shl 8)).toUShort(),
                status = raw[3].toUByte()
            )
            if (!cursor.advance(4)) return null
            return value
        }
    }
}
