// SCE-MAP: codec_zenoh_value_entry:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_value_entry

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_unit.*
import com.sce.generated.codec_zenoh_ext_zint.*
import com.sce.generated.codec_zenoh_value_zbuf.*
import kotlin.jvm.JvmInline

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohValueEntryVariant {
    data class CodecZenohExtUnit(val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnitSource) : CodecZenohValueEntryVariant()
    data class CodecZenohExtZint(val body: com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZintSource) : CodecZenohValueEntryVariant()
    data class CodecZenohValueZbuf(val body: com.sce.generated.codec_zenoh_value_zbuf.CodecZenohValueZbufSource) : CodecZenohValueEntryVariant()
    data class Default(val tag: CodecZenohValueEntryUnknownTag, val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnitSource) : CodecZenohValueEntryVariant()
}

/// The tag the catch-all arm of [CodecZenohValueEntry]'s variant holds: one the bits
/// the tag is read from hold and no arm names, held to that where a value of
/// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value
/// holds is written and read back as the catch-all.
@JvmInline
value class CodecZenohValueEntryUnknownTag private constructor(val tag: UByte) {
    /// The tag as a `ULong`, the type an encode derives a tag in.
    val wide: ULong
        get() = tag.toULong()

    companion object {
        /// `tag` as a value of the catch-all, or `null` where an arm names it or the
        /// bits the tag is read from do not hold it.
        fun of(tag: UByte): CodecZenohValueEntryUnknownTag? =
            when (tag.toUInt()) {
                in 0x0u..0x2u -> null
                else -> if (tag.toUInt() <= 0x3u) CodecZenohValueEntryUnknownTag(tag) else null
            }

        /// The least tag no arm names: what a catch-all made by default holds.
        fun fewest(): CodecZenohValueEntryUnknownTag = CodecZenohValueEntryUnknownTag(0x3u.toUByte())
    }
}

/// One-bit flags of [CodecZenohValueEntry] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecZenohValueEntry] ends them
/// on, which the chain derives from where each entry stands.
class CodecZenohValueEntryOverrides(
    /// The `M` flag of `header`, where a parent sets it.
    val header_M: Boolean? = null,
    /// The `Z` flag of `header`, where a parent sets it.
    val header_Z: Boolean? = null,
)

/// The elements a value of [CodecZenohValueEntry] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohValueEntry.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohValueEntrySource {
    val header: UByte
    val body: CodecZenohValueEntryVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohValueEntry()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohValueEntry(
    override var header: UByte = 0.toUByte(),
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecZenohValueEntryVariant = CodecZenohValueEntryVariant.CodecZenohExtUnit(com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit())
) : CodecZenohValueEntrySource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun extId(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setExtId(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun M(): Boolean = (this.header.toInt() and 0x10) != 0

    fun setM(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x10).toUByte()
        } else {
            (this.header.toInt() and 0x10.inv()).toUByte()
        }
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
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, afterShm: UByte): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        if (inputAfterShm(this).let { it != null && it != afterShm }) return CodecError.PresentIfMismatch
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
    fun encodeToByteArray(afterShm: UByte): ByteArray? {
        val _list = mutableListOf<Byte>()
        if (encode(MutableListSink(_list), afterShm) != null) return null
        return _list.toByteArray()
    }

    companion object {
        /// Write any source of [CodecZenohValueEntry] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohValueEntrySource, w: SceSink, overrides: CodecZenohValueEntryOverrides? = null): CodecError? {
            val _e_header = src.header
            val _r_body = src.body
            val _t_body: ULong = when (val _b = _r_body) {
                is CodecZenohValueEntryVariant.CodecZenohExtUnit -> 0x0uL
                is CodecZenohValueEntryVariant.CodecZenohExtZint -> 0x1uL
                is CodecZenohValueEntryVariant.CodecZenohValueZbuf -> 0x2uL
                is CodecZenohValueEntryVariant.Default -> _b.tag.wide
            }
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x60u.inv()) or ((_t_body and 0x3uL).toUInt() shl 5)
            overrides?.header_M?.let { _u_header = (_u_header and 0x10u.inv()) or (if (it) 0x10u else 0x0u) }
            overrides?.header_Z?.let { _u_header = (_u_header and 0x80u.inv()) or (if (it) 0x80u else 0x0u) }
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            when (val _b = _r_body) {
                is CodecZenohValueEntryVariant.CodecZenohExtUnit -> CodecZenohExtUnit.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohValueEntryVariant.CodecZenohExtZint -> CodecZenohExtZint.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohValueEntryVariant.CodecZenohValueZbuf -> CodecZenohValueZbuf.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohValueEntryVariant.Default -> CodecZenohExtUnit.encodeSource(_b.body, w)?.let { return it }
            }
            return null
        }

        /// The flag input `after_shm` a source of [CodecZenohValueEntry] says, which its encode
        /// is not handed (docs/adr/0065 §6 R4a), and `null` where the value says nothing of it.
        fun inputAfterShm(src: CodecZenohValueEntrySource): UByte? =
            when (val _b = src.body) { is CodecZenohValueEntryVariant.CodecZenohExtUnit -> null; is CodecZenohValueEntryVariant.CodecZenohExtZint -> null; is CodecZenohValueEntryVariant.CodecZenohValueZbuf -> CodecZenohValueZbuf.inputAfterShm(_b.body); is CodecZenohValueEntryVariant.Default -> null }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, afterShm: UByte): Decoded<CodecZenohValueEntry> {
            // Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohValueEntryVariant = when (((header.toInt() shr 5) and 0x03)) {
                0 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.CodecZenohExtUnit(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.CodecZenohExtZint(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_value_zbuf.CodecZenohValueZbuf.decode(cursor, afterShm).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.CodecZenohValueZbuf(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.Default(tag = CodecZenohValueEntryUnknownTag.of(((header.toInt() shr 5) and 0x03).toUByte()) ?: error("a tag no arm names is one CodecZenohValueEntryUnknownTag holds"), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecZenohValueEntry(
                    header = header,
                    body = body
                )
            )
        }
    }
}
