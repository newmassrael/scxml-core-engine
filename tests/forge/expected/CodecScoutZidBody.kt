// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_scout_zid_body

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecScoutZidBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecScoutZidBody(
    var zid_len_m1: UByte = 0.toUByte(),
    var zid: ByteArray = byteArrayOf()
) {
    fun encode(): ByteArray {
        val r = mutableListOf<Byte>()
        r.add(zid_len_m1.toByte())
        r.addAll(zid.toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecScoutZidBody? {
            val frameLen = cursor.remaining()
            if (frameLen < 1) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val value = CodecScoutZidBody(
                zid_len_m1 = raw[0].toUByte(),
                zid = raw.copyOfRange(1, 1 + raw[0].toInt() + 1)
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
