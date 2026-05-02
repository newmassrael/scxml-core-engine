// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_init_syn_body

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecInitSynBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecInitSynBody(
    var version: UByte = 0.toUByte(),
    var sn_res: UByte? = null,
    var batch_size: UShort? = null
) {
    @Suppress("UNUSED_PARAMETER")
    fun encode(parentFlags: UByte): ByteArray {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        val r = mutableListOf<Byte>()
        r.add(this.version.toByte())
        this.sn_res?.let { _v ->
            r.add(_v.toByte())
        }
        this.batch_size?.let { _v ->
            r.add((_v.toInt() ushr 8 and 0xFF).toByte())
            r.add((_v.toInt() and 0xFF).toByte())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, parentFlags: UByte): CodecInitSynBody? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val version = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val sn_res = if ((parentFlags.toInt() and 0x40) != 0) {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            } else {
                null
            }
            val batch_size = if ((parentFlags.toInt() and 0x40) != 0) {
                val raw = cursor.peekSlice(2) ?: return null
                val _v = (((raw[0].toInt() and 0xFF) shl 8) or (raw[1].toInt() and 0xFF)).toUShort()
                if (!cursor.advance(2)) return null
                _v
            } else {
                null
            }
            return CodecInitSynBody(
                version = version,
                sn_res = sn_res,
                batch_size = batch_size
            )
        }
    }
}
