// SCE-MAP: codec_repeat_unconditional_count:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_repeat_unconditional_count

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_repeat_elem.*
import kotlin.jvm.JvmInline

/// The `links` element of [CodecRepeatUnconditionalCount], 0 to 64 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecRepeatUnconditionalCountLinksList private constructor(val entries: List<CodecRepeatElemSource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 64

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecRepeatElemSource>): CodecRepeatUnconditionalCountLinksList? =
            if (entries.size <= MOST) CodecRepeatUnconditionalCountLinksList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): CodecRepeatUnconditionalCountLinksList = CodecRepeatUnconditionalCountLinksList(emptyList())
    }
}

/// The `weights` element of [CodecRepeatUnconditionalCount], 0 to 64 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecRepeatUnconditionalCountWeightsList private constructor(val entries: List<CodecRepeatElemSource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 64

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecRepeatElemSource>): CodecRepeatUnconditionalCountWeightsList? =
            if (entries.size <= MOST) CodecRepeatUnconditionalCountWeightsList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): CodecRepeatUnconditionalCountWeightsList = CodecRepeatUnconditionalCountWeightsList(emptyList())
    }
}

/// The `options_h` group of [CodecRepeatUnconditionalCount]: the arm the value holds, each the
/// members present while the gates hold its values (docs/adr/0065 §6 R4).
sealed class CodecRepeatUnconditionalCountOptionsH {
    /// The `options_h` group of [CodecRepeatUnconditionalCount] where `options.H` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class H(
        val links_weights: com.sce.forge.runtime.Aligned<CodecRepeatUnconditionalCountLinksList, CodecRepeatUnconditionalCountWeightsList> = com.sce.forge.runtime.Aligned.of(CodecRepeatUnconditionalCountLinksList.fewest(), CodecRepeatUnconditionalCountWeightsList.fewest())!!
    ) : CodecRepeatUnconditionalCountOptionsH()
    /// The `options_h` group of [CodecRepeatUnconditionalCount] where `options.H` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class NotH(
        val links: CodecRepeatUnconditionalCountLinksList = CodecRepeatUnconditionalCountLinksList.fewest()
    ) : CodecRepeatUnconditionalCountOptionsH()
}

/// The elements a value of [CodecRepeatUnconditionalCount] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecRepeatUnconditionalCount.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecRepeatUnconditionalCountSource {
    val options: UByte
    val options_h: CodecRepeatUnconditionalCountOptionsH
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecRepeatUnconditionalCount()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecRepeatUnconditionalCount(
    override var options: UByte = 0.toUByte(),
    override var options_h: CodecRepeatUnconditionalCountOptionsH = CodecRepeatUnconditionalCountOptionsH.NotH()
) : CodecRepeatUnconditionalCountSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun H(): Boolean = (this.options.toInt() and 0x01) != 0

    fun setH(v: Boolean) {
        this.options = if (v) {
            (this.options.toInt() or 0x01).toUByte()
        } else {
            (this.options.toInt() and 0x01.inv()).toUByte()
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
        /// Write any source of [CodecRepeatUnconditionalCount] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `links_len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecRepeatUnconditionalCountSource, w: SceSink): CodecError? {
            val _e_options = src.options
            val _g_options_h = src.options_h
            val _e_links = when (val _g = _g_options_h) { is CodecRepeatUnconditionalCountOptionsH.H -> _g.links_weights.first; is CodecRepeatUnconditionalCountOptionsH.NotH -> _g.links }
            val _e_weights = when (val _g = _g_options_h) { is CodecRepeatUnconditionalCountOptionsH.H -> _g.links_weights.second; else -> null }
            val _d_links_len = _e_links.entries.size.toUByte()
            var _u_options = _e_options.toUInt()
            _u_options = (_u_options and 0x1u.inv()) or (if (_e_weights != null) 0x1u else 0x0u)
            val _c_options = _u_options.toUByte()
            w.writeU8(_c_options.toByte())?.let { return it }
            w.writeU8(_d_links_len.toByte())?.let { return it }
            for (_entry in _e_links.entries) {
                CodecRepeatElem.encodeSource(_entry, w)?.let { return it }
            }
            if (_e_weights != null) {
                for (_entry in _e_weights.entries) {
                    CodecRepeatElem.encodeSource(_entry, w)?.let { return it }
                }
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
        fun decode(cursor: SceCursor): Decoded<CodecRepeatUnconditionalCount> {
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
            val options = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val links_len = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            if (links_len > 64u) {
                return Decoded.Refused(CodecError.TooManyElements)
            }
            val links: MutableList<CodecRepeatElem> = mutableListOf<CodecRepeatElem>().apply {
                repeat(links_len.toInt()) {
                    add(CodecRepeatElem.decode(cursor).valueOr { _refused -> return _refused })
                }
            }
            val weights: MutableList<CodecRepeatElem>? = if ((options.toInt() and 0x01) != 0) {
                val _n = links_len
                if (_n > 64u) {
                    return Decoded.Refused(CodecError.TooManyElements)
                }
                mutableListOf<CodecRepeatElem>().apply {
                    repeat(_n.toInt()) {
                        add(CodecRepeatElem.decode(cursor).valueOr { _refused -> return _refused })
                    }
                }
            } else null
            return Decoded.Ok(
                CodecRepeatUnconditionalCount(
                    options = options,
                    options_h = if (((options.toUInt() shr 0) and 0x1u) != 0x0u) CodecRepeatUnconditionalCountOptionsH.H(links_weights = com.sce.forge.runtime.Aligned.of(CodecRepeatUnconditionalCountLinksList.of(links) ?: return Decoded.Refused(CodecError.TooManyElements), CodecRepeatUnconditionalCountWeightsList.of(weights!!) ?: return Decoded.Refused(CodecError.TooManyElements)) ?: error("two lists read under one count hold as many entries")) else CodecRepeatUnconditionalCountOptionsH.NotH(links = CodecRepeatUnconditionalCountLinksList.of(links) ?: return Decoded.Refused(CodecError.TooManyElements))
                )
            )
        }
    }
}
