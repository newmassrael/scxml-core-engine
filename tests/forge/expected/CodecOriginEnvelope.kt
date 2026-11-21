// SCE-MAP: codec_origin_envelope:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_origin_envelope

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_origin_leaf.*
import kotlin.jvm.JvmInline

/// The `note` element of [CodecOriginEnvelope], 0 to 16 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecOriginEnvelopeNoteText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 16

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): CodecOriginEnvelopeNoteText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) CodecOriginEnvelopeNoteText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecOriginEnvelopeNoteText = CodecOriginEnvelopeNoteText("\u0000".repeat(LEAST))
    }
}

/// The `items` element of [CodecOriginEnvelope], 0 to 3 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecOriginEnvelopeItemsList private constructor(val entries: List<CodecOriginLeafSource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 3

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecOriginLeafSource>): CodecOriginEnvelopeItemsList? =
            if (entries.size <= MOST) CodecOriginEnvelopeItemsList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): CodecOriginEnvelopeItemsList = CodecOriginEnvelopeItemsList(emptyList())
    }
}

/// The elements a value of [CodecOriginEnvelope] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecOriginEnvelope.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecOriginEnvelopeSource {
    val hdr: UByte
    val note: CodecOriginEnvelopeNoteText
    val required: CodecOriginLeafSource
    val optional: CodecOriginLeafSource?
    val items: CodecOriginEnvelopeItemsList
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecOriginEnvelope()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecOriginEnvelope(
    override var hdr: UByte = 0.toUByte(),
    override var note: CodecOriginEnvelopeNoteText = CodecOriginEnvelopeNoteText.fewest(),
    override var required: CodecOriginLeaf = CodecOriginLeaf(),
    override var optional: CodecOriginLeaf? = null,
    override var items: CodecOriginEnvelopeItemsList = CodecOriginEnvelopeItemsList.fewest()
) : CodecOriginEnvelopeSource {
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
        /// Write any source of [CodecOriginEnvelope] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `m`, `note_len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecOriginEnvelopeSource, w: SceSink): CodecError? {
            val _e_hdr = src.hdr
            val _e_note = src.note
            val _e_required = src.required
            val _e_optional = src.optional
            val _e_items = src.items
            val _d_note_len = _e_note.text.encodeToByteArray().size.toUByte()
            val _d_m = _e_items.entries.size.toUByte()
            var _u_hdr = _e_hdr.toUInt()
            _u_hdr = (_u_hdr and 0x1u.inv()) or (if (_e_optional != null) 0x1u else 0x0u)
            val _c_hdr = _u_hdr.toUByte()
            w.writeU8(_c_hdr.toByte())?.let { return it }
            w.writeU8(_d_note_len.toByte())?.let { return it }
            w.writeBytes(_e_note.text.encodeToByteArray())?.let { return it }
            w.writeU8(_d_m.toByte())?.let { return it }
            CodecOriginLeaf.encodeSource(_e_required, w)?.let { return it }
            if (_e_optional != null) {
                CodecOriginLeaf.encodeSource(_e_optional, w)?.let { return it }
            }
            for (_entry in _e_items.entries) {
                CodecOriginLeaf.encodeSource(_entry, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecOriginEnvelope> {
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
            val note_len = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val note = run {
                val _len = note_len
                if (_len > 16u) {
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
            }
            val m = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val required = CodecOriginLeaf.decode(cursor).valueOr { _refused -> return _refused }
            val optional: CodecOriginLeaf? = if ((hdr.toInt() and 0x01) != 0) {
                CodecOriginLeaf.decode(cursor).valueOr { _refused -> return _refused }
            } else {
                null
            }
            if (m > 3u) {
                return Decoded.Refused(CodecError.TooManyElements)
            }
            val items: MutableList<CodecOriginLeaf> = mutableListOf<CodecOriginLeaf>().apply {
                repeat(m.toInt()) {
                    add(CodecOriginLeaf.decode(cursor).valueOr { _refused -> return _refused })
                }
            }
            return Decoded.Ok(
                CodecOriginEnvelope(
                    hdr = hdr,
                    note = CodecOriginEnvelopeNoteText.of(note) ?: return Decoded.Refused(CodecError.LengthOutOfRange),
                    required = required,
                    optional = optional,
                    items = CodecOriginEnvelopeItemsList.of(items) ?: return Decoded.Refused(CodecError.TooManyElements)
                )
            )
        }
    }
}
