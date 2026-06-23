// SCE-MAP: codec_init_cookie_body:36

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_init_cookie_body

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecInitCookieBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecInitCookieBody(
    var version: UByte = 0.toUByte(),
    var cookie_size: UShort? = null,
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
        w.writeU8(this.version.toByte())?.let { return it }
        this.cookie_size?.let { _v ->
        w.writeVleU16((_v).toUShort())?.let { return it }
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
        fun decode(cursor: SceCursor, A: UByte): CodecInitCookieBody? {
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
            val version = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val cookie_size: UShort? = if ((A.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU16() ?: return null
                _v
            } else {
                null
            }
            val cookie = if ((A.toInt() and 0x01) != 0) {
                val _n = cookie_size!!.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            return CodecInitCookieBody(
                version = version,
                cookie_size = cookie_size,
                cookie = cookie
            )
        }
    }
}
