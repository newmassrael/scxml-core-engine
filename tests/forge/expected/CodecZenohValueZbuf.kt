// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_value_zbuf

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_value_slice.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohValueZbuf()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohValueZbuf(
    var value_len: ULong = 0uL,
    var value: ByteArray? = null,
    var encoding: ULong? = null,
    var slice_count: ULong? = null,
    var slices: MutableList<CodecZenohValueSlice>? = null
) {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, afterShm: UByte): CodecError? {
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when null, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        w.writeVleU64(value_len)?.let { return it }
        this.value?.let { _v ->
            w.writeBytes(_v)?.let { return it }
        }
        this.encoding?.let { _v ->
        w.writeVleU64(_v)?.let { return it }
        }
        this.slice_count?.let { _v ->
        w.writeVleU64(_v)?.let { return it }
        }
        this.slices?.let { _list ->
            for (_e in _list) {
                _e.encode(w)?.let { return it }
            }
        }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(afterShm: UByte): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list), afterShm)
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §synth-5-B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, afterShm: UByte): CodecZenohValueZbuf? {
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
            val value_len = cursor.readVleU64() ?: return null
            val value = if ((afterShm.toInt() and 0x01) == 0) {
                val _n = value_len.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            val encoding: ULong? = if ((afterShm.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val slice_count: ULong? = if ((afterShm.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val slices: MutableList<CodecZenohValueSlice>? = if ((afterShm.toInt() and 0x01) != 0) {
                val _n = slice_count!!
                mutableListOf<CodecZenohValueSlice>().apply {
                    repeat(_n.toInt()) {
                        add(CodecZenohValueSlice.decode(cursor) ?: return null)
                    }
                }
            } else null
            return CodecZenohValueZbuf(
                value_len = value_len,
                value = value,
                encoding = encoding,
                slice_count = slice_count,
                slices = slices
            )
        }
    }
}
