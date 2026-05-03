// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_decl_ext_keyexpr_inner

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclExtKeyexprInner()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohDeclExtKeyexprInner(
    var inner_header: UByte = 0.toUByte(),
    var id: ULong = 0uL,
    var suffix: ByteArray? = null
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun N(): Boolean = (this.inner_header.toInt() and 0x01) != 0

    fun setN(v: Boolean) {
        this.inner_header = if (v) {
            (this.inner_header.toInt() or 0x01).toUByte()
        } else {
            (this.inner_header.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun M(): Boolean = (this.inner_header.toInt() and 0x02) != 0

    fun setM(v: Boolean) {
        this.inner_header = if (v) {
            (this.inner_header.toInt() or 0x02).toUByte()
        } else {
            (this.inner_header.toInt() and 0x02.inv()).toUByte()
        }
    }

    fun encode(): ByteArray {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        val r = mutableListOf<Byte>()
        r.add(this.inner_header.toByte())
        run {
            var _w: ULong = (id).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        this.suffix?.let { _v ->
            r.addAll(_v.toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohDeclExtKeyexprInner? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val inner_header = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val id = cursor.readVleU64() ?: return null
            val suffix = if ((inner_header.toInt() and 0x01) != 0) {
                val _n = cursor.remaining()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            return CodecZenohDeclExtKeyexprInner(
                inner_header = inner_header,
                id = id,
                suffix = suffix
            )
        }
    }
}
