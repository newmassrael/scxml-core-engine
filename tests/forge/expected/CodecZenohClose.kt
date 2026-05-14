// SCE-MAP: codec_zenoh_close:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_close

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohClose()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohClose(
    var reason: UByte = 0.toUByte()
) {
    fun encode(): ByteArray {
        return byteArrayOf(
            reason.toByte()
        )
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohClose? {
            val raw = cursor.peekSlice(1) ?: return null
            val value = CodecZenohClose(
                reason = raw[0].toUByte()
            )
            if (!cursor.advance(1)) return null
            return value
        }
    }
}
