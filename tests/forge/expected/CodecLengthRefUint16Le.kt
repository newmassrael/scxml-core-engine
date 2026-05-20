// SCE-MAP: codec_length_ref_uint16_le:12

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref_uint16_le

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRefUint16Le()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLengthRefUint16Le(
    var payload_len: UShort = 0.toUShort(),
    var payload: ByteArray = byteArrayOf()
) {
    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8((payload_len.toInt() and 0xFF).toByte())?.let { return it }
        w.writeU8((payload_len.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
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
        fun decode(cursor: SceCursor): CodecLengthRefUint16Le? {
            val frameLen = cursor.remaining()
            if (frameLen < 2) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val payload_len = ((raw[0].toInt() and 0xFF) or ((raw[1].toInt() and 0xFF) shl 8)).toUShort()
            val payload = raw.copyOfRange(2, 2 + payload_len.toInt())
            val value = CodecLengthRefUint16Le(
                payload_len = payload_len,
                payload = payload
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
