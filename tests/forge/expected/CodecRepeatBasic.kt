// SCE-MAP: codec_repeat_basic:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_repeat_basic

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_repeat_elem.*
import kotlin.jvm.JvmInline

/// The `frags` element of [CodecRepeatBasic], 0 to 32 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecRepeatBasicFragsList private constructor(val entries: List<CodecRepeatElemSource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 32

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecRepeatElemSource>): CodecRepeatBasicFragsList? =
            if (entries.size <= MOST) CodecRepeatBasicFragsList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): CodecRepeatBasicFragsList = CodecRepeatBasicFragsList(emptyList())
    }
}

/// The elements a value of [CodecRepeatBasic] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecRepeatBasic.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecRepeatBasicSource {
    val frags: CodecRepeatBasicFragsList
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecRepeatBasic()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecRepeatBasic(
    override var frags: CodecRepeatBasicFragsList = CodecRepeatBasicFragsList.fewest()
) : CodecRepeatBasicSource {
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
        /// Write any source of [CodecRepeatBasic] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `num_frags` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecRepeatBasicSource, w: SceSink): CodecError? {
            val _e_frags = src.frags
            val _d_num_frags = _e_frags.entries.size.toUByte()
            w.writeU8(_d_num_frags.toByte())?.let { return it }
            for (_entry in _e_frags.entries) {
                CodecRepeatElem.encodeSource(_entry, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecRepeatBasic> {
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
            val num_frags = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            if (num_frags > 32u) {
                return Decoded.Refused(CodecError.TooManyElements)
            }
            val frags: MutableList<CodecRepeatElem> = mutableListOf<CodecRepeatElem>().apply {
                repeat(num_frags.toInt()) {
                    add(CodecRepeatElem.decode(cursor).valueOr { _refused -> return _refused })
                }
            }
            return Decoded.Ok(
                CodecRepeatBasic(
                    frags = CodecRepeatBasicFragsList.of(frags) ?: return Decoded.Refused(CodecError.TooManyElements)
                )
            )
        }
    }
}
