// SCE-MAP: codec_zenoh_encoding:68

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_encoding

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohEncoding()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohEncoding(
    var packed_id: UInt = 0u,
    var schema_len: ULong? = null,
    var schema: String? = null
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun hasSchema(): Boolean = (this.packed_id.toLong() and 0x00000001) != 0L

    fun setHasSchema(v: Boolean) {
        this.packed_id = if (v) {
            (this.packed_id.toLong() or 0x00000001).toUInt()
        } else {
            (this.packed_id.toLong() and 0x00000001.inv()).toUInt()
        }
    }

    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when null, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        w.writeVleU32((packed_id).toUInt())?.let { return it }
        this.schema_len?.let { _v ->
        w.writeVleU64((_v).toULong())?.let { return it }
        }
        this.schema?.let { _v ->
            w.writeBytes(_v.toByteArray(Charsets.UTF_8))?.let { return it }
        }
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
        fun decode(cursor: SceCursor): CodecZenohEncoding? {
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
            val packed_id = cursor.readVleU32() ?: return null
            val schema_len: ULong? = if ((packed_id.toLong() and 0x00000001L) != 0L) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val schema = if ((packed_id.toLong() and 0x00000001L) != 0L) {
                val _n = schema_len!!.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = try {
                    java.nio.charset.StandardCharsets.UTF_8.newDecoder()
                        .decode(java.nio.ByteBuffer.wrap(raw)).toString()
                } catch (_: java.nio.charset.CharacterCodingException) { return null }
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            return CodecZenohEncoding(
                packed_id = packed_id,
                schema_len = schema_len,
                schema = schema
            )
        }
    }
}
