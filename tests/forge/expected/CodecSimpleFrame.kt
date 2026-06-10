// SCE-MAP: codec_simple_frame:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_simple_frame

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecSimpleFrame()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecSimpleFrame(
    var msgId: UByte = 0.toUByte(),
    var length: UByte = 0.toUByte(),
    var payload: UShort = 0.toUShort()
) {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8(msgId.toByte())?.let { return it }
        w.writeU8(length.toByte())?.let { return it }
        w.writeU8((payload.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
        w.writeU8((payload.toInt() and 0xFF).toByte())?.let { return it }
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
        /// (RFC §synth-5-B L494-519).
        fun decode(cursor: SceCursor): CodecSimpleFrame? {
            val raw = cursor.peekSlice(4) ?: return null
            val msgId = raw[0].toUByte()
            val length = raw[1].toUByte()
            val payload = (((raw[2].toInt() and 0xFF) shl 8) or (raw[3].toInt() and 0xFF)).toUShort()
            val value = CodecSimpleFrame(
                msgId = msgId,
                length = length,
                payload = payload
            )
            if (!cursor.advance(4)) return null
            return value
        }
    }
}
