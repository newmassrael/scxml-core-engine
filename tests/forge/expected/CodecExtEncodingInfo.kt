// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_ext_encoding_info

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecExtEncodingInfo()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecExtEncodingInfo(
    var combined_id: UInt = 0u,
    var schema_size: UByte = 0.toUByte(),
    var schema: ByteArray? = null
) {
    // RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    // field. Kotlin's UByte / UShort / UInt / ULong don't expose direct
    // bitwise infix ops with literal masks, so the body widens through
    // `.toInt()` (UByte/UShort) or `.toLong()` (UInt/ULong), runs the
    // bit op against the Int/Long mask, then narrows back via the
    // carrier's `toU*` constructor.
    fun hasSchema(): Boolean = (this.combined_id.toLong() and 0x00000001) != 0

    fun setHasSchema(v: Boolean) {
        this.combined_id = if (v) {
            (this.combined_id.toLong() or 0x00000001).toUInt()
        } else {
            (this.combined_id.toLong() and 0x00000001.inv()).toUInt()
        }
    }

    fun encode(): ByteArray {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        val r = mutableListOf<Byte>()
        run {
            var _w: ULong = (combined_id).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        r.add(this.schema_size.toByte())
        this.schema?.let { _v ->
            r.addAll(_v.toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecExtEncodingInfo? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val combined_id = cursor.readVleU32() ?: return null
            val schema_size = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val schema = if ((combined_id.toLong() and 0x00000001L) != 0) {
                val _n = schema_size.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            return CodecExtEncodingInfo(
                combined_id = combined_id,
                schema_size = schema_size,
                schema = schema
            )
        }
    }
}
