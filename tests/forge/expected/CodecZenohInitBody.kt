// SCE-MAP: codec_zenoh_init_body:42

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_init_body

import com.sce.forge.runtime.SceCursor

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohInitBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohInitBody(
    var version: UByte = 0.toUByte(),
    var cbyte: UByte = 0.toUByte(),
    var zid: ByteArray = byteArrayOf(),
    var sn_res: UByte? = null,
    var batch_size: UShort? = null,
    var cookie_len: ULong? = null,
    var cookie: ByteArray? = null
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun whatami(): UByte {
        val _carrier = this.cbyte.toInt()
        return ((_carrier shr 0) and 0x03).toUByte()
    }

    fun setWhatami(v: UByte) {
        val _carrier = this.cbyte.toInt()
        val _shifted_mask = 0x03 shl 0
        val _val = (v.toInt() and 0x03) shl 0
        this.cbyte = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun zidLenM1(): UByte {
        val _carrier = this.cbyte.toInt()
        return ((_carrier shr 4) and 0x0F).toUByte()
    }

    fun setZidLenM1(v: UByte) {
        val _carrier = this.cbyte.toInt()
        val _shifted_mask = 0x0F shl 4
        val _val = (v.toInt() and 0x0F) shl 4
        this.cbyte = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    @Suppress("UNUSED_PARAMETER")
    fun encode(S: UByte, A: UByte): ByteArray {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        val r = mutableListOf<Byte>()
        r.add(this.version.toByte())
        r.add(this.cbyte.toByte())
        r.addAll(this.zid.toList())
        this.sn_res?.let { _v ->
            r.add(_v.toByte())
        }
        this.batch_size?.let { _v ->
            r.add((_v.toInt() and 0xFF).toByte())
            r.add((_v.toInt() ushr 8 and 0xFF).toByte())
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
        fun decode(cursor: SceCursor, S: UByte, A: UByte): CodecZenohInitBody? {
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
            val cbyte = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val zid = run {
                val _n = (((cbyte.toInt() shr 4) and 0xF) + 1)
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            }
            val sn_res = if ((S.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            } else {
                null
            }
            val batch_size = if ((S.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(2) ?: return null
                val _v = ((raw[0].toInt() and 0xFF) or ((raw[1].toInt() and 0xFF) shl 8)).toUShort()
                if (!cursor.advance(2)) return null
                _v
            } else {
                null
            }
            val cookie_len: ULong? = if ((A.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val cookie = if ((A.toInt() and 0x01) != 0) {
                val _n = cookie_len!!.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            return CodecZenohInitBody(
                version = version,
                cbyte = cbyte,
                zid = zid,
                sn_res = sn_res,
                batch_size = batch_size,
                cookie_len = cookie_len,
                cookie = cookie
            )
        }
    }
}
