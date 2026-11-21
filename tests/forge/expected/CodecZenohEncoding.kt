// SCE-MAP: codec_zenoh_encoding:69 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_encoding

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `schema` element of [CodecZenohEncoding], 0 to 255 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohEncodingSchemaText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 255

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): CodecZenohEncodingSchemaText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) CodecZenohEncodingSchemaText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohEncodingSchemaText = CodecZenohEncodingSchemaText("\u0000".repeat(LEAST))
    }
}

/// The elements a value of [CodecZenohEncoding] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohEncoding.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohEncodingSource {
    val packed_id: UInt
    val schema: CodecZenohEncodingSchemaText?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohEncoding()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohEncoding(
    override var packed_id: UInt = 0u,
    override var schema: CodecZenohEncodingSchemaText? = null
) : CodecZenohEncodingSource {
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
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        return encodeSource(this, w)
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
        /// Write any source of [CodecZenohEncoding] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `schema_len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohEncodingSource, w: SceSink): CodecError? {
            val _e_packed_id = src.packed_id
            val _e_schema = src.schema
            val _d_schema_len = if (_e_schema != null) _e_schema.text.encodeToByteArray().size.toULong() else 0.toULong()
            var _u_packed_id = _e_packed_id
            _u_packed_id = (_u_packed_id and 0x1u.inv()) or (if (_e_schema != null) 0x1u else 0x0u)
            val _c_packed_id = _u_packed_id
            w.writeVleU32(_c_packed_id)?.let { return it }
            if (((_c_packed_id shr 0) and 0x1u) != 0x0u) {
                w.writeVleU64(_d_schema_len)?.let { return it }
            }
            if (_e_schema != null) {
                w.writeBytes(_e_schema.text.encodeToByteArray())?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecZenohEncoding> {
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
            val packed_id = cursor.readVleU32().valueOr { _refused -> return _refused }
            val schema_len: ULong? = if ((packed_id.toLong() and 0x00000001L) != 0L) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val schema = if ((packed_id.toLong() and 0x00000001L) != 0L) {
                val _len = schema_len!!
                if (_len > 255uL) {
                    return Decoded.Refused(CodecError.LengthOutOfRange)
                }
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = try {
                    java.nio.charset.StandardCharsets.UTF_8.newDecoder()
                        .decode(java.nio.ByteBuffer.wrap(raw)).toString()
                } catch (_: java.nio.charset.CharacterCodingException) { return Decoded.Refused(CodecError.InvalidUtf8) }
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecZenohEncoding(
                    packed_id = packed_id,
                    schema = schema?.let { CodecZenohEncodingSchemaText.of(it) ?: return Decoded.Refused(CodecError.LengthOutOfRange) }
                )
            )
        }
    }
}
