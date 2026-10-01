// SCE-MAP: codec_origin_envelope:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_origin_envelope

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_origin_leaf.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecOriginEnvelope()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecOriginEnvelope(
    var hdr: UByte = 0.toUByte(),
    var note_len: UByte = 0.toUByte(),
    var note: String = "",
    var m: UByte = 0.toUByte(),
    var required: CodecOriginLeaf = CodecOriginLeaf(),
    var optional: CodecOriginLeaf? = null,
    var items: MutableList<CodecOriginLeaf> = mutableListOf()
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun hasOpt(): Boolean = (this.hdr.toInt() and 0x01) != 0

    fun setHasOpt(v: Boolean) {
        this.hdr = if (v) {
            (this.hdr.toInt() or 0x01).toUByte()
        } else {
            (this.hdr.toInt() and 0x01.inv()).toUByte()
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
        w.writeU8(this.hdr.toByte())?.let { return it }
        w.writeU8(this.note_len.toByte())?.let { return it }
        w.writeBytes(this.note.toByteArray(Charsets.UTF_8))?.let { return it }
        w.writeU8(this.m.toByte())?.let { return it }
        this.required.encode(w)?.let { return it }
        this.optional?.let { _v ->
            _v.encode(w)?.let { return it }
        }
        for (_e in this.items) {
            _e.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecOriginEnvelope? {
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
            val hdr = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val note_len = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val note = run {
                val _n = note_len.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = try {
                    java.nio.charset.StandardCharsets.UTF_8.newDecoder()
                        .decode(java.nio.ByteBuffer.wrap(raw)).toString()
                } catch (_: java.nio.charset.CharacterCodingException) { return null }
                if (!cursor.advance(_n)) return null
                _v
            }
            val m = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val required = CodecOriginLeaf.decode(cursor) ?: return null
            val optional: CodecOriginLeaf? = if ((hdr.toInt() and 0x01) != 0) {
                CodecOriginLeaf.decode(cursor) ?: return null
            } else {
                null
            }
            val items: MutableList<CodecOriginLeaf> = mutableListOf<CodecOriginLeaf>().apply {
                repeat(m.toInt()) {
                    add(CodecOriginLeaf.decode(cursor) ?: return null)
                }
            }
            return CodecOriginEnvelope(
                hdr = hdr,
                note_len = note_len,
                note = note,
                m = m,
                required = required,
                optional = optional,
                items = items
            )
        }
    }
}
