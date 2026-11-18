// SCE-MAP: codec_length_ref:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `payload` element of [CodecLengthRef], 0 to 32 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecLengthRefPayloadBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Int = 32

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecLengthRefPayloadBytes? =
            if (bytes.size <= MOST) CodecLengthRefPayloadBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecLengthRefPayloadBytes = CodecLengthRefPayloadBytes(ByteArray(LEAST))
    }
}

/// The elements a value of [CodecLengthRef] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecLengthRef.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecLengthRefSource {
    val msgId: UByte
    val payload: CodecLengthRefPayloadBytes
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRef()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecLengthRef(
    override var msgId: UByte = 0.toUByte(),
    override var payload: CodecLengthRefPayloadBytes = CodecLengthRefPayloadBytes.fewest()
) : CodecLengthRefSource {
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
        /// Write any source of [CodecLengthRef] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecLengthRefSource, w: SceSink): CodecError? {
            val _e_msgId = src.msgId
            val _e_payload = src.payload
            val _d_len = _e_payload.bytes.size.toUByte()
            w.writeU8(_e_msgId.toByte())?.let { return it }
            w.writeU8(_d_len.toByte())?.let { return it }
            w.writeBytes(_e_payload.bytes)?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecLengthRef> {
            val frameLen = cursor.remaining()
            if (frameLen < 2) return Decoded.Refused(CodecError.NeedMoreBytes)
            val raw = cursor.peekSlice(frameLen) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val msgId = raw[0].toUByte()
            val len = raw[1].toUByte()
            if (len > 32u) {
                return Decoded.Refused(CodecError.LengthOutOfRange)
            }
            if (frameLen < 2 + len.toInt()) return Decoded.Refused(CodecError.NeedMoreBytes)
            val payload = raw.copyOfRange(2, 2 + len.toInt())
            // The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
            // does): the end of the last length-ref field, at least the fixed
            // fields' bytes, so that a codec holding this one as a body, an entry
            // or an arm decodes on after it.
            var _consumed = 2
            if (2 + len.toInt() > _consumed) _consumed = 2 + len.toInt()
            if (!cursor.advance(_consumed)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return Decoded.Ok(
                CodecLengthRef(
                    msgId = msgId,
                    payload = CodecLengthRefPayloadBytes.of(payload) ?: return Decoded.Refused(CodecError.LengthOutOfRange)
                )
            )
        }
    }
}
