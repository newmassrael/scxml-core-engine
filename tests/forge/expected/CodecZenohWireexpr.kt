// SCE-MAP: codec_zenoh_wireexpr:55 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_wireexpr

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `suffix` element of [CodecZenohWireexpr], 0 to 65535 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohWireexprSuffixText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 65535

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): CodecZenohWireexprSuffixText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) CodecZenohWireexprSuffixText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohWireexprSuffixText = CodecZenohWireexprSuffixText("\u0000".repeat(LEAST))
    }
}

/// The elements a value of [CodecZenohWireexpr] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohWireexpr.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohWireexprSource {
    val id: ULong
    val suffix: CodecZenohWireexprSuffixText?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohWireexpr()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohWireexpr(
    override var id: ULong = 0uL,
    override var suffix: CodecZenohWireexprSuffixText? = null
) : CodecZenohWireexprSource {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, N: UByte): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        if (N != inputN(this)) return CodecError.PresentIfMismatch
        return encodeSource(this, w)
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray, or
    /// `null` when `encode` refuses the value before its first byte: a
    /// gated field and the `sce:present-if` that gates it contradict each
    /// other (`CodecError.PresentIfMismatch`), or a byte string, text or
    /// list lies outside its bound (`CodecError.LengthOutOfRange`,
    /// `CodecError.TooManyElements`). Callers that need the reason, and
    /// zero-alloc hot paths, call `encode` directly against a caller-owned
    /// sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(N: UByte): ByteArray? {
        val _list = mutableListOf<Byte>()
        if (encode(MutableListSink(_list), N) != null) return null
        return _list.toByteArray()
    }

    companion object {
        /// Write any source of [CodecZenohWireexpr] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `suffix_len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohWireexprSource, w: SceSink): CodecError? {
            val _e_id = src.id
            val _e_suffix = src.suffix
            val N = (if (_e_suffix != null) 1 else 0).toUByte()
            val _d_suffix_len = if (_e_suffix != null) _e_suffix.text.encodeToByteArray().size.toULong() else 0.toULong()
            w.writeVleU64(_e_id)?.let { return it }
            if (N.toUInt() != 0u) {
                w.writeVleU64(_d_suffix_len)?.let { return it }
            }
            if (_e_suffix != null) {
                w.writeBytes(_e_suffix.text.encodeToByteArray())?.let { return it }
            }
            return null
        }

        /// The flag input `N` a source of [CodecZenohWireexpr] says, which its encode
        /// is not handed (docs/adr/0065 §6 R4a).
        fun inputN(src: CodecZenohWireexprSource): UByte =
            (if (src.suffix != null) 1 else 0).toUByte()

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, N: UByte): Decoded<CodecZenohWireexpr> {
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
            val id = cursor.readVleU64().valueOr { _refused -> return _refused }
            val suffix_len: ULong? = if ((N.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val suffix = if ((N.toInt() and 0x01) != 0) {
                val _len = suffix_len!!
                if (_len > 65535uL) {
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
                CodecZenohWireexpr(
                    id = id,
                    suffix = suffix?.let { CodecZenohWireexprSuffixText.of(it) ?: return Decoded.Refused(CodecError.LengthOutOfRange) }
                )
            )
        }
    }
}
