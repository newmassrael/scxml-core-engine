// SCE-MAP: codec_length_ref_uint16_le:12

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref_uint16_le

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRefUint16Le()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLengthRefUint16Le(
    var payload_len: UShort = 0.toUShort(),
    var payload: ByteArray = byteArrayOf()
) {
    fun encode(): ByteArray {
        val r = mutableListOf<Byte>()
        r.add((payload_len.toInt() and 0xFF).toByte())
        r.add((payload_len.toInt() ushr 8 and 0xFF).toByte())
        r.addAll(payload.toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecLengthRefUint16Le? {
            val frameLen = cursor.remaining()
            if (frameLen < 2) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val payload_len = ((raw[0].toInt() and 0xFF) or ((raw[1].toInt() and 0xFF) shl 8)).toUShort()
            val payload = raw.copyOfRange(2, 2 + payload_len.toInt())
            val value = CodecLengthRefUint16Le(
                payload_len = payload_len,
                payload = payload
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
