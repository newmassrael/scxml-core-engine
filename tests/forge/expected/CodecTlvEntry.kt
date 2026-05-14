// SCE-MAP: codec_tlv_entry:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_tlv_entry

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecTlvEntry()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecTlvEntry(
    var entry_type: UByte = 0.toUByte(),
    var entry_len: UByte = 0.toUByte(),
    var entry_body: ByteArray = byteArrayOf()
) {
    fun encode(): ByteArray {
        val r = mutableListOf<Byte>()
        r.add(entry_type.toByte())
        r.add(entry_len.toByte())
        r.addAll(entry_body.toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecTlvEntry? {
            val frameLen = cursor.remaining()
            if (frameLen < 2) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val value = CodecTlvEntry(
                entry_type = raw[0].toUByte(),
                entry_len = raw[1].toUByte(),
                entry_body = raw.copyOfRange(2, 2 + raw[1].toInt())
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
