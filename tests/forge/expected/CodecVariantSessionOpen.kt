// SCE-MAP: codec_variant_session_open:5

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_variant_session_open

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecVariantSessionOpen()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecVariantSessionOpen(
    var version: UShort = 0.toUShort()
) {
    fun encode(): ByteArray {
        return byteArrayOf(
            (version.toInt() ushr 8 and 0xFF).toByte(),
            (version.toInt() and 0xFF).toByte()
        )
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecVariantSessionOpen? {
            val raw = cursor.peekSlice(2) ?: return null
            val version = (((raw[0].toInt() and 0xFF) shl 8) or (raw[1].toInt() and 0xFF)).toUShort()
            val value = CodecVariantSessionOpen(
                version = version
            )
            if (!cursor.advance(2)) return null
            return value
        }
    }
}
