// SCE-MAP: codec_zenoh_undecl_subscriber:46

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_undecl_subscriber

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_decl_ext_keyexpr.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohUndeclSubscriber()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohUndeclSubscriber(
    var id: UInt = 0u,
    var ext_keyexpr: CodecZenohDeclExtKeyexpr? = null
) {
    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, Z: UByte): CodecError? {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        run {
            var _vle: ULong = (id).toULong()
            while (_vle >= 0x80UL) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        this.ext_keyexpr?.let { _v ->
            _v.encode(w)?.let { return it }
        }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(Z: UByte): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list), Z)
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, Z: UByte): CodecZenohUndeclSubscriber? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val id = cursor.readVleU32() ?: return null
            val ext_keyexpr: CodecZenohDeclExtKeyexpr? = if ((Z.toInt() and 0x01) != 0) {
                CodecZenohDeclExtKeyexpr.decode(cursor) ?: return null
            } else {
                null
            }
            return CodecZenohUndeclSubscriber(
                id = id,
                ext_keyexpr = ext_keyexpr
            )
        }
    }
}
