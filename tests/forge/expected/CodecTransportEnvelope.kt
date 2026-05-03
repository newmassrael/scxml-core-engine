// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_transport_envelope

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_init_body.*
import com.sce.generated.codec_zenoh_open_body.*
import com.sce.generated.codec_zenoh_close.*
import com.sce.generated.codec_zenoh_keep_alive.*
import com.sce.generated.codec_zenoh_frame.*
import com.sce.generated.codec_zenoh_fragment.*
import com.sce.generated.codec_zenoh_join.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecTransportEnvelopeVariant {
    data class CodecZenohInitBody(val body: com.sce.generated.codec_zenoh_init_body.CodecZenohInitBody) : CodecTransportEnvelopeVariant()
    data class CodecZenohOpenBody(val body: com.sce.generated.codec_zenoh_open_body.CodecZenohOpenBody) : CodecTransportEnvelopeVariant()
    data class CodecZenohClose(val body: com.sce.generated.codec_zenoh_close.CodecZenohClose) : CodecTransportEnvelopeVariant()
    data class CodecZenohKeepAlive(val body: com.sce.generated.codec_zenoh_keep_alive.CodecZenohKeepAlive) : CodecTransportEnvelopeVariant()
    data class CodecZenohFrame(val body: com.sce.generated.codec_zenoh_frame.CodecZenohFrame) : CodecTransportEnvelopeVariant()
    data class CodecZenohFragment(val body: com.sce.generated.codec_zenoh_fragment.CodecZenohFragment) : CodecTransportEnvelopeVariant()
    data class CodecZenohJoin(val body: com.sce.generated.codec_zenoh_join.CodecZenohJoin) : CodecTransportEnvelopeVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_close.CodecZenohClose) : CodecTransportEnvelopeVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecTransportEnvelope()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecTransportEnvelope(
    var header: UByte = 0.toUByte(),
    var body: CodecTransportEnvelopeVariant = CodecTransportEnvelopeVariant.CodecZenohInitBody(com.sce.generated.codec_zenoh_init_body.CodecZenohInitBody())
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

    fun A(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setA(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun S(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setS(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x40).toUByte()
        } else {
            (this.header.toInt() and 0x40.inv()).toUByte()
        }
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
            is CodecTransportEnvelopeVariant.CodecZenohInitBody -> r.addAll(_b.body.encode(this.header).toList())
            is CodecTransportEnvelopeVariant.CodecZenohOpenBody -> r.addAll(_b.body.encode(this.header).toList())
            is CodecTransportEnvelopeVariant.CodecZenohClose -> r.addAll(_b.body.encode().toList())
            is CodecTransportEnvelopeVariant.CodecZenohKeepAlive -> r.addAll(_b.body.encode().toList())
            is CodecTransportEnvelopeVariant.CodecZenohFrame -> r.addAll(_b.body.encode().toList())
            is CodecTransportEnvelopeVariant.CodecZenohFragment -> r.addAll(_b.body.encode().toList())
            is CodecTransportEnvelopeVariant.CodecZenohJoin -> r.addAll(_b.body.encode(this.header).toList())
            is CodecTransportEnvelopeVariant.Default -> r.addAll(_b.body.encode().toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecTransportEnvelope? {
            // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return null
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return null
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecTransportEnvelopeVariant = when (((header.toInt() shr 0) and 0x1F)) {
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_init_body.CodecZenohInitBody.decode(cursor, header) ?: return null
                    CodecTransportEnvelopeVariant.CodecZenohInitBody(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_open_body.CodecZenohOpenBody.decode(cursor, header) ?: return null
                    CodecTransportEnvelopeVariant.CodecZenohOpenBody(_arm)
                }
                3 -> {
                    val _arm = com.sce.generated.codec_zenoh_close.CodecZenohClose.decode(cursor) ?: return null
                    CodecTransportEnvelopeVariant.CodecZenohClose(_arm)
                }
                4 -> {
                    val _arm = com.sce.generated.codec_zenoh_keep_alive.CodecZenohKeepAlive.decode(cursor) ?: return null
                    CodecTransportEnvelopeVariant.CodecZenohKeepAlive(_arm)
                }
                5 -> {
                    val _arm = com.sce.generated.codec_zenoh_frame.CodecZenohFrame.decode(cursor) ?: return null
                    CodecTransportEnvelopeVariant.CodecZenohFrame(_arm)
                }
                6 -> {
                    val _arm = com.sce.generated.codec_zenoh_fragment.CodecZenohFragment.decode(cursor) ?: return null
                    CodecTransportEnvelopeVariant.CodecZenohFragment(_arm)
                }
                7 -> {
                    val _arm = com.sce.generated.codec_zenoh_join.CodecZenohJoin.decode(cursor, header) ?: return null
                    CodecTransportEnvelopeVariant.CodecZenohJoin(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_close.CodecZenohClose.decode(cursor) ?: return null
                    CodecTransportEnvelopeVariant.Default(tag = ((header.toInt() shr 0) and 0x1F).toUByte(), body = _arm)
                }
            }
            return CodecTransportEnvelope(
                header = header,
                body = body
            )
        }
    }
}
