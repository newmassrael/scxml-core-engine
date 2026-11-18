// SCE-MAP: codec_present_if_string:47 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_present_if_string

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `text` element of [CodecPresentIfString], 0 to 32 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecPresentIfStringTextText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 32

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): CodecPresentIfStringTextText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) CodecPresentIfStringTextText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecPresentIfStringTextText = CodecPresentIfStringTextText("\u0000".repeat(LEAST))
    }
}

/// The elements a value of [CodecPresentIfString] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecPresentIfString.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecPresentIfStringSource {
    val carrier: UByte
    val text: CodecPresentIfStringTextText?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecPresentIfString()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecPresentIfString(
    override var carrier: UByte = 0.toUByte(),
    override var text: CodecPresentIfStringTextText? = null
) : CodecPresentIfStringSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun hasText(): Boolean = (this.carrier.toInt() and 0x01) != 0

    fun setHasText(v: Boolean) {
        this.carrier = if (v) {
            (this.carrier.toInt() or 0x01).toUByte()
        } else {
            (this.carrier.toInt() and 0x01.inv()).toUByte()
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
        /// Write any source of [CodecPresentIfString] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `text_len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecPresentIfStringSource, w: SceSink): CodecError? {
            val _e_carrier = src.carrier
            val _e_text = src.text
            val _d_text_len = if (_e_text != null) _e_text.text.encodeToByteArray().size.toUByte() else 0.toUByte()
            var _u_carrier = _e_carrier.toUInt()
            _u_carrier = (_u_carrier and 0x1u.inv()) or (if (_e_text != null) 0x1u else 0x0u)
            val _c_carrier = _u_carrier.toUByte()
            w.writeU8(_c_carrier.toByte())?.let { return it }
            if (((_c_carrier.toUInt() shr 0) and 0x1u) != 0x0u) {
                w.writeU8(_d_text_len.toByte())?.let { return it }
            }
            if (_e_text != null) {
                w.writeBytes(_e_text.text.encodeToByteArray())?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecPresentIfString> {
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
            val carrier = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val text_len = if ((carrier.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val text = if ((carrier.toInt() and 0x01) != 0) {
                val _len = text_len!!
                if (_len > 32u) {
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
                CodecPresentIfString(
                    carrier = carrier,
                    text = text?.let { CodecPresentIfStringTextText.of(it) ?: return Decoded.Refused(CodecError.LengthOutOfRange) }
                )
            )
        }
    }
}
