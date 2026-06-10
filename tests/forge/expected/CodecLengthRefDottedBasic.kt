// SCE-MAP: codec_length_ref_dotted_basic:27

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref_dotted_basic

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRefDottedBasic()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLengthRefDottedBasic(
    var carrier: UByte = 0.toUByte(),
    var payload: ByteArray = byteArrayOf()
) {
    // RFC §5.B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun hdr(): UByte {
        val _carrier = this.carrier.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setHdr(v: UByte) {
        val _carrier = this.carrier.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.carrier = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun payloadLen(): UByte {
        val _carrier = this.carrier.toInt()
        return ((_carrier shr 4) and 0x0F).toUByte()
    }

    fun setPayloadLen(v: UByte) {
        val _carrier = this.carrier.toInt()
        val _shifted_mask = 0x0F shl 4
        val _val = (v.toInt() and 0x0F) shl 4
        this.carrier = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    /// RFC §5.B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8(carrier.toByte())?.let { return it }
        w.writeBytes(payload)?.let { return it }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list))
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecLengthRefDottedBasic? {
            val frameLen = cursor.remaining()
            if (frameLen < 1) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val carrier = raw[0].toUByte()
            val payload = raw.copyOfRange(1, 1 + ((carrier.toInt() ushr 4) and 0xF))
            val value = CodecLengthRefDottedBasic(
                carrier = carrier,
                payload = payload
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
