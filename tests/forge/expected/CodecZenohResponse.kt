// SCE-MAP: codec_zenoh_response:76 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_response

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_entry.*
import com.sce.generated.codec_zenoh_reply.*
import com.sce.generated.codec_zenoh_err.*
import kotlin.jvm.JvmInline

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohResponseVariant {
    data class CodecZenohReply(val body: com.sce.generated.codec_zenoh_reply.CodecZenohReplySource) : CodecZenohResponseVariant()
    data class CodecZenohErr(val body: com.sce.generated.codec_zenoh_err.CodecZenohErrSource) : CodecZenohResponseVariant()
    data class Default(val tag: CodecZenohResponseUnknownTag, val body: com.sce.generated.codec_zenoh_reply.CodecZenohReplySource) : CodecZenohResponseVariant()
}

/// The `suffix` element of [CodecZenohResponse], 0 to 65535 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohResponseSuffixText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 65535

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): CodecZenohResponseSuffixText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) CodecZenohResponseSuffixText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohResponseSuffixText = CodecZenohResponseSuffixText("\u0000".repeat(LEAST))
    }
}

/// The `extensions` element of [CodecZenohResponse], 1 or more entries, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecZenohResponseExtensionsList private constructor(val entries: List<CodecZenohExtEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecZenohExtEntrySource>): CodecZenohResponseExtensionsList? =
            if (entries.size >= LEAST) CodecZenohResponseExtensionsList(entries.toList()) else null

        /// The fewest entries the bound holds, each made by default: what a value made by default holds.
        fun fewest(): CodecZenohResponseExtensionsList = CodecZenohResponseExtensionsList(List(LEAST) { CodecZenohExtEntry() })
    }
}

/// The tag the catch-all arm of [CodecZenohResponse]'s variant holds: one the bits
/// the tag is read from hold and no arm names, held to that where a value of
/// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value
/// holds is written and read back as the catch-all.
@JvmInline
value class CodecZenohResponseUnknownTag private constructor(val tag: UByte) {
    /// The tag as a `ULong`, the type an encode derives a tag in.
    val wide: ULong
        get() = tag.toULong()

    companion object {
        /// `tag` as a value of the catch-all, or `null` where an arm names it or the
        /// bits the tag is read from do not hold it.
        fun of(tag: UByte): CodecZenohResponseUnknownTag? =
            when (tag.toUInt()) {
                in 0x4u..0x5u -> null
                else -> if (tag.toUInt() <= 0x1Fu) CodecZenohResponseUnknownTag(tag) else null
            }

        /// The least tag no arm names: what a catch-all made by default holds.
        fun fewest(): CodecZenohResponseUnknownTag = CodecZenohResponseUnknownTag(0x0u.toUByte())
    }
}

/// One-bit flags of [CodecZenohResponse] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecZenohResponse] ends them
/// on, which the chain derives from where each entry stands.
class CodecZenohResponseOverrides(
    /// The `M` flag of `header`, where a parent sets it.
    val header_M: Boolean? = null,
)

/// The elements a value of [CodecZenohResponse] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohResponse.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohResponseSource {
    val header: UByte
    val request_id: ULong
    val key_id: UInt
    val suffix: CodecZenohResponseSuffixText?
    val extensions: CodecZenohResponseExtensionsList?
    val body: CodecZenohResponseVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohResponse()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohResponse(
    override var header: UByte = 0x1b.toUByte(),
    override var request_id: ULong = 0uL,
    override var key_id: UInt = 0u,
    override var suffix: CodecZenohResponseSuffixText? = null,
    override var extensions: CodecZenohResponseExtensionsList? = null,
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecZenohResponseVariant = CodecZenohResponseVariant.CodecZenohReply(com.sce.generated.codec_zenoh_reply.CodecZenohReply())
) : CodecZenohResponseSource {
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

    fun N(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setN(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun M(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setM(v: Boolean) {
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
        /// Write any source of [CodecZenohResponse] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `suffix_len` from what each measures, and never reads it off the
        /// source. A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohResponseSource, w: SceSink, overrides: CodecZenohResponseOverrides? = null): CodecError? {
            val _e_header = src.header
            val _e_request_id = src.request_id
            val _e_key_id = src.key_id
            val _e_suffix = src.suffix
            val _e_extensions = src.extensions
            val _r_body = src.body
            val _d_suffix_len = if (_e_suffix != null) _e_suffix.text.encodeToByteArray().size.toULong() else 0.toULong()
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x20u.inv()) or (if (_e_suffix != null) 0x20u else 0x0u)
            _u_header = (_u_header and 0x80u.inv()) or (if (_e_extensions != null) 0x80u else 0x0u)
            overrides?.header_M?.let { _u_header = (_u_header and 0x40u.inv()) or (if (it) 0x40u else 0x0u) }
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            w.writeVleU64(_e_request_id)?.let { return it }
            w.writeVleU32(_e_key_id)?.let { return it }
            if (((_c_header.toUInt() shr 5) and 0x1u) != 0x0u) {
                w.writeVleU64(_d_suffix_len)?.let { return it }
            }
            if (_e_suffix != null) {
                w.writeBytes(_e_suffix.text.encodeToByteArray())?.let { return it }
            }
            if (_e_extensions != null) {
                var _left_extensions = _e_extensions.entries.size
                for (_entry in _e_extensions.entries) {
                    _left_extensions -= 1
                    CodecZenohExtEntry.encodeSource(_entry, w, CodecZenohExtEntryOverrides(header_Z = _left_extensions != 0))?.let { return it }
                }
            }
            when (val _b = _r_body) {
                is CodecZenohResponseVariant.CodecZenohReply -> CodecZenohReply.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohResponseVariant.CodecZenohErr -> CodecZenohErr.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohResponseVariant.Default -> CodecZenohReply.encodeSource(_b.body, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecZenohResponse> {
            // RFC §synth-5-B peek-byte / streaming-
            // prefix: streaming prefix decode (variable-length fields
            // supported via per-field present_if/tlv-chain/embed/repeat
            // helpers). Peek-byte mode additionally peeks the cursor's
            // next byte for variant tag without advancing — arm body
            // decoder reads it as own header.
            val header = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val request_id = cursor.readVleU64().valueOr { _refused -> return _refused }
            val key_id = cursor.readVleU32().valueOr { _refused -> return _refused }
            val suffix_len: ULong? = if ((header.toInt() and 0x20) != 0) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val suffix = if ((header.toInt() and 0x20) != 0) {
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
            val _peekRaw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        val _peek: UByte = _peekRaw[0].toUByte()
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohResponseVariant = when (((_peek.toInt() shr 0) and 0x1F)) {
                4 -> {
                    val _arm = com.sce.generated.codec_zenoh_reply.CodecZenohReply.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohResponseVariant.CodecZenohReply(_arm)
                }
                5 -> {
                    val _arm = com.sce.generated.codec_zenoh_err.CodecZenohErr.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohResponseVariant.CodecZenohErr(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_reply.CodecZenohReply.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohResponseVariant.Default(tag = CodecZenohResponseUnknownTag.of(((_peek.toInt() shr 0) and 0x1F).toUByte()) ?: error("a tag no arm names is one CodecZenohResponseUnknownTag holds"), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecZenohResponse(
                    header = header,
                    request_id = request_id,
                    key_id = key_id,
                    suffix = suffix?.let { CodecZenohResponseSuffixText.of(it) ?: return Decoded.Refused(CodecError.LengthOutOfRange) },
                    extensions = extensions?.let { CodecZenohResponseExtensionsList.of(it) ?: return Decoded.Refused(CodecError.TooManyElements) },
                    body = body
                )
            )
        }
    }
}
