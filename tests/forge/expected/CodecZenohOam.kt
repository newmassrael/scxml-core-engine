// SCE-MAP: codec_zenoh_oam:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_oam

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_entry.*
import com.sce.generated.codec_zenoh_ext_unit.*
import com.sce.generated.codec_zenoh_ext_zint.*
import com.sce.generated.codec_zenoh_ext_zbuf.*
import kotlin.jvm.JvmInline

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohOamVariant {
    data class CodecZenohExtUnit(val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnitSource) : CodecZenohOamVariant()
    data class CodecZenohExtZint(val body: com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZintSource) : CodecZenohOamVariant()
    data class CodecZenohExtZbuf(val body: com.sce.generated.codec_zenoh_ext_zbuf.CodecZenohExtZbufSource) : CodecZenohOamVariant()
    data class Default(val tag: CodecZenohOamUnknownTag, val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnitSource) : CodecZenohOamVariant()
}

/// The `extensions` element of [CodecZenohOam], 1 or more entries, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecZenohOamExtensionsList private constructor(val entries: List<CodecZenohExtEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecZenohExtEntrySource>): CodecZenohOamExtensionsList? =
            if (entries.size >= LEAST) CodecZenohOamExtensionsList(entries.toList()) else null

        /// The fewest entries the bound holds, each made by default: what a value made by default holds.
        fun fewest(): CodecZenohOamExtensionsList = CodecZenohOamExtensionsList(List(LEAST) { CodecZenohExtEntry() })
    }
}

/// The tag the catch-all arm of [CodecZenohOam]'s variant holds: one the bits
/// the tag is read from hold and no arm names, held to that where a value of
/// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value
/// holds is written and read back as the catch-all.
@JvmInline
value class CodecZenohOamUnknownTag private constructor(val tag: UByte) {
    /// The tag as a `ULong`, the type an encode derives a tag in.
    val wide: ULong
        get() = tag.toULong()

    companion object {
        /// `tag` as a value of the catch-all, or `null` where an arm names it or the
        /// bits the tag is read from do not hold it.
        fun of(tag: UByte): CodecZenohOamUnknownTag? =
            when (tag.toUInt()) {
                in 0x0u..0x2u -> null
                else -> if (tag.toUInt() <= 0x3u) CodecZenohOamUnknownTag(tag) else null
            }

        /// The least tag no arm names: what a catch-all made by default holds.
        fun fewest(): CodecZenohOamUnknownTag = CodecZenohOamUnknownTag(0x3u.toUByte())
    }
}

/// The elements a value of [CodecZenohOam] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohOam.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohOamSource {
    val header: UByte
    val id: UShort
    val extensions: CodecZenohOamExtensionsList?
    val body: CodecZenohOamVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohOam()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohOam(
    override var header: UByte = 0x1f.toUByte(),
    override var id: UShort = 0.toUShort(),
    override var extensions: CodecZenohOamExtensionsList? = null,
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecZenohOamVariant = CodecZenohOamVariant.CodecZenohExtUnit(com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit())
) : CodecZenohOamSource {
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

    fun enc(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 5) and 0x03).toUByte()
    }

    fun setEnc(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x03 shl 5
        val _val = (v.toInt() and 0x03) shl 5
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
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
        /// Write any source of [CodecZenohOam] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohOamSource, w: SceSink): CodecError? {
            val _e_header = src.header
            val _e_id = src.id
            val _e_extensions = src.extensions
            val _r_body = src.body
            val _t_body: ULong = when (val _b = _r_body) {
                is CodecZenohOamVariant.CodecZenohExtUnit -> 0x0uL
                is CodecZenohOamVariant.CodecZenohExtZint -> 0x1uL
                is CodecZenohOamVariant.CodecZenohExtZbuf -> 0x2uL
                is CodecZenohOamVariant.Default -> _b.tag.wide
            }
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x60u.inv()) or ((_t_body and 0x3uL).toUInt() shl 5)
            _u_header = (_u_header and 0x80u.inv()) or (if (_e_extensions != null) 0x80u else 0x0u)
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            w.writeVleU16(_e_id)?.let { return it }
            if (_e_extensions != null) {
                var _left_extensions = _e_extensions.entries.size
                for (_entry in _e_extensions.entries) {
                    _left_extensions -= 1
                    CodecZenohExtEntry.encodeSource(_entry, w, CodecZenohExtEntryOverrides(header_Z = _left_extensions != 0))?.let { return it }
                }
            }
            when (val _b = _r_body) {
                is CodecZenohOamVariant.CodecZenohExtUnit -> CodecZenohExtUnit.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohOamVariant.CodecZenohExtZint -> CodecZenohExtZint.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohOamVariant.CodecZenohExtZbuf -> CodecZenohExtZbuf.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohOamVariant.Default -> CodecZenohExtUnit.encodeSource(_b.body, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecZenohOam> {
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
            val id = cursor.readVleU16().valueOr { _refused -> return _refused }
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
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohOamVariant = when (((header.toInt() shr 5) and 0x03)) {
                0 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohOamVariant.CodecZenohExtUnit(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohOamVariant.CodecZenohExtZint(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zbuf.CodecZenohExtZbuf.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohOamVariant.CodecZenohExtZbuf(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohOamVariant.Default(tag = CodecZenohOamUnknownTag.of(((header.toInt() shr 5) and 0x03).toUByte()) ?: error("a tag no arm names is one CodecZenohOamUnknownTag holds"), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecZenohOam(
                    header = header,
                    id = id,
                    extensions = extensions?.let { CodecZenohOamExtensionsList.of(it) ?: return Decoded.Refused(CodecError.TooManyElements) },
                    body = body
                )
            )
        }
    }
}
