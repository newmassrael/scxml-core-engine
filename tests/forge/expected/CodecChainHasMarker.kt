// SCE-MAP: codec_chain_has_marker:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_has_marker

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_entry.*
import com.sce.generated.codec_chain_has_marker_slice.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainHasMarker()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainHasMarker(
    var header: UByte = 0.toUByte(),
    var extensions: MutableList<CodecZenohExtEntry>? = null,
    var payload_len: ULong? = null,
    var payload: ByteArray? = null,
    var slice_count: UInt? = null,
    var slices: MutableList<CodecChainHasMarkerSlice>? = null
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun kind(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x1F).toUByte()
    }

    fun setKind(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x1F shl 0
        val _val = (v.toInt() and 0x1F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun E(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setE(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
        }
    }

    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        val _has_extensions_18: Boolean = this.extensions?.any { _e -> ((_e.header).toULong() and 127UL) == 18UL } ?: false
        if (!_has_extensions_18) {
            if (this.payload_len == null) return CodecError.PresentIfMismatch
        } else if (this.payload_len != null) return CodecError.PresentIfMismatch
        if (!_has_extensions_18) {
            if (this.payload == null) return CodecError.PresentIfMismatch
        } else if (this.payload != null) return CodecError.PresentIfMismatch
        if (_has_extensions_18) {
            if (this.slice_count == null) return CodecError.PresentIfMismatch
        } else if (this.slice_count != null) return CodecError.PresentIfMismatch
        if (_has_extensions_18) {
            if (this.slices == null) return CodecError.PresentIfMismatch
        } else if (this.slices != null) return CodecError.PresentIfMismatch
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when null, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        w.writeU8(this.header.toByte())?.let { return it }
        this.extensions?.let { _list ->
            for (_e in _list) {
                _e.encode(w)?.let { return it }
            }
        }
        this.payload_len?.let { _v ->
        w.writeVleU64(_v)?.let { return it }
        }
        this.payload?.let { _v ->
            w.writeBytes(_v)?.let { return it }
        }
        this.slice_count?.let { _v ->
        w.writeVleU32(_v)?.let { return it }
        }
        this.slices?.let { _list ->
            for (_e in _list) {
                _e.encode(w)?.let { return it }
            }
        }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray, or
    /// `null` when `encode` refuses a message whose chain and a gated
    /// field contradict each other (`CodecError.PresentIfMismatch`).
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(): ByteArray? {
        val _list = mutableListOf<Byte>()
        if (encode(MutableListSink(_list)) != null) return null
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §synth-5-B L494-519).
        fun decode(cursor: SceCursor): CodecChainHasMarker? {
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
            val header = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val extensions: MutableList<CodecZenohExtEntry>? = if ((header.toInt() and 0x80) != 0) {
            mutableListOf<CodecZenohExtEntry>().also {
                var _more = false
                for (_i in 0 until 4) {
                    if (cursor.remaining() == 0) break
                    val _entry = CodecZenohExtEntry.decode(cursor) ?: return null
                    _more = _entry.Z()
                    it.add(_entry)
                    if (!_more) break
                }
                if (_more) return null
            }
        } else {
            null
        }
            val _has_extensions_18: Boolean = extensions?.any { _e -> ((_e.header).toULong() and 127UL) == 18UL } ?: false
            val payload_len: ULong? = if (!_has_extensions_18) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val payload = if (!_has_extensions_18) {
                val _n = payload_len!!.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
            val slice_count: UInt? = if (_has_extensions_18) {
                val _v = cursor.readVleU32() ?: return null
                _v
            } else {
                null
            }
            val slices: MutableList<CodecChainHasMarkerSlice>? = if (_has_extensions_18) {
                val _n = slice_count!!
                mutableListOf<CodecChainHasMarkerSlice>().apply {
                    repeat(_n.toInt()) {
                        add(CodecChainHasMarkerSlice.decode(cursor) ?: return null)
                    }
                }
            } else null
            return CodecChainHasMarker(
                header = header,
                extensions = extensions,
                payload_len = payload_len,
                payload = payload,
                slice_count = slice_count,
                slices = slices
            )
        }
    }
}
