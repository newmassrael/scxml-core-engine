// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_push_body

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_put.*
import com.sce.generated.codec_zenoh_del.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohPushBodyVariant {
    data class CodecZenohPut(val body: com.sce.generated.codec_zenoh_put.CodecZenohPut) : CodecZenohPushBodyVariant()
    data class CodecZenohDel(val body: com.sce.generated.codec_zenoh_del.CodecZenohDel) : CodecZenohPushBodyVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_put.CodecZenohPut) : CodecZenohPushBodyVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohPushBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohPushBody(
    var header: UByte = 0.toUByte(),
    var body: CodecZenohPushBodyVariant = CodecZenohPushBodyVariant.CodecZenohPut(com.sce.generated.codec_zenoh_put.CodecZenohPut())
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun mid(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x1F).toUByte()
    }

    fun setMid(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x1F shl 0
        val _val = (v.toInt() and 0x1F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun rest(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 5) and 0x07).toUByte()
    }

    fun setRest(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x07 shl 5
        val _val = (v.toInt() and 0x07) shl 5
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun encode(): ByteArray {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        // The tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set tag / body in sync
        // is the caller's responsibility (v1 keeps the layout simple).
        val r = mutableListOf<Byte>()
        r.add(header.toByte())
        // Append the active arm body's encoded bytes.
        when (val _b = this.body) {
            is CodecZenohPushBodyVariant.CodecZenohPut -> r.addAll(_b.body.encode().toList())
            is CodecZenohPushBodyVariant.CodecZenohDel -> r.addAll(_b.body.encode().toList())
            is CodecZenohPushBodyVariant.Default -> r.addAll(_b.body.encode().toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohPushBody? {
            // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return null
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return null
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohPushBodyVariant = when (((header.toInt() shr 0) and 0x1F)) {
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_put.CodecZenohPut.decode(cursor) ?: return null
                    CodecZenohPushBodyVariant.CodecZenohPut(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_del.CodecZenohDel.decode(cursor) ?: return null
                    CodecZenohPushBodyVariant.CodecZenohDel(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_put.CodecZenohPut.decode(cursor) ?: return null
                    CodecZenohPushBodyVariant.Default(tag = ((header.toInt() shr 0) and 0x1F).toUByte(), body = _arm)
                }
            }
            return CodecZenohPushBody(
                header = header,
                body = body
            )
        }
    }
}
