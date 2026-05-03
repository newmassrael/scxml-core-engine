// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_ext_zint

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohExtZint()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohExtZint(
    var value: ULong = 0uL
) {
    fun encode(): ByteArray {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable.
        val r = mutableListOf<Byte>()
        run {
            var _w: ULong = (value).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohExtZint? {
            // Streaming codec: each field reads from cursor directly
            // (VLE base-128 chain). RFC §5.B B4: per-field bit-size
            // dispatch routes Fixed / LengthRef siblings of VLE fields
            // through `present_if_decode_stmt` (predicate=None arms).
            // Pure-VLE codecs stay byte-stable.
            val value = cursor.readVleU64() ?: return null
            return CodecZenohExtZint(
                value = value
            )
        }
    }
}
