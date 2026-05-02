// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_present_if_vle

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecPresentIfVle()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecPresentIfVle(
    var flags: UByte = 0.toUByte(),
    var optional_id: ULong? = null
) {
    // RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    // field. Kotlin's UByte / UShort / UInt / ULong don't expose direct
    // bitwise infix ops with literal masks, so the body widens through
    // `.toInt()` (UByte/UShort) or `.toLong()` (UInt/ULong), runs the
    // bit op against the Int/Long mask, then narrows back via the
    // carrier's `toU*` constructor.
    fun hasId(): Boolean = (this.flags.toInt() and 0x01) != 0

    fun setHasId(v: Boolean) {
        this.flags = if (v) {
            (this.flags.toInt() or 0x01).toUByte()
        } else {
            (this.flags.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun encode(): ByteArray {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        val r = mutableListOf<Byte>()
        r.add(this.flags.toByte())
        this.optional_id?.let { _v ->
        run {
            var _w: ULong = (_v).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecPresentIfVle? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val flags = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val optional_id: ULong? = if ((flags.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            return CodecPresentIfVle(
                flags = flags,
                optional_id = optional_id
            )
        }
    }
}
