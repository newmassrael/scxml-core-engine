// SCE-MAP: codec_ext_attachment:27

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_ext_attachment

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecExtAttachment()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecExtAttachment(
    var length: UByte = 0.toUByte(),
    var body: ByteArray = byteArrayOf()
) {
    /// RFC §5.B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8(length.toByte())?.let { return it }
        w.writeBytes(body)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecExtAttachment? {
            val frameLen = cursor.remaining()
            if (frameLen < 1) return null
            val raw = cursor.peekSlice(frameLen) ?: return null
            val length = raw[0].toUByte()
            val body = raw.copyOfRange(1, 1 + length.toInt())
            val value = CodecExtAttachment(
                length = length,
                body = body
            )
            if (!cursor.advance(frameLen)) return null
            return value
        }
    }
}
