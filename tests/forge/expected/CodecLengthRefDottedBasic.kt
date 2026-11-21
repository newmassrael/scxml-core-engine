// SCE-MAP: codec_length_ref_dotted_basic:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref_dotted_basic

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `payload` element of [CodecLengthRefDottedBasic], 0 to 15 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecLengthRefDottedBasicPayloadBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Int = 15

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecLengthRefDottedBasicPayloadBytes? =
            if (bytes.size <= MOST) CodecLengthRefDottedBasicPayloadBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecLengthRefDottedBasicPayloadBytes = CodecLengthRefDottedBasicPayloadBytes(ByteArray(LEAST))
    }
}

/// The elements a value of [CodecLengthRefDottedBasic] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecLengthRefDottedBasic.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecLengthRefDottedBasicSource {
    val carrier: UByte
    val payload: CodecLengthRefDottedBasicPayloadBytes
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRefDottedBasic()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecLengthRefDottedBasic(
    override var carrier: UByte = 0.toUByte(),
    override var payload: CodecLengthRefDottedBasicPayloadBytes = CodecLengthRefDottedBasicPayloadBytes.fewest()
) : CodecLengthRefDottedBasicSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
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

    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        return encodeSource(this, w)
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
        /// Write any source of [CodecLengthRefDottedBasic] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecLengthRefDottedBasicSource, w: SceSink): CodecError? {
            val _e_carrier = src.carrier
            val _e_payload = src.payload
            var _u_carrier = _e_carrier.toUInt()
            _u_carrier = (_u_carrier and 0xF0u.inv()) or ((_e_payload.bytes.size).toUInt() shl 4)
            val _c_carrier = _u_carrier.toUByte()
            w.writeU8(_c_carrier.toByte())?.let { return it }
            w.writeBytes(_e_payload.bytes)?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecLengthRefDottedBasic> {
            val frameLen = cursor.remaining()
            if (frameLen < 1) return Decoded.Refused(CodecError.NeedMoreBytes)
            val raw = cursor.peekSlice(frameLen) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val carrier = raw[0].toUByte()
            if (frameLen < 1 + ((carrier.toInt() ushr 4) and 0xF)) return Decoded.Refused(CodecError.NeedMoreBytes)
            val payload = raw.copyOfRange(1, 1 + ((carrier.toInt() ushr 4) and 0xF))
            // The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
            // does): the end of the last length-ref field, at least the fixed
            // fields' bytes, so that a codec holding this one as a body, an entry
            // or an arm decodes on after it.
            var _consumed = 1
            if (1 + ((carrier.toInt() ushr 4) and 0xF) > _consumed) _consumed = 1 + ((carrier.toInt() ushr 4) and 0xF)
            if (!cursor.advance(_consumed)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return Decoded.Ok(
                CodecLengthRefDottedBasic(
                    carrier = carrier,
                    payload = CodecLengthRefDottedBasicPayloadBytes.of(payload) ?: return Decoded.Refused(CodecError.LengthOutOfRange)
                )
            )
        }
    }
}
