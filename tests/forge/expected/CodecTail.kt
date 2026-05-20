// SCE-MAP: codec_tail:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_tail

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecTail()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecTail(
    var msgId: UByte = 0.toUByte(),
    var status: UByte = 0.toUByte(),
    var payload: ByteArray = byteArrayOf()
) {
    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8(msgId.toByte())?.let { return it }
        w.writeU8(status.toByte())?.let { return it }
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
        fun decode(cursor: SceCursor): CodecTail? {
            val frameLen = cursor.remaining()
            if (frameLen < 2) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val msgId = raw[0].toUByte()
            val status = raw[1].toUByte()
            val payload = raw.copyOfRange(2, raw.size)
            val value = CodecTail(
                msgId = msgId,
                status = status,
                payload = payload
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
