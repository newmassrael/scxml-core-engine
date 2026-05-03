// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_decl_ext_keyexpr

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_decl_ext_keyexpr_inner.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclExtKeyexpr()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohDeclExtKeyexpr(
    var outer_header: UByte = 0.toUByte(),
    var total_length: ULong = 0uL,
    var inner: CodecZenohDeclExtKeyexprInner = byteArrayOf()
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun extId(): UByte {
        val _carrier = this.outer_header.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setExtId(v: UByte) {
        val _carrier = this.outer_header.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.outer_header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun M(): Boolean = (this.outer_header.toInt() and 0x10) != 0

    fun setM(v: Boolean) {
        this.outer_header = if (v) {
            (this.outer_header.toInt() or 0x10).toUByte()
        } else {
            (this.outer_header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun enc(): UByte {
        val _carrier = this.outer_header.toInt()
        return ((_carrier shr 5) and 0x03).toUByte()
    }

    fun setEnc(v: UByte) {
        val _carrier = this.outer_header.toInt()
        val _shifted_mask = 0x03 shl 5
        val _val = (v.toInt() and 0x03) shl 5
        this.outer_header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun Z(): Boolean = (this.outer_header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
        this.outer_header = if (v) {
            (this.outer_header.toInt() or 0x80).toUByte()
        } else {
            (this.outer_header.toInt() and 0x80.inv()).toUByte()
        }
    }

    fun encode(): ByteArray {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable.
        val r = mutableListOf<Byte>()
        r.add(this.outer_header.toByte())
        run {
            var _w: ULong = (total_length).toULong()
            while (_w >= 0x80UL) {
                r.add((_w.toLong() and 0x7F or 0x80).toByte())
                _w = _w shr 7
            }
            r.add(_w.toByte())
        }
        r.addAll(this.inner.encode().toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohDeclExtKeyexpr? {
            // Streaming codec: each field reads from cursor directly
            // (VLE base-128 chain). RFC §5.B B4: per-field bit-size
            // dispatch routes Fixed / LengthRef siblings of VLE fields
            // through `present_if_decode_stmt` (predicate=None arms).
            // Pure-VLE codecs stay byte-stable.
            val outer_header = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val total_length = cursor.readVleU64() ?: return null
            val inner = run {
                val _len = (total_length).toInt()
                val _raw = cursor.peekSlice(_len) ?: return null
                val _inner = SceCursor(_raw)
                val _v = CodecZenohDeclExtKeyexprInner.decode(_inner) ?: return null
                if (!cursor.advance(_len)) return null
                _v
            }
            return CodecZenohDeclExtKeyexpr(
                outer_header = outer_header,
                total_length = total_length,
                inner = inner
            )
        }
    }
}
