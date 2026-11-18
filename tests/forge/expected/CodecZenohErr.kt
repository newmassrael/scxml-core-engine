// SCE-MAP: codec_zenoh_err:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_err

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_encoding.*
import com.sce.generated.codec_zenoh_ext_entry.*
import kotlin.jvm.JvmInline

/// The `extensions` element of [CodecZenohErr], 1 or more entries, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecZenohErrExtensionsList private constructor(val entries: List<CodecZenohExtEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecZenohExtEntrySource>): CodecZenohErrExtensionsList? =
            if (entries.size >= LEAST) CodecZenohErrExtensionsList(entries.toList()) else null

        /// The fewest entries the bound holds, each made by default: what a value made by default holds.
        fun fewest(): CodecZenohErrExtensionsList = CodecZenohErrExtensionsList(List(LEAST) { CodecZenohExtEntry() })
    }
}

/// The `payload` element of [CodecZenohErr], 0 to 4294967295 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohErrPayloadBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Int = 4294967295

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecZenohErrPayloadBytes? =
            if (bytes.size <= MOST) CodecZenohErrPayloadBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohErrPayloadBytes = CodecZenohErrPayloadBytes(ByteArray(LEAST))
    }
}

/// One-bit flags of [CodecZenohErr] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecZenohErr] ends them
/// on, which the chain derives from where each entry stands.
class CodecZenohErrOverrides(
    /// The `X` flag of `header`, where a parent sets it.
    val header_X: Boolean? = null,
)

/// The elements a value of [CodecZenohErr] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohErr.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohErrSource {
    val header: UByte
    val encoding: CodecZenohEncodingSource?
    val extensions: CodecZenohErrExtensionsList?
    val payload: CodecZenohErrPayloadBytes
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohErr()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohErr(
    override var header: UByte = 0x05.toUByte(),
    override var encoding: CodecZenohEncoding? = null,
    override var extensions: CodecZenohErrExtensionsList? = null,
    override var payload: CodecZenohErrPayloadBytes = CodecZenohErrPayloadBytes.fewest()
) : CodecZenohErrSource {
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

    fun X(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setX(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun E(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setE(v: Boolean) {
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
        /// Write any source of [CodecZenohErr] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `payload_len` from what each measures, and never reads it off the
        /// source. A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohErrSource, w: SceSink, overrides: CodecZenohErrOverrides? = null): CodecError? {
            val _e_header = src.header
            val _e_encoding = src.encoding
            val _e_extensions = src.extensions
            val _e_payload = src.payload
            val _d_payload_len = _e_payload.bytes.size.toULong()
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x40u.inv()) or (if (_e_encoding != null) 0x40u else 0x0u)
            _u_header = (_u_header and 0x80u.inv()) or (if (_e_extensions != null) 0x80u else 0x0u)
            overrides?.header_X?.let { _u_header = (_u_header and 0x20u.inv()) or (if (it) 0x20u else 0x0u) }
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            if (_e_encoding != null) {
                CodecZenohEncoding.encodeSource(_e_encoding, w)?.let { return it }
            }
            if (_e_extensions != null) {
                var _left_extensions = _e_extensions.entries.size
                for (_entry in _e_extensions.entries) {
                    _left_extensions -= 1
                    CodecZenohExtEntry.encodeSource(_entry, w, CodecZenohExtEntryOverrides(header_Z = _left_extensions != 0))?.let { return it }
                }
            }
            w.writeVleU64(_d_payload_len)?.let { return it }
            w.writeBytes(_e_payload.bytes)?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecZenohErr> {
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
            val encoding: CodecZenohEncoding? = if ((header.toInt() and 0x40) != 0) {
                CodecZenohEncoding.decode(cursor).valueOr { _refused -> return _refused }
            } else {
                null
            }
            val extensions: MutableList<CodecZenohExtEntry>? = if ((header.toInt() and 0x80) != 0) {
            mutableListOf<CodecZenohExtEntry>().also {
                var _more = false
                while (true) {
                    val _entry = CodecZenohExtEntry.decode(cursor).valueOr { _refused -> return _refused }
                    _more = _entry.Z()
                    it.add(_entry)
                    if (!_more) break
                }
                if (_more && cursor.remaining() == 0) { return Decoded.Refused(CodecError.NeedMoreBytes) }
            }
        } else {
            null
        }
            val payload_len = cursor.readVleU64().valueOr { _refused -> return _refused }
            val payload = run {
                val _len = payload_len
                if (_len > 4294967295uL) {
                    return Decoded.Refused(CodecError.LengthOutOfRange)
                }
                if (_len > (Int.MAX_VALUE - 0).toULong()) return Decoded.Refused(CodecError.NeedMoreBytes)
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            return Decoded.Ok(
                CodecZenohErr(
                    header = header,
                    encoding = encoding,
                    extensions = extensions?.let { CodecZenohErrExtensionsList.of(it) ?: return Decoded.Refused(CodecError.TooManyElements) },
                    payload = CodecZenohErrPayloadBytes.of(payload) ?: return Decoded.Refused(CodecError.LengthOutOfRange)
                )
            )
        }
    }
}
