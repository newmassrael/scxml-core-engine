// SCE-MAP: codec_zenoh_query:52 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_query

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_entry.*
import kotlin.jvm.JvmInline

/// The `parameters` element of [CodecZenohQuery], 0 or more bytes, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohQueryParametersBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecZenohQueryParametersBytes? = CodecZenohQueryParametersBytes(bytes)

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohQueryParametersBytes = CodecZenohQueryParametersBytes(ByteArray(LEAST))
    }
}

/// The `extensions` element of [CodecZenohQuery], any number of entries, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecZenohQueryExtensionsList private constructor(val entries: List<CodecZenohExtEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecZenohExtEntrySource>): CodecZenohQueryExtensionsList? = CodecZenohQueryExtensionsList(entries.toList())

        /// No entry: what a value made by default holds.
        fun fewest(): CodecZenohQueryExtensionsList = CodecZenohQueryExtensionsList(emptyList())
    }
}

/// The elements a value of [CodecZenohQuery] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohQuery.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohQuerySource {
    val header: UByte
    val consolidation: UByte?
    val parameters: CodecZenohQueryParametersBytes?
    val extensions: CodecZenohQueryExtensionsList?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohQuery()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohQuery(
    override var header: UByte = 0x03.toUByte(),
    override var consolidation: UByte? = null,
    override var parameters: CodecZenohQueryParametersBytes? = null,
    override var extensions: CodecZenohQueryExtensionsList? = null
) : CodecZenohQuerySource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun mid(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x1F).toUByte()
    }

    fun setMid(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x1F shl 0
        val _val = (v.toInt() and 0x1F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun C(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setC(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun P(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setP(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x40).toUByte()
        } else {
            (this.header.toInt() and 0x40.inv()).toUByte()
        }
    }

    fun Z(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
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
        /// Write any source of [CodecZenohQuery] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `parameters_len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohQuerySource, w: SceSink): CodecError? {
            val _e_header = src.header
            val _e_consolidation = src.consolidation
            val _e_parameters = src.parameters
            val _e_extensions = src.extensions
            val _d_parameters_len = if (_e_parameters != null) _e_parameters.bytes.size.toULong() else 0.toULong()
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x20u.inv()) or (if (_e_consolidation != null) 0x20u else 0x0u)
            _u_header = (_u_header and 0x40u.inv()) or (if (_e_parameters != null) 0x40u else 0x0u)
            _u_header = (_u_header and 0x80u.inv()) or (if (_e_extensions != null) 0x80u else 0x0u)
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            if (_e_consolidation != null) {
                w.writeU8(_e_consolidation.toByte())?.let { return it }
            }
            if (((_c_header.toUInt() shr 6) and 0x1u) != 0x0u) {
                w.writeVleU64(_d_parameters_len)?.let { return it }
            }
            if (_e_parameters != null) {
                w.writeBytes(_e_parameters.bytes)?.let { return it }
            }
            if (_e_extensions != null) {
                for (_entry in _e_extensions.entries) {
                    CodecZenohExtEntry.encodeSource(_entry, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecZenohQuery> {
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
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val consolidation = if ((header.toInt() and 0x20) != 0) {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val parameters_len: ULong? = if ((header.toInt() and 0x40) != 0) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val parameters = if ((header.toInt() and 0x40) != 0) {
                val _len = parameters_len!!
                if (_len > (Int.MAX_VALUE - 0).toULong()) return Decoded.Refused(CodecError.NeedMoreBytes)
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val extensions: MutableList<CodecZenohExtEntry>? = if ((header.toInt() and 0x80) != 0) {
            mutableListOf<CodecZenohExtEntry>().also {
                while (true) {
                    if (cursor.remaining() == 0) break
                    it.add(CodecZenohExtEntry.decode(cursor).valueOr { _refused -> return _refused })
                }
            }
        } else {
            null
        }
            return Decoded.Ok(
                CodecZenohQuery(
                    header = header,
                    consolidation = consolidation,
                    parameters = parameters?.let { CodecZenohQueryParametersBytes.of(it) ?: return Decoded.Refused(CodecError.LengthOutOfRange) },
                    extensions = extensions?.let { CodecZenohQueryExtensionsList.of(it) ?: return Decoded.Refused(CodecError.TooManyElements) }
                )
            )
        }
    }
}
