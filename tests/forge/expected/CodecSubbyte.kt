// SCE-MAP: codec_subbyte:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_subbyte

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecSubbyte()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecSubbyte(
    var priority: UByte = 0.toUByte(),
    var channel: UByte = 0.toUByte(),
    var direction: UByte = 0.toUByte()
) {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8(((priority.toInt() and 0x07 shl 5) or (channel.toInt() and 0x07 shl 2) or (direction.toInt() and 0x03 shl 0)).toByte())?.let { return it }
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
        fun decode(cursor: SceCursor): CodecSubbyte? {
            val raw = cursor.peekSlice(1) ?: return null
            val priority = ((raw[0].toInt() ushr 5) and 0x07).toUByte()
            val channel = ((raw[0].toInt() ushr 2) and 0x07).toUByte()
            val direction = ((raw[0].toInt() ushr 0) and 0x03).toUByte()
            if (!cursor.advance(1)) return null
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return CodecSubbyte(
                priority = priority,
                channel = channel,
                direction = direction
            )
        }
    }
}
