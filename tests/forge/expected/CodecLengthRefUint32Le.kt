// SCE-MAP: codec_length_ref_uint32_le:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref_uint32_le

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRefUint32Le()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLengthRefUint32Le(
    var payload_len: UInt = 0u,
    var payload: ByteArray = byteArrayOf()
) {
    fun encode(): ByteArray {
        val r = mutableListOf<Byte>()
        r.add((payload_len.toInt() and 0xFF).toByte())
        r.add((payload_len.toInt() ushr 8 and 0xFF).toByte())
        r.add((payload_len.toInt() ushr 16 and 0xFF).toByte())
        r.add((payload_len.toInt() ushr 24 and 0xFF).toByte())
        r.addAll(payload.toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecLengthRefUint32Le? {
            val frameLen = cursor.remaining()
            if (frameLen < 4) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val payload_len = ((raw[0].toInt() and 0xFF) or ((raw[1].toInt() and 0xFF) shl 8) or ((raw[2].toInt() and 0xFF) shl 16) or ((raw[3].toInt() and 0xFF) shl 24)).toUInt()
            val payload = raw.copyOfRange(4, 4 + payload_len.toInt())
            val value = CodecLengthRefUint32Le(
                payload_len = payload_len,
                payload = payload
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
