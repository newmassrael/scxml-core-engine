// SCE-MAP: codec_zenoh_open_body:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_open_body

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohOpenBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohOpenBody(
    var lease: ULong = 0uL,
    var initial_sn: ULong = 0uL,
    var cookie_len: ULong? = null,
    var cookie: ByteArray? = null
) {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, A: UByte): CodecError? {
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when null, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        run {
            var _vle: ULong = (lease).toULong()
            var _vn = 0
            while (_vle >= 0x80UL && _vn < 8) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
                _vn++
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        run {
            var _vle: ULong = (initial_sn).toULong()
            var _vn = 0
            while (_vle >= 0x80UL && _vn < 8) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
                _vn++
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        this.cookie_len?.let { _v ->
        run {
            var _vle: ULong = (_v).toULong()
            var _vn = 0
            while (_vle >= 0x80UL && _vn < 8) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
                _vn++
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        }
        this.cookie?.let { _v ->
            w.writeBytes(_v)?.let { return it }
        }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(A: UByte): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list), A)
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §synth-5-B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, A: UByte): CodecZenohOpenBody? {
            // Streaming cursor decode (SSOT selection: `needs_streaming`).
            // The positional `raw[byte_off]` path is valid only when every
            // field's absolute offset is fixed at codegen time; this branch
            // handles every codec where it is not — present-if-gated fields
            // (runtime presence), VLE / repeat / TLV-chain / embed fields
            // (runtime width), string fields (UTF-8 decode), and a fixed
            // field after a variable-length payload (offset depends on the
            // payload length). Each field reads its own bytes from the
            // cursor and advances past what it consumed. Per-field
            // `is_repeat` / `is_tlv_chain` / `is_embed` route to their
            // dedicated helpers; every other field flows through
            // `present_if_decode_stmt`.
            val lease = cursor.readVleU64() ?: return null
            val initial_sn = cursor.readVleU64() ?: return null
            val cookie_len: ULong? = if ((A.toInt() and 0x01) == 0) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val cookie = if ((A.toInt() and 0x01) == 0) {
                val _n = cookie_len!!.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            return CodecZenohOpenBody(
                lease = lease,
                initial_sn = initial_sn,
                cookie_len = cookie_len,
                cookie = cookie
            )
        }
    }
}
