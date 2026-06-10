// SCE-MAP: codec_nested_parent:22

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_nested_parent

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_nested_body.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecNestedParent()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecNestedParent(
    var hdr: UByte = 0.toUByte(),
    var m: UByte = 0.toUByte(),
    var required_body: CodecNestedBody = CodecNestedBody(),
    var optional_body: CodecNestedBody? = null,
    var body_list: MutableList<CodecNestedBody> = mutableListOf()
) {
    // RFC §5.B flags primitive: per-bit-range accessors over
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

    /// RFC §5.B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // RFC §5.B present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        w.writeU8(this.hdr.toByte())?.let { return it }
        w.writeU8(this.m.toByte())?.let { return it }
        this.required_body.encode(w)?.let { return it }
        this.optional_body?.let { _v ->
            _v.encode(w)?.let { return it }
        }
        for (_e in this.body_list) {
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
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecNestedParent? {
            // RFC §5.B present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // Gating extends to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val hdr = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val m = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val required_body = CodecNestedBody.decode(cursor) ?: return null
            val optional_body: CodecNestedBody? = if ((hdr.toInt() and 0x01) != 0) {
                CodecNestedBody.decode(cursor) ?: return null
            } else {
                null
            }
            val body_list: MutableList<CodecNestedBody> = mutableListOf<CodecNestedBody>().apply {
                repeat(m.toInt()) {
                    add(CodecNestedBody.decode(cursor) ?: return null)
                }
            }
            return CodecNestedParent(
                hdr = hdr,
                m = m,
                required_body = required_body,
                optional_body = optional_body,
                body_list = body_list
            )
        }
    }
}
