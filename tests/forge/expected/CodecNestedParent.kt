// SCE-MAP: codec_nested_parent:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_nested_parent

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_nested_body.*
import kotlin.jvm.JvmInline

/// The `body_list` element of [CodecNestedParent], 0 to 4 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecNestedParentBodyListList private constructor(val entries: List<CodecNestedBodySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 4

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecNestedBodySource>): CodecNestedParentBodyListList? =
            if (entries.size <= MOST) CodecNestedParentBodyListList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): CodecNestedParentBodyListList = CodecNestedParentBodyListList(emptyList())
    }
}

/// The elements a value of [CodecNestedParent] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecNestedParent.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecNestedParentSource {
    val hdr: UByte
    val required_body: CodecNestedBodySource
    val optional_body: CodecNestedBodySource?
    val body_list: CodecNestedParentBodyListList
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecNestedParent()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecNestedParent(
    override var hdr: UByte = 0.toUByte(),
    override var required_body: CodecNestedBody = CodecNestedBody(),
    override var optional_body: CodecNestedBody? = null,
    override var body_list: CodecNestedParentBodyListList = CodecNestedParentBodyListList.fewest()
) : CodecNestedParentSource {
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
        /// Write any source of [CodecNestedParent] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `m` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecNestedParentSource, w: SceSink): CodecError? {
            val _e_hdr = src.hdr
            val _e_required_body = src.required_body
            val _e_optional_body = src.optional_body
            val _e_body_list = src.body_list
            val _d_m = _e_body_list.entries.size.toUByte()
            var _u_hdr = _e_hdr.toUInt()
            _u_hdr = (_u_hdr and 0x1u.inv()) or (if (_e_optional_body != null) 0x1u else 0x0u)
            val _c_hdr = _u_hdr.toUByte()
            w.writeU8(_c_hdr.toByte())?.let { return it }
            w.writeU8(_d_m.toByte())?.let { return it }
            CodecNestedBody.encodeSource(_e_required_body, w)?.let { return it }
            if (_e_optional_body != null) {
                CodecNestedBody.encodeSource(_e_optional_body, w)?.let { return it }
            }
            for (_entry in _e_body_list.entries) {
                CodecNestedBody.encodeSource(_entry, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecNestedParent> {
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
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val m = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val required_body = CodecNestedBody.decode(cursor).valueOr { _refused -> return _refused }
            val optional_body: CodecNestedBody? = if ((hdr.toInt() and 0x01) != 0) {
                CodecNestedBody.decode(cursor).valueOr { _refused -> return _refused }
            } else {
                null
            }
            if (m > 4u) {
                return Decoded.Refused(CodecError.TooManyElements)
            }
            val body_list: MutableList<CodecNestedBody> = mutableListOf<CodecNestedBody>().apply {
                repeat(m.toInt()) {
                    add(CodecNestedBody.decode(cursor).valueOr { _refused -> return _refused })
                }
            }
            return Decoded.Ok(
                CodecNestedParent(
                    hdr = hdr,
                    required_body = required_body,
                    optional_body = optional_body,
                    body_list = CodecNestedParentBodyListList.of(body_list) ?: return Decoded.Refused(CodecError.TooManyElements)
                )
            )
        }
    }
}
