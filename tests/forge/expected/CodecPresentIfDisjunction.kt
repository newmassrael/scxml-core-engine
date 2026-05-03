// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_present_if_disjunction

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecPresentIfDisjunction()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecPresentIfDisjunction(
    var flags: UByte = 0.toUByte(),
    var seq: UShort? = null
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun wantsA(): Boolean = (this.flags.toInt() and 0x01) != 0

    fun setWantsA(v: Boolean) {
        this.flags = if (v) {
            (this.flags.toInt() or 0x01).toUByte()
        } else {
            (this.flags.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun wantsB(): Boolean = (this.flags.toInt() and 0x02) != 0

    fun setWantsB(v: Boolean) {
        this.flags = if (v) {
            (this.flags.toInt() or 0x02).toUByte()
        } else {
            (this.flags.toInt() and 0x02.inv()).toUByte()
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
        this.seq?.let { _v ->
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
        fun decode(cursor: SceCursor): CodecPresentIfDisjunction? {
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
            val seq = if ((flags.toInt() and 0x01) != 0 || (flags.toInt() and 0x02) != 0) {
                val raw = cursor.peekSlice(2) ?: return null
                val _v = (((raw[0].toInt() and 0xFF) shl 8) or (raw[1].toInt() and 0xFF)).toUShort()
                if (!cursor.advance(2)) return null
                _v
            } else {
                null
            }
            return CodecPresentIfDisjunction(
                flags = flags,
                seq = seq
            )
        }
    }
}
