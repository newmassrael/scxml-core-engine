// SCE-MAP: codec_zenoh_timestamp_ext:48

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_timestamp_ext

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_timestamp.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohTimestampExt()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohTimestampExt(
    var ext_size: ULong = 0uL,
    var ts: CodecZenohTimestamp = CodecZenohTimestamp()
) {
    fun encode(): ByteArray {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable.
        val r = mutableListOf<Byte>()
        run {
            var _w: ULong = (ext_size).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        r.addAll(this.ts.encode().toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohTimestampExt? {
            // Streaming codec: each field reads from cursor directly
            // (VLE base-128 chain). RFC §5.B B4: per-field bit-size
            // dispatch routes Fixed / LengthRef siblings of VLE fields
            // through `present_if_decode_stmt` (predicate=None arms).
            // Pure-VLE codecs stay byte-stable.
            val ext_size = cursor.readVleU64() ?: return null
            val ts = run {
                val _len = (ext_size).toInt()
                val _raw = cursor.peekSlice(_len) ?: return null
                val _inner = SceCursor(_raw)
                val _v = CodecZenohTimestamp.decode(_inner) ?: return null
                if (!cursor.advance(_len)) return null
                _v
            }
            return CodecZenohTimestampExt(
                ext_size = ext_size,
                ts = ts
            )
        }
    }
}
