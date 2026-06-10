// SCE-MAP: codec_zenoh_decl_kexpr:47

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_decl_kexpr

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_wireexpr.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclKexpr()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohDeclKexpr(
    var id: UShort = 0.toUShort(),
    var wireexpr: CodecZenohWireexpr = CodecZenohWireexpr()
) {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, N: UByte): CodecError? {
        // RFC §synth-5-B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable.
        run {
            var _vle: ULong = (id).toULong()
            while (_vle >= 0x80UL) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        this.wireexpr.encode(w, N)?.let { return it }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(N: UByte): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list), N)
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §synth-5-B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, N: UByte): CodecZenohDeclKexpr? {
            // Streaming codec: each field reads from cursor directly
            // (VLE base-128 chain). RFC §synth-5-B B4: per-field bit-size
            // dispatch routes Fixed / LengthRef siblings of VLE fields
            // through `present_if_decode_stmt` (predicate=None arms).
            // Pure-VLE codecs stay byte-stable.
            val id = cursor.readVleU16() ?: return null
            val wireexpr = CodecZenohWireexpr.decode(cursor, N) ?: return null
            return CodecZenohDeclKexpr(
                id = id,
                wireexpr = wireexpr
            )
        }
    }
}
