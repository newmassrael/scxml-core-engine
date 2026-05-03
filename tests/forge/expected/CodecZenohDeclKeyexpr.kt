// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_decl_keyexpr

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_wireexpr.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclKeyexpr()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohDeclKeyexpr(
    var id: UShort = 0.toUShort(),
    var wireexpr: CodecZenohWireexpr = byteArrayOf()
) {
    @Suppress("UNUSED_PARAMETER")
    fun encode(parentFlags: UByte): ByteArray {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable.
        val r = mutableListOf<Byte>()
        run {
            var _w: ULong = (id).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        r.addAll(this.wireexpr.encode(parentFlags).toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, parentFlags: UByte): CodecZenohDeclKeyexpr? {
            // Streaming codec: each field reads from cursor directly
            // (VLE base-128 chain). RFC §5.B B4: per-field bit-size
            // dispatch routes Fixed / LengthRef siblings of VLE fields
            // through `present_if_decode_stmt` (predicate=None arms).
            // Pure-VLE codecs stay byte-stable.
            val id = cursor.readVleU16() ?: return null
            val wireexpr = CodecZenohWireexpr.decode(cursor, parentFlags) ?: return null
            return CodecZenohDeclKeyexpr(
                id = id,
                wireexpr = wireexpr
            )
        }
    }
}
