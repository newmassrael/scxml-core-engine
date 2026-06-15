// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_decl_ext_keyexpr

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_decl_ext_keyexpr_inner.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclExtKeyexpr()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohDeclExtKeyexpr(
    var outer_header: UByte = 0.toUByte(),
    var total_length: ULong = 0uL,
    var inner: CodecZenohDeclExtKeyexprInner = CodecZenohDeclExtKeyexprInner()
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun extId(): UByte {
        val _carrier = this.outer_header.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setExtId(v: UByte) {
        val _carrier = this.outer_header.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.outer_header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun M(): Boolean = (this.outer_header.toInt() and 0x10) != 0

    fun setM(v: Boolean) {
        this.outer_header = if (v) {
            (this.outer_header.toInt() or 0x10).toUByte()
        } else {
            (this.outer_header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun enc(): UByte {
        val _carrier = this.outer_header.toInt()
        return ((_carrier shr 5) and 0x03).toUByte()
    }

    fun setEnc(v: UByte) {
        val _carrier = this.outer_header.toInt()
        val _shifted_mask = 0x03 shl 5
        val _val = (v.toInt() and 0x03) shl 5
        this.outer_header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun Z(): Boolean = (this.outer_header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
        this.outer_header = if (v) {
            (this.outer_header.toInt() or 0x80).toUByte()
        } else {
            (this.outer_header.toInt() and 0x80.inv()).toUByte()
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
        w.writeU8(this.outer_header.toByte())?.let { return it }
        run {
            var _vle: ULong = (total_length).toULong()
            while (_vle >= 0x80UL) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        this.inner.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecZenohDeclExtKeyexpr? {
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
            val outer_header = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val total_length = cursor.readVleU64() ?: return null
            val inner = run {
                val _len = (total_length).toInt()
                val _raw = cursor.peekSlice(_len) ?: return null
                val _inner = SceCursor(_raw)
                val _v = CodecZenohDeclExtKeyexprInner.decode(_inner) ?: return null
                if (!cursor.advance(_len)) return null
                _v
            }
            return CodecZenohDeclExtKeyexpr(
                outer_header = outer_header,
                total_length = total_length,
                inner = inner
            )
        }
    }
}
