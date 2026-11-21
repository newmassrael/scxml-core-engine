// SCE-MAP: codec_chain_has_slice:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_has_slice

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `bytes` element of [CodecChainHasSlice], 0 to 256 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecChainHasSliceBytesBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Int = 256

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecChainHasSliceBytesBytes? =
            if (bytes.size <= MOST) CodecChainHasSliceBytesBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecChainHasSliceBytesBytes = CodecChainHasSliceBytesBytes(ByteArray(LEAST))
    }
}

/// The elements a value of [CodecChainHasSlice] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecChainHasSlice.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecChainHasSliceSource {
    val kind: UShort
    val bytes: CodecChainHasSliceBytesBytes
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainHasSlice()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainHasSlice(
    override var kind: UShort = 0.toUShort(),
    override var bytes: CodecChainHasSliceBytesBytes = CodecChainHasSliceBytesBytes.fewest()
) : CodecChainHasSliceSource {
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
        /// Write any source of [CodecChainHasSlice] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecChainHasSliceSource, w: SceSink): CodecError? {
            val _e_kind = src.kind
            val _e_bytes = src.bytes
            val _d_len = _e_bytes.bytes.size.toULong()
            w.writeVleU16(_e_kind)?.let { return it }
            w.writeVleU64(_d_len)?.let { return it }
            w.writeBytes(_e_bytes.bytes)?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecChainHasSlice> {
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
            val kind = cursor.readVleU16().valueOr { _refused -> return _refused }
            val len = cursor.readVleU64().valueOr { _refused -> return _refused }
            val bytes = run {
                val _len = len
                if (_len > 256uL) {
                    return Decoded.Refused(CodecError.LengthOutOfRange)
                }
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            return Decoded.Ok(
                CodecChainHasSlice(
                    kind = kind,
                    bytes = CodecChainHasSliceBytesBytes.of(bytes) ?: return Decoded.Refused(CodecError.LengthOutOfRange)
                )
            )
        }
    }
}
