// SCE-MAP: codec_zenoh_interest_body:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_interest_body

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_wireexpr.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohInterestBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohInterestBody(
    var header: UByte = 0.toUByte(),
    var keyexpr: CodecZenohWireexpr? = null
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun keyexprs(): Boolean = (this.header.toInt() and 0x01) != 0

    fun setKeyexprs(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x01).toUByte()
        } else {
            (this.header.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun subscribers(): Boolean = (this.header.toInt() and 0x02) != 0

    fun setSubscribers(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x02).toUByte()
        } else {
            (this.header.toInt() and 0x02.inv()).toUByte()
        }
    }

    fun queryables(): Boolean = (this.header.toInt() and 0x04) != 0

    fun setQueryables(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x04).toUByte()
        } else {
            (this.header.toInt() and 0x04.inv()).toUByte()
        }
    }

    fun tokens(): Boolean = (this.header.toInt() and 0x08) != 0

    fun setTokens(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x08).toUByte()
        } else {
            (this.header.toInt() and 0x08.inv()).toUByte()
        }
    }

    fun restricted(): Boolean = (this.header.toInt() and 0x10) != 0

    fun setRestricted(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x10).toUByte()
        } else {
            (this.header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun N(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setN(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun M(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setM(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x40).toUByte()
        } else {
            (this.header.toInt() and 0x40.inv()).toUByte()
        }
    }

    fun aggregate(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setAggregate(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
        }
    }

    fun encode(): ByteArray {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        val r = mutableListOf<Byte>()
        r.add(this.header.toByte())
        this.keyexpr?.let { _v ->
            r.addAll(_v.encode((((this.header.toInt() shr 5) and 0x1).toUByte())).toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohInterestBody? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val header = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val keyexpr: CodecZenohWireexpr? = if ((header.toInt() and 0x10) != 0) {
                CodecZenohWireexpr.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())) ?: return null
            } else {
                null
            }
            return CodecZenohInterestBody(
                header = header,
                keyexpr = keyexpr
            )
        }
    }
}
