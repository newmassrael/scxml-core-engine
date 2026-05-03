// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_open_body

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohOpenBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohOpenBody(
    var lease: ULong = 0uL,
    var initial_sn: ULong = 0uL,
    var cookie_len: ULong? = null,
    var cookie: ByteArray? = null
) {
    @Suppress("UNUSED_PARAMETER")
    fun encode(parentFlags: UByte): ByteArray {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        val r = mutableListOf<Byte>()
        run {
            var _w: ULong = (lease).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        run {
            var _w: ULong = (initial_sn).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        this.cookie_len?.let { _v ->
        run {
            var _w: ULong = (_v).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        }
        this.cookie?.let { _v ->
            r.addAll(_v.toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, parentFlags: UByte): CodecZenohOpenBody? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val lease = cursor.readVleU64() ?: return null
            val initial_sn = cursor.readVleU64() ?: return null
            val cookie_len: ULong? = if ((parentFlags.toInt() and 0x20) == 0) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val cookie = if ((parentFlags.toInt() and 0x20) == 0) {
                val _n = cookie_len!!.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            return CodecZenohOpenBody(
                lease = lease,
                initial_sn = initial_sn,
                cookie_len = cookie_len,
                cookie = cookie
            )
        }
    }
}
