// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_ext_entry

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_ext_unit.*
import com.sce.generated.codec_zenoh_ext_zint.*
import com.sce.generated.codec_zenoh_ext_zbuf.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohExtEntryVariant {
    data class CodecZenohExtUnit(val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit) : CodecZenohExtEntryVariant()
    data class CodecZenohExtZint(val body: com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint) : CodecZenohExtEntryVariant()
    data class CodecZenohExtZbuf(val body: com.sce.generated.codec_zenoh_ext_zbuf.CodecZenohExtZbuf) : CodecZenohExtEntryVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit) : CodecZenohExtEntryVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohExtEntry()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohExtEntry(
    var header: UByte = 0.toUByte(),
    var body: CodecZenohExtEntryVariant = CodecZenohExtEntryVariant.CodecZenohExtUnit(com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit())
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun extId(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setExtId(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun M(): Boolean = (this.header.toInt() and 0x10) != 0

    fun setM(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x10).toUByte()
        } else {
            (this.header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun enc(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 5) and 0x03).toUByte()
    }

    fun setEnc(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x03 shl 5
        val _val = (v.toInt() and 0x03) shl 5
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun Z(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
        }
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
            is CodecZenohExtEntryVariant.CodecZenohExtUnit -> r.addAll(_b.body.encode().toList())
            is CodecZenohExtEntryVariant.CodecZenohExtZint -> r.addAll(_b.body.encode().toList())
            is CodecZenohExtEntryVariant.CodecZenohExtZbuf -> r.addAll(_b.body.encode().toList())
            is CodecZenohExtEntryVariant.Default -> r.addAll(_b.body.encode().toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohExtEntry? {
            // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return null
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return null
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohExtEntryVariant = when (((header.toInt() shr 5) and 0x03)) {
                0 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor) ?: return null
                    CodecZenohExtEntryVariant.CodecZenohExtUnit(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint.decode(cursor) ?: return null
                    CodecZenohExtEntryVariant.CodecZenohExtZint(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zbuf.CodecZenohExtZbuf.decode(cursor) ?: return null
                    CodecZenohExtEntryVariant.CodecZenohExtZbuf(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor) ?: return null
                    CodecZenohExtEntryVariant.Default(tag = ((header.toInt() shr 5) and 0x03).toUByte(), body = _arm)
                }
            }
            return CodecZenohExtEntry(
                header = header,
                body = body
            )
        }
    }
}
