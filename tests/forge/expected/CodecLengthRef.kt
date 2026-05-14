// SCE-MAP: codec_length_ref:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRef()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLengthRef(
    var msgId: UByte = 0.toUByte(),
    var len: UByte = 0.toUByte(),
    var payload: ByteArray = byteArrayOf()
) {
    fun encode(): ByteArray {
        val r = mutableListOf<Byte>()
        r.add(msgId.toByte())
        r.add(len.toByte())
        r.addAll(payload.toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecLengthRef? {
            val frameLen = cursor.remaining()
            if (frameLen < 2) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val value = CodecLengthRef(
                msgId = raw[0].toUByte(),
                len = raw[1].toUByte(),
                payload = raw.copyOfRange(2, 2 + raw[1].toInt())
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
