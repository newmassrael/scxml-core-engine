// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_vle_zint_u64

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecVleZintU64()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecVleZintU64(
    var value: ULong = 0uL
) {
    fun encode(): ByteArray {
        val r = mutableListOf<Byte>()
        run {
            var _v: ULong = value.toULong()
            while (_v >= 0x80UL) {
                r.add((_v.toLong() and 0x7F or 0x80).toByte())
                _v = _v shr 7
            }
            r.add(_v.toByte())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecVleZintU64? {
            // Streaming codec: each field reads from cursor directly
            // (VLE base-128 chain).
            val value = cursor.readVleU64() ?: return null
            return CodecVleZintU64(
                value = value
            )
        }
    }
}
